use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyExecutionItem {
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub sector: String,
    pub dca_due_amount: f64,
    pub adjusted_decision_amount: f64,
    pub kelly_preview_amount: f64,
    pub recommended_amount: f64,
    pub source: String, // DCA, 风险调整, DCA+风险调整, 无操作
    pub reconciliation_status: String,
    pub reconciliation_warning: Option<String>,
    pub data_status: String,
    pub confidence: f64,
    pub status: String, // 今日应执行, 建议观察, 暂停执行, 等待对账, 数据不足, 无需操作
    pub warnings: Vec<String>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyExecutionPlan {
    pub date: String,
    pub total_dca_due: f64,
    pub total_adjusted_decision: f64,
    pub total_recommended_amount: f64,
    pub available_cash: f64,
    pub max_daily_buy: f64,
    pub global_risk_label: String,
    pub global_risk_score: f64,
    pub items: Vec<DailyExecutionItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DailyOperationStatus {
    Pending,
    Running,
    Success,
    PartialSuccess,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyOperationStep {
    pub name: String,
    pub status: DailyOperationStatus,
    pub message: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyOperationReport {
    pub date: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: DailyOperationStatus,
    pub steps: Vec<DailyOperationStep>,
    pub plan: Option<DailyExecutionPlan>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyOperationResult {
    pub success: bool,
    pub message: String,
}
