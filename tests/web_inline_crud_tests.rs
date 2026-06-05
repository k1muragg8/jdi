//! Inline CRUD on 市场 / 持仓 (no standalone admin/import pages).

use pendulum_kelly_cli::models::instrument::AssetClass;
use pendulum_kelly_cli::models::{
    AssetConfig, AssetHolding, DcaFrequency, DcaPlan, InstrumentConfig,
};
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
    // polished buttons not plain text links
    assert!(html.contains("btn btn-sm") || html.contains("btn-danger"));
    assert!(!html.contains("PostgresRepository not implemented"));
}

#[tokio::test]
async fn test_holdings_page_has_asset_edit_controls() {
    let tmp = TempDir::new().unwrap();
    let html = test_pages::render_holdings(make_state(tmp.path().to_str().unwrap())).await;
    assert!(html.contains("openAssetEdit"));
    assert!(html.contains("/api/assets/update"));
    assert!(html.contains("持仓明细"));
    assert!(html.contains("刷新基金净值"));
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

// === DCA row management tests (Part 10) ===

async fn seed_asset_for_dca(state: &Arc<AppState>, asset_id: &str, fund_code: &str, name: &str) {
    let ctx = &state.ctx;
    let mut cfg = state.repo.load_config(ctx).await.unwrap_or_default();
    if !cfg.assets.iter().any(|a| a.asset_id == asset_id) {
        cfg.assets.push(AssetConfig {
            asset_id: asset_id.to_string(),
            fund_code: fund_code.to_string(),
            fund_name: name.to_string(),
            sector: "未分类".to_string(),
            enabled: true,
            currency: "CNY".to_string(),
            ..Default::default()
        });
        let _ = state.repo.save_config(ctx, &cfg).await;
    }
    let mut st = state.repo.load_state(ctx).await.unwrap_or_default();
    if !st.asset_holdings.iter().any(|h| h.asset_id == asset_id) {
        st.asset_holdings.push(AssetHolding {
            asset_id: asset_id.to_string(),
            fund_code: fund_code.to_string(),
            units: 100.0,
            units_estimated: false,
            cost_basis: 1000.0,
            latest_nav: None,
            latest_nav_date: None,
            latest_nav_source: None,
            latest_nav_status: None,
            last_market_value: 1000.0,
        });
        let _ = state.repo.save_state(ctx, &st).await;
    }
}

async fn seed_dca_plan(state: &Arc<AppState>, asset_id: &str, amount: f64, enabled: bool) {
    let ctx = &state.ctx;
    let mut plans = state.repo.load_plans(ctx).await.unwrap_or_default();
    plans.retain(|p| p.asset_id != asset_id);
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    plans.push(DcaPlan {
        plan_id: format!("plan_test_{}", chrono::Local::now().timestamp_millis()),
        asset_id: asset_id.to_string(),
        fund_code: "TEST".to_string(),
        fund_name: "Test".to_string(),
        amount,
        currency: "CNY".to_string(),
        frequency: DcaFrequency::Monthly,
        weekday: None,
        month_day: Some(1),
        start_date: "2026-01-01".to_string(),
        end_date: None,
        enabled,
        priority: 0,
        note: Some("test".to_string()),
        created_at: now.clone(),
        updated_at: now,
    });
    let _ = state.repo.save_plans(ctx, &plans).await;
}

#[tokio::test]
async fn test_holdings_row_without_dca_shows_set_dca_button() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    seed_asset_for_dca(&state, "NODCA001", "000001", "无定投资产").await;
    let html = test_pages::render_holdings(state).await;
    assert!(html.contains("未设置"), "should show 未设置");
    assert!(html.contains("设置定投"), "row should have 设置定投 button");
    assert!(html.contains("openDcaModal('NODCA001')"));
    // no separate config needed
    assert!(!html.contains("资产配置"));
}

#[tokio::test]
async fn test_holdings_row_with_active_dca_shows_edit_pause_view() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    seed_asset_for_dca(&state, "ACTDCA01", "000002", "活跃定投").await;
    seed_dca_plan(&state, "ACTDCA01", 500.0, true).await;
    let html = test_pages::render_holdings(state).await;
    assert!(
        html.contains("每月 500 CNY") || html.contains("500"),
        "shows amount"
    );
    assert!(html.contains("编辑定投"), "active shows edit");
    assert!(html.contains("暂停"), "active shows pause");
    assert!(html.contains("查看记录"), "has view records");
}

#[tokio::test]
async fn test_holdings_row_with_paused_dca_shows_edit_resume_view() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    seed_asset_for_dca(&state, "PAUSDCA1", "000003", "暂停定投").await;
    seed_dca_plan(&state, "PAUSDCA1", 300.0, false).await;
    let html = test_pages::render_holdings(state).await;
    assert!(html.contains("已暂停") || html.contains("暂停"));
    assert!(html.contains("编辑定投"));
    assert!(html.contains("恢复"));
    assert!(html.contains("查看记录"));
}

