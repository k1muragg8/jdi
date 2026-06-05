//! Market (市场) page HTML.

use crate::web::view_models::market::MarketPageVm;

pub fn render(vm: &MarketPageVm) -> String {
    format!(
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
        vm.cleanup_confirm_msg,
        vm.filter_tabs_html,
        vm.last_refresh,
        vm.cache_depth,
        vm.mon_count,
        vm.fail_count_color,
        vm.fail_count,
        vm.inst_rows_html
    )
}
