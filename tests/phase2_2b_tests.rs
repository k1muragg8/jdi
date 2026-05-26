use pendulum_kelly_cli::api::{MockFxProvider, MockMarketProvider};
use pendulum_kelly_cli::engine::valuation::calculate_proxy_valuations;
use pendulum_kelly_cli::models::{
    AdjustedDecisionConfig, AssetConfig, AssetHolding, ConfigRoot, KellyConfig, MarketConfig,
    PortfolioConfig, PortfolioState,
};

#[test]
fn test_proxy_valuation_calculation() {
    let config = ConfigRoot {
        adjusted_decision: AdjustedDecisionConfig::default(),
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
        fx: Default::default(),
        market: MarketConfig {
            default_market_provider: "mock".to_string(),
            ..Default::default()
        },
        regime: Default::default(),
        reconciliation: Default::default(),
        assets: vec![AssetConfig {
            asset_id: "nasdaq_fund".to_string(),
            fund_code: "006327".to_string(),
            fund_name: "Nasdaq Fund".to_string(),
            sector: "Tech".to_string(),
            currency: "CNY".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: Some("Nasdaq 100".to_string()),
            reference_index_symbol: Some("QQQ".to_string()),
            market_data_provider: Some("mock".to_string()),
            reference_index_currency: None,
            proxy_fx_pair: None,
            use_fx_adjustment: Some(false),
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

    let provider = MockMarketProvider::new();
    let fx_provider = MockFxProvider;
    let results = calculate_proxy_valuations(&config, &state, &provider, &fx_provider);

    assert_eq!(results.len(), 1);
    let res = &results[0];
    assert_eq!(res.status, "正常");

    // Mock QQQ latest is 450.50.
    // In MockMarketProvider, history for 2026-05-20 will be 450.50 too if not careful.
    // Actually MockMarketProvider returns same price for all days currently.
    // Proxy return will be 0.0 if prices are same.
    assert_eq!(res.proxy_return, 0.0);
    assert_eq!(res.estimated_nav, 5.0);
    assert_eq!(res.estimated_market_value, 500.0);
}

#[test]
fn test_proxy_valuation_missing_symbol() {
    let config = ConfigRoot {
        adjusted_decision: AdjustedDecisionConfig::default(),
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
        fx: Default::default(),
        market: Default::default(),
        regime: Default::default(),
        reconciliation: Default::default(),
        assets: vec![AssetConfig {
            asset_id: "test_asset".to_string(),
            fund_code: "123".to_string(),
            fund_name: "Test".to_string(),
            sector: "S".to_string(),
            currency: "CNY".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: None,
            reference_index_symbol: None,
            market_data_provider: None,
            reference_index_currency: None,
            proxy_fx_pair: None,
            use_fx_adjustment: Some(false),
        }],
        sectors: vec![],
    };

    let state = PortfolioState {
        cash: 0.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "test_asset".to_string(),
            fund_code: "123".to_string(),
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

    let provider = MockMarketProvider::new();
    let fx_provider = MockFxProvider;
    let results = calculate_proxy_valuations(&config, &state, &provider, &fx_provider);
    assert_eq!(results[0].status, "缺少参考指数");
}
