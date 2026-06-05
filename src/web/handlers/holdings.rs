//! Holdings (持仓) page handlers.

use crate::web::product::snapshots_to_candidates;
use crate::web::response::AdminQuery;
use crate::web::services::holdings_service;
use crate::web::state::AppState;
use crate::web::view_models::holdings::build_holdings_vm;
use crate::web::views::{holdings, layout, layout_with_msg};
use crate::{engine, models};
use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use std::sync::Arc;

pub async fn api_holdings_bootstrap_alipay_handler(State(state): State<Arc<AppState>>) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut config = state.repo.load_config(ctx).await?;
        let portfolio_state = state.repo.load_state(ctx).await?;
        let snapshots = state.repo.load_alipay_snapshots(ctx).await?;
        let mut latest: std::collections::HashMap<String, models::AlipaySnapshot> =
            std::collections::HashMap::new();
        for s in &snapshots {
            let key = if s.asset_id.is_empty() {
                format!("unmatched_{}", s.fund_code)
            } else {
                s.asset_id.clone()
            };
            let e = latest.entry(key).or_insert_with(|| s.clone());
            if s.snapshot_date >= e.snapshot_date {
                *e = s.clone();
            }
        }
        let candidates = snapshots_to_candidates(&latest);
        if candidates.is_empty() {
            return Err(anyhow::anyhow!("无支付宝快照可初始化"));
        }
        let (created, _, _) =
            engine::alipay_holding::bootstrap_assets_from_holdings(&mut config, &candidates);
        state.repo.save_config(ctx, &config).await?;
        let nav_cache = state.repo.load_nav_cache(ctx).await.unwrap_or_default();
        let nav_map: std::collections::HashMap<String, models::FundNav> = nav_cache
            .entries
            .iter()
            .map(|e| {
                (
                    e.fund_code.clone(),
                    models::FundNav {
                        fund_code: e.fund_code.clone(),
                        nav: e.nav,
                        accumulated_nav: e.accumulated_nav,
                        nav_date: e.nav_date.clone(),
                        currency: e.currency.clone(),
                        source: e.source.clone(),
                        is_stale: false,
                        is_estimated: false,
                    },
                )
            })
            .collect();
        let preview = engine::alipay_holding::preview_bootstrap_local(
            &config,
            &portfolio_state,
            &candidates,
            &nav_map,
            true,
        );
        let (new_state, n) =
            engine::alipay_holding::apply_bootstrap_local(portfolio_state, &preview);
        state.repo.save_state(ctx, &new_state).await?;
        Ok::<String, anyhow::Error>(format!(
            "已用支付宝快照初始化 {} 项持仓（新建资产 {} 个）",
            n, created
        ))
    }
    .await;

    match result {
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
            let vm = build_holdings_vm(data);
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
