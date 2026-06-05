//! Shared form/query types for web POST handlers.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct AssetIdForm {
    pub asset_id: String,
    pub filter: Option<String>,
}

#[derive(Deserialize)]
pub struct CashSetForm {
    pub amount: f64,
}

#[derive(Deserialize)]
pub struct CashAdjustForm {
    pub amount: f64,
}
