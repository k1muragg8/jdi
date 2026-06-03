use crate::models::*;
use crate::repository::RepositoryContext;
use crate::repository::traits::*;
use crate::storage;
use anyhow::{Result, anyhow};
use async_trait::async_trait;

pub struct JsonRepository {
    pub config_path: String,
    pub state_path: String,
    pub transactions_path: String,
    pub dca_plans_path: String,
    pub dca_settlements_path: String,
    pub dca_settlement_audit_path: String,
    pub alipay_snapshots_path: String,
    pub instruments_path: String,
    pub cache_status_path: String,
    pub instrument_cache_path: String,
    pub risk_cache_path: String,
    pub proxy_cache_path: String,
    pub regime_cache_path: String,
    pub market_cache_path: String,
    pub fx_cache_path: String,
    pub nav_cache_path: String,
    pub web_audit_path: String,
    pub reconciliation_audit_path: String,
    pub operation_policy_path: String,
    pub operation_status_path: String,
    pub daily_operation_report_path: String,
    pub snapshot_path: String,
}

impl JsonRepository {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config_path: String,
        state_path: String,
        transactions_path: String,
        dca_plans_path: String,
        dca_settlements_path: String,
        dca_settlement_audit_path: String,
        alipay_snapshots_path: String,
        instruments_path: String,
        cache_status_path: String,
        instrument_cache_path: String,
        risk_cache_path: String,
        proxy_cache_path: String,
        regime_cache_path: String,
        market_cache_path: String,
        fx_cache_path: String,
        nav_cache_path: String,
        web_audit_path: String,
        reconciliation_audit_path: String,
        operation_policy_path: String,
        operation_status_path: String,
        daily_operation_report_path: String,
        snapshot_path: String,
    ) -> Self {
        Self {
            config_path,
            state_path,
            transactions_path,
            dca_plans_path,
            dca_settlements_path,
            dca_settlement_audit_path,
            alipay_snapshots_path,
            instruments_path,
            cache_status_path,
            instrument_cache_path,
            risk_cache_path,
            proxy_cache_path,
            regime_cache_path,
            market_cache_path,
            fx_cache_path,
            nav_cache_path,
            web_audit_path,
            reconciliation_audit_path,
            operation_policy_path,
            operation_status_path,
            daily_operation_report_path,
            snapshot_path,
        }
    }

    pub fn new_with_defaults(base_dir: &str) -> Self {
        let base = std::path::Path::new(base_dir);
        Self {
            config_path: base.join("config.toml").to_string_lossy().to_string(),
            state_path: base
                .join("portfolio_state.json")
                .to_string_lossy()
                .to_string(),
            transactions_path: base.join("transactions.json").to_string_lossy().to_string(),
            dca_plans_path: base.join("dca_plans.json").to_string_lossy().to_string(),
            dca_settlements_path: base
                .join("dca_settlements.json")
                .to_string_lossy()
                .to_string(),
            dca_settlement_audit_path: base
                .join("dca_settlement_audit.json")
                .to_string_lossy()
                .to_string(),
            alipay_snapshots_path: base
                .join("alipay_snapshots.json")
                .to_string_lossy()
                .to_string(),
            instruments_path: base.join("instruments.toml").to_string_lossy().to_string(),
            cache_status_path: base.join("cache_status.json").to_string_lossy().to_string(),
            instrument_cache_path: base
                .join("instrument_cache.json")
                .to_string_lossy()
                .to_string(),
            risk_cache_path: base.join("risk_cache.json").to_string_lossy().to_string(),
            proxy_cache_path: base.join("proxy_cache.json").to_string_lossy().to_string(),
            regime_cache_path: base.join("regime_cache.json").to_string_lossy().to_string(),
            market_cache_path: base.join("market_cache.json").to_string_lossy().to_string(),
            fx_cache_path: base.join("fx_cache.json").to_string_lossy().to_string(),
            nav_cache_path: base.join("nav_cache.json").to_string_lossy().to_string(),
            web_audit_path: base
                .join("web_admin_audit.json")
                .to_string_lossy()
                .to_string(),
            reconciliation_audit_path: base
                .join("reconciliation_audit.json")
                .to_string_lossy()
                .to_string(),
            operation_policy_path: base
                .join("operation_policy.json")
                .to_string_lossy()
                .to_string(),
            operation_status_path: base
                .join("operation_status.json")
                .to_string_lossy()
                .to_string(),
            daily_operation_report_path: base
                .join("daily_operation_report.json")
                .to_string_lossy()
                .to_string(),
            snapshot_path: base
                .join("portfolio_snapshots.json")
                .to_string_lossy()
                .to_string(),
        }
    }
}

