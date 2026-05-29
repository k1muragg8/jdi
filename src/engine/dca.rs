use crate::models::{ConfigRoot, DcaFrequency, DcaPlan, DcaPreviewItem, DcaPreviewSummary};
use chrono::{Datelike, NaiveDate};

pub fn calculate_dca_preview(
    config: &ConfigRoot,
    plans: &[DcaPlan],
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
            } else {
                // Default to Monday if not specified?
                // Specification says "optional for weekly", let's assume it means "every day" if Daily,
                // but for Weekly it should probably have one. If missing, treat as not due unless we decide a default.
                // Let's default to 1 (Monday) if missing for Weekly.
                if weekday == 1 {
                    "今日应投".to_string()
                } else {
                    "未到日期".to_string()
                }
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
            } else {
                // Default to 1st if missing for Monthly.
                if day == 1 {
                    "今日应投".to_string()
                } else {
                    "未到日期".to_string()
                }
            }
        }
    }
}
