use super::portfolio_summary::calculate_portfolio_summary;
use crate::models::{ConfigRoot, PortfolioState};

#[derive(Debug, Clone)]
pub struct AssetBuySuggestion {
    pub asset_id: String,
    pub fund_code: String,
    pub fund_name: String,
    pub sector_name: String,
    pub current_value: f64,
    pub suggested_buy: f64,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct SectorBuySuggestion {
    pub sector_id: String,
    pub sector_name: String,
    pub target_value: f64,
    pub current_value: f64,
    pub gap_value: f64,
    pub gap_ratio: f64,
    pub priority: i32,
    pub suggested_buy: f64,
    pub asset_suggestions: Vec<AssetBuySuggestion>,
}

#[derive(Debug, Clone)]
pub struct DecisionResult {
    pub date: String,
    pub available_cash: f64,
    pub target_equity_value: f64,
    pub current_equity_value: f64,
    pub equity_gap: f64,
    pub max_daily_buy_total: f64,
    pub suggested_total_buy: f64,
    pub sector_suggestions: Vec<SectorBuySuggestion>,
    pub warnings: Vec<String>,
}

pub fn generate_buy_suggestions(
    config: &ConfigRoot,
    state: &PortfolioState,
    date: String,
) -> DecisionResult {
    let summary = calculate_portfolio_summary(config, state);
    let mut warnings = Vec::new();
    let mut suggested_total_buy = 0.0;
    let mut sector_suggestions = Vec::new();

    let mut res = DecisionResult {
        date,
        available_cash: summary.available_cash,
        target_equity_value: summary.target_equity_value,
        current_equity_value: summary.equity_value,
        equity_gap: summary.equity_gap,
        max_daily_buy_total: config.portfolio.max_daily_buy_total,
        suggested_total_buy,
        sector_suggestions: Vec::new(),
        warnings: Vec::new(),
    };

    if summary.available_cash <= 0.0 {
        warnings.push("可用现金小于等于 0，今日不建议买入".to_string());
        res.warnings = warnings;
        return res;
    }

    if summary.equity_gap <= 0.0 {
        warnings.push("当前权益仓已经达到或超过目标权益仓".to_string());
        res.warnings = warnings;
        return res;
    }

    let max_buy = config.portfolio.max_daily_buy_total;
    let mut daily_budget = summary.available_cash;
    if summary.equity_gap < daily_budget {
        daily_budget = summary.equity_gap;
    }
    if max_buy > 0.0 && max_buy < daily_budget {
        daily_budget = max_buy;
    }

    if daily_budget <= 0.0 {
        res.warnings = warnings;
        return res;
    }

    #[derive(Debug)]
    struct AllocationSector {
        name: String,
        score: f64,
        gap: f64,
        id: String,
        target_val: f64,
        curr_val: f64,
        gap_ratio: f64,
        priority: i32,
    }

    let mut allocation_sectors = Vec::new();
    let mut total_score = 0.0;

    for s_summary in summary.sector_summaries {
        if !s_summary.enabled || s_summary.asset_class != "equity" || s_summary.gap_value <= 0.0 {
            continue;
        }

        // Check if there is at least one enabled asset for this sector
        let has_enabled_asset = config
            .assets
            .iter()
            .any(|a| a.sector == s_summary.sector_name && a.enabled);
        if !has_enabled_asset {
            continue;
        }

        let priority_factor = match s_summary.priority {
            1 => 1.0,
            2 => 0.8,
            3 => 0.6,
            4 => 0.4,
            _ => 0.2,
        };

        let sector_score = s_summary.gap_ratio * priority_factor;
        total_score += sector_score;

        allocation_sectors.push(AllocationSector {
            name: s_summary.sector_name.clone(),
            score: sector_score,
            gap: s_summary.gap_value,
            id: s_summary.sector_id.clone(),
            target_val: s_summary.target_value,
            curr_val: s_summary.current_value,
            gap_ratio: s_summary.gap_ratio,
            priority: s_summary.priority,
        });
    }

    if total_score <= 0.0 {
        res.warnings = warnings;
        return res;
    }

    let min_buy = config.risk.min_buy_amount;
    let max_single_sector = config.risk.max_single_sector_daily_buy;
    let max_single_asset = config.risk.max_single_asset_daily_buy;

    let mut remaining_budget = daily_budget;

    // Initialize allocations for assets
    #[derive(Debug)]
    struct AssetAllocation {
        asset_id: String,
        fund_code: String,
        fund_name: String,
        sector_name: String,
        current_value: f64,
        allocated: f64,
        capped: bool,
    }

    let mut all_assets = Vec::new();

    for sec in &allocation_sectors {
        let enabled_assets: Vec<_> = config
            .assets
            .iter()
            .filter(|a| a.sector == sec.name && a.enabled)
            .collect();

        for asset in enabled_assets {
            let mut c_val = 0.0;
            if let Some(holding) = state
                .asset_holdings
                .iter()
                .find(|h| h.asset_id == asset.asset_id)
            {
                c_val = holding.last_market_value;
            }

            all_assets.push(AssetAllocation {
                asset_id: asset.asset_id.clone(),
                fund_code: asset.fund_code.clone(),
                fund_name: asset.fund_name.clone(),
                sector_name: sec.name.clone(),
                current_value: c_val,
                allocated: 0.0,
                capped: false,
            });
        }
    }

    // Water-filling allocation
    let mut changed = true;
    while remaining_budget > 0.01 && changed {
        changed = false;

        // 1. Calculate active sector scores
        let mut active_sector_scores = std::collections::HashMap::new();
        let mut active_total_score = 0.0;

        for sec in &allocation_sectors {
            let mut sec_capped = true;
            // A sector is active if it has uncapped assets and its own sum is below its sector cap and gap
            let current_sec_alloc: f64 = all_assets
                .iter()
                .filter(|a| a.sector_name == sec.name)
                .map(|a| a.allocated)
                .sum();

            let mut has_uncapped_assets = false;
            for a in &all_assets {
                if a.sector_name == sec.name && !a.capped {
                    has_uncapped_assets = true;
                    break;
                }
            }

            if has_uncapped_assets
                && current_sec_alloc < sec.gap
                && (max_single_sector <= 0.0 || current_sec_alloc < max_single_sector)
            {
                sec_capped = false;
            }

            if !sec_capped {
                active_sector_scores.insert(sec.name.clone(), sec.score);
                active_total_score += sec.score;
            }
        }

        if active_total_score <= 0.0 {
            break; // No active sectors left to distribute to
        }

        let loop_budget = remaining_budget;

        // 2. Distribute to sectors
        for sec in &allocation_sectors {
            if let Some(&sec_score) = active_sector_scores.get(&sec.name) {
                let proportion = sec_score / active_total_score;
                let mut sector_budget = loop_budget * proportion;

                let current_sec_alloc: f64 = all_assets
                    .iter()
                    .filter(|a| a.sector_name == sec.name)
                    .map(|a| a.allocated)
                    .sum();

                // Cap sector budget
                if sector_budget > sec.gap - current_sec_alloc {
                    sector_budget = sec.gap - current_sec_alloc;
                }
                if max_single_sector > 0.0 && sector_budget > max_single_sector - current_sec_alloc
                {
                    sector_budget = max_single_sector - current_sec_alloc;
                }

                // 3. Distribute to assets within sector
                let mut uncapped_assets: Vec<_> = all_assets
                    .iter_mut()
                    .filter(|a| a.sector_name == sec.name && !a.capped)
                    .collect();

                if uncapped_assets.is_empty() || sector_budget <= 0.0 {
                    continue;
                }

                let asset_count = uncapped_assets.len() as f64;
                let per_asset = sector_budget / asset_count;

                for asset in uncapped_assets.iter_mut() {
                    let mut increment = per_asset;

                    if max_single_asset > 0.0 && asset.allocated + increment > max_single_asset {
                        increment = max_single_asset - asset.allocated;
                        asset.capped = true;
                    }

                    if increment > 0.0 {
                        asset.allocated += increment;
                        remaining_budget -= increment;
                        changed = true;
                    } else if increment <= 0.0 && !asset.capped {
                        asset.capped = true;
                        changed = true;
                    }
                }

                let new_sec_alloc: f64 = all_assets
                    .iter()
                    .filter(|a| a.sector_name == sec.name)
                    .map(|a| a.allocated)
                    .sum();

                if new_sec_alloc >= sec.gap - 0.01
                    || (max_single_sector > 0.0 && new_sec_alloc >= max_single_sector - 0.01)
                {
                    for a in all_assets.iter_mut() {
                        if a.sector_name == sec.name {
                            a.capped = true;
                        }
                    }
                    changed = true;
                }
            }
        }
    }

    // Process output
    for sec in allocation_sectors {
        let mut asset_suggestions = Vec::new();
        let mut actual_sector_allocated = 0.0;

        for asset in all_assets.iter().filter(|a| a.sector_name == sec.name) {
            let mut final_asset_buy = asset.allocated;

            if final_asset_buy < min_buy {
                final_asset_buy = 0.0;
            }

            if final_asset_buy > 0.0 {
                actual_sector_allocated += final_asset_buy;
                suggested_total_buy += final_asset_buy;

                asset_suggestions.push(AssetBuySuggestion {
                    asset_id: asset.asset_id.clone(),
                    fund_code: asset.fund_code.clone(),
                    fund_name: asset.fund_name.clone(),
                    sector_name: sec.name.clone(),
                    current_value: asset.current_value,
                    suggested_buy: final_asset_buy,
                    reason: format!("低配赛道，优先级 {}", sec.priority),
                });
            }
        }

        if actual_sector_allocated > 0.0 {
            sector_suggestions.push(SectorBuySuggestion {
                sector_id: sec.id,
                sector_name: sec.name,
                target_value: sec.target_val,
                current_value: sec.curr_val,
                gap_value: sec.gap,
                gap_ratio: sec.gap_ratio,
                priority: sec.priority,
                suggested_buy: actual_sector_allocated,
                asset_suggestions,
            });
        }
    }

    res.suggested_total_buy = suggested_total_buy;
    res.sector_suggestions = sector_suggestions;
    res.warnings = warnings;

    res
}
