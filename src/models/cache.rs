use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavCacheEntry {
    pub fund_code: String,
    pub nav: f64,
    pub accumulated_nav: Option<f64>,
    pub nav_date: String,
    pub currency: String,
    pub source: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NavCache {
    pub entries: Vec<NavCacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatus {
    pub key: String,
    pub source: String,
    pub last_updated_at: String,
    pub data_date: Option<String>,
    pub status: String, // 正常, 过期, 缺失, 获取失败, 模拟
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStatusRegistry {
    pub statuses: Vec<CacheStatus>,
    pub market_cache_size: usize,
    pub last_market_update: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskCache {
    pub overlay: super::risk_overlay::GlobalRiskOverlay,
    pub fetched_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeCacheEntry {
    pub symbol: String,
    pub result: super::regime::MarketRegimeResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeCache {
    pub entries: Vec<RegimeCacheEntry>,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyValuationCache {
    pub results: Vec<super::valuation::ProxyValuationResult>,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentQuoteCacheEntry {
    pub instrument_id: String,
    pub symbol: String,
    pub name_zh: Option<String>,
    pub price: f64,
    pub date: String,
    pub currency: String,
    pub quote_unit: String,
    pub provider: String,
    pub source: String,
    pub status: String,
    pub fetched_at: String,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstrumentQuoteCache {
    pub entries: Vec<InstrumentQuoteCacheEntry>,
    pub fetched_at: String,
}