#[async_trait]
impl PortfolioRepository for JsonRepository {
    fn name(&self) -> String {
        "JSON".to_string()
    }

    async fn get_db_status(&self, _ctx: &RepositoryContext) -> Result<DbStatus> {
        let mut tables = Vec::new();

        // Helper to count JSON items if file exists
        fn count_json_items<T: serde::de::DeserializeOwned>(path: &str) -> i64 {
            if let Ok(content) = std::fs::read_to_string(path) {
                // Try as Vec<T>
                if let Ok(items) = serde_json::from_str::<Vec<T>>(&content) {
                    return items.len() as i64;
                }
                // Try as single object if it's not a Vec
                if let Ok(_item) = serde_json::from_str::<T>(&content) {
                    return 1;
                }
            }
            0
        }

        tables.push(TableCount {
            name: "portfolios".to_string(),
            count: 1,
        });
        tables.push(TableCount {
            name: "transactions".to_string(),
            count: count_json_items::<Transaction>(&self.transactions_path),
        });
        tables.push(TableCount {
            name: "dca_plans".to_string(),
            count: count_json_items::<DcaPlan>(&self.dca_plans_path),
        });
        tables.push(TableCount {
            name: "alipay_snapshots".to_string(),
            count: count_json_items::<AlipaySnapshot>(&self.alipay_snapshots_path),
        });
        tables.push(TableCount {
            name: "instrument_cache".to_string(),
            count: count_json_items::<InstrumentQuoteCache>(&self.instrument_cache_path),
        });
        tables.push(TableCount {
            name: "web_admin_audit_logs".to_string(),
            count: count_json_items::<WebAdminAudit>(&self.web_audit_path),
        });

        Ok(DbStatus {
            backend: "JSON".to_string(),
            database_url_source: "filesystem".to_string(),
            database_name: None,
            schema: None,
            user: None,
            host: None,
            port: None,
            fallback: false,
            data_dir: Some("data".to_string()),
            tables,
            migrations_active: false,
            active_portfolio_id: _ctx.portfolio_id.clone(),
            portfolio_records: Vec::new(),
        })
    }

