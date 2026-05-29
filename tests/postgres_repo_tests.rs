use pendulum_kelly_cli::models::{AssetHolding, PortfolioState, Transaction};
use pendulum_kelly_cli::repository::{PostgresRepository, RepositoryContext, traits::*};
use std::env;

#[tokio::test]
#[ignore]
async fn test_postgres_state_lifecycle() {
    let db_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set for postgres integration tests");
    let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
    let db = pendulum_kelly_cli::db::postgres::PostgresDb { pool: pool.clone() };
    db.run_migrations().await.expect("Failed to run migrations");

    let repo = PostgresRepository::new(pool, "dummy_config.toml".to_string());
    let ctx = RepositoryContext {
        portfolio_id: format!("test_p_state_{}", chrono::Utc::now().timestamp_millis()),
        ..Default::default()
    };

    // 1. Initial empty load
    let state = repo.load_state(&ctx).await.unwrap();
    assert_eq!(state.cash, 0.0);
    assert!(state.asset_holdings.is_empty());

    // 2. Save state
    let new_state = PortfolioState {
        cash: 15000.50,
        asset_holdings: vec![
            AssetHolding {
                asset_id: "nasdaq".to_string(),
                fund_code: "006327".to_string(),
                units: 100.0,
                units_estimated: false,
                cost_basis: 500.0,
                last_market_value: 550.0,
                latest_nav: Some(5.5),
                latest_nav_date: Some("2026-05-20".to_string()),
                latest_nav_source: Some("mock".to_string()),
                latest_nav_status: Some("正常".to_string()),
            },
            AssetHolding {
                asset_id: "sp500".to_string(),
                fund_code: "000001".to_string(),
                units: 50.0,
                units_estimated: true,
                cost_basis: 100.0,
                last_market_value: 120.0,
                latest_nav: None,
                latest_nav_date: None,
                latest_nav_source: None,
                latest_nav_status: None,
            },
        ],
    };

    repo.save_state(&ctx, &new_state).await.unwrap();

    // 3. Load state and verify
    let loaded = repo.load_state(&ctx).await.unwrap();
    assert_eq!(loaded.cash, 15000.50);
    assert_eq!(loaded.asset_holdings.len(), 2);

    let nasdaq = loaded
        .asset_holdings
        .iter()
        .find(|h| h.asset_id == "nasdaq")
        .unwrap();
    assert_eq!(nasdaq.units, 100.0);
    assert_eq!(nasdaq.latest_nav, Some(5.5));
    assert_eq!(nasdaq.latest_nav_date, Some("2026-05-20".to_string()));

    let sp500 = loaded
        .asset_holdings
        .iter()
        .find(|h| h.asset_id == "sp500")
        .unwrap();
    assert!(sp500.units_estimated);
    assert_eq!(sp500.latest_nav, None);
    assert_eq!(sp500.latest_nav_date, None);

    // 4. Update state (modify cash, remove sp500, update nasdaq)
    let updated_state = PortfolioState {
        cash: 10000.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "nasdaq".to_string(),
            fund_code: "006327".to_string(),
            units: 200.0,
            units_estimated: false,
            cost_basis: 1000.0,
            last_market_value: 1100.0,
            latest_nav: Some(5.5),
            latest_nav_date: Some("2026-05-20".to_string()),
            latest_nav_source: Some("mock".to_string()),
            latest_nav_status: Some("正常".to_string()),
        }],
    };

    repo.save_state(&ctx, &updated_state).await.unwrap();

    let final_load = repo.load_state(&ctx).await.unwrap();
    assert_eq!(final_load.cash, 10000.0);
    assert_eq!(final_load.asset_holdings.len(), 1);
    assert_eq!(final_load.asset_holdings[0].asset_id, "nasdaq");
    assert_eq!(final_load.asset_holdings[0].units, 200.0);
}

