use crate::models::{
    AlipayHoldingCandidate, AlipayHoldingImportPreview, AlipaySnapshot, ConfigRoot, PortfolioState,
};
use anyhow::Result;
use chrono::Local;
use csv::ReaderBuilder;
use std::io::Cursor;

pub fn parse_alipay_holdings_from_csv(csv_content: &str) -> Result<Vec<AlipayHoldingCandidate>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(Cursor::new(csv_content));

    let headers = rdr.headers()?.clone();

    let mut candidates = Vec::new();

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };

        let mut candidate = AlipayHoldingCandidate::default();

        for (i, header) in headers.iter().enumerate() {
            let val = record.get(i).unwrap_or("").trim();
            if val.is_empty() {
                continue;
            }

            match header {
                "fund_code" | "基金代码" | "代码" => candidate.fund_code = val.to_string(),
                "fund_name" | "基金名称" | "名称" => candidate.fund_name = val.to_string(),
                "shares" | "持有份额" | "份额" | "份额(份)" => {
                    candidate.units = val.replace(',', "").parse().unwrap_or(0.0)
                }
                "market_value" | "市值" | "市值(元)" | "持有金额" => {
                    candidate.market_value = val.replace(',', "").parse().unwrap_or(0.0)
                }
                "nav" | "单位净值" | "净值" => {
                    candidate.nav = val.replace(',', "").parse().ok()
                }
                "nav_date" | "净值日期" => candidate.nav_date = Some(val.to_string()),
                "cost_basis" | "成本价" | "成本" => {
                    candidate.cost_basis = val.replace(',', "").parse().ok()
                }
                "holding_profit" | "total_profit" | "累计收益" | "持有收益" => {
                    candidate.total_profit = val.replace(',', "").parse().ok()
                }
                "holding_profit_rate" | "profit_rate" | "持有收益率" => {
                    candidate.profit_rate = val.replace('%', "").replace(',', "").parse().ok()
                }
                "source" | "来源" => candidate.source = Some(val.to_string()),
                _ => {}
            }
        }

        // Row is valid if either fund_code or fund_name is present
        if !candidate.fund_code.is_empty() || !candidate.fund_name.is_empty() {
            candidates.push(candidate);
        }
    }

    Ok(candidates)
}

pub fn preview_alipay_holdings(
    config: &ConfigRoot,
    state: &PortfolioState,
    candidates: Vec<AlipayHoldingCandidate>,
    snapshot_date: &str,
) -> AlipayHoldingImportPreview {
    let mut matched_asset_ids = Vec::new();
    let mut system_units = Vec::new();
    let mut system_market_values = Vec::new();
    let mut unit_diffs = Vec::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    let mut valid_rows = 0;
    let invalid_rows = 0;
    let mut unmatched_rows = 0;

    for candidate in &candidates {
        let mut row_warnings = Vec::new();
        let row_errors = Vec::new();

        // Improved matching logic:
        // 1. fund_code (exact)
        // 2. asset_id (exact)
        // 3. fund_name (exact)
        let asset_config = if !candidate.fund_code.is_empty() {
            config
                .assets
                .iter()
                .find(|a| a.fund_code == candidate.fund_code)
                .or_else(|| config.assets.iter().find(|a| a.asset_id == candidate.fund_code))
        } else {
            None
        }
        .or_else(|| {
            if !candidate.fund_name.is_empty() {
                config
                    .assets
                    .iter()
                    .find(|a| a.fund_name == candidate.fund_name)
                    .or_else(|| config.assets.iter().find(|a| a.asset_id == candidate.fund_name))
            } else {
                None
            }
        });

        if let Some(ac) = asset_config {
            matched_asset_ids.push(Some(ac.asset_id.clone()));
            valid_rows += 1;

            // Find in state
            let holding = state
                .asset_holdings
                .iter()
                .find(|h| h.asset_id == ac.asset_id);
            if let Some(h) = holding {
                system_units.push(Some(h.units));
                system_market_values.push(Some(h.last_market_value));

                if candidate.units > 0.0 {
                    unit_diffs.push(Some(candidate.units - h.units));
                    if (candidate.units - h.units).abs() > 0.0001 {
                        row_warnings.push(format!(
                            "份额不匹配: 差额 {:.4} (Alipay: {:.4}, 系统: {:.4})",
                            candidate.units - h.units,
                            candidate.units,
                            h.units
                        ));
                    }
                } else {
                    unit_diffs.push(None);
                    // If screenshot only has market_value, we might want to warn if it's very different
                    let mkt_diff = candidate.market_value - h.last_market_value;
                    if mkt_diff.abs() > 1.0 {
                        row_warnings.push(format!(
                            "市值存在差异: {:.2} (Alipay: {:.2}, 系统: {:.2})",
                            mkt_diff, candidate.market_value, h.last_market_value
                        ));
                    }
                }
            } else {
                system_units.push(Some(0.0));
                system_market_values.push(Some(0.0));
                if candidate.units > 0.0 {
                    unit_diffs.push(Some(candidate.units));
                    row_warnings.push("系统中无此资产持仓".to_string());
                } else {
                    unit_diffs.push(None);
                    row_warnings.push(format!(
                        "系统中无此资产持仓 (Alipay市值: {:.2})",
                        candidate.market_value
                    ));
                }
            }
        } else {
            matched_asset_ids.push(None);
            system_units.push(None);
            system_market_values.push(None);
            unit_diffs.push(None);
            unmatched_rows += 1;
            // Unmatched is a warning, not an error
            row_warnings.push(format!(
                "未找到匹配的资产配置: {} {}",
                candidate.fund_code, candidate.fund_name
            ));
        }

        warnings.push(row_warnings);
        errors.push(row_errors);
    }

    AlipayHoldingImportPreview {
        snapshot_date: snapshot_date.to_string(),
        total_rows: candidates.len(),
        candidates,
        matched_asset_ids,
        system_units,
        system_market_values,
        unit_diffs,
        warnings,
        errors,
        valid_rows,
        invalid_rows,
        unmatched_rows,
    }
}

