//! Holdings (持仓) display model.

use crate::engine::asset_enrichment::is_asset_archived;
use crate::models;
use crate::web::product::equity_region_bucket;
use crate::web::services::holdings_service::HoldingsPageData;
use crate::web::utils::color_class;

pub struct HoldingsPageVm {
    pub bootstrap_html: String,
    pub display_book: f64,
    pub equity_value: f64,
    pub bond_value: f64,
    pub cash_value: f64,
    pub holdings_rows_html: String,
    pub assets_json: String,
    pub show_archived: bool,
    pub active_count: usize,
    pub archived_count: usize,
}

pub fn build_holdings_vm(data: HoldingsPageData, list_filter: Option<&str>) -> HoldingsPageVm {
    let config = data.config;
    let portfolio_state = data.portfolio_state;
    let summary = data.summary;
    let dca_plans = data.dca_plans;
    let show_archived = list_filter == Some("archived");

    let ledger_value: f64 = portfolio_state
        .asset_holdings
        .iter()
        .filter(|h| {
            config
                .assets
                .iter()
                .find(|a| a.asset_id == h.asset_id)
                .map(|a| a.enabled && !is_asset_archived(a))
                .unwrap_or(false)
        })
        .map(|h| h.last_market_value)
        .sum();
    let display_book = if ledger_value > 0.01 {
        ledger_value
    } else {
        summary.total_asset_value
    };

    let bootstrap_html = String::new(); // alipay ui removed

    let mut rows = String::new();
    append_ledger_rows(
        &mut rows,
        &config,
        &portfolio_state,
        &dca_plans,
        show_archived,
    );

    if rows.is_empty() {
        rows = r#"<tr><td colspan="10"><div class="empty-state"><span class="empty-state-icon">💰</span><div class="empty-state-text">暂无本地持仓，请先新增资产。</div></div></td></tr>"#.to_string();
    }

    let mut assets_for_json: Vec<serde_json::Value> = Vec::new();
    for a in &config.assets {
        let archived = is_asset_archived(a);
        if show_archived && !archived {
            continue;
        }
        if !show_archived && archived {
            continue;
        }
        let region = equity_region_bucket(&a.sector);
        assets_for_json.push(serde_json::json!({
            "asset_id": a.asset_id,
            "fund_code": a.fund_code,
            "fund_name": a.fund_name,
            "sector": a.sector,
            "region": region,
            "currency": a.currency,
            "enabled": a.enabled,
            "reference_index_symbol": a.reference_index_symbol,
            "reference_instrument_symbol": a.reference_instrument_symbol,
            "market_data_provider": a.market_data_provider,
            "valuation_method": a.valuation_method,
        }));
    }
    let assets_json = serde_json::to_string(&assets_for_json).unwrap_or_else(|_| "[]".into());

    let active_count = config
        .assets
        .iter()
        .filter(|a| a.enabled && !is_asset_archived(a))
        .count();
    let archived_count = config
        .assets
        .iter()
        .filter(|a| is_asset_archived(a))
        .count();

    HoldingsPageVm {
        bootstrap_html,
        display_book,
        equity_value: summary.equity_value,
        bond_value: summary.bond_value,
        cash_value: summary.cash,
        holdings_rows_html: rows,
        assets_json,
        show_archived,
        active_count,
        archived_count,
    }
}

