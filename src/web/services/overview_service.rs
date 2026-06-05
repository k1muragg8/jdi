//! Overview / dashboard data aggregation.

use crate::repository::RepositoryContext;
use crate::web::state::AppState;
use crate::{engine, models};
use anyhow::Result;
use chrono::Local;

pub async fn fetch_dashboard_summary(
    state: &AppState,
    ctx: &RepositoryContext,
) -> Result<models::DashboardSummary> {
    let config = state.repo.load_config(ctx).await?;
    let date = Local::now().format("%Y-%m-%d").to_string();

    let portfolio_state = state.repo.load_state(ctx).await?;
    let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);

    let dca_plans = state.repo.load_plans(ctx).await?;
    let settlements = state.repo.load_settlements(ctx).await?;
    let snapshots = state.repo.load_alipay_snapshots(ctx).await?;
    let nav_cache = state.repo.load_nav_cache(ctx).await?;

    let lifecycle = engine::dca_lifecycle::calculate_dca_lifecycle(
        &config,
        &dca_plans,
        &settlements,
        &snapshots,
        &portfolio_state,
        &nav_cache,
        &date,
    );

    let mut cache_status = state.repo.load_cache_status(ctx).await.unwrap_or_default();
    let market_cache = state.repo.load_market_cache(ctx).await.unwrap_or_default();
    cache_status.market_cache_size = market_cache.entries.len();
    cache_status.last_market_update = market_cache
        .entries
        .iter()
        .map(|e| &e.fetched_at)
        .max()
        .cloned();
    let risk_cache = state.repo.load_risk_cache(ctx).await?.unwrap_or_default();
    let regime_cache = state.repo.load_regime_cache(ctx).await?;

    let mut regimes = std::collections::HashMap::new();
    for entry in &regime_cache.entries {
        for asset in &config.assets {
            let symbol_opt = asset
                .reference_instrument_symbol
                .clone()
                .or(asset.reference_index_symbol.clone());
            if let Some(s) = symbol_opt {
                if s == entry.symbol {
                    regimes.insert(asset.asset_id.clone(), entry.result.clone());
                }
            }
        }
    }

    let decision = engine::explanation::explain_decision(
        &config,
        &portfolio_state,
        ctx.portfolio_id.clone(),
        date.clone(),
        &risk_cache.overlay,
        &regimes,
    );

    let operation_status = state
        .repo
        .load_operation_status(ctx)
        .await
        .unwrap_or_default();

    let mut alipay_total_value = None;
    let mut alipay_snapshot_date = None;
    if !snapshots.is_empty() {
        let mut latest_snaps: std::collections::HashMap<String, models::AlipaySnapshot> =
            std::collections::HashMap::new();
        for s in &snapshots {
            let key = if s.asset_id.is_empty() {
                format!("unmatched_{}", s.fund_code)
            } else {
                s.asset_id.clone()
            };
            let entry = latest_snaps.entry(key).or_insert(s.clone());
            if s.snapshot_date >= entry.snapshot_date {
                *entry = s.clone();
            }
        }
        alipay_total_value = Some(latest_snaps.values().map(|s| s.market_value).sum::<f64>());
        alipay_snapshot_date = latest_snaps
            .values()
            .map(|s| &s.snapshot_date)
            .max()
            .cloned();
    }

    let unclassified_asset_count = config
        .assets
        .iter()
        .filter(|a| a.enabled && (a.sector.is_empty() || a.sector == "未分类"))
        .count();

    let transactions = state.repo.load_transactions(ctx).await.unwrap_or_default();
    let report = crate::engine::portfolio_reconciliation::reconcile_portfolio(
        &ctx.portfolio_id,
        &portfolio_state,
        &transactions,
    );
    let reconciliation_issue_count = report.issues.len();

    let mut alipay_mismatch_count = 0;
    if let Ok(snapshots) = state.repo.load_alipay_snapshots(ctx).await {
        let mut latest_snaps = std::collections::HashMap::new();
        for s in &snapshots {
            let key = if s.asset_id.is_empty() {
                format!("unmatched_{}", s.fund_code)
            } else {
                s.asset_id.clone()
            };
            let entry = latest_snaps.entry(key).or_insert(s.clone());
            if s.snapshot_date >= entry.snapshot_date {
                *entry = s.clone();
            }
        }
        let mut processed_keys = std::collections::HashSet::new();
        for asset in &config.assets {
            if let Some(s) = latest_snaps.get(&asset.asset_id) {
                let res =
                    crate::engine::reconciliation::reconcile_asset(&config, &portfolio_state, s);
                if res.status == "需要校准"
                    || res.status == "份额不一致"
                    || res.status == "明显差异"
                    || res.status == "缺少系统持仓"
                {
                    alipay_mismatch_count += 1;
                }
                processed_keys.insert(asset.asset_id.clone());
            }
        }
        for (key, s) in latest_snaps {
            if !processed_keys.contains(&key) {
                let res =
                    crate::engine::reconciliation::reconcile_asset(&config, &portfolio_state, &s);
                if res.status == "需要校准"
                    || res.status == "份额不一致"
                    || res.status == "明显差异"
                    || res.status == "缺少系统持仓"
                {
                    alipay_mismatch_count += 1;
                }
            }
        }
    }

    Ok(models::DashboardSummary {
        portfolio: summary,
        lifecycle,
        cache_status,
        decision,
        risk_overlay: risk_cache.overlay,
        operation_status,
        backend: state.repo.name(),
        portfolio_name: config.portfolio.name,
        date,
        alipay_total_value,
        alipay_snapshot_date,
        unclassified_asset_count,
        reconciliation_issue_count,
        alipay_mismatch_count,
    })
}
