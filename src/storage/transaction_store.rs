use crate::models::Transaction;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_transactions<P: AsRef<Path>>(path: P) -> Result<Vec<Transaction>> {
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read transactions file at {:?}", path.as_ref()))?;
    let transactions: Vec<Transaction> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse transactions file at {:?}", path.as_ref()))?;
    Ok(transactions)
}
