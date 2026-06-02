use super::holding::AssetHolding;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Portfolio {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_user_id: String,
    pub current_cash: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PortfolioState {
    pub cash: f64,
    #[serde(default)]
    pub asset_holdings: Vec<AssetHolding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorSummary {
    pub sector_id: String,
    pub sector_name: String,
    pub asset_class: String,
    pub target_weight: f64,
    pub target_value: f64,
    pub current_value: f64,
    pub current_weight: f64,
    pub gap_value: f64,
    pub gap_ratio: f64,
    pub priority: i32,
    pub enabled: bool,
    pub status: String, // "underweight", "neutral", "overweight", "disabled"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PortfolioSummary {
    pub cash: f64,
    pub equity_value: f64,
    pub bond_value: f64,
    pub crypto_value: f64,
    pub fund_value: f64,
    pub total_asset_value: f64,
    pub target_equity_value: f64,
    pub equity_gap: f64,
    pub available_cash: f64,
    pub current_weight: f64,
    pub reserve_cash: f64,
    pub upcoming_expense: f64,
    pub sector_summaries: Vec<SectorSummary>,
}
