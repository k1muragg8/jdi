use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetHolding {
    pub asset_id: String,
    pub fund_code: String,
    pub units: f64,
    pub units_estimated: bool,
    pub cost_basis: f64,
    pub latest_nav: Option<f64>,
    pub latest_nav_date: Option<String>,
    pub last_market_value: f64,
}
