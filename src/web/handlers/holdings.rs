//! Holdings (持仓) page handlers.

use crate::web::product::{
    equity_region_bucket, render_alipay_bootstrap_card, snapshots_to_candidates,
};
use crate::web::response::AdminQuery;
use crate::web::state::AppState;
use crate::web::utils::*;
use crate::web::views::layout::{layout, layout_with_msg};
use crate::{engine, models};
use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use std::sync::Arc;

pub async fn api_holdings_bootstrap_alipay_handler(State(state): State<Arc<AppState>>) -> Redirect {
    let ctx = &state.ctx;
    let result = async {
        let mut config = state.repo.load_config(ctx).await?;
        let portfolio_state = state.repo.load_state(ctx).await?;
        let snapshots = state.repo.load_alipay_snapshots(ctx).await?;
        let mut latest: std::collections::HashMap<String, models::AlipaySnapshot> =
            std::collections::HashMap::new();
        for s in &snapshots {
            let key = if s.asset_id.is_empty() {
                format!("unmatched_{}", s.fund_code)
            } else {
                s.asset_id.clone()
            };
            let e = latest.entry(key).or_insert_with(|| s.clone());
            if s.snapshot_date >= e.snapshot_date {
                *e = s.clone();
            }
        }
        let candidates = snapshots_to_candidates(&latest);
        if candidates.is_empty() {
            return Err(anyhow::anyhow!("无支付宝快照可初始化"));
        }
        let (created, _, _) =
            engine::alipay_holding::bootstrap_assets_from_holdings(&mut config, &candidates);
        state.repo.save_config(ctx, &config).await?;
        let nav_cache = state.repo.load_nav_cache(ctx).await.unwrap_or_default();
        let nav_map: std::collections::HashMap<String, models::FundNav> = nav_cache
            .entries
            .iter()
            .map(|e| {
                (
                    e.fund_code.clone(),
                    models::FundNav {
                        fund_code: e.fund_code.clone(),
                        nav: e.nav,
                        accumulated_nav: e.accumulated_nav,
                        nav_date: e.nav_date.clone(),
                        currency: e.currency.clone(),
                        source: e.source.clone(),
                        is_stale: false,
                        is_estimated: false,
                    },
                )
            })
            .collect();
        let preview = engine::alipay_holding::preview_bootstrap_local(
            &config,
            &portfolio_state,
            &candidates,
            &nav_map,
            true,
        );
        let (new_state, n) =
            engine::alipay_holding::apply_bootstrap_local(portfolio_state, &preview);
        state.repo.save_state(ctx, &new_state).await?;
        Ok::<String, anyhow::Error>(format!(
            "已用支付宝快照初始化 {} 项持仓（新建资产 {} 个）",
            n, created
        ))
    }
    .await;

    match result {
        Ok(msg) => Redirect::to(&format!("/holdings?success={}", msg)),
        Err(e) => Redirect::to(&format!("/holdings?error={}", e)),
    }
}

