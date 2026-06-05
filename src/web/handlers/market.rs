//! Market watchlist handlers.

use crate::web::response::AdminQuery;
use crate::web::services::market_service;
use crate::web::state::AppState;
use crate::web::views::layout::{layout, layout_with_msg};
use crate::{engine, models};
use axum::extract::{Query, State};
use axum::response::Html;
use std::sync::Arc;

pub async fn instruments_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AdminQuery>,
) -> Html<String> {
    let ctx = &state.ctx;
    match market_service::load_market_page(&state).await {
        Ok(page) => {
            let instruments = page.instruments;
            let market_cache = page.market_cache;
            let cleanup_confirm_msg = page.cleanup_confirm_msg;
            let dup_ids = page.dup_ids;
            let list_filter = engine::MarketListFilter::from_query(query.filter.as_deref());
            let filtered: Vec<&models::InstrumentConfig> = instruments
                .iter()
                .filter(|i| engine::matches_filter(i, list_filter, &dup_ids))
                .collect();

            let filter_tabs = format!(
                r#"<div class="market-filter-bar" style="display:flex;flex-wrap:wrap;gap:8px;margin-bottom:12px;align-items:center;">
                    <span style="font-size:0.85rem;color:var(--text-muted);font-weight:600;">筛选:</span>
                    <a href="/market?filter=active" class="btn btn-sm {}">监控中</a>
                    <a href="/market?filter=all" class="btn btn-sm {}">全部</a>
                    <a href="/market?filter=disabled" class="btn btn-sm {}">已禁用</a>
                    <a href="/market?filter=archived" class="btn btn-sm {}">已归档</a>
                    <a href="/market?filter=test" class="btn btn-sm {}">重复/测试</a>
                </div>"#,
                if list_filter == engine::MarketListFilter::Active {
                    ""
                } else {
                    "btn-outline"
                },
                if list_filter == engine::MarketListFilter::All {
                    ""
                } else {
                    "btn-outline"
                },
                if list_filter == engine::MarketListFilter::Disabled {
                    ""
                } else {
                    "btn-outline"
                },
                if list_filter == engine::MarketListFilter::Archived {
                    ""
                } else {
                    "btn-outline"
                },
                if list_filter == engine::MarketListFilter::Test {
                    ""
                } else {
                    "btn-outline"
                },
            );

            let mut inst_rows = String::new();
            let cache_map: std::collections::HashMap<String, &models::MarketCacheEntry> =
                market_cache
                    .entries
                    .iter()
                    .map(|e| (e.symbol.clone(), e))
                    .collect();

            for inst in &filtered {
                let quote = cache_map.get(&inst.symbol);
                let price_html = if let Some(q) = quote {
                    if q.price > 0.0 || q.status.as_deref() == Some("ok") {
                        format!("<span class='price-cell tabular'>{:.2}</span>", q.price)
                    } else {
                        "<span class='text-muted'>—</span>".to_string()
                    }
                } else {
                    "<span class='text-muted'>—</span>".to_string()
                };

                let is_dup = dup_ids.contains(&inst.instrument_id);
                let is_test = engine::is_test_instrument(inst);
                let is_arch = engine::is_instrument_archived(inst);
                let status_badge = if is_dup {
                    "<span class='badge badge-orange'>重复标的</span>"
                } else if is_test {
                    "<span class='badge badge-orange'>测试标的</span>"
                } else if is_arch {
                    "<span class='badge badge-gray'>已归档</span>"
                } else if inst.enabled {
                    "<span class='badge badge-blue'>监控中</span>"
                } else {
                    "<span class='badge badge-gray'>未启用</span>"
                };

                let (chg_html, chg_pct_html) = if let Some(q) = quote {
                    if let Some(ch) = q.change {
                        let pct = q.change_percent.unwrap_or(0.0);
                        let cls = if ch >= 0.0 { "text-up" } else { "text-down" };
                        (
                            format!("<span class='{} tabular'>{:+.2}</span>", cls, ch),
                            format!("<span class='{} tabular'>{:+.2}%</span>", cls, pct),
                        )
                    } else if q.price > 0.0 {
                        (
                            "<span class='text-muted tabular'>暂无</span>".to_string(),
                            "<span style='font-size:0.65rem;'>未返回昨收</span>".to_string(),
                        )
                    } else {
                        ("-".to_string(), "-".to_string())
                    }
                } else {
                    ("-".to_string(), "-".to_string())
                };

                let row_status = if is_dup {
                    status_badge.to_string()
                } else if let Some(q) = quote {
                    if q.status.as_deref() == Some("ok") && q.price > 0.0 {
                        "<span class='badge badge-green'>已更新</span>".to_string()
                    } else if let Some(em) = &q.error_message {
                        format!("<span class='badge badge-red' title='{}'>失败</span>", em)
                    } else {
                        "<span class='badge badge-gray'>暂无</span>".to_string()
                    }
                } else {
                    status_badge.to_string()
                };

                let display_nm = inst.name_zh.as_deref().unwrap_or(&inst.symbol);
                let prov = quote
                    .map(|q| q.source.as_str())
                    .unwrap_or(inst.provider.as_str());
                let id_for_form = &inst.instrument_id;
                let ac_str = format!("{:?}", inst.asset_class);

                let enable_disable_form = if inst.enabled {
                    format!(
                        r#"<form action="/admin/instruments/disable" method="POST" style="display:inline;" onsubmit="return confirm('禁用此标的?');"><input type="hidden" name="instrument_id" value="{}"><button type="submit" class="btn-ghost" style="font-size:0.7rem;">禁用</button></form>"#,
                        id_for_form
                    )
                } else {
                    format!(
                        r#"<form action="/admin/instruments/enable" method="POST" style="display:inline;"><input type="hidden" name="instrument_id" value="{}"><button type="submit" class="btn-ghost" style="font-size:0.7rem;">启用</button></form>"#,
                        id_for_form
                    )
                };

                let archive_form = format!(
                    r#"<form action="/admin/instruments/archive" method="POST" style="display:inline;" onsubmit="return confirm('归档此标的?');"><input type="hidden" name="instrument_id" value="{}"><button type="submit" class="btn-ghost" style="font-size:0.7rem;color:#c00;">归档</button></form>"#,
                    id_for_form
                );

                let edit_btn = format!(
                    r#"<button type="button" class="btn-ghost" style="font-size:0.7rem;" onclick="openInstEdit(this)"
                        data-id="{}" data-name="{}" data-symbol="{}" data-provider="{}" data-psym="{}" data-currency="{}" data-class="{}">编辑</button>"#,
                    id_for_form,
                    display_nm.replace('"', "&quot;"),
                    inst.symbol,
                    inst.provider,
                    inst.provider_symbol,
                    inst.currency,
                    ac_str
                );

                inst_rows.push_str(&format!(
                    "<tr>
                        <td><div style='font-weight:700;'>{}</div></td>
                        <td><code style='font-size:0.75rem;'>{}</code></td>
                        <td><span class='badge badge-outline' style='font-size:0.65rem;'>{:?}</span></td>
                        <td class='tabular price-cell'>{}</td>
                        <td class='tabular'>{}</td>
                        <td class='tabular'>{}</td>
                        <td>{}</td>
                        <td style='font-size:0.75rem;'>{}</td>
                        <td>{}</td>
                        <td class='text-right' style='white-space:nowrap;'>
                            {}<button class='btn-ghost' onclick='refreshOneSymbol(\"{}\")' style='font-size:0.7rem;'>刷新</button>
                            {}{}
                        </td>
                    </tr>",
                    display_nm,
                    inst.symbol,
                    inst.asset_class,
                    price_html,
                    chg_html,
                    chg_pct_html,
                    inst.currency,
                    prov,
                    row_status,
                    edit_btn,
                    inst.symbol,
                    enable_disable_form,
                    archive_form
                ));
            }

            if inst_rows.is_empty() {
                let empty_hint = match list_filter {
                    engine::MarketListFilter::Archived => "当前筛选下没有已归档标的",
                    engine::MarketListFilter::Test => "当前筛选下没有测试/重复标的",
                    engine::MarketListFilter::Disabled => "当前筛选下没有已禁用标的",
                    _ => "暂无监控标的，请先新增、恢复默认或切换筛选",
                };
                inst_rows = format!(
                    "<tr><td colspan='10'><div class='empty-state'><span class='empty-state-icon'>📉</span><div class='empty-state-text'>{}</div><span style='font-size:0.7rem;'>使用上方表单新增，或点击「恢复默认标的」</span></div></td></tr>",
                    empty_hint
                );
            }

            // Use MarketCache (quote cache) as source of truth for summary cards, for consistency with table
            let last_refresh = market_cache
                .entries
                .iter()
                .map(|e| e.fetched_at.as_str())
                .max()
                .unwrap_or("从未刷新")
                .to_string();
            let cache_depth = market_cache.entries.len();
            let market_job_opt = state
                .repo
                .get_latest_job(&ctx, "market_refresh")
                .await
                .unwrap_or(None);
            let fail_count = if let Some(j) = &market_job_opt {
                j.result_json
                    .as_ref()
                    .and_then(|v| v.get("failed_count").and_then(|x| x.as_u64()))
                    .unwrap_or(0) as usize
            } else {
                0
            };
            let mon_count = instruments
                .iter()
                .filter(|i| engine::matches_filter(i, engine::MarketListFilter::Active, &dup_ids))
                .count();

            let content = format!(
                r#"
                <div style="display:flex;justify-content:space-between;align-items:flex-end;margin-bottom:16px;flex-wrap:wrap;gap:10px;">
                    <div>
                        <h1 style="margin-bottom:4px;">市场</h1>
                        <p style="color:var(--text-muted);font-size:0.88rem;margin:0;">监控标的行情，用于估值与仓位参考</p>
                    </div>
                    <div class="market-toolbar">
                        <button id="refreshBtn" onclick="startMarketRefresh(this)" class="btn btn-sm">刷新全部行情</button>
                        <button type="button" class="btn btn-outline btn-sm" onclick="document.getElementById('addInstPanel').style.display='block'">新增标的</button>
                        <form action="/admin/instruments/restore-defaults" method="POST" style="display:inline;" onsubmit="return confirm('恢复默认标的？');">
                            <button type="submit" class="btn btn-outline btn-sm">恢复默认</button>
                        </form>
                        <form action="/admin/instruments/cleanup-test" method="POST" style="display:inline;" onsubmit="return confirm('{}');">
                            <input type="hidden" name="confirm" value="1">
                            <button type="submit" class="btn btn-outline btn-sm">清理测试</button>
                        </form>
                    </div>
                </div>

                <style>
                    .market-input {{ padding:8px 10px; font-size:0.9rem; border:1px solid var(--border-color); border-radius:6px; }}
                    .market-input-name {{ min-width:260px; width:100%; }}
                    .market-input-symbol {{ min-width:160px; width:100%; }}
                    .market-input-select {{ min-width:140px; padding:8px; font-size:0.9rem; }}
                    .market-input-currency {{ min-width:120px; }}
                    .market-input-class {{ min-width:160px; }}
                    .market-crud-form {{ display:flex; flex-wrap:wrap; gap:10px; align-items:flex-end; }}
                    .table-wrap {{ overflow-x:auto; max-height:70vh; }}
                </style>

                {}

                <div class="market-summary-grid">
                    <div class="card"><div class="card-header"><span class="card-title">最近同步</span></div><div class="card-value" style="font-size:1rem;">{}</div></div>
                    <div class="card"><div class="card-header"><span class="card-title">缓存深度</span></div><div class="card-value">{}</div></div>
                    <div class="card"><div class="card-header"><span class="card-title">监控标的</span></div><div class="card-value">{}</div></div>
                    <div class="card"><div class="card-header"><span class="card-title">失败</span></div><div class="card-value" style="color:{};">{}</div></div>
                </div>

                <div id="addInstPanel" class="card" style="margin-bottom:12px;background:#f8f9fa;padding:14px;display:none;">
                    <h3 style="margin:0 0 10px;font-size:0.95rem;">新增标的</h3>
                    <form action="/admin/instruments/add" method="POST" class="market-crud-form">
                        <label style="display:flex;flex-direction:column;gap:4px;font-size:0.75rem;color:var(--text-muted);">代码
                            <input type="text" name="symbol" placeholder="QQQ" required class="market-input market-input-symbol">
                        </label>
                        <label style="display:flex;flex-direction:column;gap:4px;font-size:0.75rem;color:var(--text-muted);">显示名
                            <input type="text" name="name_zh" placeholder="可选" class="market-input market-input-name">
                        </label>
                        <label style="display:flex;flex-direction:column;gap:4px;font-size:0.75rem;color:var(--text-muted);">类型
                            <select name="asset_class" class="market-input-select market-input-class">
                            <option value="Index">指数</option>
                            <option value="Etf">ETF</option>
                            <option value="Crypto">加密</option>
                            <option value="Fx">外汇</option>
                            <option value="SpotCommodity">商品</option>
                        </select></label>
                        <label style="display:flex;flex-direction:column;gap:4px;font-size:0.75rem;color:var(--text-muted);">数据源
                            <select name="provider" class="market-input-select">
                            <option value="yahoo">yahoo</option>
                            <option value="eastmoney">eastmoney</option>
                        </select></label>
                        <label style="display:flex;flex-direction:column;gap:4px;font-size:0.75rem;color:var(--text-muted);">币种
                            <input type="text" name="currency" value="USD" class="market-input market-input-currency">
                        </label>
                        <button type="submit" class="btn btn-sm">新增</button>
                        <button type="button" class="btn btn-outline btn-sm" onclick="document.getElementById('addInstPanel').style.display='none'">取消</button>
                    </form>
                </div>

                <div class="table-container">
                    <div class="table-wrap">
                        <table class="market-compact">
                            <thead>
                                <tr>
                                    <th>名称</th>
                                    <th>代码</th>
                                    <th>类型</th>
                                    <th>最新价</th>
                                    <th>涨跌</th>
                                    <th>涨跌幅</th>
                                    <th>币种</th>
                                    <th>数据源</th>
                                    <th>状态</th>
                                    <th class="text-right">操作</th>
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                    </div>
                </div>

                <div id="instEditModal" class="modal-overlay" onclick="if(event.target===this)closeInstEdit()">
                    <div class="modal-panel" onclick="event.stopPropagation()">
                        <h3 style="margin:0 0 14px;">编辑标的</h3>
                        <form id="instEditForm" action="/admin/instruments/update-metadata" method="POST" class="market-crud-form" style="flex-direction:column;align-items:stretch;">
                            <input type="hidden" name="instrument_id" id="editInstId">
                            <label style="font-size:0.8rem;color:var(--text-muted);">显示名
                                <input type="text" name="name_zh" id="editInstName" class="market-input market-input-name">
                            </label>
                            <label style="font-size:0.8rem;color:var(--text-muted);">代码（只读）
                                <input type="text" id="editInstSymbol" class="market-input market-input-symbol" readonly style="background:#f5f5f5;">
                            </label>
                            <label style="font-size:0.8rem;color:var(--text-muted);">数据源
                                <select name="provider" id="editInstProvider" class="market-input-select">
                                    <option value="yahoo">yahoo</option>
                                    <option value="eastmoney">eastmoney</option>
                                    <option value="manual">manual</option>
                                </select>
                            </label>
                            <label style="font-size:0.8rem;color:var(--text-muted);">provider_symbol
                                <input type="text" name="provider_symbol" id="editInstPsym" class="market-input market-input-symbol">
                            </label>
                            <div style="display:flex;gap:8px;margin-top:8px;">
                                <button type="submit" class="btn btn-sm">保存</button>
                                <button type="button" class="btn btn-outline btn-sm" onclick="closeInstEdit()">取消</button>
                            </div>
                        </form>
                    </div>
                </div>

                <script>
                    function openInstEdit(btn) {{
                        document.getElementById('editInstId').value = btn.dataset.id;
                        document.getElementById('editInstName').value = btn.dataset.name || '';
                        document.getElementById('editInstSymbol').value = btn.dataset.symbol || '';
                        document.getElementById('editInstProvider').value = btn.dataset.provider || 'yahoo';
                        document.getElementById('editInstPsym').value = btn.dataset.psym || '';
                        document.getElementById('instEditModal').classList.add('open');
                    }}
                    function closeInstEdit() {{
                        document.getElementById('instEditModal').classList.remove('open');
                    }}
                    async function refreshOneSymbol(sym) {{
                        try {{
                            const res = await fetch('/api/market/refresh-symbol', {{
                                method: 'POST',
                                headers: {{ 'Content-Type': 'application/json' }},
                                body: JSON.stringify({{ symbol: sym }})
                            }});
                            const r = await res.json();
                            if (r.success) {{
                                location.reload();
                            }} else {{
                                alert('刷新失败: ' + (r.message || ''));
                            }}
                        }} catch (e) {{
                            alert('网络错误: ' + e);
                        }}
                    }}
                    async function startMarketRefresh(btn) {{
                        if (btn) {{
                            btn.disabled = true;
                            btn.innerText = '⏳ 正在刷新...';
                        }}
                        try {{
                            const res = await fetch('/api/jobs/market/refresh', {{ method: 'POST' }});
                            const jr = await res.json();
                            if (jr.status === 'error') {{
                                alert('失败: ' + (jr.message || ''));
                                if (btn) {{ btn.disabled = false; btn.innerText = '📈 刷新全部行情'; }}
                                return;
                            }}
                            // poll until done then reload
                            const iv = setInterval(async () => {{
                                try {{
                                    const s = await fetch('/api/jobs/market/status');
                                    const d = await s.json();
                                    if (!d.is_running && (!d.job || (d.job.status !== 'queued' && d.job.status !== 'running'))) {{
                                        clearInterval(iv);
                                        location.reload();
                                    }}
                                }} catch(e) {{}}
                            }}, 1500);
                        }} catch (e) {{
                            alert('网络错误: ' + e);
                            if (btn) {{ btn.disabled = false; btn.innerText = '📈 刷新全部行情'; }}
                        }}
                    }}
                    // auto poll if running on load
                    (function initMarket() {{
                        fetch('/api/jobs/market/status').then(r=>r.json()).then(d => {{
                            if (d.is_running) {{
                                const b = document.getElementById('refreshBtn');
                                if (b) {{ b.disabled = true; b.innerText = '⏳ 正在刷新...'; }}
                                const iv = setInterval(async () => {{
                                    try {{
                                        const s = await fetch('/api/jobs/market/status');
                                        const dd = await s.json();
                                        if (!dd.is_running) {{ clearInterval(iv); location.reload(); }}
                                    }} catch(e){{}}
                                }}, 1500);
                            }}
                        }}).catch(() => {{}});
                    }})();
                </script>
                "#,
                cleanup_confirm_msg,
                filter_tabs,
                last_refresh,
                cache_depth,
                mon_count,
                if fail_count > 0 {
                    "var(--warn-color)"
                } else {
                    "var(--text-muted)"
                },
                fail_count,
                inst_rows
            );

            layout_with_msg("市场", content, query.success, query.error)
        }
        Err(e) => layout(
            "市场",
            format!(
                "<div class='message-banner message-error'>数据加载失败: {}</div>",
                e
            ),
        ),
    }
}
