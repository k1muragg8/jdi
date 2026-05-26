use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AssetClass {
    SpotCommodity,
    Futures,
    Index,
    Etf,
    Fx,
    Crypto,
    Rate,
    Fund,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentConfig {
    pub instrument_id: String,
    pub symbol: String,
    pub display_symbol: Option<String>,
    pub name: String,
    pub asset_class: AssetClass,
    pub provider: String,
    pub provider_symbol: String,
    pub market: Option<String>,
    pub exchange: Option<String>,
    pub currency: String,
    pub quote_unit: String,
    pub price_unit: String,
    pub timezone: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub tags: Vec<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentRegistry {
    pub instruments: Vec<InstrumentConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentQuote {
    pub instrument_id: String,
    pub symbol: String,
    pub name: String,
    pub asset_class: AssetClass,
    pub latest_price: f64,
    pub latest_date: String,
    pub currency: String,
    pub quote_unit: String,
    pub provider: String,
    pub source: String,
    pub status: String,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentCandle {
    pub instrument_id: String,
    pub symbol: String,
    pub date: String,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: f64,
    pub volume: Option<f64>,
    pub source: String,
}
