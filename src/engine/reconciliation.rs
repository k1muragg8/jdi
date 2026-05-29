use crate::models::{
    AlipaySnapshot, CalibrationSuggestion, ConfigRoot, PortfolioState, ReconciliationAudit,
    ReconciliationResult,
};
use chrono::NaiveDate;

pub fn reconcile_asset(
    config: &ConfigRoot,
    state: &PortfolioState,
    snapshot: &AlipaySnapshot,
) -> ReconciliationResult {
    let mut warnings = Vec::new();
    let mut status = "一致".to_string();
    let mut suggested_action = "无".to_string();

    let holding = state
        .asset_holdings
        .iter()
        .find(|h| h.asset_id == snapshot.asset_id);

    let system_market_value = holding.map(|h| h.last_market_value).unwrap_or(0.0);
    let system_units = holding.map(|h| h.units).unwrap_or(0.0);
    let system_cost_basis = holding.map(|h| h.cost_basis).unwrap_or(0.0);
    let system_nav = holding.and_then(|h| h.latest_nav).unwrap_or(0.0);
    let system_nav_date = holding.and_then(|h| h.latest_nav_date.clone());

    let market_value_diff = snapshot.market_value - system_market_value;
    let market_value_diff_pct = if system_market_value != 0.0 {
        market_value_diff / system_market_value
    } else if snapshot.market_value != 0.0 {
        1.0
    } else {
        0.0
    };

    let units_diff = snapshot.units.map(|u| u - system_units);
    let units_diff_pct = units_diff.map(|d| {
        if system_units != 0.0 {
            d / system_units
        } else if snapshot.units.unwrap_or(0.0) != 0.0 {
            1.0
        } else {
            0.0
        }
    });

    let cost_basis_diff = snapshot.cost_basis.map(|cb| cb - system_cost_basis);
    let cost_basis_diff_pct = cost_basis_diff.map(|d| {
        if system_cost_basis != 0.0 {
            d / system_cost_basis
        } else if snapshot.cost_basis.unwrap_or(0.0) != 0.0 {
            1.0
        } else {
            0.0
        }
    });
    let _nav_diff = snapshot.nav.map(|n| n - system_nav);
    let nav_date_diff = if let (Some(sn), Some(an)) = (system_nav_date, snapshot.nav_date.clone()) {
        let sd = NaiveDate::parse_from_str(&sn, "%Y-%m-%d").ok();
        let ad = NaiveDate::parse_from_str(&an, "%Y-%m-%d").ok();
        if let (Some(s), Some(a)) = (sd, ad) {
            Some((a - s).num_days())
        } else {
            None
        }
    } else {
        None
    };

    // Check tolerances
    let mv_tol_abs = config.reconciliation.market_value_tolerance_abs;
    let mv_tol_pct = config.reconciliation.market_value_tolerance_pct;

    let mut needs_calibration = false;

    if market_value_diff.abs() > mv_tol_abs || market_value_diff_pct.abs() > mv_tol_pct {
        status = if market_value_diff_pct.abs() > 0.01 {
            "明显差异".to_string()
        } else {
            "小幅差异".to_string()
        };
        suggested_action = "核对交易记录".to_string();
    }

    if let (Some(ud), Some(up)) = (units_diff, units_diff_pct) {
        if ud.abs() > config.reconciliation.units_tolerance_abs
            || up.abs() > config.reconciliation.units_tolerance_pct
        {
            status = "份额不一致".to_string();
            warnings.push(format!("系统份额与支付宝份额不符: diff {:.4}", ud));
            suggested_action = "校准持仓份额".to_string();
            needs_calibration = true;
        }
    }

    if let (Some(cd), Some(cp)) = (cost_basis_diff, cost_basis_diff_pct) {
        if cd.abs() > config.reconciliation.cost_basis_tolerance_abs
            || cp.abs() > config.reconciliation.cost_basis_tolerance_pct
        {
            if status == "一致" || status == "小幅差异" {
                status = "成本不一致".to_string();
            }
            warnings.push(format!("系统成本与支付宝成本不符: diff {:.2}", cd));
            if suggested_action == "无" || suggested_action == "核对交易记录" {
                suggested_action = "校准成本价格".to_string();
            }
            needs_calibration = true;
        }
    }

    if let Some(nd) = nav_date_diff {
        if nd != 0 {
            warnings.push(format!("系统净值日期与支付宝净值日期不符: 相差 {} 天", nd));
            if status == "一致" {
                status = "净值日期不一致".to_string();
            }
        }
    }

    if holding.is_none() {
        status = "缺少系统持仓".to_string();
        warnings.push("系统中未找到该资产的持仓记录".to_string());
        suggested_action = "初始化持仓".to_string();
        needs_calibration = true;
    }

    if needs_calibration
        && status != "缺少系统持仓"
        && status != "份额不一致"
        && status != "成本不一致"
    {
        status = "需要校准".to_string();
    }

    ReconciliationResult {
        snapshot_id: snapshot.snapshot_id.clone(),
        asset_id: snapshot.asset_id.clone(),
        fund_code: snapshot.fund_code.clone(),
        fund_name: snapshot.fund_name.clone(),
        snapshot_date: snapshot.snapshot_date.clone(),
        system_market_value,
        alipay_market_value: snapshot.market_value,
        market_value_diff,
        market_value_diff_pct,
        system_units: Some(system_units),
        alipay_units: snapshot.units,
        units_diff,
        units_diff_pct,
        system_cost_basis: Some(system_cost_basis),
        alipay_cost_basis: snapshot.cost_basis,
        cost_basis_diff,
        cost_basis_diff_pct,
        system_nav: Some(system_nav),
        alipay_nav: snapshot.nav,
        nav_diff: snapshot.nav.map(|n| n - system_nav),
        nav_date_diff,
        status,
        warnings,
        suggested_action,
    }
}

