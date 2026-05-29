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

    let repo = PostgresRepository::new(pool);
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
async fn test_postgres_transactions_lifecycle() {
    // This test requires a running PostgreSQL instance and DATABASE_URL env var.
    let db_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set for postgres integration tests");
    let pool = sqlx::PgPool::connect(&db_url).await.unwrap();

    // Run migrations
    let db = pendulum_kelly_cli::db::postgres::PostgresDb { pool: pool.clone() };
    db.run_migrations().await.expect("Failed to run migrations");

    let repo = PostgresRepository::new(pool);
    let ctx = RepositoryContext {
        portfolio_id: format!("test_p_{}", chrono::Utc::now().timestamp_millis()),
        ..Default::default()
    };

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
    };

    repo.save_transactions(&ctx, &[t1.clone()]).await.unwrap();

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