pub fn convert_to_snapshots(preview: &AlipayHoldingImportPreview) -> Vec<AlipaySnapshot> {
    let mut snapshots = Vec::new();
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    for (i, candidate) in preview.candidates.iter().enumerate() {
        // Skip only if there are actual errors (not warnings)
        if !preview.errors[i].is_empty() {
            continue;
        }

        let asset_id = preview.matched_asset_ids[i]
            .clone()
            .unwrap_or_else(|| "".to_string());

        snapshots.push(AlipaySnapshot {
            snapshot_id: format!(
                "snap_{}_{}_{}",
                if asset_id.is_empty() {
                    format!("unmatched_{}", i)
                } else {
                    asset_id.clone()
                },
                preview.snapshot_date,
                Local::now().timestamp_millis() + (i as i64)
            ),
            asset_id,
            fund_code: candidate.fund_code.clone(),
            fund_name: candidate.fund_name.clone(),
            snapshot_date: preview.snapshot_date.clone(),
            market_value: candidate.market_value,
            units: if candidate.units > 0.0 {
                Some(candidate.units)
            } else {
                None
            },
            cost_basis: candidate.cost_basis,
            nav: candidate.nav,
            nav_date: candidate.nav_date.clone(),
            daily_pnl: None,
            total_pnl: candidate.total_profit,
            source: candidate
                .source
                .clone()
                .unwrap_or_else(|| "alipay_import".to_string()),
            created_at: now.clone(),
            note: Some("Imported from CSV".to_string()),
        });
    }
    snapshots
}

pub fn bootstrap_assets_from_holdings(
    config: &mut ConfigRoot,
    candidates: &[AlipayHoldingCandidate],
) -> (usize, usize, usize) {
    use crate::models::AssetConfig;

    let mut created = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for candidate in candidates {
        if candidate.fund_code.is_empty() && candidate.fund_name.is_empty() {
            failed += 1;
            continue;
        }

        // Check if already exists
        let exists = if !candidate.fund_code.is_empty() {
            config
                .assets
                .iter()
                .any(|a| a.fund_code == candidate.fund_code || a.asset_id == candidate.fund_code)
        } else {
            config
                .assets
                .iter()
                .any(|a| a.fund_name == candidate.fund_name || a.asset_id == candidate.fund_name)
        };

        if exists {
            skipped += 1;
            continue;
        }

        // Create new AssetConfig
        let asset_id = if !candidate.fund_code.is_empty() {
            format!("fund_{}", candidate.fund_code)
        } else {
            // Slugify fund name if no code
            candidate
                .fund_name
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        };

        // Double check asset_id uniqueness
        if config.assets.iter().any(|a| a.asset_id == asset_id) {
            skipped += 1;
            continue;
        }

        let mut new_asset = AssetConfig::default();
        new_asset.asset_id = asset_id;
        new_asset.fund_code = candidate.fund_code.clone();
        new_asset.fund_name = candidate.fund_name.clone();
        new_asset.sector = "未分类".to_string(); // Uncategorized
        new_asset.currency = "CNY".to_string();
        new_asset.valuation_method = "nav".to_string();
        new_asset.market_data_provider = Some("eastmoney".to_string());
        new_asset.enabled = true;

        config.assets.push(new_asset);
        created += 1;
    }

    (created, skipped, failed)
}

