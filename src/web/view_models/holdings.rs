//! Holdings (持仓) display model.

use crate::engine::asset_enrichment::is_asset_archived;
use crate::models;
use crate::models::AlipaySnapshot;
use crate::web::product::{equity_region_bucket, render_alipay_bootstrap_card};
use crate::web::services::asset_enrichment_service::asset_row_source;
use crate::web::services::holdings_service::HoldingsPageData;
use crate::web::utils::color_class;
use std::collections::HashMap;

pub struct HoldingsPageVm {
    pub bootstrap_html: String,
    pub display_book: f64,
    pub alipay_total: f64,
    pub diff: f64,
    pub diff_class: String,
    pub holdings_rows_html: String,
    pub asset_table_html: String,
    pub assets_json: String,
    pub show_archived: bool,
}

pub fn build_holdings_vm(data: HoldingsPageData, list_filter: Option<&str>) -> HoldingsPageVm {
    let config = data.config;
    let portfolio_state = data.portfolio_state;
    let summary = data.summary;
    let latest_snaps = data.latest_snaps;
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
    let alipay_total: f64 = latest_snaps.values().map(|s| s.market_value).sum();
    let snap_date = latest_snaps
        .values()
        .map(|s| s.snapshot_date.as_str())
        .max()
        .map(|s| s.to_string());
    let show_bootstrap = ledger_value < 1.0 && alipay_total > 1.0;
    let display_book = if ledger_value > 0.01 {
        ledger_value
    } else {
        summary.total_asset_value
    };
    let diff = display_book - alipay_total;

    let bootstrap_html = if show_bootstrap {
        render_alipay_bootstrap_card(alipay_total, snap_date.as_deref())
    } else {
        String::new()
    };

    let mut rows = String::new();
    if show_bootstrap {
        append_snapshot_rows(&mut rows, &config, &latest_snaps);
    }
    append_ledger_rows(&mut rows, &config, &portfolio_state, &latest_snaps);

    if rows.is_empty() && !show_bootstrap {
        rows = r#"<tr><td colspan="9"><div class="empty-state"><span class="empty-state-icon">💰</span><div class="empty-state-text">请导入支付宝持仓快照或手动新增持仓</div></div></td></tr>"#.to_string();
    }

    let diff_class = if diff.abs() < 10.0 {
        "text-muted".to_string()
    } else if diff > 0.0 {
        "text-up".to_string()
    } else {
        "text-down".to_string()
    };

    let mut assets_for_json: Vec<serde_json::Value> = Vec::new();
    let mut asset_table = String::new();
    for a in &config.assets {
        let archived = is_asset_archived(a);
        if show_archived && !archived {
            continue;
        }
        if !show_archived && archived {
            continue;
        }
        let region = equity_region_bucket(&a.sector);
        let source = asset_row_source(a);
        let status_b = if archived {
            "<span class='badge badge-gray'>已归档</span>"
        } else if a.enabled {
            "<span class='badge badge-blue'>启用</span>"
        } else {
            "<span class='badge badge-gray'>禁用</span>"
        };
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
        asset_table.push_str(&format!(
            r#"<tr>
                <td><div style="font-weight:600;">{}</div><div style="font-size:0.7rem;color:var(--text-muted);">{}</div></td>
                <td><code>{}</code></td>
                <td>{}</td>
                <td><span class="badge badge-outline">{}</span></td>
                <td><span class="source-hint">{}</span></td>
                <td>{}</td>
                <td class="text-right" style="white-space:nowrap;">
                    <button type="button" class="btn btn-outline btn-sm" onclick="openAssetEdit('{}')">编辑</button>
                    <button type="button" class="btn btn-outline btn-sm" onclick="enrichOneAsset('{}', this)">自动补全</button>
                    <button type="button" class="btn-ghost btn-sm" onclick="createDcaForAsset('{}','{}')">+定投</button>
                </td>
            </tr>"#,
            a.fund_name,
            a.asset_id,
            a.fund_code,
            status_b,
            a.sector,
            region,
            source,
            a.asset_id,
            a.asset_id,
            a.asset_id,
            a.fund_code
        ));
    }
    if asset_table.is_empty() {
        asset_table = "<tr><td colspan='7'>无资产。点击「新增资产」创建。</td></tr>".to_string();
    }
    let assets_json = serde_json::to_string(&assets_for_json).unwrap_or_else(|_| "[]".into());

    HoldingsPageVm {
        bootstrap_html,
        display_book,
        alipay_total,
        diff,
        diff_class,
        holdings_rows_html: rows,
        asset_table_html: asset_table,
        assets_json,
        show_archived,
    }
}

