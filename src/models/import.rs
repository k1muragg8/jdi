use crate::models::Transaction;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedTransactionCandidate {
    pub date: String,
    pub transaction_type: String, // buy, sell, dividend, cash_in, cash_out, fee, unknown
    pub asset_id: Option<String>,
    pub asset_name: Option<String>,
    pub amount: f64,
    pub units: Option<f64>,
    pub price: Option<f64>,
    pub fee: f64,
    pub currency: String,
    pub source: String,
    pub note: String,
    pub external_id: Option<String>,
    pub raw_description: String,
}

impl ImportedTransactionCandidate {
    pub fn to_transaction(&self) -> Transaction {
        Transaction {
            id: self
                .external_id
                .clone()
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            date: self.date.clone(),
            transaction_type: self.transaction_type.clone(),
            asset_id: self.asset_id.clone(),
            amount: self.amount,
            units: self.units,
            price: self.price,
            fee: self.fee,
            currency: self.currency.clone(),
            note: self.note.clone(),
            source: self.source.clone(),
            raw_description: self.raw_description.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionImportPreview {
    pub candidates: Vec<ImportedTransactionCandidate>,
    pub duplicates: Vec<bool>,      // indices matching candidates
    pub warnings: Vec<Vec<String>>, // indices matching candidates
    pub errors: Vec<Vec<String>>,   // indices matching candidates
    pub summary: ImportSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportSummary {
    pub total_rows: usize,
    pub valid_rows: usize,
    pub error_rows: usize,
    pub warning_rows: usize,
    pub duplicate_rows: usize,
    pub new_rows: usize,
}
