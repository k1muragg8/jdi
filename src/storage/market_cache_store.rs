use crate::models::MarketCache;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_market_cache(path: &str) -> Result<MarketCache> {
    if !Path::new(path).exists() {
        return Ok(MarketCache::default());
    }
    let content =
        fs::read_to_string(path).context(format!("Failed to read market cache file: {}", path))?;
    let cache: MarketCache = serde_json::from_str(&content)
        .context(format!("Failed to parse market cache file: {}", path))?;
    Ok(cache)
}

pub fn save_market_cache<P: AsRef<Path>>(path: P, cache: &MarketCache) -> Result<()> {
    let content =
        serde_json::to_string_pretty(cache).context("Failed to serialize market cache")?;
    crate::storage::safe_write(path, content)?;
    Ok(())
}
