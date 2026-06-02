use pendulum_kelly_cli::models::{
    CacheStatusRegistry, DashboardSummary, DcaLifecycleSummary, DecisionExplanation,
    GlobalRiskOverlay, PortfolioSummary, RiskAdjustmentExplanation,
};

#[test]
fn test_dashboard_summary_serialization() {
    let summary = DashboardSummary {
        portfolio: PortfolioSummary::default(),
        lifecycle: DcaLifecycleSummary::default(),
        cache_status: CacheStatusRegistry::default(),
        decision: DecisionExplanation {
            date: "2026-05-29".to_string(),
            portfolio_id: "default".to_string(),
            base_currency: "CNY".to_string(),
            available_cash: 1000.0,
            daily_budget: 1000.0,
            target_equity_value: 10000.0,
            current_equity_value: 5000.0,
            equity_gap: 5000.0,
            risk_summary: RiskAdjustmentExplanation {
                score: 20.0,
                label: "低风险".to_string(),
                multiplier: 1.0,
                factors: vec![],
            },
            asset_explanations: vec![],
            sector_explanations: vec![],
            warnings: vec![],
            global_caps: vec![],
        },
        risk_overlay: GlobalRiskOverlay::default(),
        operation_status: pendulum_kelly_cli::models::OperationStatus::default(),
        backend: "JSON".to_string(),
        portfolio_name: "My Portfolio".to_string(),
        date: "2026-05-29".to_string(),
        alipay_total_value: None,
        alipay_snapshot_date: None,
        unclassified_asset_count: 0,
        reconciliation_issue_count: 0,
        alipay_mismatch_count: 0,
    };

    let json = serde_json::to_string(&summary).unwrap();
    let deserialized: DashboardSummary = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.portfolio_name, "My Portfolio");
    assert_eq!(deserialized.backend, "JSON");
    assert_eq!(deserialized.decision.available_cash, 1000.0);
}

#[test]
fn test_dashboard_empty_portfolio() {
    let summary = DashboardSummary {
        portfolio: PortfolioSummary::default(),
        lifecycle: DcaLifecycleSummary::default(),
        cache_status: CacheStatusRegistry::default(),
        decision: DecisionExplanation {
            date: "2026-05-29".to_string(),
            portfolio_id: "default".to_string(),
            base_currency: "CNY".to_string(),
            available_cash: 0.0,
            daily_budget: 0.0,
            target_equity_value: 0.0,
            current_equity_value: 0.0,
            equity_gap: 0.0,
            risk_summary: RiskAdjustmentExplanation {
                score: 0.0,
                label: "N/A".to_string(),
                multiplier: 1.0,
                factors: vec![],
            },
            asset_explanations: vec![],
            sector_explanations: vec![],
            warnings: vec![],
            global_caps: vec![],
        },
        risk_overlay: GlobalRiskOverlay::default(),
        operation_status: pendulum_kelly_cli::models::OperationStatus::default(),
        backend: "JSON".to_string(),
        portfolio_name: "Empty".to_string(),
        date: "2026-05-29".to_string(),
        alipay_total_value: None,
        alipay_snapshot_date: None,
        unclassified_asset_count: 0,
        reconciliation_issue_count: 0,
        alipay_mismatch_count: 0,
    };

    let json = serde_json::to_string(&summary).unwrap();
    let deserialized: DashboardSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.portfolio.total_asset_value, 0.0);
}
