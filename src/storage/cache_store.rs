use crate::models::NavCache;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_cache(path: &str) -> Result<NavCache> {
    if !Path::new(path).exists() {
        return Ok(NavCache::default());
    }
    let content =
        fs::read_to_string(path).context(format!("Failed to read cache file: {}", path))?;
    let cache: NavCache =
        serde_json::from_str(&content).context(format!("Failed to parse cache file: {}", path))?;
    Ok(cache)
}

pub fn save_cache(path: &str, cache: &NavCache) -> Result<()> {
    let content = serde_json::to_string_pretty(cache).context("Failed to serialize cache")?;
    fs::write(path, content).context(format!("Failed to write cache file: {}", path))?;
    Ok(())
}
