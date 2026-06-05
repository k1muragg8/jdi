//! API: nav_jobs

use crate::web::state::AppState;
use crate::{engine, models};
use axum::extract::State;
use axum::response::Json;
use chrono::Local;
use std::sync::Arc;

pub async fn api_nav_refresh_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::import::ImportResult> {
    let ctx = &state.ctx;
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let count = engine::refresh::refresh_fund_navs(state.repo.as_ref(), &ctx, &config).await?;

        let mut status = state.refresh_status.write().await;
        status.last_fund_refresh = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());

        Ok::<usize, anyhow::Error>(count)
    }
    .await;

    match result {
        Ok(count) => Json(models::import::ImportResult {
            success: count > 0,
            inserted: count,
            message: if count > 0 {
                format!("成功刷新 {} 个基金净值", count)
            } else {
                "未发现需要刷新的活跃基金。请先启用资产并配置基金代码。".to_string()
            },
            ..Default::default()
        }),
        Err(e) => Json(models::import::ImportResult {
            success: false,
            message: format!("基金净值刷新失败: {}", e),
            ..Default::default()
        }),
    }
}

pub async fn api_jobs_nav_refresh_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::StartJobResponse> {
    let ctx = &state.ctx;
    match state.repo.start_job(ctx, "fund_nav_refresh").await {
        Ok(job) => {
            let job_id = job.job_id.clone();
            if !matches!(job.status, models::WebJobStatus::Running) {
                let repo = state.repo.clone();
                let ctx2 = state.ctx.clone();
                let jid = job_id.clone();
                tokio::spawn(async move {
                    let _ = repo
                        .update_job_progress(&ctx2, &jid, 0, 1, Some("刷新基金净值".into()))
                        .await;
                    let res = async {
                        let c = repo.load_config(&ctx2).await?;
                        engine::refresh::refresh_fund_navs(repo.as_ref(), &ctx2, &c).await
                    }
                    .await;
                    match res {
                        Ok(cnt) => {
                            let rj = serde_json::json!({"refreshed": cnt});
                            let _ = repo
                                .finish_job(
                                    &ctx2,
                                    &jid,
                                    models::WebJobStatus::Success,
                                    Some(format!("刷新 {} 基金", cnt)),
                                    Some(rj),
                                )
                                .await;
                        }
                        Err(e) => {
                            let _ = repo.fail_job(&ctx2, &jid, &e.to_string()).await;
                        }
                    }
                });
            }
            Json(models::StartJobResponse {
                job_id: job_id.clone(),
                status: "started".into(),
                message: job.message,
            })
        }
        Err(e) => Json(models::StartJobResponse {
            job_id: "".into(),
            status: "error".into(),
            message: Some(e.to_string()),
        }),
    }
}

pub async fn api_jobs_auto_classify_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    match state.repo.start_job(ctx, "asset_auto_classify").await {
        Ok(job) => {
            let job_id = job.job_id.clone();
            if !matches!(job.status, models::WebJobStatus::Running) {
                let repo = state.repo.clone();
                let ctx2 = state.ctx.clone();
                let jid = job_id.clone();
                tokio::spawn(async move {
                    let _ = repo
                        .update_job_progress(&ctx2, &jid, 0, 1, Some("自动分类资产".into()))
                        .await;
                    let mut changed = 0usize;
                    if let Ok(mut cfg) = repo.load_config(&ctx2).await {
                        changed = engine::classify_unassigned_assets(&mut cfg.assets);
                        if changed > 0 {
                            let _ = repo.save_config(&ctx2, &cfg).await;
                        }
                    }
                    let rj = serde_json::json!({"changed": changed});
                    let _ = repo
                        .finish_job(
                            &ctx2,
                            &jid,
                            models::WebJobStatus::Success,
                            Some(format!("分类了 {} 个", changed)),
                            Some(rj),
                        )
                        .await;
                });
            }
            Json(serde_json::json!({"success": true, "job_id": job_id}))
        }
        Err(e) => Json(serde_json::json!({"success": false, "message": e.to_string()})),
    }
}
