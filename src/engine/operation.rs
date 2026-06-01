use crate::engine;
use crate::models::{
    ConfigRoot, DcaExecutionResult, DcaFrequency, DcaPlan, DcaSettlement, DcaSettlementStatus,
    OperationPolicy, OperationReport, OperationSuggestion, Transaction,
};
use crate::repository::{Repository, RepositoryContext};
use anyhow::Result;
use chrono::{DateTime, Local, NaiveDate};
use std::collections::HashMap;

pub async fn run_autonomous_operation(
    repo: &dyn Repository,
    ctx: &RepositoryContext,
    config: &ConfigRoot,
) -> Result<OperationReport> {
    // 1. Evaluate and refresh state if needed
    evaluate_operation_state(repo, ctx, config).await?;

    // 2. Load context
    let policy = repo.load_operation_policy(ctx).await?;
    let mut state = repo.load_state(ctx).await?;
    let nav_cache = repo.load_nav_cache(ctx).await?;
    let mut plans = repo.load_plans(ctx).await?;
    let mut settlements = repo.load_settlements(ctx).await?;
    let mut transactions = repo.load_transactions(ctx).await?;
    let regime_cache = repo.load_regime_cache(ctx).await?;
    let risk_cache = repo.load_risk_cache(ctx).await?.unwrap_or_default();

    let date = Local::now().format("%Y-%m-%d").to_string();
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 3. Calculate valuation and weights
    let summary = engine::calculate_portfolio_summary(config, &state);
    let total_value = summary.total_asset_value;
    let equity_value = summary.equity_value;
    let current_equity_weight = if total_value > 0.0 {
        equity_value / total_value
    } else {
        0.0
    };

    let mut execution_result = DcaExecutionResult::default();
    let mut suggestions = Vec::new();
    let mut warnings = Vec::new();

    // Sort plans by priority
    plans.sort_by(|a, b| b.priority.cmp(&a.priority));

    // Index regime cache
    let mut regimes = HashMap::new();
    for entry in regime_cache.entries {
        regimes.insert(entry.symbol.clone(), entry.result);
    }

    let mut current_daily_buy_total = 0.0;
    let mut plans_changed = false;

    for plan in &mut plans {
        let asset_config = config.assets.iter().find(|a| a.asset_id == plan.asset_id);
        if asset_config.is_none() {
            warnings.push(format!("计划 {} 的资产不存在", plan.plan_id));
            continue;
        }
        let ac = asset_config.unwrap();

        let holding = state
            .asset_holdings
            .iter()
            .find(|h| h.asset_id == plan.asset_id);
        let current_asset_weight = if total_value > 0.0 {
            holding.map(|h| h.last_market_value).unwrap_or(0.0) / total_value
        } else {
            0.0
        };

        let sector_summary = summary
            .sector_summaries
            .iter()
            .find(|s| s.sector_name == ac.sector);
        let current_sector_weight = if total_value > 0.0 {
            sector_summary.map(|s| s.current_weight).unwrap_or(0.0)
        } else {
            0.0
        };

        let target_asset_weight = policy
            .target_asset_weights
            .get(&plan.asset_id)
            .cloned()
            .unwrap_or(0.0);
        let target_sector_weight = policy
            .target_sector_weights
            .get(&ac.sector)
            .cloned()
            .unwrap_or(0.0);

        let benchmark_symbol = ac
            .reference_instrument_symbol
            .as_deref()
            .or(ac.reference_index_symbol.as_deref());

        if benchmark_symbol.is_none() {
            warnings.push(format!("资产 {} 未配置基准指数", plan.asset_id));
        }

        // 4. Evaluate DCA with Kelly and Pendulum
        let mut suggestion = evaluate_dca_with_kelly_and_pendulum(
            plan,
            ac,
            &policy,
            config,
            &regimes,
            &risk_cache.overlay,
            current_equity_weight,
            current_asset_weight,
            current_sector_weight,
            target_asset_weight,
            target_sector_weight,
            state.cash,
            current_daily_buy_total,
            &nav_cache,
            &date,
        );

        // Check for Auto-pause / Auto-resume side effects on plan
        if suggestion.status == "pause" && policy.dca_auto_pause_when_target_reached && plan.enabled
        {
            plan.enabled = false;
            plans_changed = true;
            suggestion.reason += "，已自动暂停计划";
        } else if !plan.enabled
            && suggestion.status == "resume"
            && policy.dca_auto_resume_when_below_target
        {
            plan.enabled = true;
            plans_changed = true;
            suggestion.reason += "，已自动恢复计划";
        }

        // 5. Execute if needed
        if suggestion.status == "execute" {
            // Idempotency check
            let already_executed = settlements
                .iter()
                .any(|s| s.plan_id.as_deref() == Some(&plan.plan_id) && s.deduction_date == date);

            if already_executed {
                suggestion.status = "skip".to_string();
                suggestion.reason = "今日已执行".to_string();
            } else if suggestion.suggested_amount < 1.0 {
                suggestion.status = "skip".to_string();
                suggestion.reason = "建议买入金额过小".to_string();
            } else {
                let nav_entry = nav_cache
                    .entries
                    .iter()
                    .find(|e| e.fund_code == plan.fund_code)
                    .unwrap();
                let units = suggestion.suggested_amount / nav_entry.nav;
                let settlement_id = format!(
                    "dca_ops_{}_{}",
                    plan.plan_id,
                    Local::now().timestamp_millis()
                );
                let tx_id = format!("tx_ops_{}", settlement_id);

                let settlement = DcaSettlement {
                    settlement_id: settlement_id.clone(),
                    plan_id: Some(plan.plan_id.clone()),
                    asset_id: plan.asset_id.clone(),
                    fund_code: plan.fund_code.clone(),
                    fund_name: plan.fund_name.clone(),
                    scheduled_date: Some(date.clone()),
                    deduction_date: date.clone(),
                    confirmation_date: date.clone(),
                    amount: suggestion.suggested_amount,
                    confirmed_nav: nav_entry.nav,
                    confirmed_units: units,
                    fee: Some(0.0),
                    currency: plan.currency.clone(),
                    source: "ops_auto".to_string(),
                    status: DcaSettlementStatus::Confirmed,
                    applied: true,
                    note: Some(format!("Ops Auto: {}", suggestion.explanation)),
                    created_at: timestamp.clone(),
                };

                let tx = Transaction {
                    id: tx_id,
                    date: date.clone(),
                    transaction_type: "buy".to_string(),
                    asset_id: Some(plan.asset_id.clone()),
                    amount: suggestion.suggested_amount,
                    units: Some(units),
                    price: Some(nav_entry.nav),
                    fee: 0.0,
                    currency: plan.currency.clone(),
                    note: format!("Ops Auto: {}", plan.fund_name),
                    source: "ops".to_string(),
                    raw_description: format!("Ops Auto Execution: {}", suggestion.explanation),
                };

                if let Err(e) = engine::holdings::apply_transaction(&mut state, &tx) {
                    suggestion.status = "failed".to_string();
                    suggestion.reason = format!("执行失败: {}", e);
                    execution_result.failed_count += 1;
                } else {
                    settlements.push(settlement);
                    transactions.push(tx);
                    execution_result.executed_count += 1;
                    current_daily_buy_total += suggestion.suggested_amount;
                }
            }
        } else if suggestion.status == "skip" && check_is_due(plan, &date) == "今日应投" {
            execution_result.skipped_count += 1;
        }

        suggestions.push(suggestion);
    }

    // 6. Save changes
    if execution_result.executed_count > 0 {
        repo.save_settlements(ctx, &settlements).await?;
        repo.save_transactions(ctx, &transactions).await?;
        repo.save_state(ctx, &state).await?;
    }
    if plans_changed {
        repo.save_plans(ctx, &plans).await?;
    }

    // 7. Build report
    let report = build_operation_report(
        ctx,
        config,
        &policy,
        total_value,
        summary.available_cash,
        equity_value,
        current_equity_weight,
        execution_result,
        suggestions,
        warnings,
    );

    // Update status
    let mut status = repo.load_operation_status(ctx).await?;
    status.last_run_at = Some(report.timestamp.clone());
    status.last_report = Some(report.clone());
    repo.save_operation_status(ctx, &status).await?;

    Ok(report)
}

