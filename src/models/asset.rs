use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetConfig {
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub sector: String,
    pub currency: String,
    pub valuation_method: String,
    pub enabled: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_index_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_index_symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_data_provider: Option<String>,
}
