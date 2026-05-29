use super::web_reports::{
    ReportQuery, build_daily_report, build_monthly_report, build_weekly_report,
};
use crate::models::InvestmentReport;
use crate::web::{AppState, badge_status, layout};
use axum::{
    extract::{Query, State},
    response::Html,
};
use std::sync::Arc;

fn render_report_html(
    report: &InvestmentReport,
    portfolio_id: &str,
    _report_type_label: &str,
) -> String {
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
            html.push_str(&format!(
                "<p><strong>总结:</strong> {}</p>",
                section.summary
            ));
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

pub async fn html_reports_index_handler(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<ReportQuery>,
) -> Html<String> {
    let portfolio_id = params
        .portfolio_id
        .clone()
        .unwrap_or_else(|| "default".to_string());

    let html = format!(
        "<div class='card mb-4'>
            <h2>复盘报告</h2>
            <p>请选择要查看的报告类型:</p>
            <ul>
                <li><a href='/reports/daily?portfolio_id={}'>每日复盘报告</a></li>
                <li><a href='/reports/weekly?portfolio_id={}'>每周复盘报告</a></li>
                <li><a href='/reports/monthly?portfolio_id={}'>月度复盘报告</a></li>
            </ul>
        </div>",
        portfolio_id, portfolio_id, portfolio_id
    );

    layout("复盘报告", html)
}

pub async fn html_reports_daily_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ReportQuery>,
) -> Html<String> {
    let portfolio_id = params
        .portfolio_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let report = build_daily_report(&state, &params).await;
    let html = render_report_html(&report, &portfolio_id, "每日复盘");
    layout(&report.title, html)
}

pub async fn html_reports_weekly_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ReportQuery>,
) -> Html<String> {
    let portfolio_id = params
        .portfolio_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let report = build_weekly_report(&state, &params).await;
    let html = render_report_html(&report, &portfolio_id, "每周复盘");
    layout(&report.title, html)
}

pub async fn html_reports_monthly_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ReportQuery>,
) -> Html<String> {
    let portfolio_id = params
        .portfolio_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let report = build_monthly_report(&state, &params).await;
    let html = render_report_html(&report, &portfolio_id, "月度复盘");
    layout(&report.title, html)
}
