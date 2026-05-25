use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactorSnapshot {
    pub name: String,
    pub symbol: String,
    pub latest_value: f64,
    pub latest_date: String,
    pub source: String,
    pub status: String,
    pub short_return: f64,    // 20d
    pub medium_return: f64,   // 60d
    pub z_score: Option<f64>, // 250d
    pub drawdown: f64,        // 250d
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalRiskOverlay {
    pub risk_score: f64,
    pub risk_label: String,
    pub factor_results: Vec<RiskFactorSnapshot>,
    pub warnings: Vec<String>,
    pub explanation: String,
}
