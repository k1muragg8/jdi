use crate::models::InstrumentQuoteCache;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_instrument_cache<P: AsRef<Path>>(path: P) -> Result<InstrumentQuoteCache> {
    if !path.as_ref().exists() {
        return Ok(InstrumentQuoteCache::default());
    }
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read instrument cache file: {:?}", path.as_ref()))?;
    let cache: InstrumentQuoteCache = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse instrument cache JSON: {:?}", path.as_ref()))?;
    Ok(cache)
}

pub fn save_instrument_cache<P: AsRef<Path>>(path: P, cache: &InstrumentQuoteCache) -> Result<()> {
    let content =
        serde_json::to_string_pretty(cache).context("Failed to serialize instrument cache")?;
    fs::write(path, content).context("Failed to write instrument cache file")?;
    Ok(())
}
