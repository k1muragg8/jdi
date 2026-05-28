use crate::models::PortfolioSnapshot;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_snapshots<P: AsRef<Path>>(path: P) -> Result<Vec<PortfolioSnapshot>> {
    if !path.as_ref().exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read snapshots file at {:?}", path.as_ref()))?;
    let snapshots: Vec<PortfolioSnapshot> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse snapshots JSON at {:?}", path.as_ref()))?;
    Ok(snapshots)
}

pub fn save_snapshots<P: AsRef<Path>>(path: P, snapshots: &[PortfolioSnapshot]) -> Result<()> {
    let content =
        serde_json::to_string_pretty(snapshots).context("Failed to serialize snapshots")?;
    crate::storage::safe_write(path, content)?;
    Ok(())
}
