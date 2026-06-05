//! POST actions: instruments

use super::types::*;
use crate::web::state::AppState;
use crate::{engine, models};
use axum::extract::{Form, State};
use axum::response::Redirect;
use serde::Deserialize;
use std::sync::Arc;

pub async fn admin_instrument_enable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentIdForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        if let Some(inst) = instruments
            .iter_mut()
            .find(|i| i.instrument_id == form.instrument_id)
        {
            inst.enabled = true;
            state.repo.save_instruments(&ctx, &instruments).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "enable_instrument".to_string(),
                target_file: "instruments.json".to_string(),
                target_id: Some(form.instrument_id.clone()),
                old_value_summary: "enabled: false".to_string(),
                new_value_summary: "enabled: true".to_string(),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("证券未找到"))
        }
    }
    .await;

    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok(_) => Redirect::to(&format!("/market?success=证券已启用{}", filter_suffix)),
        Err(e) => Redirect::to(&format!("/market?error={}{}", e, filter_suffix)),
    }
}

pub async fn admin_instrument_disable_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentIdForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        if let Some(inst) = instruments
            .iter_mut()
            .find(|i| i.instrument_id == form.instrument_id)
        {
            inst.enabled = false;
            state.repo.save_instruments(&ctx, &instruments).await?;

            let audit = models::WebAdminAudit {
                audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                actor: "local_web".to_string(),
                actor_user_id: Some(ctx.actor_user_id.clone()),
                target_user_id: Some(ctx.target_user_id.clone()),
                portfolio_id: Some(ctx.portfolio_id.clone()),
                role: Some(ctx.role.clone()),
                action: "disable_instrument".to_string(),
                target_file: "instruments.json".to_string(),
                target_id: Some(form.instrument_id.clone()),
                old_value_summary: "enabled: true".to_string(),
                new_value_summary: "enabled: false".to_string(),
                status: "success".to_string(),
                note: None,
            };
            state.repo.append_web_admin_audit(&ctx, audit).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("证券未找到"))
        }
    }
    .await;

    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok(_) => Redirect::to(&format!("/market?success=证券已禁用{}", filter_suffix)),
        Err(e) => Redirect::to(&format!("/market?error={}{}", e, filter_suffix)),
    }
}

pub async fn admin_instrument_update_metadata_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentMetadataForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        let instrument_id = form.instrument_id.clone();

        let (old_meta, new_meta) = {
            if let Some(inst) = instruments
                .iter_mut()
                .find(|i| i.instrument_id == instrument_id)
            {
                let old_meta = format!(
                    "name_zh: {:?}, label: {:?}, provider: {}, symbol: {}",
                    inst.name_zh, inst.display_label, inst.provider, inst.provider_symbol
                );

                if let Some(n) = form.name_zh.filter(|n| !n.trim().is_empty()) {
                    inst.name_zh = Some(n.trim().to_string());
                }
                if let Some(l) = form.display_label.filter(|l| !l.trim().is_empty()) {
                    inst.display_label = Some(l.trim().to_string());
                }
                if let Some(p) = form.provider.filter(|p| !p.trim().is_empty()) {
                    inst.provider = p.trim().to_lowercase();
                }
                if let Some(ps) = form.provider_symbol.filter(|p| !p.trim().is_empty()) {
                    inst.provider_symbol = ps.trim().to_string();
                }
                engine::instrument_watchlist::migrate_au9999_provider(inst);

                let new_meta = format!(
                    "name_zh: {:?}, label: {:?}, provider: {}, symbol: {}",
                    inst.name_zh, inst.display_label, inst.provider, inst.provider_symbol
                );
                (old_meta, new_meta)
            } else {
                return Err(anyhow::anyhow!("证券未找到"));
            }
        };

        state.repo.save_instruments(&ctx, &instruments).await?;

        let audit = models::WebAdminAudit {
            audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            actor: "local_web".to_string(),
            actor_user_id: Some(ctx.actor_user_id.clone()),
            target_user_id: Some(ctx.target_user_id.clone()),
            portfolio_id: Some(ctx.portfolio_id.clone()),
            role: Some(ctx.role.clone()),
            action: "update_instrument_metadata".to_string(),
            target_file: "instruments.json".to_string(),
            target_id: Some(instrument_id),
            old_value_summary: old_meta,
            new_value_summary: new_meta,
            status: "success".to_string(),
            note: None,
        };
        state.repo.append_web_admin_audit(&ctx, audit).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok(_) => Redirect::to(&format!(
            "/market?success=证券元数据更新成功{}",
            filter_suffix
        )),
        Err(e) => Redirect::to(&format!("/market?error={}{}", e, filter_suffix)),
    }
}

