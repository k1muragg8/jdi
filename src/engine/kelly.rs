use crate::engine::decision::DecisionResult;
use crate::models::{
    ConfigRoot, GlobalRiskOverlay, KellyPortfolioPreview, KellyPreviewResult, MarketRegimeResult,
};

pub fn calculate_kelly_preview(
    config: &ConfigRoot,
    decision: &DecisionResult,
    risk_overlay: &GlobalRiskOverlay,
    regimes: &std::collections::HashMap<String, MarketRegimeResult>,
) -> KellyPortfolioPreview {
    let mut results = Vec::new();
    let mut preview_total_buy = 0.0;
    let mut warnings = Vec::new();

    for sector in &decision.sector_suggestions {
        for asset in &sector.asset_suggestions {
            let regime = regimes.get(&asset.asset_id);
            let kelly_res = calculate_single_asset_kelly(KellyContext {
                config,
                asset_id: asset.asset_id.clone(),
                fund_code: asset.fund_code.clone(),
                fund_name: asset.fund_name.clone(),
                sector: asset.sector_name.clone(),
                base_suggested_buy: asset.suggested_buy,
                risk_overlay,
                regime,
            });
            preview_total_buy += kelly_res.capped_preview_buy_amount;
            results.push(kelly_res);
        }
    }

    // Portfolio level caps
    if preview_total_buy > decision.suggested_total_buy * config.kelly.max_total_buy_multiplier {
        let cap = decision.suggested_total_buy * config.kelly.max_total_buy_multiplier;
        if preview_total_buy > 0.0 {
            let ratio = cap / preview_total_buy;
            for res in &mut results {
                res.capped_preview_buy_amount *= ratio;
            }
            preview_total_buy = cap;
            warnings.push(format!(
                "组合总预览买入量触发上限倍率 ({:.2}x)，已等比缩放。",
                config.kelly.max_total_buy_multiplier
            ));
        }
    }

    let total_multiplier = if decision.suggested_total_buy > 0.0 {
        preview_total_buy / decision.suggested_total_buy
    } else {
        1.0
    };

    KellyPortfolioPreview {
        base_total_buy: decision.suggested_total_buy,
        preview_total_buy,
        total_multiplier,
        global_risk_score: risk_overlay.risk_score,
        global_risk_label: risk_overlay.risk_label.clone(),
        results,
        warnings,
    }
}

pub struct KellyContext<'a> {
    pub config: &'a ConfigRoot,
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub sector: String,
    pub base_suggested_buy: f64,
    pub risk_overlay: &'a GlobalRiskOverlay,
    pub regime: Option<&'a MarketRegimeResult>,
}