#[tokio::test]
#[ignore]
async fn test_postgres_dca_lifecycle() {
    use pendulum_kelly_cli::models::{
        DcaFrequency, DcaPlan, DcaSettlement, DcaSettlementAudit, DcaSettlementStatus,
    };

    let db_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set for postgres integration tests");
    let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
    let db = pendulum_kelly_cli::db::postgres::PostgresDb { pool: pool.clone() };
    db.run_migrations().await.expect("Failed to run migrations");

    let repo = PostgresRepository::new(pool, "dummy_config.toml".to_string());
    let ctx = RepositoryContext {
        portfolio_id: format!("test_p_dca_{}", chrono::Utc::now().timestamp_millis()),
        ..Default::default()
    };

    // Initialize portfolio in DB to satisfy foreign keys
    repo.save_state(&ctx, &PortfolioState::default())
        .await
        .unwrap();

    // Ensure empty
    let plans = repo.load_plans(&ctx).await.unwrap();
    assert!(plans.is_empty());

    // Insert Plan
    let plan = DcaPlan {
        plan_id: "plan_1".to_string(),
        asset_id: "a1".to_string(),
        fund_code: "123".to_string(),
        fund_name: "Test Fund".to_string(),
        amount: 500.0,
        currency: "CNY".to_string(),
        frequency: DcaFrequency::Weekly,
        weekday: Some(1),
        month_day: None,
        start_date: "2026-05-20".to_string(),
        end_date: None,
        enabled: true,
        priority: 10,
        note: None,
    };
    repo.save_plans(&ctx, std::slice::from_ref(&plan))
        .await
        .unwrap();

    let loaded_plans = repo.load_plans(&ctx).await.unwrap();
    assert_eq!(loaded_plans.len(), 1);
    assert_eq!(loaded_plans[0].plan_id, "plan_1");

    // Insert Settlement
    let settlement = DcaSettlement {
        settlement_id: "s_1".to_string(),
        plan_id: Some("plan_1".to_string()),
        asset_id: "a1".to_string(),
        fund_code: "123".to_string(),
        fund_name: "Test Fund".to_string(),
        scheduled_date: Some("2026-05-25".to_string()),
        deduction_date: "2026-05-25".to_string(),
        confirmation_date: "2026-05-26".to_string(),
        amount: 500.0,
        confirmed_nav: 1.25,
        confirmed_units: 400.0,
        fee: Some(0.0),
        currency: "CNY".to_string(),
        source: "alipay".to_string(),
        status: DcaSettlementStatus::Confirmed,
        applied: false,
        note: None,
        created_at: "2026-05-26 12:00:00".to_string(),
    };
    repo.save_settlements(&ctx, std::slice::from_ref(&settlement))
        .await
        .unwrap();

    let loaded_settlements = repo.load_settlements(&ctx).await.unwrap();
    assert_eq!(loaded_settlements.len(), 1);
    assert_eq!(loaded_settlements[0].settlement_id, "s_1");

    // Insert Audit
    let audit = DcaSettlementAudit {
        audit_id: "audit_1".to_string(),
        timestamp: "2026-05-26 12:05:00".to_string(),
        settlement_id: "s_1".to_string(),
        asset_id: "a1".to_string(),
        old_units: 0.0,
        new_units: 400.0,
        old_cost_basis: 0.0,
        new_cost_basis: 1.25,
        transaction_id: None,
        note: None,
    };
    repo.save_settlement_audits(&ctx, std::slice::from_ref(&audit))
        .await
        .unwrap();

    let loaded_audits = repo.load_settlement_audits(&ctx).await.unwrap();
    assert_eq!(loaded_audits.len(), 1);
    assert_eq!(loaded_audits[0].audit_id, "audit_1");
}

