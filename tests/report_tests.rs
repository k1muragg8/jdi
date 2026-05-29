use pendulum_kelly_cli::engine::report_summary::generate_report_summary;
use pendulum_kelly_cli::models::{AssetHolding, PortfolioState, Transaction};

#[test]
fn test_generate_report_summary_no_transactions() {
    let state = PortfolioState::default();
    let transactions = vec![];
    let summary = generate_report_summary(
        "default",
        "json",
        "2026-05-01",
        "2026-05-31",
        &transactions,
        &state,
    );

    assert_eq!(summary.tx_summary.count, 0);
    assert_eq!(summary.cash_flow.net_flow, 0.0);
    assert_eq!(summary.holding_changes.len(), 0);
}

#[test]
fn test_generate_report_summary_with_transactions() {
    let mut state = PortfolioState::default();
    let asset = AssetHolding {
        asset_id: "test_asset".to_string(),
        fund_code: "123456".to_string(),
        units: 10.0,
        units_estimated: false,
        cost_basis: 100.0,
        latest_nav: Some(10.0),
        latest_nav_date: None,
        latest_nav_source: None,
        latest_nav_status: None,
        last_market_value: 100.0,
    };
    state.asset_holdings.push(asset);
    state.cash = 1000.0;

    let tx1 = Transaction {
        id: "tx1".to_string(),
        date: "2026-05-10".to_string(),
        transaction_type: "买入".to_string(),
        asset_id: Some("test_asset".to_string()),
        amount: 200.0,
        units: Some(20.0),
        price: Some(10.0),
        fee: 5.0,
        currency: "CNY".to_string(),
        note: "".to_string(),
        source: "".to_string(),
        raw_description: "".to_string(),
    };

    let tx2 = Transaction {
        id: "tx2".to_string(),
        date: "2026-05-15".to_string(),
        transaction_type: "现金转入".to_string(),
        asset_id: None,
        amount: 500.0,
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: "".to_string(),
        source: "".to_string(),
        raw_description: "".to_string(),
    };

    let transactions = vec![tx1, tx2];
    let summary = generate_report_summary(
        "default",
        "json",
        "2026-05-01",
        "2026-05-31",
        &transactions,
        &state,
    );

    assert_eq!(summary.tx_summary.count, 2);
    assert_eq!(summary.tx_summary.total_amount, 700.0);
    assert_eq!(summary.tx_summary.buy_amount, 200.0);
    assert_eq!(summary.tx_summary.fee_amount, 5.0);
    assert_eq!(summary.cash_flow.cash_in, 500.0);
    assert_eq!(summary.cash_flow.net_flow, 500.0);

    assert_eq!(summary.holding_changes.len(), 1);
    assert_eq!(summary.holding_changes[0].asset_id, "test_asset");
    assert_eq!(summary.holding_changes[0].units_changed, 20.0);
    assert_eq!(summary.holding_changes[0].value_changed, 200.0);
}