pub async fn holdings_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminQuery>,
) -> Html<String> {
    let ctx = &state.ctx;
    let result = async {
        let config = state.repo.load_config(&ctx).await?;
        let portfolio_state = state.repo.load_state(&ctx).await?;
        let summary = engine::calculate_portfolio_summary(&config, &portfolio_state);
        let snapshots = state
            .repo
            .load_alipay_snapshots(&ctx)
            .await
            .unwrap_or_default();
        Ok::<
            (
                models::ConfigRoot,
                models::PortfolioState,
                models::PortfolioSummary,
                Vec<models::AlipaySnapshot>,
            ),
            anyhow::Error,
        >((config, portfolio_state, summary, snapshots))
    }
    .await;

    match result {
        Ok((config, portfolio_state, summary, snapshots)) => {
            let mut latest_snaps: std::collections::HashMap<String, models::AlipaySnapshot> =
                std::collections::HashMap::new();
            for s in &snapshots {
                let key = if s.asset_id.is_empty() {
                    format!("unmatched_{}", s.fund_code)
                } else {
                    s.asset_id.clone()
                };
                let entry = latest_snaps.entry(key).or_insert(s.clone());
                if s.snapshot_date >= entry.snapshot_date {
                    *entry = s.clone();
                }
            }

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
                        s.nav.map(|n| format!("{:.4}", n)).unwrap_or_else(|| "—".to_string()),
                        color_class(pnl),
                        pnl,
                        pnl_pct
                    ));
                }
            }
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

            if rows.is_empty() && !show_bootstrap {
                rows = r#"<tr><td colspan="8"><div class="empty-state"><span class="empty-state-icon">💰</span><div class="empty-state-text">请导入支付宝持仓快照或手动新增持仓</div></div></td></tr>"#.to_string();
            }
            let diff_class = if diff.abs() < 10.0 {
                "text-muted"
            } else if diff > 0.0 {
                "text-up"
            } else {
                "text-down"
            };

            // Build inline asset config management table (part of /holdings per UX req: edit name/fund/sector/status/archive/create-dca directly)
            let mut asset_mgmt_rows = String::new();
            for a in &config.assets {
                let unclassified =
                    a.sector.is_empty() || a.sector == "未分类" || a.sector == "待确认";
                let sector_badge = if unclassified {
                    "<span class='badge badge-orange'>未分类</span>"
                } else {
                    &format!("<span class='badge badge-outline'>{}</span>", a.sector)
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
                    a.fund_name, a.asset_id, rename_form, fund_form, sector_badge, sector_form, status_b, en_dis, archive_btn, create_dca_btn, if unclassified { "<span style='color:#f80;font-size:0.6rem;'>需分类</span>" } else { "" }
                ));
            }
            if asset_mgmt_rows.is_empty() {
                asset_mgmt_rows = "<tr><td colspan='5'>无资产配置，请新增</td></tr>".to_string();
            }

            let content = format!(
                r#"
                <div style="display:flex;justify-content:space-between;align-items:flex-end;margin-bottom:20px;flex-wrap:wrap;gap:12px;">
                    <div>
                        <h1 style="margin-bottom:4px;">持仓</h1>
                        <p style="color:var(--text-muted);font-size:0.9rem;margin:0;">基于支付宝快照与本地账本，查看市值与差异</p>
                    </div>
                    <div class="action-group" style="margin:0;">
                        <button type="button" class="btn btn-outline btn-sm" onclick="document.getElementById('holdingsImportPanel').style.display='block'">导入支付宝快照</button>
                        <button type="button" class="btn btn-outline btn-sm" onclick="document.getElementById('addAssetInline').style.display='block'">手动新增</button>
                    </div>
                </div>

                {}

                <div class="overview-metrics" style="margin-bottom:16px;">
                    <div class="card"><div class="card-header"><span class="card-title">系统账面</span></div><div class="card-value tabular">{:.2}</div></div>
                    <div class="card"><div class="card-header"><span class="card-title">支付宝快照</span></div><div class="card-value tabular">{:.2}</div></div>
                    <div class="card"><div class="card-header"><span class="card-title">差额</span></div><div class="card-value tabular {}">{:+.2}</div></div>
                </div>

                <div class="table-container">
                    <div class="table-wrap">
                        <table class="holdings-compact">
                            <thead>
                                <tr>
                                    <th>资产 / 代码</th>
                                    <th>赛道</th>
                                    <th>地区</th>
                                    <th class="text-right">市值</th>
                                    <th class="text-right">份额</th>
                                    <th class="text-right">净值</th>
                                    <th class="text-right">盈亏</th>
                                    <th class="text-right">差异</th>
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                    </div>
                </div>

                <div style="margin: 24px 0 8px;">
                    <div style="display:flex; justify-content:space-between; align-items:center;">
                        <h3 style="margin:0; font-size:1rem;">资产配置管理 (Inline CRUD - 无需去系统)</h3>
                        <button onclick="document.getElementById('addAssetInline').style.display='block'" class="btn btn-sm">➕ 新增资产</button>
                    </div>
                    <div id="addAssetInline" style="display:none; background:#f8f9fa; padding:8px; margin:8px 0; border-radius:6px;">
                        <form action="/admin/assets/add" method="POST" style="display:flex; gap:6px; flex-wrap:wrap;">
                            <input type="text" name="fund_name" placeholder="资产名称" required style="flex:1;min-width:120px;padding:4px;">
                            <input type="text" name="fund_code" placeholder="基金代码" required style="width:90px;padding:4px;">
                            <input type="text" name="sector" placeholder="赛道(可选)" style="width:90px;padding:4px;">
                            <button type="submit" class="btn btn-sm">创建</button>
                            <button type="button" onclick="document.getElementById('addAssetInline').style.display='none'" class="btn-ghost">取消</button>
                        </form>
                    </div>
                    <div class="table-container" style="margin-top:8px;">
                        <div class="table-wrap">
                            <table style="font-size:0.85rem;">
                                <thead>
                                    <tr>
                                        <th>名称/ID</th>
                                        <th>基金代码</th>
                                        <th>赛道</th>
                                        <th>状态</th>
                                        <th class="text-right">操作 (编辑/归档/+定投)</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {}
                                </tbody>
                            </table>
                        </div>
                    </div>
                    <small style="color:var(--text-muted);">未分类资产将影响仓位统计；可在此设置赛道或归档。</small>
                </div>

                <script>
                    async function createDcaForAsset(assetId, fundCode) {{
                        const amt = parseFloat(prompt("定投金额 (CNY, 默认1000):", "1000") || "1000");
                        if (!amt || amt <= 0) return;
                        const freq = prompt("频率 daily/weekly/monthly (默认monthly):", "monthly") || "monthly";
                        try {{
                            const resp = await fetch('/api/dca/plans', {{
                                method: 'POST',
                                headers: {{'Content-Type': 'application/json'}},
                                body: JSON.stringify({{ asset_id: assetId, amount: amt, frequency: freq, day: freq==="monthly"?1:(freq==="weekly"?1:null), note: "从持仓创建" }})
                            }});
                            const data = await resp.json();
                            if (data && (data.success || data.executed_count !== undefined)) {{
                                alert('定投计划已创建: ' + (data.message || '成功'));
                                location.reload();
                            }} else {{
                                alert('创建失败: ' + (data.message || JSON.stringify(data)));
                            }}
                        }} catch(e) {{ alert('网络错误: '+e); }}
                    }}
                async function autoClassify(el) {{
                    if (el) el.disabled = true;
                    try {{
                        await fetch('/api/jobs/assets/auto-classify', {{method:'POST'}});
                        location.reload();
                    }} catch(e){{ alert('失败:'+e); if(el) el.disabled=false; }}
                }}
                </script>

                "#,
                bootstrap_html, display_book, alipay_total, diff_class, diff, rows, asset_mgmt_rows
            );

            layout_with_msg("持仓", content, query.success, query.error)
        }
        Err(e) => layout(
            "持仓",
            format!(
                "<div class='message-banner message-error'>数据加载失败: {}</div>",
                e
            ),
        ),
    }
}
