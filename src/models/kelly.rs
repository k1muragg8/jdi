use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KellyPreviewResult {
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub sector: String,
    pub base_suggested_buy: f64,
    pub pendulum_score: f64,
    pub market_regime_label: String,
    pub global_risk_score: f64,
    pub global_risk_label: String,
    pub volatility: f64,
    pub drawdown: f64,
    pub expected_edge: f64,
    pub estimated_win_probability: f64,
    pub payoff_ratio: f64,
    pub raw_kelly_fraction: f64,
    pub fractional_kelly_fraction: f64,
    pub kelly_multiplier: f64,
    pub preview_buy_amount: f64,
    pub capped_preview_buy_amount: f64,
    pub confidence: f64,
    pub status: String,
    pub warnings: Vec<String>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KellyPortfolioPreview {
    pub base_total_buy: f64,
    pub preview_total_buy: f64,
    pub total_multiplier: f64,
    pub global_risk_score: f64,
    pub global_risk_label: String,
    pub results: Vec<KellyPreviewResult>,
    pub warnings: Vec<String>,
}
