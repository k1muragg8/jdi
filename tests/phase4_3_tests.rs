use pendulum_kelly_cli::engine::daily_plan::generate_daily_execution_plan;
use pendulum_kelly_cli::models::config::DailyPlanConfig;
use pendulum_kelly_cli::models::{
    AdjustedDecisionItem, AdjustedDecisionPreview, ConfigRoot, DcaFrequency, DcaPreviewItem,
    DcaPreviewSummary, KellyPortfolioPreview, KellyPreviewResult, PortfolioState,
    ReconciliationResult,
};

#[test]
fn test_daily_plan_combination() {
    let mut config = ConfigRoot::default();
    config.portfolio.max_daily_buy_total = 1000.0;
    config.daily_plan = DailyPlanConfig {
        pause_on_reconciliation_mismatch: true,
        ..Default::default()
    };

    let state = PortfolioState {
        cash: 5000.0,
        asset_holdings: vec![],
    };

    let date = "2026-05-26".to_string();

    let dca_preview = DcaPreviewSummary {
        date: date.clone(),
        total_due_amount: 200.0,
        items: vec![DcaPreviewItem {
            plan_id: "dca_1".to_string(),
            asset_id: "a1".to_string(),
            fund_code: "001".to_string(),
            fund_name: "F1".to_string(),
            amount: 200.0,
            currency: "CNY".to_string(),
            due_date: date.clone(),
            frequency: DcaFrequency::Daily,
            status: "今日应投".to_string(),
                latest_nav: None,
                nav_date: None,
            warnings: vec![],
        }],
        warnings: vec![],
    };

    let adjusted_decision = AdjustedDecisionPreview {
        available_cash: 5000.0,
        target_equity_value: 0.0,
        current_equity_value: 0.0,
        equity_gap: 0.0,
        max_daily_buy: 1000.0,
        base_total_buy: 300.0,
        adjusted_total_buy: 300.0,
        total_multiplier: 1.0,
        global_risk_label: "正常".to_string(),
        global_risk_score: 0.0,
        items: vec![AdjustedDecisionItem {
            sector: "S1".to_string(),
            asset_id: "a1".to_string(),
            fund_code: "001".to_string(),
            fund_name: "F1".to_string(),
            benchmark_symbol: None,
            volatility: None,
            base_suggested_buy: 300.0,
            regime_label: "中性".to_string(),
            pendulum_score: 0.0,
            regime_multiplier: 1.0,
            global_risk_label: "正常".to_string(),
            global_risk_score: 0.0,
            risk_multiplier: 1.0,
            kelly_multiplier: 1.0,
            data_quality_multiplier: 1.0,
            combined_multiplier: 1.0,
            adjusted_buy: 300.0,
            capped_adjusted_buy: 300.0,
            status: "正常".to_string(),
            warnings: vec![],
            explanation: "Test".to_string(),
        }],
        warnings: vec![],
    };

    let kelly_preview = KellyPortfolioPreview {
        base_total_buy: 300.0,
        preview_total_buy: 300.0,
        total_multiplier: 1.0,
        global_risk_score: 0.0,
        global_risk_label: "正常".to_string(),
        results: vec![KellyPreviewResult {
            asset_id: "a1".to_string(),
            fund_code: "001".to_string(),
            fund_name: "F1".to_string(),
            sector: "S1".to_string(),
            benchmark_symbol: None,
            base_suggested_buy: 300.0,
            pendulum_score: 0.0,
            market_regime_label: "中性".to_string(),
            global_risk_score: 0.0,
            global_risk_label: "正常".to_string(),
            volatility: 0.2,
            drawdown: 0.0,
            expected_edge: 0.05,
            estimated_win_probability: 0.5,
            payoff_ratio: 2.0,
            raw_kelly_fraction: 0.1,
            fractional_kelly_fraction: 0.025,
            kelly_multiplier: 1.0,
            preview_buy_amount: 300.0,
            capped_preview_buy_amount: 300.0,
            confidence: 1.0,
            status: "正常".to_string(),
            warnings: vec![],
            explanation: "Test".to_string(),
        }],
        warnings: vec![],
    };

    let reconciliation_results = vec![ReconciliationResult {
        snapshot_id: "snap_1".to_string(),
        asset_id: "a1".to_string(),
        fund_code: "001".to_string(),
        fund_name: "F1".to_string(),
        snapshot_date: date.clone(),
        system_market_value: 1000.0,
        alipay_market_value: 1000.0,
        market_value_diff: 0.0,
        market_value_diff_pct: 0.0,
        system_units: Some(1000.0),
        alipay_units: Some(1000.0),
        units_diff: Some(0.0),
        units_diff_pct: Some(0.0),
        system_cost_basis: Some(1.0),
        alipay_cost_basis: Some(1.0),
        cost_basis_diff: Some(0.0),
        cost_basis_diff_pct: Some(0.0),
        system_nav: Some(1.0),
        alipay_nav: Some(1.0),
        nav_diff: Some(0.0),
        nav_date_diff: Some(0),
        status: "一致".to_string(),
        warnings: vec![],
        suggested_action: "无".to_string(),
    }];

    let plan = generate_daily_execution_plan(
        &config,
        &state,
        date,
        &dca_preview,
        &adjusted_decision,
        &kelly_preview,
        &reconciliation_results,
    );

    assert_eq!(plan.total_recommended_amount, 500.0); // 200 (DCA) + 300 (Adjusted)
    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].asset_id, "a1");
    assert_eq!(plan.items[0].status, "今日应执行");
}

