use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub date: String,
    pub transaction_type: String, // "buy", "sell", "cash_in", "cash_out", "expense"
    pub asset_id: Option<String>,
    pub amount: f64,
    pub units: Option<f64>,
    pub price: Option<f64>,
    pub fee: f64,
    pub currency: String,
    pub note: String,
}
