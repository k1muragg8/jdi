//! Market watchlist page data (repository + cache; no HTML).

use crate::engine;
use crate::models::{CacheStatusRegistry, InstrumentConfig, MarketCache};
use crate::web::state::AppState;
use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;

pub struct MarketPageData {
    pub instruments: Vec<InstrumentConfig>,
    pub market_cache: MarketCache,
    pub cache_status: CacheStatusRegistry,
    pub dup_ids: HashSet<String>,
    pub cleanup_confirm_msg: String,
}

pub async fn load_market_page(state: &Arc<AppState>) -> Result<MarketPageData> {
    let ctx = &state.ctx;
    let _ = state.repo.load_config(ctx).await?;
    let cache_status = state.repo.load_cache_status(ctx).await.unwrap_or_default();
    let instruments = state.repo.load_instruments(ctx).await?;
    let market_cache = state.repo.load_market_cache(ctx).await.unwrap_or_default();
    let dup_ids: HashSet<String> = engine::duplicate_instrument_ids(&instruments)
        .into_iter()
        .collect();
    let mut inst_preview = instruments.clone();
    let test_pending_count = engine::cleanup_test_instruments(&mut inst_preview, true);
    let cleanup_confirm_msg = format!("检测到 {} 个测试标的，是否归档？", test_pending_count);
    Ok(MarketPageData {
        instruments,
        market_cache,
        cache_status,
        dup_ids,
        cleanup_confirm_msg,
    })
}
