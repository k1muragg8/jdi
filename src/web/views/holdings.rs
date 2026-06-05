//! Holdings (持仓) page HTML.

use crate::web::view_models::holdings::HoldingsPageVm;
use crate::web::views::components::{admin_extra_css, admin_js_core};

pub fn render(vm: &HoldingsPageVm) -> String {
    let arch_link = if vm.show_archived {
        r#"<a href="/holdings" class="btn btn-sm">显示活跃</a>"#
    } else {
        r#"<a href="/holdings?filter=archived" class="btn btn-sm btn-outline">显示已归档</a>"#
    };
    format!(
        r#"
        <style>{}</style>
        <div id="adminToast"></div>
        <div style="display:flex;justify-content:space-between;align-items:flex-end;margin-bottom:12px;flex-wrap:wrap;gap:12px;">
            <div>
                <h1 style="margin-bottom:4px;">持仓</h1>
                <p style="color:var(--text-muted);font-size:0.9rem;margin:0;">管理资产、基金代码、赛道与定投；数值来自本地账本与支付宝快照</p>
            </div>
        </div>

        <div class="admin-toolbar">
            <button type="button" class="btn btn-sm" onclick="openDrawer('addAssetDrawer')">新增资产</button>
            <button type="button" class="btn btn-outline btn-sm" id="btnEnrichAll" onclick="enrichAllAssets(this)">自动补全基金信息</button>
            <button type="button" class="btn btn-outline btn-sm" onclick="runBtnAction(this, ()=>adminFetch('/api/jobs/assets/auto-classify',{{method:'POST'}}))">自动分类</button>
            <button type="button" class="btn btn-outline btn-sm" onclick="runBtnAction(this, ()=>adminFetch('/api/jobs/nav/refresh',{{method:'POST'}}))">刷新基金净值</button>
            <form action="/api/holdings/bootstrap-alipay" method="POST" style="display:inline;" onsubmit="return confirm('用最新支付宝快照初始化本地持仓？');">
                <button type="submit" class="btn btn-outline btn-sm">用支付宝快照初始化持仓</button>
            </form>
            <button type="button" class="btn btn-outline btn-sm" onclick="openDrawer('importDrawer')">导入支付宝快照</button>
            {arch_link}
        </div>

        {}

        <div class="overview-metrics" style="margin-bottom:16px;">
            <div class="card"><div class="card-header"><span class="card-title">系统账面</span><div class="source-hint">本地持仓汇总</div></div><div class="card-value tabular">{:.2}</div></div>
            <div class="card"><div class="card-header"><span class="card-title">支付宝快照</span><div class="source-hint">最新快照合计</div></div><div class="card-value tabular">{:.2}</div></div>
            <div class="card"><div class="card-header"><span class="card-title">差额</span><div class="source-hint">账面 − 快照</div></div><div class="card-value tabular {}">{:+.2}</div></div>
        </div>

        <div class="card" style="margin-bottom:16px;">
            <div class="card-header"><span class="card-title">持仓明细</span></div>
            <div class="table-container"><div class="table-wrap">
                <table class="holdings-compact">
                    <thead><tr>
                        <th>资产 / 代码</th><th>赛道</th><th>地区</th>
                        <th class="text-right">市值</th><th class="text-right">份额</th><th class="text-right">净值</th>
                        <th class="text-right">盈亏</th><th class="text-right">差异</th><th class="text-right">操作</th>
                    </tr></thead>
                    <tbody>{}</tbody>
                </table>
            </div></div>
        </div>

        <div class="card">
            <div class="card-header"><span class="card-title">资产配置</span></div>
            <div class="table-container"><div class="table-wrap">
                <table class="holdings-compact" style="font-size:0.85rem;">
                    <thead><tr>
                        <th>名称</th><th>基金代码</th><th>赛道</th><th>地区</th><th>来源</th><th>状态</th><th class="text-right">操作</th>
                    </tr></thead>
                    <tbody>{}</tbody>
                </table>
            </div></div>
        </div>

        <div id="assetEditDrawer" class="drawer-overlay" onclick="if(event.target===this)closeDrawer('assetEditDrawer')">
            <div class="drawer-panel" onclick="event.stopPropagation()">
                <h3 style="margin:0 0 12px;">编辑资产</h3>
                <div class="form-grid">
                    <label>资产ID<input id="aeId" readonly style="background:#f5f5f5;"></label>
                    <label>基金代码<input id="aeCode"></label>
                    <label style="grid-column:1/-1;">显示名称<input id="aeName"></label>
                    <label>赛道/分类<input id="aeSector" placeholder="如 美国科技、债券、黄金"></label>
                    <label>地区<span id="aeRegion" class="source-hint"></span></label>
                    <label>币种<input id="aeCurrency" value="CNY"></label>
                    <label>估值方式<input id="aeValMethod" value="nav"></label>
                    <label>行情数据源<input id="aeProvider" placeholder="eastmoney"></label>
                    <label>基准代码<input id="aeBench" placeholder="QQQ"></label>
                    <label>关联标的代码<input id="aeInstSym"></label>
                </div>
                <div class="form-actions">
                    <button type="button" class="btn btn-sm" id="aeSaveBtn" onclick="saveAssetEdit(this)">保存</button>
                    <button type="button" class="btn btn-outline btn-sm" onclick="lookupFundForEdit(this)">查询基金</button>
                    <button type="button" class="btn btn-outline btn-sm" onclick="enrichCurrentAsset(this)">自动补全</button>
                    <form id="aeArchiveForm" method="POST" action="/admin/assets/remove" style="display:inline;" onsubmit="return confirm('归档此资产？');">
                        <input type="hidden" name="asset_id" id="aeArchiveId">
                        <button type="submit" class="btn btn-outline btn-sm">归档</button>
                    </form>
                    <button type="button" class="btn-ghost btn-sm" onclick="closeDrawer('assetEditDrawer')">取消</button>
                </div>
                <div id="aeLookupHint" class="source-hint" style="margin-top:8px;"></div>
            </div>
        </div>

        <div id="addAssetDrawer" class="drawer-overlay" onclick="if(event.target===this)closeDrawer('addAssetDrawer')">
            <div class="drawer-panel" onclick="event.stopPropagation()">
                <h3 style="margin:0 0 12px;">新增资产</h3>
                <form action="/admin/assets/add" method="POST" class="form-grid" onsubmit="return true;">
                    <label style="grid-column:1/-1;">资产名称<input name="fund_name" required></label>
                    <label>基金代码<input name="fund_code" id="newFundCode" required></label>
                    <label>赛道<input name="sector" placeholder="可选"></label>
                    <div class="form-actions" style="grid-column:1/-1;">
                        <button type="button" class="btn btn-outline btn-sm" onclick="lookupNewFund(this)">查询基金</button>
                        <button type="submit" class="btn btn-sm">创建</button>
                        <button type="button" class="btn-ghost btn-sm" onclick="closeDrawer('addAssetDrawer')">取消</button>
                    </div>
                </form>
                <div id="newFundHint" class="source-hint"></div>
            </div>
        </div>

        <div id="importDrawer" class="drawer-overlay" onclick="if(event.target===this)closeDrawer('importDrawer')">
            <div class="drawer-panel" onclick="event.stopPropagation()">
                <h3 style="margin:0 0 12px;">导入支付宝持仓快照</h3>
                <p class="source-hint">上传 CSV 后系统会解析并写入快照，可在上方一键初始化本地持仓。</p>
                <form action="/api/import/preview" method="POST" enctype="multipart/form-data">
                    <input type="file" name="file" accept=".csv" required style="margin:8px 0;">
                    <div class="form-actions">
                        <button type="submit" class="btn btn-sm">上传预览</button>
                        <button type="button" class="btn-ghost btn-sm" onclick="closeDrawer('importDrawer')">取消</button>
                    </div>
                </form>
            </div>
        </div>

        <script type="application/json" id="assetsData">{}</script>
        <script>{}
        const ASSETS = JSON.parse(document.getElementById('assetsData').textContent || '[]');
        function findAsset(id) {{ return ASSETS.find(a => a.asset_id === id); }}
        function openAssetEdit(id) {{
            const a = findAsset(id);
            if (!a) return;
            document.getElementById('aeId').value = a.asset_id;
            document.getElementById('aeArchiveId').value = a.asset_id;
            document.getElementById('aeCode').value = a.fund_code || '';
            document.getElementById('aeName').value = a.fund_name || '';
            document.getElementById('aeSector').value = a.sector || '';
            document.getElementById('aeRegion').textContent = a.region || '—';
            document.getElementById('aeCurrency').value = a.currency || 'CNY';
            document.getElementById('aeValMethod').value = a.valuation_method || 'nav';
            document.getElementById('aeProvider').value = a.market_data_provider || '';
            document.getElementById('aeBench').value = a.reference_index_symbol || '';
            document.getElementById('aeInstSym').value = a.reference_instrument_symbol || '';
            document.getElementById('aeLookupHint').textContent = '';
            openDrawer('assetEditDrawer');
        }}
        async function saveAssetEdit(btn) {{
            await runBtnAction(btn, () => adminFetch('/api/assets/update', {{
                method: 'POST',
                headers: {{'Content-Type':'application/json'}},
                body: JSON.stringify({{
                    asset_id: document.getElementById('aeId').value,
                    fund_code: document.getElementById('aeCode').value,
                    fund_name: document.getElementById('aeName').value,
                    sector: document.getElementById('aeSector').value,
                    currency: document.getElementById('aeCurrency').value,
                    valuation_method: document.getElementById('aeValMethod').value,
                    market_data_provider: document.getElementById('aeProvider').value || null,
                    reference_index_symbol: document.getElementById('aeBench').value || null,
                    reference_instrument_symbol: document.getElementById('aeInstSym').value || null,
                }})
            }}));
        }}
        async function lookupFundForEdit(btn) {{
            const code = document.getElementById('aeCode').value.trim();
            if (!code) return showToast('请先填写基金代码', false);
            if (btn) btn.disabled = true;
            const r = await adminFetch('/api/fund/lookup', {{ method:'POST', headers:{{'Content-Type':'application/json'}}, body: JSON.stringify({{fund_code: code}}) }});
            if (btn) btn.disabled = false;
            let hint = r.message || '';
            if (r.fund_name) document.getElementById('aeName').value = r.fund_name;
            if (r.inferred_sector) document.getElementById('aeSector').value = r.inferred_sector;
            if (r.warnings && r.warnings.length) hint += ' ' + r.warnings.join('; ');
            document.getElementById('aeLookupHint').textContent = r.success ? ('已查询: ' + (r.fund_name||'') + ' ' + hint) : hint;
            if (!r.success) showToast(hint || '查询失败', false);
        }}
        async function lookupNewFund(btn) {{
            const code = document.getElementById('newFundCode').value.trim();
            if (!code) return showToast('请填写基金代码', false);
            const r = await adminFetch('/api/fund/lookup', {{ method:'POST', headers:{{'Content-Type':'application/json'}}, body: JSON.stringify({{fund_code: code}}) }});
            document.getElementById('newFundHint').textContent = r.success ? (r.fund_name + ' / ' + (r.inferred_sector||'')) : (r.message||'失败');
        }}
        async function enrichCurrentAsset(btn) {{
            const id = document.getElementById('aeId').value;
            await enrichOneAsset(id, btn);
        }}
        async function enrichOneAsset(id, btn) {{
            await runBtnAction(btn, () => adminFetch('/api/assets/enrich', {{ method:'POST', headers:{{'Content-Type':'application/json'}}, body: JSON.stringify({{asset_id: id}}) }}));
        }}
        async function enrichAllAssets(btn) {{
            await runBtnAction(btn, () => adminFetch('/api/assets/enrich-all', {{ method:'POST' }}));
        }}
        async function createDcaForAsset(assetId, fundCode) {{
            const amt = parseFloat(prompt('定投金额 (CNY):', '1000') || '0');
            if (!amt || amt <= 0) return;
            const freq = prompt('频率 daily/weekly/monthly:', 'monthly') || 'monthly';
            const r = await adminFetch('/api/dca/plans', {{
                method:'POST', headers:{{'Content-Type':'application/json'}},
                body: JSON.stringify({{ asset_id: assetId, amount: amt, frequency: freq, note: '从持仓创建' }})
            }});
            if (r.success !== false) {{ showToast('定投计划已创建', true); location.reload(); }}
            else showToast(r.message || '失败', false);
        }}
        </script>
        "#,
        admin_extra_css(),
        vm.bootstrap_html,
        vm.display_book,
        vm.alipay_total,
        vm.diff_class,
        vm.diff,
        vm.holdings_rows_html,
        vm.asset_table_html,
        vm.assets_json,
        admin_js_core()
    )
}
