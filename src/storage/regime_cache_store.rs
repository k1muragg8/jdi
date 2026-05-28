use crate::models::RegimeCache;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_regime_cache<P: AsRef<Path>>(path: P) -> Result<RegimeCache> {
    if !path.as_ref().exists() {
        return Ok(RegimeCache::default());
    }
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read regime cache file: {:?}", path.as_ref()))?;
    let cache: RegimeCache = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse regime cache JSON: {:?}", path.as_ref()))?;
    Ok(cache)
}

pub fn save_regime_cache<P: AsRef<Path>>(path: P, cache: &RegimeCache) -> Result<()> {
    let content =
        serde_json::to_string_pretty(cache).context("Failed to serialize regime cache")?;
    crate::storage::safe_write(path, content)?;
    Ok(())
}
