use pendulum_kelly_cli::api::EastMoneyMarketProvider;
use pendulum_kelly_cli::api::MarketDataProvider;
use pendulum_kelly_cli::engine::{
    MarketListFilter, archive_instrument, cleanup_test_instruments, duplicate_instrument_ids,
    is_instrument_archived, is_test_instrument, matches_filter, migrate_au9999_provider,
    normalize_market_price, restore_default_instruments,
};
use pendulum_kelly_cli::models::instrument::AssetClass;
use pendulum_kelly_cli::models::{InstrumentConfig, MarketCache, MarketCacheEntry, MarketPrice};
use pendulum_kelly_cli::repository::RepositoryContext;
use pendulum_kelly_cli::repository::json::JsonRepository;
use pendulum_kelly_cli::storage::instrument_store::get_default_instruments;
use pendulum_kelly_cli::web::test_pages;
use pendulum_kelly_cli::web::{AppState, BackgroundRefreshStatus};
use std::sync::Arc;
use tempfile::TempDir;

fn sample(inst_id: &str, sym: &str, enabled: bool, archived: bool) -> InstrumentConfig {
    InstrumentConfig {
        instrument_id: inst_id.to_string(),
        symbol: sym.to_string(),
        display_symbol: None,
        name: sym.to_string(),
        name_zh: None,
        name_en: None,
        description_zh: None,
        category_zh: None,
        display_label: None,
        asset_class: AssetClass::Etf,
        provider: "yahoo".to_string(),
        provider_symbol: sym.to_string(),
        market: None,
        exchange: None,
        currency: "USD".to_string(),
        quote_unit: "1".to_string(),
        price_unit: "1".to_string(),
        timezone: None,
        enabled,
        archived,
        priority: 0,
        tags: vec![],
        note: None,
    }
}

#[test]
fn quote_with_previous_close_calculates_change() {
    let price = MarketPrice {
        symbol: "QQQ".to_string(),
        price: 450.0,
        date: "2026-06-05".to_string(),
        currency: "USD".to_string(),
        source: "yahoo".to_string(),
        is_stale: false,
        previous_close: Some(440.0),
        change: None,
        change_percent: None,
    };
    let n = normalize_market_price(price);
    assert!((n.change.unwrap() - 10.0).abs() < 0.01);
    assert!((n.change_percent.unwrap() - (10.0 / 440.0 * 100.0)).abs() < 0.01);
}

#[test]
fn archive_hides_from_default_active_filter() {
    let mut i = sample("t", "TEST", true, false);
    assert!(is_test_instrument(&i));
    archive_instrument(&mut i);
    let dups = duplicate_instrument_ids(&[i.clone()]);
    assert!(!matches_filter(&i, MarketListFilter::Active, &dups));
    assert!(matches_filter(&i, MarketListFilter::Archived, &dups));
}

#[test]
fn au9999_default_provider_is_eastmoney() {
    let defs = get_default_instruments();
    let au = defs
        .iter()
        .find(|i| i.symbol == "AU9999")
        .expect("AU9999 default");
    assert_eq!(au.provider, "eastmoney");
    assert_eq!(au.provider_symbol, "118.AU9999");
    assert!(au.enabled);
}

#[test]
fn restore_defaults_migrates_au9999_from_yahoo() {
    let mut list = vec![InstrumentConfig {
        instrument_id: "au9999".to_string(),
        symbol: "AU9999".to_string(),
        display_symbol: Some("AU9999".to_string()),
        name: "AU9999".to_string(),
        name_zh: Some("上海黄金交易所 Au9999 现货黄金".to_string()),
        name_en: None,
        description_zh: None,
        category_zh: None,
        display_label: None,
        asset_class: AssetClass::SpotCommodity,
        provider: "yahoo".to_string(),
        provider_symbol: "AU9999".to_string(),
        market: None,
        exchange: None,
        currency: "CNY".to_string(),
        quote_unit: "g".to_string(),
        price_unit: "CNY/g".to_string(),
        timezone: None,
        enabled: true,
        archived: false,
        priority: 0,
        tags: vec![],
        note: None,
    }];
    let _ = restore_default_instruments(&mut list, false);
    let au = list.iter().find(|i| i.symbol == "AU9999").unwrap();
    assert_eq!(au.provider, "eastmoney");
    assert_eq!(au.provider_symbol, "118.AU9999");
}

#[test]
fn migrate_au9999_provider_from_manual() {
    let mut inst = sample("au9999", "AU9999", true, false);
    inst.provider = "manual".to_string();
    inst.provider_symbol = "AU9999".to_string();
    migrate_au9999_provider(&mut inst);
    assert_eq!(inst.provider, "eastmoney");
    assert_eq!(inst.provider_symbol, "118.AU9999");
}