fn append_ledger_rows(
    rows: &mut String,
    config: &models::ConfigRoot,
    portfolio_state: &models::PortfolioState,
    dca_plans: &[models::DcaPlan],
    show_archived: bool,
) {
    for asset in &config.assets {
        let archived = is_asset_archived(asset);
        if show_archived {
            if !archived {
                continue;
            }
        } else if archived || !asset.enabled {
            continue;
        }

        let holding = portfolio_state
            .asset_holdings
            .iter()
            .find(|h| h.asset_id == asset.asset_id);

        let name = &asset.fund_name;
        let sector = &asset.sector;
        let is_unclassified = sector == "未分类" || sector.is_empty();
        let region = equity_region_bucket(sector);

        let units_disp = if let Some(h) = holding {
            if h.units > 0.0 {
                format!("{:.4}", h.units)
            } else {
                "待录入".to_string()
            }
        } else {
            "待录入".to_string()
        };

        let nav_disp = holding
            .and_then(|h| h.latest_nav)
            .map(|n| format!("{:.4}", n))
            .unwrap_or_else(|| "待刷新".to_string());
        let nav_src = if holding.and_then(|h| h.latest_nav).is_some() {
            "NAV缓存"
        } else {
            "—"
        };
        let nav_date = holding
            .and_then(|h| h.latest_nav_date.as_deref())
            .unwrap_or("—");

        let market_value = holding.map(|h| h.last_market_value).unwrap_or(0.0);
        let market_value_disp = if market_value > 0.0 {
            format!("{:.2}", market_value)
        } else {
            "—".to_string()
        };

        let cost = holding.map(|h| h.cost_basis).unwrap_or(0.0);
        let pnl = market_value - cost;
        let pnl_pct = if cost.abs() > 0.01 {
            pnl / cost * 100.0
        } else {
            0.0
        };

        let pnl_html = if market_value > 0.0 || cost > 0.0 {
            format!(
                r#"<td class="text-right tabular {}">
                    <div>{:+.2}</div>
                    <div style="font-size: 0.75rem;">{:+.1}%</div>
                </td>"#,
                color_class(pnl),
                pnl,
                pnl_pct
            )
        } else {
            r#"<td class="text-right tabular text-muted">—</td>"#.to_string()
        };

        let plan = dca_plans.iter().find(|p| p.asset_id == asset.asset_id);
        let (dca_status_html, dca_action_html) = if let Some(p) = plan {
            let status = if p.enabled {
                format!(
                    "{} {:.0} CNY",
                    match p.frequency {
                        models::DcaFrequency::Daily => "每日",
                        models::DcaFrequency::Weekly => "每周",
                        models::DcaFrequency::Monthly => "每月",
                    },
                    p.amount
                )
            } else {
                "已暂停".to_string()
            };
            let status_badge = if p.enabled {
                format!("<span class='badge badge-blue'>{}</span>", status)
            } else {
                format!("<span class='badge badge-gray'>{}</span>", status)
            };
            let actions = if p.enabled {
                format!(
                    r#"<button type="button" class="btn btn-sm btn-outline" onclick="openDcaModal('{}')">编辑定投</button>
                       <button type="button" class="btn btn-sm btn-outline" onclick="pauseDca('{}', this)">暂停</button>
                       <button type="button" class="btn btn-sm btn-outline" onclick="viewDcaRecords('{}')">查看记录</button>"#,
                    asset.asset_id, asset.asset_id, asset.asset_id
                )
            } else {
                format!(
                    r#"<button type="button" class="btn btn-sm btn-outline" onclick="openDcaModal('{}')">编辑定投</button>
                       <button type="button" class="btn btn-sm btn-outline" onclick="resumeDca('{}', this)">恢复</button>
                       <button type="button" class="btn btn-sm btn-outline" onclick="viewDcaRecords('{}')">查看记录</button>"#,
                    asset.asset_id, asset.asset_id, asset.asset_id
                )
            };
            (status_badge, actions)
        } else {
            (
                "<span class='badge badge-gray'>未设置</span>".to_string(),
                format!(
                    r#"<button type="button" class="btn btn-sm btn-outline" onclick="openDcaModal('{}')">设置定投</button>"#,
                    asset.asset_id
                ),
            )
        };

        rows.push_str(&format!(
            r#"<tr>
                <td>
                    <div style="font-weight: 700;">{}</div>
                    <div style="font-size: 0.75rem; color: var(--text-muted);"><code>{}</code></div>
                </td>
                <td><span class="badge {}">{}</span></td>
                <td><span class="badge badge-outline">{}</span></td>
                <td class="text-right tabular">{}</td>
                <td class="text-right tabular" title="{}">{}</td>
                <td class="text-right tabular">{}</td>
                <td class="text-right tabular" style="font-weight: 700;">{}</td>
                {}
                <td class="text-right tabular">{}</td>
                <td class="text-right"><button type="button" class="btn btn-sm btn-outline" onclick="openAssetEdit('{}')">编辑</button> {}</td>
            </tr>"#,
            name,
            asset.fund_code,
            if is_unclassified {
                "badge-orange"
            } else {
                "badge-outline"
            },
            sector,
            region,
            units_disp,
            nav_src,
            nav_disp,
            nav_date,
            market_value_disp,
            pnl_html,
            dca_status_html,
            asset.asset_id,
            dca_action_html
        ));
    }
}
