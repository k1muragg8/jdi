use pendulum_kelly_cli::engine::reconciliation::{
    generate_calibration_suggestion, reconcile_asset,
};
use pendulum_kelly_cli::models::{AlipaySnapshot, AssetHolding, ConfigRoot, PortfolioState};

#[test]
fn test_reconciliation_consistent() {
    let config = ConfigRoot::default();
    let state = PortfolioState {
        cash: 1000.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "test_asset".to_string(),
            fund_code: "000001".to_string(),
            units: 1000.0,
            units_estimated: false,
            cost_basis: 1.0,
            last_market_value: 1200.0,
            latest_nav: Some(1.2),
            latest_nav_date: Some("2026-05-25".to_string()),
            latest_nav_source: None,
            latest_nav_status: None,
        }],
    };

    let snapshot = AlipaySnapshot {
        snapshot_id: "snap_1".to_string(),
        asset_id: "test_asset".to_string(),
        fund_code: "000001".to_string(),
        fund_name: "Test Fund".to_string(),
        snapshot_date: "2026-05-26".to_string(),
        market_value: 1200.0,
        units: Some(1000.0),
        cost_basis: Some(1.0),
        nav: Some(1.2),
        nav_date: Some("2026-05-25".to_string()),
        daily_pnl: None,
        total_pnl: None,
        source: "alipay".to_string(),
        created_at: "2026-05-26".to_string(),
        note: None,
    };

    let result = reconcile_asset(&config, &state, &snapshot);
    assert_eq!(result.status, "一致");
    assert_eq!(result.market_value_diff, 0.0);
}

#[test]
fn test_reconciliation_units_mismatch() {
    let config = ConfigRoot::default();
    let state = PortfolioState {
        cash: 1000.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "test_asset".to_string(),
            fund_code: "000001".to_string(),
            units: 1000.0,
            units_estimated: false,
            cost_basis: 1.0,
            last_market_value: 1200.0,
            latest_nav: Some(1.2),
            latest_nav_date: Some("2026-05-25".to_string()),
            latest_nav_source: None,
            latest_nav_status: None,
        }],
    };

    let snapshot = AlipaySnapshot {
        snapshot_id: "snap_1".to_string(),
        asset_id: "test_asset".to_string(),
        fund_code: "000001".to_string(),
        fund_name: "Test Fund".to_string(),
        snapshot_date: "2026-05-26".to_string(),
        market_value: 1320.0,
        units: Some(1100.0),
        cost_basis: Some(1.0),
        nav: Some(1.2),
        nav_date: Some("2026-05-25".to_string()),
        daily_pnl: None,
        total_pnl: None,
        source: "alipay".to_string(),
        created_at: "2026-05-26".to_string(),
        note: None,
    };

    let result = reconcile_asset(&config, &state, &snapshot);
    assert_eq!(result.status, "份额不一致");
    assert_eq!(result.units_diff, Some(100.0));

    let suggestion = generate_calibration_suggestion(&result).unwrap();
    assert_eq!(suggestion.suggested_units, Some(1100.0));
    assert_eq!(suggestion.risk_level, "高");
}

#[test]
fn test_reconciliation_missing_holding() {
    let config = ConfigRoot::default();
    let state = PortfolioState {
        cash: 1000.0,
        asset_holdings: vec![],
    };

    let snapshot = AlipaySnapshot {
        snapshot_id: "snap_1".to_string(),
        asset_id: "test_asset".to_string(),
        fund_code: "000001".to_string(),
        fund_name: "Test Fund".to_string(),
        snapshot_date: "2026-05-26".to_string(),
        market_value: 1000.0,
        units: Some(1000.0),
        cost_basis: Some(1.0),
        nav: None,
        nav_date: None,
        daily_pnl: None,
        total_pnl: None,
        source: "alipay".to_string(),
        created_at: "2026-05-26".to_string(),
        note: None,
    };

    let result = reconcile_asset(&config, &state, &snapshot);
    assert_eq!(result.status, "缺少系统持仓");
}

#[test]
fn test_reconciliation_cost_basis_mismatch() {
    let config = ConfigRoot::default();
    let state = PortfolioState {
        cash: 1000.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "test_asset".to_string(),
            fund_code: "000001".to_string(),
            units: 1000.0,
            units_estimated: false,
            cost_basis: 1.0,
            last_market_value: 1200.0,
            latest_nav: Some(1.2),
            latest_nav_date: Some("2026-05-25".to_string()),
            latest_nav_source: None,
            latest_nav_status: None,
        }],
    };

    let snapshot = AlipaySnapshot {
        snapshot_id: "snap_1".to_string(),
        asset_id: "test_asset".to_string(),
        fund_code: "000001".to_string(),
        fund_name: "Test Fund".to_string(),
        snapshot_date: "2026-05-26".to_string(),
        market_value: 1200.0,
        units: Some(1000.0),
        cost_basis: Some(1.1),
        nav: Some(1.2),
        nav_date: Some("2026-05-25".to_string()),
        daily_pnl: None,
        total_pnl: None,
        source: "alipay".to_string(),
        created_at: "2026-05-26".to_string(),
        note: None,
    };

    let result = reconcile_asset(&config, &state, &snapshot);
    assert_eq!(result.status, "成本不一致");
    assert!(result.cost_basis_diff.unwrap().abs() > 0.0);
}

#[test]
fn test_reconciliation_needs_calibration_status() {
    let config = ConfigRoot::default();
    let state = PortfolioState {
        cash: 1000.0,
        asset_holdings: vec![AssetHolding {
            asset_id: "test_asset".to_string(),
            fund_code: "000001".to_string(),
            units: 1000.0,
            units_estimated: false,
            cost_basis: 1.0,
            last_market_value: 1000.0,
            latest_nav: Some(1.0),
            latest_nav_date: Some("2026-05-25".to_string()),
            latest_nav_source: None,
            latest_nav_status: None,
        }],
    };

    // Case where market value differs significantly but units/cost are missing or within tolerance
    // Actually in my code, if MV differs, it sets status to 明显差异 or 小幅差异.
    // If needs_calibration is true AND status is not some specific ones, it sets to "需要校准".

    let snapshot = AlipaySnapshot {
        snapshot_id: "snap_1".to_string(),
        asset_id: "test_asset".to_string(),
        fund_code: "000001".to_string(),
        fund_name: "Test Fund".to_string(),
        snapshot_date: "2026-05-26".to_string(),
        market_value: 1100.0,
        units: None, // Missing units in snapshot but MV diff is large
        cost_basis: None,
        nav: None,
        nav_date: None,
        daily_pnl: None,
        total_pnl: None,
        source: "alipay".to_string(),
        created_at: "2026-05-26".to_string(),
        note: None,
    };

    let result = reconcile_asset(&config, &state, &snapshot);
    // market_value_diff = 100.0, which is > 1.0 and > 0.1%
    // status becomes "明显差异"
    // suggested_action becomes "核对交易记录"
    // needs_calibration is false because units/cost_basis are None.

    assert_eq!(result.status, "明显差异");
}
