use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyValuationResult {
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub sector: String,
    pub units: f64,
    pub official_nav: f64,
    pub official_nav_date: String,
    pub official_market_value: f64,
    pub reference_index_name: String,
    pub reference_index_symbol: String,
    pub reference_price_on_nav_date: f64,
    pub reference_latest_price: f64,
    pub reference_latest_date: String,
    pub proxy_return: f64,
    pub index_return: f64,
    pub fx_return: f64,
    pub combined_proxy_return: f64,
    pub use_fx_adjustment: bool,
    pub estimated_nav: f64,
    pub estimated_market_value: f64,
    pub estimated_pnl: f64,
    pub data_source: String,
    pub status: String,
    pub warning: Option<String>,
}
