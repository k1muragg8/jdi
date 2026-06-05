//! API: market

use crate::web::state::{AppState, BackgroundRefreshStatus};
use crate::{api, engine, models};
use anyhow::Result;
use axum::extract::State;
use axum::response::Json;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

pub async fn api_market_refresh_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::import::ImportResult> {
    // Delegate to job (non-blocking). UI uses jobs endpoints for status.
    let job_res = api_jobs_market_refresh_handler(State(state)).await;
    let jr = job_res.0;
    let success = jr.status != "error" && jr.status != "running";
    Json(models::import::ImportResult {
        success,
        message: jr.message.unwrap_or_else(|| "市场刷新已启动".to_string()),
        ..Default::default()
    })
}

pub async fn api_market_refresh_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<BackgroundRefreshStatus> {
    let status = state.refresh_status.read().await;
    Json(status.clone())
}

// Job-based market refresh (async, persisted, detailed result)

pub async fn api_jobs_market_refresh_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::StartJobResponse> {
    let ctx = &state.ctx;
    {
        let guards = state.running_jobs.read().await;
        if guards.contains("market_refresh") {
            if let Ok(Some(r)) = state.repo.get_running_job(ctx, "market_refresh").await {
                return Json(models::StartJobResponse {
                    job_id: r.job_id,
                    status: "running".to_string(),
                    message: Some("行情刷新已在运行".to_string()),
                });
            }
        }
    }
    match state.repo.start_job(ctx, "market_refresh").await {
        Ok(job) => {
            let job_id = job.job_id.clone();
            let already = matches!(job.status, models::WebJobStatus::Running);
            if !already {
                {
                    let mut g = state.running_jobs.write().await;
                    g.insert("market_refresh".to_string());
                }
                let repo = state.repo.clone();
                let ctx2 = state.ctx.clone();
                let g = state.running_jobs.clone();
                let job_id_for_spawn = job_id.clone();
                tokio::spawn(async move {
                    let _ = repo
                        .update_job_progress(
                            &ctx2,
                            &job_id_for_spawn,
                            0,
                            1,
                            Some("正在发现活跃标的并刷新行情".to_string()),
                        )
                        .await;
                    // Enhanced discovery per spec (builds on existing refresh logic)
                    let mut symbols: std::collections::HashSet<String> =
                        std::collections::HashSet::new();
                    if let Ok(cfg) = repo.load_config(&ctx2).await {
                        for a in &cfg.assets {
                            if a.enabled {
                                if let Some(s) = &a.reference_instrument_symbol {
                                    symbols.insert(s.clone());
                                }
                                if let Some(s) = &a.reference_index_symbol {
                                    symbols.insert(s.clone());
                                }
                            }
                        }
                        symbols.insert(cfg.risk.vix_symbol.clone());
                        symbols.insert(cfg.risk.us30y_symbol.clone());
                        for s in &cfg.risk.crypto_symbols {
                            symbols.insert(s.clone());
                        }
                        for s in &cfg.risk.equity_symbols {
                            symbols.insert(s.clone());
                        }
                        symbols.insert(cfg.fx.usd_cnh_symbol.clone());
                        // Kelly / operation related via risk etc already covered
                    }
                    if let Ok(instrs) = repo.load_instruments(&ctx2).await {
                        for i in instrs {
                            if i.enabled {
                                symbols.insert(i.symbol);
                            }
                        }
                    }
                    if let Ok(st) = repo.load_state(&ctx2).await {
                        for h in &st.asset_holdings {
                            if h.units > 0.0 {
                                if let Ok(cfg) = repo.load_config(&ctx2).await {
                                    if let Some(a) =
                                        cfg.assets.iter().find(|aa| aa.asset_id == h.asset_id)
                                    {
                                        if let Some(s) = &a.reference_instrument_symbol {
                                            symbols.insert(s.clone());
                                        }
                                        if let Some(s) = &a.reference_index_symbol {
                                            symbols.insert(s.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    let active: Vec<String> =
                        symbols.into_iter().filter(|s| !s.is_empty()).collect();
                    let active_count = active.len();

                    let config_for_refresh = repo.load_config(&ctx2).await.unwrap_or_default();
                    let refresh_res = engine::refresh::refresh_market_data(
                        repo.as_ref(),
                        &ctx2,
                        &config_for_refresh,
                    )
                    .await;

                    // Load fresh cache to classify per-symbol accurately
                    let latest_cache = repo.load_market_cache(&ctx2).await.unwrap_or_default();
                    let cache_map: std::collections::HashMap<_, _> = latest_cache
                        .entries
                        .iter()
                        .map(|e| (e.symbol.clone(), e))
                        .collect();

                    let mut result = models::MarketRefreshResult {
                        active_symbols_count: active_count,
                        success_count: 0,
                        skipped_count: 0,
                        failed_count: 0,
                        inserted_count: 0,
                        updated_count: 0,
                        ..Default::default()
                    };

                    for sym in &active {
                        if let Some(entry) = cache_map.get(sym) {
                            if entry.price > 0.0 || entry.status.as_deref() == Some("ok") {
                                result.success_count += 1;
                                result.refreshed_symbols.push(sym.clone());
                                if entry.previous_close.is_none() {
                                    // price present but no prev close
                                    result.skipped_symbols.push(sym.clone());
                                }
                            } else if entry.status.as_deref() == Some("failed") {
                                result.failed_count += 1;
                                result.failed_symbols.push(sym.clone());
                                if let Some(err) = &entry.error_message {
                                    result.provider_errors.push(format!("{}: {}", sym, err));
                                }
                            } else {
                                result.skipped_count += 1;
                                result.skipped_symbols.push(sym.clone());
                            }
                        } else {
                            // no entry at all after refresh -> treat as no_data or unsupported
                            result.failed_count += 1;
                            result.no_data_symbols.push(sym.clone());
                            result.failed_symbols.push(sym.clone());
                        }
                    }

                    result.inserted_count = result.success_count; // rough
                    if !result.failed_symbols.is_empty() {
                        result.provider_errors.push(format!(
                            "{} symbols had issues",
                            result.failed_symbols.len()
                        ));
                    }

                    match refresh_res {
                        Ok(_) => {}
                        Err(e) => {
                            result.provider_errors.push(e.to_string());
                        }
                    }
                    if active_count == 0 {
                        let msg = Some(
                            "没有可刷新的活跃标的，请先启用市场标的或配置资产锚定指数。"
                                .to_string(),
                        );
                        let _ = repo
                            .finish_job(
                                &ctx2,
                                &job_id_for_spawn,
                                models::WebJobStatus::Warning,
                                msg,
                                Some(
                                    serde_json::to_value(&result).unwrap_or(serde_json::json!({})),
                                ),
                            )
                            .await;
                    } else if result.success_count == 0 {
                        let msg = if !result.no_data_symbols.is_empty() {
                            Some("未获取到行情，请检查数据源或标的代码。".to_string())
                        } else {
                            Some("数据源不支持该标的或全部失败。".to_string())
                        };
                        let status = if result.failed_count > 0 {
                            models::WebJobStatus::Failed
                        } else {
                            models::WebJobStatus::Warning
                        };
                        let _ = repo
                            .finish_job(
                                &ctx2,
                                &job_id_for_spawn,
                                status,
                                msg,
                                Some(
                                    serde_json::to_value(&result).unwrap_or(serde_json::json!({})),
                                ),
                            )
                            .await;
                    } else {
                        let overall = if result.failed_count > 0 && result.success_count > 0 {
                            models::WebJobStatus::PartialSuccess
                        } else if result.failed_count > 0 {
                            models::WebJobStatus::Warning
                        } else {
                            models::WebJobStatus::Success
                        };
                        let msg = Some(format!(
                            "刷新完成: 成功 {} / 失败 {} (活跃 {})",
                            result.success_count, result.failed_count, active_count
                        ));
                        let _ = repo
                            .finish_job(
                                &ctx2,
                                &job_id_for_spawn,
                                overall,
                                msg,
                                Some(
                                    serde_json::to_value(&result).unwrap_or(serde_json::json!({})),
                                ),
                            )
                            .await;
                    }

                    // Update persisted CacheStatusRegistry from the quote cache (MarketCache is source of truth)
                    if let Ok(mut cs) = repo.load_cache_status(&ctx2).await {
                        if let Ok(mc) = repo.load_market_cache(&ctx2).await {
                            cs.market_cache_size = mc.entries.len();
                            cs.last_market_update =
                                mc.entries.iter().map(|e| &e.fetched_at).max().cloned();
                            let _ = repo.save_cache_status(&ctx2, &cs).await;
                        }
                    }

                    let mut gg = g.write().await;
                    gg.remove("market_refresh");
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

pub async fn api_jobs_market_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::JobStatusResponse> {
    let ctx = &state.ctx;
    let job = state
        .repo
        .get_latest_job(ctx, "market_refresh")
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

#[derive(Deserialize)]
pub struct SymbolRefresh {
    symbol: String,
}

pub async fn api_market_refresh_symbol_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SymbolRefresh>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    let sym = payload.symbol.trim().to_string();
    if sym.is_empty() {
        return Json(serde_json::json!({"success": false, "message": "symbol required"}));
    }
    // lookup inst to get correct fetch key (provider_symbol)
    let instruments = state.repo.load_instruments(ctx).await.unwrap_or_default();
    let inst_opt = instruments.iter().find(|i| i.symbol == sym);
    let (provider_name, fetch_sym) = inst_opt
        .map(|i| {
            let fetch = if i.provider_symbol.is_empty() {
                i.symbol.clone()
            } else {
                i.provider_symbol.clone()
            };
            (i.provider.clone(), fetch)
        })
        .unwrap_or_else(|| ("yahoo".to_string(), sym.clone()));
    let config = match state.repo.load_config(ctx).await {
        Ok(c) => c,
        Err(e) => {
            return Json(
                serde_json::json!({"success": false, "message": format!("config: {}", e)}),
            );
        }
    };
    let price_res: Result<models::MarketPrice, anyhow::Error> = {
        let fetch_sym2 = fetch_sym.clone();
        let provider_name2 = provider_name.clone();
        let config2 = config.clone();
        tokio::task::spawn_blocking(move || {
            api::fetch_market_price(&config2.market, &provider_name2, &fetch_sym2)
        })
        .await
        .unwrap_or_else(|_| Err(anyhow::anyhow!("task join error")))
    };
    let mut cache = state.repo.load_market_cache(ctx).await.unwrap_or_default();
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut success = false;
    let mut message = String::new();
    if let Ok(price) = price_res {
        let price = engine::normalize_market_price(price);
        success = true;
        message = format!("{} refreshed to {:.2}", sym, price.price);
        if let Some(entry) = cache.entries.iter_mut().find(|e| e.symbol == sym) {
            engine::apply_price_to_cache_entry(entry, &price, &now_str);
        } else {
            cache
                .entries
                .push(engine::new_cache_entry_from_price(&price, &sym, &now_str));
        }
    } else if let Err(e) = price_res {
        message = e.to_string();
        let currency = if provider_name == "eastmoney" {
            "CNY"
        } else {
            "USD"
        };
        if let Some(entry) = cache.entries.iter_mut().find(|e| e.symbol == sym) {
            entry.status = Some("failed".to_string());
            entry.error_message = Some(message.clone());
            entry.source = provider_name.clone();
            entry.fetched_at = now_str.clone();
        } else {
            cache.entries.push(models::MarketCacheEntry {
                symbol: sym.clone(),
                price: 0.0,
                date: now_str.clone(),
                currency: currency.to_string(),
                source: provider_name.clone(),
                fetched_at: now_str,
                previous_close: None,
                change: None,
                change_percent: None,
                status: Some("failed".to_string()),
                error_message: Some(message.clone()),
            });
        }
    }
    let _ = state.repo.save_market_cache(ctx, &cache).await;
    Json(serde_json::json!({"success": success, "message": message, "symbol": sym}))
}