#[test]
fn test_daily_plan_reconciliation_gate() {
    let mut config = ConfigRoot::default();
    config.daily_plan.pause_on_reconciliation_mismatch = true;

    let state = PortfolioState {
        cash: 5000.0,
        asset_holdings: vec![],
    };

    let date = "2026-05-26".to_string();

    let dca_preview = DcaPreviewSummary {
        date: date.clone(),
        total_due_amount: 200.0,
        items: vec![DcaPreviewItem {
            plan_id: "dca_1".to_string(),
            asset_id: "a1".to_string(),
            fund_code: "001".to_string(),
            fund_name: "F1".to_string(),
            amount: 200.0,
            currency: "CNY".to_string(),
            due_date: date.clone(),
            frequency: DcaFrequency::Daily,
            status: "今日应投".to_string(),
                latest_nav: None,
                nav_date: None,
            warnings: vec![],
        }],
        warnings: vec![],
    };

    let adjusted_decision = AdjustedDecisionPreview {
        available_cash: 5000.0,
        target_equity_value: 0.0,
        current_equity_value: 0.0,
        equity_gap: 0.0,
        max_daily_buy: 1000.0,
        base_total_buy: 0.0,
        adjusted_total_buy: 0.0,
        total_multiplier: 1.0,
        global_risk_label: "正常".to_string(),
        global_risk_score: 0.0,
        items: vec![],
        warnings: vec![],
    };

    let kelly_preview = KellyPortfolioPreview {
        base_total_buy: 0.0,
        preview_total_buy: 0.0,
        total_multiplier: 1.0,
        global_risk_score: 0.0,
        global_risk_label: "正常".to_string(),
        results: vec![],
        warnings: vec![],
    };

    let reconciliation_results = vec![ReconciliationResult {
        snapshot_id: "snap_1".to_string(),
        asset_id: "a1".to_string(),
        fund_code: "001".to_string(),
        fund_name: "F1".to_string(),
        snapshot_date: date.clone(),
        system_market_value: 1000.0,
        alipay_market_value: 1100.0,
        market_value_diff: 100.0,
        market_value_diff_pct: 0.1,
        system_units: Some(1000.0),
        alipay_units: Some(1100.0),
        units_diff: Some(100.0),
        units_diff_pct: Some(0.1),
        system_cost_basis: Some(1.0),
        alipay_cost_basis: Some(1.0),
        cost_basis_diff: Some(0.0),
        cost_basis_diff_pct: Some(0.0),
        system_nav: Some(1.0),
        alipay_nav: Some(1.1),
        nav_diff: Some(0.1),
        nav_date_diff: Some(0),
        status: "份额不一致".to_string(),
        warnings: vec!["Mismatch".to_string()],
        suggested_action: "Calibrate".to_string(),
    }];

    let plan = generate_daily_execution_plan(
        &config,
        &state,
        date,
        &dca_preview,
        &adjusted_decision,
        &kelly_preview,
        &reconciliation_results,
    );

    assert_eq!(plan.total_recommended_amount, 0.0);
    assert_eq!(plan.items[0].status, "等待对账");
}

#[test]
fn test_daily_plan_max_daily_buy_cap() {
    let mut config = ConfigRoot::default();
    config.portfolio.max_daily_buy_total = 300.0;

    let state = PortfolioState {
        cash: 5000.0,
        asset_holdings: vec![],
    };

    let date = "2026-05-26".to_string();

    let dca_preview = DcaPreviewSummary {
        date: date.clone(),
        total_due_amount: 500.0,
        items: vec![DcaPreviewItem {
            plan_id: "dca_1".to_string(),
            asset_id: "a1".to_string(),
            fund_code: "001".to_string(),
            fund_name: "F1".to_string(),
            amount: 500.0,
            currency: "CNY".to_string(),
            due_date: date.clone(),
            frequency: DcaFrequency::Daily,
            status: "今日应投".to_string(),
                latest_nav: None,
                nav_date: None,
            warnings: vec![],
        }],
        warnings: vec![],
    };

    let adjusted_decision = AdjustedDecisionPreview {
        available_cash: 5000.0,
        target_equity_value: 0.0,
        current_equity_value: 0.0,
        equity_gap: 0.0,
        max_daily_buy: 300.0,
        base_total_buy: 0.0,
        adjusted_total_buy: 0.0,
        total_multiplier: 1.0,
        global_risk_label: "正常".to_string(),
        global_risk_score: 0.0,
        items: vec![],
        warnings: vec![],
    };

    let kelly_preview = KellyPortfolioPreview {
        base_total_buy: 0.0,
        preview_total_buy: 0.0,
        total_multiplier: 1.0,
        global_risk_score: 0.0,
        global_risk_label: "正常".to_string(),
        results: vec![],
        warnings: vec![],
    };
    let reconciliation_results = vec![];

    let plan = generate_daily_execution_plan(
        &config,
        &state,
        date,
        &dca_preview,
        &adjusted_decision,
        &kelly_preview,
        &reconciliation_results,
    );

    assert_eq!(plan.total_recommended_amount, 300.0);
}
