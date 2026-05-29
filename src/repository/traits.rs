use crate::models::*;
use crate::repository::RepositoryContext;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait PortfolioRepository: Send + Sync {
    fn name(&self) -> String;
    async fn load_config(&self, ctx: &RepositoryContext) -> Result<ConfigRoot>;
    async fn save_config(&self, ctx: &RepositoryContext, config: &ConfigRoot) -> Result<()>;
    async fn load_state(&self, ctx: &RepositoryContext) -> Result<PortfolioState>;
    async fn save_state(&self, ctx: &RepositoryContext, state: &PortfolioState) -> Result<()>;
    async fn load_transactions(&self, ctx: &RepositoryContext) -> Result<Vec<Transaction>>;
    async fn save_transactions(
        &self,
        ctx: &RepositoryContext,
        transactions: &[Transaction],
    ) -> Result<()>;
    async fn update_transaction(&self, ctx: &RepositoryContext, tx: &Transaction) -> Result<()>;
    async fn delete_transaction(&self, ctx: &RepositoryContext, id: &str) -> Result<()>;

    // Portfolio management
    async fn list_portfolios(&self, ctx: &RepositoryContext) -> Result<Vec<Portfolio>>;
    async fn create_portfolio(&self, ctx: &RepositoryContext, name: &str) -> Result<Portfolio>;
    async fn get_portfolio(
        &self,
        ctx: &RepositoryContext,
        id_or_name: &str,
    ) -> Result<Option<Portfolio>>;
}

#[async_trait]
pub trait DcaRepository: Send + Sync {
    async fn load_plans(&self, ctx: &RepositoryContext) -> Result<Vec<DcaPlan>>;
    async fn save_plans(&self, ctx: &RepositoryContext, plans: &[DcaPlan]) -> Result<()>;
    async fn load_settlements(&self, ctx: &RepositoryContext) -> Result<Vec<DcaSettlement>>;
    async fn save_settlements(
        &self,
        ctx: &RepositoryContext,
        settlements: &[DcaSettlement],
    ) -> Result<()>;
    async fn load_settlement_audits(
        &self,
        ctx: &RepositoryContext,
    ) -> Result<Vec<DcaSettlementAudit>>;
    async fn save_settlement_audits(
        &self,
        ctx: &RepositoryContext,
        audits: &[DcaSettlementAudit],
    ) -> Result<()>;
}

#[async_trait]
pub trait ReconciliationRepository: Send + Sync {
    async fn load_alipay_snapshots(&self, ctx: &RepositoryContext) -> Result<Vec<AlipaySnapshot>>;
    async fn save_alipay_snapshots(
        &self,
        ctx: &RepositoryContext,
        snapshots: &[AlipaySnapshot],
    ) -> Result<()>;
    async fn load_reconciliation_audits(
        &self,
        ctx: &RepositoryContext,
    ) -> Result<Vec<ReconciliationAudit>>;
    async fn save_reconciliation_audits(
        &self,
        ctx: &RepositoryContext,
        audits: &[ReconciliationAudit],
    ) -> Result<()>;
}

#[async_trait]
pub trait InstrumentRepository: Send + Sync {
    async fn load_instruments(&self, ctx: &RepositoryContext) -> Result<Vec<InstrumentConfig>>;
    async fn save_instruments(
        &self,
        ctx: &RepositoryContext,
        instruments: &[InstrumentConfig],
    ) -> Result<()>;
    async fn load_instrument_cache(&self, ctx: &RepositoryContext) -> Result<InstrumentQuoteCache>;
    async fn save_instrument_cache(
        &self,
        ctx: &RepositoryContext,
        cache: &InstrumentQuoteCache,
    ) -> Result<()>;
}

#[async_trait]
pub trait ReportRepository: Send + Sync {
    async fn load_portfolio_snapshots(
        &self,
        ctx: &RepositoryContext,
    ) -> Result<Vec<PortfolioSnapshot>>;
    async fn save_portfolio_snapshots(
        &self,
        ctx: &RepositoryContext,
        snapshots: &[PortfolioSnapshot],
    ) -> Result<()>;
    async fn save_markdown_report(
        &self,
        ctx: &RepositoryContext,
        content: &str,
        filename: &str,
    ) -> Result<()>;
    fn get_snapshot_path(&self) -> String;
}

#[async_trait]
pub trait AuditRepository: Send + Sync {
    async fn load_web_admin_audit(&self, ctx: &RepositoryContext) -> Result<WebAdminAuditLog>;
    async fn append_web_admin_audit(
        &self,
        ctx: &RepositoryContext,
        record: WebAdminAudit,
    ) -> Result<()>;
}

#[async_trait]
pub trait CacheRepository: Send + Sync {
    async fn load_cache_status(&self, ctx: &RepositoryContext) -> Result<CacheStatusRegistry>;
    async fn save_cache_status(
        &self,
        ctx: &RepositoryContext,
        registry: &CacheStatusRegistry,
    ) -> Result<()>;
    async fn load_risk_cache(&self, ctx: &RepositoryContext) -> Result<Option<RiskCache>>;
    async fn save_risk_cache(&self, ctx: &RepositoryContext, cache: &RiskCache) -> Result<()>;
    async fn load_proxy_cache(&self, ctx: &RepositoryContext) -> Result<ProxyValuationCache>;
    async fn save_proxy_cache(
        &self,
        ctx: &RepositoryContext,
        cache: &ProxyValuationCache,
    ) -> Result<()>;
    async fn load_regime_cache(&self, ctx: &RepositoryContext) -> Result<RegimeCache>;
    async fn save_regime_cache(&self, ctx: &RepositoryContext, cache: &RegimeCache) -> Result<()>;
    async fn load_market_cache(&self, ctx: &RepositoryContext) -> Result<MarketCache>;
    async fn save_market_cache(&self, ctx: &RepositoryContext, cache: &MarketCache) -> Result<()>;
    async fn load_fx_cache(&self, ctx: &RepositoryContext) -> Result<FxCache>;
    async fn save_fx_cache(&self, ctx: &RepositoryContext, cache: &FxCache) -> Result<()>;
    async fn load_nav_cache(&self, ctx: &RepositoryContext) -> Result<NavCache>;
    async fn save_nav_cache(&self, ctx: &RepositoryContext, cache: &NavCache) -> Result<()>;
}
