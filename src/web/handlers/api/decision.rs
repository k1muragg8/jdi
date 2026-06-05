//! API: decision

use crate::web::state::{AppState, BackgroundRefreshStatus};
use crate::{api, engine, models};
use anyhow::Result;
use axum::extract::{Multipart, State};
use axum::response::Json;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

pub async fn api_decision_explain_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::DecisionExplanation> {
    let ctx = &state.ctx;
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let date = Local::now().format("%Y-%m-%d").to_string();

        // Load caches for risk and regime
        let risk_cache = state.repo.load_risk_cache(&ctx).await?.unwrap_or_default();
        let regime_cache = state.repo.load_regime_cache(&ctx).await?;

        let mut regimes = std::collections::HashMap::new();
        for entry in &regime_cache.entries {
            for asset in &config.assets {
                let symbol_opt = asset
                    .reference_instrument_symbol
                    .clone()
                    .or(asset.reference_index_symbol.clone());
                if let Some(s) = symbol_opt {
                    if s == entry.symbol {
                        regimes.insert(asset.asset_id.clone(), entry.result.clone());
                    }
                }
            }
        }

        let explanation = engine::explanation::explain_decision(
            &config,
            &portfolio_state,
            ctx.portfolio_id.clone(),
            date,
            &risk_cache.overlay,
            &regimes,
        );
        Ok::<models::DecisionExplanation, anyhow::Error>(explanation)
    }
    .await;

    match result {
        Ok(e) => Json(e),
        Err(e) => {
            // Return an empty explanation with the error in warnings
            Json(models::DecisionExplanation {
                date: Local::now().format("%Y-%m-%d").to_string(),
                portfolio_id: "error".to_string(),
                base_currency: "CNY".to_string(),
                available_cash: 0.0,
                daily_budget: 0.0,
                target_equity_value: 0.0,
                current_equity_value: 0.0,
                equity_gap: 0.0,
                risk_summary: models::RiskAdjustmentExplanation {
                    score: 0.0,
                    label: "Error".to_string(),
                    multiplier: 0.0,
                    factors: vec![e.to_string()],
                },
                asset_explanations: vec![],
                sector_explanations: vec![],
                warnings: vec![format!("Failed to generate explanation: {}", e)],
                global_caps: vec![],
            })
        }
    }
}


pub async fn api_kelly_plan_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::KellyPortfolioPreview> {
    let ctx = &state.ctx;
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let decision = engine::generate_buy_suggestions(&config, &portfolio_state, date);

        // Load caches
        let risk_cache = state.repo.load_risk_cache(&ctx).await?;
        let regime_cache = state.repo.load_regime_cache(&ctx).await?.clone();

        let risk_overlay = if let Some(rc) = risk_cache {
            rc.overlay
        } else {
            models::GlobalRiskOverlay {
                risk_score: 0.0,
                risk_label: "未知".to_string(),
                factor_results: vec![],
                warnings: vec!["请运行 data refresh --risk".to_string()],
                explanation: "请运行 data refresh --risk".to_string(),
            }
        };

        let mut regimes = std::collections::HashMap::new();
        for entry in regime_cache.entries {
            for asset in &config.assets {
                let symbol_opt = asset
                    .reference_instrument_symbol
                    .clone()
                    .or(asset.reference_index_symbol.clone());
                if let Some(_s) = symbol_opt.filter(|s| *s == entry.symbol) {
                    regimes.insert(asset.asset_id.clone(), entry.result.clone());
                }
            }
        }

        let preview =
            engine::kelly::calculate_kelly_preview(&config, &decision, &risk_overlay, &regimes);

        Ok::<models::KellyPortfolioPreview, anyhow::Error>(preview)
    }
    .await;

    match result {
        Ok(p) => Json(p),
        Err(e) => Json(models::KellyPortfolioPreview {
            base_total_buy: 0.0,
            preview_total_buy: 0.0,
            total_multiplier: 0.0,
            global_risk_score: 0.0,
            global_risk_label: "错误".to_string(),
            results: vec![],
            warnings: vec![format!("加载 Kelly 数据失败: {}", e)],
        }),
    }
}
