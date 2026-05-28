use crate::models::WebAdminAuditLog;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_web_audit<P: AsRef<Path>>(path: P) -> Result<WebAdminAuditLog> {
    if !path.as_ref().exists() {
        return Ok(WebAdminAuditLog::default());
    }
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read web audit file at {:?}", path.as_ref()))?;
    let log: WebAdminAuditLog = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse web audit JSON at {:?}", path.as_ref()))?;
    Ok(log)
}

pub fn save_web_audit<P: AsRef<Path>>(path: P, log: &WebAdminAuditLog) -> Result<()> {
    let content = serde_json::to_string_pretty(&log).context("Failed to serialize audit log")?;
    crate::storage::safe_write(path.as_ref(), content)?;
    Ok(())
}

pub fn add_audit_record<P: AsRef<Path>>(
    path: P,
    record: crate::models::WebAdminAuditRecord,
) -> Result<()> {
    let mut log = load_web_audit(&path)?;
    log.records.push(record);
    // Keep only last 1000 records for performance
    if log.records.len() > 1000 {
        log.records.drain(0..log.records.len() - 1000);
    }
    save_web_audit(path, &log)
}
