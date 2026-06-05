//! POST actions: assets

use super::types::*;
use crate::web::handlers::forms::AssetIdForm;
use crate::web::services::holdings_service;
use crate::web::state::AppState;
use axum::extract::{Form, State};
use axum::response::Redirect;
use std::sync::Arc;

pub async fn admin_asset_set_fund_code_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetFundCodeForm>,
) -> Redirect {
    match holdings_service::set_asset_fund_code(&state, &form.asset_id, &form.fund_code).await {
        Ok(_) => Redirect::to("/holdings?success=基金代码设置成功"),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

pub async fn admin_asset_rename_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetRenameForm>,
) -> Redirect {
    match holdings_service::rename_asset(&state, &form.asset_id, &form.fund_name).await {
        Ok(_) => Redirect::to("/holdings?success=资产更名成功"),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

pub async fn admin_asset_set_sector_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetSectorForm>,
) -> Redirect {
    match holdings_service::set_asset_sector(&state, &form.asset_id, &form.sector).await {
        Ok(_) => Redirect::to("/holdings?success=资产板块设置成功"),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

pub async fn admin_asset_enable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetIdForm>,
) -> Redirect {
    set_asset_enabled(&state, &form.asset_id, true).await
}

pub async fn admin_asset_disable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetIdForm>,
) -> Redirect {
    set_asset_enabled(&state, &form.asset_id, false).await
}

pub async fn set_asset_enabled(state: &Arc<AppState>, asset_id: &str, enabled: bool) -> Redirect {
    match holdings_service::set_asset_enabled(state, asset_id, enabled).await {
        Ok(_) => Redirect::to(&format!(
            "/holdings?success={}",
            if enabled {
                "资产已启用"
            } else {
                "资产已禁用"
            }
        )),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

pub async fn admin_asset_add_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetAddForm>,
) -> Redirect {
    match holdings_service::add_asset(
        &state,
        &form.fund_name,
        &form.fund_code,
        form.sector.as_deref(),
    )
    .await
    {
        Ok(_) => Redirect::to("/holdings?success=资产已添加"),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

pub async fn admin_asset_remove_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AssetIdForm>,
) -> Redirect {
    match holdings_service::remove_asset(&state, &form.asset_id).await {
        Ok(msg) => Redirect::to(&format!("/holdings?success={}", msg)),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
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
    match holdings_service::restore_asset(&state, &form.asset_id).await {
        Ok(_) => Redirect::to("/holdings?success=资产已恢复"),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}
