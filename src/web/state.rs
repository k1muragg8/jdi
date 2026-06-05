use crate::repository::{Repository, RepositoryContext};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackgroundRefreshStatus {
    pub last_market_refresh: Option<String>,
    pub last_fund_refresh: Option<String>,
    pub is_running: bool,
    pub last_error: Option<String>,
    pub latest_daily_report: Option<crate::models::DailyOperationReport>,
}

pub struct AppState {
    pub repo: Arc<dyn Repository>,
    pub ctx: RepositoryContext,
    pub refresh_status: Arc<RwLock<BackgroundRefreshStatus>>,
    pub last_backtest_report: Arc<RwLock<Option<crate::models::BacktestReport>>>,
    pub running_jobs: Arc<RwLock<std::collections::HashSet<String>>>,
}
