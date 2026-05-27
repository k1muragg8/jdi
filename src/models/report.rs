use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReportPeriod {
    Daily,
    Weekly,
    Monthly,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub title: String,
    pub status: String,
    pub summary: String,
    pub details: Vec<String>,
    pub warnings: Vec<String>,
    pub suggested_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestmentReport {
    pub report_id: String,
    pub report_type: ReportPeriod,
    pub start_date: String,
    pub end_date: String,
    pub generated_at: String,
    pub title: String,

    // Aggregated data for easy access
    pub portfolio_summary: Option<super::portfolio::PortfolioSummary>,
    pub dca_summary: Option<super::dca::DcaLifecycleSummary>,
    pub reconciliation_summary: Option<super::reconciliation::ReconciliationResult>, // Placeholder, might need multiple
    pub risk_summary: Option<super::risk_overlay::GlobalRiskOverlay>,

    pub sections: Vec<ReportSection>,
    pub warnings: Vec<String>,
    pub pending_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    pub snapshot_id: String,
    pub date: String,
    pub total_assets: f64,
    pub cash: f64,
    pub equity_value: f64,
    pub fund_value: f64,
    pub bond_value: f64,
    pub crypto_value: f64,
    pub source: String,
    pub created_at: String,
}
