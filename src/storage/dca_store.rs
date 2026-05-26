use crate::models::{DcaPlan, DcaSettlement, DcaSettlementAudit};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_dca_plans<P: AsRef<Path>>(path: P) -> Result<Vec<DcaPlan>> {
    if !path.as_ref().exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read DCA plans file at {:?}", path.as_ref()))?;
    let plans: Vec<DcaPlan> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse DCA plans file at {:?}", path.as_ref()))?;
    Ok(plans)
}

pub fn save_dca_plans<P: AsRef<Path>>(path: P, plans: &[DcaPlan]) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    let content =
        serde_json::to_string_pretty(plans).with_context(|| "Failed to serialize DCA plans")?;
    fs::write(path.as_ref(), content)
        .with_context(|| format!("Failed to write DCA plans file to {:?}", path.as_ref()))?;
    Ok(())
}

pub fn load_dca_settlements<P: AsRef<Path>>(path: P) -> Result<Vec<DcaSettlement>> {
    if !path.as_ref().exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read DCA settlements file at {:?}", path.as_ref()))?;
    let settlements: Vec<DcaSettlement> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse DCA settlements file at {:?}", path.as_ref()))?;
    Ok(settlements)
}

pub fn save_dca_settlements<P: AsRef<Path>>(path: P, settlements: &[DcaSettlement]) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    let content = serde_json::to_string_pretty(settlements)
        .with_context(|| "Failed to serialize DCA settlements")?;
    fs::write(path.as_ref(), content).with_context(|| {
        format!("Failed to write DCA settlements file to {:?}", path.as_ref())
    })?;
    Ok(())
}

pub fn load_dca_settlement_audits<P: AsRef<Path>>(path: P) -> Result<Vec<DcaSettlementAudit>> {
    if !path.as_ref().exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path.as_ref()).with_context(|| {
        format!(
            "Failed to read DCA settlement audits file at {:?}",
            path.as_ref()
        )
    })?;
    let audits: Vec<DcaSettlementAudit> = serde_json::from_str(&content).with_context(|| {
        format!(
            "Failed to parse DCA settlement audits file at {:?}",
            path.as_ref()
        )
    })?;
    Ok(audits)
}

pub fn save_dca_settlement_audits<P: AsRef<Path>>(
    path: P,
    audits: &[DcaSettlementAudit],
) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    let content = serde_json::to_string_pretty(audits)
        .with_context(|| "Failed to serialize DCA settlement audits")?;
    fs::write(path.as_ref(), content).with_context(|| {
        format!(
            "Failed to write DCA settlement audits file to {:?}",
            path.as_ref()
        )
    })?;
    Ok(())
}
