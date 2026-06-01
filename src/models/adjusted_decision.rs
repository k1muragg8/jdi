use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustedDecisionItem {
    pub sector: String,
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub benchmark_symbol: Option<String>,
    pub volatility: Option<f64>,
    pub base_suggested_buy: f64,
    pub regime_label: String,
    pub pendulum_score: f64,
    pub regime_multiplier: f64,
    pub global_risk_label: String,
    pub global_risk_score: f64,
    pub risk_multiplier: f64,
    pub kelly_multiplier: f64,
    pub data_quality_multiplier: f64,
    pub combined_multiplier: f64,
    pub adjusted_buy: f64,
    pub capped_adjusted_buy: f64,
    pub status: String,
    pub warnings: Vec<String>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustedDecisionPreview {
    pub available_cash: f64,
    pub target_equity_value: f64,
    pub current_equity_value: f64,
    pub equity_gap: f64,
    pub max_daily_buy: f64,
    pub base_total_buy: f64,
    pub adjusted_total_buy: f64,
    pub total_multiplier: f64,
    pub global_risk_score: f64,
    pub global_risk_label: String,
    pub items: Vec<AdjustedDecisionItem>,
    pub warnings: Vec<String>,
}