fn append_snapshot_rows(
    rows: &mut String,
    config: &models::ConfigRoot,
    latest_snaps: &HashMap<String, AlipaySnapshot>,
) {
    let mut snap_list: Vec<_> = latest_snaps.values().collect();
    snap_list.sort_by(|a, b| {
        b.market_value
            .partial_cmp(&a.market_value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for s in snap_list {
        let sector = config
            .assets
            .iter()
            .find(|a| a.fund_code == s.fund_code || a.asset_id == s.asset_id)
            .map(|a| a.sector.as_str())
            .unwrap_or("—");
        let region = if sector != "—" {
            equity_region_bucket(sector)
        } else {
            "—"
        };
        let pnl = s.total_pnl.unwrap_or(0.0);
        let pnl_pct = s
            .cost_basis
            .filter(|c| *c > 0.01)
            .map(|c| pnl / c * 100.0)
            .unwrap_or(0.0);
        rows.push_str(&format!(
            r#"<tr>
                <td><div style="font-weight:700;">{}</div><div style="font-size:0.75rem;color:var(--text-muted);"><code>{}</code></div></td>
                <td><span class="badge badge-outline">{}</span></td>
                <td><span class="badge badge-outline">{}</span></td>
                <td class="text-right tabular" style="font-weight:700;">{:.2}</td>
                <td class="text-right tabular">{}</td>
                <td class="text-right tabular">{}</td>
                <td class="text-right tabular {}"><div>{:+.2}</div><div style="font-size:0.75rem;">{:+.1}%</div></td>
                <td class="text-muted" style="font-size:0.8rem;">支付宝快照</td>
                <td class="text-muted" style="font-size:0.8rem;">待初始化</td>
            </tr>"#,
            s.fund_name,
            s.fund_code,
            sector,
            region,
            s.market_value,
            s.units
                .map(|u| format!("{:.4}", u))
                .unwrap_or_else(|| "—".to_string()),
            s.nav
                .map(|n| format!("{:.4}", n))
                .unwrap_or_else(|| "—".to_string()),
            color_class(pnl),
            pnl,
            pnl_pct
        ));
    }
}

fn append_ledger_rows(
    rows: &mut String,
    config: &models::ConfigRoot,
    portfolio_state: &models::PortfolioState,
    latest_snaps: &HashMap<String, AlipaySnapshot>,
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

        let snap_key = holding.asset_id.clone();
        let alipay_diff_html = latest_snaps
            .get(&snap_key)
            .or_else(|| latest_snaps.get(&format!("unmatched_{}", holding.fund_code)))
            .map(|snap| {
                let d = market_value - snap.market_value;
                let diff_class = if d.abs() < 1.0 {
                    "text-muted"
                } else if d > 0.0 {
                    "text-up"
                } else {
                    "text-down"
                };
                format!(
                    "<span class='{} tabular' title='对比支付宝快照'>{:+.2}</span>",
                    diff_class, d
                )
            })
            .unwrap_or_else(|| "<span class='text-muted'>—</span>".to_string());
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

        rows.push_str(&format!(
            r#"<tr>
                <td>
                    <div style="font-weight: 700;">{}</div>
                    <div style="font-size: 0.75rem; color: var(--text-muted);"><code>{}</code></div>
                </td>
                <td><span class="badge {}">{}</span></td>
                <td><span class="badge badge-outline">{}</span></td>
                <td class="text-right tabular" style="font-weight: 700;">{:.2}</td>
                <td class="text-right tabular">{:.4}</td>
                <td class="text-right tabular" title="{}">{}</td>
                <td class="text-right tabular {}">
                    <div>{:+.2}</div>
                    <div style="font-size: 0.75rem;">{:+.1}%</div>
                </td>
                <td class="text-right">{}</td>
                <td class="text-right"><button type="button" class="btn-ghost btn-sm" onclick="openAssetEdit('{}')">编辑</button></td>
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
            market_value,
            holding.units,
            nav_src,
            nav_disp,
            color_class(pnl),
            pnl,
            pnl_pct,
            alipay_diff_html,
            holding.asset_id
        ));
    }
}
