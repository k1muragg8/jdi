use crate::engine::adjusted_decision::calculate_adjusted_decision;
use crate::engine::calculate_portfolio_summary;
use crate::engine::decision::generate_buy_suggestions;
use crate::engine::kelly::calculate_kelly_preview;
use crate::models::{
    AssetDecisionExplanation, CapExplanation, ConfigRoot, DecisionExplanation, GlobalRiskOverlay,
    KellyAdjustmentExplanation, MarketRegimeResult, PortfolioState, RegimeAdjustmentExplanation,
    RiskAdjustmentExplanation, SectorAllocationExplanation,
};

pub fn explain_decision(
    config: &ConfigRoot,
    state: &PortfolioState,
    portfolio_id: String,
    date: String,
    risk_overlay: &GlobalRiskOverlay,
    regimes: &std::collections::HashMap<String, MarketRegimeResult>,
) -> DecisionExplanation {
    let summary = calculate_portfolio_summary(config, state);
    let base_decision = generate_buy_suggestions(config, state, date.clone());
    let kelly_preview = calculate_kelly_preview(config, &base_decision, risk_overlay, regimes);
    let adjusted_preview =
        calculate_adjusted_decision(config, state, &base_decision, risk_overlay, regimes);

    let mut asset_explanations = Vec::new();
    let mut sector_explanations = Vec::new();

    // 1. Sector Allocation Explanations
    for s_summary in &summary.sector_summaries {
        let allocated = base_decision
            .sector_suggestions
            .iter()
            .find(|s| s.sector_id == s_summary.sector_id)
            .map(|s| s.suggested_buy)
            .unwrap_or(0.0);

        sector_explanations.push(SectorAllocationExplanation {
            sector_id: s_summary.sector_id.clone(),
            sector_name: s_summary.sector_name.clone(),
            target_weight: s_summary.target_weight,
            current_weight: s_summary.current_weight,
            target_value: s_summary.target_value,
            current_value: s_summary.current_value,
            gap_value: s_summary.gap_value,
            priority: s_summary.priority,
            allocated_amount: allocated,
        });
    }

    // 2. Asset Decision Explanations
    for asset_config in &config.assets {
        let asset_id = &asset_config.asset_id;

        let base_suggestion = base_decision
            .sector_suggestions
            .iter()
            .flat_map(|s| &s.asset_suggestions)
            .find(|a| a.asset_id == *asset_id);

        let adjusted_item = adjusted_preview
            .items
            .iter()
            .find(|i| i.asset_id == *asset_id);

        let kelly_res = kelly_preview
            .results
            .iter()
            .find(|r| r.asset_id == *asset_id);

        let mut caps = Vec::new();
        let mut skip_reason = None;
        let mut status = "正常".to_string();

        if !asset_config.enabled {
            status = "已禁用".to_string();
            skip_reason = Some("资产在配置中已禁用".to_string());
        } else if base_suggestion.is_none() && asset_config.enabled {
            // Why was it skipped in base?
            let sector = summary
                .sector_summaries
                .iter()
                .find(|s| s.sector_name == asset_config.sector);
            if let Some(s) = sector {
                if s.gap_value <= 0.0 {
                    skip_reason = Some(format!("所属赛道 ({}) 暂无缺口", s.sector_name));
                } else if s.asset_class != "equity" {
                    skip_reason = Some(format!("所属赛道 ({}) 不是权益类", s.sector_name));
                } else {
                    skip_reason = Some("分配算法未覆盖此资产".to_string());
                }
            } else {
                skip_reason = Some("所属赛道未在配置中找到".to_string());
            }
            status = "跳过".to_string();
        }

        if let Some(item) = adjusted_item {
            status = item.status.clone();
            if item.base_suggested_buy > 0.0 && item.capped_adjusted_buy < item.adjusted_buy {
                caps.push(CapExplanation {
                    name: "组合总买入上限倍率".to_string(),
                    limit_value: config.adjusted_decision.max_total_adjusted_buy_multiplier,
                    applied: true,
                    description: format!(
                        "触发组合级别倍率上限 ({:.2}x)",
                        config.adjusted_decision.max_total_adjusted_buy_multiplier
                    ),
                });
            }
            if item.combined_multiplier > config.adjusted_decision.max_adjusted_multiplier {
                caps.push(CapExplanation {
                    name: "单资产综合倍率上限".to_string(),
                    limit_value: config.adjusted_decision.max_adjusted_multiplier,
                    applied: true,
                    description: format!(
                        "触发单资产倍率上限 ({:.2}x)",
                        config.adjusted_decision.max_adjusted_multiplier
                    ),
                });
            }
        }

        let regime_adj = adjusted_item
            .map(|i| RegimeAdjustmentExplanation {
                score: i.pendulum_score,
                label: i.regime_label.clone(),
                multiplier: i.regime_multiplier,
            })
            .unwrap_or_else(|| {
                let r = regimes.get(asset_id);
                RegimeAdjustmentExplanation {
                    score: r.map(|r| r.pendulum_score).unwrap_or(0.0),
                    label: r
                        .map(|r| r.regime_label.clone())
                        .unwrap_or_else(|| "未知".to_string()),
                    multiplier: 1.0,
                }
            });

        let risk_adj = adjusted_item
            .map(|i| RiskAdjustmentExplanation {
                score: i.global_risk_score,
                label: i.global_risk_label.clone(),
                multiplier: i.risk_multiplier,
                factors: Vec::new(), // Would need to extract from overlay
            })
            .unwrap_or_else(|| RiskAdjustmentExplanation {
                score: risk_overlay.risk_score,
                label: risk_overlay.risk_label.clone(),
                multiplier: 1.0,
                factors: Vec::new(),
            });

        let kelly_adj = kelly_res
            .map(|k| KellyAdjustmentExplanation {
                win_probability: k.estimated_win_probability,
                payoff_ratio: k.payoff_ratio,
                raw_kelly: k.raw_kelly_fraction,
                adjusted_kelly: k.fractional_kelly_fraction,
                multiplier: k.kelly_multiplier,
                status: k.status.clone(),
            })
            .unwrap_or_else(|| KellyAdjustmentExplanation {
                win_probability: 0.5,
                payoff_ratio: 1.0,
                raw_kelly: 0.0,
                adjusted_kelly: 0.0,
                multiplier: 1.0,
                status: "N/A".to_string(),
            });

        let base_buy = base_suggestion.map(|s| s.suggested_buy).unwrap_or(0.0);
        let adj_buy = adjusted_item.map(|i| i.capped_adjusted_buy).unwrap_or(0.0);

        let mut summary_text = if status == "已禁用" {
            "资产已禁用".to_string()
        } else if status == "跳过" {
            skip_reason
                .clone()
                .unwrap_or_else(|| "由于缺口或现金原因被跳过".to_string())
        } else if adj_buy > 0.0 {
            format!("建议买入 {:.2} {}", adj_buy, config.portfolio.base_currency)
        } else {
            format!("状态: {}", status)
        };

        if let Some(item) = adjusted_item {
            if !item.warnings.is_empty() {
                summary_text += &format!(" (注意: {})", item.warnings.join(", "));
            }
        }

        asset_explanations.push(AssetDecisionExplanation {
            asset_id: asset_id.clone(),
            fund_code: asset_config.fund_code.clone(),
            fund_name: asset_config.fund_name.clone(),
            sector_id: asset_config.sector.clone(),
            status,
            base_suggested_buy: base_buy,
            adjusted_suggested_buy: adj_buy,
            final_suggested_buy: adj_buy, // Current final
            regime_adjustment: regime_adj,
            risk_adjustment: risk_adj,
            kelly_adjustment: kelly_adj,
            data_quality_multiplier: adjusted_item
                .map(|i| i.data_quality_multiplier)
                .unwrap_or(1.0),
            caps,
            skip_reason,
            summary: summary_text,
        });
    }

    let mut global_caps = Vec::new();

    for w in &adjusted_preview.warnings {
        if w.contains("触发上限倍率") {
            global_caps.push(CapExplanation {
                name: "总买入倍率上限".to_string(),
                limit_value: config.adjusted_decision.max_total_adjusted_buy_multiplier,
                applied: true,
                description: w.clone(),
            });
        }
    }

    DecisionExplanation {
        date: date.clone(),
        portfolio_id,
        base_currency: config.portfolio.base_currency.clone(),
        available_cash: summary.available_cash,
        daily_budget: config.portfolio.max_daily_buy_total,
        target_equity_value: summary.target_equity_value,
        current_equity_value: summary.equity_value,
        equity_gap: summary.equity_gap,
        risk_summary: RiskAdjustmentExplanation {
            score: risk_overlay.risk_score,
            label: risk_overlay.risk_label.clone(),
            multiplier: 1.0, // Global doesn't have a multiplier itself
            factors: risk_overlay
                .factor_results
                .iter()
                .map(|f| format!("{}: {:.2}", f.name, f.latest_value))
                .collect(),
        },
        asset_explanations,
        sector_explanations,
        warnings: adjusted_preview.warnings.clone(),
        global_caps,
    }
}
