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

impl Default for PortfolioConfig {
    fn default() -> Self {
        Self {
            name: "Default Portfolio".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 1000.0,
        }
    }
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

    // Global Risk Overlay Settings
    #[serde(default = "default_vix_symbol")]
    pub vix_symbol: String,
    #[serde(default = "default_us30y_symbol")]
    pub us30y_symbol: String,
    #[serde(default = "default_crypto_symbols")]
    pub crypto_symbols: Vec<String>,
    #[serde(default = "default_equity_symbols")]
    pub equity_symbols: Vec<String>,
    #[serde(default = "default_risk_lookback_days")]
    pub lookback_days: usize,
    #[serde(default = "default_short_window_days")]
    pub short_window_days: usize,
    #[serde(default = "default_medium_window_days")]
    pub medium_window_days: usize,
    #[serde(default = "default_high_vix_threshold")]
    pub high_vix_threshold: f64,
    #[serde(default = "default_extreme_vix_threshold")]
    pub extreme_vix_threshold: f64,
    #[serde(default = "default_us30y_fast_rise_bps_60d")]
    pub us30y_fast_rise_bps_60d: f64,
    #[serde(default = "default_crypto_drawdown_warning")]
    pub crypto_drawdown_warning: f64,
    #[serde(default = "default_risk_score_warning_threshold")]
    pub risk_score_warning_threshold: f64,
    #[serde(default = "default_risk_score_extreme_threshold")]
    pub risk_score_extreme_threshold: f64,
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
fn default_vix_symbol() -> String {
    "^VIX".to_string()
}
fn default_us30y_symbol() -> String {
    "^TYX".to_string()
}
fn default_crypto_symbols() -> Vec<String> {
    vec![
        "BTC-USD".to_string(),
        "ETH-USD".to_string(),
        "SOL-USD".to_string(),
    ]
}
fn default_equity_symbols() -> Vec<String> {
    vec!["QQQ".to_string(), "SPY".to_string()]
}
fn default_risk_lookback_days() -> usize {
    250
}
fn default_short_window_days() -> usize {
    20
}
fn default_medium_window_days() -> usize {
    60
}
fn default_high_vix_threshold() -> f64 {
    25.0
}
fn default_extreme_vix_threshold() -> f64 {
    35.0
}
fn default_us30y_fast_rise_bps_60d() -> f64 {
    50.0
}
fn default_crypto_drawdown_warning() -> f64 {
    -0.20
}
fn default_risk_score_warning_threshold() -> f64 {
    60.0
}
fn default_risk_score_extreme_threshold() -> f64 {
    80.0
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_single_sector_daily_buy: default_max_single_sector_daily_buy(),
            max_single_asset_daily_buy: default_max_single_asset_daily_buy(),
            min_buy_amount: default_min_buy_amount(),
            allow_buy_overweight: false,
            vix_symbol: default_vix_symbol(),
            us30y_symbol: default_us30y_symbol(),
            crypto_symbols: default_crypto_symbols(),
            equity_symbols: default_equity_symbols(),
            lookback_days: default_risk_lookback_days(),
            short_window_days: default_short_window_days(),
            medium_window_days: default_medium_window_days(),
            high_vix_threshold: default_high_vix_threshold(),
            extreme_vix_threshold: default_extreme_vix_threshold(),
            us30y_fast_rise_bps_60d: default_us30y_fast_rise_bps_60d(),
            crypto_drawdown_warning: default_crypto_drawdown_warning(),
            risk_score_warning_threshold: default_risk_score_warning_threshold(),
            risk_score_extreme_threshold: default_risk_score_extreme_threshold(),
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
pub struct RegimeConfig {
    #[serde(default = "default_windows")]
    pub default_windows: Vec<usize>,
    #[serde(default = "default_lookback_days")]
    pub default_lookback_days: usize,
    #[serde(default = "default_hot_z_threshold")]
    pub hot_z_threshold: f64,
    #[serde(default = "default_cold_z_threshold")]
    pub cold_z_threshold: f64,
    #[serde(default = "default_high_volatility_threshold")]
    pub high_volatility_threshold: f64,
    #[serde(default = "default_deep_drawdown_threshold")]
    pub deep_drawdown_threshold: f64,
}

fn default_windows() -> Vec<usize> {
    vec![20, 60, 120, 250]
}
fn default_lookback_days() -> usize {
    250
}
fn default_hot_z_threshold() -> f64 {
    2.0
}
fn default_cold_z_threshold() -> f64 {
    -2.0
}
fn default_high_volatility_threshold() -> f64 {
    0.35
}
fn default_deep_drawdown_threshold() -> f64 {
    -0.20
}

impl Default for RegimeConfig {
    fn default() -> Self {
        Self {
            default_windows: default_windows(),
            default_lookback_days: default_lookback_days(),
            hot_z_threshold: default_hot_z_threshold(),
            cold_z_threshold: default_cold_z_threshold(),
            high_volatility_threshold: default_high_volatility_threshold(),
            deep_drawdown_threshold: default_deep_drawdown_threshold(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KellyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_fractional_kelly")]
    pub fractional_kelly: f64,
    #[serde(default = "default_min_multiplier")]
    pub min_multiplier: f64,
    #[serde(default = "default_max_multiplier")]
    pub max_multiplier: f64,
    #[serde(default = "default_neutral_multiplier")]
    pub neutral_multiplier: f64,
    #[serde(default = "default_hot_market_multiplier")]
    pub hot_market_multiplier: f64,
    #[serde(default = "default_overheated_market_multiplier")]
    pub overheated_market_multiplier: f64,
    #[serde(default = "default_cold_market_multiplier")]
    pub cold_market_multiplier: f64,
    #[serde(default = "default_extreme_cold_market_multiplier")]
    pub extreme_cold_market_multiplier: f64,
    #[serde(default = "default_high_risk_multiplier")]
    pub high_risk_multiplier: f64,
    #[serde(default = "default_extreme_risk_multiplier")]
    pub extreme_risk_multiplier: f64,
    #[serde(default = "default_max_single_asset_buy_multiplier")]
    pub max_single_asset_buy_multiplier: f64,
    #[serde(default = "default_max_total_buy_multiplier")]
    pub max_total_buy_multiplier: f64,
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,
}

fn default_fractional_kelly() -> f64 {
    0.25
}
fn default_min_multiplier() -> f64 {
    0.0
}
fn default_max_multiplier() -> f64 {
    1.5
}
fn default_neutral_multiplier() -> f64 {
    1.0
}
fn default_hot_market_multiplier() -> f64 {
    0.5
}
fn default_overheated_market_multiplier() -> f64 {
    0.2
}
fn default_cold_market_multiplier() -> f64 {
    1.2
}
fn default_extreme_cold_market_multiplier() -> f64 {
    1.5
}
fn default_high_risk_multiplier() -> f64 {
    0.5
}
fn default_extreme_risk_multiplier() -> f64 {
    0.0
}
fn default_max_single_asset_buy_multiplier() -> f64 {
    1.5
}
fn default_max_total_buy_multiplier() -> f64 {
    1.5
}
fn default_min_confidence() -> f64 {
    0.3
}

impl Default for KellyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            fractional_kelly: default_fractional_kelly(),
            min_multiplier: default_min_multiplier(),
            max_multiplier: default_max_multiplier(),
            neutral_multiplier: default_neutral_multiplier(),
            hot_market_multiplier: default_hot_market_multiplier(),
            overheated_market_multiplier: default_overheated_market_multiplier(),
            cold_market_multiplier: default_cold_market_multiplier(),
            extreme_cold_market_multiplier: default_extreme_cold_market_multiplier(),
            high_risk_multiplier: default_high_risk_multiplier(),
            extreme_risk_multiplier: default_extreme_risk_multiplier(),
            max_single_asset_buy_multiplier: default_max_single_asset_buy_multiplier(),
            max_total_buy_multiplier: default_max_total_buy_multiplier(),
            min_confidence: default_min_confidence(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustedDecisionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_max_adjusted_multiplier")]
    pub max_adjusted_multiplier: f64,
    #[serde(default = "default_min_adjusted_multiplier")]
    pub min_adjusted_multiplier: f64,
    #[serde(default = "default_allow_increase_above_base")]
    pub allow_increase_above_base: bool,
    #[serde(default = "default_max_total_adjusted_buy_multiplier")]
    pub max_total_adjusted_buy_multiplier: f64,
    #[serde(default = "default_stale_data_multiplier")]
    pub stale_data_multiplier: f64,
    #[serde(default = "default_mock_data_multiplier")]
    pub mock_data_multiplier: f64,
    #[serde(default = "default_missing_regime_multiplier")]
    pub missing_regime_multiplier: f64,
    #[serde(default = "default_missing_risk_overlay_multiplier")]
    pub missing_risk_overlay_multiplier: f64,
    #[serde(default = "default_missing_kelly_multiplier")]
    pub missing_kelly_multiplier: f64,
    #[serde(default = "default_overheated_market_max_multiplier")]
    pub overheated_market_max_multiplier: f64,
    #[serde(default = "default_extreme_risk_max_multiplier")]
    pub extreme_risk_max_multiplier: f64,
    #[serde(default = "default_require_real_fund_nav")]
    pub require_real_fund_nav: bool,
    #[serde(default = "default_require_real_market_data")]
    pub require_real_market_data: bool,
}

fn default_max_adjusted_multiplier() -> f64 {
    1.5
}
fn default_min_adjusted_multiplier() -> f64 {
    0.0
}
fn default_allow_increase_above_base() -> bool {
    true
}
fn default_max_total_adjusted_buy_multiplier() -> f64 {
    1.5
}
fn default_stale_data_multiplier() -> f64 {
    0.5
}
fn default_mock_data_multiplier() -> f64 {
    0.0
}
fn default_missing_regime_multiplier() -> f64 {
    0.7
}
fn default_missing_risk_overlay_multiplier() -> f64 {
    0.7
}
fn default_missing_kelly_multiplier() -> f64 {
    0.7
}
fn default_overheated_market_max_multiplier() -> f64 {
    0.3
}
fn default_extreme_risk_max_multiplier() -> f64 {
    0.0
}
fn default_require_real_fund_nav() -> bool {
    true
}
fn default_require_real_market_data() -> bool {
    true
}

impl Default for AdjustedDecisionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_adjusted_multiplier: default_max_adjusted_multiplier(),
            min_adjusted_multiplier: default_min_adjusted_multiplier(),
            allow_increase_above_base: default_allow_increase_above_base(),
            max_total_adjusted_buy_multiplier: default_max_total_adjusted_buy_multiplier(),
            stale_data_multiplier: default_stale_data_multiplier(),
            mock_data_multiplier: default_mock_data_multiplier(),
            missing_regime_multiplier: default_missing_regime_multiplier(),
            missing_risk_overlay_multiplier: default_missing_risk_overlay_multiplier(),
            missing_kelly_multiplier: default_missing_kelly_multiplier(),
            overheated_market_max_multiplier: default_overheated_market_max_multiplier(),
            extreme_risk_max_multiplier: default_extreme_risk_max_multiplier(),
            require_real_fund_nav: default_require_real_fund_nav(),
            require_real_market_data: default_require_real_market_data(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationConfig {
    #[serde(default = "default_market_value_tolerance_abs")]
    pub market_value_tolerance_abs: f64,
    #[serde(default = "default_market_value_tolerance_pct")]
    pub market_value_tolerance_pct: f64,
    #[serde(default = "default_units_tolerance_abs")]
    pub units_tolerance_abs: f64,
    #[serde(default = "default_units_tolerance_pct")]
    pub units_tolerance_pct: f64,
    #[serde(default = "default_cost_basis_tolerance_abs")]
    pub cost_basis_tolerance_abs: f64,
    #[serde(default = "default_cost_basis_tolerance_pct")]
    pub cost_basis_tolerance_pct: f64,
    #[serde(default)]
    pub allow_calibration_apply: bool,
}

fn default_market_value_tolerance_abs() -> f64 {
    1.0
}
fn default_market_value_tolerance_pct() -> f64 {
    0.001
}
fn default_units_tolerance_abs() -> f64 {
    0.01
}
fn default_units_tolerance_pct() -> f64 {
    0.0001
}
fn default_cost_basis_tolerance_abs() -> f64 {
    1.0
}
fn default_cost_basis_tolerance_pct() -> f64 {
    0.001
}

impl Default for ReconciliationConfig {
    fn default() -> Self {
        Self {
            market_value_tolerance_abs: default_market_value_tolerance_abs(),
            market_value_tolerance_pct: default_market_value_tolerance_pct(),
            units_tolerance_abs: default_units_tolerance_abs(),
            units_tolerance_pct: default_units_tolerance_pct(),
            cost_basis_tolerance_abs: default_cost_basis_tolerance_abs(),
            cost_basis_tolerance_pct: default_cost_basis_tolerance_pct(),
            allow_calibration_apply: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyPlanConfig {
    #[serde(default = "default_true")]
    pub pause_on_reconciliation_mismatch: bool,
    #[serde(default)]
    pub pause_on_missing_reconciliation: bool,
    #[serde(default = "default_true")]
    pub pause_on_mock_data: bool,
    #[serde(default = "default_one")]
    pub max_total_daily_plan_multiplier: f64,
    #[serde(default = "default_true")]
    pub include_dca: bool,
    #[serde(default = "default_true")]
    pub include_adjusted_decision: bool,
    #[serde(default = "default_true")]
    pub include_kelly_preview: bool,
    #[serde(default = "default_true")]
    pub include_reconciliation_gate: bool,
}

fn default_true() -> bool {
    true
}
fn default_one() -> f64 {
    1.0
}

impl Default for DailyPlanConfig {
    fn default() -> Self {
        Self {
            pause_on_reconciliation_mismatch: true,
            pause_on_missing_reconciliation: false,
            pause_on_mock_data: true,
            max_total_daily_plan_multiplier: 1.0,
            include_dca: true,
            include_adjusted_decision: true,
            include_kelly_preview: true,
            include_reconciliation_gate: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    #[default]
    Json,
    Postgres,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresStorageConfig {
    #[serde(default = "default_database_url_env")]
    pub database_url_env: String,
}

fn default_database_url_env() -> String {
    "DATABASE_URL".to_string()
}

impl Default for PostgresStorageConfig {
    fn default() -> Self {
        Self {
            database_url_env: default_database_url_env(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageConfig {
    #[serde(default)]
    pub backend: StorageBackend,
    #[serde(default)]
    pub postgres: PostgresStorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    pub regime: RegimeConfig,
    #[serde(default)]
    pub kelly: KellyConfig,
    #[serde(default)]
    pub adjusted_decision: AdjustedDecisionConfig,
    #[serde(default)]
    pub reconciliation: ReconciliationConfig,
    #[serde(default)]
    pub daily_plan: DailyPlanConfig,
    #[serde(default)]
    pub assets: Vec<AssetConfig>,
    #[serde(default)]
    pub sectors: Vec<SectorConfig>,
    #[serde(default)]
    pub storage: StorageConfig,
}

impl ConfigRoot {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.storage.backend == StorageBackend::Postgres {
            let env_var = &self.storage.postgres.database_url_env;
            if std::env::var(env_var).is_err() {
                anyhow::bail!(
                    "Storage backend is set to 'postgres', but environment variable '{}' is missing.",
                    env_var
                );
            }
        }
        Ok(())
    }
}
