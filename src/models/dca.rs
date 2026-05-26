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
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaPreviewSummary {
    pub date: String,
    pub total_due_amount: f64,
    pub items: Vec<DcaPreviewItem>,
    pub warnings: Vec<String>,
}