    async fn load_config(&self, _ctx: &RepositoryContext) -> Result<ConfigRoot> {
        let path = self.config_path.clone();
        tokio::task::spawn_blocking(move || storage::config_store::load_config(&path)).await?
    }
    async fn save_config(&self, _ctx: &RepositoryContext, config: &ConfigRoot) -> Result<()> {
        let path = self.config_path.clone();
        let config = config.clone();
        tokio::task::spawn_blocking(move || storage::config_store::save_config(&path, &config))
            .await?
    }
    async fn load_state(&self, _ctx: &RepositoryContext) -> Result<PortfolioState> {
        let path = self.state_path.clone();
        tokio::task::spawn_blocking(move || storage::state_store::load_state(&path)).await?
    }
    async fn save_state(&self, _ctx: &RepositoryContext, state: &PortfolioState) -> Result<()> {
        let path = self.state_path.clone();
        let state = state.clone();
        tokio::task::spawn_blocking(move || storage::state_store::save_state(&path, &state)).await?
    }
    async fn load_transactions(&self, _ctx: &RepositoryContext) -> Result<Vec<Transaction>> {
        let path = self.transactions_path.clone();
        tokio::task::spawn_blocking(move || storage::transaction_store::load_transactions(&path))
            .await?
    }
    async fn save_transactions(
        &self,
        _ctx: &RepositoryContext,
        transactions: &[Transaction],
    ) -> Result<()> {
        let path = self.transactions_path.clone();
        let transactions = transactions.to_vec();
        tokio::task::spawn_blocking(move || {
            storage::transaction_store::save_transactions(&path, &transactions)
        })
        .await?
    }
    async fn update_transaction(&self, ctx: &RepositoryContext, tx: &Transaction) -> Result<()> {
        let mut transactions = self.load_transactions(ctx).await?;
        if let Some(pos) = transactions.iter().position(|t| t.id == tx.id) {
            transactions[pos] = tx.clone();
            self.save_transactions(ctx, &transactions).await?;
            Ok(())
        } else {
            Err(anyhow!("Transaction not found"))
        }
    }
    async fn delete_transaction(&self, ctx: &RepositoryContext, id: &str) -> Result<()> {
        let mut transactions = self.load_transactions(ctx).await?;
        if let Some(pos) = transactions.iter().position(|t| t.id == id) {
            transactions.remove(pos);
            self.save_transactions(ctx, &transactions).await?;
            Ok(())
        } else {
            Err(anyhow!("Transaction not found"))
        }
    }
    async fn list_portfolios(&self, _ctx: &RepositoryContext) -> Result<Vec<Portfolio>> {
        Ok(vec![Portfolio {
            id: "default".to_string(),
            name: "Default Portfolio".to_string(),
            description: None,
            owner_user_id: "local_user".to_string(),
            current_cash: 0.0,
            created_at: "".to_string(),
            updated_at: "".to_string(),
        }])
    }
    async fn create_portfolio(&self, _ctx: &RepositoryContext, _name: &str) -> Result<Portfolio> {
        Err(anyhow!(
            "JSON repository does not support multiple portfolios"
        ))
    }
    async fn get_portfolio(
        &self,
        _ctx: &RepositoryContext,
        id_or_name: &str,
    ) -> Result<Option<Portfolio>> {
        if id_or_name == "default" || id_or_name == "Default Portfolio" {
            Ok(Some(Portfolio {
                id: "default".to_string(),
                name: "Default Portfolio".to_string(),
                description: None,
                owner_user_id: "local_user".to_string(),
                current_cash: 0.0,
                created_at: "".to_string(),
                updated_at: "".to_string(),
            }))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl DcaRepository for JsonRepository {
    async fn load_plans(&self, _ctx: &RepositoryContext) -> Result<Vec<DcaPlan>> {
        let path = self.dca_plans_path.clone();
        tokio::task::spawn_blocking(move || storage::dca_store::load_dca_plans(&path)).await?
    }
    async fn save_plans(&self, _ctx: &RepositoryContext, plans: &[DcaPlan]) -> Result<()> {
        let path = self.dca_plans_path.clone();
        let plans = plans.to_vec();
        tokio::task::spawn_blocking(move || storage::dca_store::save_dca_plans(&path, &plans))
            .await?
    }
    async fn load_settlements(&self, _ctx: &RepositoryContext) -> Result<Vec<DcaSettlement>> {
        let path = self.dca_settlements_path.clone();
        tokio::task::spawn_blocking(move || storage::dca_store::load_dca_settlements(&path)).await?
    }
    async fn save_settlements(
        &self,
        _ctx: &RepositoryContext,
        settlements: &[DcaSettlement],
    ) -> Result<()> {
        let path = self.dca_settlements_path.clone();
        let settlements = settlements.to_vec();
        tokio::task::spawn_blocking(move || {
            storage::dca_store::save_dca_settlements(&path, &settlements)
        })
        .await?
    }
    async fn load_settlement_audits(
        &self,
        _ctx: &RepositoryContext,
    ) -> Result<Vec<DcaSettlementAudit>> {
        let path = self.dca_settlement_audit_path.clone();
        tokio::task::spawn_blocking(move || storage::dca_store::load_dca_settlement_audits(&path))
            .await?
    }
    async fn save_settlement_audits(
        &self,
        _ctx: &RepositoryContext,
        audits: &[DcaSettlementAudit],
    ) -> Result<()> {
        let path = self.dca_settlement_audit_path.clone();
        let audits = audits.to_vec();
        tokio::task::spawn_blocking(move || {
            storage::dca_store::save_dca_settlement_audits(&path, &audits)
        })
        .await?
    }
}

#[async_trait]
impl ReconciliationRepository for JsonRepository {
    async fn load_alipay_snapshots(&self, _ctx: &RepositoryContext) -> Result<Vec<AlipaySnapshot>> {
        let path = self.alipay_snapshots_path.clone();
        tokio::task::spawn_blocking(move || {
            storage::reconciliation_store::load_alipay_snapshots(&path)
        })
        .await?
    }
    async fn save_alipay_snapshots(
        &self,
        _ctx: &RepositoryContext,
        snapshots: &[AlipaySnapshot],
    ) -> Result<()> {
        let path = self.alipay_snapshots_path.clone();
        let snapshots = snapshots.to_vec();
        tokio::task::spawn_blocking(move || {
            storage::reconciliation_store::save_alipay_snapshots(&path, &snapshots)
        })
        .await?
    }
    async fn load_reconciliation_audits(
        &self,
        _ctx: &RepositoryContext,
    ) -> Result<Vec<ReconciliationAudit>> {
        let path = self.reconciliation_audit_path.clone();
        tokio::task::spawn_blocking(move || {
            storage::reconciliation_store::load_reconciliation_audits(&path)
        })
        .await?
    }
    async fn save_reconciliation_audits(
        &self,
        _ctx: &RepositoryContext,
        audits: &[ReconciliationAudit],
    ) -> Result<()> {
        let path = self.reconciliation_audit_path.clone();
        let audits = audits.to_vec();
        tokio::task::spawn_blocking(move || {
            storage::reconciliation_store::save_reconciliation_audits(&path, &audits)
        })
        .await?
    }
}

#[async_trait]
impl InstrumentRepository for JsonRepository {
    async fn load_instruments(&self, _ctx: &RepositoryContext) -> Result<Vec<InstrumentConfig>> {
        let path = self.instruments_path.clone();
        tokio::task::spawn_blocking(move || storage::instrument_store::load_instruments(&path))
            .await?
    }
    async fn save_instruments(
        &self,
        _ctx: &RepositoryContext,
        instruments: &[InstrumentConfig],
    ) -> Result<()> {
        let path = self.instruments_path.clone();
        let instruments = instruments.to_vec();
        tokio::task::spawn_blocking(move || {
            storage::instrument_store::save_instruments(&path, &instruments)
        })
        .await?
    }
    async fn load_instrument_cache(
        &self,
        _ctx: &RepositoryContext,
    ) -> Result<InstrumentQuoteCache> {
        let path = self.instrument_cache_path.clone();
        tokio::task::spawn_blocking(move || {
            storage::instrument_cache_store::load_instrument_cache(&path)
        })
        .await?
    }
    async fn save_instrument_cache(
        &self,
        _ctx: &RepositoryContext,
        cache: &InstrumentQuoteCache,
    ) -> Result<()> {
        let path = self.instrument_cache_path.clone();
        let cache = cache.clone();
        tokio::task::spawn_blocking(move || {
            storage::instrument_cache_store::save_instrument_cache(&path, &cache)
        })
        .await?
    }
}

#[async_trait]
impl ReportRepository for JsonRepository {
    async fn save_portfolio_snapshots(
        &self,
        _ctx: &RepositoryContext,
        snapshots: &[PortfolioSnapshot],
    ) -> Result<()> {
        let path = self.snapshot_path.clone();
        let snapshots = snapshots.to_vec();
        tokio::task::spawn_blocking(move || {
            storage::snapshot_store::save_snapshots(&path, &snapshots)
        })
        .await?
    }
    async fn load_portfolio_snapshots(
        &self,
        _ctx: &RepositoryContext,
    ) -> Result<Vec<PortfolioSnapshot>> {
        let path = self.snapshot_path.clone();
        tokio::task::spawn_blocking(move || storage::snapshot_store::load_snapshots(&path)).await?
    }
    async fn save_markdown_report(
        &self,
        _ctx: &RepositoryContext,
        _path: &str,
        _content: &str,
    ) -> Result<()> {
        // For JSON repo, we don't necessarily persist MD reports in a specific place
        Ok(())
    }
    fn get_snapshot_path(&self) -> String {
        self.snapshot_path.clone()
    }
}

#[async_trait]
impl AuditRepository for JsonRepository {
    async fn append_web_admin_audit(
        &self,
        _ctx: &RepositoryContext,
        audit: WebAdminAudit,
    ) -> Result<()> {
        let path = self.web_audit_path.clone();
        tokio::task::spawn_blocking(move || {
            storage::web_audit_store::add_audit_record(&path, audit)
        })
        .await?
    }
    async fn load_web_admin_audit(&self, _ctx: &RepositoryContext) -> Result<WebAdminAuditLog> {
        let path = self.web_audit_path.clone();
        tokio::task::spawn_blocking(move || storage::web_audit_store::load_web_audit(&path)).await?
    }
}

#[async_trait]
impl OperationRepository for JsonRepository {
    async fn load_operation_policy(&self, _ctx: &RepositoryContext) -> Result<OperationPolicy> {
        let path = self.operation_policy_path.clone();
        tokio::task::spawn_blocking(move || storage::operation_store::load_operation_policy(&path))
            .await?
    }
    async fn save_operation_policy(
        &self,
        _ctx: &RepositoryContext,
        policy: &OperationPolicy,
    ) -> Result<()> {
        let path = self.operation_policy_path.clone();
        let policy = policy.clone();
        tokio::task::spawn_blocking(move || {
            storage::operation_store::save_operation_policy(&path, &policy)
        })
        .await?
    }
    async fn load_operation_status(&self, _ctx: &RepositoryContext) -> Result<OperationStatus> {
        let path = self.operation_status_path.clone();
        tokio::task::spawn_blocking(move || storage::operation_store::load_operation_status(&path))
            .await?
    }
    async fn save_operation_status(
        &self,
        _ctx: &RepositoryContext,
        status: &OperationStatus,
    ) -> Result<()> {
        let path = self.operation_status_path.clone();
        let status = status.clone();
        tokio::task::spawn_blocking(move || {
            storage::operation_store::save_operation_status(&path, &status)
        })
        .await?
    }

    async fn load_daily_operation_report(
        &self,
        _ctx: &RepositoryContext,
    ) -> Result<Option<DailyOperationReport>> {
        let path = self.daily_operation_report_path.clone();
        if !std::path::Path::new(&path).exists() {
            return Ok(None);
        }
        tokio::task::spawn_blocking(move || {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        })
        .await?
    }

    async fn save_daily_operation_report(
        &self,
        _ctx: &RepositoryContext,
        report: &DailyOperationReport,
    ) -> Result<()> {
        let path = self.daily_operation_report_path.clone();
        let report = report.clone();
        tokio::task::spawn_blocking(move || {
            let content = serde_json::to_string_pretty(&report)?;
            std::fs::write(path, content)?;
            Ok::<(), anyhow::Error>(())
        })
        .await?
    }
}

#[async_trait]
impl CacheRepository for JsonRepository {
    async fn load_cache_status(&self, _ctx: &RepositoryContext) -> Result<CacheStatusRegistry> {
        let path = self.cache_status_path.clone();
        tokio::task::spawn_blocking(move || storage::cache_status_store::load_cache_status(&path))
            .await?
    }
    async fn save_cache_status(
        &self,
        _ctx: &RepositoryContext,
        registry: &CacheStatusRegistry,
    ) -> Result<()> {
        let path = self.cache_status_path.clone();
        let registry = registry.clone();
        tokio::task::spawn_blocking(move || {
            storage::cache_status_store::save_cache_status(&path, &registry)
        })
        .await?
    }
    async fn load_risk_cache(&self, _ctx: &RepositoryContext) -> Result<Option<RiskCache>> {
        let path = self.risk_cache_path.clone();
        tokio::task::spawn_blocking(move || storage::risk_cache_store::load_risk_cache(&path))
            .await?
    }
    async fn save_risk_cache(&self, _ctx: &RepositoryContext, cache: &RiskCache) -> Result<()> {
        let path = self.risk_cache_path.clone();
        let cache = cache.clone();
        tokio::task::spawn_blocking(move || {
            storage::risk_cache_store::save_risk_cache(&path, &cache)
        })
        .await?
    }
    async fn load_proxy_cache(&self, _ctx: &RepositoryContext) -> Result<ProxyValuationCache> {
        let path = self.proxy_cache_path.clone();
        tokio::task::spawn_blocking(move || storage::proxy_cache_store::load_proxy_cache(&path))
            .await??
            .ok_or_else(|| anyhow!("Proxy cache not found"))
    }
    async fn save_proxy_cache(
        &self,
        _ctx: &RepositoryContext,
        cache: &ProxyValuationCache,
    ) -> Result<()> {
        let path = self.proxy_cache_path.clone();
        let cache = cache.clone();
        tokio::task::spawn_blocking(move || {
            storage::proxy_cache_store::save_proxy_cache(&path, &cache)
        })
        .await?
    }
    async fn load_regime_cache(&self, _ctx: &RepositoryContext) -> Result<RegimeCache> {
        let path = self.regime_cache_path.clone();
        tokio::task::spawn_blocking(move || storage::regime_cache_store::load_regime_cache(&path))
            .await?
    }
    async fn save_regime_cache(&self, _ctx: &RepositoryContext, cache: &RegimeCache) -> Result<()> {
        let path = self.regime_cache_path.clone();
        let cache = cache.clone();
        tokio::task::spawn_blocking(move || {
            storage::regime_cache_store::save_regime_cache(&path, &cache)
        })
        .await?
    }
    async fn load_market_cache(&self, _ctx: &RepositoryContext) -> Result<MarketCache> {
        let path = self.market_cache_path.clone();
        tokio::task::spawn_blocking(move || storage::market_cache_store::load_market_cache(&path))
            .await?
    }
    async fn save_market_cache(&self, _ctx: &RepositoryContext, cache: &MarketCache) -> Result<()> {
        let path = self.market_cache_path.clone();
        let cache = cache.clone();
        tokio::task::spawn_blocking(move || {
            storage::market_cache_store::save_market_cache(&path, &cache)
        })
        .await?
    }
    async fn load_fx_cache(&self, _ctx: &RepositoryContext) -> Result<FxCache> {
        let path = self.fx_cache_path.clone();
        tokio::task::spawn_blocking(move || storage::fx_cache_store::load_fx_cache(&path)).await?
    }
    async fn save_fx_cache(&self, _ctx: &RepositoryContext, cache: &FxCache) -> Result<()> {
        let path = self.fx_cache_path.clone();
        let cache = cache.clone();
        tokio::task::spawn_blocking(move || storage::fx_cache_store::save_fx_cache(&path, &cache))
            .await?
    }
    async fn load_nav_cache(&self, _ctx: &RepositoryContext) -> Result<NavCache> {
        let path = self.nav_cache_path.clone();
        tokio::task::spawn_blocking(move || storage::cache_store::load_cache(&path)).await?
    }
    async fn save_nav_cache(&self, _ctx: &RepositoryContext, cache: &NavCache) -> Result<()> {
        let path = self.nav_cache_path.clone();
        let cache = cache.clone();
        tokio::task::spawn_blocking(move || storage::cache_store::save_cache(&path, &cache)).await?
    }
}
