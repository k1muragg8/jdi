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
    pub backend: String, // "JSON" or "PostgreSQL"
    pub portfolio_name: String,
    pub date: String,
}
