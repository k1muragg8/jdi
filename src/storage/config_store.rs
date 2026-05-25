use crate::models::ConfigRoot;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<ConfigRoot> {
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read config file at {:?}", path.as_ref()))?;
    let config: ConfigRoot = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file at {:?}", path.as_ref()))?;
    Ok(config)
}
