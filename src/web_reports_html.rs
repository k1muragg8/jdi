use crate::models::{InvestmentReport, ReportPeriod};
use crate::web::{badge_status, layout, AppState};
use axum::{
    extract::{Query, State},
    response::Html,
};
use std::sync::Arc;
use super::web_reports::{ReportQuery, api_reports_daily_handler, api_reports_weekly_handler, api_reports_monthly_handler};
use axum::response::IntoResponse;

fn render_report_html(report: &InvestmentReport, portfolio_id: &str, report_type_label: &str) -> String {
    let mut html = String::new();

    html.push_str(&format!(
        "<div class='card mb-4'>
            <h2>{}</h2>
            <div style='margin-bottom: 1rem;'>
                <strong>报告期间:</strong> {} 至 {} <br/>
                <strong>生成时间:</strong> {} <br/>
                <strong>投资组合:</strong> {}
            </div>",
        report.title, report.start_date, report.end_date, report.generated_at, portfolio_id
    ));

    if !report.warnings.is_empty() {
        html.push_str("<div class='alert alert-warning'><h4>全局警告</h4><ul>");
        for w in &report.warnings {
            html.push_str(&format!("<li>{}</li>", w));
        }
        html.push_str("</ul></div>");
    }

    html.push_str("</div>"); // Close card

    for section in &report.sections {
        html.push_str(&format!(
            "<div class='card mb-4'>
                <div style='display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #e0e0e0; padding-bottom: 0.5rem; margin-bottom: 1rem;'>
                    <h3 style='margin: 0;'>{}</h3>
                    {}
                </div>",
            section.title,
            badge_status(&section.status)
        ));

        if !section.summary.is_empty() {
            html.push_str(&format!("<p><strong>总结:</strong> {}</p>", section.summary));
        }

        if !section.details.is_empty() {
            html.push_str("<ul>");
            for detail in &section.details {
                html.push_str(&format!("<li>{}</li>", detail));
            }
            html.push_str("</ul>");
        }

        if !section.warnings.is_empty() {
            html.push_str("<div class='alert alert-warning'><strong>警告:</strong><ul>");
            for warning in &section.warnings {
                html.push_str(&format!("<li>{}</li>", warning));
            }
            html.push_str("</ul></div>");
        }

        if !section.suggested_actions.is_empty() {
            html.push_str("<div class='alert alert-info'><strong>建议操作:</strong><ul>");
            for action in &section.suggested_actions {
                html.push_str(&format!("<li>{}</li>", action));
            }
            html.push_str("</ul></div>");
        }

        html.push_str("</div>");
    }

    html
}

async fn fetch_report_from_api<F, Fut>(
    state: State<Arc<AppState>>,
    query: Query<ReportQuery>,
    api_handler: F,
) -> Result<InvestmentReport, String>
where
    F: FnOnce(State<Arc<AppState>>, Query<ReportQuery>) -> Fut,
    Fut: std::future::Future<Output = axum::response::Response>,
{
    // This is a bit hacky to reuse the API handler logic directly,
    // but since we want to avoid duplicating logic, we can call the API handler,
    // get the response, and deserialize the JSON.
    // However, axum IntoResponse doesn't easily let us extract the JSON value back out in a typed way
    // without hyper body reading.
    // Instead, we should refactor `api_reports_*` to call a common function that returns `Result<InvestmentReport>`.
    
    // Actually, I can just call the shared engine logic directly here!
    Err("To be refactored".to_string())
}
