use chrono::{Duration, Local};
use pendulum_kelly_cli::api::{GenericHttpFundProvider, MockFundProvider, create_fund_provider};
use pendulum_kelly_cli::engine::mark_to_market;
use pendulum_kelly_cli::models::{
    ApiConfig, AssetConfig, AssetHolding, ConfigRoot, NavCache, NavCacheEntry, PortfolioConfig,
    PortfolioState,
};

#[test]
fn test_provider_selection() {
    let mut config = ApiConfig::default();

    config.default_fund_provider = "mock".to_string();
    let p1 = create_fund_provider(&config);
    assert!(p1.fetch_latest_nav("006327").is_ok());

    config.default_fund_provider = "generic_http".to_string();
    let p2 = create_fund_provider(&config);
    assert!(p2.fetch_latest_nav("006327").is_err());

    config.default_fund_provider = "eastmoney".to_string();
    let _p3 = create_fund_provider(&config);
    // eastmoney may succeed or fail depending on environment, don't assert on side effect here
}
#[test]
fn test_mtm_mock_fallback() {
    let config = ConfigRoot {
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        risk: Default::default(),
        market: Default::default(),
        fx: Default::default(),
        api: ApiConfig {
            default_fund_provider: "generic_http".to_string(), // Will fail
            allow_mock_fallback: true,
            ..Default::default()
        },
        assets: vec![AssetConfig {
            asset_id: "test_asset".to_string(),
            fund_code: "006327".to_string(),
            fund_name: "Test".to_string(),
            sector: "Test".to_string(),
            currency: "CNY".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: None,
            reference_index_symbol: None,
            reference_index_currency: None,
            proxy_fx_pair: None,
            use_fx_adjustment: None,
            market_data_provider: None,
        }],
        sectors: vec![],
    };

    let mut state = PortfolioState {
        cash: 0.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "test_asset".to_string(),
            fund_code: "006327".to_string(),
            units: 100.0,
            units_estimated: false,
            cost_basis: 0.0,
            latest_nav: None,
            latest_nav_date: None,
            latest_nav_source: None,
            latest_nav_status: None,
            last_market_value: 0.0,
        }],
    };

    let mut cache = NavCache::default();
    let provider = GenericHttpFundProvider::new(10, 2);

    mark_to_market(&config, &mut state, &provider, &mut cache).unwrap();

    let holding = &state.asset_holdings[0];
    assert_eq!(holding.latest_nav, Some(5.38)); // From Mock
    assert_eq!(holding.latest_nav_source, Some("mock".to_string()));
    assert_eq!(holding.latest_nav_status, Some("模拟".to_string()));
}
#[test]
fn test_mtm_with_cache_fallback() {
    let config = ConfigRoot {
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        risk: Default::default(),
        market: Default::default(),
        fx: Default::default(),
        api: ApiConfig {
            default_fund_provider: "generic_http".to_string(), // Will fail
            fund_nav_stale_days: 3,
            allow_mock_fallback: false,
            ..Default::default()
        },
        assets: vec![AssetConfig {
            asset_id: "test_asset".to_string(),
            fund_code: "123".to_string(),
            fund_name: "Test".to_string(),
            sector: "Test".to_string(),
            currency: "CNY".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: None,
            reference_index_symbol: None,
            reference_index_currency: None,
            proxy_fx_pair: None,
            use_fx_adjustment: None,
            market_data_provider: None,
        }],
        sectors: vec![],
    };

    let mut state = PortfolioState {
        cash: 0.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "test_asset".to_string(),
            fund_code: "123".to_string(),
            units: 100.0,
            units_estimated: false,
            cost_basis: 0.0,
            latest_nav: None,
            latest_nav_date: None,
            latest_nav_source: None,
            latest_nav_status: None,
            last_market_value: 0.0,
        }],
    };

    let mut cache = NavCache {
        entries: vec![NavCacheEntry {
            fund_code: "123".to_string(),
            nav: 1.5,
            accumulated_nav: None,
            nav_date: Local::now().format("%Y-%m-%d").to_string(),
            currency: "CNY".to_string(),
            source: "eastmoney".to_string(),
            fetched_at: "".to_string(),
        }],
    };

    let provider = GenericHttpFundProvider::new(10, 2);

    mark_to_market(&config, &mut state, &provider, &mut cache).unwrap();

    let holding = &state.asset_holdings[0];
    assert_eq!(holding.latest_nav, Some(1.5));
    assert_eq!(holding.latest_nav_status, Some("正常".to_string()));
}

