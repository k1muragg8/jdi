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
pub struct ApiConfig {
    #[serde(default = "default_fund_provider")]
    pub default_fund_provider: String,
    #[serde(default = "default_fund_provider_timeout")]
    pub fund_provider_timeout_seconds: u64,
    #[serde(default = "default_fund_provider_retry")]
    pub fund_provider_retry_count: u32,
    #[serde(default = "default_fund_nav_stale_days")]
    pub fund_nav_stale_days: i64,
    #[serde(default)]
    pub allow_mock_fallback: bool,
}

fn default_fund_provider() -> String {
    "eastmoney".to_string()
}
fn default_fund_provider_timeout() -> u64 {
    10
}
fn default_fund_provider_retry() -> u32 {
    2
}
fn default_fund_nav_stale_days() -> i64 {
    3
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            default_fund_provider: default_fund_provider(),
            fund_provider_timeout_seconds: default_fund_provider_timeout(),
            fund_provider_retry_count: default_fund_provider_retry(),
            fund_nav_stale_days: default_fund_nav_stale_days(),
            allow_mock_fallback: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConfig {
    #[serde(default = "default_market_provider")]
    pub default_market_provider: String,
    #[serde(default = "default_allow_mock_market_fallback")]
    pub allow_mock_market_fallback: bool,
    #[serde(default = "default_market_provider_timeout")]
    pub market_provider_timeout_seconds: u64,
    #[serde(default = "default_market_provider_retry")]
    pub market_provider_retry_count: u32,
    #[serde(default = "default_market_cache_stale_hours")]
    pub market_cache_stale_hours: i64,
}

fn default_market_provider() -> String {
    "mock".to_string()
}
fn default_allow_mock_market_fallback() -> bool {
    true
}
fn default_market_provider_timeout() -> u64 {
    10
}
fn default_market_provider_retry() -> u32 {
    2
}
fn default_market_cache_stale_hours() -> i64 {
    24
}

impl Default for MarketConfig {
    fn default() -> Self {
        Self {
            default_market_provider: default_market_provider(),
            allow_mock_market_fallback: default_allow_mock_market_fallback(),
            market_provider_timeout_seconds: default_market_provider_timeout(),
            market_provider_retry_count: default_market_provider_retry(),
            market_cache_stale_hours: default_market_cache_stale_hours(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxConfig {
    #[serde(default = "default_fx_provider")]
    pub default_fx_provider: String,
    #[serde(default = "default_usd_cnh_symbol")]
    pub usd_cnh_symbol: String,
    #[serde(default = "default_fx_cache_stale_hours")]
    pub fx_cache_stale_hours: i64,
    #[serde(default = "default_allow_mock_fx_fallback")]
    pub allow_mock_fx_fallback: bool,
}

fn default_fx_provider() -> String {
    "yahoo".to_string()
}
fn default_usd_cnh_symbol() -> String {
    "USDCNH=X".to_string()
}
fn default_fx_cache_stale_hours() -> i64 {
    24
}
fn default_allow_mock_fx_fallback() -> bool {
    true
}

impl Default for FxConfig {
    fn default() -> Self {
        Self {
            default_fx_provider: default_fx_provider(),
            usd_cnh_symbol: default_usd_cnh_symbol(),
            fx_cache_stale_hours: default_fx_cache_stale_hours(),
            allow_mock_fx_fallback: default_allow_mock_fx_fallback(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRoot {
    pub portfolio: PortfolioConfig,
    #[serde(default)]
    pub risk: RiskConfig,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub market: MarketConfig,
    #[serde(default)]
    pub fx: FxConfig,
    #[serde(default)]
    pub assets: Vec<AssetConfig>,
    #[serde(default)]
    pub sectors: Vec<SectorConfig>,
}
