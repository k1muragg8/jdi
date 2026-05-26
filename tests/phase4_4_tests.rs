use pendulum_kelly_cli::engine::dca_settlement::calculate_settlement_impact;
use pendulum_kelly_cli::models::{
    AssetHolding, ConfigRoot, DcaSettlement, DcaSettlementStatus, PortfolioState,
};

#[test]
fn test_settlement_impact_new_holding() {
    let config = ConfigRoot::default();
    let state = PortfolioState {
        cash: 1000.0,
        asset_holdings: vec![],
    };

    let settlement = DcaSettlement {
        settlement_id: "s1".to_string(),
        plan_id: None,
        asset_id: "a1".to_string(),
        fund_code: "001".to_string(),
        fund_name: "F1".to_string(),
        scheduled_date: None,
        deduction_date: "2026-05-26".to_string(),
        confirmation_date: "2026-05-27".to_string(),
        amount: 200.0,
        confirmed_nav: 1.0,
        confirmed_units: 200.0,
        fee: Some(0.0),
        currency: "CNY".to_string(),
        source: "alipay".to_string(),
        status: DcaSettlementStatus::Confirmed,
        applied: false,
        note: None,
        created_at: "2026-05-26 10:00:00".to_string(),
    };

    let impact = calculate_settlement_impact(&config, &state, &settlement);

    assert_eq!(impact.old_units, 0.0);
    assert_eq!(impact.new_units, 200.0);
    assert_eq!(impact.new_cost_basis, 1.0);
    assert_eq!(impact.estimated_new_market_value, 200.0);
    assert!(impact.warnings.iter().any(|w| w.contains("初始化新持仓")));
}

#[test]
fn test_settlement_impact_existing_holding() {
    let config = ConfigRoot::default();
    let state = PortfolioState {
        cash: 1000.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "a1".to_string(),
            fund_code: "001".to_string(),
            units: 100.0,
            units_estimated: false,
            cost_basis: 1.0,
            last_market_value: 100.0,
            latest_nav: Some(1.0),
            latest_nav_date: Some("2026-05-25".to_string()),
            latest_nav_source: None,
            latest_nav_status: None,
        }],
    };

    let settlement = DcaSettlement {
        settlement_id: "s1".to_string(),
        plan_id: None,
        asset_id: "a1".to_string(),
        fund_code: "001".to_string(),
        fund_name: "F1".to_string(),
        scheduled_date: None,
        deduction_date: "2026-05-26".to_string(),
        confirmation_date: "2026-05-27".to_string(),
        amount: 220.0, // Buying 200 units at 1.1 each
        confirmed_nav: 1.1,
        confirmed_units: 200.0,
        fee: Some(0.0),
        currency: "CNY".to_string(),
        source: "alipay".to_string(),
        status: DcaSettlementStatus::Confirmed,
        applied: false,
        note: None,
        created_at: "2026-05-26 10:00:00".to_string(),
    };

    let impact = calculate_settlement_impact(&config, &state, &settlement);

    assert_eq!(impact.old_units, 100.0);
    assert_eq!(impact.new_units, 300.0);
    // (100 * 1.0 + 220) / 300 = 320 / 300 = 1.06666...
    assert!((impact.new_cost_basis - 1.066666).abs() < 0.0001);
    assert_eq!(impact.estimated_new_market_value, 330.0); // 300 * 1.1
}
