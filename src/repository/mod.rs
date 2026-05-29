pub mod json;
pub mod postgres;
pub mod traits;

pub use json::JsonRepository;
pub use postgres::PostgresRepository;

use crate::cli::Cli;
use crate::models::{ConfigRoot, StorageBackend};
use serde::{Deserialize, Serialize};

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

use std::sync::Arc;

pub struct RepositoryFactory;

impl RepositoryFactory {
    /// Creates a repository based on the global configuration.
    pub async fn from_config(
        config: &ConfigRoot,
        cli: &Cli,
    ) -> anyhow::Result<Arc<dyn Repository>> {
        match config.storage.backend {
            StorageBackend::Json => Ok(Arc::new(json::JsonRepository::new(
                cli.config.clone(),
                cli.state.clone(),
                cli.transactions.clone(),
                cli.dca_plans.clone(),
                cli.dca_settlements.clone(),
                cli.dca_settlement_audit.clone(),
                cli.alipay_snapshots.clone(),
                cli.instruments.clone(),
                cli.cache_status.clone(),
                cli.instrument_cache.clone(),
                cli.risk_cache.clone(),
                cli.proxy_cache.clone(),
                cli.regime_cache.clone(),
                cli.market_cache.clone(),
                cli.fx_cache.clone(),
                cli.cache.clone(),
                cli.web_audit.clone(),
                cli.reconciliation_audit.clone(),
                "data/portfolio_snapshots.json".to_string(),
            ))),
            StorageBackend::Postgres => {
                let env_var = &config.storage.postgres.database_url_env;
                let database_url = std::env::var(env_var).map_err(|_| {
                    anyhow::anyhow!(
                        "Environment variable '{}' for PostgreSQL not found",
                        env_var
                    )
                })?;
                let pool = sqlx::PgPool::connect(&database_url).await?;
                Ok(Arc::new(postgres::PostgresRepository::new(
                    pool,
                    cli.config.clone(),
                )))
            }
        }
    }

    /// Creates a default JsonRepository using the standard "data" directory.
    pub fn json_default() -> Arc<dyn Repository> {
        Arc::new(json::JsonRepository::new_with_defaults("data"))
    }

    /// Creates a JsonRepository with a custom base directory.
    pub fn json_from_dir(base_dir: &str) -> Arc<dyn Repository> {
        Arc::new(json::JsonRepository::new_with_defaults(base_dir))
    }
}

#[derive(Debug, Default)]
pub struct MigrationReport {
    pub read: usize,
    pub inserted: usize,
    pub skipped: usize,
    pub failed: usize,
}

/// Helper to migrate transactions between repositories.
/// This fulfills the requirement for a migration path between backends.
pub async fn migrate_transactions(
    source: &dyn traits::PortfolioRepository,
    target: &dyn traits::PortfolioRepository,
    ctx: &RepositoryContext,
) -> anyhow::Result<MigrationReport> {
    let source_txs = source.load_transactions(ctx).await?;
    let target_txs = target.load_transactions(ctx).await?;

    let mut report = MigrationReport {
        read: source_txs.len(),
        ..Default::default()
    };

    let target_ids: std::collections::HashSet<String> =
        target_txs.into_iter().map(|t| t.id).collect();

    let mut to_insert = Vec::new();
    for s_tx in source_txs {
        if target_ids.contains(&s_tx.id) {
            report.skipped += 1;
        } else {
            to_insert.push(s_tx);
        }
    }

    if !to_insert.is_empty() {
        let count = to_insert.len();
        match target.save_transactions(ctx, &to_insert).await {
            Ok(_) => report.inserted = count,
            Err(e) => {
                #[allow(unused_assignments)]
                {
                    report.failed = count;
                }
                return Err(anyhow::anyhow!("Migration failed during insert: {}", e));
            }
        }
    }

    Ok(report)
}

/// Helper to migrate state between repositories.
pub async fn migrate_state(
    source: &dyn traits::PortfolioRepository,
    target: &dyn traits::PortfolioRepository,
    ctx: &RepositoryContext,
) -> anyhow::Result<()> {
    let state = source.load_state(ctx).await?;
    target.save_state(ctx, &state).await?;
    Ok(())
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