#[test]
fn eastmoney_au9999_quote_normalizes_change() {
    let body: serde_json::Value = serde_json::json!({
        "rc": 0,
        "data": { "f43": 96899, "f57": "AU9999", "f60": 97450, "f169": -551, "f170": -57 }
    });
    let p = EastMoneyMarketProvider::parse_quote("118.AU9999", "118.AU9999", &body).unwrap();
    assert!(p.change.is_some());
    assert!(p.change_percent.is_some());
    assert_eq!(p.currency, "CNY");
}

#[test]
#[ignore = "live network"]
fn eastmoney_au9999_live_fetch() {
    let provider = EastMoneyMarketProvider::new(15);
    let p = provider.fetch_latest_price("118.AU9999").unwrap();
    assert!(p.price > 0.0);
}

#[test]
fn restore_defaults_idempotent_no_duplicate_qqq() {
    let mut list = get_default_instruments();
    let len = list.len();
    let (a1, _) = restore_default_instruments(&mut list, false);
    assert_eq!(a1, 0);
    let (a2, _) = restore_default_instruments(&mut list, false);
    assert_eq!(a2, 0);
    assert_eq!(list.len(), len);
    assert_eq!(list.iter().filter(|i| i.symbol == "QQQ").count(), 1);
}

#[test]
fn cleanup_test_archives_test_symbol() {
    let mut list = vec![
        sample("t1", "TEST", true, false),
        sample("nasdaq_qqq", "QQQ", true, false),
    ];
    let n = cleanup_test_instruments(&mut list, false);
    assert_eq!(n, 1);
    let test_row = list.iter().find(|i| i.symbol == "TEST").unwrap();
    assert!(is_instrument_archived(test_row));
}

fn make_state(dir: &str) -> Arc<AppState> {
    std::fs::create_dir_all(dir).ok();
    for (src, name) in [
        ("tests/fixtures/json_backend/config.toml", "config.toml"),
        (
            "tests/fixtures/json_backend/portfolio_state.json",
            "portfolio_state.json",
        ),
        (
            "tests/fixtures/json_backend/transactions.json",
            "transactions.json",
        ),
    ] {
        let dest = format!("{}/{}", dir, name);
        if !std::path::Path::new(&dest).exists() {
            let _ = std::fs::copy(src, &dest);
        }
    }
    Arc::new(AppState {
        repo: Arc::new(JsonRepository::new_with_defaults(dir)),
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
async fn market_page_has_filters_restore_and_usable_inputs() {
    let tmp = TempDir::new().unwrap();
    let html = test_pages::render_market(make_state(tmp.path().to_str().unwrap())).await;
    assert!(html.contains("filter=active"));
    assert!(html.contains("恢复默认"));
    assert!(html.contains("清理测试"));
    assert!(html.contains("market-input-name"));
    assert!(html.contains("min-width:260px") || html.contains("min-width: 260px"));
    assert!(html.contains("instEditModal"));
}

#[tokio::test]
async fn archived_test_hidden_from_default_market_render() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    let ctx = &state.ctx;
    let mut instruments = state.repo.load_instruments(ctx).await.unwrap_or_default();
    instruments.push(sample("test_row", "TEST", true, false));
    instruments.push(sample("qqq_row", "QQQ", true, false));
    state
        .repo
        .save_instruments(ctx, &instruments)
        .await
        .unwrap();

    let html = test_pages::render_market(make_state(dir)).await;
    assert!(!html.contains("<code>TEST</code>") || html.contains("filter=test"));
    assert!(html.contains("QQQ"));
}

#[tokio::test]
async fn quote_cache_persists_after_reload_json() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_str().unwrap();
    let state = make_state(dir);
    let ctx = &state.ctx;
    let mut cache = MarketCache::default();
    cache.entries.push(MarketCacheEntry {
        symbol: "QQQ".to_string(),
        price: 450.0,
        date: "2026-06-05".to_string(),
        currency: "USD".to_string(),
        source: "yahoo".to_string(),
        fetched_at: "2026-06-05 12:00:00".to_string(),
        previous_close: Some(440.0),
        change: Some(10.0),
        change_percent: Some(2.27),
        status: Some("ok".to_string()),
        error_message: None,
    });
    state.repo.save_market_cache(ctx, &cache).await.unwrap();

    let state2 = make_state(dir);
    let loaded = state2.repo.load_market_cache(&state2.ctx).await.unwrap();
    let entry = loaded.entries.iter().find(|e| e.symbol == "QQQ").unwrap();
    assert_eq!(entry.change, Some(10.0));
    assert_eq!(entry.previous_close, Some(440.0));
}
