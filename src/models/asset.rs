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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_instrument_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_instrument_symbol: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_index_currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_fx_pair: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_fx_adjustment: Option<bool>,
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            asset_id: "".to_string(),
            fund_code: "".to_string(),
            fund_name: "".to_string(),
            sector: "".to_string(),
            currency: "CNY".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: None,
            reference_index_symbol: None,
            market_data_provider: None,
            reference_instrument_id: None,
            reference_instrument_symbol: None,
            reference_index_currency: None,
            proxy_fx_pair: None,
            use_fx_adjustment: None,
        }
    }
}
