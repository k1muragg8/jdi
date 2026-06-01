use pendulum_kelly_cli::engine::calculate_dca_lifecycle;
use pendulum_kelly_cli::models::{
    AssetConfig, AssetHolding, ConfigRoot, DcaFrequency, DcaPlan, DcaSettlement,
    DcaSettlementStatus, NavCache, PortfolioState,
};

#[test]
fn test_dca_lifecycle_due_no_settlement() {
    let config = create_mock_config();
    let plans = vec![DcaPlan {
        plan_id: "p1".to_string(),
        asset_id: "a1".to_string(),
        fund_code: "f1".to_string(),
        fund_name: "n1".to_string(),
        amount: 100.0,
        currency: "CNY".to_string(),
        frequency: DcaFrequency::Daily,
        weekday: None,
        month_day: None,
        start_date: "2026-01-01".to_string(),
        end_date: None,
        enabled: true,
        priority: 0,
        note: None,
        created_at: "".to_string(),
        updated_at: "".to_string(),
    }];
    let settlements = vec![];
    let snapshots = vec![];
    let state = PortfolioState::default();
    let date = "2026-05-26";

    let nav_cache = NavCache::default();
    let summary = calculate_dca_lifecycle(&config, &plans, &settlements, &snapshots, &state, &nav_cache, date);
    let item = summary.items.iter().find(|i| i.asset_id == "a1").unwrap();

    assert_eq!(item.lifecycle_status, "今日应定投");
    assert_eq!(item.suggested_next_action, "录入定投确认");
}

#[test]
fn test_dca_lifecycle_confirmed_unapplied() {
    let config = create_mock_config();
    let plans = vec![];
    let settlements = vec![DcaSettlement {
        settlement_id: "s1".to_string(),
        plan_id: Some("p1".to_string()),
        asset_id: "a1".to_string(),
        fund_code: "f1".to_string(),
        fund_name: "n1".to_string(),
        scheduled_date: Some("2026-05-26".to_string()),
        deduction_date: "2026-05-26".to_string(),
        confirmation_date: "2026-05-27".to_string(),
        amount: 100.0,
        confirmed_nav: 1.0,
        confirmed_units: 100.0,
        fee: Some(0.0),
        currency: "CNY".to_string(),
        source: "manual".to_string(),
        status: DcaSettlementStatus::Confirmed,
        applied: false,
        note: None,
        created_at: "".to_string(),
    }];
    let snapshots = vec![];
    let state = PortfolioState::default();
    let date = "2026-05-26";

    let nav_cache = NavCache::default();
    let summary = calculate_dca_lifecycle(&config, &plans, &settlements, &snapshots, &state, &nav_cache, date);
    let item = summary.items.iter().find(|i| i.asset_id == "a1").unwrap();

    assert_eq!(item.lifecycle_status, "已确认未入账");
    assert_eq!(item.suggested_next_action, "执行定投确认入账");
}

#[test]
fn test_dca_lifecycle_applied_no_snapshot() {
    let config = create_mock_config();
    let plans = vec![];
    let settlements = vec![DcaSettlement {
        settlement_id: "s1".to_string(),
        plan_id: Some("p1".to_string()),
        asset_id: "a1".to_string(),
        fund_code: "f1".to_string(),
        fund_name: "n1".to_string(),
        scheduled_date: Some("2026-05-26".to_string()),
        deduction_date: "2026-05-26".to_string(),
        confirmation_date: "2026-05-27".to_string(),
        amount: 100.0,
        confirmed_nav: 1.0,
        confirmed_units: 100.0,
        fee: Some(0.0),
        currency: "CNY".to_string(),
        source: "manual".to_string(),
        status: DcaSettlementStatus::Confirmed,
        applied: true,
        note: None,
        created_at: "".to_string(),
    }];
    let snapshots = vec![];
    let mut state = PortfolioState::default();
    state.asset_holdings.push(AssetHolding {
        asset_id: "a1".to_string(),
        fund_code: "f1".to_string(),
        units: 100.0,
        units_estimated: false,
        cost_basis: 1.0,
        last_market_value: 100.0,
        latest_nav: Some(1.0),
        latest_nav_date: Some("2026-05-27".to_string()),
        latest_nav_source: None,
        latest_nav_status: None,
    });
    let date = "2026-05-26";

    let nav_cache = NavCache::default();
    let summary = calculate_dca_lifecycle(&config, &plans, &settlements, &snapshots, &state, &nav_cache, date);
    let item = summary.items.iter().find(|i| i.asset_id == "a1").unwrap();

    assert_eq!(item.lifecycle_status, "等待支付宝快照");
    assert_eq!(item.suggested_next_action, "录入支付宝快照");
}

fn create_mock_config() -> ConfigRoot {
    let mut config = ConfigRoot::default();
    config.assets.push(AssetConfig {
        asset_id: "a1".to_string(),
        fund_code: "f1".to_string(),
        fund_name: "n1".to_string(),
        sector: "s1".to_string(),
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
    });
    config
}
