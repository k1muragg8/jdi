use crate::api;
use crate::engine;
use crate::models::*;
use crate::repository::{Repository, RepositoryContext};
use anyhow::{Result, anyhow};
use chrono::{Duration, Local, NaiveDate};
use std::collections::HashMap;

pub async fn run_backtest(
    repo: &dyn Repository,
    ctx: &RepositoryContext,
    config: &ConfigRoot,
    req: BacktestRequest,
) -> Result<BacktestReport> {
    let start_date = NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d")?;
    let end_date = NaiveDate::parse_from_str(&req.end_date, "%Y-%m-%d")?;

    if start_date > end_date {
        return Err(anyhow!("Start date must be before end date"));
    }

    // 1. Prepare Policy
    let policy = req
        .policy_override
        .clone()
        .unwrap_or(repo.load_operation_policy(ctx).await?);

    // 2. Load DCA Plans
    let plans = repo.load_plans(ctx).await?;

    // 3. Fetch Historical Data
    let mut warnings = Vec::new();
    let (nav_history, market_history) =
        fetch_all_history(config, &start_date, &end_date, &mut warnings).await?;

    // 4. Initialize Simulation State
    let initial_state = PortfolioState {
        cash: req.initial_cash,
        asset_holdings: Vec::new(), // Start fresh for backtest
    };

    // 5. Run Main Simulation
    let (main_results, main_metrics) = run_simulation_loop(
        &initial_state,
        &policy,
        config,
        &plans,
        &nav_history,
        &market_history,
        &start_date,
        &end_date,
        false, // not baseline
    )?;

    // 6. Run Baseline Simulation (Fixed DCA)
    let baseline_metrics = if req.include_baseline {
        let mut baseline_policy = policy.clone();
        baseline_policy.kelly_enabled = false;
        baseline_policy.pendulum_enabled = false;

        let (_, metrics) = run_simulation_loop(
            &initial_state,
            &baseline_policy,
            config,
            &plans,
            &nav_history,
            &market_history,
            &start_date,
            &end_date,
            true, // baseline
        )?;
        Some(metrics)
    } else {
        None
    };

    Ok(BacktestReport {
        request: req,
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        main_metrics,
        baseline_metrics,
        daily_results: main_results,
        warnings,
    })
}

async fn fetch_all_history(
    config: &ConfigRoot,
    start_date: &NaiveDate,
    end_date: &NaiveDate,
    warnings: &mut Vec<BacktestWarning>,
) -> Result<(HashMap<String, Vec<FundNav>>, HashMap<String, Vec<Candle>>)> {
    let mut nav_history = HashMap::new();
    let mut market_history = HashMap::new();

    let fund_provider = api::create_fund_provider(&config.api);
    let market_provider = api::create_market_provider(&config.market, Some("yahoo"));

    // Funds
    for asset in &config.assets {
        if !asset.enabled {
            continue;
        }
        match fund_provider.fetch_nav_history(&asset.fund_code) {
            Ok(history) => {
                nav_history.insert(asset.fund_code.clone(), history);
            }
            Err(e) => {
                warnings.push(BacktestWarning {
                    date: None,
                    asset_id: Some(asset.asset_id.clone()),
                    message: format!("Failed to fetch NAV history: {}", e),
                });
            }
        }
    }

    // Benchmarks
    let symbols: Vec<String> = config
        .assets
        .iter()
        .filter(|a| a.enabled)
        .filter_map(|a| {
            a.reference_instrument_symbol
                .clone()
                .or(a.reference_index_symbol.clone())
        })
        .collect();

    let lookback = 252; // Need enough for regime calculation
    for sym in symbols {
        if market_history.contains_key(&sym) {
            continue;
        }
        // We need data starting from start_date - lookback
        match market_provider.fetch_daily_candles(
            &sym,
            lookback + (end_date.signed_duration_since(*start_date).num_days() as usize),
        ) {
            Ok(history) => {
                market_history.insert(sym.clone(), history);
            }
            Err(e) => {
                warnings.push(BacktestWarning {
                    date: None,
                    asset_id: None,
                    message: format!("Failed to fetch market history for {}: {}", sym, e),
                });
            }
        }
    }

    Ok((nav_history, market_history))
}

