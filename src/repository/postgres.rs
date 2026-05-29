use crate::models::*;
use crate::repository::RepositoryContext;
use crate::repository::traits::*;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use sqlx::PgPool;

pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PortfolioRepository for PostgresRepository {
    async fn load_config(&self, _ctx: &RepositoryContext) -> Result<ConfigRoot> {
        Err(anyhow!("PostgresRepository::load_config not implemented"))
    }
    async fn save_config(&self, _ctx: &RepositoryContext, _config: &ConfigRoot) -> Result<()> {
        Err(anyhow!("PostgresRepository::save_config not implemented"))
    }
    async fn load_state(&self, ctx: &RepositoryContext) -> Result<PortfolioState> {
        let mut tx = self.pool.begin().await?;

        // Try to fetch the portfolio cash balance
        let row = sqlx::query("SELECT current_cash FROM portfolios WHERE id = $1")
            .bind(&ctx.portfolio_id)
            .fetch_optional(&mut *tx)
            .await?;

        let cash = if let Some(r) = row {
            use sqlx::Row;
            r.get("current_cash")
        } else {
            0.0
        };

        let rows = sqlx::query(
            r#"
            SELECT asset_id, fund_code, units, units_estimated, cost_basis, last_market_value, 
                   latest_nav, latest_nav_date, latest_nav_source, latest_nav_status
            FROM holdings
            WHERE portfolio_id = $1
            "#,
        )
        .bind(&ctx.portfolio_id)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;

        let mut asset_holdings = Vec::new();
        for r in rows {
            use sqlx::Row;

            let nav_date_opt: Option<chrono::NaiveDate> = r.get("latest_nav_date");

            asset_holdings.push(AssetHolding {
                asset_id: r.get("asset_id"),
                fund_code: r.get("fund_code"),
                units: r.get("units"),
                units_estimated: r.get("units_estimated"),
                cost_basis: r.get("cost_basis"),
                last_market_value: r.get("last_market_value"),
                latest_nav: r.get("latest_nav"),
                latest_nav_date: nav_date_opt.map(|d| d.format("%Y-%m-%d").to_string()),
                latest_nav_source: r.get("latest_nav_source"),
                latest_nav_status: r.get("latest_nav_status"),
            });
        }

        Ok(PortfolioState {
            cash,
            asset_holdings,
        })
    }
    async fn save_state(&self, ctx: &RepositoryContext, state: &PortfolioState) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Upsert portfolio cash
        sqlx::query(
            r#"
            INSERT INTO portfolios (id, current_cash)
            VALUES ($1, $2)
            ON CONFLICT (id) DO UPDATE SET
                current_cash = EXCLUDED.current_cash,
                updated_at = NOW()
            "#,
        )
        .bind(&ctx.portfolio_id)
        .bind(state.cash)
        .execute(&mut *tx)
        .await?;

        let asset_ids: Vec<&str> = state
            .asset_holdings
            .iter()
            .map(|h| h.asset_id.as_str())
            .collect();

        sqlx::query("DELETE FROM holdings WHERE portfolio_id = $1 AND asset_id != ALL($2)")
            .bind(&ctx.portfolio_id)
            .bind(&asset_ids)
            .execute(&mut *tx)
            .await?;

        for h in &state.asset_holdings {
            let nav_date = if let Some(d_str) = &h.latest_nav_date {
                Some(
                    chrono::NaiveDate::parse_from_str(d_str, "%Y-%m-%d").map_err(|e| {
                        anyhow!("Invalid date format in holding {}: {}", h.asset_id, e)
                    })?,
                )
            } else {
                None
            };

            sqlx::query(
                r#"
                INSERT INTO holdings (
                    portfolio_id, asset_id, fund_code, units, units_estimated, cost_basis, 
                    last_market_value, latest_nav, latest_nav_date, latest_nav_source, latest_nav_status
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                ON CONFLICT (portfolio_id, asset_id) DO UPDATE SET
                    fund_code = EXCLUDED.fund_code,
                    units = EXCLUDED.units,
                    units_estimated = EXCLUDED.units_estimated,
                    cost_basis = EXCLUDED.cost_basis,
                    last_market_value = EXCLUDED.last_market_value,
                    latest_nav = EXCLUDED.latest_nav,
                    latest_nav_date = EXCLUDED.latest_nav_date,
                    latest_nav_source = EXCLUDED.latest_nav_source,
                    latest_nav_status = EXCLUDED.latest_nav_status,
                    updated_at = NOW()
                "#
            )
            .bind(&ctx.portfolio_id)
            .bind(&h.asset_id)
            .bind(&h.fund_code)
            .bind(h.units)
            .bind(h.units_estimated)
            .bind(h.cost_basis)
            .bind(h.last_market_value)
            .bind(h.latest_nav)
            .bind(nav_date)
            .bind(&h.latest_nav_source)
            .bind(&h.latest_nav_status)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
    async fn load_transactions(&self, ctx: &RepositoryContext) -> Result<Vec<Transaction>> {
        let rows = sqlx::query(
            r#"
            SELECT id, transaction_date, transaction_type, asset_id, amount, units, price, fee, currency, note
            FROM transactions
            WHERE portfolio_id = $1
            ORDER BY transaction_date DESC, created_at DESC
            "#,
        )
        .bind(&ctx.portfolio_id)
        .fetch_all(&self.pool)
        .await?;

        let transactions = rows
            .into_iter()
            .map(|row| {
                use sqlx::Row;
                let id: String = row.get("id");
                let transaction_date: chrono::NaiveDate = row.get("transaction_date");
                let transaction_type: String = row.get("transaction_type");
                let asset_id: Option<String> = row.get("asset_id");
                let amount: f64 = row.get("amount");
                let units: Option<f64> = row.get("units");
                let price: Option<f64> = row.get("price");
                let fee: f64 = row.get("fee");
                let currency: String = row.get("currency");
                let note: String = row.get("note");

                Transaction {
                    id,
                    date: transaction_date.format("%Y-%m-%d").to_string(),
                    transaction_type,
                    asset_id,
                    amount,
                    units,
                    price,
                    fee,
                    currency,
                    note,
                }
            })
            .collect();

        Ok(transactions)
    }
    async fn save_transactions(
        &self,
        ctx: &RepositoryContext,
        transactions: &[Transaction],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for t in transactions {
            let date = chrono::NaiveDate::parse_from_str(&t.date, "%Y-%m-%d")
                .map_err(|e| anyhow!("Invalid date format in transaction {}: {}", t.id, e))?;

            sqlx::query(
                r#"
                INSERT INTO transactions (id, portfolio_id, transaction_date, transaction_type, asset_id, amount, units, price, fee, currency, note)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                ON CONFLICT (id) DO UPDATE SET
                    portfolio_id = EXCLUDED.portfolio_id,
                    transaction_date = EXCLUDED.transaction_date,
                    transaction_type = EXCLUDED.transaction_type,
                    asset_id = EXCLUDED.asset_id,
                    amount = EXCLUDED.amount,
                    units = EXCLUDED.units,
                    price = EXCLUDED.price,
                    fee = EXCLUDED.fee,
                    currency = EXCLUDED.currency,
                    note = EXCLUDED.note
                "#,
            )
            .bind(&t.id)
            .bind(&ctx.portfolio_id)
            .bind(date)
            .bind(&t.transaction_type)
            .bind(&t.asset_id)
            .bind(t.amount)
            .bind(t.units)
            .bind(t.price)
            .bind(t.fee)
            .bind(&t.currency)
            .bind(&t.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

#[async_trait]
impl DcaRepository for PostgresRepository {
    async fn load_plans(&self, _ctx: &RepositoryContext) -> Result<Vec<DcaPlan>> {
        Err(anyhow!("PostgresRepository::load_plans not implemented"))
    }
    async fn save_plans(&self, _ctx: &RepositoryContext, _plans: &[DcaPlan]) -> Result<()> {
        Err(anyhow!("PostgresRepository::save_plans not implemented"))
    }
    async fn load_settlements(&self, _ctx: &RepositoryContext) -> Result<Vec<DcaSettlement>> {
        Err(anyhow!(
            "PostgresRepository::load_settlements not implemented"
        ))
    }
    async fn save_settlements(
        &self,
        _ctx: &RepositoryContext,
        _settlements: &[DcaSettlement],
    ) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_settlements not implemented"
        ))
    }
    async fn load_settlement_audits(
        &self,
        _ctx: &RepositoryContext,
    ) -> Result<Vec<DcaSettlementAudit>> {
        Err(anyhow!(
            "PostgresRepository::load_settlement_audits not implemented"
        ))
    }
    async fn save_settlement_audits(
        &self,
        _ctx: &RepositoryContext,
        _audits: &[DcaSettlementAudit],
    ) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_settlement_audits not implemented"
        ))
    }
}

