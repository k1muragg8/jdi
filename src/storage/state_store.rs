use crate::models::PortfolioState;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_state<P: AsRef<Path>>(path: P) -> Result<PortfolioState> {
    if !path.as_ref().exists() {
        return Ok(PortfolioState::default());
    }
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read state file at {:?}", path.as_ref()))?;
    let state: PortfolioState = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse state file at {:?}", path.as_ref()))?;
    Ok(state)
}

pub fn save_state<P: AsRef<Path>>(path: P, state: &PortfolioState) -> Result<()> {
    let content = serde_json::to_string_pretty(state)
        .with_context(|| "Failed to serialize portfolio state")?;
    crate::storage::safe_write(path.as_ref(), content)?;
    Ok(())
}