pub async fn evaluate_operation_state(
    repo: &dyn Repository,
    ctx: &RepositoryContext,
    config: &ConfigRoot,
) -> Result<()> {
    let policy = repo.load_operation_policy(ctx).await?;
    let nav_cache = repo.load_nav_cache(ctx).await?;
    let regime_cache = repo.load_regime_cache(ctx).await?;

    let now = Local::now();
    let interval = chrono::Duration::seconds(policy.market_refresh_interval_seconds as i64);

    // Check Market/Regime Cache
    let market_stale = if let Ok(fetched_at) =
        DateTime::parse_from_str(&regime_cache.fetched_at, "%Y-%m-%d %H:%M:%S")
    {
        now.signed_duration_since(fetched_at.with_timezone(&Local)) > interval
    } else {
        true
    };

    if market_stale {
        let _ = engine::refresh::refresh_market_data(repo, ctx, config).await;
    }

    // Check NAV Cache
    let nav_stale = if nav_cache.entries.is_empty() {
        true
    } else {
        let latest_fetch = nav_cache
            .entries
            .iter()
            .filter_map(|e| DateTime::parse_from_str(&e.fetched_at, "%Y-%m-%d %H:%M:%S").ok())
            .max();
        if let Some(fetch_at) = latest_fetch {
            now.signed_duration_since(fetch_at.with_timezone(&Local)) > chrono::Duration::hours(1)
        } else {
            true
        }
    };

    if nav_stale {
        let _ = engine::refresh::refresh_fund_navs(repo, ctx, config).await;
    }

    Ok(())
}