#[async_trait]
impl ReconciliationRepository for PostgresRepository {
    async fn load_alipay_snapshots(&self, _ctx: &RepositoryContext) -> Result<Vec<AlipaySnapshot>> {
        Err(anyhow!(
            "PostgresRepository::load_alipay_snapshots not implemented"
        ))
    }
    async fn save_alipay_snapshots(
        &self,
        _ctx: &RepositoryContext,
        _snapshots: &[AlipaySnapshot],
    ) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_alipay_snapshots not implemented"
        ))
    }
    async fn load_reconciliation_audits(
        &self,
        _ctx: &RepositoryContext,
    ) -> Result<Vec<ReconciliationAudit>> {
        Err(anyhow!(
            "PostgresRepository::load_reconciliation_audits not implemented"
        ))
    }
    async fn save_reconciliation_audits(
        &self,
        _ctx: &RepositoryContext,
        _audits: &[ReconciliationAudit],
    ) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_reconciliation_audits not implemented"
        ))
    }
}

#[async_trait]
impl InstrumentRepository for PostgresRepository {
    async fn load_instruments(&self, _ctx: &RepositoryContext) -> Result<Vec<InstrumentConfig>> {
        Err(anyhow!(
            "PostgresRepository::load_instruments not implemented"
        ))
    }
    async fn save_instruments(
        &self,
        _ctx: &RepositoryContext,
        _instruments: &[InstrumentConfig],
    ) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_instruments not implemented"
        ))
    }
    async fn load_instrument_cache(
        &self,
        _ctx: &RepositoryContext,
    ) -> Result<InstrumentQuoteCache> {
        Err(anyhow!(
            "PostgresRepository::load_instrument_cache not implemented"
        ))
    }
    async fn save_instrument_cache(
        &self,
        _ctx: &RepositoryContext,
        _cache: &InstrumentQuoteCache,
    ) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_instrument_cache not implemented"
        ))
    }
}

