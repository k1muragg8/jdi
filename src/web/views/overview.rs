//! Overview (概览) page HTML.

use crate::web::product::product_extra_css;
use crate::web::view_models::overview::OverviewPageVm;

pub fn render(vm: &OverviewPageVm) -> String {
    format!(
        r#"
        <style>{}</style>
        <div class="overview-compact">
            <h1 style="margin-bottom:4px;">概览</h1>
            <p style="color:var(--text-muted);font-size:0.9rem;margin:0 0 16px 0;">我的资产分布、仓位比例与今日建议</p>
            {}
            {}
            <div class="overview-metrics">
                <div class="card"><div class="card-header"><span class="card-title">总资产</span></div><div class="card-value tabular">{:.2}</div><div class="card-sub">CNY</div></div>
                <div class="card"><div class="card-header"><span class="card-title">权益仓</span></div><div class="card-value tabular">{:.2}</div></div>
                <div class="card"><div class="card-header"><span class="card-title">债券</span></div><div class="card-value tabular">{:.2}</div></div>
                <div class="card"><div class="card-header"><span class="card-title">货币/现金</span></div><div class="card-value tabular">{:.2}</div></div>
                <div class="card"><div class="card-header"><span class="card-title">今日建议买入</span></div><div class="card-value tabular text-up">{:.2}</div></div>
                <div class="card"><div class="card-header"><span class="card-title">权益仓位</span></div><div class="card-value tabular" style="font-size:1.1rem;">{:.1}% / {:.1}%</div><div class="card-sub">当前 / 目标</div></div>
            </div>
            <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px;">
                <div class="card"><div class="card-header"><span class="card-title">大类资产分布</span></div>{}</div>
                <div class="card"><div class="card-header"><span class="card-title">权益国家/地区</span></div>{}</div>
            </div>
            <div class="card">
                <div class="card-header"><span class="card-title">赛道分布（当前 vs 目标）</span></div>
                <div class="table-container"><table class="holdings-compact"><thead><tr><th>赛道</th><th>市值</th><th>当前%</th><th>目标%</th><th>偏差</th></tr></thead><tbody>{}</tbody></table></div>
            </div>
        </div>
        <script>
        async function autoClassify(el) {{
            if (el) el.disabled = true;
            try {{ await fetch('/api/jobs/assets/auto-classify', {{method:'POST'}}); location.reload(); }}
            catch(e) {{ alert('失败:'+e); if(el) el.disabled=false; }}
        }}
        </script>
        "#,
        product_extra_css(),
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
        vm.sector_rows_html
    )
}
