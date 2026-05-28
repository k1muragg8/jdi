use crate::models::ProxyValuationCache;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_proxy_cache<P: AsRef<Path>>(path: P) -> Result<Option<ProxyValuationCache>> {
    if !path.as_ref().exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read proxy cache file: {:?}", path.as_ref()))?;
    let cache: ProxyValuationCache = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse proxy cache JSON: {:?}", path.as_ref()))?;
    Ok(Some(cache))
}

pub fn save_proxy_cache<P: AsRef<Path>>(path: P, cache: &ProxyValuationCache) -> Result<()> {
    let content = serde_json::to_string_pretty(cache).context("Failed to serialize proxy cache")?;
    crate::storage::safe_write(path, content)?;
    Ok(())
}