pub async fn admin_instrument_add_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentAddForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        let sym = form.symbol.trim().to_string();
        if sym.is_empty() {
            return Err(anyhow::anyhow!("symbol 不能为空"));
        }
        let id = form
            .instrument_id
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| sym.clone());
        let asset_class = match form.asset_class.as_deref().unwrap_or("Index") {
            "Etf" | "etf" => models::AssetClass::Etf,
            "Crypto" => models::AssetClass::Crypto,
            "Fx" => models::AssetClass::Fx,
            "SpotCommodity" => models::AssetClass::SpotCommodity,
            _ => models::AssetClass::Index,
        };
        let new_inst = models::InstrumentConfig {
            instrument_id: id.clone(),
            symbol: sym.clone(),
            display_symbol: Some(sym.clone()),
            name: form.name_zh.clone().unwrap_or_else(|| sym.clone()),
            name_zh: form.name_zh,
            name_en: None,
            description_zh: None,
            category_zh: None,
            display_label: None,
            asset_class,
            provider: form.provider.unwrap_or_else(|| "yahoo".to_string()),
            provider_symbol: sym.clone(),
            market: None,
            exchange: None,
            currency: form.currency.unwrap_or_else(|| "USD".to_string()),
            quote_unit: "1".to_string(),
            price_unit: "1".to_string(),
            timezone: None,
            enabled: true,
            archived: false,
            priority: 0,
            tags: vec![],
            note: Some("web added".to_string()),
        };
        engine::instrument_watchlist::upsert_instrument(&mut instruments, new_inst)?;
        state.repo.save_instruments(&ctx, &instruments).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok(_) => Redirect::to(&format!(
            "/market?success=标的新增或更新成功{}",
            filter_suffix
        )),
        Err(e) => Redirect::to(&format!("/market?error={}{}", e, filter_suffix)),
    }
}

pub async fn admin_instrument_archive_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentIdForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        let config = state.repo.load_config(&ctx).await.unwrap_or_default();
        let mut found = false;
        let mut referenced = false;
        for inst in &mut instruments {
            if inst.instrument_id == form.instrument_id {
                found = true;
                for a in &config.assets {
                    if a.reference_index_symbol.as_deref() == Some(&inst.symbol)
                        || a.reference_index_symbol.as_deref() == Some(&inst.provider_symbol)
                        || a.reference_index_name
                            .as_deref()
                            .map(|n| n.contains(&inst.symbol))
                            .unwrap_or(false)
                    {
                        referenced = true;
                    }
                }
                engine::archive_instrument(inst);
                break;
            }
        }
        if !found {
            return Err(anyhow::anyhow!("标的未找到"));
        }
        state.repo.save_instruments(&ctx, &instruments).await?;
        if referenced {
            Ok(
                "该标的仍被资产或策略引用，已归档并禁用。已归档，不再显示在默认行情列表。"
                    .to_string(),
            )
        } else {
            Ok("已归档，不再显示在默认行情列表。".to_string())
        }
    }
    .await;

    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok(msg) => Redirect::to(&format!("/market?success={}{}", msg, filter_suffix)),
        Err(e) => Redirect::to(&format!("/market?error={}{}", e, filter_suffix)),
    }
}

