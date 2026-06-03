use crate::engine;
use crate::models::{
    DailyExecutionPlan, DailyOperationReport, DailyOperationStatus, DailyOperationStep,
};
use crate::repository::{Repository, RepositoryContext};
use anyhow::Result;
use chrono::Local;

pub async fn run_daily_operation(
    repo: &dyn Repository,
    ctx: &RepositoryContext,
) -> Result<DailyOperationReport> {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let started_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let mut report = DailyOperationReport {
        date: date.clone(),
        started_at,
        completed_at: None,
        status: DailyOperationStatus::Running,
        steps: Vec::new(),
        plan: None,
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    let config = repo.load_config(ctx).await?;

    // Step 1: Refresh Fund NAV
    add_step(&mut report, "刷新基金净值");
    match engine::refresh::refresh_fund_navs(repo, ctx, &config).await {
        Ok(count) => complete_step(
            &mut report,
            DailyOperationStatus::Success,
            format!("成功刷新 {} 个基金净值", count),
        ),
        Err(e) => {
            let msg = format!("基金净值刷新失败: {}", e);
            report.warnings.push(msg.clone());
            complete_step(&mut report, DailyOperationStatus::PartialSuccess, msg);
        }
    }

    // Step 2: Refresh Market Data
    add_step(&mut report, "刷新市场行情");
    match engine::refresh::refresh_market_data(repo, ctx, &config).await {
        Ok((success, _skipped, failed)) => {
            if failed > 0 {
                let msg = format!("刷新成功: {}, 失败: {}", success, failed);
                report
                    .warnings
                    .push(format!("{} 个市场行情刷新失败", failed));
                complete_step(&mut report, DailyOperationStatus::PartialSuccess, msg);
            } else {
                complete_step(
                    &mut report,
                    DailyOperationStatus::Success,
                    format!("成功刷新 {} 个标的", success),
                );
            }
        }
        Err(e) => {
            let msg = format!("市场行情刷新失败: {}", e);
            report.warnings.push(msg.clone());
            complete_step(&mut report, DailyOperationStatus::PartialSuccess, msg);
        }
    }

    // Step 3: Run Due DCA Plans
    add_step(&mut report, "执行到期定投");
    match engine::dca::auto_execute_dca(repo, ctx, &config, &date).await {
        Ok(res) => {
            let status = if res.success {
                DailyOperationStatus::Success
            } else {
                DailyOperationStatus::PartialSuccess
            };
            complete_step(&mut report, status, res.message);
        }
        Err(e) => {
            let msg = format!("定投执行失败: {}", e);
            report.warnings.push(msg.clone());
            complete_step(&mut report, DailyOperationStatus::Failed, msg);
        }
    }

    // Step 4: Alipay Holding Alignment (Check only)
    add_step(&mut report, "持仓对齐检查");
    let snapshots_res = repo.load_alipay_snapshots(ctx).await;
    let snapshots = match snapshots_res {
        Ok(snaps) => {
            let latest_snap_date = snaps.iter().map(|s| s.snapshot_date.clone()).max();
            if let Some(d) = latest_snap_date {
                complete_step(
                    &mut report,
                    DailyOperationStatus::Success,
                    format!("发现最新支付宝快照日期: {}", d),
                );
            } else {
                complete_step(
                    &mut report,
                    DailyOperationStatus::Skipped,
                    "未找到支付宝快照数据，跳过对齐检查".to_string(),
                );
            }
            snaps
        }
        Err(e) => {
            let msg = format!("加载快照失败: {}", e);
            report.warnings.push(msg.clone());
            complete_step(&mut report, DailyOperationStatus::PartialSuccess, msg);
            Vec::new()
        }
    };

    // Step 5: System Reconciliation
    add_step(&mut report, "系统一致性对账");
    let state_res = repo.load_state(ctx).await;
    let txs_res = repo.load_transactions(ctx).await;

    let state = match (state_res, txs_res) {
        (Ok(state), Ok(txs)) => {
            let recon_report = engine::portfolio_reconciliation::reconcile_portfolio(
                &ctx.portfolio_id,
                &state,
                &txs,
            );
            if recon_report.summary.total_issues == 0 {
                complete_step(
                    &mut report,
                    DailyOperationStatus::Success,
                    "数据一致，未发现问题".to_string(),
                );
            } else {
                let msg = format!(
                    "发现 {} 个一致性问题 (严重: {})",
                    recon_report.summary.total_issues, recon_report.summary.critical_issues
                );
                report.warnings.push(msg.clone());
                complete_step(&mut report, DailyOperationStatus::PartialSuccess, msg);
            }
            state
        }
        (Err(e), _) | (_, Err(e)) => {
            let msg = format!("加载对账数据失败: {}", e);
            report.errors.push(msg.clone());
            complete_step(&mut report, DailyOperationStatus::Failed, msg);
            crate::models::PortfolioState::default()
        }
    };

    // Step 6: Data Verify
    add_step(&mut report, "数据完整性校验");
    match engine::verification::verify_data(repo, ctx, false).await {
        Ok(verify_res) => {
            if verify_res.summary.errors == 0 {
                complete_step(
                    &mut report,
                    DailyOperationStatus::Success,
                    "校验通过".to_string(),
                );
            } else {
                let msg = format!("发现 {} 个数据异常", verify_res.summary.errors);
                report.warnings.push(msg.clone());
                complete_step(&mut report, DailyOperationStatus::PartialSuccess, msg);
            }
        }
        Err(e) => {
            complete_step(
                &mut report,
                DailyOperationStatus::Failed,
                format!("校验引擎错误: {}", e),
            );
        }
    }

    // Step 7: Generate Kelly Daily Plan
    add_step(&mut report, "生成每日执行计划");
    let result = async {
        let nav_cache = repo.load_nav_cache(ctx).await?;
        let dca_plans = repo.load_plans(ctx).await?;
        let dca_preview =
            engine::dca::calculate_dca_preview(&config, &dca_plans, &nav_cache, &date);

        // Load caches for risk and regime
        let risk_cache = repo.load_risk_cache(ctx).await?;
        let regime_cache = repo.load_regime_cache(ctx).await?.clone();

        let risk_overlay = if let Some(rc) = risk_cache {
            rc.overlay
        } else {
            crate::models::GlobalRiskOverlay::default()
        };

        let mut regimes = std::collections::HashMap::new();
        for entry in regime_cache.entries {
            for asset in &config.assets {
                let symbol_opt = asset
                    .reference_instrument_symbol
                    .clone()
                    .or(asset.reference_index_symbol.clone());
                if let Some(_s) = symbol_opt.filter(|s| *s == entry.symbol) {
                    regimes.insert(asset.asset_id.clone(), entry.result.clone());
                }
            }
        }

        let decision = engine::decision::generate_buy_suggestions(&config, &state, date.clone());
        let adjusted = engine::adjusted_decision::calculate_adjusted_decision(
            &config,
            &state,
            &decision,
            &risk_overlay,
            &regimes,
        );
        let kelly =
            engine::kelly::calculate_kelly_preview(&config, &decision, &risk_overlay, &regimes);

        let mut latest_snaps = std::collections::HashMap::new();
        for s in &snapshots {
            let entry = latest_snaps.entry(s.asset_id.clone()).or_insert(s.clone());
            if s.snapshot_date >= entry.snapshot_date {
                *entry = s.clone();
            }
        }
        let mut reconciliation_results = Vec::new();
        for asset in &config.assets {
            if let Some(s) = latest_snaps.get(&asset.asset_id) {
                reconciliation_results
                    .push(engine::reconciliation::reconcile_asset(&config, &state, s));
            }
        }

        let plan = engine::daily_plan::generate_daily_execution_plan(
            &config,
            &state,
            date.clone(),
            &dca_preview,
            &adjusted,
            &kelly,
            &reconciliation_results,
        );
        Ok::<DailyExecutionPlan, anyhow::Error>(plan)
    }
    .await;

    match result {
        Ok(plan) => {
            report.plan = Some(plan);
            complete_step(
                &mut report,
                DailyOperationStatus::Success,
                "计划生成成功".to_string(),
            );
        }
        Err(e) => {
            let msg = format!("计划生成失败: {}", e);
            report.errors.push(msg.clone());
            complete_step(&mut report, DailyOperationStatus::Failed, msg);
        }
    }

    report.completed_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
    report.status = if !report.errors.is_empty() {
        DailyOperationStatus::Failed
    } else if !report.warnings.is_empty() {
        DailyOperationStatus::PartialSuccess
    } else {
        DailyOperationStatus::Success
    };

    Ok(report)
}

fn add_step(report: &mut DailyOperationReport, name: &str) {
    report.steps.push(DailyOperationStep {
        name: name.to_string(),
        status: DailyOperationStatus::Running,
        message: String::new(),
        started_at: Some(Local::now().format("%H:%M:%S").to_string()),
        completed_at: None,
    });
}

fn complete_step(report: &mut DailyOperationReport, status: DailyOperationStatus, message: String) {
    if let Some(step) = report.steps.last_mut() {
        step.status = status;
        step.message = message;
        step.completed_at = Some(Local::now().format("%H:%M:%S").to_string());
    }
}
