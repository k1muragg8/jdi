use pendulum_kelly_cli::engine::render_report_to_markdown;
use pendulum_kelly_cli::models::{InvestmentReport, ReportPeriod, ReportSection};

#[test]
fn test_render_report_to_markdown() {
    let report = InvestmentReport {
        report_id: "test".to_string(),
        report_type: ReportPeriod::Daily,
        start_date: "2026-05-26".to_string(),
        end_date: "2026-05-26".to_string(),
        generated_at: "2026-05-26 12:00:00".to_string(),
        title: "Test Report".to_string(),
        portfolio_summary: None,
        dca_summary: None,
        reconciliation_summary: None,
        risk_summary: None,
        sections: vec![ReportSection {
            title: "Section 1".to_string(),
            status: "OK".to_string(),
            summary: "Everything is fine.".to_string(),
            details: vec!["Detail A".to_string()],
            warnings: vec![],
            suggested_actions: vec![],
        }],
        warnings: vec![],
        pending_actions: vec!["Do something".to_string()],
    };

    let md = render_report_to_markdown(&report);
    assert!(md.contains("# Test Report"));
    assert!(md.contains("## Section 1"));
    assert!(md.contains("Everything is fine."));
    assert!(md.contains("Detail A"));
    assert!(md.contains("[ ] Do something"));
}