fn run_simulation_loop(
    initial_state: &PortfolioState,
    policy: &OperationPolicy,
    config: &ConfigRoot,
    plans: &[DcaPlan],
    nav_history: &HashMap<String, Vec<FundNav>>,
    market_history: &HashMap<String, Vec<Candle>>,
    start_date: &NaiveDate,
    end_date: &NaiveDate,
    is_baseline: bool,
) -> Result<(Vec<BacktestDayResult>, BacktestMetrics)> {
    let mut state = initial_state.clone();
    let mut results = Vec::new();
    let mut metrics = BacktestMetrics::default();

    let mut day = *start_date;

    // Performance tracking
    let mut peak_value = 0.0;
    let mut total_buy_amount = 0.0;

    // Sort plans by priority
    let mut plans = plans.to_vec();
    plans.sort_by(|a, b| b.priority.cmp(&a.priority));

    while day <= *end_date {
        let date_str = day.format("%Y-%m-%d").to_string();

        // 1. Check if it's a trading day (at least one NAV or benchmark exists)
        let mut daily_navs = HashMap::new();
        let mut day_has_data = false;

        for (code, history) in nav_history {
            if let Some(nav) = history.iter().find(|n| n.nav_date == date_str) {
                daily_navs.insert(code.clone(), nav.clone());
                day_has_data = true;
            }
        }

        if !day_has_data {
            day += Duration::days(1);
            continue;
        }

        // 2. Update prices in simulated state
        for holding in &mut state.asset_holdings {
            if let Some(nav) = daily_navs.get(&holding.fund_code) {
                holding.latest_nav = Some(nav.nav);
                holding.last_market_value = holding.units * nav.nav;
            }
        }

        // 3. Prepare today's data context
        let summary = engine::calculate_portfolio_summary(config, &state);
        let total_value = summary.total_asset_value;
        let equity_value = summary.equity_value;
        let current_equity_weight = if total_value > 0.0 {
            equity_value / total_value
        } else {
            0.0
        };

        if total_value > peak_value {
            peak_value = total_value;
        }
        let drawdown = if peak_value > 0.0 {
            (total_value - peak_value) / peak_value
        } else {
            0.0
        };
        if drawdown < metrics.max_drawdown {
            metrics.max_drawdown = drawdown;
        }

        let mut regimes = HashMap::new();
        for (sym, history) in market_history {
            // Find index of today's candle or latest before today
            let candles_subset: Vec<Candle> = history
                .iter()
                .filter(|c| NaiveDate::parse_from_str(&c.date, "%Y-%m-%d").unwrap() <= day)
                .cloned()
                .collect();

            if !candles_subset.is_empty() {
                let regime =
                    engine::regime::calculate_market_regime(sym, &candles_subset, &config.regime);
                regimes.insert(sym.clone(), regime);
            }
        }

        let mut nav_cache = NavCache::default();
        for nav in daily_navs.values() {
            nav_cache.entries.push(NavCacheEntry {
                fund_code: nav.fund_code.clone(),
                nav: nav.nav,
                accumulated_nav: nav.accumulated_nav,
                nav_date: nav.nav_date.clone(),
                currency: nav.currency.clone(),
                source: nav.source.clone(),
                fetched_at: date_str.clone(),
            });
        }

        let mut execution_result = DcaExecutionResult::default();
        let mut suggestions = Vec::new();
        let mut daily_buy_total = 0.0;
        let mut trades = Vec::new();

        // 4. Evaluate Decisions
        for plan in &plans {
            let asset_config = config.assets.iter().find(|a| a.asset_id == plan.asset_id);
            if asset_config.is_none() {
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

            let suggestion = engine::operation::evaluate_dca_with_kelly_and_pendulum(
                plan,
                ac,
                policy,
                config,
                &regimes,
                &GlobalRiskOverlay::default(), // Baseline risk for backtest
                current_equity_weight,
                current_asset_weight,
                current_sector_weight,
                target_asset_weight,
                target_sector_weight,
                state.cash,
                daily_buy_total,
                &nav_cache,
                &date_str,
            );

            // Execute in simulation
            if suggestion.status == "execute" {
                if let Some(nav) = daily_navs.get(&plan.fund_code) {
                    let units = suggestion.suggested_amount / nav.nav;

                    // Apply to state
                    state.cash -= suggestion.suggested_amount;
                    if let Some(h) = state
                        .asset_holdings
                        .iter_mut()
                        .find(|h| h.asset_id == plan.asset_id)
                    {
                        h.units += units;
                        h.last_market_value = h.units * nav.nav;
                    } else {
                        state.asset_holdings.push(AssetHolding {
                            asset_id: plan.asset_id.clone(),
                            fund_code: plan.fund_code.clone(),
                            units,
                            units_estimated: false,
                            cost_basis: suggestion.suggested_amount,
                            latest_nav: Some(nav.nav),
                            latest_nav_date: Some(date_str.clone()),
                            latest_nav_source: Some("backtest".to_string()),
                            latest_nav_status: Some("正常".to_string()),
                            last_market_value: suggestion.suggested_amount,
                        });
                    }

                    trades.push(BacktestTradeSimulation {
                        asset_id: plan.asset_id.clone(),
                        fund_name: plan.fund_name.clone(),
                        amount: suggestion.suggested_amount,
                        units,
                        price: nav.nav,
                        trade_type: "buy".to_string(),
                    });

                    execution_result.executed_count += 1;
                    daily_buy_total += suggestion.suggested_amount;
                    total_buy_amount += suggestion.suggested_amount;
                    metrics.total_buy_days += 1;
                    if suggestion.suggested_amount > metrics.largest_buy_amount {
                        metrics.largest_buy_amount = suggestion.suggested_amount;
                    }
                }
            } else if suggestion.status == "skip" || suggestion.status == "pause" {
                execution_result.skipped_count += 1;
                metrics.total_skipped_days += 1;

                // Track metric counters
                if suggestion.reason.contains("现金储备不足") {
                    metrics.cash_reserve_block_count += 1;
                }
                if suggestion.reason.contains("目标") {
                    metrics.target_allocation_block_count += 1;
                }
            }

            if suggestion.caps_applied.contains("上限") {
                metrics.kelly_cap_hit_count += 1;
            }
            if suggestion.explanation.contains("高波动") {
                metrics.high_volatility_reduction_count += 1;
            }
            if suggestion.explanation.contains("热") {
                metrics.hot_market_reduction_count += 1;
            }

            suggestions.push(suggestion);
        }

        if !is_baseline {
            results.push(BacktestDayResult {
                date: date_str,
                total_value,
                cash: state.cash,
                equity_weight: current_equity_weight,
                execution_result,
                suggestions,
                trades,
            });
        }

        day += Duration::days(1);
    }

    // Final Metrics
    metrics.final_value = engine::calculate_portfolio_summary(config, &state).total_asset_value;
    metrics.total_invested = total_buy_amount;
    metrics.cash_remaining = state.cash;
    metrics.average_buy_amount = if metrics.total_buy_days > 0 {
        total_buy_amount / metrics.total_buy_days as f64
    } else {
        0.0
    };

    Ok((results, metrics))
}
