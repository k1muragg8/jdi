//! POST actions: cash

use crate::web::handlers::forms::{AssetIdForm, CashAdjustForm, CashSetForm};
use super::types::*;
use crate::web::state::AppState;
use crate::{engine, models};
use axum::extract::{Form, State};
use axum::response::Redirect;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;

pub async fn api_cash_set_initial_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CashSetForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let tx = crate::models::Transaction {
        id: uuid::Uuid::new_v4().to_string(),
        date: Local::now().format("%Y-%m-%d").to_string(),
        transaction_type: "cash_set".to_string(),
        asset_id: None,
        amount: form.amount,
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: "Web端初始现金设定".to_string(),
        source: "manual".to_string(),
        raw_description: "Initial cash set".to_string(),
    };
    let mut transactions = state.repo.load_transactions(&ctx).await.unwrap_or_default();
    transactions.push(tx);
    let _ = state.repo.save_transactions(&ctx, &transactions).await;
    if let Ok(new_state) =
        crate::engine::holdings::rebuild_holdings_from_transactions(&transactions)
    {
        let _ = state.repo.save_state(&ctx, &new_state).await;
    }
    Redirect::to("/holdings?success=初始现金已设置")
}


pub async fn api_cash_adjust_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CashAdjustForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let tx_type = if form.amount >= 0.0 {
        "cash_in"
    } else {
        "cash_out"
    };
    let amount = form.amount.abs();
    let tx = crate::models::Transaction {
        id: uuid::Uuid::new_v4().to_string(),
        date: Local::now().format("%Y-%m-%d").to_string(),
        transaction_type: tx_type.to_string(),
        asset_id: None,
        amount,
        units: None,
        price: None,
        fee: 0.0,
        currency: "CNY".to_string(),
        note: "Web端现金调整".to_string(),
        source: "manual".to_string(),
        raw_description: format!("Cash {}", tx_type),
    };
    let mut transactions = state.repo.load_transactions(&ctx).await.unwrap_or_default();
    transactions.push(tx);
    let _ = state.repo.save_transactions(&ctx, &transactions).await;
    if let Ok(new_state) =
        crate::engine::holdings::rebuild_holdings_from_transactions(&transactions)
    {
        let _ = state.repo.save_state(&ctx, &new_state).await;
    }
    Redirect::to("/holdings?success=现金调整已记录")
}

pub async fn api_cash_reverse_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CashReverseForm>,
) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut transactions = state.repo.load_transactions(&ctx).await?;
        let tx = transactions
            .iter()
            .find(|t| t.id == form.tx_id)
            .ok_or_else(|| anyhow::anyhow!("流水未找到"))?;
        if tx.note.contains("已冲正") {
            anyhow::bail!("该流水已冲正");
        }
        let reverse_type = if tx.transaction_type == "cash_in" || tx.transaction_type == "现金转入"
        {
            "cash_out"
        } else {
            "cash_in"
        };
        let rev = crate::models::Transaction {
            id: uuid::Uuid::new_v4().to_string(),
            date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            transaction_type: reverse_type.to_string(),
            asset_id: None,
            amount: tx.amount,
            units: None,
            price: None,
            fee: 0.0,
            currency: tx.currency.clone(),
            note: format!("冲正流水 {} (Web)", form.tx_id),
            source: "manual".to_string(),
            raw_description: format!("Reverse {}", form.tx_id),
        };
        if let Some(orig) = transactions.iter_mut().find(|t| t.id == form.tx_id) {
            orig.note = format!("{} [已冲正]", orig.note);
        }
        transactions.push(rev);
        state.repo.save_transactions(&ctx, &transactions).await?;
        if let Ok(new_state) =
            crate::engine::holdings::rebuild_holdings_from_transactions(&transactions)
        {
            state.repo.save_state(&ctx, &new_state).await?;
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;
    match result {
        Ok(_) => Redirect::to("/cash?success=冲正成功"),
        Err(e) => Redirect::to(&format!("/cash?error={}", e)),
    }
}
