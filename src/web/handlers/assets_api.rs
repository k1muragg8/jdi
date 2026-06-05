//! Asset/fund JSON APIs for editable admin panel.

use crate::engine::asset_enrichment;
use crate::models::OperationPolicy;
use crate::web::services::asset_enrichment_service;
use crate::web::state::AppState;
use axum::extract::State;
use axum::response::Json;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct FundLookupRequest {
    pub fund_code: String,
}

pub async fn api_fund_lookup_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<FundLookupRequest>,
) -> Json<asset_enrichment::FundLookupResult> {
    let ctx = &state.ctx;
    let config = state.repo.load_config(ctx).await.unwrap_or_default();
    Json(asset_enrichment_service::lookup_fund_code(
        &config,
        &body.fund_code,
    ))
}

#[derive(Deserialize)]
pub struct AssetIdBody {
    pub asset_id: String,
}

pub async fn api_asset_enrich_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AssetIdBody>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    match asset_enrichment_service::enrich_asset_by_id(&state, ctx, &body.asset_id).await {
        Ok((changed, warnings)) => Json(serde_json::json!({
            "success": true,
            "changed_fields": changed,
            "warnings": warnings,
            "message": format!("已更新 {} 个字段", changed.len())
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "message": e.to_string(),
            "warnings": []
        })),
    }
}

pub async fn api_assets_enrich_all_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    match asset_enrichment_service::enrich_all_assets(&state, ctx).await {
        Ok((n, warnings)) => Json(serde_json::json!({
            "success": true,
            "changed_count": n,
            "warnings": warnings,
            "message": format!("已补全 {} 个资产", n)
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "message": e.to_string()
        })),
    }
}

#[derive(Deserialize, Default)]
pub struct AssetUpdateBody {
    pub asset_id: String,
    pub fund_name: Option<String>,
    pub fund_code: Option<String>,
    pub sector: Option<String>,
    pub currency: Option<String>,
    pub enabled: Option<bool>,
    pub reference_index_symbol: Option<String>,
    pub reference_instrument_symbol: Option<String>,
    pub market_data_provider: Option<String>,
    pub valuation_method: Option<String>,
}

pub async fn api_asset_update_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AssetUpdateBody>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    let asset_id = body.asset_id.clone();
    let result = async {
        let mut config = state.repo.load_config(ctx).await?;
        let asset = config
            .assets
            .iter_mut()
            .find(|a| a.asset_id == asset_id)
            .ok_or_else(|| anyhow::anyhow!("资产未找到"))?;
        if let Some(v) = &body.fund_name {
            asset.fund_name = v.clone();
        }
        if let Some(v) = &body.fund_code {
            asset.fund_code = v.clone();
        }
        if let Some(v) = &body.sector {
            asset.sector = v.clone();
        }
        if let Some(v) = &body.currency {
            asset.currency = v.clone();
        }
        if let Some(v) = body.enabled {
            asset.enabled = v;
        }
        if let Some(v) = &body.reference_index_symbol {
            asset.reference_index_symbol = Some(v.clone());
        }
        if let Some(v) = &body.reference_instrument_symbol {
            asset.reference_instrument_symbol = Some(v.clone());
        }
        if let Some(v) = &body.market_data_provider {
            asset.market_data_provider = Some(v.clone());
        }
        if let Some(v) = &body.valuation_method {
            asset.valuation_method = v.clone();
        }
        state.repo.save_config(ctx, &config).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    match result {
        Ok(()) => {
            let config = state.repo.load_config(ctx).await.unwrap_or_default();
            let a = config
                .assets
                .iter()
                .find(|x| x.asset_id == asset_id)
                .cloned();
            Json(serde_json::json!({ "success": true, "asset": a }))
        }
        Err(e) => Json(serde_json::json!({ "success": false, "message": e.to_string() })),
    }
}

#[derive(Deserialize)]
pub struct TargetEquityBody {
    pub target_equity_weight: f64,
}

pub async fn api_policy_target_equity_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TargetEquityBody>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    if !(0.0..=1.0).contains(&body.target_equity_weight) {
        return Json(serde_json::json!({
            "success": false,
            "message": "目标权益仓位须在 0~1 之间"
        }));
    }
    let mut policy = state
        .repo
        .load_operation_policy(ctx)
        .await
        .unwrap_or_default();
    policy.target_equity_weight = body.target_equity_weight;
    match state.repo.save_operation_policy(ctx, &policy).await {
        Ok(_) => Json(serde_json::json!({ "success": true, "policy": policy })),
        Err(e) => Json(serde_json::json!({ "success": false, "message": e.to_string() })),
    }
}

#[derive(Deserialize)]
pub struct SectorWeightBody {
    pub sector_name: String,
    pub target_weight: f64,
}

pub async fn api_sector_target_weight_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SectorWeightBody>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    let result = async {
        let mut config = state.repo.load_config(ctx).await?;
        if let Some(s) = config
            .sectors
            .iter_mut()
            .find(|s| s.name == body.sector_name)
        {
            s.target_weight = body.target_weight;
        } else {
            anyhow::bail!("赛道未找到: {}", body.sector_name);
        }
        state.repo.save_config(ctx, &config).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;
    match result {
        Ok(_) => Json(serde_json::json!({ "success": true })),
        Err(e) => Json(serde_json::json!({ "success": false, "message": e.to_string() })),
    }
}

pub async fn api_get_operation_policy_handler(
    State(state): State<Arc<AppState>>,
) -> Json<OperationPolicy> {
    let ctx = &state.ctx;
    Json(
        state
            .repo
            .load_operation_policy(ctx)
            .await
            .unwrap_or_default(),
    )
}

pub async fn api_save_operation_policy_handler(
    State(state): State<Arc<AppState>>,
    Json(policy): Json<OperationPolicy>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    match state.repo.save_operation_policy(ctx, &policy).await {
        Ok(_) => Json(serde_json::json!({ "success": true })),
        Err(e) => Json(serde_json::json!({ "success": false, "message": e.to_string() })),
    }
}

#[derive(Deserialize)]
pub struct CashAdjustBody {
    pub amount: f64,
    pub note: Option<String>,
}

pub async fn api_cash_adjust_json_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CashAdjustBody>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    let tx_type = if body.amount >= 0.0 {
        "cash_in"
    } else {
        "cash_out"
    };
    let tx = crate::models::Transaction {
        id: uuid::Uuid::new_v4().to_string(),
        date: chrono::Local::now().format("%Y-%m-%d").to_string(),
        transaction_type: tx_type.to_string(),
        asset_id: None,
        amount: body.amount.abs(),
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: body
            .note
            .clone()
            .unwrap_or_else(|| "Web 现金调整".to_string()),
        source: "manual".to_string(),
        raw_description: "Cash adjust via API".to_string(),
    };
    let mut transactions = state.repo.load_transactions(ctx).await.unwrap_or_default();
    transactions.push(tx);
    match state.repo.save_transactions(ctx, &transactions).await {
        Ok(_) => {
            if let Ok(new_state) =
                crate::engine::holdings::rebuild_holdings_from_transactions(&transactions)
            {
                let _ = state.repo.save_state(ctx, &new_state).await;
            }
            Json(serde_json::json!({ "success": true, "message": "现金已调整" }))
        }
        Err(e) => Json(serde_json::json!({ "success": false, "message": e.to_string() })),
    }
}
