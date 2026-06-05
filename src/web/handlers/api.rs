//! JSON API handlers (no HTML).

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

pub async fn api_dca_run_due_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::DcaExecutionResult> {
    let ctx = &state.ctx;
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let date = Local::now().format("%Y-%m-%d").to_string();
        let res = engine::dca::auto_execute_dca(state.repo.as_ref(), &ctx, &config, &date).await?;
        Ok::<models::DcaExecutionResult, anyhow::Error>(res)
    }
    .await;

    match result {
        Ok(res) => Json(res),
        Err(e) => Json(models::DcaExecutionResult {
            success: false,
            message: format!("DCA execution failed: {}", e),
            ..Default::default()
        }),
    }
}

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
                        for a in &mut cfg.assets {
                            if a.sector.is_empty() || a.sector == "未分类" || a.sector == "待确认"
                            {
                                let name = a.fund_name.to_lowercase();
                                let new_sector =
                                    if name.contains("纳斯达克") || name.contains("qqq") {
                                        Some("美国科技".to_string())
                                    } else if name.contains("标普") || name.contains("spx") {
                                        Some("美国".to_string())
                                    } else if name.contains("沪深300") || name.contains("300") {
                                        Some("A股".to_string())
                                    } else if name.contains("债") {
                                        Some("债券".to_string())
                                    } else {
                                        None
                                    };
                                if let Some(s) = new_sector {
                                    a.sector = s;
                                    changed += 1;
                                }
                            }
                        }
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

pub async fn api_dca_plans_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<models::DcaPlan>> {
    let ctx = &state.ctx;
    let plans = state.repo.load_plans(&ctx).await.unwrap_or_default();
    Json(plans)
}

#[derive(Deserialize)]
pub struct DcaPlanForm {
    asset_id: String,
    amount: f64,
    frequency: String,
    day: Option<u32>,
    note: Option<String>,
}

