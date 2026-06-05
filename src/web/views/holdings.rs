//! Holdings (持仓) page HTML.

use crate::web::view_models::holdings::HoldingsPageVm;

pub fn render(vm: &HoldingsPageVm) -> String {
    format!(
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
        vm.bootstrap_html,
        vm.display_book,
        vm.alipay_total,
        vm.diff_class,
        vm.diff,
        vm.holdings_rows_html,
        vm.asset_mgmt_rows_html
    )
}
