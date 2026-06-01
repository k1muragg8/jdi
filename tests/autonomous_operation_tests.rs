use pendulum_kelly_cli::engine;
use pendulum_kelly_cli::models::*;
use pendulum_kelly_cli::repository::traits::{
    CacheRepository, DcaRepository, OperationRepository, PortfolioRepository,
};
use pendulum_kelly_cli::repository::{JsonRepository, RepositoryContext};
use std::collections::HashMap;
use tempfile::tempdir;

async fn setup_test_env(base_dir: &str) -> (JsonRepository, RepositoryContext, ConfigRoot) {
    let repo = JsonRepository::new_with_defaults(base_dir);
    let ctx = RepositoryContext::default();

    let mut config = ConfigRoot::default();
    config.portfolio.name = "Test Portfolio".to_string();
    config.assets.push(AssetConfig {
        asset_id: "fund1".to_string(),
        fund_code: "000001".to_string(),
        fund_name: "Fund 1".to_string(),
        sector: "Sector A".to_string(),
        currency: "CNY".to_string(),
        valuation_method: "nav".to_string(),
        enabled: true,
        reference_index_symbol: Some("INDEX1".to_string()),
        reference_index_name: None,
        market_data_provider: None,
        reference_instrument_id: None,
        reference_instrument_symbol: None,
        reference_index_currency: None,
        proxy_fx_pair: None,
        use_fx_adjustment: None,
    });
    repo.save_config(&ctx, &config).await.unwrap();

    let state = PortfolioState {
        cash: 100000.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "fund1".to_string(),
            fund_code: "000001".to_string(),
            units: 1000.0,
            units_estimated: false,
            cost_basis: 1000.0,
            latest_nav: Some(1.0),
            latest_nav_date: Some("2026-06-01".to_string()),
            latest_nav_source: Some("test".to_string()),
            latest_nav_status: Some("正常".to_string()),
            last_market_value: 1000.0,
        }],
    };
    repo.save_state(&ctx, &state).await.unwrap();

    let plan = DcaPlan {
        plan_id: "plan1".to_string(),
        asset_id: "fund1".to_string(),
        fund_code: "000001".to_string(),
        fund_name: "Fund 1".to_string(),
        amount: 1000.0,
        currency: "CNY".to_string(),
        frequency: DcaFrequency::Daily,
        enabled: true,
        start_date: "2026-01-01".to_string(),
        end_date: None,
        weekday: None,
        month_day: None,
        priority: 0,
        note: None,
        created_at: "now".to_string(),
        updated_at: "now".to_string(),
    };
    repo.save_plans(&ctx, &[plan]).await.unwrap();

    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut nav_cache = NavCache::default();
    nav_cache.entries.push(NavCacheEntry {
        fund_code: "000001".to_string(),
        nav: 1.0,
        accumulated_nav: None,
        nav_date: "2026-06-01".to_string(),
        currency: "CNY".to_string(),
        source: "test".to_string(),
        fetched_at: now_str,
    });
    repo.save_nav_cache(&ctx, &nav_cache).await.unwrap();

    (repo, ctx, config)
}

async fn setup_regime(
    repo: &JsonRepository,
    ctx: &RepositoryContext,
    symbol: &str,
    prices: Vec<f64>,
    config: &RegimeConfig,
) {
    let mut candles = Vec::new();
    for (i, &p) in prices.iter().enumerate() {
        candles.push(Candle {
            symbol: symbol.to_string(),
            date: format!("2026-05-{:02}", 60 - i),
            open: p,
            high: p,
            low: p,
            close: p,
            volume: 0,
            source: "test".to_string(),
        });
    }
    let regime = engine::regime::calculate_market_regime(symbol, &candles, config);
    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let regime_cache = RegimeCache {
        fetched_at: now_str,
        entries: vec![RegimeCacheEntry {
            symbol: symbol.to_string(),
            result: regime,
        }],
    };
    repo.save_regime_cache(ctx, &regime_cache).await.unwrap();
}