#[test]
fn test_mtm_stale_cache() {
    let config = ConfigRoot {
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        risk: Default::default(),
        market: Default::default(),
        fx: Default::default(),
        api: ApiConfig {
            default_fund_provider: "generic_http".to_string(), // Will fail
            fund_nav_stale_days: 3,
            allow_mock_fallback: false,
            ..Default::default()
        },
        assets: vec![AssetConfig {
            asset_id: "test_asset".to_string(),
            fund_code: "123".to_string(),
            fund_name: "Test".to_string(),
            sector: "Test".to_string(),
            currency: "CNY".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: None,
            reference_index_symbol: None,
            reference_index_currency: None,
            proxy_fx_pair: None,
            use_fx_adjustment: None,
            market_data_provider: None,
        }],
        sectors: vec![],
    };

    let mut state = PortfolioState {
        cash: 0.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "test_asset".to_string(),
            fund_code: "123".to_string(),
            units: 100.0,
            units_estimated: false,
            cost_basis: 0.0,
            latest_nav: None,
            latest_nav_date: None,
            latest_nav_source: None,
            latest_nav_status: None,
            last_market_value: 0.0,
        }],
    };

    // 5 days ago
    let stale_date = (Local::now() - Duration::days(5))
        .format("%Y-%m-%d")
        .to_string();

    let mut cache = NavCache {
        entries: vec![NavCacheEntry {
            fund_code: "123".to_string(),
            nav: 1.5,
            accumulated_nav: None,
            nav_date: stale_date,
            currency: "CNY".to_string(),
            source: "eastmoney".to_string(),
            fetched_at: "".to_string(),
        }],
    };

    let provider = GenericHttpFundProvider::new(10, 2);

    mark_to_market(&config, &mut state, &provider, &mut cache).unwrap();

    let holding = &state.asset_holdings[0];
    assert_eq!(holding.latest_nav, Some(1.5));
    assert_eq!(holding.latest_nav_status, Some("过期".to_string()));
}

#[test]
fn test_mtm_continue_on_failure() {
    let config = ConfigRoot {
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        risk: Default::default(),
        market: Default::default(),
        fx: Default::default(),
        api: ApiConfig {
            default_fund_provider: "mock".to_string(),
            allow_mock_fallback: false,
            ..Default::default()
        },
        assets: vec![
            AssetConfig {
                asset_id: "valid".to_string(),
                fund_code: "006327".to_string(),
                fund_name: "Valid".to_string(),
                sector: "Test".to_string(),
                currency: "CNY".to_string(),
                valuation_method: "nav".to_string(),
                enabled: true,
                reference_index_name: None,
                reference_index_symbol: None,
                reference_index_currency: None,
                proxy_fx_pair: None,
                use_fx_adjustment: None,
                market_data_provider: None,
            },
            AssetConfig {
                asset_id: "invalid".to_string(),
                fund_code: "999999".to_string(),
                fund_name: "Invalid".to_string(),
                sector: "Test".to_string(),
                currency: "CNY".to_string(),
                valuation_method: "nav".to_string(),
                enabled: true,
                reference_index_name: None,
                reference_index_symbol: None,
                reference_index_currency: None,
                proxy_fx_pair: None,
                use_fx_adjustment: None,
                market_data_provider: None,
            },
        ],
        sectors: vec![],
    };

    let mut state = PortfolioState {
        cash: 0.0,
        asset_holdings: vec![
            AssetHolding {
                asset_id: "valid".to_string(),
                fund_code: "006327".to_string(),
                units: 100.0,
                units_estimated: false,
                cost_basis: 0.0,
                latest_nav: None,
                latest_nav_date: None,
                latest_nav_source: None,
                latest_nav_status: None,
                last_market_value: 0.0,
            },
            AssetHolding {
                asset_id: "invalid".to_string(),
                fund_code: "999999".to_string(),
                units: 100.0,
                units_estimated: false,
                cost_basis: 0.0,
                latest_nav: None,
                latest_nav_date: None,
                latest_nav_source: None,
                latest_nav_status: None,
                last_market_value: 0.0,
            },
        ],
    };

    let mut cache = NavCache::default();
    let provider = MockFundProvider::new();

    mark_to_market(&config, &mut state, &provider, &mut cache).unwrap();

    assert!(state.asset_holdings[0].latest_nav.is_some());
    assert_eq!(state.asset_holdings[1].latest_nav, None);
    assert_eq!(
        state.asset_holdings[1].latest_nav_status,
        Some("获取失败".to_string())
    );
}
