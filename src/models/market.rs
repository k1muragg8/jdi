use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketPrice {
    pub symbol: String,
    pub price: f64,
    pub date: String,
    pub currency: String,
    pub source: String,
    pub is_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub symbol: String,
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketCacheEntry {
    pub symbol: String,
    pub price: f64,
    pub date: String,
    pub currency: String,
    pub source: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxRate {
    pub pair: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub rate: f64,
    pub date: String,
    pub source: String,
    pub is_stale: bool,
    pub is_estimated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxCacheEntry {
    pub pair: String,
    pub rate: f64,
    pub date: String,
    pub source: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FxCache {
    pub entries: Vec<FxCacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketCache {
    pub entries: Vec<MarketCacheEntry>,
}
