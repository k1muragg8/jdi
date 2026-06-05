//! API: import_alipay

use crate::web::state::{AppState, BackgroundRefreshStatus};
use crate::{api, engine, models};
use anyhow::Result;
use axum::extract::{Multipart, State};
use axum::response::Json;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

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
