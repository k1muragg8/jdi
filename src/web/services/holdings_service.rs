//! Holdings page data (repository only; no HTML).

use crate::engine;
use crate::models::{AlipaySnapshot, ConfigRoot, PortfolioState, PortfolioSummary};
use crate::web::state::AppState;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

pub struct HoldingsPageData {
    pub config: ConfigRoot,
    pub portfolio_state: PortfolioState,
    pub summary: PortfolioSummary,
    pub latest_snaps: HashMap<String, AlipaySnapshot>,
}

pub async fn load_holdings_page(state: &Arc<AppState>) -> Result<HoldingsPageData> {
    let ctx = &state.ctx;
    let config = state.repo.load_config(ctx).await?;
    let portfolio_state = state.repo.load_state(ctx).await?;
    let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);
    let snapshots = state
        .repo
        .load_alipay_snapshots(ctx)
        .await
        .unwrap_or_default();

    let mut latest_snaps: HashMap<String, AlipaySnapshot> = HashMap::new();
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

    Ok(HoldingsPageData {
        config,
        portfolio_state,
        summary,
        latest_snaps,
    })
}
