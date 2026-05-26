use crate::engine::decision::DecisionResult;
use crate::engine::kelly::calculate_single_asset_kelly;
use crate::models::{
    AdjustedDecisionItem, AdjustedDecisionPreview, ConfigRoot, GlobalRiskOverlay,
    MarketRegimeResult, PortfolioState,
};
use chrono::Local;

pub fn calculate_adjusted_decision(
    config: &ConfigRoot,
    state: &PortfolioState,
    decision: &DecisionResult,
    risk_overlay: &GlobalRiskOverlay,
    regimes: &std::collections::HashMap<String, MarketRegimeResult>,
) -> AdjustedDecisionPreview {
    let mut items = Vec::new();
    let mut adjusted_total_buy = 0.0;
    let mut warnings = Vec::new();

    for sector in &decision.sector_suggestions {
        for asset in &sector.asset_suggestions {
            let regime = regimes.get(&asset.asset_id);
            let item = calculate_single_adjusted_item(
                config,
                state,
                asset.asset_id.clone(),
                asset.fund_code.clone(),
                asset.fund_name.clone(),
                asset.sector_name.clone(),
                asset.suggested_buy,
                risk_overlay,
                regime,
            );
            adjusted_total_buy += item.capped_adjusted_buy;
            items.push(item);
        }
    }

    // Portfolio level caps
    let max_total_mult = config.adjusted_decision.max_total_adjusted_buy_multiplier;
    if adjusted_total_buy > decision.suggested_total_buy * max_total_mult {
        let cap = decision.suggested_total_buy * max_total_mult;
        if adjusted_total_buy > 0.0 {
            let ratio = cap / adjusted_total_buy;
            for item in &mut items {
                item.capped_adjusted_buy *= ratio;
            }
            adjusted_total_buy = cap;
            warnings.push(format!(
                "组合总调整买入量触发上限倍率 ({:.2}x)，已等比缩放。",
                max_total_mult
            ));
        }
    }

    let total_multiplier = if decision.suggested_total_buy > 0.0 {
        adjusted_total_buy / decision.suggested_total_buy
    } else {
        1.0
    };

    AdjustedDecisionPreview {
        available_cash: decision.available_cash,
        target_equity_value: decision.target_equity_value,
        current_equity_value: decision.current_equity_value,
        equity_gap: decision.equity_gap,
        max_daily_buy: decision.max_daily_buy_total,
        base_total_buy: decision.suggested_total_buy,
        adjusted_total_buy,
        total_multiplier,
        global_risk_score: risk_overlay.risk_score,
        global_risk_label: risk_overlay.risk_label.clone(),
        items,
        warnings,
    }
}