pub fn calculate_single_asset_kelly(ctx: KellyContext) -> KellyPreviewResult {
    let KellyContext {
        config,
        asset_id,
        fund_code,
        fund_name,
        sector,
        base_suggested_buy,
        risk_overlay,
        regime,
    } = ctx;

    let mut warnings = Vec::new();
    let mut status = "正常".to_string();
    let mut explanation_steps = Vec::new();

    let pendulum_score = regime.map(|r| r.pendulum_score).unwrap_or(0.0);
    let regime_label = regime.map(|r| r.regime_label.as_str()).unwrap_or("未知");
    let mut volatility = 0.0;
    let mut drawdown = 0.0;

    if let Some(r) = regime {
        if let Some(stats) = r.windows.iter().find(|w| w.window_days == 250) {
            volatility = stats.annualized_volatility;
            drawdown = stats.drawdown;
        } else if !r.windows.is_empty() {
            volatility = r.windows[0].annualized_volatility;
            drawdown = r.windows[0].drawdown;
        }
    } else {
        status = "数据不足".to_string();
        warnings.push("缺失市场周期数据，使用保守估算。".to_string());
    }

    // 1. Estimate win probability p
    let mut p: f64 = 0.50;
    explanation_steps.push(format!("初始胜率估算 p = {:.2}", p));

    // Regime adjustment
    let regime_adj = if pendulum_score <= -60.0 {
        0.08 // Extreme Cold
    } else if pendulum_score <= -20.0 {
        0.04 // Cold
    } else if pendulum_score >= 60.0 {
        -0.08 // Overheated
    } else if pendulum_score >= 20.0 {
        -0.04 // Hot
    } else {
        0.0 // Neutral
    };
    p += regime_adj;
    if regime_adj != 0.0 {
        explanation_steps.push(format!(
            "市场周期调节 ({}) : {:+.2}",
            regime_label, regime_adj
        ));
    }

    // Global Risk adjustment
    let risk_adj = if risk_overlay.risk_score >= 80.0 {
        -0.15
    } else if risk_overlay.risk_score >= 60.0 {
        -0.10
    } else if risk_overlay.risk_score >= 40.0 {
        -0.05
    } else {
        0.0
    };
    p += risk_adj;
    if risk_adj != 0.0 {
        explanation_steps.push(format!(
            "全局风险调节 ({}) : {:+.2}",
            risk_overlay.risk_label, risk_adj
        ));
    }

    // Volatility adjustment
    if volatility > 0.40 {
        p -= 0.05;
        explanation_steps.push("高波动率调节: -0.05".to_string());
    }

    p = p.clamp(0.35, 0.60);
    explanation_steps.push(format!("最终胜率 p = {:.2}", p));

    // 2. Estimate payoff ratio b
    let mut b = 1.0;
    explanation_steps.push(format!("初始赔率估算 b = {:.2}", b));

    if drawdown < -0.20 {
        let dd_adj = ((-drawdown - 0.20) * 0.5).min(0.5);
        b += dd_adj;
        explanation_steps.push(format!("大幅回撤调节 (提升赔率) : {:+.2}", dd_adj));
    }

    if pendulum_score >= 50.0 {
        b -= 0.2;
        explanation_steps.push("市场高位调节 (降低赔率) : -0.20".to_string());
    }

    b = b.clamp(0.5, 1.5);
    explanation_steps.push(format!("最终赔率 b = {:.2}", b));

    // 3. Calculate Raw Kelly
    // f* = p - (1 - p) / b
    let raw_kelly = p - (1.0 - p) / b;
    explanation_steps.push(format!("原始 Kelly 分数 f* = {:.4}", raw_kelly));

    // 4. Fractional Kelly
    let fractional_kelly_fraction = if raw_kelly > 0.0 {
        raw_kelly * config.kelly.fractional_kelly
    } else {
        0.0
    };
    explanation_steps.push(format!(
        "分段 Kelly ({:.2}x) = {:.4}",
        config.kelly.fractional_kelly, fractional_kelly_fraction
    ));

    // 5. Multiplier calculation
    // Map fractional_kelly to multiplier
    // 0.0 -> neutral_multiplier (usually 1.0)
    // Here we need to define how Kelly fraction translates to multiplier.
    // If raw_kelly is 0, multiplier should be small or 0 if risk is high.

    let mut multiplier = config.kelly.neutral_multiplier;

    if raw_kelly < 0.0 {
        multiplier = config.kelly.min_multiplier;
        status = "暂停买入".to_string();

        // Even if Kelly is negative, we still want to know WHY we are pausing
        if risk_overlay.risk_score >= 80.0 {
            status = "风险过高".to_string();
        } else if regime_label == "过热" {
            status = "市场过热".to_string();
        }
    } else {
        // Adjust multiplier based on regime and risk
        if regime_label == "过热" {
            multiplier = config.kelly.overheated_market_multiplier;
            status = "市场过热".to_string();
        } else if regime_label == "偏热" {
            multiplier = config.kelly.hot_market_multiplier;
            status = "市场偏热".to_string();
        } else if regime_label == "极冷" {
            multiplier = config.kelly.extreme_cold_market_multiplier;
        } else if regime_label == "偏冷" {
            multiplier = config.kelly.cold_market_multiplier;
        }

        // Global risk caps
        if risk_overlay.risk_score >= 80.0 {
            multiplier = config.kelly.extreme_risk_multiplier;
            status = "风险过高".to_string();
        } else if risk_overlay.risk_score >= 60.0 {
            multiplier = multiplier.min(config.kelly.high_risk_multiplier);
            if multiplier < 1.0 {
                status = "风险过高".to_string();
            }
        }

        // Apply Kelly fraction as a boost if multiplier > 0
        if multiplier > 0.0 && raw_kelly > 0.0 {
            // If we are in cold market, Kelly can boost further
            let boost = 1.0 + (fractional_kelly_fraction * 2.0); // Simple boost
            multiplier *= boost;
        }
    }

    multiplier = multiplier.clamp(config.kelly.min_multiplier, config.kelly.max_multiplier);

    let preview_buy_amount = base_suggested_buy * multiplier;
    let mut capped_preview_buy_amount = preview_buy_amount;

    if multiplier > config.kelly.max_single_asset_buy_multiplier {
        capped_preview_buy_amount =
            base_suggested_buy * config.kelly.max_single_asset_buy_multiplier;
        warnings.push(format!(
            "单资产预览倍率触发上限 ({:.2}x)",
            config.kelly.max_single_asset_buy_multiplier
        ));
    }

    let confidence = (p - 0.35) / (0.60 - 0.35); // Normalize p to [0, 1] as confidence

    if confidence < config.kelly.min_confidence && status == "正常" {
        status = "信心不足".to_string();
    }

    let explanation = explanation_steps.join(" -> ")
        + &format!(
            " -> 基础倍率调节 -> 最终倍率: {:.2} (上限 {:.2})",
            multiplier, config.kelly.max_single_asset_buy_multiplier
        );

    KellyPreviewResult {
        asset_id,
        fund_code,
        fund_name,
        sector,
        base_suggested_buy,
        pendulum_score,
        market_regime_label: regime_label.to_string(),
        global_risk_score: risk_overlay.risk_score,
        global_risk_label: risk_overlay.risk_label.clone(),
        volatility,
        drawdown,
        expected_edge: (p * b) - (1.0 - p),
        estimated_win_probability: p,
        payoff_ratio: b,
        raw_kelly_fraction: raw_kelly,
        fractional_kelly_fraction,
        kelly_multiplier: multiplier,
        preview_buy_amount,
        capped_preview_buy_amount,
        confidence,
        status,
        warnings,
        explanation,
    }
}
