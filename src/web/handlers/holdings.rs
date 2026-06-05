//! Holdings (持仓) page handlers.

use crate::web::response::AdminQuery;
use crate::web::services::holdings_service;
use crate::web::state::AppState;
use crate::web::view_models::holdings::build_holdings_vm;
use crate::web::views::{holdings, layout, layout_with_msg};
use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use std::sync::Arc;

pub async fn api_holdings_bootstrap_alipay_handler(State(state): State<Arc<AppState>>) -> Redirect {
    match holdings_service::bootstrap_holdings_from_alipay(&state).await {
        Ok(msg) => Redirect::to(&format!("/holdings?success={}", msg)),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

pub async fn holdings_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminQuery>,
) -> Html<String> {
    match holdings_service::load_holdings_page(&state).await {
        Ok(data) => {
            let vm = build_holdings_vm(data, query.filter.as_deref());
            layout_with_msg("持仓", holdings::render(&vm), query.success, query.error)
        }
        Err(e) => layout(
            "持仓",
            format!(
                "<div class='message-banner message-error'>数据加载失败: {}</div>",
                e
            ),
        ),
    }
}
