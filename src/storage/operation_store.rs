use crate::models::{OperationPolicy, OperationStatus};
use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn load_operation_policy(path: &str) -> Result<OperationPolicy> {
    if !Path::new(path).exists() {
        return Ok(OperationPolicy::default());
    }
    let content = fs::read_to_string(path)?;
    let policy: OperationPolicy = serde_json::from_str(&content)?;
    Ok(policy)
}

pub fn save_operation_policy(path: &str, policy: &OperationPolicy) -> Result<()> {
    let content = serde_json::to_string_pretty(policy)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn load_operation_status(path: &str) -> Result<OperationStatus> {
    if !Path::new(path).exists() {
        return Ok(OperationStatus::default());
    }
    let content = fs::read_to_string(path)?;
    let status: OperationStatus = serde_json::from_str(&content)?;
    Ok(status)
}

pub fn save_operation_status(path: &str, status: &OperationStatus) -> Result<()> {
    let content = serde_json::to_string_pretty(status)?;
    fs::write(path, content)?;
    Ok(())
}