pub fn preview_bootstrap_local(
    config: &ConfigRoot,
    state: &PortfolioState,
    candidates: &[AlipayHoldingCandidate],
    nav_cache: &std::collections::HashMap<String, crate::models::FundNav>,
    replace_existing: bool,
) -> crate::models::BootstrapLocalPreview {
    let mut rows = Vec::new();
    let mut total_bootstrapped_value = 0.0;

    for candidate in candidates {
        // Find asset
        let asset_config = if !candidate.fund_code.is_empty() {
            config
                .assets
                .iter()
                .find(|a| a.fund_code == candidate.fund_code)
                .or_else(|| config.assets.iter().find(|a| a.asset_id == candidate.fund_code))
        } else {
            None
        }
        .or_else(|| {
            if !candidate.fund_name.is_empty() {
                config
                    .assets
                    .iter()
                    .find(|a| a.fund_name == candidate.fund_name)
                    .or_else(|| config.assets.iter().find(|a| a.asset_id == candidate.fund_name))
            } else {
                None
            }
        });

        let mut row = crate::models::BootstrapLocalPreviewRow {
            asset_id: asset_config.map(|a| a.asset_id.clone()),
            fund_code: candidate.fund_code.clone(),
            fund_name: candidate.fund_name.clone(),
            market_value: candidate.market_value,
            latest_nav: None,
            nav_date: None,
            estimated_shares: None,
            estimated_cost_basis: None,
            existing_shares: None,
            action: "skip".to_string(),
            warning: None,
        };

        if let Some(ac) = asset_config {
            // Check if already in state
            if let Some(existing_holding) = state.asset_holdings.iter().find(|h| h.asset_id == ac.asset_id) {
                row.existing_shares = Some(existing_holding.units);
                if existing_holding.units > 0.0 && !replace_existing {
                    row.warning = Some("Local holding already exists. Use --replace-existing to overwrite.".to_string());
                    rows.push(row);
                    continue;
                }
            }

            // Get NAV
            let nav_opt = candidate.nav.or_else(|| {
                nav_cache.get(&ac.fund_code).map(|n| n.nav)
            });

            if let Some(nav) = nav_opt {
                row.latest_nav = Some(nav);
                row.nav_date = candidate.nav_date.clone().or_else(|| nav_cache.get(&ac.fund_code).and_then(|n| n.nav_date.clone()));
                
                if nav > 0.0 {
                    let shares = candidate.market_value / nav;
                    row.estimated_shares = Some(shares);
                }

                if let Some(profit) = candidate.total_profit {
                    row.estimated_cost_basis = Some(candidate.market_value - profit);
                }

                row.action = if replace_existing && row.existing_shares.unwrap_or(0.0) > 0.0 {
                    "replace".to_string()
                } else {
                    "create".to_string()
                };

                total_bootstrapped_value += candidate.market_value;
            } else {
                row.warning = Some("NAV not found in cache. Cannot estimate shares.".to_string());
            }
        } else {
            row.warning = Some("AssetConfig not found. Please run bootstrap-assets first.".to_string());
        }

        rows.push(row);
    }

    crate::models::BootstrapLocalPreview {
        rows,
        total_bootstrapped_value,
    }
}

pub fn apply_bootstrap_local(
    mut state: PortfolioState,
    preview: &crate::models::BootstrapLocalPreview,
) -> (PortfolioState, usize) {
    let mut count = 0;
    for row in &preview.rows {
        if row.action == "create" || row.action == "replace" {
            if let Some(asset_id) = &row.asset_id {
                if let Some(shares) = row.estimated_shares {
                    let nav = row.latest_nav.unwrap_or(0.0);
                    let mut cost_basis = row.estimated_cost_basis.unwrap_or(row.market_value);
                    if cost_basis <= 0.0 {
                        cost_basis = row.market_value;
                    }
                    // Find existing and update or insert new
                    if let Some(holding) = state.asset_holdings.iter_mut().find(|h| h.asset_id == *asset_id) {
                        holding.units = shares;
                        holding.cost_basis = cost_basis;
                        holding.latest_nav = Some(nav);
                        holding.last_market_value = row.market_value;
                        if let Some(nav_date) = &row.nav_date {
                            holding.latest_nav_date = Some(nav_date.clone());
                        }
                    } else {
                        state.asset_holdings.push(crate::models::AssetHolding {
                            asset_id: asset_id.clone(),
                            fund_code: row.fund_code.clone(),
                            units: shares,
                            units_estimated: true,
                            cost_basis,
                            latest_nav: Some(nav),
                            latest_nav_date: row.nav_date.clone(),
                            latest_nav_source: Some("alipay_snapshot_bootstrap".to_string()),
                            latest_nav_status: Some("OK".to_string()),
                            last_market_value: row.market_value,
                        });
                    }
                    count += 1;
                }
            }
        }
    }
    (state, count)
}
