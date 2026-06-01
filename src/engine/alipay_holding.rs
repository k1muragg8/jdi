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
            match header {
                "基金代码" | "代码" => candidate.fund_code = val.to_string(),
                "基金名称" | "名称" => candidate.fund_name = val.to_string(),
                "持有份额" | "份额" | "份额(份)" => {
                    candidate.units = val.replace(',', "").parse().unwrap_or(0.0)
                }
                "市值" | "市值(元)" => {
                    candidate.market_value = val.replace(',', "").parse().unwrap_or(0.0)
                }
                "单位净值" | "净值" => candidate.nav = val.replace(',', "").parse().ok(),
                "净值日期" => candidate.nav_date = Some(val.to_string()),
                "成本价" | "成本" => candidate.cost_basis = val.replace(',', "").parse().ok(),
                "累计收益" => candidate.total_profit = val.replace(',', "").parse().ok(),
                _ => {}
            }
        }

        if !candidate.fund_code.is_empty() {
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

    for candidate in &candidates {
        let mut row_warnings = Vec::new();
        let mut row_errors = Vec::new();

        // Find match in config by fund_code
        let asset_config = config
            .assets
            .iter()
            .find(|a| a.fund_code == candidate.fund_code);

        if let Some(ac) = asset_config {
            matched_asset_ids.push(Some(ac.asset_id.clone()));

            // Find in state
            let holding = state
                .asset_holdings
                .iter()
                .find(|h| h.asset_id == ac.asset_id);
            if let Some(h) = holding {
                system_units.push(Some(h.units));
                system_market_values.push(Some(h.last_market_value));
                unit_diffs.push(Some(candidate.units - h.units));

                if (candidate.units - h.units).abs() > 0.0001 {
                    row_warnings.push(format!("份额不匹配: 差额 {:.4}", candidate.units - h.units));
                }
            } else {
                system_units.push(Some(0.0));
                system_market_values.push(Some(0.0));
                unit_diffs.push(Some(candidate.units));
                if candidate.units > 0.0 {
                    row_warnings.push("系统中无此资产持仓".to_string());
                }
            }
        } else {
            matched_asset_ids.push(None);
            system_units.push(None);
            system_market_values.push(None);
            unit_diffs.push(None);
            row_errors.push(format!("未找到匹配的资产配置: {}", candidate.fund_code));
        }

        warnings.push(row_warnings);
        errors.push(row_errors);
    }

    AlipayHoldingImportPreview {
        snapshot_date: snapshot_date.to_string(),
        candidates,
        matched_asset_ids,
        system_units,
        system_market_values,
        unit_diffs,
        warnings,
        errors,
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
                units: Some(candidate.units),
                cost_basis: candidate.cost_basis,
                nav: candidate.nav,
                nav_date: candidate.nav_date.clone(),
                daily_pnl: None,
                total_pnl: candidate.total_profit,
                source: "alipay_import".to_string(),
                created_at: now.clone(),
                note: Some("Imported from CSV".to_string()),
            });
        }
    }
    snapshots
}