pub fn evaluate_dca_with_kelly_and_pendulum(
    plan: &DcaPlan,
    ac: &crate::models::AssetConfig,
    policy: &OperationPolicy,
    config: &ConfigRoot,
    regimes: &HashMap<String, crate::models::MarketRegimeResult>,
    risk_overlay: &crate::models::GlobalRiskOverlay,
    current_equity_weight: f64,
    current_asset_weight: f64,
    current_sector_weight: f64,
    target_asset_weight: f64,
    target_sector_weight: f64,
    total_cash: f64,
    current_daily_buy_total: f64,
    nav_cache: &crate::models::NavCache,
    date: &str,
) -> OperationSuggestion {
    let benchmark_symbol = ac
        .reference_instrument_symbol
        .as_deref()
        .or(ac.reference_index_symbol.as_deref());

    let mut benchmark_return = 0.0;
    let mut volatility = 0.0;
    let mut pendulum_score = 0.0;
    let mut regime_label = "未知".to_string();
    let mut kelly_multiplier = 1.0;
    let mut kelly_explanation = "未启用 Kelly".to_string();
    let mut regime_result = None;

    if let Some(symbol) = benchmark_symbol {
        if let Some(regime) = regimes.get(symbol) {
            benchmark_return = regime.latest_return;
            volatility = regime
                .windows
                .iter()
                .find(|w| w.window_days == policy.volatility_window_days)
                .map(|w| w.annualized_volatility)
                .unwrap_or_else(|| {
                    regime
                        .windows
                        .first()
                        .map(|w| w.annualized_volatility)
                        .unwrap_or(0.0)
                });

            pendulum_score = regime.pendulum_score;
            regime_label = regime.regime_label.clone();
            regime_result = Some(regime);
        }
    }

    if policy.kelly_enabled {
        let kelly_res = engine::kelly::calculate_single_asset_kelly(engine::kelly::KellyContext {
            config,
            asset_id: plan.asset_id.clone(),
            fund_code: plan.fund_code.clone(),
            fund_name: plan.fund_name.clone(),
            sector: ac.sector.clone(),
            base_suggested_buy: plan.amount,
            risk_overlay,
            regime: regime_result,
        });
        kelly_multiplier = kelly_res.kelly_multiplier;
        kelly_explanation = kelly_res.explanation;
    }

    let base_amount = plan.amount;
    let mut suggested_amount = base_amount * kelly_multiplier;
    let kelly_adjusted_amount = suggested_amount;

    let is_due = check_is_due(plan, date) == "今日应投";
    let mut status = "skip".to_string();
    let mut reason = "未到定投日".to_string();
    let mut caps_applied = Vec::new();

    if is_due {
        status = "execute".to_string();
        reason = "正常定投".to_string();

        if !plan.enabled {
            status = "skip".to_string();
            reason = "计划已禁用".to_string();
        }
        // 1. Check Equity Weight Target
        else if current_equity_weight >= policy.target_equity_weight * policy.dca_pause_threshold
        {
            status = "pause".to_string();
            reason = format!(
                "权益仓位 ({:.2}%) 已达目标 ({:.2}%)",
                current_equity_weight * 100.0,
                policy.target_equity_weight * 100.0
            );
        } else if current_equity_weight >= policy.target_equity_weight {
            status = "skip".to_string();
            reason = format!(
                "权益仓位 ({:.2}%) 已超目标 ({:.2}%)",
                current_equity_weight * 100.0,
                policy.target_equity_weight * 100.0
            );
        }
        // 2. Check Asset Weight Limit/Target
        else if target_asset_weight > 0.0
            && current_asset_weight >= target_asset_weight * policy.dca_pause_threshold
        {
            status = "pause".to_string();
            reason = format!(
                "资产权重 ({:.2}%) 已达暂停阈值 ({:.2}%)",
                current_asset_weight * 100.0,
                target_asset_weight * policy.dca_pause_threshold * 100.0
            );
        } else if target_asset_weight > 0.0 && current_asset_weight >= target_asset_weight {
            status = "skip".to_string();
            reason = format!(
                "资产权重 ({:.2}%) 已达目标 ({:.2}%)",
                current_asset_weight * 100.0,
                target_asset_weight * 100.0
            );
        } else if current_asset_weight >= policy.max_single_asset_weight {
            status = "skip".to_string();
            reason = format!(
                "单资产权重 ({:.2}%) 超过限制 ({:.2}%)",
                current_asset_weight * 100.0,
                policy.max_single_asset_weight * 100.0
            );
        }
        // 3. Check Sector Weight Limit/Target
        else if target_sector_weight > 0.0
            && current_sector_weight >= target_sector_weight * policy.dca_pause_threshold
        {
            status = "skip".to_string(); // Sector pause doesn't auto-pause plans usually, just skip
            reason = format!(
                "板块权重 ({:.2}%) 已达暂停阈值 ({:.2}%)",
                current_sector_weight * 100.0,
                target_sector_weight * policy.dca_pause_threshold * 100.0
            );
        } else if target_sector_weight > 0.0 && current_sector_weight >= target_sector_weight {
            status = "skip".to_string();
            reason = format!(
                "板块权重 ({:.2}%) 已达目标 ({:.2}%)",
                current_sector_weight * 100.0,
                target_sector_weight * 100.0
            );
        } else if current_sector_weight >= policy.max_sector_weight {
            status = "skip".to_string();
            reason = format!(
                "板块权重 ({:.2}%) 超过限制 ({:.2}%)",
                current_sector_weight * 100.0,
                policy.max_sector_weight * 100.0
            );
        }
        // 4. Check Cash Reserve
        else if total_cash - suggested_amount < policy.min_cash_reserve {
            status = "skip".to_string();
            reason = format!("现金储备不足 (低于 {:.2})", policy.min_cash_reserve);
        }
        // 5. Check Daily Buy Cap
        else if current_daily_buy_total + suggested_amount > policy.max_daily_buy_amount {
            status = "skip".to_string();
            reason = format!("今日买入已达上限 ({:.2})", policy.max_daily_buy_amount);
        }
        // 6. Check Single Asset Buy Cap
        else if suggested_amount > policy.max_single_asset_buy_amount {
            suggested_amount = policy.max_single_asset_buy_amount;
            caps_applied.push(format!(
                "单资产日买入上限 {:.2}",
                policy.max_single_asset_buy_amount
            ));
        }

        // 7. Check NAV missing
        if status == "execute" {
            let nav_entry = nav_cache
                .entries
                .iter()
                .find(|e| e.fund_code == plan.fund_code);
            if nav_entry.is_none() {
                status = "skip".to_string();
                reason = "缺少基金净值数据".to_string();
            }
        }
    } else {
        // Check for Auto-resume
        if !plan.enabled && policy.dca_auto_resume_when_below_target {
            let below_equity =
                current_equity_weight < policy.target_equity_weight * policy.dca_resume_threshold;
            let below_asset = target_asset_weight > 0.0
                && current_asset_weight < target_asset_weight * policy.dca_resume_threshold;

            if below_equity || below_asset {
                status = "resume".to_string();
                reason = if below_equity {
                    format!(
                        "权益仓位 ({:.2}%) 低于恢复阈值 ({:.2}%)",
                        current_equity_weight * 100.0,
                        policy.target_equity_weight * policy.dca_resume_threshold * 100.0
                    )
                } else {
                    format!(
                        "资产权重 ({:.2}%) 低于恢复阈值 ({:.2}%)",
                        current_asset_weight * 100.0,
                        target_asset_weight * policy.dca_resume_threshold * 100.0
                    )
                };
            }
        }
    }

    OperationSuggestion {
        asset_id: plan.asset_id.clone(),
        fund_name: plan.fund_name.clone(),
        fund_code: plan.fund_code.clone(),
        benchmark_symbol: benchmark_symbol.map(|s| s.to_string()),
        benchmark_return,
        volatility,
        pendulum_score,
        regime_label,
        current_weight: current_asset_weight,
        target_weight: target_asset_weight,
        allocation_gap: target_asset_weight - current_asset_weight,
        suggested_amount,
        kelly_adjusted_amount,
        kelly_multiplier,
        caps_applied: caps_applied.join(", "),
        status,
        reason,
        explanation: kelly_explanation,
    }
}

