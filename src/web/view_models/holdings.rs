//! Holdings (持仓) display model.

use crate::models::AlipaySnapshot;
use crate::web::product::{equity_region_bucket, render_alipay_bootstrap_card};
use crate::web::services::holdings_service::HoldingsPageData;
use crate::web::utils::color_class;
use crate::models;
use std::collections::HashMap;

pub struct HoldingsPageVm {
    pub bootstrap_html: String,
    pub display_book: f64,
    pub alipay_total: f64,
    pub diff: f64,
    pub diff_class: String,
    pub holdings_rows_html: String,
    pub asset_mgmt_rows_html: String,
}

pub fn build_holdings_vm(data: HoldingsPageData) -> HoldingsPageVm {
    let config = data.config;
    let portfolio_state = data.portfolio_state;
    let summary = data.summary;
    let latest_snaps = data.latest_snaps;

    let ledger_value: f64 = portfolio_state
        .asset_holdings
        .iter()
        .filter(|h| {
            config
                .assets
                .iter()
                .find(|a| a.asset_id == h.asset_id)
                .map(|a| a.enabled)
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
        rows = r#"<tr><td colspan="8"><div class="empty-state"><span class="empty-state-icon">💰</span><div class="empty-state-text">请导入支付宝持仓快照或手动新增持仓</div></div></td></tr>"#.to_string();
    }

    let diff_class = if diff.abs() < 10.0 {
        "text-muted".to_string()
    } else if diff > 0.0 {
        "text-up".to_string()
    } else {
        "text-down".to_string()
    };

    let asset_mgmt_rows_html = build_asset_mgmt_rows(&config);

    HoldingsPageVm {
        bootstrap_html,
        display_book,
        alipay_total,
        diff,
        diff_class,
        holdings_rows_html: rows,
        asset_mgmt_rows_html,
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
                format!("<span class='{} tabular'>{:+.2}</span>", diff_class, d)
            })
            .unwrap_or_else(|| "<span class='text-muted'>—</span>".to_string());
        let region = equity_region_bucket(sector);
        let nav_disp = holding
            .latest_nav
            .map(|n| format!("{:.4}", n))
            .unwrap_or_else(|| "—".to_string());

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
                <td class="text-right tabular">{}</td>
                <td class="text-right tabular {}">
                    <div>{:+.2}</div>
                    <div style="font-size: 0.75rem;">{:+.1}%</div>
                </td>
                <td class="text-right">{}</td>
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
            nav_disp,
            color_class(pnl),
            pnl,
            pnl_pct,
            alipay_diff_html
        ));
    }
}

fn build_asset_mgmt_rows(config: &models::ConfigRoot) -> String {
    let mut asset_mgmt_rows = String::new();
    for a in &config.assets {
        let unclassified = a.sector.is_empty() || a.sector == "未分类" || a.sector == "待确认";
        let sector_badge = if unclassified {
            "<span class='badge badge-orange'>未分类</span>".to_string()
        } else {
            format!("<span class='badge badge-outline'>{}</span>", a.sector)
        };
        let status_b = if a.enabled {
            "<span class='badge badge-blue'>启用</span>"
        } else {
            "<span class='badge badge-gray'>禁用</span>"
        };
        let en_dis = if a.enabled {
            format!(
                r#"<form action="/admin/assets/disable" method="POST" style="display:inline;"><input type="hidden" name="asset_id" value="{}"><button type="submit" class="btn-ghost" style="font-size:0.65rem;">禁用</button></form>"#,
                a.asset_id
            )
        } else {
            format!(
                r#"<form action="/admin/assets/enable" method="POST" style="display:inline;"><input type="hidden" name="asset_id" value="{}"><button type="submit" class="btn-ghost" style="font-size:0.65rem;">启用</button></form>"#,
                a.asset_id
            )
        };
        let rename_form = format!(
            r#"<form action="/admin/assets/rename" method="POST" style="display:inline-flex;gap:2px;margin-top:2px;"><input type="hidden" name="asset_id" value="{0}"><input type="text" name="fund_name" value="{1}" style="width:90px;font-size:0.7rem;padding:1px;"><button class="btn-ghost" style="font-size:0.6rem;">改名</button></form>"#,
            a.asset_id, a.fund_name
        );
        let fund_form = format!(
            r#"<form action="/admin/assets/set-fund-code" method="POST" style="display:inline-flex;gap:2px;"><input type="hidden" name="asset_id" value="{0}"><input type="text" name="fund_code" value="{1}" style="width:70px;font-size:0.7rem;padding:1px;"><button class="btn-ghost" style="font-size:0.6rem;">存</button></form>"#,
            a.asset_id, a.fund_code
        );
        let sector_form = format!(
            r#"<form action="/admin/assets/set-sector" method="POST" style="display:inline-flex;gap:2px;"><input type="hidden" name="asset_id" value="{0}"><input type="text" name="sector" value="{1}" style="width:70px;font-size:0.7rem;padding:1px;" placeholder="赛道"><button class="btn-ghost" style="font-size:0.6rem;">存</button></form>"#,
            a.asset_id, a.sector
        );
        let archive_btn = format!(
            r#"<form action="/admin/assets/remove" method="POST" style="display:inline;" onsubmit="return confirm('归档此资产? (引用数据保留)');"><input type="hidden" name="asset_id" value="{}"><button type="submit" class="btn-ghost" style="font-size:0.65rem;color:#c00;">归档</button></form>"#,
            a.asset_id
        );
        let create_dca_btn = format!(
            r#"<button class="btn-ghost" style="font-size:0.65rem;" onclick="createDcaForAsset('{}', '{}')">+定投</button>"#,
            a.asset_id, a.fund_code
        );
        asset_mgmt_rows.push_str(&format!(
            r#"<tr>
                <td><div style="font-weight:600;">{}</div><div style="font-size:0.6rem;color:var(--text-muted);">{}</div>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td style="white-space:nowrap;">{} {} {} {} {}</td>
            </tr>"#,
            a.fund_name,
            a.asset_id,
            rename_form,
            fund_form,
            sector_badge,
            sector_form,
            status_b,
            en_dis,
            archive_btn,
            create_dca_btn,
            if unclassified {
                "<span style='color:#f80;font-size:0.6rem;'>需分类</span>"
            } else {
                ""
            }
        ));
    }
    if asset_mgmt_rows.is_empty() {
        "<tr><td colspan='5'>无资产配置，请新增</td></tr>".to_string()
    } else {
        asset_mgmt_rows
    }
}
