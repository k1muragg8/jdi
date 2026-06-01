use pendulum_kelly_cli::engine::{calculate_portfolio_summary, generate_buy_suggestions};
use pendulum_kelly_cli::models::{
    AdjustedDecisionConfig, AssetConfig, AssetHolding, ConfigRoot, KellyConfig, PortfolioConfig,
    PortfolioState, SectorConfig,
};

#[test]
fn test_percentage_calculations() {
    let config = ConfigRoot {
        adjusted_decision: AdjustedDecisionConfig::default(),
        kelly: KellyConfig::default(),
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 1000.0,
            reserve_cash: 200.0,
            upcoming_expense: 100.0,
            max_daily_buy_total: 500.0,
        },
        api: Default::default(),
        fx: Default::default(),
        market: Default::default(),
        risk: Default::default(),
        regime: Default::default(),
        reconciliation: Default::default(),
        daily_plan: Default::default(),
        market_refresh: Default::default(),
        assets: vec![AssetConfig {
            asset_id: "a1".to_string(),
            fund_code: "123".to_string(),
            fund_name: "N1".to_string(),
            sector: "S1".to_string(),
            currency: "CNY".to_string(),
            valuation_method: "nav".to_string(),
            enabled: true,
            reference_index_name: None,
            reference_index_symbol: None,
            market_data_provider: None,
            reference_index_currency: None,
            proxy_fx_pair: None,
            use_fx_adjustment: Some(false),
            reference_instrument_id: None,
            reference_instrument_symbol: None,
        }],
        sectors: vec![SectorConfig {
            sector_id: "s1".to_string(),
            name: "S1".to_string(),
            asset_class: "equity".to_string(),
            target_weight: 1.0,
            priority: 1,
            enabled: true,
        }],
        storage: Default::default(),
    };

    let state = PortfolioState {
        cash: 1000.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "a1".to_string(),
            fund_code: "123".to_string(),
            units: 10.0,
            units_estimated: false,
            cost_basis: 50.0,
            latest_nav: Some(10.0),
            latest_nav_date: Some("2024-05-20".to_string()),
            latest_nav_source: None,
            latest_nav_status: None,
            last_market_value: 100.0,
        }],
    };

    let summary = calculate_portfolio_summary(&config, &state);

    // Total assets: 1000 cash + 100 market value = 1100
    assert_eq!(summary.total_asset_value, 1100.0);

    // Equity value: 100
    // Target equity: 1000
    // Completion: 100 / 1000 = 10%
    let completion = summary.equity_value / summary.target_equity_value;
    assert_eq!(completion, 0.1);

    // Equity/Total: 100 / 1100 = 9.09%
    let equity_to_total = summary.equity_value / summary.total_asset_value;
    assert!((equity_to_total - 0.0909).abs() < 0.0001);

    // Available cash: 1000 - 200 - 100 = 700
    assert_eq!(summary.available_cash, 700.0);
    let av_cash_to_total = summary.available_cash / summary.total_asset_value;
    assert!((av_cash_to_total - 0.6363).abs() < 0.0001);

    let date = "2024-05-25".to_string();
    let decision = generate_buy_suggestions(&config, &state, date);

    // Suggested buy should be maxed by max_daily_buy_total = 500
    assert_eq!(decision.suggested_total_buy, 500.0);
    let buy_to_max = decision.suggested_total_buy / decision.max_daily_buy_total;
    assert_eq!(buy_to_max, 1.0);
}

#[test]
fn test_sector_gap_ratio() {
    let config = ConfigRoot {
        adjusted_decision: AdjustedDecisionConfig::default(),
        kelly: KellyConfig::default(),
        portfolio: PortfolioConfig {
            name: "test".to_string(),
            base_currency: "CNY".to_string(),
            target_equity_value: 1000.0,
            reserve_cash: 0.0,
            upcoming_expense: 0.0,
            max_daily_buy_total: 0.0,
        },
        api: Default::default(),
        fx: Default::default(),
        market: Default::default(),
        risk: Default::default(),
        regime: Default::default(),
        reconciliation: Default::default(),
        daily_plan: Default::default(),
        market_refresh: Default::default(),
        assets: vec![],
        sectors: vec![SectorConfig {
            sector_id: "s1".to_string(),
            name: "S1".to_string(),
            asset_class: "equity".to_string(),
            target_weight: 0.5,
            priority: 1,
            enabled: true,
        }],
        storage: Default::default(),
    };

    let state = PortfolioState {
        cash: 1000.0,
        asset_holdings: vec![],
    };

    let summary = calculate_portfolio_summary(&config, &state);
    let s1 = &summary.sector_summaries[0];

    // target_value = 1000 * 0.5 = 500
    // current_value = 0
    // gap_value = 500
    // gap_ratio = 500 / 500 = 1.0 (100%)
    assert_eq!(s1.gap_ratio, 1.0);
}