pub fn generate_calibration_suggestion(
    result: &ReconciliationResult,
) -> Option<CalibrationSuggestion> {
    if result.status == "一致" {
        return None;
    }

    let mut suggestion = CalibrationSuggestion {
        asset_id: result.asset_id.clone(),
        fund_code: result.fund_code.clone(),
        snapshot_id: result.snapshot_id.clone(),
        suggested_units: result.alipay_units,
        suggested_cost_basis: result.alipay_cost_basis,
        suggested_market_value: Some(result.alipay_market_value),
        reason: format!("基于支付宝快照 {} 的校准", result.snapshot_date),
        risk_level: "中".to_string(),
        would_modify_state: true,
        would_create_adjustment_transaction: true,
    };

    if result.status == "明显差异" || result.status == "份额不一致" {
        suggestion.risk_level = "高".to_string();
    } else if result.status == "小幅差异" || result.status == "成本不一致" {
        suggestion.risk_level = "低".to_string();
    }

    Some(suggestion)
}

pub fn apply_calibration(
    state: &mut PortfolioState,
    suggestion: &CalibrationSuggestion,
) -> ReconciliationAudit {
    let holding = state
        .asset_holdings
        .iter_mut()
        .find(|h| h.asset_id == suggestion.asset_id);

    let mut audit = ReconciliationAudit {
        audit_id: format!("audit_{}", chrono::Local::now().timestamp_millis()),
        timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        snapshot_id: suggestion.snapshot_id.clone(),
        asset_id: suggestion.asset_id.clone(),
        old_units: 0.0,
        new_units: 0.0,
        old_cost_basis: 0.0,
        new_cost_basis: 0.0,
        old_market_value: 0.0,
        new_market_value: 0.0,
        reason: suggestion.reason.clone(),
        note: None,
    };

    if let Some(h) = holding {
        audit.old_units = h.units;
        audit.old_cost_basis = h.cost_basis;
        audit.old_market_value = h.last_market_value;

        if let Some(u) = suggestion.suggested_units {
            h.units = u;
        }
        if let Some(c) = suggestion.suggested_cost_basis {
            h.cost_basis = c;
        }
        if let Some(mv) = suggestion.suggested_market_value {
            h.last_market_value = mv;
        }

        audit.new_units = h.units;
        audit.new_cost_basis = h.cost_basis;
        audit.new_market_value = h.last_market_value;
    } else {
        // Create new holding if it doesn't exist (Calibration initialization)
        let new_units = suggestion.suggested_units.unwrap_or(0.0);
        let new_cost_basis = suggestion.suggested_cost_basis.unwrap_or(0.0);
        let new_market_value = suggestion.suggested_market_value.unwrap_or(0.0);

        state.asset_holdings.push(crate::models::AssetHolding {
            asset_id: suggestion.asset_id.clone(),
            fund_code: suggestion.fund_code.clone(),
            units: new_units,
            units_estimated: false,
            cost_basis: new_cost_basis,
            last_market_value: new_market_value,
            latest_nav: None,
            latest_nav_date: None,
            latest_nav_source: Some("reconciliation".to_string()),
            latest_nav_status: Some("估算".to_string()),
        });

        audit.new_units = new_units;
        audit.new_cost_basis = new_cost_basis;
        audit.new_market_value = new_market_value;
    }

    audit
}