pub async fn admin_instrument_restore_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentIdForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        if let Some(inst) = instruments
            .iter_mut()
            .find(|i| i.instrument_id == form.instrument_id)
        {
            engine::restore_instrument(inst);
            state.repo.save_instruments(&ctx, &instruments).await?;
            Ok::<(), anyhow::Error>(())
        } else {
            Err(anyhow::anyhow!("标的未找到"))
        }
    }
    .await;

    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok(_) => Redirect::to(&format!("/market?success=标的已恢复{}", filter_suffix)),
        Err(e) => Redirect::to(&format!("/market?error={}{}", e, filter_suffix)),
    }
}

pub async fn admin_instrument_delete_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<InstrumentIdForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result: Result<String, anyhow::Error> = async {
        let instruments = state.repo.load_instruments(&ctx).await?;
        let sym_to_purge = instruments
            .iter()
            .find(|i| i.instrument_id == form.instrument_id)
            .map(|i| i.symbol.clone());

        state
            .repo
            .delete_instrument(&ctx, &form.instrument_id)
            .await?;

        if let Some(sym) = sym_to_purge {
            if let Ok(mut cache) = state.repo.load_market_cache(&ctx).await {
                cache.entries.retain(|e| e.symbol != sym);
                let _ = state.repo.save_market_cache(&ctx, &cache).await;
            }
        }
        Ok("标的已永久删除".to_string())
    }
    .await;

    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok(msg) => Redirect::to(&format!("/market?success={}{}", msg, filter_suffix)),
        Err(e) => Redirect::to(&format!("/market?error={}{}", e, filter_suffix)),
    }
}

#[derive(Deserialize, Default)]
pub struct RestoreDefaultsForm {
    pub cleanup_test: Option<String>,
    pub filter: Option<String>,
}

pub async fn admin_instrument_restore_defaults_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<RestoreDefaultsForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let also_cleanup = form
        .cleanup_test
        .as_deref()
        .is_some_and(|v| v == "1" || v == "on");
    let result = async {
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        engine::migrate_instrument_flags(&mut instruments);
        let (added, reactivated) =
            engine::restore_default_instruments(&mut instruments, also_cleanup);
        state.repo.save_instruments(&ctx, &instruments).await?;
        Ok::<(usize, usize), anyhow::Error>((added, reactivated))
    }
    .await;

    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok((added, reactivated)) => {
            let msg = if also_cleanup {
                format!(
                    "已恢复默认标的：新增 {}，重新启用 {}；测试标的已清理",
                    added, reactivated
                )
            } else {
                format!("已恢复默认标的：新增 {}，重新启用 {}", added, reactivated)
            };
            Redirect::to(&format!("/market?success={}{}", msg, filter_suffix))
        }
        Err(e) => Redirect::to(&format!("/market?error={}{}", e, filter_suffix)),
    }
}

#[derive(Deserialize, Default)]
pub struct CleanupTestForm {
    pub confirm: Option<String>,
    pub filter: Option<String>,
}

pub async fn admin_instrument_cleanup_test_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CleanupTestForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        if form.confirm.as_deref() != Some("1") {
            return Err(anyhow::anyhow!("缺少确认参数"));
        }
        let mut instruments = state.repo.load_instruments(&ctx).await?;
        let preview = engine::cleanup_test_instruments(&mut instruments.clone(), true);
        if preview == 0 {
            return Ok("未检测到待清理的测试标的".to_string());
        }
        let n = engine::cleanup_test_instruments(&mut instruments, false);
        state.repo.save_instruments(&ctx, &instruments).await?;
        Ok(format!("已归档 {} 个测试标的，不再显示在默认行情列表。", n))
    }
    .await;

    let filter_suffix = form
        .filter
        .as_ref()
        .map(|f| format!("&filter={}", f))
        .unwrap_or_default();
    match result {
        Ok(msg) => Redirect::to(&format!("/market?success={}{}", msg, filter_suffix)),
        Err(e) => Redirect::to(&format!("/market?error={}{}", e, filter_suffix)),
    }
}

// --- Autonomous Operation Handlers ---
