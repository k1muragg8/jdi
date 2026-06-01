use crate::models::{
    ConfigRoot, DcaExecutionResult, DcaFrequency, DcaPlan, DcaPreviewItem, DcaPreviewSummary,
    NavCache,
};
use crate::repository::{Repository, RepositoryContext};
use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate};

pub fn calculate_dca_preview(
    config: &ConfigRoot,
    plans: &[DcaPlan],
    nav_cache: &NavCache,
    target_date_str: &str,
) -> DcaPreviewSummary {
    let mut items = Vec::new();
    let mut total_due_amount = 0.0;
    let warnings = Vec::new();

    let target_date = match NaiveDate::parse_from_str(target_date_str, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            return DcaPreviewSummary {
                date: target_date_str.to_string(),
                total_due_amount: 0.0,
                items: Vec::new(),
                warnings: vec![format!("无效的日期格式: {}", target_date_str)],
            };
        }
    };

    for plan in plans {
        let mut plan_warnings = Vec::new();
        let mut status = "未到日期".to_string();

        let start_date = NaiveDate::parse_from_str(&plan.start_date, "%Y-%m-%d").ok();
        let end_date = plan
            .end_date
            .as_ref()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

        // Validate asset existence
        let asset_config = config.assets.iter().find(|a| a.asset_id == plan.asset_id);
        if asset_config.is_none() {
            status = "资产不存在".to_string();
            plan_warnings.push(format!("资产ID {} 在配置中不存在", plan.asset_id));
        } else if let Some(ac) = asset_config.filter(|ac| ac.fund_code != plan.fund_code) {
            plan_warnings.push(format!(
                "基金代码不一致: 计划为 {}, 配置为 {}",
                plan.fund_code, ac.fund_code
            ));
            status = "基金代码不一致".to_string();
        }

        if plan.amount <= 0.0 {
            status = "金额无效".to_string();
            plan_warnings.push("定投金额必须大于 0".to_string());
        }

        if !plan.enabled {
            status = "已禁用".to_string();
        } else if let Some(sd) = start_date {
            if target_date < sd {
                status = "未到日期".to_string();
            } else if let Some(ed) = end_date {
                if target_date > ed {
                    status = "已过结束日期".to_string();
                } else {
                    status = check_is_due(plan, &target_date);
                }
            } else {
                status = check_is_due(plan, &target_date);
            }
        }

        if status == "今日应投" {
            total_due_amount += plan.amount;
        }

        let nav_entry = nav_cache
            .entries
            .iter()
            .find(|e| e.fund_code == plan.fund_code);

        items.push(DcaPreviewItem {
            plan_id: plan.plan_id.clone(),
            asset_id: plan.asset_id.clone(),
            fund_code: plan.fund_code.clone(),
            fund_name: plan.fund_name.clone(),
            amount: plan.amount,
            currency: plan.currency.clone(),
            due_date: target_date_str.to_string(),
            frequency: plan.frequency.clone(),
            status,
            latest_nav: nav_entry.map(|e| e.nav),
            nav_date: nav_entry.map(|e| e.nav_date.clone()),
            warnings: plan_warnings,
        });
    }

    // Sort items by priority desc
    items.sort_by(|a, b| {
        let plan_a = plans.iter().find(|p| p.plan_id == a.plan_id);
        let plan_b = plans.iter().find(|p| p.plan_id == b.plan_id);
        let prio_a = plan_a.map(|p| p.priority).unwrap_or(0);
        let prio_b = plan_b.map(|p| p.priority).unwrap_or(0);
        prio_b.cmp(&prio_a)
    });

    DcaPreviewSummary {
        date: target_date_str.to_string(),
        total_due_amount,
        items,
        warnings,
    }
}

