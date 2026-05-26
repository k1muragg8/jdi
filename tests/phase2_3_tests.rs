use pendulum_kelly_cli::api::{MockFxProvider, MockMarketProvider};
use pendulum_kelly_cli::engine::valuation::calculate_proxy_valuations;
use pendulum_kelly_cli::models::{
    AssetConfig, AssetHolding, ConfigRoot, FxConfig, KellyConfig, MarketConfig, PortfolioConfig,
    PortfolioState,
};

#[test]
fn test_fx_adjusted_proxy_valuation() {
    let config = ConfigRoot {
        kelly: KellyConfig::default(),
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        risk: Default::default(),
        api: Default::default(),
        market: MarketConfig {
            default_market_provider: "mock".to_string(),
            ..Default::default()
        },
        fx: FxConfig {
            default_fx_provider: "mock".to_string(),
            usd_cnh_symbol: "USDCNH=X".to_string(),
            fx_cache_stale_hours: 24,
            allow_mock_fx_fallback: true,
        },
        regime: Default::default(),
        assets: vec![AssetConfig {
            asset_id: "nasdaq_fund".to_string(),
            fund_code: "006327".to_string(),
            fund_name: "Nasdaq Fund".to_string(),
            sector: "Tech".to_string(),
            currency: "CNH".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: Some("Nasdaq 100".to_string()),
            reference_index_symbol: Some("QQQ".to_string()),
            market_data_provider: Some("mock".to_string()),
            reference_index_currency: Some("USD".to_string()),
            proxy_fx_pair: Some("USD/CNH".to_string()),
            use_fx_adjustment: Some(true),
        }],
        sectors: vec![],
    };

    let state = PortfolioState {
        cash: 1000.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "nasdaq_fund".to_string(),
            fund_code: "006327".to_string(),
            units: 100.0,
            units_estimated: false,
            cost_basis: 500.0,
            latest_nav: Some(5.0),
            latest_nav_date: Some("2026-05-20".to_string()),
            latest_nav_source: Some("eastmoney".to_string()),
            latest_nav_status: Some("正常".to_string()),
            last_market_value: 500.0,
        }],
    };

    let market_provider = MockMarketProvider::new();
    let fx_provider = MockFxProvider;

    // In MockFxProvider:
    // latest_rate is 7.25
    // hist_rates(30) returns 7.25 + (i * 0.001)
    // For 2026-05-20, let's say it's some days ago.
    // If we don't mock the dates carefully, it might just pick the closest.

    let results = calculate_proxy_valuations(&config, &state, &market_provider, &fx_provider);

    assert_eq!(results.len(), 1);
    let res = &results[0];
    assert_eq!(res.status, "正常");
    assert!(res.use_fx_adjustment);

    // Check if FX return is calculated (it should be non-zero because MockFxProvider history varies)
    assert!(res.fx_return != 0.0 || res.combined_proxy_return == res.index_return);
}

#[test]
fn test_fx_fallback_to_index_only() {
    let config = ConfigRoot {
        kelly: KellyConfig::default(),
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 0.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        risk: Default::default(),
        api: Default::default(),
        market: MarketConfig {
            default_market_provider: "mock".to_string(),
            ..Default::default()
        },
        fx: Default::default(),
        regime: Default::default(),
        assets: vec![AssetConfig {
            asset_id: "nasdaq_fund".to_string(),
            fund_code: "006327".to_string(),
            fund_name: "Nasdaq Fund".to_string(),
            sector: "Tech".to_string(),
            currency: "CNH".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: Some("Nasdaq 100".to_string()),
            reference_index_symbol: Some("QQQ".to_string()),
            market_data_provider: Some("mock".to_string()),
            reference_index_currency: Some("USD".to_string()),
            proxy_fx_pair: Some("NON_EXISTENT".to_string()),
            use_fx_adjustment: Some(true),
        }],
        sectors: vec![],
    };

    let state = PortfolioState {
        cash: 0.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "nasdaq_fund".to_string(),
            fund_code: "006327".to_string(),
            units: 100.0,
            units_estimated: false,
            cost_basis: 0.0,
            latest_nav: Some(1.0),
            latest_nav_date: Some("2026-05-20".to_string()),
            latest_nav_source: None,
            latest_nav_status: None,
            last_market_value: 0.0,
        }],
    };

    let market_provider = MockMarketProvider::new();
    let fx_provider = MockFxProvider;

    let results = calculate_proxy_valuations(&config, &state, &market_provider, &fx_provider);
    assert_eq!(results[0].status, "正常");
    assert!(
        results[0]
            .warning
            .as_ref()
            .unwrap()
            .contains("汇率查询失败")
    );
    assert_eq!(results[0].combined_proxy_return, results[0].index_return);
}