pub fn calculate_single_adjusted_item(
    config: &ConfigRoot,
    state: &PortfolioState,
    asset_id: String,
    fund_code: String,
    fund_name: String,
    sector: String,
    base_suggested_buy: f64,
    risk_overlay: &GlobalRiskOverlay,
    regime: Option<&MarketRegimeResult>,
) -> AdjustedDecisionItem {
    let mut warnings = Vec::new();
    let mut status = "正常".to_string();
    let mut explanation_parts = Vec::new();

    let pendulum_score = regime.map(|r| r.pendulum_score).unwrap_or(0.0);
    let regime_label = regime.map(|r| r.regime_label.as_str()).unwrap_or("未知");

    // 1. Regime multiplier
    let regime_multiplier = if let Some(r) = regime {
        match r.regime_label.as_str() {
            "极冷" => 1.5,
            "偏冷" => 1.2,
            "中性" => 1.0,
            "偏热" => 0.5,
            "过热" => 0.2,
            _ => 1.0,
        }
    } else {
        warnings.push("缺少市场冷热数据，已降低建议买入。".to_string());
        config.adjusted_decision.missing_regime_multiplier
    };
    explanation_parts.push(format!(
        "周期倍率 ({}): {:.2}",
        regime_label, regime_multiplier
    ));

    // 2. Risk multiplier
    let risk_multiplier = match risk_overlay.risk_label.as_str() {
        "低风险" => 1.0,
        "正常" => 1.0,
        "偏高" => 0.7,
        "高风险" => 0.4,
        "极高风险" => 0.0,
        _ => 1.0,
    };
    explanation_parts.push(format!(
        "风险倍率 ({}): {:.2}",
        risk_overlay.risk_label, risk_multiplier
    ));

    if risk_overlay.risk_label == "极高风险" {
        status = "风险过高".to_string();
    }

    // 3. Kelly multiplier
    let kelly_res = calculate_single_asset_kelly(
        config,
        asset_id.clone(),
        fund_code.clone(),
        fund_name.clone(),
        sector.clone(),
        base_suggested_buy,
        risk_overlay,
        regime,
    );
    let kelly_multiplier = kelly_res.kelly_multiplier;
    explanation_parts.push(format!("Kelly倍率: {:.2}", kelly_multiplier));

    if kelly_res.status != "正常" && status == "正常" {
        status = kelly_res.status.clone();
    }

    if kelly_multiplier == 0.0 && status == "正常" {
        status = "暂停买入".to_string();
    }

    // 4. Data quality multiplier
    let mut data_quality_multiplier = 1.0;

    // Check fund NAV quality
    if let Some(holding) = state.asset_holdings.iter().find(|h| h.asset_id == asset_id) {
        // Mock check
        if let Some(nav_status) = &holding.latest_nav_status {
            if nav_status == "模拟" || nav_status == "mock" {
                data_quality_multiplier *= config.adjusted_decision.mock_data_multiplier;
                warnings.push("基金净值为模拟数据，已降低建议买入。".to_string());
                status = "使用模拟数据".to_string();
            }
        }

        // Stale check
        if let Some(nav_date) = &holding.latest_nav_date {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(nav_date, "%Y-%m-%d") {
                let today = Local::now().date_naive();
                let days_diff = (today - date).num_days();
                if days_diff > config.api.fund_nav_stale_days {
                    data_quality_multiplier *= config.adjusted_decision.stale_data_multiplier;
                    warnings.push(format!(
                        "基金净值已过期 ({}天)，已降低建议买入。",
                        days_diff
                    ));
                    if status == "正常" {
                        status = "数据过期".to_string();
                    }
                }
            }
        }
    }

    // Check regime mock
    if let Some(r) = regime {
        if r.source == "mock" && data_quality_multiplier > 0.0 {
            data_quality_multiplier *= config.adjusted_decision.mock_data_multiplier;
            warnings.push("市场周期为模拟数据，已降低建议买入。".to_string());
            if status == "正常" {
                status = "使用模拟数据".to_string();
            }
        }
    }

    explanation_parts.push(format!("数据质量倍率: {:.2}", data_quality_multiplier));

    // 5. Combined multiplier
    let mut combined_multiplier =
        regime_multiplier * risk_multiplier * kelly_multiplier * data_quality_multiplier;

    // Caps
    let max_mult = config.adjusted_decision.max_adjusted_multiplier;
    if combined_multiplier > max_mult {
        combined_multiplier = max_mult;
        warnings.push(format!("综合倍率触发上限 ({:.2}x)", max_mult));
    }

    if !config.adjusted_decision.allow_increase_above_base && combined_multiplier > 1.0 {
        combined_multiplier = 1.0;
        warnings.push("已配置不允许超过基础建议，倍率已限制为 1.0".to_string());
    }

    let adjusted_buy = base_suggested_buy * combined_multiplier;
    let capped_adjusted_buy = adjusted_buy;

    if combined_multiplier == 0.0 && status == "正常" {
        status = "暂停买入".to_string();
    }

    if base_suggested_buy <= 0.0 {
        status = "无需买入".to_string();
    }

    let explanation =
        explanation_parts.join(" * ") + &format!(" = 最终倍率: {:.2}", combined_multiplier);

    AdjustedDecisionItem {
        sector,
        asset_id,
        fund_code,
        fund_name,
        base_suggested_buy,
        regime_label: regime_label.to_string(),
        pendulum_score,
        regime_multiplier,
        global_risk_label: risk_overlay.risk_label.clone(),
        global_risk_score: risk_overlay.risk_score,
        risk_multiplier,
        kelly_multiplier,
        data_quality_multiplier,
        combined_multiplier,
        adjusted_buy,
        capped_adjusted_buy,
        status,
        warnings,
        explanation,
    }
}
