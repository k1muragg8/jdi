use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod traits;
pub mod json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageMode {
    Json,
    Postgres,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryContext {
    pub actor_user_id: String,
    pub target_user_id: String,
    pub portfolio_id: String,
    pub role: String,
    pub storage_mode: StorageMode,
}

impl Default for RepositoryContext {
    fn default() -> Self {
        Self {
            actor_user_id: "local_user".to_string(),
            target_user_id: "local_user".to_string(),
            portfolio_id: "default".to_string(),
            role: "owner".to_string(),
            storage_mode: StorageMode::Json,
        }
    }
}

pub trait Repository:
    traits::PortfolioRepository
    + traits::DcaRepository
    + traits::ReconciliationRepository
    + traits::InstrumentRepository
    + traits::ReportRepository
    + traits::AuditRepository
    + traits::CacheRepository
{}

impl<T> Repository for T where
    T: traits::PortfolioRepository
        + traits::DcaRepository
        + traits::ReconciliationRepository
        + traits::InstrumentRepository
        + traits::ReportRepository
        + traits::AuditRepository
        + traits::CacheRepository
{}