pub async fn api_dca_add_plan_handler(
    State(state): State<Arc<AppState>>,
    Json(form): Json<DcaPlanForm>,
) -> Json<models::DcaExecutionResult> {
    let ctx = &state.ctx;
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let asset = config.assets.iter().find(|a| a.asset_id == form.asset_id);

        if let Some(a) = asset {
            let mut plans = state.repo.load_plans(&ctx).await?;
            let freq = match form.frequency.as_str() {
                "daily" => models::DcaFrequency::Daily,
                "weekly" => models::DcaFrequency::Weekly,
                "monthly" => models::DcaFrequency::Monthly,
                _ => return Err(anyhow::anyhow!("无效的频率")),
            };

            let plan_id = format!("plan_{}", chrono::Local::now().timestamp_millis());
            let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let new_plan = models::DcaPlan {
                plan_id: plan_id.clone(),
                asset_id: form.asset_id.clone(),
                fund_code: a.fund_code.clone(),
                fund_name: a.fund_name.clone(),
                amount: form.amount,
                currency: "CNY".to_string(),
                frequency: freq,
                weekday: if form.frequency == "weekly" {
                    form.day
                } else {
                    None
                },
                month_day: if form.frequency == "monthly" {
                    form.day
                } else {
                    None
                },
                start_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
                end_date: None,
                enabled: true,
                priority: 0,
                note: form.note.or(Some("Via Web API".to_string())),
                created_at: now_str.clone(),
                updated_at: now_str,
            };

            plans.push(new_plan);
            state.repo.save_plans(&ctx, &plans).await?;
            Ok::<String, anyhow::Error>(plan_id)
        } else {
            Err(anyhow::anyhow!("资产未找到"))
        }
    }
    .await;

    match result {
        Ok(id) => Json(models::DcaExecutionResult {
            success: true,
            message: format!("Plan created: {}", id),
            ..Default::default()
        }),
        Err(e) => Json(models::DcaExecutionResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

#[derive(Deserialize)]
pub struct DcaUpdateForm {
    amount: Option<f64>,
    frequency: Option<String>,
    day: Option<u32>,
    note: Option<String>,
    enabled: Option<bool>,
}

pub async fn api_dca_update_plan_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(plan_id): axum::extract::Path<String>,
    Json(form): Json<DcaUpdateForm>,
) -> Json<models::DcaExecutionResult> {
    let ctx = &state.ctx;
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        if let Some(p) = plans.iter_mut().find(|p| p.plan_id == plan_id) {
            if let Some(a) = form.amount {
                p.amount = a;
            }
            if let Some(f) = form.frequency {
                p.frequency = match f.as_str() {
                    "daily" => models::DcaFrequency::Daily,
                    "weekly" => models::DcaFrequency::Weekly,
                    "monthly" => models::DcaFrequency::Monthly,
                    _ => p.frequency.clone(),
                };
                if f == "weekly" {
                    p.weekday = form.day;
                    p.month_day = None;
                } else if f == "monthly" {
                    p.month_day = form.day;
                    p.weekday = None;
                }
            }
            if let Some(n) = form.note {
                p.note = Some(n);
            }
            if let Some(e) = form.enabled {
                p.enabled = e;
            }
            p.updated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            state.repo.save_plans(&ctx, &plans).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Json(models::DcaExecutionResult {
            success: true,
            message: "Plan updated".to_string(),
            ..Default::default()
        }),
        Err(e) => Json(models::DcaExecutionResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

pub async fn api_dca_remove_plan_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(plan_id): axum::extract::Path<String>,
) -> Json<models::DcaExecutionResult> {
    let ctx = &state.ctx;
    let result = async {
        let mut plans = state.repo.load_plans(&ctx).await?;
        let len_before = plans.len();
        plans.retain(|p| p.plan_id != plan_id);
        if plans.len() < len_before {
            state.repo.save_plans(&ctx, &plans).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("计划未找到"))
        }
    }
    .await;

    match result {
        Ok(_) => Json(models::DcaExecutionResult {
            success: true,
            message: "Plan removed".to_string(),
            ..Default::default()
        }),
        Err(e) => Json(models::DcaExecutionResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

pub async fn api_dca_executions_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<models::DcaSettlement>> {
    let ctx = &state.ctx;
    let mut settlements = state.repo.load_settlements(&ctx).await.unwrap_or_default();
    // Sort by deduction_date DESC
    settlements.sort_by(|a, b| b.deduction_date.cmp(&a.deduction_date));
    Json(settlements)
}

pub async fn api_import_preview_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<models::import::TransactionImportPreview> {
    let ctx = &state.ctx;
    let result = async {
        let mut content = String::new();
        while let Some(field) = multipart.next_field().await? {
            if field.name() == Some("file") {
                content = field.text().await?;
                break;
            }
        }

        if content.is_empty() {
            anyhow::bail!("Empty file or no file field found");
        }

        let transactions = state.repo.load_transactions(&ctx).await?;
        let candidates = engine::import::parse_transactions_from_csv(&content)?;
        let preview = engine::import::preview_import(candidates, &transactions);
        Ok::<models::import::TransactionImportPreview, anyhow::Error>(preview)
    }
    .await;

    match result {
        Ok(p) => Json(p),
        Err(_e) => Json(models::import::TransactionImportPreview {
            candidates: vec![],
            duplicates: vec![],
            warnings: vec![],
            errors: vec![],
            summary: models::import::ImportSummary {
                total_rows: 0,
                valid_rows: 0,
                error_rows: 1,
                warning_rows: 0,
                duplicate_rows: 0,
                new_rows: 0,
            },
        }),
    }
}

pub async fn api_import_commit_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<models::import::ImportResult> {
    let ctx = &state.ctx;
    let result = async {
        let mut content = String::new();
        while let Some(field) = multipart.next_field().await? {
            if field.name() == Some("file") {
                content = field.text().await?;
                break;
            }
        }

        if content.is_empty() {
            anyhow::bail!("Empty file or no file field found");
        }

        let mut transactions = state.repo.load_transactions(&ctx).await?;
        let mut portfolio_state = state.repo.load_state(&ctx).await?;
        let candidates = engine::import::parse_transactions_from_csv(&content)?;
        let preview = engine::import::preview_import(candidates, &transactions);

        if preview.summary.error_rows > 0 {
            anyhow::bail!("Import rejected: file contains errors.");
        }

        let import_result = engine::import::commit_import(
            &preview,
            &mut portfolio_state,
            &mut transactions,
            true, // skip duplicates
        );

        if import_result.inserted > 0 {
            state.repo.save_state(&ctx, &portfolio_state).await?;
            state.repo.save_transactions(&ctx, &transactions).await?;
        }

        Ok::<models::import::ImportResult, anyhow::Error>(import_result)
    }
    .await;

    match result {
        Ok(r) => Json(r),
        Err(e) => Json(models::import::ImportResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

pub async fn api_alipay_holdings_preview_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<models::AlipayHoldingImportPreview> {
    let ctx = &state.ctx;
    let result = async {
        let mut content = String::new();
        let mut date = String::new();
        while let Some(field) = multipart.next_field().await? {
            match field.name() {
                Some("file") => content = field.text().await?,
                Some("date") => date = field.text().await?,
                _ => {}
            }
        }

        if content.is_empty() {
            anyhow::bail!("Empty file or no file field found");
        }
        if date.is_empty() {
            date = Local::now().format("%Y-%m-%d").to_string();
        }

        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let candidates = engine::alipay_holding::parse_alipay_holdings_from_csv(&content)?;
        let preview = engine::alipay_holding::preview_alipay_holdings(
            &config,
            &portfolio_state,
            candidates,
            &date,
        );
        Ok::<models::AlipayHoldingImportPreview, anyhow::Error>(preview)
    }
    .await;

    match result {
        Ok(p) => Json(p),
        Err(_e) => Json(models::AlipayHoldingImportPreview::default()),
    }
}

pub async fn api_alipay_holdings_align_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<models::AlipayHoldingImportResult> {
    let ctx = &state.ctx;
    let result = async {
        let mut content = String::new();
        let mut date = String::new();
        while let Some(field) = multipart.next_field().await? {
            match field.name() {
                Some("file") => content = field.text().await?,
                Some("date") => date = field.text().await?,
                _ => {}
            }
        }

        if content.is_empty() {
            anyhow::bail!("Empty file or no file field found");
        }
        if date.is_empty() {
            date = Local::now().format("%Y-%m-%d").to_string();
        }

        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let candidates = engine::alipay_holding::parse_alipay_holdings_from_csv(&content)?;
        let preview = engine::alipay_holding::preview_alipay_holdings(
            &config,
            &portfolio_state,
            candidates,
            &date,
        );

        let snapshots = engine::alipay_holding::convert_to_snapshots(&preview);
        let imported_count = snapshots.len();

        if imported_count > 0 {
            let mut existing = state.repo.load_alipay_snapshots(&ctx).await?;
            existing.extend(snapshots);
            state.repo.save_alipay_snapshots(&ctx, &existing).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "web_user".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "IMPORT_ALIPAY_SNAPSHOTS".to_string(),
                target_file: "alipay_snapshots.json".to_string(),
                target_id: Some(date),
                old_value_summary: format!("existing: {}", existing.len() - imported_count),
                new_value_summary: format!("total: {}", existing.len()),
                status: "success".to_string(),
                note: Some(format!("Imported {} snapshots", imported_count)),
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
        }

        Ok::<usize, anyhow::Error>(imported_count)
    }
    .await;

    match result {
        Ok(count) => Json(models::AlipayHoldingImportResult {
            imported_count: count,
            success: true,
            message: format!(
                "成功导入 {} 笔快照。请前往对账页面查看并进行必要的手动校准。",
                count
            ),
            ..Default::default()
        }),
        Err(e) => Json(models::AlipayHoldingImportResult {
            success: false,
            message: e.to_string(),
            ..Default::default()
        }),
    }
}

pub async fn api_reconciliation_report_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::ReconciliationReport> {
    let ctx = &state.ctx;
    let result = async {
        let transactions = state.repo.load_transactions(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let report =
            engine::reconcile_portfolio(&ctx.portfolio_id, &portfolio_state, &transactions);
        Ok::<models::ReconciliationReport, anyhow::Error>(report)
    }
    .await;

    match result {
        Ok(r) => Json(r),
        Err(_e) => Json(models::ReconciliationReport {
            portfolio_id: "error".to_string(),
            generated_at: chrono::Local::now().to_rfc3339(),
            summary: models::ReconciliationSummary::default(),
            issues: vec![],
        }),
    }
}

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

pub async fn api_operation_status_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::OperationStatus> {
    let ctx = &state.ctx;
    let status = state
        .repo
        .load_operation_status(&ctx)
        .await
        .unwrap_or_else(|_| models::OperationStatus::default());
    Json(status)
}

pub async fn api_operation_report_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    let status = state
        .repo
        .load_operation_status(&ctx)
        .await
        .unwrap_or_else(|_| models::OperationStatus::default());

    if let Some(report) = status.last_report {
        Json(serde_json::to_value(report).unwrap())
    } else {
        Json(serde_json::json!({ "error": "No report available" }))
    }
}

pub async fn api_operation_run_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    let config_res = state.repo.load_config(&ctx).await;

    match config_res {
        Ok(config) => {
            // run_autonomous_operation now handles internal refresh if needed via evaluate_operation_state
            match engine::run_autonomous_operation(state.repo.as_ref(), &ctx, &config).await {
                Ok(report) => Json(serde_json::json!({ "success": true, "report": report })),
                Err(e) => Json(
                    serde_json::json!({ "success": false, "message": e.to_string() as String }),
                ),
            }
        }
        Err(e) => Json(serde_json::json!({ "success": false, "message": e.to_string() as String })),
    }
}

pub async fn api_get_operation_policies_handler(
    State(state): State<Arc<AppState>>,
) -> Json<models::OperationPolicy> {
    let ctx = &state.ctx;
    let policy = state
        .repo
        .load_operation_policy(&ctx)
        .await
        .unwrap_or_else(|_| models::OperationPolicy::default());
    Json(policy)
}

pub async fn api_save_operation_policies_handler(
    State(state): State<Arc<AppState>>,
    Json(policy): Json<models::OperationPolicy>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    match state.repo.save_operation_policy(&ctx, &policy).await {
        Ok(_) => Json(serde_json::json!({ "success": true })),
        Err(e) => Json(serde_json::json!({ "success": false, "message": e.to_string() })),
    }
}

// --- Backtest Handlers ---

#[derive(Deserialize)]
pub struct BacktestRunForm {
    start_date: String,
    end_date: String,
    initial_cash: f64,
    include_baseline: bool,
}

pub async fn api_backtest_run_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BacktestRunForm>,
) -> Json<serde_json::Value> {
    let ctx = &state.ctx;
    let config = match state.repo.load_config(&ctx).await {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({ "success": false, "message": e.to_string() })),
    };

    let req = models::BacktestRequest {
        start_date: payload.start_date,
        end_date: payload.end_date,
        initial_cash: payload.initial_cash,
        portfolio_id: ctx.portfolio_id.clone(),
        policy_override: None,
        include_baseline: payload.include_baseline,
    };

    match engine::backtest::run_backtest(state.repo.as_ref(), &ctx, &config, req).await {
        Ok(report) => {
            let mut last_report = state.last_backtest_report.write().await;
            *last_report = Some(report.clone());
            Json(serde_json::json!({ "success": true, "report": report }))
        }
        Err(e) => Json(serde_json::json!({ "success": false, "message": e.to_string() })),
    }
}

pub async fn api_backtest_latest_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let report_opt = state.last_backtest_report.read().await;
    if let Some(report) = report_opt.as_ref() {
        Json(serde_json::json!({ "success": true, "report": report }))
    } else {
        Json(serde_json::json!({ "success": false, "message": "No backtest report found" }))
    }
}

// Cash structs
#[derive(Deserialize)]
pub struct AssetIdForm {
    pub asset_id: String,
}

#[derive(Deserialize)]
pub struct CashSetForm {
    pub amount: f64,
}

#[derive(Deserialize)]
pub struct CashAdjustForm {
    pub amount: f64, // positive for cash_in, negative for cash_out
}
