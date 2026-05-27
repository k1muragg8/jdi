use crate::models::RiskCache;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_risk_cache<P: AsRef<Path>>(path: P) -> Result<Option<RiskCache>> {
    if !path.as_ref().exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read risk cache file: {:?}", path.as_ref()))?;
    let cache: RiskCache = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse risk cache JSON: {:?}", path.as_ref()))?;
    Ok(Some(cache))
}

pub fn save_risk_cache<P: AsRef<Path>>(path: P, cache: &RiskCache) -> Result<()> {
    let content = serde_json::to_string_pretty(cache).context("Failed to serialize risk cache")?;
    fs::write(path, content).context("Failed to write risk cache file")?;
    Ok(())
}
