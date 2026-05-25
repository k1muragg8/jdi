use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundInfo {
    pub fund_code: String,
    pub fund_name: String,
    pub fund_type: String,
    pub currency: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundNav {
    pub fund_code: String,
    pub nav: f64,
    pub accumulated_nav: Option<f64>,
    pub nav_date: String,
    pub currency: String,
    pub source: String,
    pub is_stale: bool,
    pub is_estimated: bool,
}
