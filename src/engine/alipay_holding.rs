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
    let mut invalid_rows = 0;
    let mut unmatched_rows = 0;

    for candidate in &candidates {
        let mut row_warnings = Vec::new();
        let mut row_errors = Vec::new();

        // Find match in config: try code first, then name
        let asset_config = if !candidate.fund_code.is_empty() {
            config
                .assets
                .iter()
                .find(|a| a.fund_code == candidate.fund_code)
        } else {
            config
                .assets
                .iter()
                .find(|a| a.fund_name == candidate.fund_name)
        };

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
            invalid_rows += 1;
            row_errors.push(format!(
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
        if let Some(asset_id_ref) = &preview.matched_asset_ids[i] {
            if !preview.errors[i].is_empty() {
                continue;
            }

            let asset_id: String = asset_id_ref.clone();

            snapshots.push(AlipaySnapshot {
                snapshot_id: format!(
                    "snap_{}_{}_{}",
                    asset_id,
                    preview.snapshot_date,
                    Local::now().timestamp_millis()
                ),
                asset_id: asset_id.clone(),
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
    }
    snapshots
}
