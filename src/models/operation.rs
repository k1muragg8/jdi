use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationPolicy {
    #[serde(default)]
    pub target_total_investment_amount: Option<f64>,
    pub target_equity_weight: f64,
    pub min_cash_reserve: f64,
    pub max_daily_buy_amount: f64,
    pub max_single_asset_buy_amount: f64,
    pub max_single_asset_weight: f64,
    pub max_sector_weight: f64,
    #[serde(default)]
    pub target_asset_weights: HashMap<String, f64>,
    #[serde(default)]
    pub target_sector_weights: HashMap<String, f64>,
    #[serde(default)]
    pub dca_auto_pause_when_target_reached: bool,
    #[serde(default)]
    pub dca_auto_resume_when_below_target: bool,
    pub dca_resume_threshold: f64, // e.g., 0.95 of target
    pub dca_pause_threshold: f64,  // e.g., 1.05 of target
    #[serde(default)]
    pub kelly_enabled: bool,
    pub max_kelly_fraction: f64,
    #[serde(default)]
    pub pendulum_enabled: bool,
    pub volatility_window_days: usize,
    #[serde(default)]
    pub risk_overlay_enabled: bool,
    #[serde(default = "default_market_refresh_interval")]
    pub market_refresh_interval_seconds: u64,
}

fn default_market_refresh_interval() -> u64 {
    180
}

impl Default for OperationPolicy {
    fn default() -> Self {
        Self {
            target_total_investment_amount: None,
            target_equity_weight: 0.8,
            min_cash_reserve: 10000.0,
            max_daily_buy_amount: 3000.0,
            max_single_asset_buy_amount: 1000.0,
            max_single_asset_weight: 0.15,
            max_sector_weight: 0.3,
            target_asset_weights: HashMap::new(),
            target_sector_weights: HashMap::new(),
            dca_auto_pause_when_target_reached: true,
            dca_auto_resume_when_below_target: true,
            dca_resume_threshold: 0.95,
            dca_pause_threshold: 1.05,
            kelly_enabled: true,
            max_kelly_fraction: 0.25,
            pendulum_enabled: true,
            volatility_window_days: 20,
            risk_overlay_enabled: true,
            market_refresh_interval_seconds: 180,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationReport {
    pub date: String,
    pub timestamp: String,
    pub portfolio_id: String,
    pub portfolio_name: String,
    pub total_value: f64,
    pub cash_value: f64,
    pub equity_value: f64,
    pub current_equity_weight: f64,
    pub target_equity_weight: f64,
    pub equity_gap: f64,
    pub dca_execution_result: super::dca::DcaExecutionResult,
    pub suggestions: Vec<OperationSuggestion>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationSuggestion {
    pub asset_id: String,
    pub fund_name: String,
    pub fund_code: String,
    pub benchmark_symbol: Option<String>,
    pub benchmark_return: f64,
    pub volatility: f64,
    pub pendulum_score: f64,
    pub regime_label: String,
    pub current_weight: f64,
    pub target_weight: f64,
    pub allocation_gap: f64,
    pub suggested_amount: f64,
    pub kelly_adjusted_amount: f64,
    pub kelly_multiplier: f64,
    pub caps_applied: String,
    pub status: String, // "execute", "skip", "pause", "resume"
    pub reason: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OperationStatus {
    pub last_run_at: Option<String>,
    pub last_report: Option<OperationReport>,
    pub policy: OperationPolicy,
    pub is_running: bool,
}
