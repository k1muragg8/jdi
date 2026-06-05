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
    database_url_source: String,
}

impl PostgresRepository {
    pub fn new(pool: PgPool, config_path: String, database_url_source: String) -> Self {
        Self {
            pool,
            config_path,
            database_url_source,
        }
    }
}

#[async_trait]
impl PortfolioRepository for PostgresRepository {
    fn name(&self) -> String {
        "PostgreSQL".to_string()
    }

    async fn get_db_status(&self, _ctx: &RepositoryContext) -> Result<DbStatus> {
        let db_name: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&self.pool)
            .await?;
        let schema: String = sqlx::query_scalar("SELECT current_schema()")
            .fetch_one(&self.pool)
            .await?;
        let user: String = sqlx::query_scalar("SELECT current_user")
            .fetch_one(&self.pool)
            .await?;

        let host_opt: Option<String> = sqlx::query_scalar("SELECT inet_server_addr()::text")
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);
        let port_opt: Option<i32> = sqlx::query_scalar("SELECT inet_server_port()")
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);

        let mut tables = Vec::new();

        async fn count_table(pool: &PgPool, name: &str, table: &str) -> TableCount {
            let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table))
                .fetch_one(pool)
                .await
                .unwrap_or(0);
            TableCount {
                name: name.to_string(),
                count,
            }
        }

        tables.push(count_table(&self.pool, "portfolios", "portfolios").await);
        tables.push(count_table(&self.pool, "holdings", "holdings").await);
        tables.push(count_table(&self.pool, "transactions", "transactions").await);
        tables.push(count_table(&self.pool, "dca_plans", "dca_plans").await);
        tables.push(count_table(&self.pool, "alipay_snapshots", "alipay_snapshots").await);
        tables.push(count_table(&self.pool, "instruments", "instruments").await);
        tables.push(count_table(&self.pool, "web_admin_audit_logs", "web_admin_audit_logs").await);

        let mut portfolio_records = Vec::new();
        async fn count_portfolio_table(
            pool: &PgPool,
            name: &str,
            table: &str,
            portfolio_id: &str,
        ) -> TableCount {
            let count: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM {} WHERE portfolio_id = $1",
                table
            ))
            .bind(portfolio_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);
            TableCount {
                name: name.to_string(),
                count,
            }
        }

        portfolio_records.push(
            count_portfolio_table(&self.pool, "holdings", "holdings", &_ctx.portfolio_id).await,
        );
        portfolio_records.push(
            count_portfolio_table(
                &self.pool,
                "transactions",
                "transactions",
                &_ctx.portfolio_id,
            )
            .await,
        );
        portfolio_records.push(
            count_portfolio_table(&self.pool, "dca_plans", "dca_plans", &_ctx.portfolio_id).await,
        );

        Ok(DbStatus {
            backend: "PostgreSQL".to_string(),
            database_url_source: self.database_url_source.clone(),
            database_name: Some(db_name),
            schema: Some(schema),
            user: Some(user),
            host: host_opt,
            port: port_opt.map(|p| p as u16),
            fallback: false,
            data_dir: None,
            tables,
            migrations_active: true,
            active_portfolio_id: _ctx.portfolio_id.clone(),
            portfolio_records,
        })
    }

    async fn load_config(&self, _ctx: &RepositoryContext) -> Result<ConfigRoot> {
        let row = sqlx::query("SELECT value FROM application_metadata WHERE key = 'config_root'")
            .fetch_optional(&self.pool)
            .await?;

        if let Some(r) = row {
            use sqlx::Row;
            let value: serde_json::Value = r.get("value");
            Ok(serde_json::from_value(value)?)
        } else {
            let path = self.config_path.clone();
            let mut config =
                tokio::task::spawn_blocking(move || crate::storage::load_config(&path)).await??;

            // Do not seed demo assets into real portfolio
            config
                .assets
                .retain(|a| a.fund_name != "华夏成长混合" && a.fund_name != "Demo Asset");

            let value = serde_json::to_value(&config)?;
            sqlx::query(
                "INSERT INTO application_metadata (key, value) VALUES ('config_root', $1) ON CONFLICT (key) DO NOTHING"
            )
            .bind(value)
            .execute(&self.pool)
            .await?;

            Ok(config)
        }
    }
    async fn save_config(&self, _ctx: &RepositoryContext, config: &ConfigRoot) -> Result<()> {
        let value = serde_json::to_value(config)?;
        sqlx::query(
            "INSERT INTO application_metadata (key, value) VALUES ('config_root', $1) ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()"
        )
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
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
            SELECT plan_id, asset_id, fund_code, fund_name, amount, currency, frequency, weekday, month_day, start_date, end_date, enabled, priority, note, created_at, updated_at
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

            let ca: chrono::DateTime<chrono::Utc> = r.get("created_at");
            let ua: chrono::DateTime<chrono::Utc> = r.get("updated_at");

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
                created_at: ca.to_rfc3339(),
                updated_at: ua.to_rfc3339(),
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

            let ca = chrono::DateTime::parse_from_rfc3339(&p.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            sqlx::query(
                r#"
                INSERT INTO dca_plans (
                    plan_id, portfolio_id, asset_id, fund_code, fund_name, amount, currency, 
                    frequency, weekday, month_day, start_date, end_date, enabled, priority, note,
                    created_at, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
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
            .bind(ca)
            .bind(chrono::Utc::now())
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
                   asset_class, provider, provider_symbol, market, exchange, currency, quote_unit, price_unit, timezone, enabled, archived, priority, tags, note
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
                archived: r.get("archived"),
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
                    asset_class, provider, provider_symbol, market, exchange, currency, quote_unit, price_unit, timezone, enabled, archived, priority, tags, note
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
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
                    archived = EXCLUDED.archived,
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
            .bind(i.archived)
            .bind(i.priority)
            .bind(tags_json)
            .bind(&i.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn delete_instrument(&self, _ctx: &RepositoryContext, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM instruments WHERE instrument_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
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
    async fn load_web_admin_audit(&self, ctx: &RepositoryContext) -> Result<WebAdminAuditLog> {
        let rows = sqlx::query(
            r#"
            SELECT audit_id, timestamp, actor, actor_user_id, target_user_id, portfolio_id,
                   role, action, target_file, target_id, old_value_summary, new_value_summary,
                   status, note
            FROM web_admin_audit_logs
            WHERE portfolio_id = $1
            ORDER BY timestamp DESC
            LIMIT 100
            "#,
        )
        .bind(&ctx.portfolio_id)
        .fetch_all(&self.pool)
        .await?;

        let mut records = Vec::new();
        for r in rows {
            use sqlx::Row;
            records.push(WebAdminAudit {
                audit_id: r.get("audit_id"),
                timestamp: r.get("timestamp"),
                actor: r.get("actor"),
                actor_user_id: r.get("actor_user_id"),
                target_user_id: r.get("target_user_id"),
                portfolio_id: r.get("portfolio_id"),
                role: r.get("role"),
                action: r.get("action"),
                target_file: r.get("target_file"),
                target_id: r.get("target_id"),
                old_value_summary: r.get("old_value_summary"),
                new_value_summary: r.get("new_value_summary"),
                status: r.get("status"),
                note: r.get("note"),
            });
        }

        Ok(WebAdminAuditLog { records })
    }

    async fn append_web_admin_audit(
        &self,
        _ctx: &RepositoryContext,
        record: WebAdminAudit,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO web_admin_audit_logs (
                audit_id, timestamp, actor, actor_user_id, target_user_id, portfolio_id,
                role, action, target_file, target_id, old_value_summary, new_value_summary,
                status, note
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(&record.audit_id)
        .bind(&record.timestamp)
        .bind(&record.actor)
        .bind(&record.actor_user_id)
        .bind(&record.target_user_id)
        .bind(&record.portfolio_id)
        .bind(&record.role)
        .bind(&record.action)
        .bind(&record.target_file)
        .bind(&record.target_id)
        .bind(&record.old_value_summary)
        .bind(&record.new_value_summary)
        .bind(&record.status)
        .bind(&record.note)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl OperationRepository for PostgresRepository {
    async fn load_operation_policy(&self, ctx: &RepositoryContext) -> Result<OperationPolicy> {
        let row = sqlx::query(
            r#"
            SELECT target_total_investment_amount, target_equity_weight, min_cash_reserve, 
                   max_daily_buy_amount, max_single_asset_buy_amount, max_single_asset_weight, 
                   max_sector_weight, dca_auto_pause_when_target_reached, 
                   dca_auto_resume_when_below_target, dca_resume_threshold, dca_pause_threshold, 
                   kelly_enabled, max_kelly_fraction, pendulum_enabled, volatility_window_days,
                   risk_overlay_enabled, market_refresh_interval_seconds,
                   target_asset_weights_json, target_sector_weights_json
            FROM operation_policies
            WHERE portfolio_id = $1
            "#,
        )
        .bind(&ctx.portfolio_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(r) = row {
            use sqlx::Row;
            let target_asset_weights = r
                .get::<Option<String>, _>("target_asset_weights_json")
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let target_sector_weights = r
                .get::<Option<String>, _>("target_sector_weights_json")
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            Ok(OperationPolicy {
                target_total_investment_amount: r.get("target_total_investment_amount"),
                target_equity_weight: r.get("target_equity_weight"),
                min_cash_reserve: r.get("min_cash_reserve"),
                max_daily_buy_amount: r.get("max_daily_buy_amount"),
                max_single_asset_buy_amount: r.get("max_single_asset_buy_amount"),
                max_single_asset_weight: r.get("max_single_asset_weight"),
                max_sector_weight: r.get("max_sector_weight"),
                target_asset_weights,
                target_sector_weights,
                dca_auto_pause_when_target_reached: r.get("dca_auto_pause_when_target_reached"),
                dca_auto_resume_when_below_target: r.get("dca_auto_resume_when_below_target"),
                dca_resume_threshold: r.get("dca_resume_threshold"),
                dca_pause_threshold: r.get("dca_pause_threshold"),
                kelly_enabled: r.get("kelly_enabled"),
                max_kelly_fraction: r.get("max_kelly_fraction"),
                pendulum_enabled: r.get("pendulum_enabled"),
                volatility_window_days: r.get::<i32, _>("volatility_window_days") as usize,
                risk_overlay_enabled: r.get("risk_overlay_enabled"),
                market_refresh_interval_seconds: r.get::<i32, _>("market_refresh_interval_seconds")
                    as u64,
            })
        } else {
            Ok(OperationPolicy::default())
        }
    }
    async fn save_operation_policy(
        &self,
        ctx: &RepositoryContext,
        policy: &OperationPolicy,
    ) -> Result<()> {
        let target_asset_weights_json = serde_json::to_string(&policy.target_asset_weights)?;
        let target_sector_weights_json = serde_json::to_string(&policy.target_sector_weights)?;

        sqlx::query(
            r#"
            INSERT INTO operation_policies (
                portfolio_id, target_total_investment_amount, target_equity_weight, min_cash_reserve, 
                max_daily_buy_amount, max_single_asset_buy_amount, max_single_asset_weight, 
                max_sector_weight, dca_auto_pause_when_target_reached, 
                dca_auto_resume_when_below_target, dca_resume_threshold, dca_pause_threshold, 
                kelly_enabled, max_kelly_fraction, pendulum_enabled, volatility_window_days,
                risk_overlay_enabled, market_refresh_interval_seconds,
                target_asset_weights_json, target_sector_weights_json
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
            ON CONFLICT (portfolio_id) DO UPDATE SET
                target_total_investment_amount = EXCLUDED.target_total_investment_amount,
                target_equity_weight = EXCLUDED.target_equity_weight,
                min_cash_reserve = EXCLUDED.min_cash_reserve,
                max_daily_buy_amount = EXCLUDED.max_daily_buy_amount,
                max_single_asset_buy_amount = EXCLUDED.max_single_asset_buy_amount,
                max_single_asset_weight = EXCLUDED.max_single_asset_weight,
                max_sector_weight = EXCLUDED.max_sector_weight,
                dca_auto_pause_when_target_reached = EXCLUDED.dca_auto_pause_when_target_reached,
                dca_auto_resume_when_below_target = EXCLUDED.dca_auto_resume_when_below_target,
                dca_resume_threshold = EXCLUDED.dca_resume_threshold,
                dca_pause_threshold = EXCLUDED.dca_pause_threshold,
                kelly_enabled = EXCLUDED.kelly_enabled,
                max_kelly_fraction = EXCLUDED.max_kelly_fraction,
                pendulum_enabled = EXCLUDED.pendulum_enabled,
                volatility_window_days = EXCLUDED.volatility_window_days,
                risk_overlay_enabled = EXCLUDED.risk_overlay_enabled,
                market_refresh_interval_seconds = EXCLUDED.market_refresh_interval_seconds,
                target_asset_weights_json = EXCLUDED.target_asset_weights_json,
                target_sector_weights_json = EXCLUDED.target_sector_weights_json,
                updated_at = NOW()
            "#,
        )
        .bind(&ctx.portfolio_id)
        .bind(policy.target_total_investment_amount)
        .bind(policy.target_equity_weight)
        .bind(policy.min_cash_reserve)
        .bind(policy.max_daily_buy_amount)
        .bind(policy.max_single_asset_buy_amount)
        .bind(policy.max_single_asset_weight)
        .bind(policy.max_sector_weight)
        .bind(policy.dca_auto_pause_when_target_reached)
        .bind(policy.dca_auto_resume_when_below_target)
        .bind(policy.dca_resume_threshold)
        .bind(policy.dca_pause_threshold)
        .bind(policy.kelly_enabled)
        .bind(policy.max_kelly_fraction)
        .bind(policy.pendulum_enabled)
        .bind(policy.volatility_window_days as i32)
        .bind(policy.risk_overlay_enabled)
        .bind(policy.market_refresh_interval_seconds as i32)
        .bind(target_asset_weights_json)
        .bind(target_sector_weights_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    async fn load_operation_status(&self, ctx: &RepositoryContext) -> Result<OperationStatus> {
        let row = sqlx::query(
            "SELECT last_run_at, last_report_json, is_running FROM operation_statuses WHERE portfolio_id = $1"
        )
        .bind(&ctx.portfolio_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(r) = row {
            use sqlx::Row;
            let report_json: Option<String> = r.get("last_report_json");
            let last_report = if let Some(j) = report_json {
                serde_json::from_str(&j).ok()
            } else {
                None
            };

            let policy = self.load_operation_policy(ctx).await?;

            Ok(OperationStatus {
                last_run_at: r.get("last_run_at"),
                last_report,
                policy,
                is_running: r.get("is_running"),
            })
        } else {
            let policy = self.load_operation_policy(ctx).await?;
            Ok(OperationStatus {
                policy,
                ..Default::default()
            })
        }
    }
    async fn save_operation_status(
        &self,
        ctx: &RepositoryContext,
        status: &OperationStatus,
    ) -> Result<()> {
        let report_json = status
            .last_report
            .as_ref()
            .and_then(|r| serde_json::to_string(r).ok());

        sqlx::query(
            r#"
            INSERT INTO operation_statuses (portfolio_id, last_run_at, last_report_json, is_running)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (portfolio_id) DO UPDATE SET
                last_run_at = EXCLUDED.last_run_at,
                last_report_json = EXCLUDED.last_report_json,
                is_running = EXCLUDED.is_running,
                updated_at = NOW()
            "#,
        )
        .bind(&ctx.portfolio_id)
        .bind(&status.last_run_at)
        .bind(report_json)
        .bind(status.is_running)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_daily_operation_report(
        &self,
        ctx: &RepositoryContext,
    ) -> Result<Option<DailyOperationReport>> {
        let row =
            sqlx::query("SELECT report_json FROM daily_operation_reports WHERE portfolio_id = $1")
                .bind(&ctx.portfolio_id)
                .fetch_optional(&self.pool)
                .await?;

        if let Some(r) = row {
            use sqlx::Row;
            let report_json: String = r.get("report_json");
            Ok(serde_json::from_str(&report_json).ok())
        } else {
            Ok(None)
        }
    }

    async fn save_daily_operation_report(
        &self,
        ctx: &RepositoryContext,
        report: &DailyOperationReport,
    ) -> Result<()> {
        let report_json = serde_json::to_string(report)?;

        sqlx::query(
            r#"
            INSERT INTO daily_operation_reports (portfolio_id, report_json)
            VALUES ($1, $2)
            ON CONFLICT (portfolio_id) DO UPDATE SET
                report_json = EXCLUDED.report_json,
                updated_at = NOW()
            "#,
        )
        .bind(&ctx.portfolio_id)
        .bind(report_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn start_job(
        &self,
        ctx: &RepositoryContext,
        job_type: &str,
    ) -> Result<crate::models::WebJob> {
        // Check for existing running/queued for same portfolio+type
        let existing = sqlx::query(
            r#"
            SELECT id, portfolio_id, job_type, status, started_at, finished_at,
                   progress_current, progress_total, message, result_json, error_message,
                   created_at, updated_at
            FROM web_jobs
            WHERE portfolio_id = $1 AND job_type = $2 AND status IN ('queued', 'running')
            ORDER BY created_at DESC LIMIT 1
            "#,
        )
        .bind(&ctx.portfolio_id)
        .bind(job_type)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = existing {
            use sqlx::Row;
            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "queued" => crate::models::WebJobStatus::Queued,
                "running" => crate::models::WebJobStatus::Running,
                "success" => crate::models::WebJobStatus::Success,
                "partial_success" => crate::models::WebJobStatus::PartialSuccess,
                "warning" => crate::models::WebJobStatus::Warning,
                "failed" => crate::models::WebJobStatus::Failed,
                "interrupted" => crate::models::WebJobStatus::Interrupted,
                _ => crate::models::WebJobStatus::Queued,
            };
            let job = crate::models::WebJob {
                job_id: row.get("id"),
                portfolio_id: row.get("portfolio_id"),
                job_type: row.get("job_type"),
                status,
                started_at: row
                    .get::<Option<chrono::DateTime<chrono::Utc>>, _>("started_at")
                    .map(|d| d.to_rfc3339()),
                finished_at: row
                    .get::<Option<chrono::DateTime<chrono::Utc>>, _>("finished_at")
                    .map(|d| d.to_rfc3339()),
                progress_current: row.get("progress_current"),
                progress_total: row.get("progress_total"),
                message: row.get("message"),
                result_json: row.get("result_json"),
                error_message: row.get("error_message"),
                created_at: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .to_rfc3339(),
                updated_at: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                    .to_rfc3339(),
            };
            return Ok(job);
        }

        let job_id = format!("{}_{}", job_type, chrono::Local::now().timestamp_millis());
        let now = chrono::Utc::now();
        sqlx::query(
            r#"
            INSERT INTO web_jobs (id, portfolio_id, job_type, status, progress_current, progress_total, message, created_at, updated_at)
            VALUES ($1, $2, $3, 'queued', 0, 0, '已加入队列', $4, $4)
            "#,
        )
        .bind(&job_id)
        .bind(&ctx.portfolio_id)
        .bind(job_type)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(crate::models::WebJob {
            job_id,
            portfolio_id: ctx.portfolio_id.clone(),
            job_type: job_type.to_string(),
            status: crate::models::WebJobStatus::Queued,
            started_at: None,
            finished_at: None,
            progress_current: 0,
            progress_total: 0,
            message: Some("已加入队列".to_string()),
            result_json: None,
            error_message: None,
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
        })
    }

    async fn get_latest_job(
        &self,
        ctx: &RepositoryContext,
        job_type: &str,
    ) -> Result<Option<crate::models::WebJob>> {
        let row = sqlx::query(
            r#"
            SELECT id, portfolio_id, job_type, status, started_at, finished_at,
                   progress_current, progress_total, message, result_json, error_message,
                   created_at, updated_at
            FROM web_jobs
            WHERE portfolio_id = $1 AND job_type = $2
            ORDER BY created_at DESC LIMIT 1
            "#,
        )
        .bind(&ctx.portfolio_id)
        .bind(job_type)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "queued" => crate::models::WebJobStatus::Queued,
                "running" => crate::models::WebJobStatus::Running,
                "success" => crate::models::WebJobStatus::Success,
                "partial_success" => crate::models::WebJobStatus::PartialSuccess,
                "warning" => crate::models::WebJobStatus::Warning,
                "failed" => crate::models::WebJobStatus::Failed,
                "interrupted" => crate::models::WebJobStatus::Interrupted,
                _ => crate::models::WebJobStatus::Queued,
            };
            let job = crate::models::WebJob {
                job_id: row.get("id"),
                portfolio_id: row.get("portfolio_id"),
                job_type: row.get("job_type"),
                status,
                started_at: row
                    .get::<Option<chrono::DateTime<chrono::Utc>>, _>("started_at")
                    .map(|d| d.to_rfc3339()),
                finished_at: row
                    .get::<Option<chrono::DateTime<chrono::Utc>>, _>("finished_at")
                    .map(|d| d.to_rfc3339()),
                progress_current: row.get("progress_current"),
                progress_total: row.get("progress_total"),
                message: row.get("message"),
                result_json: row.get("result_json"),
                error_message: row.get("error_message"),
                created_at: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .to_rfc3339(),
                updated_at: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                    .to_rfc3339(),
            };
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    async fn get_running_job(
        &self,
        ctx: &RepositoryContext,
        job_type: &str,
    ) -> Result<Option<crate::models::WebJob>> {
        let row = sqlx::query(
            r#"
            SELECT id, portfolio_id, job_type, status, started_at, finished_at,
                   progress_current, progress_total, message, result_json, error_message,
                   created_at, updated_at
            FROM web_jobs
            WHERE portfolio_id = $1 AND job_type = $2 AND status IN ('queued', 'running')
            ORDER BY created_at DESC LIMIT 1
            "#,
        )
        .bind(&ctx.portfolio_id)
        .bind(job_type)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let status_str: String = row.get("status");
            let status = match status_str.as_str() {
                "queued" => crate::models::WebJobStatus::Queued,
                "running" => crate::models::WebJobStatus::Running,
                "success" => crate::models::WebJobStatus::Success,
                "partial_success" => crate::models::WebJobStatus::PartialSuccess,
                "warning" => crate::models::WebJobStatus::Warning,
                "failed" => crate::models::WebJobStatus::Failed,
                "interrupted" => crate::models::WebJobStatus::Interrupted,
                _ => crate::models::WebJobStatus::Queued,
            };
            let job = crate::models::WebJob {
                job_id: row.get("id"),
                portfolio_id: row.get("portfolio_id"),
                job_type: row.get("job_type"),
                status,
                started_at: row
                    .get::<Option<chrono::DateTime<chrono::Utc>>, _>("started_at")
                    .map(|d| d.to_rfc3339()),
                finished_at: row
                    .get::<Option<chrono::DateTime<chrono::Utc>>, _>("finished_at")
                    .map(|d| d.to_rfc3339()),
                progress_current: row.get("progress_current"),
                progress_total: row.get("progress_total"),
                message: row.get("message"),
                result_json: row.get("result_json"),
                error_message: row.get("error_message"),
                created_at: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .to_rfc3339(),
                updated_at: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                    .to_rfc3339(),
            };
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    async fn update_job_progress(
        &self,
        ctx: &RepositoryContext,
        job_id: &str,
        progress_current: i32,
        progress_total: i32,
        message: Option<String>,
    ) -> Result<()> {
        let now = chrono::Utc::now();
        sqlx::query(
            r#"
            UPDATE web_jobs
            SET progress_current = $2, progress_total = $3, message = COALESCE($4, message),
                status = CASE WHEN status = 'queued' THEN 'running' ELSE status END,
                started_at = COALESCE(started_at, $5),
                updated_at = $5
            WHERE id = $1 AND portfolio_id = $6
            "#,
        )
        .bind(job_id)
        .bind(progress_current)
        .bind(progress_total)
        .bind(message)
        .bind(now)
        .bind(&ctx.portfolio_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn finish_job(
        &self,
        ctx: &RepositoryContext,
        job_id: &str,
        status: crate::models::WebJobStatus,
        message: Option<String>,
        result_json: Option<serde_json::Value>,
    ) -> Result<()> {
        let status_str = match status {
            crate::models::WebJobStatus::Queued => "queued",
            crate::models::WebJobStatus::Running => "running",
            crate::models::WebJobStatus::Success => "success",
            crate::models::WebJobStatus::PartialSuccess => "partial_success",
            crate::models::WebJobStatus::Warning => "warning",
            crate::models::WebJobStatus::Failed => "failed",
            crate::models::WebJobStatus::Interrupted => "interrupted",
        };
        let now = chrono::Utc::now();
        sqlx::query(
            r#"
            UPDATE web_jobs
            SET status = $2, message = COALESCE($3, message), result_json = $4,
                finished_at = $5, updated_at = $5
            WHERE id = $1 AND portfolio_id = $6
            "#,
        )
        .bind(job_id)
        .bind(status_str)
        .bind(message)
        .bind(result_json)
        .bind(now)
        .bind(&ctx.portfolio_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn fail_job(
        &self,
        ctx: &RepositoryContext,
        job_id: &str,
        error_message: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now();
        sqlx::query(
            r#"
            UPDATE web_jobs
            SET status = 'failed', error_message = $2, finished_at = $3, updated_at = $3
            WHERE id = $1 AND portfolio_id = $4
            "#,
        )
        .bind(job_id)
        .bind(error_message)
        .bind(now)
        .bind(&ctx.portfolio_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn mark_stale_running_jobs_interrupted(&self, ctx: &RepositoryContext) -> Result<usize> {
        let now = chrono::Utc::now();
        let res = sqlx::query(
            r#"
            UPDATE web_jobs
            SET status = 'interrupted', error_message = '服务器重启导致任务中断', finished_at = $1, updated_at = $1
            WHERE portfolio_id = $2 AND status IN ('queued', 'running')
            "#,
        )
        .bind(now)
        .bind(&ctx.portfolio_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() as usize)
    }
}

#[async_trait]
impl CacheRepository for PostgresRepository {
    async fn load_cache_status(&self, _ctx: &RepositoryContext) -> Result<CacheStatusRegistry> {
        let row =
            sqlx::query("SELECT data_json FROM global_caches WHERE cache_key = 'cache_status'")
                .fetch_optional(&self.pool)
                .await?;

        if let Some(r) = row {
            use sqlx::Row;
            let data_json: String = r.get("data_json");
            Ok(serde_json::from_str(&data_json).unwrap_or_default())
        } else {
            Ok(CacheStatusRegistry::default())
        }
    }

    async fn save_cache_status(
        &self,
        _ctx: &RepositoryContext,
        registry: &CacheStatusRegistry,
    ) -> Result<()> {
        let data_json = serde_json::to_string(registry)?;
        sqlx::query(
            "INSERT INTO global_caches (cache_key, data_json) VALUES ('cache_status', $1) ON CONFLICT (cache_key) DO UPDATE SET data_json = EXCLUDED.data_json, updated_at = NOW()"
        )
        .bind(data_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_risk_cache(&self, _ctx: &RepositoryContext) -> Result<Option<RiskCache>> {
        let row = sqlx::query("SELECT data_json FROM global_caches WHERE cache_key = 'risk_cache'")
            .fetch_optional(&self.pool)
            .await?;

        if let Some(r) = row {
            use sqlx::Row;
            let data_json: String = r.get("data_json");
            Ok(serde_json::from_str(&data_json).ok())
        } else {
            Ok(None)
        }
    }

    async fn save_risk_cache(&self, _ctx: &RepositoryContext, cache: &RiskCache) -> Result<()> {
        let data_json = serde_json::to_string(cache)?;
        sqlx::query(
            "INSERT INTO global_caches (cache_key, data_json) VALUES ('risk_cache', $1) ON CONFLICT (cache_key) DO UPDATE SET data_json = EXCLUDED.data_json, updated_at = NOW()"
        )
        .bind(data_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_proxy_cache(&self, _ctx: &RepositoryContext) -> Result<ProxyValuationCache> {
        let row =
            sqlx::query("SELECT data_json FROM global_caches WHERE cache_key = 'proxy_cache'")
                .fetch_optional(&self.pool)
                .await?;

        if let Some(r) = row {
            use sqlx::Row;
            let data_json: String = r.get("data_json");
            Ok(
                serde_json::from_str(&data_json).unwrap_or(ProxyValuationCache {
                    results: vec![],
                    fetched_at: "never".to_string(),
                }),
            )
        } else {
            Ok(ProxyValuationCache {
                results: vec![],
                fetched_at: "never".to_string(),
            })
        }
    }

    async fn save_proxy_cache(
        &self,
        _ctx: &RepositoryContext,
        cache: &ProxyValuationCache,
    ) -> Result<()> {
        let data_json = serde_json::to_string(cache)?;
        sqlx::query(
            "INSERT INTO global_caches (cache_key, data_json) VALUES ('proxy_cache', $1) ON CONFLICT (cache_key) DO UPDATE SET data_json = EXCLUDED.data_json, updated_at = NOW()"
        )
        .bind(data_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_regime_cache(&self, _ctx: &RepositoryContext) -> Result<RegimeCache> {
        let row =
            sqlx::query("SELECT data_json FROM global_caches WHERE cache_key = 'regime_cache'")
                .fetch_optional(&self.pool)
                .await?;

        if let Some(r) = row {
            use sqlx::Row;
            let data_json: String = r.get("data_json");
            Ok(serde_json::from_str(&data_json).unwrap_or_default())
        } else {
            Ok(RegimeCache::default())
        }
    }

    async fn save_regime_cache(&self, _ctx: &RepositoryContext, cache: &RegimeCache) -> Result<()> {
        let data_json = serde_json::to_string(cache)?;
        sqlx::query(
            "INSERT INTO global_caches (cache_key, data_json) VALUES ('regime_cache', $1) ON CONFLICT (cache_key) DO UPDATE SET data_json = EXCLUDED.data_json, updated_at = NOW()"
        )
        .bind(data_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_market_cache(&self, _ctx: &RepositoryContext) -> Result<MarketCache> {
        let row =
            sqlx::query("SELECT data_json FROM global_caches WHERE cache_key = 'market_cache'")
                .fetch_optional(&self.pool)
                .await?;

        if let Some(r) = row {
            use sqlx::Row;
            let data_json: String = r.get("data_json");
            Ok(serde_json::from_str(&data_json).unwrap_or_default())
        } else {
            Ok(MarketCache::default())
        }
    }

    async fn save_market_cache(&self, _ctx: &RepositoryContext, cache: &MarketCache) -> Result<()> {
        let data_json = serde_json::to_string(cache)?;
        sqlx::query(
            "INSERT INTO global_caches (cache_key, data_json) VALUES ('market_cache', $1) ON CONFLICT (cache_key) DO UPDATE SET data_json = EXCLUDED.data_json, updated_at = NOW()"
        )
        .bind(data_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_fx_cache(&self, _ctx: &RepositoryContext) -> Result<FxCache> {
        let row = sqlx::query("SELECT data_json FROM global_caches WHERE cache_key = 'fx_cache'")
            .fetch_optional(&self.pool)
            .await?;

        if let Some(r) = row {
            use sqlx::Row;
            let data_json: String = r.get("data_json");
            Ok(serde_json::from_str(&data_json).unwrap_or_default())
        } else {
            Ok(FxCache::default())
        }
    }

    async fn save_fx_cache(&self, _ctx: &RepositoryContext, cache: &FxCache) -> Result<()> {
        let data_json = serde_json::to_string(cache)?;
        sqlx::query(
            "INSERT INTO global_caches (cache_key, data_json) VALUES ('fx_cache', $1) ON CONFLICT (cache_key) DO UPDATE SET data_json = EXCLUDED.data_json, updated_at = NOW()"
        )
        .bind(data_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_nav_cache(&self, _ctx: &RepositoryContext) -> Result<NavCache> {
        let row = sqlx::query("SELECT data_json FROM global_caches WHERE cache_key = 'nav_cache'")
            .fetch_optional(&self.pool)
            .await?;

        if let Some(r) = row {
            use sqlx::Row;
            let data_json: String = r.get("data_json");
            Ok(serde_json::from_str(&data_json).unwrap_or_default())
        } else {
            Ok(NavCache::default())
        }
    }

    async fn save_nav_cache(&self, _ctx: &RepositoryContext, cache: &NavCache) -> Result<()> {
        let data_json = serde_json::to_string(cache)?;
        sqlx::query(
            "INSERT INTO global_caches (cache_key, data_json) VALUES ('nav_cache', $1) ON CONFLICT (cache_key) DO UPDATE SET data_json = EXCLUDED.data_json, updated_at = NOW()"
        )
        .bind(data_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
