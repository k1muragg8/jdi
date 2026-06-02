use crate::models::{
    CacheStatusRegistry, DcaLifecycleSummary, DecisionExplanation, GlobalRiskOverlay,
    PortfolioSummary,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DashboardSummary {
    pub portfolio: PortfolioSummary,
    pub lifecycle: DcaLifecycleSummary,
    pub cache_status: CacheStatusRegistry,
    pub decision: DecisionExplanation,
    pub risk_overlay: GlobalRiskOverlay,
    pub operation_status: super::operation::OperationStatus,
    pub backend: String, // "JSON" or "PostgreSQL"
    pub portfolio_name: String,
    pub date: String,
    pub alipay_total_value: Option<f64>,
    pub alipay_snapshot_date: Option<String>,
    pub unclassified_asset_count: usize,
    pub reconciliation_issue_count: usize,
    pub alipay_mismatch_count: usize,
}
