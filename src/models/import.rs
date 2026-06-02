use crate::models::Transaction;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedTransactionCandidate {
    #[serde(alias = "交易日期", alias = "date")]
    pub date: String,
    #[serde(alias = "交易类型", alias = "type", alias = "transaction_type")]
    pub transaction_type: String, // buy, sell, dividend, cash_in, cash_out, fee, unknown
    #[serde(alias = "资产代码", alias = "资产ID", alias = "asset_id")]
    pub asset_id: Option<String>,
    #[serde(alias = "资产名称", alias = "基金名称", alias = "asset_name")]
    pub asset_name: Option<String>,
    #[serde(alias = "金额", alias = "成交金额", alias = "amount")]
    pub amount: f64,
    #[serde(alias = "份额", alias = "成交份额", alias = "units")]
    pub units: Option<f64>,
    #[serde(alias = "价格", alias = "成交价格", alias = "price")]
    pub price: Option<f64>,
    #[serde(alias = "手续费", alias = "fee")]
    pub fee: f64,
    #[serde(alias = "币种", alias = "currency")]
    pub currency: String,
    #[serde(alias = "来源", alias = "source")]
    pub source: String,
    #[serde(alias = "备注", alias = "note")]
    pub note: String,
    #[serde(alias = "外部ID", alias = "external_id")]
    pub external_id: Option<String>,
    #[serde(alias = "原始描述", alias = "raw_description")]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportResult {
    pub inserted: usize,
    pub skipped: usize,
    pub failed: usize,
    pub success: bool,
    pub message: String,
}
