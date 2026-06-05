//! POST actions: assets

use super::types::*;
use crate::web::services::holdings_service;
use crate::web::state::AppState;
use axum::extract::{Form, State};
use axum::response::Redirect;
use std::sync::Arc;

pub async fn admin_asset_set_fund_code_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetFundCodeForm>,
) -> Redirect {
    let result =
        holdings_service::set_asset_fund_code(&state, &form.asset_id, &form.fund_code).await;
    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok(_) => Redirect::to(&format!(
            "/holdings?success=基金代码设置成功{}",
            filter_suffix
        )),
        Err(e) => Redirect::to(&format!("/holdings?error={}{}", e, filter_suffix)),
    }
}

pub async fn admin_asset_rename_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetRenameForm>,
) -> Redirect {
    let result = holdings_service::rename_asset(&state, &form.asset_id, &form.fund_name).await;
    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok(_) => Redirect::to(&format!("/holdings?success=资产更名成功{}", filter_suffix)),
        Err(e) => Redirect::to(&format!("/holdings?error={}{}", e, filter_suffix)),
    }
}

pub async fn admin_asset_set_sector_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetSectorForm>,
) -> Redirect {
    let result = holdings_service::set_asset_sector(&state, &form.asset_id, &form.sector).await;
    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok(_) => Redirect::to(&format!(
            "/holdings?success=资产板块设置成功{}",
            filter_suffix
        )),
        Err(e) => Redirect::to(&format!("/holdings?error={}{}", e, filter_suffix)),
    }
}

pub async fn admin_asset_enable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetIdForm>,
) -> Redirect {
    set_asset_enabled(&state, &form.asset_id, true, form.filter).await
}

pub async fn admin_asset_disable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetIdForm>,
) -> Redirect {
    set_asset_enabled(&state, &form.asset_id, false, form.filter).await
}

pub async fn set_asset_enabled(
    state: &Arc<AppState>,
    asset_id: &str,
    enabled: bool,
    filter: Option<String>,
) -> Redirect {
    let filter_suffix = filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match holdings_service::set_asset_enabled(state, asset_id, enabled).await {
        Ok(_) => Redirect::to(&format!(
            "/holdings?success={}{}",
            if enabled {
                "资产已启用"
            } else {
                "资产已禁用"
            },
            filter_suffix
        )),
        Err(e) => Redirect::to(&format!("/holdings?error={}{}", e, filter_suffix)),
    }
}

pub async fn admin_asset_add_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetAddForm>,
) -> Redirect {
    let result = holdings_service::add_asset(
        &state,
        &form.fund_name,
        &form.fund_code,
        form.sector.as_deref(),
    )
    .await;

    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();

    match result {
        Ok(_) => Redirect::to(&format!(
            "/holdings?success=资产已添加，请录入份额或刷新净值{}",
            filter_suffix
        )),
        Err(e) => Redirect::to(&format!("/holdings?error={}{}", e, filter_suffix)),
    }
}

pub async fn admin_asset_remove_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetIdForm>,
) -> Redirect {
    let result = holdings_service::remove_asset(&state, &form.asset_id).await;
    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok(msg) => Redirect::to(&format!("/holdings?success={}{}", msg, filter_suffix)),
        Err(e) => Redirect::to(&format!("/holdings?error={}{}", e, filter_suffix)),
    }
}

pub async fn api_assets_auto_classify_handler(State(state): State<Arc<AppState>>) -> Redirect {
    let ctx = &state.ctx;
    match crate::web::services::asset_enrichment_service::auto_classify_assets(&state, ctx).await {
        Ok(n) => Redirect::to(&format!(
            "/holdings?success=自动分类完成，更新 {} 个资产",
            n
        )),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

pub async fn admin_asset_restore_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetIdForm>,
) -> Redirect {
    let result = holdings_service::restore_asset(&state, &form.asset_id).await;
    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok(_) => Redirect::to(&format!("/holdings?success=资产已恢复{}", filter_suffix)),
        Err(e) => Redirect::to(&format!("/holdings?error={}{}", e, filter_suffix)),
    }
}