#[tokio::test]
async fn test_autonomous_operation_idempotency() {
    let dir = tempdir().unwrap();
    let (repo, ctx, config) = setup_test_env(dir.path().to_str().unwrap()).await;

    let prices = vec![100.0; 60];
    setup_regime(&repo, &ctx, "INDEX1", prices, &config.regime).await;

    let report1 = engine::run_autonomous_operation(&repo, &ctx, &config)
        .await
        .unwrap();
    assert_eq!(report1.dca_execution_result.executed_count, 1);

    let report2 = engine::run_autonomous_operation(&repo, &ctx, &config)
        .await
        .unwrap();
    assert_eq!(report2.dca_execution_result.executed_count, 0);
    assert_eq!(report2.suggestions[0].status, "skip");
    assert!(report2.suggestions[0].reason.contains("今日已执行"));
}

#[tokio::test]
async fn test_autonomous_operation_high_volatility_reduction() {
    let dir = tempdir().unwrap();
    let (repo, ctx, config) = setup_test_env(dir.path().to_str().unwrap()).await;

    // Create highly volatile prices
    let mut prices = Vec::new();
    for i in 0..60 {
        if i % 2 == 0 {
            prices.push(110.0);
        } else {
            prices.push(90.0);
        }
    }
    setup_regime(&repo, &ctx, "INDEX1", prices, &config.regime).await;

    let policy = OperationPolicy {
        kelly_enabled: true,
        ..Default::default()
    };
    repo.save_operation_policy(&ctx, &policy).await.unwrap();

    let report = engine::run_autonomous_operation(&repo, &ctx, &config)
        .await
        .unwrap();
    let sug = &report.suggestions[0];
    assert!(sug.volatility > 0.4);
    assert!(sug.kelly_multiplier < 1.0);
}

#[tokio::test]
async fn test_autonomous_operation_target_reached_pauses_dca() {
    let dir = tempdir().unwrap();
    let (repo, ctx, config) = setup_test_env(dir.path().to_str().unwrap()).await;

    let prices = vec![100.0; 60];
    setup_regime(&repo, &ctx, "INDEX1", prices, &config.regime).await;

    // Asset value is 1000. Total value is 101000. Current weight ~ 0.99%.
    // Set target to 0.5%
    let mut target_asset_weights = HashMap::new();
    target_asset_weights.insert("fund1".to_string(), 0.005);

    let policy = OperationPolicy {
        target_asset_weights,
        dca_auto_pause_when_target_reached: true,
        ..Default::default()
    };
    repo.save_operation_policy(&ctx, &policy).await.unwrap();

    let report = engine::run_autonomous_operation(&repo, &ctx, &config)
        .await
        .unwrap();
    assert_eq!(report.dca_execution_result.executed_count, 0);
    assert_eq!(report.suggestions[0].status, "pause");
    assert!(report.suggestions[0].reason.contains("资产权重"));

    // Check if plan was auto-paused
    let plans = repo.load_plans(&ctx).await.unwrap();
    assert!(!plans[0].enabled);
}

#[tokio::test]
async fn test_autonomous_operation_below_target_allows_dca() {
    let dir = tempdir().unwrap();
    let (repo, ctx, config) = setup_test_env(dir.path().to_str().unwrap()).await;

    let prices = vec![100.0; 60];
    setup_regime(&repo, &ctx, "INDEX1", prices, &config.regime).await;

    // Current weight ~ 0.99%. Set target to 5%
    let mut target_asset_weights = HashMap::new();
    target_asset_weights.insert("fund1".to_string(), 0.05);

    let policy = OperationPolicy {
        target_asset_weights,
        ..Default::default()
    };
    repo.save_operation_policy(&ctx, &policy).await.unwrap();

    let report = engine::run_autonomous_operation(&repo, &ctx, &config)
        .await
        .unwrap();
    assert_eq!(report.dca_execution_result.executed_count, 1);
    assert_eq!(report.suggestions[0].status, "execute");
}