pub fn build_operation_report(
    ctx: &RepositoryContext,
    config: &ConfigRoot,
    policy: &OperationPolicy,
    total_value: f64,
    cash_value: f64,
    equity_value: f64,
    current_equity_weight: f64,
    execution_result: DcaExecutionResult,
    suggestions: Vec<OperationSuggestion>,
    warnings: Vec<String>,
) -> OperationReport {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    OperationReport {
        date,
        timestamp,
        portfolio_id: ctx.portfolio_id.clone(),
        portfolio_name: config.portfolio.name.clone(),
        total_value,
        cash_value,
        equity_value,
        current_equity_weight,
        target_equity_weight: policy.target_equity_weight,
        equity_gap: policy.target_equity_weight - current_equity_weight,
        dca_execution_result: execution_result,
        suggestions,
        warnings,
    }
}

fn check_is_due(plan: &DcaPlan, date_str: &str) -> String {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .unwrap_or_else(|_| Local::now().naive_local().date());
    match plan.frequency {
        DcaFrequency::Daily => "今日应投".to_string(),
        DcaFrequency::Weekly => {
            use chrono::Datelike;
            let weekday = date.weekday().number_from_monday(); // 1-7
            if let Some(target_weekday) = plan.weekday {
                if weekday == target_weekday {
                    "今日应投".to_string()
                } else {
                    "未到日期".to_string()
                }
            } else if weekday == 1 {
                "今日应投".to_string()
            } else {
                "未到日期".to_string()
            }
        }
        DcaFrequency::Monthly => {
            use chrono::Datelike;
            let day = date.day();
            if let Some(target_day) = plan.month_day {
                if day == target_day {
                    "今日应投".to_string()
                } else {
                    "未到日期".to_string()
                }
            } else if day == 1 {
                "今日应投".to_string()
            } else {
                "未到日期".to_string()
            }
        }
    }
}
