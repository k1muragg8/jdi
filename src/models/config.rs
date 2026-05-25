use super::asset::AssetConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioConfig {
    pub name: String,
    pub base_currency: String,
    pub target_equity_value: f64,
    pub reserve_cash: f64,
    pub upcoming_expense: f64,
    pub max_daily_buy_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorConfig {
    pub sector_id: String,
    pub name: String,
    pub asset_class: String,
    pub target_weight: f64,
    pub priority: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    #[serde(default = "default_max_single_sector_daily_buy")]
    pub max_single_sector_daily_buy: f64,
    #[serde(default = "default_max_single_asset_daily_buy")]
    pub max_single_asset_daily_buy: f64,
    #[serde(default = "default_min_buy_amount")]
    pub min_buy_amount: f64,
    #[serde(default)]
    pub allow_buy_overweight: bool,
}

fn default_max_single_sector_daily_buy() -> f64 {
    1500.0
}
fn default_max_single_asset_daily_buy() -> f64 {
    1000.0
}
fn default_min_buy_amount() -> f64 {
    10.0
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_single_sector_daily_buy: default_max_single_sector_daily_buy(),
            max_single_asset_daily_buy: default_max_single_asset_daily_buy(),
            min_buy_amount: default_min_buy_amount(),
            allow_buy_overweight: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRoot {
    pub portfolio: PortfolioConfig,
    #[serde(default)]
    pub risk: RiskConfig,
    #[serde(default)]
    pub assets: Vec<AssetConfig>,
    #[serde(default)]
    pub sectors: Vec<SectorConfig>,
}