#[tokio::test]
async fn test_autonomous_operation_cash_reserve_limit() {
    let dir = tempdir().unwrap();
    let (repo, ctx, config) = setup_test_env(dir.path().to_str().unwrap()).await;

    let prices = vec![100.0; 60];
    setup_regime(&repo, &ctx, "INDEX1", prices, &config.regime).await;

    // Set cash to 10500. Reserve is 10000. Buying 1000 would violate reserve.
    repo.save_state(
        &ctx,
        &PortfolioState {
            cash: 10500.0,
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let policy = OperationPolicy {
        min_cash_reserve: 10000.0,
        ..Default::default()
    };
    repo.save_operation_policy(&ctx, &policy).await.unwrap();

    let report = engine::run_autonomous_operation(&repo, &ctx, &config)
        .await
        .unwrap();
    assert_eq!(report.dca_execution_result.executed_count, 0);
    assert_eq!(report.suggestions[0].status, "skip");
    assert!(report.suggestions[0].reason.contains("现金储备不足"));
}

#[tokio::test]
async fn test_autonomous_operation_daily_buy_cap() {
    let dir = tempdir().unwrap();
    let (repo, ctx, config) = setup_test_env(dir.path().to_str().unwrap()).await;

    let prices = vec![100.0; 60];
    setup_regime(&repo, &ctx, "INDEX1", prices, &config.regime).await;

    let policy = OperationPolicy {
        max_daily_buy_amount: 500.0, // Plan amount is 1000
        ..Default::default()
    };
    repo.save_operation_policy(&ctx, &policy).await.unwrap();

    let report = engine::run_autonomous_operation(&repo, &ctx, &config)
        .await
        .unwrap();
    assert_eq!(report.dca_execution_result.executed_count, 0);
    assert_eq!(report.suggestions[0].status, "skip");
    assert!(report.suggestions[0].reason.contains("今日买入已达上限"));
}

#[tokio::test]
async fn test_autonomous_operation_single_asset_buy_cap() {
    let dir = tempdir().unwrap();
    let (repo, ctx, config) = setup_test_env(dir.path().to_str().unwrap()).await;

    let prices = vec![100.0; 60];
    setup_regime(&repo, &ctx, "INDEX1", prices, &config.regime).await;

    let policy = OperationPolicy {
        max_single_asset_buy_amount: 500.0, // Plan amount is 1000
        ..Default::default()
    };
    repo.save_operation_policy(&ctx, &policy).await.unwrap();

    let report = engine::run_autonomous_operation(&repo, &ctx, &config)
        .await
        .unwrap();
    assert_eq!(report.dca_execution_result.executed_count, 1);
    assert_eq!(report.suggestions[0].suggested_amount, 500.0);
    assert!(
        report.suggestions[0]
            .caps_applied
            .contains("单资产日买入上限")
    );
}

#[tokio::test]
async fn test_autonomous_operation_missing_benchmark_warning() {
    let dir = tempdir().unwrap();
    let (repo, ctx, mut config) = setup_test_env(dir.path().to_str().unwrap()).await;

    // Remove benchmark mapping
    config.assets[0].reference_index_symbol = None;
    config.assets[0].reference_instrument_symbol = None;
    repo.save_config(&ctx, &config).await.unwrap();

    let report = engine::run_autonomous_operation(&repo, &ctx, &config)
        .await
        .unwrap();
    assert!(report.warnings.iter().any(|w| w.contains("未配置基准指数")));
}

#[tokio::test]
async fn test_autonomous_operation_missing_nav_skip() {
    let dir = tempdir().unwrap();
    let (repo, ctx, mut config) = setup_test_env(dir.path().to_str().unwrap()).await;

    // Change fund_code to something that won't be in cache and won't refresh correctly if it tried
    config.assets[0].fund_code = "999999".to_string();
    repo.save_config(&ctx, &config).await.unwrap();

    // Update plan to match new fund_code
    let mut plans = repo.load_plans(&ctx).await.unwrap();
    plans[0].fund_code = "999999".to_string();
    repo.save_plans(&ctx, &plans).await.unwrap();

    // Setup NAV cache with a DIFFERENT fund to make nav_stale = false
    let mut nav_cache = NavCache::default();
    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    nav_cache.entries.push(NavCacheEntry {
        fund_code: "OTHER".to_string(),
        nav: 1.0,
        accumulated_nav: None,
        nav_date: "2026-06-01".to_string(),
        currency: "CNY".to_string(),
        source: "test".to_string(),
        fetched_at: now_str.clone(),
    });
    repo.save_nav_cache(&ctx, &nav_cache).await.unwrap();

    let prices = vec![100.0; 60];
    setup_regime(&repo, &ctx, "INDEX1", prices, &config.regime).await;

    let report = engine::run_autonomous_operation(&repo, &ctx, &config)
        .await
        .unwrap();
    assert_eq!(report.dca_execution_result.executed_count, 0);
    assert_eq!(report.suggestions[0].status, "skip");
    assert!(report.suggestions[0].reason.contains("缺少基金净值数据"));
}
