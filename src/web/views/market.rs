//! Market (市场) page HTML.

use crate::engine;
use crate::web::view_models::market::MarketPageVm;

pub fn render(vm: &MarketPageVm) -> String {
    let current_filter = match vm.list_filter {
        engine::MarketListFilter::All => "all",
        engine::MarketListFilter::Enabled => "enabled",
        engine::MarketListFilter::Disabled => "disabled",
        engine::MarketListFilter::Archived => "archived",
        engine::MarketListFilter::Test => "test",
        engine::MarketListFilter::Duplicate => "duplicate",
        _ => "active",
    };

    format!(
        r#"
        <style>
            .market-crud-form {{ display:flex; gap:8px; align-items:center; }}
            .market-input {{ padding:6px 10px; border:1px solid var(--border-color); border-radius:6px; font-size:0.85rem; min-width:260px; }}
            .market-input-name {{ min-width:180px; }}
            .market-input-symbol {{ min-width:120px; }}
            .market-top-actions {{ display:flex; gap:12px; align-items:center; }}
            .market-table th {{ position:sticky; top:0; background:var(--bg-color); z-index:10; }}
            .market-stats-row {{ display:flex; gap:20px; margin-bottom:16px; font-size:0.85rem; color:var(--text-muted); background:var(--card-bg); padding:12px 16px; border-radius:8px; border:1px solid var(--border-color); }}
            .market-stat-item span {{ font-weight:700; color:var(--text-main); margin-left:4px; }}
            .actions-col {{ width:240px; }}
            .price-cell {{ font-weight:700; color:var(--text-main); }}
        </style>
        <div style="display:flex;justify-content:space-between;align-items:flex-end;margin-bottom:14px;flex-wrap:wrap;gap:12px;">
            <div>
                <h1 style="margin-bottom:4px;">市场监控</h1>
                <p style="color:var(--text-muted);font-size:0.9rem;margin:0;">管理标的行情源、汇率与自动刷新</p>
            </div>
            <div class="market-top-actions">
                <button type="button" class="btn btn-sm" onclick="refreshMarket(this)">刷新全部行情</button>
                <form action="/admin/instruments/add" method="POST" class="market-crud-form">
                    <input type="hidden" name="filter" value="{}">
                    <input type="text" name="symbol" placeholder="代码 (如 BTC-USD)" class="market-input" required>
                    <button type="submit" class="btn btn-sm">新增标的</button>
                    <button type="button" class="btn btn-sm btn-outline" onclick="openRestoreDefaults()">恢复默认</button>
                    <button type="button" class="btn btn-sm btn-outline" onclick="openCleanupTest()">清理测试</button>
                </form>
            </div>
        </div>

        {}

        <div style="margin-bottom:16px;">
            {}
        </div>

        <div class="market-stats-row">
            <div class="market-stat-item">最近刷新：<span>{}</span></div>
            <div class="market-stat-item">监控深度：<span>{}</span></div>
            <div class="market-stat-item">监控中：<span>{}</span></div>
            <div class="market-stat-item">失败标的：<span style="color:{}">{}</span></div>
        </div>

        <div class="card">
            <div class="table-container"><div class="table-wrap">
                <table class="market-table">
                    <thead>
                        <tr>
                            <th>名称</th>
                            <th>代码</th>
                            <th>类型</th>
                            <th class="text-right">现价</th>
                            <th class="text-right">涨跌</th>
                            <th class="text-right">幅度</th>
                            <th>币种</th>
                            <th>源</th>
                            <th>状态</th>
                            <th class="text-right">操作</th>
                        </tr>
                    </thead>
                    <tbody>
                        {}
                    </tbody>
                </table>
            </div></div>
        </div>

        <div id="instEditModal" class="modal-overlay" onclick="if(event.target===this)closeInstEdit()">
            <div class="modal-panel" onclick="event.stopPropagation()">
                <h3 style="margin:0 0 14px;">编辑标的</h3>
                <form id="instEditForm" action="/admin/instruments/update-metadata" method="POST" class="market-crud-form" style="flex-direction:column;align-items:stretch;">
                    <input type="hidden" name="instrument_id" id="editInstId">
                    <input type="hidden" name="filter" id="editInstFilter">
                    <label style="font-size:0.8rem;color:var(--text-muted);">显示名
                        <input type="text" name="name_zh" id="editInstName" class="market-input market-input-name">
                    </label>
                    <label style="font-size:0.8rem;color:var(--text-muted);">代码（只读）
                        <input type="text" id="editInstSymbol" class="market-input market-input-symbol" readonly style="background:#f5f5f5;">
                    </label>
                    <label style="font-size:0.8rem;color:var(--text-muted);">行情源
                        <select name="provider" id="editInstProvider" class="market-input">
                            <option value="yahoo">Yahoo Finance</option>
                            <option value="eastmoney">Eastmoney (东方财富)</option>
                            <option value="eastmoney_market">Eastmoney Market (A股/港美股行情)</option>
                        </select>
                    </label>
                    <label style="font-size:0.8rem;color:var(--text-muted);">源对应代码
                        <input type="text" name="provider_symbol" id="editInstPsym" class="market-input">
                    </label>
                    <div style="margin-top:16px;display:flex;justify-content:flex-end;gap:10px;">
                        <button type="button" class="btn btn-outline" onclick="closeInstEdit()">取消</button>
                        <button type="submit" class="btn">保存修改</button>
                    </div>
                </form>
            </div>
        </div>

        <div id="cleanupTestModal" class="modal-overlay" onclick="if(event.target===this)closeCleanupTest()">
            <div class="modal-panel" onclick="event.stopPropagation()">
                <h3 style="margin:0 0 14px;">清理测试标的</h3>
                <p style="margin-bottom:12px;font-size:0.9rem;">{}</p>
                <form action="/admin/instruments/cleanup-test" method="POST">
                    <input type="hidden" name="filter" value="{}">
                    <input type="hidden" name="confirm" value="1">
                    <div style="display:flex;justify-content:flex-end;gap:10px;">
                        <button type="button" class="btn btn-outline" onclick="closeCleanupTest()">取消</button>
                        <button type="submit" class="btn btn-danger">立即归档</button>
                    </div>
                </form>
            </div>
        </div>

        <div id="restoreDefaultsModal" class="modal-overlay" onclick="if(event.target===this)closeRestoreDefaults()">
            <div class="modal-panel" onclick="event.stopPropagation()">
                <h3 style="margin:0 0 14px;">恢复默认标的</h3>
                <p style="margin-bottom:12px;font-size:0.9rem;">将从内置配置中恢复常用标的（如标普500、纳指100、主要汇率等）。</p>
                <form action="/admin/instruments/restore-defaults" method="POST">
                    <input type="hidden" name="filter" value="{}">
                    <div style="margin-bottom:16px;">
                        <label style="display:flex;align-items:center;gap:8px;font-size:0.9rem;cursor:pointer;">
                            <input type="checkbox" name="cleanup_test" value="1" checked> 同时清理（归档）当前测试标的
                        </label>
                    </div>
                    <div style="display:flex;justify-content:flex-end;gap:10px;">
                        <button type="button" class="btn btn-outline" onclick="closeRestoreDefaults()">取消</button>
                        <button type="submit" class="btn">确认恢复</button>
                    </div>
                </form>
            </div>
        </div>

        <script>
            let autoEnabled = true;
            let secondsToNext = 60;
            let refreshTimer = null;

            function updateAutoUI() {{
                const status = document.getElementById('autoStatus');
                const next = document.getElementById('nextRefresh');
                const btn = document.getElementById('toggleAutoBtn');
                if (!status || !next || !btn) return;
                
                if (autoEnabled) {{
                    status.textContent = '自动刷新：开启';
                    next.textContent = '下次刷新：' + secondsToNext + ' 秒后';
                    btn.textContent = '暂停自动刷新';
                }} else {{
                    status.textContent = '自动刷新：已暂停';
                    next.textContent = '—';
                    btn.textContent = '恢复自动刷新';
                }}
            }}

            function tick() {{
                if (!autoEnabled) return;
                secondsToNext--;
                if (secondsToNext <= 0) {{
                    location.reload();
                }} else {{
                    updateAutoUI();
                }}
            }}

            function toggleAutoRefresh(btn) {{
                autoEnabled = !autoEnabled;
                updateAutoUI();
            }}

            if (refreshTimer) clearInterval(refreshTimer);
            refreshTimer = setInterval(tick, 1000);

            function openInstEdit(btn) {{
                document.getElementById('editInstId').value = btn.dataset.id;
                document.getElementById('editInstName').value = btn.dataset.name || '';
                document.getElementById('editInstSymbol').value = btn.dataset.symbol || '';
                document.getElementById('editInstProvider').value = btn.dataset.provider || 'yahoo';
                document.getElementById('editInstPsym').value = btn.dataset.psym || '';
                if (document.getElementById('editInstFilter')) {{
                    document.getElementById('editInstFilter').value = btn.dataset.filter || '';
                }}
                document.getElementById('instEditModal').classList.add('open');
                autoEnabled = false;
                updateAutoUI();
            }}
            function closeInstEdit() {{
                document.getElementById('instEditModal').classList.remove('open');
                if (autoEnabled) {{
                    secondsToNext = 60;
                    updateAutoUI();
                }}
            }}
            function openCleanupTest() {{
                document.getElementById('cleanupTestModal').classList.add('open');
            }}
            function closeCleanupTest() {{
                document.getElementById('cleanupTestModal').classList.remove('open');
            }}
            function openRestoreDefaults() {{
                document.getElementById('restoreDefaultsModal').classList.add('open');
            }}
            function closeRestoreDefaults() {{
                document.getElementById('restoreDefaultsModal').classList.remove('open');
            }}

            async function refreshOneSymbol(sym, btn) {{
                if (btn) {{
                    btn.disabled = true;
                    btn.textContent = '...';
                }}
                try {{
                    const res = await fetch('/api/market/refresh?symbol=' + encodeURIComponent(sym), {{ method: 'POST' }});
                    const data = await res.json();
                    if (data.success) {{
                        location.reload();
                    }} else {{
                        alert('刷新失败: ' + (data.message || '未知错误'));
                        if (btn) {{
                            btn.disabled = false;
                            btn.textContent = '刷新';
                        }}
                    }}
                }} catch(e) {{
                    alert('网络错误');
                    if (btn) {{
                        btn.disabled = false;
                        btn.textContent = '刷新';
                    }}
                }}
            }}
        </script>
        "#,
        current_filter,
        vm.auto_refresh_html,
        vm.filter_tabs_html,
        vm.last_refresh,
        vm.cache_depth,
        vm.mon_count,
        vm.fail_count_color,
        vm.fail_count,
        vm.inst_rows_html,
        vm.cleanup_confirm_msg,
        current_filter,
        current_filter
    )
}