#[tokio::test]
#[ignore]
async fn test_postgres_multi_user_foundation() {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = sqlx::PgPool::connect(&db_url).await.unwrap();

    let db = pendulum_kelly_cli::db::postgres::PostgresDb { pool: pool.clone() };
    db.run_migrations().await.expect("Failed to run migrations");

    // Verify default user exists
    let user_count: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE id = 'local_user'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(user_count, 1);

    // Verify default portfolio exists
    let pf_count: i64 = sqlx::query_scalar("SELECT count(*) FROM portfolios WHERE id = 'default'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(pf_count, 1);

    // Verify default role exists
    let role_count: i64 = sqlx::query_scalar("SELECT count(*) FROM user_portfolio_roles WHERE user_id = 'local_user' AND portfolio_id = 'default' AND role = 'owner'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(role_count, 1);
}

#[tokio::test]
#[ignore]
async fn test_postgres_instrument_lifecycle() {
    use pendulum_kelly_cli::models::{
        AssetClass, InstrumentConfig, InstrumentQuoteCache, InstrumentQuoteCacheEntry,
    };

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
    let db = pendulum_kelly_cli::db::postgres::PostgresDb { pool: pool.clone() };
    db.run_migrations().await.expect("Failed to run migrations");

    let repo = PostgresRepository::new(pool, "dummy_config.toml".to_string());
    let ctx = RepositoryContext::default();

    let instruments = repo.load_instruments(&ctx).await.unwrap();
    let initial_count = instruments.len();

    let inst_id = format!("inst_{}", chrono::Utc::now().timestamp_millis());
    let inst = InstrumentConfig {
        instrument_id: inst_id.clone(),
        symbol: "TEST".to_string(),
        display_symbol: Some("TEST".to_string()),
        name: "Test Inst".to_string(),
        name_zh: Some("测试".to_string()),
        name_en: None,
        description_zh: None,
        category_zh: Some("测试类".to_string()),
        display_label: None,
        asset_class: AssetClass::Etf,
        provider: "yahoo".to_string(),
        provider_symbol: "TEST".to_string(),
        market: Some("US".to_string()),
        exchange: Some("NASDAQ".to_string()),
        currency: "USD".to_string(),
        quote_unit: "share".to_string(),
        price_unit: "USD/share".to_string(),
        timezone: Some("UTC".to_string()),
        enabled: true,
        priority: 10,
        tags: vec!["tag1".to_string(), "tag2".to_string()],
        note: None,
    };

    repo.save_instruments(&ctx, std::slice::from_ref(&inst))
        .await
        .unwrap();

    let loaded_insts = repo.load_instruments(&ctx).await.unwrap();
    assert_eq!(loaded_insts.len(), initial_count + 1);

    let loaded_inst = loaded_insts
        .iter()
        .find(|i| i.instrument_id == inst_id)
        .unwrap();
    assert_eq!(loaded_inst.name_zh, Some("测试".to_string()));
    assert_eq!(loaded_inst.tags.len(), 2);

    let cache = repo.load_instrument_cache(&ctx).await.unwrap();
    let initial_cache_count = cache.entries.len();

    let cache_entry = InstrumentQuoteCacheEntry {
        instrument_id: inst_id.clone(),
        symbol: "TEST".to_string(),
        name_zh: Some("测试".to_string()),
        price: 100.0,
        date: "2026-05-26".to_string(),
        currency: "USD".to_string(),
        quote_unit: "share".to_string(),
        provider: "yahoo".to_string(),
        source: "yahoo".to_string(),
        status: "正常".to_string(),
        fetched_at: "2026-05-26T12:00:00Z".to_string(),
        warning: None,
    };

    let cache_to_save = InstrumentQuoteCache {
        entries: vec![cache_entry],
        fetched_at: "2026-05-26T12:00:00Z".to_string(),
    };

    repo.save_instrument_cache(&ctx, &cache_to_save)
        .await
        .unwrap();

    let loaded_cache = repo.load_instrument_cache(&ctx).await.unwrap();
    assert_eq!(loaded_cache.entries.len(), initial_cache_count + 1);

    let loaded_cache_entry = loaded_cache
        .entries
        .iter()
        .find(|e| e.instrument_id == inst_id)
        .unwrap();
    assert_eq!(loaded_cache_entry.price, 100.0);
}

#[tokio::test]
#[ignore]
async fn test_postgres_reconciliation_lifecycle() {
    use pendulum_kelly_cli::models::{AlipaySnapshot, ReconciliationAudit};

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
    let db = pendulum_kelly_cli::db::postgres::PostgresDb { pool: pool.clone() };
    db.run_migrations().await.expect("Failed to run migrations");

    let repo = PostgresRepository::new(pool, "dummy_config.toml".to_string());
    let ctx = RepositoryContext {
        portfolio_id: format!("test_p_recon_{}", chrono::Utc::now().timestamp_millis()),
        ..Default::default()
    };

    // Initialize portfolio in DB to satisfy foreign keys
    repo.save_state(&ctx, &PortfolioState::default())
        .await
        .unwrap();

    let snaps = repo.load_alipay_snapshots(&ctx).await.unwrap();
    assert!(snaps.is_empty());

    let snapshot = AlipaySnapshot {
        snapshot_id: "snap_1".to_string(),
        asset_id: "a1".to_string(),
        fund_code: "123".to_string(),
        fund_name: "Test Fund".to_string(),
        snapshot_date: "2026-05-25".to_string(),
        market_value: 1000.0,
        units: Some(100.0),
        cost_basis: Some(900.0),
        nav: Some(1.0),
        nav_date: Some("2026-05-25".to_string()),
        daily_pnl: Some(10.0),
        total_pnl: Some(100.0),
        source: "alipay".to_string(),
        note: None,
        created_at: "2026-05-25 12:00:00".to_string(),
    };
    repo.save_alipay_snapshots(&ctx, std::slice::from_ref(&snapshot))
        .await
        .unwrap();

    let loaded_snaps = repo.load_alipay_snapshots(&ctx).await.unwrap();
    assert_eq!(loaded_snaps.len(), 1);
    assert_eq!(loaded_snaps[0].snapshot_id, "snap_1");
    assert_eq!(loaded_snaps[0].market_value, 1000.0);

    let audits = repo.load_reconciliation_audits(&ctx).await.unwrap();
    assert!(audits.is_empty());

    let audit = ReconciliationAudit {
        audit_id: "audit_1".to_string(),
        timestamp: "2026-05-26 12:05:00".to_string(),
        snapshot_id: "snap_1".to_string(),
        asset_id: "a1".to_string(),
        old_units: 0.0,
        new_units: 100.0,
        old_cost_basis: 0.0,
        new_cost_basis: 900.0,
        old_market_value: 0.0,
        new_market_value: 1000.0,
        reason: "initial".to_string(),
        note: None,
    };
    repo.save_reconciliation_audits(&ctx, std::slice::from_ref(&audit))
        .await
        .unwrap();

    let loaded_audits = repo.load_reconciliation_audits(&ctx).await.unwrap();
    assert_eq!(loaded_audits.len(), 1);
    assert_eq!(loaded_audits[0].audit_id, "audit_1");
}

#[tokio::test]
#[ignore]
async fn test_postgres_transactions_lifecycle() {
    // This test requires a running PostgreSQL instance and DATABASE_URL env var.
    let db_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set for postgres integration tests");
    let pool = sqlx::PgPool::connect(&db_url).await.unwrap();

    // Run migrations
    let db = pendulum_kelly_cli::db::postgres::PostgresDb { pool: pool.clone() };
    db.run_migrations().await.expect("Failed to run migrations");

    let repo = PostgresRepository::new(pool, "dummy_config.toml".to_string());
    let ctx = RepositoryContext {
        portfolio_id: format!("test_p_{}", chrono::Utc::now().timestamp_millis()),
        ..Default::default()
    };

    // Initialize portfolio in DB to satisfy foreign keys
    repo.save_state(&ctx, &PortfolioState::default())
        .await
        .unwrap();

    // 1. Ensure empty
    let txs = repo.load_transactions(&ctx).await.unwrap();
    // Since we used a unique portfolio_id, it should be empty even if DB is shared
    assert!(txs.is_empty());

    // 2. Save new transaction
    let t1 = Transaction {
        id: format!("tx_{}", ctx.portfolio_id),
        date: "2026-05-22".to_string(),
        transaction_type: "buy".to_string(),
        asset_id: Some("asset1".to_string()),
        amount: 1000.0,
        units: Some(10.0),
        price: Some(100.0),
        fee: 5.0,
        currency: "CNY".to_string(),
        note: "Initial buy".to_string(),
        source: "manual".to_string(),
        raw_description: "".to_string(),
    };

    repo.save_transactions(&ctx, std::slice::from_ref(&t1))
        .await
        .unwrap();

    // 3. Load and verify
    let loaded = repo.load_transactions(&ctx).await.unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, t1.id);
    assert_eq!(loaded[0].note, "Initial buy");
    assert_eq!(loaded[0].amount, 1000.0);

    // 4. Update (Upsert)
    let mut t1_updated = t1.clone();
    t1_updated.note = "Updated note".to_string();
    t1_updated.amount = 1100.0;

    repo.save_transactions(&ctx, &[t1_updated]).await.unwrap();

    let loaded_after_update = repo.load_transactions(&ctx).await.unwrap();
    assert_eq!(loaded_after_update.len(), 1);
    assert_eq!(loaded_after_update[0].note, "Updated note");
    assert_eq!(loaded_after_update[0].amount, 1100.0);

    // 5. Multiple transactions
    let t2 = Transaction {
        id: format!("tx2_{}", ctx.portfolio_id),
        date: "2026-05-23".to_string(),
        transaction_type: "cash_in".to_string(),
        asset_id: None,
        amount: 5000.0,
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: "Cash in".to_string(),
        source: "manual".to_string(),
        raw_description: "".to_string(),
    };

    // save_transactions usually takes the whole current state in the vector-based traits
    let current_txs = vec![t1.clone(), t2.clone()];
    repo.save_transactions(&ctx, &current_txs).await.unwrap();

    let loaded_all = repo.load_transactions(&ctx).await.unwrap();
    assert_eq!(loaded_all.len(), 2);
    // Ordered by date DESC in our implementation
    assert_eq!(loaded_all[0].id, t2.id);
    assert_eq!(loaded_all[1].id, t1.id);
}
