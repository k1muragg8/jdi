//! Market watchlist handlers.

use crate::web::response::AdminQuery;
use crate::web::services::market_service;
use crate::web::state::AppState;
use crate::web::view_models::market::build_market_vm;
use crate::web::views::{layout, layout_with_msg, market};
use axum::extract::{Query, State};
use axum::response::Html;
use std::sync::Arc;

pub async fn instruments_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminQuery>,
) -> Html<String> {
    match market_service::load_market_page(&state).await {
        Ok(page) => match build_market_vm(&state, page, query.filter.as_deref()).await {
            Ok(vm) => layout_with_msg("市场", market::render(&vm), query.success, query.error),
            Err(e) => layout(
                "市场",
                format!(
                    "<div class='message-banner message-error'>页面渲染失败: {}</div>",
                    e
                ),
            ),
        },
        Err(e) => layout(
            "市场",
            format!(
                "<div class='message-banner message-error'>数据加载失败: {}</div>",
                e
            ),
        ),
    }
}