#[async_trait]
impl ReportRepository for PostgresRepository {
    async fn load_portfolio_snapshots(
        &self,
        _ctx: &RepositoryContext,
    ) -> Result<Vec<PortfolioSnapshot>> {
        Err(anyhow!(
            "PostgresRepository::load_portfolio_snapshots not implemented"
        ))
    }
    async fn save_portfolio_snapshots(
        &self,
        _ctx: &RepositoryContext,
        _snapshots: &[PortfolioSnapshot],
    ) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_portfolio_snapshots not implemented"
        ))
    }
    async fn save_markdown_report(
        &self,
        _ctx: &RepositoryContext,
        _content: &str,
        _filename: &str,
    ) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_markdown_report not implemented"
        ))
    }
    fn get_snapshot_path(&self) -> String {
        "postgres://snapshots".to_string()
    }
}

#[async_trait]
impl AuditRepository for PostgresRepository {
    async fn load_web_admin_audit(&self, _ctx: &RepositoryContext) -> Result<WebAdminAuditLog> {
        Err(anyhow!(
            "PostgresRepository::load_web_admin_audit not implemented"
        ))
    }
    async fn append_web_admin_audit(
        &self,
        _ctx: &RepositoryContext,
        _record: WebAdminAudit,
    ) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::append_web_admin_audit not implemented"
        ))
    }
}

#[async_trait]
impl CacheRepository for PostgresRepository {
    async fn load_cache_status(&self, _ctx: &RepositoryContext) -> Result<CacheStatusRegistry> {
        Err(anyhow!(
            "PostgresRepository::load_cache_status not implemented"
        ))
    }
    async fn save_cache_status(
        &self,
        _ctx: &RepositoryContext,
        _registry: &CacheStatusRegistry,
    ) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_cache_status not implemented"
        ))
    }
    async fn load_risk_cache(&self, _ctx: &RepositoryContext) -> Result<Option<RiskCache>> {
        Err(anyhow!(
            "PostgresRepository::load_risk_cache not implemented"
        ))
    }
    async fn save_risk_cache(&self, _ctx: &RepositoryContext, _cache: &RiskCache) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_risk_cache not implemented"
        ))
    }
    async fn load_proxy_cache(&self, _ctx: &RepositoryContext) -> Result<ProxyValuationCache> {
        Err(anyhow!(
            "PostgresRepository::load_proxy_cache not implemented"
        ))
    }
    async fn save_proxy_cache(
        &self,
        _ctx: &RepositoryContext,
        _cache: &ProxyValuationCache,
    ) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_proxy_cache not implemented"
        ))
    }
    async fn load_regime_cache(&self, _ctx: &RepositoryContext) -> Result<RegimeCache> {
        Err(anyhow!(
            "PostgresRepository::load_regime_cache not implemented"
        ))
    }
    async fn save_regime_cache(
        &self,
        _ctx: &RepositoryContext,
        _cache: &RegimeCache,
    ) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_regime_cache not implemented"
        ))
    }
    async fn load_market_cache(&self, _ctx: &RepositoryContext) -> Result<MarketCache> {
        Err(anyhow!(
            "PostgresRepository::load_market_cache not implemented"
        ))
    }
    async fn save_market_cache(
        &self,
        _ctx: &RepositoryContext,
        _cache: &MarketCache,
    ) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_market_cache not implemented"
        ))
    }
    async fn load_fx_cache(&self, _ctx: &RepositoryContext) -> Result<FxCache> {
        Err(anyhow!("PostgresRepository::load_fx_cache not implemented"))
    }
    async fn save_fx_cache(&self, _ctx: &RepositoryContext, _cache: &FxCache) -> Result<()> {
        Err(anyhow!("PostgresRepository::save_fx_cache not implemented"))
    }
    async fn load_nav_cache(&self, _ctx: &RepositoryContext) -> Result<NavCache> {
        Err(anyhow!(
            "PostgresRepository::load_nav_cache not implemented"
        ))
    }
    async fn save_nav_cache(&self, _ctx: &RepositoryContext, _cache: &NavCache) -> Result<()> {
        Err(anyhow!(
            "PostgresRepository::save_nav_cache not implemented"
        ))
    }
}
