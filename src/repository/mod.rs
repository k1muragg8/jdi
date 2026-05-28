use serde::{Deserialize, Serialize};

pub mod json;
pub mod traits;

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

impl RepositoryContext {
    pub fn local_owner() -> Self {
        Self::default()
    }

    pub fn default_json() -> Self {
        Self::default()
    }
}

pub struct RepositoryFactory;

impl RepositoryFactory {
    /// Creates a repository based on the global configuration.
    /// Currently defaults to JsonRepository until Postgres is implemented.
    pub fn from_config(_config: &crate::models::ConfigRoot) -> Box<dyn Repository + Send + Sync> {
        Self::json_default()
    }

    /// Creates a default JsonRepository using the standard "data" directory.
    pub fn json_default() -> Box<dyn Repository + Send + Sync> {
        Box::new(json::JsonRepository::new_with_defaults("data"))
    }

    /// Creates a JsonRepository with a custom base directory.
    pub fn json_from_dir(base_dir: &str) -> Box<dyn Repository + Send + Sync> {
        Box::new(json::JsonRepository::new_with_defaults(base_dir))
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
    + Send
    + Sync
{
}

impl<T> Repository for T where
    T: traits::PortfolioRepository
        + traits::DcaRepository
        + traits::ReconciliationRepository
        + traits::InstrumentRepository
        + traits::ReportRepository
        + traits::AuditRepository
        + traits::CacheRepository
        + Send
        + Sync
{
}