pub async fn auto_execute_dca(
    repo: &dyn Repository,
    ctx: &RepositoryContext,
    config: &ConfigRoot,
    target_date_str: &str,
) -> Result<DcaExecutionResult> {
    let plans = repo.load_plans(ctx).await?;
    let nav_cache = repo.load_nav_cache(ctx).await?;
    let preview = calculate_dca_preview(config, &plans, &nav_cache, target_date_str);

    let mut settlements = repo.load_settlements(ctx).await?;
    let mut transactions = repo.load_transactions(ctx).await?;
    let mut state = repo.load_state(ctx).await?;

    let mut executed_count = 0;
    let mut skipped_count = 0;
    let mut failed_count = 0;
    let mut messages = Vec::new();

    for item in preview.items {
        if item.status != "今日应投" {
            continue;
        }

        // Idempotency check: look for existing settlement for this plan and date
        let already_executed = settlements.iter().any(|s| {
            s.plan_id.as_deref() == Some(&item.plan_id) && s.deduction_date == target_date_str
        });

        if already_executed {
            skipped_count += 1;
            continue;
        }

        // Need NAV to execute
        let nav_entry = nav_cache
            .entries
            .iter()
            .find(|e| e.fund_code == item.fund_code);

        if let Some(nav) = nav_entry {
            let units = item.amount / nav.nav;
            let settlement_id = format!(
                "dca_auto_{}_{}",
                item.plan_id,
                Local::now().timestamp_millis()
            );
            let tx_id = format!("tx_dca_{}", settlement_id);

            let settlement = crate::models::DcaSettlement {
                settlement_id: settlement_id.clone(),
                plan_id: Some(item.plan_id.clone()),
                asset_id: item.asset_id.clone(),
                fund_code: item.fund_code.clone(),
                fund_name: item.fund_name.clone(),
                scheduled_date: Some(target_date_str.to_string()),
                deduction_date: target_date_str.to_string(),
                confirmation_date: target_date_str.to_string(),
                amount: item.amount,
                confirmed_nav: nav.nav,
                confirmed_units: units,
                fee: Some(0.0),
                currency: item.currency.clone(),
                source: "dca_auto".to_string(),
                status: crate::models::DcaSettlementStatus::Confirmed,
                applied: true,
                note: Some("DCA Auto Execution".to_string()),
                created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            };

            let tx = crate::models::Transaction {
                id: tx_id,
                date: target_date_str.to_string(),
                transaction_type: "buy".to_string(),
                asset_id: Some(item.asset_id.clone()),
                amount: item.amount,
                units: Some(units),
                price: Some(nav.nav),
                fee: 0.0,
                currency: item.currency.clone(),
                note: format!("DCA Auto: {}", item.fund_name),
                source: "dca".to_string(),
                raw_description: "DCA Auto Execution".to_string(),
            };

            if let Err(e) = crate::engine::holdings::apply_transaction(&mut state, &tx) {
                failed_count += 1;
                messages.push(format!("应用交易失败 ({}): {}", item.fund_name, e));
                continue;
            }

            settlements.push(settlement);
            transactions.push(tx);
            executed_count += 1;
            messages.push(format!("成功执行: {} ({})", item.fund_name, item.amount));
        } else {
            failed_count += 1;
            messages.push(format!("缺少净值数据，无法执行: {}", item.fund_name));
        }
    }

    if executed_count > 0 {
        repo.save_settlements(ctx, &settlements).await?;
        repo.save_transactions(ctx, &transactions).await?;
        repo.save_state(ctx, &state).await?;
    }

    Ok(DcaExecutionResult {
        executed_count,
        skipped_count,
        failed_count,
        success: failed_count == 0,
        message: if executed_count > 0 || failed_count > 0 {
            messages.join("; ")
        } else {
            "今日无待执行计划或已全部执行".to_string()
        },
    })
}

fn check_is_due(plan: &DcaPlan, date: &NaiveDate) -> String {
    match plan.frequency {
        DcaFrequency::Daily => "今日应投".to_string(),
        DcaFrequency::Weekly => {
            let weekday = date.weekday().number_from_monday(); // 1-7
            if let Some(target_weekday) = plan.weekday {
                if weekday == target_weekday {
                    "今日应投".to_string()
                } else {
                    "未到日期".to_string()
                }
            } else if weekday == 1 {
                "今日应投".to_string()
            } else {
                "未到日期".to_string()
            }
        }
        DcaFrequency::Monthly => {
            let day = date.day();
            if let Some(target_day) = plan.month_day {
                if day == target_day {
                    "今日应投".to_string()
                } else {
                    "未到日期".to_string()
                }
            } else if day == 1 {
                "今日应投".to_string()
            } else {
                "未到日期".to_string()
            }
        }
    }
}
