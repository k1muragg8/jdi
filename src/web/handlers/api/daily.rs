//! API: daily

use crate::web::state::{AppState, BackgroundRefreshStatus};
use crate::{api, engine, models};
use anyhow::Result;
use axum::extract::{Multipart, State};
use axum::response::Json;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

pub async fn api_daily_run_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::DailyOperationResult> {
    // Delegate to job-based (non-blocking start). Old callers will reload to see status.
    let job_res = api_jobs_daily_run_handler(State(state)).await;
    let jr = job_res.0;
    let success = jr.status != "error";
    Json(models::DailyOperationResult {
        success,
        message: jr.message.unwrap_or_else(|| "started".to_string()),
    })
}


pub async fn api_daily_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Option<models::DailyOperationReport>> {
    let status = state.refresh_status.read().await;
    Json(status.latest_daily_report.clone())
}


pub async fn api_daily_report_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Option<models::DailyOperationReport>> {
    let status = state.refresh_status.read().await;
    Json(status.latest_daily_report.clone())
}

// New job-based daily pipeline endpoints (POST starts, GET polls; persisted)


pub async fn api_jobs_daily_run_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::StartJobResponse> {
    let ctx = &state.ctx;
    // fast in-mem guard
    {
        let guards = state.running_jobs.read().await;
        if guards.contains("daily_pipeline") {
            if let Ok(Some(running)) = state.repo.get_running_job(ctx, "daily_pipeline").await {
                return Json(models::StartJobResponse {
                    job_id: running.job_id,
                    status: "running".to_string(),
                    message: Some("已在运行中".to_string()),
                });
            }
        }
    }
    match state.repo.start_job(ctx, "daily_pipeline").await {
        Ok(job) => {
            let job_id = job.job_id.clone();
            let already = matches!(job.status, models::WebJobStatus::Running);
            if !already {
                {
                    let mut guards = state.running_jobs.write().await;
                    guards.insert("daily_pipeline".to_string());
                }
                let repo = state.repo.clone();
                let ctx2 = state.ctx.clone();
                let guards = state.running_jobs.clone();
                let job_id_for_spawn = job_id.clone();
                tokio::spawn(async move {
                    let _ = repo
                        .update_job_progress(
                            &ctx2,
                            &job_id_for_spawn,
                            0,
                            7,
                            Some("正在执行每日流水线".to_string()),
                        )
                        .await;
                    match engine::daily_operation::run_daily_operation(repo.as_ref(), &ctx2).await {
                        Ok(report) => {
                            let mut steps: Vec<models::JobStepResult> = Vec::new();
                            let mut has_err = false;
                            let mut has_warn = false;
                            for s in &report.steps {
                                let st = match s.status {
                                    models::DailyOperationStatus::Success => "ok",
                                    models::DailyOperationStatus::PartialSuccess
                                    | models::DailyOperationStatus::Skipped => "warning",
                                    models::DailyOperationStatus::Failed => {
                                        has_err = true;
                                        "error"
                                    }
                                    _ => "ok",
                                };
                                if st == "warning" {
                                    has_warn = true;
                                }
                                steps.push(models::JobStepResult {
                                    name: s.name.clone(),
                                    status: st.to_string(),
                                    message: s.message.clone(),
                                    started_at: s.started_at.clone(),
                                    finished_at: s.completed_at.clone(),
                                    affected_count: 0,
                                    action_url: None,
                                });
                            }
                            let overall = if has_err {
                                models::WebJobStatus::Failed
                            } else if has_warn || !report.warnings.is_empty() {
                                models::WebJobStatus::PartialSuccess
                            } else {
                                models::WebJobStatus::Success
                            };
                            let mut msg = if steps
                                .iter()
                                .any(|s| s.name.contains("净值") && s.status != "ok")
                            {
                                Some("部分定投计划因缺少基金净值未执行。请先刷新净值或检查基金代码。".to_string())
                            } else {
                                None
                            };
                            if msg.is_none() {
                                msg = Some(if overall == models::WebJobStatus::PartialSuccess {
                                    "完成（部分警告）".to_string()
                                } else {
                                    "流水线完成".to_string()
                                });
                            }
                            let result = serde_json::json!({
                                "steps": steps,
                                "plan": report.plan,
                                "warnings": report.warnings,
                                "errors": report.errors,
                                "date": report.date,
                            });
                            let _ = repo
                                .finish_job(&ctx2, &job_id_for_spawn, overall, msg, Some(result))
                                .await;
                            let _ = repo.save_daily_operation_report(&ctx2, &report).await;
                        }
                        Err(e) => {
                            let _ = repo
                                .fail_job(&ctx2, &job_id_for_spawn, &format!("{}", e))
                                .await;
                        }
                    }
                    let mut g = guards.write().await;
                    g.remove("daily_pipeline");
                });
            }
            Json(models::StartJobResponse {
                job_id,
                status: if already {
                    "running".to_string()
                } else {
                    "started".to_string()
                },
                message: job.message,
            })
        }
        Err(e) => Json(models::StartJobResponse {
            job_id: String::new(),
            status: "error".to_string(),
            message: Some(format!("启动失败: {}", e)),
        }),
    }
}


pub async fn api_jobs_daily_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::JobStatusResponse> {
    let ctx = &state.ctx;
    let job = state
        .repo
        .get_latest_job(ctx, "daily_pipeline")
        .await
        .unwrap_or(None);
    let is_running = job.as_ref().is_some_and(|j| {
        matches!(
            j.status,
            models::WebJobStatus::Queued | models::WebJobStatus::Running
        )
    });
    Json(models::JobStatusResponse { job, is_running })
}
