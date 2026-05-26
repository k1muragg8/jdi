use crate::models::{AlipaySnapshot, ReconciliationAudit};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_alipay_snapshots<P: AsRef<Path>>(path: P) -> Result<Vec<AlipaySnapshot>> {
    if !path.as_ref().exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path.as_ref()).with_context(|| {
        format!(
            "Failed to read Alipay snapshots file at {:?}",
            path.as_ref()
        )
    })?;
    let snapshots: Vec<AlipaySnapshot> = serde_json::from_str(&content).with_context(|| {
        format!(
            "Failed to parse Alipay snapshots file at {:?}",
            path.as_ref()
        )
    })?;
    Ok(snapshots)
}

pub fn save_alipay_snapshots<P: AsRef<Path>>(path: P, snapshots: &[AlipaySnapshot]) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    let content = serde_json::to_string_pretty(snapshots)
        .with_context(|| "Failed to serialize Alipay snapshots")?;
    fs::write(path.as_ref(), content).with_context(|| {
        format!(
            "Failed to write Alipay snapshots file to {:?}",
            path.as_ref()
        )
    })?;
    Ok(())
}

pub fn load_reconciliation_audits<P: AsRef<Path>>(path: P) -> Result<Vec<ReconciliationAudit>> {
    if !path.as_ref().exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path.as_ref()).with_context(|| {
        format!(
            "Failed to read reconciliation audits file at {:?}",
            path.as_ref()
        )
    })?;
    let audits: Vec<ReconciliationAudit> = serde_json::from_str(&content).with_context(|| {
        format!(
            "Failed to parse reconciliation audits file at {:?}",
            path.as_ref()
        )
    })?;
    Ok(audits)
}

pub fn save_reconciliation_audits<P: AsRef<Path>>(
    path: P,
    audits: &[ReconciliationAudit],
) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    let content = serde_json::to_string_pretty(audits)
        .with_context(|| "Failed to serialize reconciliation audits")?;
    fs::write(path.as_ref(), content).with_context(|| {
        format!(
            "Failed to write reconciliation audits file to {:?}",
            path.as_ref()
        )
    })?;
    Ok(())
}
