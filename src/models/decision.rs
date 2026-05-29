use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionExplanation {
    pub date: String,
    pub portfolio_id: String,
    pub base_currency: String,
    pub available_cash: f64,
    pub daily_budget: f64,
    pub target_equity_value: f64,
    pub current_equity_value: f64,
    pub equity_gap: f64,
    pub risk_summary: RiskAdjustmentExplanation,
    pub asset_explanations: Vec<AssetDecisionExplanation>,
    pub sector_explanations: Vec<SectorAllocationExplanation>,
    pub warnings: Vec<String>,
    pub global_caps: Vec<CapExplanation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDecisionExplanation {
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub sector_id: String,
    pub status: String,

    // Allocation factors
    pub base_suggested_buy: f64,
    pub adjusted_suggested_buy: f64,
    pub final_suggested_buy: f64,

    // Multipliers
    pub regime_adjustment: RegimeAdjustmentExplanation,
    pub risk_adjustment: RiskAdjustmentExplanation,
    pub kelly_adjustment: KellyAdjustmentExplanation,
    pub data_quality_multiplier: f64,

    // Limits
    pub caps: Vec<CapExplanation>,
    pub skip_reason: Option<String>,

    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorAllocationExplanation {
    pub sector_id: String,
    pub sector_name: String,
    pub target_weight: f64,
    pub current_weight: f64,
    pub target_value: f64,
    pub current_value: f64,
    pub gap_value: f64,
    pub priority: i32,
    pub allocated_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAdjustmentExplanation {
    pub score: f64,
    pub label: String,
    pub multiplier: f64,
    pub factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeAdjustmentExplanation {
    pub score: f64,
    pub label: String,
    pub multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KellyAdjustmentExplanation {
    pub win_probability: f64,
    pub payoff_ratio: f64,
    pub raw_kelly: f64,
    pub adjusted_kelly: f64,
    pub multiplier: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapExplanation {
    pub name: String,
    pub limit_value: f64,
    pub applied: bool,
    pub description: String,
}
