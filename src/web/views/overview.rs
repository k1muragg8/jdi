//! Overview (概览) page HTML.

use crate::web::product::product_extra_css;
use crate::web::view_models::overview::OverviewPageVm;
use crate::web::views::components::{admin_extra_css, admin_js_core};

pub fn render(vm: &OverviewPageVm) -> String {
    format!(
        r#"
        <style>{}{}</style>
        <div id="adminToast"></div>
        <div class="overview-compact">
            <h1 style="margin-bottom:4px;">概览</h1>
            <p style="color:var(--text-muted);font-size:0.9rem;margin:0 0 16px 0;">我的资产分布、仓位比例与今日建议（点击可编辑项修改源数据后自动重算）</p>
            {}
            {}
            <div class="overview-metrics">
                <div class="card">
                    <div class="card-header"><span class="card-title">总资产</span></div>
                    <div class="card-value tabular">{:.2}</div>
                    <div class="source-hint">持仓 + 现金 汇总</div>
                </div>
                <div class="card">
                    <div class="card-header"><span class="card-title">权益仓</span></div>
                    <div class="card-value tabular">{:.2}</div>
                    <div class="source-hint">权益类持仓市值</div>
                </div>
                <div class="card">
                    <div class="card-header"><span class="card-title">债券</span></div>
                    <div class="card-value tabular">{:.2}</div>
                </div>
                <div class="card">
                    <div class="card-header"><span class="card-title">货币/现金</span></div>
                    <div class="card-value tabular metric-editable" onclick="openCashAdjust()" title="点击调整现金">{:.2}</div>
                    <div class="source-hint">现金账本 · <a href="/holdings">去持仓</a></div>
                </div>
                <div class="card">
                    <div class="card-header"><span class="card-title">今日建议买入</span></div>
                    <div class="card-value tabular text-up">{:.2}</div>
                    <div class="source-hint">只读 · Kelly/Pendulum/DCA 决策输出</div>
                </div>
                <div class="card">
                    <div class="card-header"><span class="card-title">权益仓位</span></div>
                    <div class="card-value tabular metric-editable" style="font-size:1.1rem;" onclick="openTargetEquityEdit()" title="点击编辑目标权益仓位">{:.1}% / {:.1}%</div>
                    <div class="source-hint">当前计算 / 目标配置</div>
                </div>
            </div>
            <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px;">
                <div class="card"><div class="card-header"><span class="card-title">大类资产分布</span></div>{}</div>
                <div class="card"><div class="card-header"><span class="card-title">权益国家/地区</span><div class="source-hint">按赛道映射 · 在持仓页改赛道</div></div>{}</div>
            </div>
            <div class="card">
                <div class="card-header"><span class="card-title">赛道分布（当前 vs 目标）</span></div>
                <div class="table-container"><table class="holdings-compact"><thead><tr><th>赛道</th><th>市值</th><th>当前%</th><th>目标%</th><th>偏差</th><th>操作</th></tr></thead><tbody>{}</tbody></table></div>
            </div>
        </div>

        <div id="targetEquityDrawer" class="drawer-overlay" onclick="if(event.target===this)closeDrawer('targetEquityDrawer')">
            <div class="drawer-panel" onclick="event.stopPropagation()">
                <h3>编辑目标权益仓位</h3>
                <p class="source-hint">修改运营策略中的目标权重（0~100%），保存后概览将重算。</p>
                <label>目标权益 %<input type="number" id="tePct" min="0" max="100" step="0.1" value="{:.1}"></label>
                <div class="form-actions">
                    <button class="btn btn-sm" onclick="saveTargetEquity(this)">保存</button>
                    <button class="btn-ghost btn-sm" onclick="closeDrawer('targetEquityDrawer')">取消</button>
                </div>
            </div>
        </div>
        <div id="cashAdjustDrawer" class="drawer-overlay" onclick="if(event.target===this)closeDrawer('cashAdjustDrawer')">
            <div class="drawer-panel" onclick="event.stopPropagation()">
                <h3>现金调整</h3>
                <p class="source-hint">写入现金流水（正数入账，负数出账），不会直接改计算值。</p>
                <label>金额 CNY<input type="number" id="cashAmt" step="0.01" placeholder="正=入账 负=出账"></label>
                <label>备注<input type="text" id="cashNote" placeholder="可选"></label>
                <div class="form-actions">
                    <button class="btn btn-sm" onclick="saveCashAdjust(this)">保存</button>
                    <button class="btn-ghost btn-sm" onclick="closeDrawer('cashAdjustDrawer')">取消</button>
                </div>
            </div>
        </div>
        <div id="sectorTargetDrawer" class="drawer-overlay" onclick="if(event.target===this)closeDrawer('sectorTargetDrawer')">
            <div class="drawer-panel" onclick="event.stopPropagation()">
                <h3>编辑赛道目标权重</h3>
                <input type="hidden" id="stName">
                <label>赛道<span id="stNameLbl"></span></label>
                <label>目标权重 (0~1)<input type="number" id="stWeight" min="0" max="1" step="0.01"></label>
                <div class="form-actions">
                    <button class="btn btn-sm" onclick="saveSectorTarget(this)">保存</button>
                    <button class="btn-ghost btn-sm" onclick="closeDrawer('sectorTargetDrawer')">取消</button>
                </div>
            </div>
        </div>

        <script>{}
        function openTargetEquityEdit() {{ openDrawer('targetEquityDrawer'); }}
        function openCashAdjust() {{ openDrawer('cashAdjustDrawer'); }}
        function editSectorTarget(name, w) {{
            document.getElementById('stName').value = name;
            document.getElementById('stNameLbl').textContent = name;
            document.getElementById('stWeight').value = w;
            openDrawer('sectorTargetDrawer');
        }}
        async function saveTargetEquity(btn) {{
            const pct = parseFloat(document.getElementById('tePct').value);
            if (isNaN(pct)) return showToast('无效数值', false);
            await runBtnAction(btn, () => adminFetch('/api/operation/policy/target-equity', {{
                method:'POST', headers:{{'Content-Type':'application/json'}},
                body: JSON.stringify({{ target_equity_weight: pct / 100 }})
            }}));
        }}
        async function saveCashAdjust(btn) {{
            const amt = parseFloat(document.getElementById('cashAmt').value);
            if (isNaN(amt)) return showToast('请输入金额', false);
            await runBtnAction(btn, () => adminFetch('/api/cash/adjust-json', {{
                method:'POST', headers:{{'Content-Type':'application/json'}},
                body: JSON.stringify({{ amount: amt, note: document.getElementById('cashNote').value || null }})
            }}));
        }}
        async function saveSectorTarget(btn) {{
            const name = document.getElementById('stName').value;
            const w = parseFloat(document.getElementById('stWeight').value);
            await runBtnAction(btn, () => adminFetch('/api/sectors/target-weight', {{
                method:'POST', headers:{{'Content-Type':'application/json'}},
                body: JSON.stringify({{ sector_name: name, target_weight: w }})
            }}));
        }}
        async function autoClassify(el) {{
            if (el) el.disabled = true;
            try {{ await fetch('/api/jobs/assets/auto-classify', {{method:'POST'}}); location.reload(); }}
            catch(e) {{ alert('失败:'+e); if(el) el.disabled=false; }}
        }}
        </script>
        "#,
        product_extra_css(),
        admin_extra_css(),
        vm.auto_task_html,
        vm.warnings_html,
        vm.display_total,
        vm.equity_value,
        vm.bond_value,
        vm.cash_mm,
        vm.total_suggested,
        vm.current_equity_pct,
        vm.target_equity_pct,
        vm.asset_class_html,
        vm.region_html,
        vm.sector_rows_html,
        vm.target_equity_pct,
        admin_js_core()
    )
}
