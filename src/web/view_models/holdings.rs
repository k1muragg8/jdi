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
    append_ledger_rows(&mut rows, &config, &portfolio_state, &dca_plans);

    if rows.is_empty() {
        rows = r#"<tr><td colspan=\"10\"><div class=\"empty-state\"><span class=\"empty-state-icon\">💰</span><div class=\"empty-state-text\">暂无本地持仓，请新增资产。</div></div></td></tr>"#.to_string();
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

    HoldingsPageVm {
        bootstrap_html,
        display_book,
        equity_value: summary.equity_value,
        bond_value: summary.bond_value,
        cash_value: summary.cash,
        holdings_rows_html: rows,
        assets_json,
        show_archived,
    }
}

fn append_ledger_rows(
    rows: &mut String,
    config: &models::ConfigRoot,
    portfolio_state: &models::PortfolioState,
    dca_plans: &[models::DcaPlan],
) {
    for holding in &portfolio_state.asset_holdings {
        let asset_config = config
            .assets
            .iter()
            .find(|a| a.asset_id == holding.asset_id);
        if !asset_config.map(|a| a.enabled).unwrap_or(false) {
            continue;
        }
        if asset_config.is_some_and(is_asset_archived) {
            continue;
        }

        let name = asset_config
            .map(|a| a.fund_name.as_str())
            .unwrap_or("Unknown");
        let sector = asset_config.map(|a| a.sector.as_str()).unwrap_or("未分类");
        let is_unclassified = sector == "未分类" || sector.is_empty();

        let market_value = holding.last_market_value;
        let cost = holding.cost_basis;
        let pnl = market_value - cost;
        let pnl_pct = if cost.abs() > 0.01 {
            pnl / cost * 100.0
        } else {
            0.0
        };

        let region = equity_region_bucket(sector);
        let nav_disp = holding
            .latest_nav
            .map(|n| format!("{:.4}", n))
            .unwrap_or_else(|| "—".to_string());
        let nav_src = if holding.latest_nav.is_some() {
            "NAV缓存"
        } else {
            "—"
        };
        let nav_date = holding.latest_nav_date.as_deref().unwrap_or("—");

        let plan = dca_plans.iter().find(|p| p.asset_id == holding.asset_id);
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
                    holding.asset_id, holding.asset_id, holding.asset_id
                )
            } else {
                format!(
                    r#"<button type="button" class="btn btn-sm btn-outline" onclick="openDcaModal('{}')">编辑定投</button>
                       <button type="button" class="btn btn-sm btn-outline" onclick="resumeDca('{}', this)">恢复</button>
                       <button type="button" class="btn btn-sm btn-outline" onclick="viewDcaRecords('{}')">查看记录</button>"#,
                    holding.asset_id, holding.asset_id, holding.asset_id
                )
            };
            (status_badge, actions)
        } else {
            (
                "<span class='badge badge-gray'>未设置</span>".to_string(),
                format!(
                    r#"<button type="button" class="btn btn-sm btn-outline" onclick="openDcaModal('{}')">设置定投</button>"#,
                    holding.asset_id
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
                <td class="text-right tabular">{:.4}</td>
                <td class="text-right tabular" title="{}">{}</td>
                <td class="text-right tabular">{}</td>
                <td class="text-right tabular" style="font-weight: 700;">{:.2}</td>
                <td class="text-right tabular {}">
                    <div>{:+.2}</div>
                    <div style="font-size: 0.75rem;">{:+.1}%</div>
                </td>
                <td class="text-right tabular">{}</td>
                <td class="text-right"><button type="button" class="btn btn-sm btn-outline" onclick="openAssetEdit('{}')">编辑</button> {}</td>
            </tr>"#,
            name,
            holding.fund_code,
            if is_unclassified {
                "badge-orange"
            } else {
                "badge-outline"
            },
            sector,
            region,
            holding.units,
            nav_src,
            nav_disp,
            nav_date,
            market_value,
            color_class(pnl),
            pnl,
            pnl_pct,
            dca_status_html,
            holding.asset_id,
            dca_action_html
        ));
    }
}
