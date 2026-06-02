use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlipaySnapshot {
    pub snapshot_id: String,
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub snapshot_date: String, // YYYY-MM-DD
    pub market_value: f64,
    pub units: Option<f64>,
    pub cost_basis: Option<f64>,
    pub nav: Option<f64>,
    pub nav_date: Option<String>,
    pub daily_pnl: Option<f64>,
    pub total_pnl: Option<f64>,
    #[serde(default = "default_source")]
    pub source: String,
    pub created_at: String,
    pub note: Option<String>,
}

fn default_source() -> String {
    "alipay".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlipayHoldingCandidate {
    #[serde(alias = "基金代码", alias = "fund_code")]
    pub fund_code: String,
    #[serde(alias = "基金名称", alias = "资产名称", alias = "fund_name")]
    pub fund_name: String,
    #[serde(alias = "持有份额", alias = "份额", alias = "units")]
    pub units: f64,
    #[serde(alias = "持有金额", alias = "市值", alias = "market_value")]
    pub market_value: f64,
    #[serde(alias = "最新净值", alias = "nav")]
    pub nav: Option<f64>,
    #[serde(alias = "净值日期", alias = "nav_date")]
    pub nav_date: Option<String>,
    #[serde(alias = "投入本金", alias = "cost_basis")]
    pub cost_basis: Option<f64>,
    #[serde(alias = "持有收益", alias = "total_profit")]
    pub total_profit: Option<f64>,
    #[serde(alias = "持有收益率", alias = "profit_rate")]
    pub profit_rate: Option<f64>,
    #[serde(alias = "来源", alias = "source")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlipayHoldingImportPreview {
    pub snapshot_date: String,
    pub candidates: Vec<AlipayHoldingCandidate>,
    pub matched_asset_ids: Vec<Option<String>>,
    pub system_units: Vec<Option<f64>>,
    pub system_market_values: Vec<Option<f64>>,
    pub unit_diffs: Vec<Option<f64>>,
    pub warnings: Vec<Vec<String>>,
    pub errors: Vec<Vec<String>>,
    pub total_rows: usize,
    pub valid_rows: usize,
    pub invalid_rows: usize,
    pub unmatched_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlipayHoldingImportResult {
    pub imported_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapLocalPreviewRow {
    pub asset_id: Option<String>,
    pub fund_code: String,
    pub fund_name: String,
    pub market_value: f64,
    pub latest_nav: Option<f64>,
    pub nav_date: Option<String>,
    pub estimated_shares: Option<f64>,
    pub estimated_cost_basis: Option<f64>,
    pub existing_shares: Option<f64>,
    pub action: String,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapLocalPreview {
    pub rows: Vec<BootstrapLocalPreviewRow>,
    pub total_bootstrapped_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationResult {
    pub snapshot_id: String,
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub snapshot_date: String,
    pub system_market_value: f64,
    pub alipay_market_value: f64,
    pub market_value_diff: f64,
    pub market_value_diff_pct: f64,
    pub system_units: Option<f64>,
    pub alipay_units: Option<f64>,
    pub units_diff: Option<f64>,
    pub units_diff_pct: Option<f64>,
    pub system_cost_basis: Option<f64>,
    pub alipay_cost_basis: Option<f64>,
    pub cost_basis_diff: Option<f64>,
    pub cost_basis_diff_pct: Option<f64>,
    pub system_nav: Option<f64>,
    pub alipay_nav: Option<f64>,
    pub nav_diff: Option<f64>,
    pub nav_date_diff: Option<i64>,
    pub status: String,
    pub warnings: Vec<String>,
    pub suggested_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationSuggestion {
    pub asset_id: String,
    pub fund_code: String,
    pub snapshot_id: String,
    pub suggested_units: Option<f64>,
    pub suggested_cost_basis: Option<f64>,
    pub suggested_market_value: Option<f64>,
    pub reason: String,
    pub risk_level: String,
    pub would_modify_state: bool,
    pub would_create_adjustment_transaction: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationAudit {
    pub audit_id: String,
    pub timestamp: String,
    pub snapshot_id: String,
    pub asset_id: String,
    pub old_units: f64,
    pub new_units: f64,
    pub old_cost_basis: f64,
    pub new_cost_basis: f64,
    pub old_market_value: f64,
    pub new_market_value: f64,
    pub reason: String,
    pub note: Option<String>,
}
