use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundInfo {
    pub fund_code: String,
    pub fund_name: String,
    pub fund_type: String,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundNav {
    pub fund_code: String,
    pub nav: f64,
    pub nav_date: String,
    pub currency: String,
    pub source: String,
}
