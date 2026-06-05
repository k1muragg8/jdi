//! Inline CRUD on 市场 / 持仓 (no standalone admin/import pages).

use pendulum_kelly_cli::models::instrument::AssetClass;
use pendulum_kelly_cli::models::{AssetConfig, AssetHolding, InstrumentConfig};
use pendulum_kelly_cli::repository::RepositoryContext;
use pendulum_kelly_cli::repository::json::JsonRepository;
use pendulum_kelly_cli::web::test_pages;
use pendulum_kelly_cli::web::{AppState, BackgroundRefreshStatus};
use std::sync::Arc;
use tempfile::TempDir;

fn make_state(dir: &str) -> Arc<AppState> {
    std::fs::create_dir_all(dir).ok();
    for (src, name) in [
        ("examples/config.toml", "config.toml"),
        ("examples/portfolio_state.json", "portfolio_state.json"),
        ("examples/transactions.json", "transactions.json"),
    ] {
        let dest = format!("{}/{}", dir, name);
        if !std::path::Path::new(&dest).exists() {
            let _ = std::fs::copy(src, &dest);
        }
    }
    let repo = Arc::new(JsonRepository::new_with_defaults(dir));
    Arc::new(AppState {
        repo,
        ctx: RepositoryContext::default(),
        refresh_status: Arc::new(tokio::sync::RwLock::new(BackgroundRefreshStatus {
            last_market_refresh: None,
            last_fund_refresh: None,
            is_running: false,
            last_error: None,
            latest_daily_report: None,
        })),
        last_backtest_report: Arc::new(tokio::sync::RwLock::new(None)),
        running_jobs: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
    })
}

#[tokio::test]
async fn test_market_page_has_watchlist_controls() {
    let tmp = TempDir::new().unwrap();
    let html = test_pages::render_market(make_state(tmp.path().to_str().unwrap())).await;
    assert!(html.contains("/admin/instruments/add"));
    assert!(html.contains("归档"));
    assert!(html.contains("刷新全部行情"));
    assert!(html.contains("instEditModal"));
    assert!(!html.contains("PostgresRepository not implemented"));
}

#[tokio::test]
async fn test_holdings_page_has_asset_edit_controls() {
    let tmp = TempDir::new().unwrap();
    let html = test_pages::render_holdings(make_state(tmp.path().to_str().unwrap())).await;
    assert!(html.contains("/admin/assets/set-sector"));
    assert!(html.contains("资产配置"));
    assert!(html.contains("+定投"));
}

#[tokio::test]
async fn test_no_normal_ui_button_links_directly_to_json_api() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    for html in [
        test_pages::render_holdings(state.clone()).await,
        test_pages::render_market(state.clone()).await,
        test_pages::render_overview(state).await,
    ] {
        assert!(!html.contains("href=\"/api/"));
    }
}

#[tokio::test]
async fn test_market_instrument_create_persists_json_backend() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    let ctx = &state.ctx;
    let mut instruments = state.repo.load_instruments(ctx).await.unwrap_or_default();
    instruments.push(InstrumentConfig {
        instrument_id: "inst_testqqq".to_string(),
        symbol: "TESTQQQ".to_string(),
        display_symbol: None,
        name: "Test".to_string(),
        name_zh: Some("测试".to_string()),
        name_en: None,
        description_zh: None,
        category_zh: None,
        display_label: None,
        asset_class: AssetClass::Etf,
        provider: "yahoo".to_string(),
        provider_symbol: "TESTQQQ".to_string(),
        market: None,
        exchange: None,
        currency: "USD".to_string(),
        quote_unit: "USD".to_string(),
        price_unit: "USD".to_string(),
        timezone: None,
        enabled: true,
        archived: false,
        priority: 0,
        tags: vec![],
        note: None,
    });
    state
        .repo
        .save_instruments(ctx, &instruments)
        .await
        .unwrap();

    let state2 = make_state(dir);
    let loaded = state2.repo.load_instruments(&state2.ctx).await.unwrap();
    assert!(loaded.iter().any(|i| i.symbol == "TESTQQQ"));
}

#[tokio::test]
async fn test_asset_sector_update_persists_json_backend() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    let ctx = &state.ctx;
    let mut config = state.repo.load_config(ctx).await.unwrap();
    config.assets.push(AssetConfig {
        asset_id: "TST001".to_string(),
        fund_code: "TST001".to_string(),
        fund_name: "测试".to_string(),
        sector: "未分类".to_string(),
        enabled: true,
        ..Default::default()
    });
    state.repo.save_config(ctx, &config).await.unwrap();

    let mut config2 = state.repo.load_config(ctx).await.unwrap();
    if let Some(a) = config2.assets.iter_mut().find(|a| a.asset_id == "TST001") {
        a.sector = "美国科技".to_string();
    }
    state.repo.save_config(ctx, &config2).await.unwrap();

    let state3 = make_state(dir);
    let final_cfg = state3.repo.load_config(&state3.ctx).await.unwrap();
    assert_eq!(
        final_cfg
            .assets
            .iter()
            .find(|a| a.asset_id == "TST001")
            .unwrap()
            .sector,
        "美国科技"
    );
}

#[tokio::test]
async fn test_asset_archive_does_not_hard_delete_referenced_json() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    let ctx = &state.ctx;
    let mut config = state.repo.load_config(ctx).await.unwrap();
    config.assets.push(AssetConfig {
        asset_id: "REF001".to_string(),
        fund_code: "REF001".to_string(),
        fund_name: "引用资产".to_string(),
        sector: "测试".to_string(),
        enabled: true,
        ..Default::default()
    });
    state.repo.save_config(ctx, &config).await.unwrap();

    let mut state_pf = state.repo.load_state(ctx).await.unwrap();
    state_pf.asset_holdings.push(AssetHolding {
        asset_id: "REF001".to_string(),
        fund_code: "REF001".to_string(),
        units: 10.0,
        units_estimated: false,
        cost_basis: 1000.0,
        latest_nav: None,
        latest_nav_date: None,
        latest_nav_source: None,
        latest_nav_status: None,
        last_market_value: 1000.0,
    });
    state.repo.save_state(ctx, &state_pf).await.unwrap();

    let mut config2 = state.repo.load_config(ctx).await.unwrap();
    if let Some(a) = config2.assets.iter_mut().find(|a| a.asset_id == "REF001") {
        a.enabled = false;
        a.sector = format!("{} (已归档)", a.sector);
    }
    state.repo.save_config(ctx, &config2).await.unwrap();

    let state3 = make_state(dir);
    let cfg = state3.repo.load_config(&state3.ctx).await.unwrap();
    assert!(
        cfg.assets
            .iter()
            .any(|a| a.asset_id == "REF001" && !a.enabled)
    );
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and Postgres"]
async fn test_postgres_backend_loads_for_web_repo() {
    use pendulum_kelly_cli::repository::PostgresRepository;
    use pendulum_kelly_cli::repository::traits::PortfolioRepository;
    use std::env;
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = sqlx::PgPool::connect(&db_url).await.unwrap();
    let repo = Arc::new(PostgresRepository::new(
        pool,
        "data/config.toml".to_string(),
        "DATABASE_URL".to_string(),
    ));
    let _ = repo
        .load_config(&RepositoryContext::default())
        .await
        .unwrap();
}
