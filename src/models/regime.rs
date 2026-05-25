use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketRegimeResult {
    pub symbol: String,
    pub latest_price: f64,
    pub latest_date: String,
    pub source: String,
    pub windows: Vec<CycleWindowStats>,
    pub pendulum_score: f64,
    pub regime_label: String,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleWindowStats {
    pub window_days: usize,
    pub moving_average: f64,
    pub price_stddev: f64,
    pub daily_return_stddev: f64,
    pub annualized_volatility: f64,
    pub z_score: Option<f64>,
    pub rolling_high: f64,
    pub drawdown: f64,
    pub cumulative_return: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendulumScore {
    pub score: f64,
    pub label: String,
    pub explanation: String,
}
