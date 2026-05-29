use pendulum_kelly_cli::engine::kelly::{KellyContext, calculate_single_asset_kelly};
use pendulum_kelly_cli::models::{ConfigRoot, GlobalRiskOverlay, KellyConfig, MarketRegimeResult};

#[test]
fn test_kelly_logic_neutral() {
    let mut config = ConfigRoot {
        kelly: KellyConfig::default(),
        ..ConfigRoot::default()
    };
    config.kelly.enabled = true;

    let risk_overlay = GlobalRiskOverlay {
        risk_score: 10.0,
        risk_label: "低风险".to_string(),
        factor_results: vec![],
        warnings: vec![],
        explanation: "OK".to_string(),
    };

    let res = calculate_single_asset_kelly(KellyContext {
        config: &config,
        asset_id: "test_asset".to_string(),
        fund_code: "000001".to_string(),
        fund_name: "Test Fund".to_string(),
        sector: "Tech".to_string(),
        base_suggested_buy: 1000.0,
        risk_overlay: &risk_overlay,
        regime: None, // No regime data
    });

    assert!(res.kelly_multiplier > 0.0);
    assert_eq!(res.status, "数据不足");
}

#[test]
fn test_kelly_logic_extreme_risk() {
    let mut config = ConfigRoot {
        kelly: KellyConfig::default(),
        ..ConfigRoot::default()
    };
    config.kelly.extreme_risk_multiplier = 0.0;

    let risk_overlay = GlobalRiskOverlay {
        risk_score: 90.0,
        risk_label: "极高风险".to_string(),
        factor_results: vec![],
        warnings: vec![],
        explanation: "Risk high".to_string(),
    };

    let res = calculate_single_asset_kelly(KellyContext {
        config: &config,
        asset_id: "test_asset".to_string(),
        fund_code: "000001".to_string(),
        fund_name: "Test Fund".to_string(),
        sector: "Tech".to_string(),
        base_suggested_buy: 1000.0,
        risk_overlay: &risk_overlay,
        regime: None,
    });

    assert_eq!(res.kelly_multiplier, 0.0);
    assert_eq!(res.capped_preview_buy_amount, 0.0);
    assert_eq!(res.status, "风险过高");
}

#[test]
fn test_kelly_logic_overheated_market() {
    let mut config = ConfigRoot {
        kelly: KellyConfig::default(),
        ..ConfigRoot::default()
    };
    config.kelly.overheated_market_multiplier = 0.2;

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

    let res = calculate_single_asset_kelly(KellyContext {
        config: &config,
        asset_id: "test_asset".to_string(),
        fund_code: "000001".to_string(),
        fund_name: "Test Fund".to_string(),
        sector: "Tech".to_string(),
        base_suggested_buy: 1000.0,
        risk_overlay: &risk_overlay,
        regime: Some(&regime),
    });

    assert!(res.kelly_multiplier <= 0.2 * 1.5); // 0.2 base * (1 + boost)
    assert_eq!(res.status, "市场过热");
}
