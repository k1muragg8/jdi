use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DcaFrequency {
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaPlan {
    pub plan_id: String,
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub amount: f64,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub frequency: DcaFrequency,
    pub weekday: Option<u32>,   // 1-7 for weekly
    pub month_day: Option<u32>, // 1-31 for monthly
    pub start_date: String,     // YYYY-MM-DD
    pub end_date: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub priority: i32,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_currency() -> String {
    "CNY".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaPreviewItem {
    pub plan_id: String,
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub amount: f64,
    pub currency: String,
    pub due_date: String,
    pub frequency: DcaFrequency,
    pub status: String,
    pub latest_nav: Option<f64>,
    pub nav_date: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaPreviewSummary {
    pub date: String,
    pub total_due_amount: f64,
    pub items: Vec<DcaPreviewItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DcaSettlementStatus {
    Confirmed,
    Pending,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaSettlement {
    pub settlement_id: String,
    pub plan_id: Option<String>,
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub scheduled_date: Option<String>,
    pub deduction_date: String,
    pub confirmation_date: String,
    pub amount: f64,
    pub confirmed_nav: f64,
    pub confirmed_units: f64,
    pub fee: Option<f64>,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default = "default_source")]
    pub source: String,
    pub status: DcaSettlementStatus,
    #[serde(default)]
    pub applied: bool,
    pub note: Option<String>,
    pub created_at: String,
}

fn default_source() -> String {
    "alipay".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaSettlementImpact {
    pub settlement_id: String,
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub amount: f64,
    pub confirmed_nav: f64,
    pub confirmed_units: f64,
    pub old_units: f64,
    pub new_units: f64,
    pub old_cost_basis: f64,
    pub new_cost_basis: f64,
    pub old_market_value: f64,
    pub estimated_new_market_value: f64,
    pub would_modify_state: bool,
    pub would_create_transaction: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaSettlementAudit {
    pub audit_id: String,
    pub timestamp: String,
    pub settlement_id: String,
    pub asset_id: String,
    pub old_units: f64,
    pub new_units: f64,
    pub old_cost_basis: f64,
    pub new_cost_basis: f64,
    pub transaction_id: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaLifecycleItem {
    pub date: String,
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub plan_id: Option<String>,
    pub planned_amount: f64,
    pub settlement_id: Option<String>,
    pub settlement_amount: Option<f64>,
    pub confirmed_nav: Option<f64>,
    pub confirmed_units: Option<f64>,
    pub settlement_applied: bool,
    pub latest_alipay_snapshot_id: Option<String>,
    pub alipay_market_value: Option<f64>,
    pub system_market_value: Option<f64>,
    pub reconciliation_status: String,
    pub lifecycle_status: String,
    pub warnings: Vec<String>,
    pub suggested_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DcaLifecycleSummary {
    pub date: String,
    pub total_planned_amount: f64,
    pub total_confirmed_amount: f64,
    pub total_unapplied_settlement_amount: f64,
    pub total_reconciliation_diff: f64,
    pub count_due: usize,
    pub count_waiting_confirmation: usize,
    pub count_unapplied: usize,
    pub count_reconciled: usize,
    pub count_attention_required: usize,
    pub items: Vec<DcaLifecycleItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DcaExecutionResult {
    pub executed_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub success: bool,
    pub message: String,
}
