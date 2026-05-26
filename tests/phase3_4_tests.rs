use pendulum_kelly_cli::engine::adjusted_decision::calculate_single_adjusted_item;
use pendulum_kelly_cli::models::{
    ConfigRoot, GlobalRiskOverlay, MarketRegimeResult, PortfolioState,
};

#[test]
fn test_adjusted_multiplier_logic() {
    let config = ConfigRoot::default();
    let state = PortfolioState {
        cash: 10000.0,
        asset_holdings: vec![],
    };
    let risk_overlay = GlobalRiskOverlay {
        risk_score: 10.0,
        risk_label: "低风险".to_string(),
        factor_results: vec![],
        warnings: vec![],
        explanation: "OK".to_string(),
    };

    // Case 1: Neutral regime
    let res = calculate_single_adjusted_item(
        &config,
        &state,
        "test_asset".to_string(),
        "000001".to_string(),
        "Test Fund".to_string(),
        "Tech".to_string(),
        1000.0,
        &risk_overlay,
        None,
    );
    assert_eq!(res.combined_multiplier, 0.7);
    assert_eq!(res.capped_adjusted_buy, 700.0);
}

#[test]
fn test_extreme_risk_adjusted_buy() {
    let config = ConfigRoot::default();
    let state = PortfolioState {
        cash: 0.0,
        asset_holdings: vec![],
    };
    let risk_overlay = GlobalRiskOverlay {
        risk_score: 90.0,
        risk_label: "极高风险".to_string(),
        factor_results: vec![],
        warnings: vec![],
        explanation: "Risk high".to_string(),
    };

    let res = calculate_single_adjusted_item(
        &config,
        &state,
        "test_asset".to_string(),
        "000001".to_string(),
        "Test Fund".to_string(),
        "Tech".to_string(),
        1000.0,
        &risk_overlay,
        None,
    );

    assert_eq!(res.combined_multiplier, 0.0);
    assert_eq!(res.capped_adjusted_buy, 0.0);
    assert_eq!(res.status, "风险过高");
}

#[test]
fn test_overheated_market_reduction() {
    let config = ConfigRoot::default();
    let state = PortfolioState {
        cash: 0.0,
        asset_holdings: vec![],
    };
    let risk_overlay = GlobalRiskOverlay {
        risk_score: 20.0,
        risk_label: "正常".to_string(),
        factor_results: vec![],
        warnings: vec![],
        explanation: "OK".to_string(),
    };

    let regime = MarketRegimeResult {
        symbol: "QQQ".to_string(),
        latest_price: 100.0,
        latest_date: "2023-01-01".to_string(),
        source: "mock".to_string(),
        windows: vec![],
        pendulum_score: 80.0,
        regime_label: "过热".to_string(),
        warning: None,
    };

    let res = calculate_single_adjusted_item(
        &config,
        &state,
        "test_asset".to_string(),
        "000001".to_string(),
        "Test Fund".to_string(),
        "Tech".to_string(),
        1000.0,
        &risk_overlay,
        Some(&regime),
    );

    assert_eq!(res.combined_multiplier, 0.0);
    assert_eq!(res.status, "市场过热");
}
