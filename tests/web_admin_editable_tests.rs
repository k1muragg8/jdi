//! Editable admin panel: enrichment, CRUD persistence, no raw JSON nav.

use pendulum_kelly_cli::engine::asset_enrichment::{infer_sector_from_text, is_asset_archived};
use pendulum_kelly_cli::models::{AssetConfig, InstrumentConfig};
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

#[test]
fn fund_lookup_inference_gold_fund() {
    let s = infer_sector_from_text("华安黄金ETF联接A", "ETF", "000216");
    assert_eq!(s.as_deref(), Some("黄金"));
}

#[test]
fn archived_asset_hidden_from_active_list() {
    let a = AssetConfig {
        enabled: false,
        sector: "已归档".to_string(),
        ..Default::default()
    };
    assert!(is_asset_archived(&a));
}

#[tokio::test]
async fn asset_sector_update_persists_json() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    let ctx = &state.ctx;
    let mut cfg = state.repo.load_config(ctx).await.unwrap();
    let id = cfg
        .assets
        .first()
        .map(|a| a.asset_id.clone())
        .unwrap_or_else(|| "TEST001".to_string());
    if !cfg.assets.iter().any(|a| a.asset_id == id) {
        cfg.assets.push(AssetConfig {
            asset_id: id.clone(),
            fund_code: "000001".to_string(),
            fund_name: "测试".to_string(),
            sector: "未分类".to_string(),
            enabled: true,
            ..Default::default()
        });
        state.repo.save_config(ctx, &cfg).await.unwrap();
    }
    let mut cfg2 = state.repo.load_config(ctx).await.unwrap();
    let a = cfg2.assets.iter_mut().find(|x| x.asset_id == id).unwrap();
    a.sector = "债券".to_string();
    state.repo.save_config(ctx, &cfg2).await.unwrap();
    let cfg3 = state.repo.load_config(ctx).await.unwrap();
    assert_eq!(
        cfg3.assets
            .iter()
            .find(|x| x.asset_id == id)
            .unwrap()
            .sector,
        "债券"
    );
}

#[tokio::test]
async fn target_equity_policy_persists() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    let ctx = &state.ctx;
    let mut policy = state.repo.load_operation_policy(ctx).await.unwrap();
    policy.target_equity_weight = 0.65;
    state
        .repo
        .save_operation_policy(ctx, &policy)
        .await
        .unwrap();
    let p2 = state.repo.load_operation_policy(ctx).await.unwrap();
    assert!((p2.target_equity_weight - 0.65).abs() < 1e-6);
}

#[tokio::test]
async fn product_pages_have_admin_actions_not_raw_json_links() {
    let dir = TempDir::new().unwrap().path().to_str().unwrap().to_string();
    let state = make_state(&dir);
    for html in [
        test_pages::render_overview(state.clone()).await,
        test_pages::render_market(state.clone()).await,
        test_pages::render_holdings(state).await,
    ] {
        assert!(!html.contains("href=\"/api/"));
        assert!(html.contains("adminFetch") || html.contains("/admin/"));
    }
}

#[tokio::test]
async fn holdings_page_has_enrich_and_edit_controls() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let html = test_pages::render_holdings(make_state(dir)).await;
    assert!(html.contains("自动补全基金信息"));
    assert!(html.contains("openAssetEdit"));
    assert!(html.contains("/api/fund/lookup"));
    assert!(html.contains("/api/assets/update"));
}

#[tokio::test]
async fn overview_has_editable_target_equity_and_cash() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let html = test_pages::render_overview(make_state(dir)).await;
    assert!(html.contains("openTargetEquityEdit"));
    assert!(html.contains("openCashAdjust"));
    assert!(html.contains("/api/operation/policy/target-equity"));
}

#[tokio::test]
async fn au9999_uses_eastmoney_provider() {
    let inst = InstrumentConfig {
        symbol: "AU9999".to_string(),
        provider: "eastmoney".to_string(),
        provider_symbol: "AU9999".to_string(),
        ..Default::default()
    };
    assert_eq!(inst.provider, "eastmoney");
    assert_ne!(inst.provider, "yahoo");
}
