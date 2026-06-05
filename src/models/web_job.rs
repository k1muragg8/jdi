use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WebJobStatus {
    #[default]
    Queued,
    Running,
    Success,
    PartialSuccess,
    Warning,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebJob {
    pub job_id: String,
    pub portfolio_id: String,
    pub job_type: String,
    pub status: WebJobStatus,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub progress_current: i32,
    pub progress_total: i32,
    pub message: Option<String>,
    pub result_json: Option<Value>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobStepResult {
    pub name: String,
    pub status: String, // "ok", "warning", "error"
    pub message: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub affected_count: i32,
    pub action_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketRefreshResult {
    pub active_symbols_count: usize,
    pub success_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub inserted_count: usize,
    pub updated_count: usize,
    pub provider_errors: Vec<String>,
    pub unsupported_symbols: Vec<String>,
    pub no_data_symbols: Vec<String>,
    pub refreshed_symbols: Vec<String>,
    pub skipped_symbols: Vec<String>,
    pub failed_symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartJobResponse {
    pub job_id: String,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusResponse {
    pub job: Option<WebJob>,
    pub is_running: bool,
}