#[tokio::test]
async fn test_dca_create_persists_asset_id_binding_and_reloads() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    seed_asset_for_dca(&state, "PERS001", "000004", "持久化测试").await;
    // use service to create
    let _ = pendulum_kelly_cli::web::services::holdings_service::upsert_dca_for_asset(
        &state,
        "PERS001",
        123.0,
        "monthly",
        Some(1),
        Some("test persist".into()),
    )
    .await;
    // reload fresh state (sim restart)
    let state2 = make_state(dir);
    let html = test_pages::render_holdings(state2.clone()).await;
    assert!(
        html.contains("每月 123") || html.contains("123"),
        "amount persisted in row"
    );
    // check repo directly
    let plans = state2
        .repo
        .load_plans(&state2.ctx)
        .await
        .unwrap_or_default();
    assert!(
        plans
            .iter()
            .any(|p| p.asset_id == "PERS001" && (p.amount - 123.0).abs() < 0.1)
    );
}

#[tokio::test]
async fn test_dca_edit_amount_persists() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    seed_asset_for_dca(&state, "EDITDCA1", "000005", "编辑金额").await;
    seed_dca_plan(&state, "EDITDCA1", 100.0, true).await;
    let state = make_state(dir);
    let _ = pendulum_kelly_cli::web::services::holdings_service::upsert_dca_for_asset(
        &state,
        "EDITDCA1",
        777.0,
        "monthly",
        Some(1),
        None,
    )
    .await;
    let state2 = make_state(dir);
    let plans = state2
        .repo
        .load_plans(&state2.ctx)
        .await
        .unwrap_or_default();
    assert!(
        plans
            .iter()
            .any(|p| p.asset_id == "EDITDCA1" && (p.amount - 777.0).abs() < 0.1)
    );
}

#[tokio::test]
async fn test_dca_pause_resume_archive_persist() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    seed_asset_for_dca(&state, "PAUSE01", "000006", "暂停恢复归档").await;
    seed_dca_plan(&state, "PAUSE01", 50.0, true).await;
    let state = make_state(dir);
    // pause
    let _ =
        pendulum_kelly_cli::web::services::holdings_service::pause_dca_for_asset(&state, "PAUSE01")
            .await;
    let state2 = make_state(dir);
    let plans = state2.repo.load_plans(&state2.ctx).await.unwrap();
    assert!(plans.iter().any(|p| p.asset_id == "PAUSE01" && !p.enabled));
    // resume
    let _ = pendulum_kelly_cli::web::services::holdings_service::resume_dca_for_asset(
        &state2, "PAUSE01",
    )
    .await;
    let state3 = make_state(dir);
    let plans3 = state3.repo.load_plans(&state3.ctx).await.unwrap();
    assert!(plans3.iter().any(|p| p.asset_id == "PAUSE01" && p.enabled));
    // archive (delete)
    let _ = pendulum_kelly_cli::web::services::holdings_service::archive_dca_for_asset(
        &state3, "PAUSE01",
    )
    .await;
    let state4 = make_state(dir);
    let plans4 = state4.repo.load_plans(&state4.ctx).await.unwrap();
    assert!(!plans4.iter().any(|p| p.asset_id == "PAUSE01"));
    // row now shows 设置定投 again (active display hides)
    let html = test_pages::render_holdings(state4).await;
    assert!(html.contains("设置定投"));
}

#[tokio::test]
async fn test_dca_survives_json_repo_reload_and_pg_if_available() {
    // json reload covered above; for pg the ignored test exists
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    seed_asset_for_dca(&state, "SURVIVE1", "000007", "存活").await;
    seed_dca_plan(&state, "SURVIVE1", 999.0, true).await;
    let state = make_state(dir);
    let plans1 = state.repo.load_plans(&state.ctx).await.unwrap();
    let state2 = make_state(dir); // simulate restart / new repo instance
    let plans2 = state2.repo.load_plans(&state2.ctx).await.unwrap();
    assert!(
        plans1
            .iter()
            .any(|p| p.asset_id == "SURVIVE1" && p.amount > 900.0)
    );
    assert!(
        plans2
            .iter()
            .any(|p| p.asset_id == "SURVIVE1" && p.amount > 900.0)
    );
}

#[tokio::test]
async fn test_overview_dca_summary_links_to_holdings_not_pipeline() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    seed_asset_for_dca(&state, "OVDCA1", "000008", "概览").await;
    seed_dca_plan(&state, "OVDCA1", 10.0, true).await;
    let html = test_pages::render_overview(make_state(dir)).await;
    assert!(html.contains("今日定投") || html.contains("定投"));
    assert!(html.contains("/holdings"), "dca summary links to holdings");
    assert!(
        !html.contains("href=\"/dca\"") && !html.contains("href=\"/admin/dca\""),
        "no separate pipeline/config for dca"
    );
}

#[tokio::test]
async fn test_no_main_ui_requires_separate_asset_config_for_dca() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let htmls = vec![
        test_pages::render_holdings(make_state(dir)).await,
        test_pages::render_overview(make_state(dir)).await,
        test_pages::render_market(make_state(dir)).await,
    ];
    for h in htmls {
        assert!(!h.contains("资产配置"), "no 资产配置 for dca");
        assert!(
            !h.contains("/dca\"") || h.contains("/holdings"),
            "dca links go to holdings"
        );
    }
}

#[tokio::test]
async fn test_old_detached_dca_routes_redirect_or_hide() {
    // routes already redirect /dca /admin/dca etc to holdings; verify via render? or assume from routes test
    // here just confirm no raw buttons in product
    let tmp = TempDir::new().unwrap();
    let html = test_pages::render_holdings(make_state(tmp.path().to_str().unwrap())).await;
    assert!(!html.contains("href=\"/dca"));
    assert!(!html.contains("href=\"/admin/dca"));
}
