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
