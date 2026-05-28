use crate::models::CacheStatusRegistry;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_cache_status<P: AsRef<Path>>(path: P) -> Result<CacheStatusRegistry> {
    if !path.as_ref().exists() {
        return Ok(CacheStatusRegistry::default());
    }
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read cache status file: {:?}", path.as_ref()))?;
    let registry: CacheStatusRegistry = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse cache status JSON: {:?}", path.as_ref()))?;
    Ok(registry)
}

pub fn save_cache_status<P: AsRef<Path>>(path: P, registry: &CacheStatusRegistry) -> Result<()> {
    let content =
        serde_json::to_string_pretty(registry).context("Failed to serialize cache status")?;
    crate::storage::safe_write(path, content)?;
    Ok(())
}
