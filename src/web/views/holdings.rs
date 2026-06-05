//! Holdings (持仓) page HTML.

use crate::web::view_models::holdings::HoldingsPageVm;
use crate::web::views::components::{admin_extra_css, admin_js_core};

pub fn render(vm: &HoldingsPageVm) -> String {
    let arch_link = if vm.show_archived {
        r#"<a href="/holdings" class="btn btn-sm">显示活跃</a>"#.to_string()
    } else if vm.archived_count > 0 {
        r#"<a href="/holdings?filter=archived" class="btn btn-sm btn-outline">显示已归档</a>"#
            .to_string()
    } else {
        "".to_string()
    };

    let batch_buttons = if vm.active_count > 0 {
        r#"
            <button type="button" class="btn btn-outline btn-sm" id="btnEnrichAll" onclick="enrichAllAssets(this)">自动补全基金信息</button>
            <button type="button" class="btn btn-outline btn-sm" onclick="runBtnAction(this, ()=>adminFetch('/api/jobs/assets/auto-classify',{{method:'POST'}}))">自动分类</button>
            <button type="button" class="btn btn-outline btn-sm" onclick="runBtnAction(this, ()=>adminFetch('/api/jobs/nav/refresh',{{method:'POST'}}))">刷新基金净值</button>
        "#.to_string()
    } else {
        "".to_string()
    };

    let current_filter = if vm.show_archived { "archived" } else { "" };

    format!(
        r#"
        <style>{extra_css}</style>
        <div id="adminToast"></div>
        <div style="display:flex;justify-content:space-between;align-items:flex-end;margin-bottom:12px;flex-wrap:wrap;gap:12px;">
            <div>
                <h1 style="margin-bottom:4px;">持仓</h1>
                <p style="color:var(--text-muted);font-size:0.9rem;margin:0;">管理本地持仓、基金代码与赛道；市值由份额 × 最新NAV计算</p>
            </div>
        </div>

        <div class="admin-toolbar">
            <button type="button" class="btn btn-sm" onclick="openDrawer('addAssetDrawer')">新增资产</button>
            {batch_buttons}
            {arch_link}
        </div>

        {bootstrap_html}

        <div class="overview-metrics" style="margin-bottom:16px;">
            <div class="card"><div class="card-header"><span class="card-title">持仓总市值</span><div class="source-hint">本地持仓汇总</div></div><div class="card-value tabular">{display_book:.2}</div></div>
            <div class="card"><div class="card-header"><span class="card-title">权益</span></div><div class="card-value tabular">{equity_value:.2}</div></div>
            <div class="card"><div class="card-header"><span class="card-title">债券</span></div><div class="card-value tabular">{bond_value:.2}</div></div>
            <div class="card"><div class="card-header"><span class="card-title">货币/现金</span></div><div class="card-value tabular">{cash_value:.2}</div></div>
        </div>

        <div class="card" style="margin-bottom:16px;">
            <div class="card-header"><span class="card-title">持仓明细</span></div>
            <div class="table-container"><div class="table-wrap">
                <table class="holdings-compact">
                    <thead><tr>
                        <th>资产 / 代码</th><th>赛道</th><th>地区</th>
                        <th class="text-right">份额</th><th class="text-right">最新净值</th><th class="text-right">净值日期</th>
                        <th class="text-right">市值</th><th class="text-right">盈亏</th><th class="text-right">定投</th><th class="text-right">操作</th>
                    </tr></thead>
                    <tbody>{rows_html}</tbody>
                </table>
            </div></div>
        </div>

        <div id="assetEditDrawer" class="drawer-overlay" onclick="if(event.target===this)closeDrawer('assetEditDrawer')">
            <div class="drawer-panel" onclick="event.stopPropagation()">
                <h3 style="margin:0 0 12px;">编辑资产</h3>
                <div class="form-grid">
                    <label>资产ID<input id="aeId" readonly style="background:#f5f5f5;"></label>
                    <input type="hidden" id="aeFilter" value="{current_filter}">
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
                    <form id="aeArchiveForm" method="POST" action="/admin/assets/remove" style="display:inline;" onsubmit="return confirm('归档/删除此资产？');">
                        <input type="hidden" name="asset_id" id="aeArchiveId">
                        <input type="hidden" name="filter" value="{current_filter}">
                        <button type="submit" class="btn btn-sm btn-danger">归档</button>
                    </form>
                    <button type="button" class="btn btn-sm btn-outline" onclick="restoreCurrentAsset(this)">恢复</button>
                    <button type="button" class="btn btn-outline btn-sm" onclick="closeDrawer('assetEditDrawer')">取消</button>
                </div>
                <div id="aeLookupHint" class="source-hint" style="margin-top:8px;"></div>
            </div>
        </div>

        <div id="addAssetDrawer" class="drawer-overlay" onclick="if(event.target===this)closeDrawer('addAssetDrawer')">
            <div class="drawer-panel" onclick="event.stopPropagation()">
                <h3 style="margin:0 0 12px;">新增资产</h3>
                <form action="/admin/assets/add" method="POST" class="form-grid" onsubmit="return true;">
                    <input type="hidden" name="filter" value="{current_filter}">
                    <label style="grid-column:1/-1;">资产名称<input name="fund_name" required></label>
                    <label>基金代码<input name="fund_code" id="newFundCode" required></label>
                    <label>赛道<input name="sector" placeholder="可选"></label>
                    <div class="form-actions" style="grid-column:1/-1;">
                        <button type="button" class="btn btn-outline btn-sm" onclick="lookupNewFund(this)">查询基金</button>
                        <button type="submit" class="btn btn-sm">创建</button>
                        <button type="button" class="btn btn-outline btn-sm" onclick="closeDrawer('addAssetDrawer')">取消</button>
                    </div>
                </form>
                <div id="newFundHint" class="source-hint"></div>
            </div>
        </div>

        <div id="dcaEditDrawer" class="drawer-overlay" onclick="if(event.target===this)closeDrawer('dcaEditDrawer')">
            <div class="drawer-panel" onclick="event.stopPropagation()">
                <h3 id="dcaModalTitle" style="margin:0 0 12px;">设置定投</h3>
                <input type="hidden" id="dcaAssetId">
                <div class="form-grid">
                    <label>定投金额 (CNY)<input type="number" id="dcaAmount" step="0.01" value="1000"></label>
                    <label>频率
                        <select id="dcaFreq" onchange="updateDcaDayOptions()">
                            <option value="daily">每日</option>
                            <option value="weekly">每周</option>
                            <option value="monthly">每月</option>
                        </select>
                    </label>
                    <label>执行日 <select id="dcaDay"></select></label>
                    <label>开始日期 <input type="date" id="dcaStart"></label>
                    <label>结束日期 <input type="date" id="dcaEnd"></label>
                    <label style="grid-column:1/-1;">备注 <input type="text" id="dcaNote"></label>
                </div>
                <div class="form-actions">
                    <button type="button" class="btn btn-sm" onclick="saveDcaPlan(this)">保存</button>
                    <button type="button" class="btn btn-sm btn-outline" onclick="pauseDcaPlan(this)">暂停</button>
                    <button type="button" class="btn btn-sm btn-outline" onclick="resumeDcaPlan(this)">恢复</button>
                    <button type="button" class="btn btn-sm btn-danger" onclick="archiveDcaPlan(this)">归档</button>
                    <button type="button" class="btn btn-sm btn-outline" onclick="viewDcaRecords()">查看记录</button>
                    <button type="button" class="btn btn-outline btn-sm" onclick="closeDrawer('dcaEditDrawer')">取消</button>
                </div>
                <div id="dcaHint" class="source-hint" style="margin-top:8px;"></div>
            </div>
        </div>

        <script type="application/json" id="assetsData">{assets_json}</script>
        <script>{js_core}
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
            const archBtn = document.querySelector('#aeArchiveForm button');
            if (archBtn) {{
                const isArch = (a.sector || '').includes('已归档') || !a.enabled;
                archBtn.textContent = isArch ? '删除' : '归档';
            }}
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
                    filter: document.getElementById('aeFilter').value || null,
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
        function restoreCurrentAsset(btn) {{
            const id = document.getElementById('aeId').value;
            const filter = document.getElementById('aeFilter').value;
            if (!id) return;
            if (btn) {{ btn.disabled = true; }}
            const f = document.createElement('form');
            f.method = 'POST';
            f.action = '/admin/assets/restore';
            const i = document.createElement('input');
            i.type = 'hidden'; i.name = 'asset_id'; i.value = id;
            f.appendChild(i);
            if (filter) {{
                const fi = document.createElement('input');
                fi.type = 'hidden'; fi.name = 'filter'; fi.value = filter;
                f.appendChild(fi);
            }}
            document.body.appendChild(f);
            f.submit();
        }}
        async function enrichOneAsset(id, btn) {{
            await runBtnAction(btn, () => adminFetch('/api/assets/enrich', {{ method:'POST', headers:{{'Content-Type':'application/json'}}, body: JSON.stringify({{asset_id: id}}) }}));
        }}
        async function enrichAllAssets(btn) {{
            await runBtnAction(btn, () => adminFetch('/api/assets/enrich-all', {{ method:'POST' }}));
        }}
        function updateDcaDayOptions() {{
            const freq = document.getElementById('dcaFreq').value;
            const daySel = document.getElementById('dcaDay');
            daySel.innerHTML = '';
            if (freq === 'daily') {{
                const opt = document.createElement('option');
                opt.value = ''; opt.text = '每天';
                daySel.appendChild(opt);
                daySel.disabled = true;
            }} else if (freq === 'weekly') {{
                daySel.disabled = false;
                ['1','2','3','4','5','6','7'].forEach((d,i) => {{
                    const opt = document.createElement('option');
                    opt.value = d;
                    opt.text = ['周一','周二','周三','周四','周五','周六','周日'][i];
                    daySel.appendChild(opt);
                }});
            }} else if (freq === 'monthly') {{
                daySel.disabled = false;
                for (let d=1; d<=28; d++) {{
                    const opt = document.createElement('option');
                    opt.value = d;
                    opt.text = d + '日';
                    daySel.appendChild(opt);
                }}
                const opt = document.createElement('option');
                opt.value = '0';
                opt.text = '月底';
                daySel.appendChild(opt);
            }}
        }}
        async function openDcaModal(assetId) {{
            document.getElementById('dcaAssetId').value = assetId;
            document.getElementById('dcaHint').textContent = '';
            const plansRes = await adminFetch('/api/dca/plans', {{method:'GET'}});
            const plans = (plansRes && Array.isArray(plansRes)) ? plansRes : (plansRes.plans || []);
            const plan = plans.find(p => p.asset_id === assetId);
            const title = plan ? '编辑定投' : '设置定投';
            document.getElementById('dcaModalTitle').textContent = title + '：' + (ASSETS.find(a=>a.asset_id===assetId)?.fund_name || assetId);
            if (plan) {{
                document.getElementById('dcaAmount').value = plan.amount || 1000;
                let freq = 'monthly';
                if (plan.frequency === 'daily' || plan.frequency === 'Daily') freq = 'daily';
                else if (plan.frequency === 'weekly' || plan.frequency === 'Weekly') freq = 'weekly';
                document.getElementById('dcaFreq').value = freq;
                updateDcaDayOptions();
                const dayVal = plan.weekday || plan.month_day || (freq==='monthly' ? '1' : '1');
                document.getElementById('dcaDay').value = dayVal || '';
                document.getElementById('dcaStart').value = plan.start_date || '';
                document.getElementById('dcaEnd').value = plan.end_date || '';
                document.getElementById('dcaNote').value = plan.note || '';
            }} else {{
                document.getElementById('dcaAmount').value = 1000;
                document.getElementById('dcaFreq').value = 'monthly';
                updateDcaDayOptions();
                document.getElementById('dcaDay').value = '1';
                const today = new Date().toISOString().slice(0,10);
                document.getElementById('dcaStart').value = today;
                document.getElementById('dcaEnd').value = '';
                document.getElementById('dcaNote').value = '';
            }}
            openDrawer('dcaEditDrawer');
        }}
        async function saveDcaPlan(btn) {{
            const assetId = document.getElementById('dcaAssetId').value;
            const amt = parseFloat(document.getElementById('dcaAmount').value);
            const freq = document.getElementById('dcaFreq').value;
            const day = parseInt(document.getElementById('dcaDay').value) || null;
            const start = document.getElementById('dcaStart').value;
            const end = document.getElementById('dcaEnd').value || null;
            const note = document.getElementById('dcaNote').value || null;
            if (!assetId || !amt || amt <= 0) return showToast('请输入有效金额', false);
            if (btn) btn.disabled = true;
            const body = {{ asset_id: assetId, amount: amt, frequency: freq, day: day, note: note }};
            const plansRes = await adminFetch('/api/dca/plans', {{method:'GET'}});
            const plans = (plansRes && Array.isArray(plansRes)) ? plansRes : [];
            const existing = plans.find(p => p.asset_id === assetId);
            let r;
            if (existing) {{
                r = await adminFetch('/api/dca/plans/' + existing.plan_id, {{
                    method:'PATCH', headers:{{'Content-Type':'application/json'}},
                    body: JSON.stringify({{ amount: amt, frequency: freq, day: day, note: note }})
                }});
            }} else {{
                r = await adminFetch('/api/dca/plans', {{
                    method:'POST', headers:{{'Content-Type':'application/json'}},
                    body: JSON.stringify(body)
                }});
            }}
            if (btn) btn.disabled = false;
            if (r && r.success !== false) {{
                showToast('定投计划已保存', true);
                closeDrawer('dcaEditDrawer');
                setTimeout(()=>location.reload(), 400);
            }} else {{
                showToast((r && r.message) || '保存失败', false);
            }}
        }}
        async function pauseDcaPlan(btn) {{
            const assetId = document.getElementById('dcaAssetId').value;
            if (!assetId) return;
            const plansRes = await adminFetch('/api/dca/plans', {{method:'GET'}});
            const plans = (plansRes && Array.isArray(plansRes)) ? plansRes : [];
            const plan = plans.find(p => p.asset_id === assetId);
            if (!plan) return;
            if (btn) btn.disabled = true;
            const r = await adminFetch('/api/dca/plans/' + plan.plan_id, {{
                method:'PATCH', headers:{{'Content-Type':'application/json'}},
                body: JSON.stringify({{ enabled: false }})
            }});
            if (btn) btn.disabled = false;
            if (r && r.success !== false) {{
                showToast('已暂停', true);
                closeDrawer('dcaEditDrawer');
                setTimeout(()=>location.reload(), 400);
            }} else showToast((r&&r.message)||'失败', false);
        }}
        async function resumeDcaPlan(btn) {{
            const assetId = document.getElementById('dcaAssetId').value;
            if (!assetId) return;
            const plansRes = await adminFetch('/api/dca/plans', {{method:'GET'}});
            const plans = (plansRes && Array.isArray(plansRes)) ? plansRes : [];
            const plan = plans.find(p => p.asset_id === assetId);
            if (!plan) return;
            if (btn) btn.disabled = true;
            const r = await adminFetch('/api/dca/plans/' + plan.plan_id, {{
                method:'PATCH', headers:{{'Content-Type':'application/json'}},
                body: JSON.stringify({{ enabled: true }})
            }});
            if (btn) btn.disabled = false;
            if (r && r.success !== false) {{
                showToast('已恢复', true);
                closeDrawer('dcaEditDrawer');
                setTimeout(()=>location.reload(), 400);
            }} else showToast((r&&r.message)||'失败', false);
        }}
        async function archiveDcaPlan(btn) {{
            const assetId = document.getElementById('dcaAssetId').value;
            if (!assetId) return;
            if (!confirm('确认归档/删除此定投计划？历史记录将保留。')) return;
            const plansRes = await adminFetch('/api/dca/plans', {{method:'GET'}});
            const plans = (plansRes && Array.isArray(plansRes)) ? plansRes : [];
            const plan = plans.find(p => p.asset_id === assetId);
            if (!plan) return;
            if (btn) btn.disabled = true;
            const r = await adminFetch('/api/dca/plans/' + plan.plan_id, {{ method:'DELETE' }});
            if (btn) btn.disabled = false;
            if (r && r.success !== false) {{
                showToast('定投计划已归档', true);
                closeDrawer('dcaEditDrawer');
                setTimeout(()=>location.reload(), 400);
            }} else showToast((r&&r.message)||'失败', false);
        }}
        async function viewDcaRecords(id) {{
            try {{
                const res = await fetch('/api/dca/executions');
                const data = await res.json();
                let assetId = id || document.getElementById('dcaAssetId')?.value || '';
                let filtered = data;
                if (assetId && Array.isArray(data)) {{
                    filtered = data.filter(s => s.asset_id === assetId || !s.asset_id).slice(0,10);
                }}
                alert('最近定投记录 (最近10条):\n' + JSON.stringify(filtered, null, 2));
            }} catch(e) {{ alert('加载记录失败: ' + e); }}
        }}
        // direct row actions (for table buttons without opening drawer first)
        async function pauseDca(assetId, btn) {{
            if (!assetId) return;
            const plansRes = await adminFetch('/api/dca/plans', {{method:'GET'}});
            const plans = (plansRes && Array.isArray(plansRes)) ? plansRes : [];
            const plan = plans.find(p => p.asset_id === assetId);
            if (!plan) {{ showToast('无定投计划', false); return; }}
            if (btn) btn.disabled = true;
            const r = await adminFetch('/api/dca/plans/' + plan.plan_id, {{
                method:'PATCH', headers:{{'Content-Type':'application/json'}},
                body: JSON.stringify({{ enabled: false }})
            }});
            if (btn) btn.disabled = false;
            if (r && r.success !== false) {{
                showToast('已暂停', true);
                setTimeout(()=>location.reload(), 300);
            }} else showToast((r&&r.message)||'失败', false);
        }}
        async function resumeDca(assetId, btn) {{
            if (!assetId) return;
            const plansRes = await adminFetch('/api/dca/plans', {{method:'GET'}});
            const plans = (plansRes && Array.isArray(plansRes)) ? plansRes : [];
            const plan = plans.find(p => p.asset_id === assetId);
            if (!plan) {{ showToast('无定投计划', false); return; }}
            if (btn) btn.disabled = true;
            const r = await adminFetch('/api/dca/plans/' + plan.plan_id, {{
                method:'PATCH', headers:{{'Content-Type':'application/json'}},
                body: JSON.stringify({{ enabled: true }})
            }});
            if (btn) btn.disabled = false;
            if (r && r.success !== false) {{
                showToast('已恢复', true);
                setTimeout(()=>location.reload(), 300);
            }} else showToast((r&&r.message)||'失败', false);
        }}
        // auto refresh fund NAV on page load (once, background)
        setTimeout(function() {{
            fetch('/api/jobs/nav/refresh', {{method:'POST'}}).catch(function(){{}});
        }}, 800);
        </script>
        "#,
        extra_css = admin_extra_css(),
        batch_buttons = batch_buttons,
        arch_link = arch_link,
        bootstrap_html = vm.bootstrap_html,
        display_book = vm.display_book,
        equity_value = vm.equity_value,
        bond_value = vm.bond_value,
        cash_value = vm.cash_value,
        rows_html = vm.holdings_rows_html,
        current_filter = current_filter,
        assets_json = vm.assets_json,
        js_core = admin_js_core()
    )
}
