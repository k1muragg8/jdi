use crate::models::market::FxCache;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_fx_cache(path: &str) -> Result<FxCache> {
    if !Path::new(path).exists() {
        return Ok(FxCache::default());
    }
    let content =
        fs::read_to_string(path).context(format!("Failed to read FX cache file: {}", path))?;
    let cache: FxCache = serde_json::from_str(&content)
        .context(format!("Failed to parse FX cache file: {}", path))?;
    Ok(cache)
}

pub fn save_fx_cache<P: AsRef<Path>>(path: P, cache: &FxCache) -> Result<()> {
    let content = serde_json::to_string_pretty(cache).context("Failed to serialize FX cache")?;
    crate::storage::safe_write(path, content)?;
    Ok(())
}
