use crate::models::*;
use crate::repository::RepositoryContext;
use crate::repository::traits::*;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PostgresRepository {
    pool: PgPool,
    config_path: String,
}

impl PostgresRepository {
    pub fn new(pool: PgPool, config_path: String) -> Self {
        Self { pool, config_path }
    }
}

#[async_trait]
impl PortfolioRepository for PostgresRepository {
    fn name(&self) -> String {
        "PostgreSQL".to_string()
    }
    async fn load_config(&self, _ctx: &RepositoryContext) -> Result<ConfigRoot> {
        let path = self.config_path.clone();
        tokio::task::spawn_blocking(move || crate::storage::load_config(&path)).await?
    }
    async fn save_config(&self, _ctx: &RepositoryContext, config: &ConfigRoot) -> Result<()> {
        let path = self.config_path.clone();
        let config = config.clone();
        tokio::task::spawn_blocking(move || crate::storage::save_config(&path, &config)).await?
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
            INSERT INTO portfolios (id, current_cash, owner_user_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE SET
                current_cash = EXCLUDED.current_cash,
                updated_at = NOW()
            "#,
        )
        .bind(&ctx.portfolio_id)
        .bind(state.cash)
        .bind(&ctx.actor_user_id)
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
            SELECT id, transaction_date, transaction_type, asset_id, amount, units, price, fee, currency, note, source, raw_description
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
                let source: String = row.get("source");
                let raw_description: String = row.get("raw_description");

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
                    source,
                    raw_description,
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
                INSERT INTO transactions (id, portfolio_id, transaction_date, transaction_type, asset_id, amount, units, price, fee, currency, note, source, raw_description)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
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
                    note = EXCLUDED.note,
                    source = EXCLUDED.source,
                    raw_description = EXCLUDED.raw_description
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
            .bind(&t.source)
            .bind(&t.raw_description)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn update_transaction(&self, ctx: &RepositoryContext, tx: &Transaction) -> Result<()> {
        let result = sqlx::query("SELECT 1 FROM transactions WHERE id = $1 AND portfolio_id = $2")
            .bind(&tx.id)
            .bind(&ctx.portfolio_id)
            .fetch_optional(&self.pool)
            .await?;

        if result.is_none() {
            anyhow::bail!(
                "Transaction {} not found in portfolio {}",
                tx.id,
                ctx.portfolio_id
            );
        }

        self.save_transactions(ctx, std::slice::from_ref(tx)).await
    }

    async fn delete_transaction(&self, ctx: &RepositoryContext, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM transactions WHERE id = $1 AND portfolio_id = $2")
            .bind(id)
            .bind(&ctx.portfolio_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!(
                "Transaction {} not found in portfolio {}",
                id,
                ctx.portfolio_id
            );
        }
        Ok(())
    }

    async fn list_portfolios(&self, ctx: &RepositoryContext) -> Result<Vec<Portfolio>> {
        let rows = sqlx::query(
            "SELECT id, name, description, owner_user_id, current_cash, created_at, updated_at FROM portfolios WHERE owner_user_id = $1"
        )
        .bind(&ctx.actor_user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut portfolios = Vec::new();
        for r in rows {
            use sqlx::Row;
            let created_at: chrono::DateTime<chrono::Utc> = r.get("created_at");
            let updated_at: chrono::DateTime<chrono::Utc> = r.get("updated_at");

            portfolios.push(Portfolio {
                id: r.get("id"),
                name: r
                    .get::<Option<String>, _>("name")
                    .unwrap_or_else(|| r.get("id")),
                description: r.get("description"),
                owner_user_id: r.get("owner_user_id"),
                current_cash: r.get("current_cash"),
                created_at: created_at.to_rfc3339(),
                updated_at: updated_at.to_rfc3339(),
            });
        }
        Ok(portfolios)
    }

    async fn create_portfolio(&self, ctx: &RepositoryContext, name: &str) -> Result<Portfolio> {
        let id = format!("p_{}", &Uuid::new_v4().to_string()[..8]);

        let row = sqlx::query(
            "INSERT INTO portfolios (id, name, owner_user_id) VALUES ($1, $2, $3) RETURNING id, name, description, owner_user_id, current_cash, created_at, updated_at"
        )
        .bind(&id)
        .bind(name)
        .bind(&ctx.actor_user_id)
        .fetch_one(&self.pool)
        .await?;

        use sqlx::Row;
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");

        Ok(Portfolio {
            id: row.get("id"),
            name: row
                .get::<Option<String>, _>("name")
                .unwrap_or_else(|| row.get("id")),
            description: row.get("description"),
            owner_user_id: row.get("owner_user_id"),
            current_cash: row.get("current_cash"),
            created_at: created_at.to_rfc3339(),
            updated_at: updated_at.to_rfc3339(),
        })
    }

    async fn get_portfolio(
        &self,
        ctx: &RepositoryContext,
        id_or_name: &str,
    ) -> Result<Option<Portfolio>> {
        let row = sqlx::query(
            "SELECT id, name, description, owner_user_id, current_cash, created_at, updated_at FROM portfolios WHERE (id = $1 OR name = $1) AND owner_user_id = $2"
        )
        .bind(id_or_name)
        .bind(&ctx.actor_user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(r) = row {
            use sqlx::Row;
            let created_at: chrono::DateTime<chrono::Utc> = r.get("created_at");
            let updated_at: chrono::DateTime<chrono::Utc> = r.get("updated_at");

            Ok(Some(Portfolio {
                id: r.get("id"),
                name: r
                    .get::<Option<String>, _>("name")
                    .unwrap_or_else(|| r.get("id")),
                description: r.get("description"),
                owner_user_id: r.get("owner_user_id"),
                current_cash: r.get("current_cash"),
                created_at: created_at.to_rfc3339(),
                updated_at: updated_at.to_rfc3339(),
            }))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl DcaRepository for PostgresRepository {
    async fn load_plans(&self, ctx: &RepositoryContext) -> Result<Vec<DcaPlan>> {
        let rows = sqlx::query(
            r#"
            SELECT plan_id, asset_id, fund_code, fund_name, amount, currency, frequency, weekday, month_day, start_date, end_date, enabled, priority, note
            FROM dca_plans
            WHERE portfolio_id = $1
            ORDER BY priority DESC, created_at ASC
            "#
        )
        .bind(&ctx.portfolio_id)
        .fetch_all(&self.pool)
        .await?;

        let mut plans = Vec::new();
        for r in rows {
            use sqlx::Row;
            let freq_str: String = r.get("frequency");
            let frequency = match freq_str.as_str() {
                "daily" => DcaFrequency::Daily,
                "weekly" => DcaFrequency::Weekly,
                "monthly" => DcaFrequency::Monthly,
                _ => DcaFrequency::Daily,
            };

            let sd: chrono::NaiveDate = r.get("start_date");
            let ed_opt: Option<chrono::NaiveDate> = r.get("end_date");

            plans.push(DcaPlan {
                plan_id: r.get("plan_id"),
                asset_id: r.get("asset_id"),
                fund_code: r.get("fund_code"),
                fund_name: r.get("fund_name"),
                amount: r.get("amount"),
                currency: r.get("currency"),
                frequency,
                weekday: r.get::<'_, Option<i32>, _>("weekday").map(|v| v as u32),
                month_day: r.get::<'_, Option<i32>, _>("month_day").map(|v| v as u32),
                start_date: sd.format("%Y-%m-%d").to_string(),
                end_date: ed_opt.map(|d| d.format("%Y-%m-%d").to_string()),
                enabled: r.get("enabled"),
                priority: r.get("priority"),
                note: r.get("note"),
            });
        }
        Ok(plans)
    }

    async fn save_plans(&self, ctx: &RepositoryContext, plans: &[DcaPlan]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for p in plans {
            let freq_str = match p.frequency {
                DcaFrequency::Daily => "daily",
                DcaFrequency::Weekly => "weekly",
                DcaFrequency::Monthly => "monthly",
            };

            let sd = chrono::NaiveDate::parse_from_str(&p.start_date, "%Y-%m-%d")
                .map_err(|e| anyhow!("Invalid start_date for plan {}: {}", p.plan_id, e))?;
            let ed = if let Some(d) = &p.end_date {
                Some(
                    chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
                        .map_err(|e| anyhow!("Invalid end_date for plan {}: {}", p.plan_id, e))?,
                )
            } else {
                None
            };

            sqlx::query(
                r#"
                INSERT INTO dca_plans (
                    plan_id, portfolio_id, asset_id, fund_code, fund_name, amount, currency, 
                    frequency, weekday, month_day, start_date, end_date, enabled, priority, note
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                ON CONFLICT (plan_id) DO UPDATE SET
                    portfolio_id = EXCLUDED.portfolio_id,
                    asset_id = EXCLUDED.asset_id,
                    fund_code = EXCLUDED.fund_code,
                    fund_name = EXCLUDED.fund_name,
                    amount = EXCLUDED.amount,
                    currency = EXCLUDED.currency,
                    frequency = EXCLUDED.frequency,
                    weekday = EXCLUDED.weekday,
                    month_day = EXCLUDED.month_day,
                    start_date = EXCLUDED.start_date,
                    end_date = EXCLUDED.end_date,
                    enabled = EXCLUDED.enabled,
                    priority = EXCLUDED.priority,
                    note = EXCLUDED.note,
                    updated_at = NOW()
                "#,
            )
            .bind(&p.plan_id)
            .bind(&ctx.portfolio_id)
            .bind(&p.asset_id)
            .bind(&p.fund_code)
            .bind(&p.fund_name)
            .bind(p.amount)
            .bind(&p.currency)
            .bind(freq_str)
            .bind(p.weekday.map(|v| v as i32))
            .bind(p.month_day.map(|v| v as i32))
            .bind(sd)
            .bind(ed)
            .bind(p.enabled)
            .bind(p.priority)
            .bind(&p.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn load_settlements(&self, ctx: &RepositoryContext) -> Result<Vec<DcaSettlement>> {
        let rows = sqlx::query(
            r#"
            SELECT settlement_id, plan_id, asset_id, fund_code, fund_name, scheduled_date, 
                   deduction_date, confirmation_date, amount, confirmed_nav, confirmed_units, 
                   fee, currency, source, status, applied, note, created_at
            FROM dca_settlements
            WHERE portfolio_id = $1
            ORDER BY deduction_date DESC, created_at DESC
            "#,
        )
        .bind(&ctx.portfolio_id)
        .fetch_all(&self.pool)
        .await?;

        let mut settlements = Vec::new();
        for r in rows {
            use sqlx::Row;
            let status_str: String = r.get("status");
            let status = match status_str.as_str() {
                "confirmed" => DcaSettlementStatus::Confirmed,
                "pending" => DcaSettlementStatus::Pending,
                "failed" => DcaSettlementStatus::Failed,
                _ => DcaSettlementStatus::Pending,
            };

            let sched_opt: Option<chrono::NaiveDate> = r.get("scheduled_date");
            let ded_date: chrono::NaiveDate = r.get("deduction_date");
            let conf_date: chrono::NaiveDate = r.get("confirmation_date");
            let created_at: chrono::DateTime<chrono::Utc> = r.get("created_at");

            settlements.push(DcaSettlement {
                settlement_id: r.get("settlement_id"),
                plan_id: r.get("plan_id"),
                asset_id: r.get("asset_id"),
                fund_code: r.get("fund_code"),
                fund_name: r.get("fund_name"),
                scheduled_date: sched_opt.map(|d| d.format("%Y-%m-%d").to_string()),
                deduction_date: ded_date.format("%Y-%m-%d").to_string(),
                confirmation_date: conf_date.format("%Y-%m-%d").to_string(),
                amount: r.get("amount"),
                confirmed_nav: r.get("confirmed_nav"),
                confirmed_units: r.get("confirmed_units"),
                fee: r.get("fee"),
                currency: r.get("currency"),
                source: r.get("source"),
                status,
                applied: r.get("applied"),
                note: r.get("note"),
                created_at: created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            });
        }
        Ok(settlements)
    }

    async fn save_settlements(
        &self,
        ctx: &RepositoryContext,
        settlements: &[DcaSettlement],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for s in settlements {
            let status_str = match s.status {
                DcaSettlementStatus::Confirmed => "confirmed",
                DcaSettlementStatus::Pending => "pending",
                DcaSettlementStatus::Failed => "failed",
            };

            let sched = if let Some(d) = &s.scheduled_date {
                Some(
                    chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").map_err(|e| {
                        anyhow!(
                            "Invalid scheduled_date for settlement {}: {}",
                            s.settlement_id,
                            e
                        )
                    })?,
                )
            } else {
                None
            };

            let ded =
                chrono::NaiveDate::parse_from_str(&s.deduction_date, "%Y-%m-%d").map_err(|e| {
                    anyhow!(
                        "Invalid deduction_date for settlement {}: {}",
                        s.settlement_id,
                        e
                    )
                })?;
            let conf = chrono::NaiveDate::parse_from_str(&s.confirmation_date, "%Y-%m-%d")
                .map_err(|e| {
                    anyhow!(
                        "Invalid confirmation_date for settlement {}: {}",
                        s.settlement_id,
                        e
                    )
                })?;

            sqlx::query(
                r#"
                INSERT INTO dca_settlements (
                    settlement_id, portfolio_id, plan_id, asset_id, fund_code, fund_name, scheduled_date, 
                    deduction_date, confirmation_date, amount, confirmed_nav, confirmed_units, fee, currency, 
                    source, status, applied, note
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
                ON CONFLICT (settlement_id) DO UPDATE SET
                    portfolio_id = EXCLUDED.portfolio_id,
                    plan_id = EXCLUDED.plan_id,
                    asset_id = EXCLUDED.asset_id,
                    fund_code = EXCLUDED.fund_code,
                    fund_name = EXCLUDED.fund_name,
                    scheduled_date = EXCLUDED.scheduled_date,
                    deduction_date = EXCLUDED.deduction_date,
                    confirmation_date = EXCLUDED.confirmation_date,
                    amount = EXCLUDED.amount,
                    confirmed_nav = EXCLUDED.confirmed_nav,
                    confirmed_units = EXCLUDED.confirmed_units,
                    fee = EXCLUDED.fee,
                    currency = EXCLUDED.currency,
                    source = EXCLUDED.source,
                    status = EXCLUDED.status,
                    applied = EXCLUDED.applied,
                    note = EXCLUDED.note
                "#
            )
            .bind(&s.settlement_id)
            .bind(&ctx.portfolio_id)
            .bind(&s.plan_id)
            .bind(&s.asset_id)
            .bind(&s.fund_code)
            .bind(&s.fund_name)
            .bind(sched)
            .bind(ded)
            .bind(conf)
            .bind(s.amount)
            .bind(s.confirmed_nav)
            .bind(s.confirmed_units)
            .bind(s.fee)
            .bind(&s.currency)
            .bind(&s.source)
            .bind(status_str)
            .bind(s.applied)
            .bind(&s.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn load_settlement_audits(
        &self,
        ctx: &RepositoryContext,
    ) -> Result<Vec<DcaSettlementAudit>> {
        let rows = sqlx::query(
            r#"
            SELECT audit_id, settlement_id, asset_id, old_units, new_units, old_cost_basis, new_cost_basis, transaction_id, note, created_at
            FROM dca_settlement_audits
            WHERE portfolio_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(&ctx.portfolio_id)
        .fetch_all(&self.pool)
        .await?;

        let mut audits = Vec::new();
        for r in rows {
            use sqlx::Row;
            let created_at: chrono::DateTime<chrono::Utc> = r.get("created_at");

            audits.push(DcaSettlementAudit {
                audit_id: r.get("audit_id"),
                timestamp: created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                settlement_id: r.get("settlement_id"),
                asset_id: r.get("asset_id"),
                old_units: r.get("old_units"),
                new_units: r.get("new_units"),
                old_cost_basis: r.get("old_cost_basis"),
                new_cost_basis: r.get("new_cost_basis"),
                transaction_id: r.get("transaction_id"),
                note: r.get("note"),
            });
        }
        Ok(audits)
    }

    async fn save_settlement_audits(
        &self,
        ctx: &RepositoryContext,
        audits: &[DcaSettlementAudit],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for a in audits {
            sqlx::query(
                r#"
                INSERT INTO dca_settlement_audits (
                    audit_id, portfolio_id, settlement_id, asset_id, old_units, new_units, old_cost_basis, new_cost_basis, transaction_id, note
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT (audit_id) DO UPDATE SET
                    portfolio_id = EXCLUDED.portfolio_id,
                    settlement_id = EXCLUDED.settlement_id,
                    asset_id = EXCLUDED.asset_id,
                    old_units = EXCLUDED.old_units,
                    new_units = EXCLUDED.new_units,
                    old_cost_basis = EXCLUDED.old_cost_basis,
                    new_cost_basis = EXCLUDED.new_cost_basis,
                    transaction_id = EXCLUDED.transaction_id,
                    note = EXCLUDED.note
                "#
            )
            .bind(&a.audit_id)
            .bind(&ctx.portfolio_id)
            .bind(&a.settlement_id)
            .bind(&a.asset_id)
            .bind(a.old_units)
            .bind(a.new_units)
            .bind(a.old_cost_basis)
            .bind(a.new_cost_basis)
            .bind(&a.transaction_id)
            .bind(&a.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

#[async_trait]
impl ReconciliationRepository for PostgresRepository {
    async fn load_alipay_snapshots(&self, ctx: &RepositoryContext) -> Result<Vec<AlipaySnapshot>> {
        let rows = sqlx::query(
            r#"
            SELECT snapshot_id, asset_id, fund_code, fund_name, snapshot_date, market_value, 
                   units, cost_basis, nav, nav_date, daily_pnl, total_pnl, source, note, created_at
            FROM alipay_snapshots
            WHERE portfolio_id = $1
            ORDER BY snapshot_date DESC, created_at DESC
            "#,
        )
        .bind(&ctx.portfolio_id)
        .fetch_all(&self.pool)
        .await?;

        let mut snapshots = Vec::new();
        for r in rows {
            use sqlx::Row;
            let snap_date: chrono::NaiveDate = r.get("snapshot_date");
            let nav_date_opt: Option<chrono::NaiveDate> = r.get("nav_date");
            let created_at: chrono::DateTime<chrono::Utc> = r.get("created_at");

            snapshots.push(AlipaySnapshot {
                snapshot_id: r.get("snapshot_id"),
                asset_id: r.get("asset_id"),
                fund_code: r.get("fund_code"),
                fund_name: r.get("fund_name"),
                snapshot_date: snap_date.format("%Y-%m-%d").to_string(),
                market_value: r.get("market_value"),
                units: r.get("units"),
                cost_basis: r.get("cost_basis"),
                nav: r.get("nav"),
                nav_date: nav_date_opt.map(|d| d.format("%Y-%m-%d").to_string()),
                daily_pnl: r.get("daily_pnl"),
                total_pnl: r.get("total_pnl"),
                source: r.get("source"),
                note: r.get("note"),
                created_at: created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            });
        }
        Ok(snapshots)
    }

    async fn save_alipay_snapshots(
        &self,
        ctx: &RepositoryContext,
        snapshots: &[AlipaySnapshot],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for s in snapshots {
            let snap_date = chrono::NaiveDate::parse_from_str(&s.snapshot_date, "%Y-%m-%d")
                .map_err(|e| anyhow!("Invalid snapshot_date {}: {}", s.snapshot_id, e))?;

            let nav_date = if let Some(d) = &s.nav_date {
                Some(
                    chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
                        .map_err(|e| anyhow!("Invalid nav_date {}: {}", s.snapshot_id, e))?,
                )
            } else {
                None
            };

            sqlx::query(
                r#"
                INSERT INTO alipay_snapshots (
                    snapshot_id, portfolio_id, asset_id, fund_code, fund_name, snapshot_date, market_value,
                    units, cost_basis, nav, nav_date, daily_pnl, total_pnl, source, note
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                ON CONFLICT (snapshot_id) DO UPDATE SET
                    portfolio_id = EXCLUDED.portfolio_id,
                    asset_id = EXCLUDED.asset_id,
                    fund_code = EXCLUDED.fund_code,
                    fund_name = EXCLUDED.fund_name,
                    snapshot_date = EXCLUDED.snapshot_date,
                    market_value = EXCLUDED.market_value,
                    units = EXCLUDED.units,
                    cost_basis = EXCLUDED.cost_basis,
                    nav = EXCLUDED.nav,
                    nav_date = EXCLUDED.nav_date,
                    daily_pnl = EXCLUDED.daily_pnl,
                    total_pnl = EXCLUDED.total_pnl,
                    source = EXCLUDED.source,
                    note = EXCLUDED.note
                "#
            )
            .bind(&s.snapshot_id)
            .bind(&ctx.portfolio_id)
            .bind(&s.asset_id)
            .bind(&s.fund_code)
            .bind(&s.fund_name)
            .bind(snap_date)
            .bind(s.market_value)
            .bind(s.units)
            .bind(s.cost_basis)
            .bind(s.nav)
            .bind(nav_date)
            .bind(s.daily_pnl)
            .bind(s.total_pnl)
            .bind(&s.source)
            .bind(&s.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn load_reconciliation_audits(
        &self,
        ctx: &RepositoryContext,
    ) -> Result<Vec<ReconciliationAudit>> {
        let rows = sqlx::query(
            r#"
            SELECT audit_id, snapshot_id, asset_id, old_units, new_units, old_cost_basis, new_cost_basis, 
                   old_market_value, new_market_value, reason, note, created_at
            FROM reconciliation_audits
            WHERE portfolio_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(&ctx.portfolio_id)
        .fetch_all(&self.pool)
        .await?;

        let mut audits = Vec::new();
        for r in rows {
            use sqlx::Row;
            let created_at: chrono::DateTime<chrono::Utc> = r.get("created_at");

            audits.push(ReconciliationAudit {
                audit_id: r.get("audit_id"),
                timestamp: created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                snapshot_id: r.get("snapshot_id"),
                asset_id: r.get("asset_id"),
                old_units: r.get("old_units"),
                new_units: r.get("new_units"),
                old_cost_basis: r.get("old_cost_basis"),
                new_cost_basis: r.get("new_cost_basis"),
                old_market_value: r.get("old_market_value"),
                new_market_value: r.get("new_market_value"),
                reason: r.get("reason"),
                note: r.get("note"),
            });
        }
        Ok(audits)
    }

    async fn save_reconciliation_audits(
        &self,
        ctx: &RepositoryContext,
        audits: &[ReconciliationAudit],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for a in audits {
            sqlx::query(
                r#"
                INSERT INTO reconciliation_audits (
                    audit_id, portfolio_id, snapshot_id, asset_id, old_units, new_units, 
                    old_cost_basis, new_cost_basis, old_market_value, new_market_value, reason, note
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                ON CONFLICT (audit_id) DO UPDATE SET
                    portfolio_id = EXCLUDED.portfolio_id,
                    snapshot_id = EXCLUDED.snapshot_id,
                    asset_id = EXCLUDED.asset_id,
                    old_units = EXCLUDED.old_units,
                    new_units = EXCLUDED.new_units,
                    old_cost_basis = EXCLUDED.old_cost_basis,
                    new_cost_basis = EXCLUDED.new_cost_basis,
                    old_market_value = EXCLUDED.old_market_value,
                    new_market_value = EXCLUDED.new_market_value,
                    reason = EXCLUDED.reason,
                    note = EXCLUDED.note
                "#,
            )
            .bind(&a.audit_id)
            .bind(&ctx.portfolio_id)
            .bind(&a.snapshot_id)
            .bind(&a.asset_id)
            .bind(a.old_units)
            .bind(a.new_units)
            .bind(a.old_cost_basis)
            .bind(a.new_cost_basis)
            .bind(a.old_market_value)
            .bind(a.new_market_value)
            .bind(&a.reason)
            .bind(&a.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

#[async_trait]
impl InstrumentRepository for PostgresRepository {
    async fn load_instruments(&self, _ctx: &RepositoryContext) -> Result<Vec<InstrumentConfig>> {
        let rows = sqlx::query(
            r#"
            SELECT instrument_id, symbol, display_symbol, name, name_zh, name_en, description_zh, category_zh, display_label, 
                   asset_class, provider, provider_symbol, market, exchange, currency, quote_unit, price_unit, timezone, enabled, priority, tags, note
            FROM instruments
            ORDER BY priority DESC, symbol ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut instruments = Vec::new();
        for r in rows {
            use sqlx::Row;
            let asset_class_str: String = r.get("asset_class");
            let asset_class = serde_json::from_str(&format!("\"{}\"", asset_class_str))
                .unwrap_or(AssetClass::Custom);

            let tags_json: serde_json::Value = r.get("tags");
            let tags = tags_json
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            instruments.push(InstrumentConfig {
                instrument_id: r.get("instrument_id"),
                symbol: r.get("symbol"),
                display_symbol: r.get("display_symbol"),
                name: r.get("name"),
                name_zh: r.get("name_zh"),
                name_en: r.get("name_en"),
                description_zh: r.get("description_zh"),
                category_zh: r.get("category_zh"),
                display_label: r.get("display_label"),
                asset_class,
                provider: r.get("provider"),
                provider_symbol: r.get("provider_symbol"),
                market: r.get("market"),
                exchange: r.get("exchange"),
                currency: r.get("currency"),
                quote_unit: r.get("quote_unit"),
                price_unit: r.get("price_unit"),
                timezone: r.get("timezone"),
                enabled: r.get("enabled"),
                priority: r.get("priority"),
                tags,
                note: r.get("note"),
            });
        }
        Ok(instruments)
    }

    async fn save_instruments(
        &self,
        _ctx: &RepositoryContext,
        instruments: &[InstrumentConfig],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for i in instruments {
            let asset_class_str = serde_json::to_string(&i.asset_class)
                .unwrap_or_else(|_| "\"custom\"".to_string())
                .trim_matches('"')
                .to_string();
            let tags_json =
                serde_json::to_value(&i.tags).unwrap_or(serde_json::Value::Array(vec![]));

            sqlx::query(
                r#"
                INSERT INTO instruments (
                    instrument_id, symbol, display_symbol, name, name_zh, name_en, description_zh, category_zh, display_label, 
                    asset_class, provider, provider_symbol, market, exchange, currency, quote_unit, price_unit, timezone, enabled, priority, tags, note
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)
                ON CONFLICT (instrument_id) DO UPDATE SET
                    symbol = EXCLUDED.symbol,
                    display_symbol = EXCLUDED.display_symbol,
                    name = EXCLUDED.name,
                    name_zh = EXCLUDED.name_zh,
                    name_en = EXCLUDED.name_en,
                    description_zh = EXCLUDED.description_zh,
                    category_zh = EXCLUDED.category_zh,
                    display_label = EXCLUDED.display_label,
                    asset_class = EXCLUDED.asset_class,
                    provider = EXCLUDED.provider,
                    provider_symbol = EXCLUDED.provider_symbol,
                    market = EXCLUDED.market,
                    exchange = EXCLUDED.exchange,
                    currency = EXCLUDED.currency,
                    quote_unit = EXCLUDED.quote_unit,
                    price_unit = EXCLUDED.price_unit,
                    timezone = EXCLUDED.timezone,
                    enabled = EXCLUDED.enabled,
                    priority = EXCLUDED.priority,
                    tags = EXCLUDED.tags,
                    note = EXCLUDED.note,
                    updated_at = NOW()
                "#
            )
            .bind(&i.instrument_id)
            .bind(&i.symbol)
            .bind(&i.display_symbol)
            .bind(&i.name)
            .bind(&i.name_zh)
            .bind(&i.name_en)
            .bind(&i.description_zh)
            .bind(&i.category_zh)
            .bind(&i.display_label)
            .bind(asset_class_str)
            .bind(&i.provider)
            .bind(&i.provider_symbol)
            .bind(&i.market)
            .bind(&i.exchange)
            .bind(&i.currency)
            .bind(&i.quote_unit)
            .bind(&i.price_unit)
            .bind(&i.timezone)
            .bind(i.enabled)
            .bind(i.priority)
            .bind(tags_json)
            .bind(&i.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn load_instrument_cache(
        &self,
        _ctx: &RepositoryContext,
    ) -> Result<InstrumentQuoteCache> {
        let rows = sqlx::query(
            r#"
            SELECT instrument_id, symbol, name_zh, price, date, currency, quote_unit, provider, source, status, warning, fetched_at
            FROM cache_instruments
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut entries = Vec::new();
        for r in rows {
            use sqlx::Row;
            let date: chrono::NaiveDate = r.get("date");
            let fetched_at: chrono::DateTime<chrono::Utc> = r.get("fetched_at");

            entries.push(InstrumentQuoteCacheEntry {
                instrument_id: r.get("instrument_id"),
                symbol: r.get("symbol"),
                name_zh: r.get("name_zh"),
                price: r.get("price"),
                date: date.format("%Y-%m-%d").to_string(),
                currency: r.get("currency"),
                quote_unit: r.get("quote_unit"),
                provider: r.get("provider"),
                source: r.get("source"),
                status: r.get("status"),
                warning: r.get("warning"),
                fetched_at: fetched_at.to_rfc3339(),
            });
        }

        let fetched_at = if entries.is_empty() {
            chrono::Utc::now().to_rfc3339()
        } else {
            entries[0].fetched_at.clone()
        };

        Ok(InstrumentQuoteCache {
            entries,
            fetched_at,
        })
    }

    async fn save_instrument_cache(
        &self,
        _ctx: &RepositoryContext,
        cache: &InstrumentQuoteCache,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for e in &cache.entries {
            let date = chrono::NaiveDate::parse_from_str(&e.date, "%Y-%m-%d")
                .map_err(|err| anyhow!("Invalid date {}: {}", e.instrument_id, err))?;
            let fetched_at = chrono::DateTime::parse_from_rfc3339(&e.fetched_at)
                .map_err(|err| anyhow!("Invalid fetched_at {}: {}", e.instrument_id, err))?;

            sqlx::query(
                r#"
                INSERT INTO cache_instruments (
                    instrument_id, symbol, name_zh, price, date, currency, quote_unit, provider, source, status, warning, fetched_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                ON CONFLICT (instrument_id) DO UPDATE SET
                    symbol = EXCLUDED.symbol,
                    name_zh = EXCLUDED.name_zh,
                    price = EXCLUDED.price,
                    date = EXCLUDED.date,
                    currency = EXCLUDED.currency,
                    quote_unit = EXCLUDED.quote_unit,
                    provider = EXCLUDED.provider,
                    source = EXCLUDED.source,
                    status = EXCLUDED.status,
                    warning = EXCLUDED.warning,
                    fetched_at = EXCLUDED.fetched_at
                "#
            )
            .bind(&e.instrument_id)
            .bind(&e.symbol)
            .bind(&e.name_zh)
            .bind(e.price)
            .bind(date)
            .bind(&e.currency)
            .bind(&e.quote_unit)
            .bind(&e.provider)
            .bind(&e.source)
            .bind(&e.status)
            .bind(&e.warning)
            .bind(fetched_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
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
        Ok(CacheStatusRegistry::default())
    }
    async fn save_cache_status(
        &self,
        _ctx: &RepositoryContext,
        _registry: &CacheStatusRegistry,
    ) -> Result<()> {
        Ok(())
    }
    async fn load_risk_cache(&self, _ctx: &RepositoryContext) -> Result<Option<RiskCache>> {
        Ok(None)
    }
    async fn save_risk_cache(&self, _ctx: &RepositoryContext, _cache: &RiskCache) -> Result<()> {
        Ok(())
    }
    async fn load_proxy_cache(&self, _ctx: &RepositoryContext) -> Result<ProxyValuationCache> {
        Ok(ProxyValuationCache {
            results: vec![],
            fetched_at: "never".to_string(),
        })
    }
    async fn save_proxy_cache(
        &self,
        _ctx: &RepositoryContext,
        _cache: &ProxyValuationCache,
    ) -> Result<()> {
        Ok(())
    }
    async fn load_regime_cache(&self, _ctx: &RepositoryContext) -> Result<RegimeCache> {
        Ok(RegimeCache::default())
    }
    async fn save_regime_cache(
        &self,
        _ctx: &RepositoryContext,
        _cache: &RegimeCache,
    ) -> Result<()> {
        Ok(())
    }
    async fn load_market_cache(&self, _ctx: &RepositoryContext) -> Result<MarketCache> {
        Ok(MarketCache::default())
    }
    async fn save_market_cache(
        &self,
        _ctx: &RepositoryContext,
        _cache: &MarketCache,
    ) -> Result<()> {
        Ok(())
    }
    async fn load_fx_cache(&self, _ctx: &RepositoryContext) -> Result<FxCache> {
        Ok(FxCache::default())
    }
    async fn save_fx_cache(&self, _ctx: &RepositoryContext, _cache: &FxCache) -> Result<()> {
        Ok(())
    }
    async fn load_nav_cache(&self, _ctx: &RepositoryContext) -> Result<NavCache> {
        Ok(NavCache::default())
    }
    async fn save_nav_cache(&self, _ctx: &RepositoryContext, _cache: &NavCache) -> Result<()> {
        Ok(())
    }
}
