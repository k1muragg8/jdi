pub mod json;
pub mod postgres;
pub mod traits;

pub use json::JsonRepository;
pub use postgres::PostgresRepository;

use crate::cli::Cli;
use crate::models::{ConfigRoot, StorageBackend};
use anyhow::Context;
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
                cli.operation_policy.clone(),
                cli.operation_status.clone(),
                cli.daily_operation_report.clone(),
                crate::resolve_data_dir()
                    .join("portfolio_snapshots.json")
                    .to_string_lossy()
                    .to_string(),
                cli.web_jobs_dir.clone(),
            ))),
            StorageBackend::Postgres => {
                let env_var = &config.storage.postgres.database_url_env;
                let database_url = std::env::var(env_var).map_err(|_| {
                    anyhow::anyhow!(
                        "PostgreSQL backend selected but environment variable {} is not set. Refusing to fallback to JSON.",
                        env_var
                    )
                })?;
                let pool = sqlx::PgPool::connect(&database_url).await
                    .with_context(|| format!("PostgreSQL backend selected but connection using env {} failed. Refusing to fallback to JSON. Please ensure the database exists and the URL is correct.", env_var))?;
                sqlx::migrate!("./migrations")
                    .run(&pool)
                    .await
                    .context("Failed to run database migrations")?;
                Ok(Arc::new(postgres::PostgresRepository::new(
                    pool,
                    cli.config.clone(),
                    env_var.clone(),
                )))
            }
        }
    }

    /// Creates a default JsonRepository using the standard "data" directory.
    pub fn json_default() -> Arc<dyn Repository> {
        Arc::new(json::JsonRepository::new_with_defaults(
            &crate::resolve_data_dir().to_string_lossy(),
        ))
    }

    /// Creates a JsonRepository with a custom base directory.
    pub fn json_from_dir(base_dir: &str) -> Arc<dyn Repository> {
        Arc::new(json::JsonRepository::new_with_defaults(base_dir))
    }
}

#[derive(Debug, Default, Clone)]
pub struct MigrationReport {
    pub domain: String,
    pub read: usize,
    pub inserted: usize,
    pub skipped: usize,
    pub failed: usize,
}

impl MigrationReport {
    pub fn new(domain: &str) -> Self {
        Self {
            domain: domain.to_string(),
            ..Default::default()
        }
    }
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

    let mut report = MigrationReport::new("Transactions");
    report.read = source_txs.len();

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
                report.failed = count;
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
) -> anyhow::Result<MigrationReport> {
    let state = source.load_state(ctx).await?;
    let mut report = MigrationReport::new("PortfolioState");
    report.read = 1 + state.asset_holdings.len(); // Cash + Holdings

    match target.save_state(ctx, &state).await {
        Ok(_) => report.inserted = report.read,
        Err(e) => {
            report.failed = report.read;
            return Err(anyhow::anyhow!("Migration failed during state save: {}", e));
        }
    }

    Ok(report)
}

/// Helper to migrate DCA plans, settlements, and audits.
pub async fn migrate_dca(
    source: &dyn traits::DcaRepository,
    target: &dyn traits::DcaRepository,
    ctx: &RepositoryContext,
) -> anyhow::Result<MigrationReport> {
    let mut report = MigrationReport::new("DCA");

    let plans = source.load_plans(ctx).await?;
    report.read += plans.len();
    target.save_plans(ctx, &plans).await?;
    report.inserted += plans.len();

    let settlements = source.load_settlements(ctx).await?;
    report.read += settlements.len();
    target.save_settlements(ctx, &settlements).await?;
    report.inserted += settlements.len();

    let audits = source.load_settlement_audits(ctx).await?;
    report.read += audits.len();
    target.save_settlement_audits(ctx, &audits).await?;
    report.inserted += audits.len();

    Ok(report)
}

/// Helper to migrate Reconciliation snapshots and audits.
pub async fn migrate_reconciliation(
    source: &dyn traits::ReconciliationRepository,
    target: &dyn traits::ReconciliationRepository,
    ctx: &RepositoryContext,
) -> anyhow::Result<MigrationReport> {
    let mut report = MigrationReport::new("Reconciliation");

    let snaps = source.load_alipay_snapshots(ctx).await?;
    report.read += snaps.len();
    target.save_alipay_snapshots(ctx, &snaps).await?;
    report.inserted += snaps.len();

    let audits = source.load_reconciliation_audits(ctx).await?;
    report.read += audits.len();
    target.save_reconciliation_audits(ctx, &audits).await?;
    report.inserted += audits.len();

    Ok(report)
}

/// Helper to migrate Instruments and their cache.
pub async fn migrate_instruments(
    source: &dyn traits::InstrumentRepository,
    target: &dyn traits::InstrumentRepository,
    ctx: &RepositoryContext,
) -> anyhow::Result<MigrationReport> {
    let mut report = MigrationReport::new("Instruments");

    let instruments = source.load_instruments(ctx).await?;
    report.read += instruments.len();
    target.save_instruments(ctx, &instruments).await?;
    report.inserted += instruments.len();

    let cache = source.load_instrument_cache(ctx).await?;
    report.read += cache.entries.len();
    target.save_instrument_cache(ctx, &cache).await?;
    report.inserted += cache.entries.len();

    Ok(report)
}

pub trait Repository:
    traits::PortfolioRepository
    + traits::DcaRepository
    + traits::ReconciliationRepository
    + traits::InstrumentRepository
    + traits::ReportRepository
    + traits::AuditRepository
    + traits::OperationRepository
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
        + traits::OperationRepository
        + traits::CacheRepository
        + Send
        + Sync
{
}
