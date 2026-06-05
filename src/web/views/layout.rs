//! App shell: HTML document, nav, shared CSS/JS.

use crate::web::product::product_extra_css;
use axum::response::Html;

pub fn layout(title: &str, content: String) -> Html<String> {
    layout_with_msg(title, content, None, None)
}

pub fn layout_with_msg(
    title: &str,
    content: String,
    success: Option<String>,
    error: Option<String>,
) -> Html<String> {
    let mut msg_html = String::new();
    if let Some(s) = success {
        msg_html.push_str(&format!(
            r#"<div class="message-banner message-success">
                <span class="banner-icon">✓</span>
                <span>{}</span>
            </div>"#,
            s
        ));
    }
    if let Some(e) = error {
        msg_html.push_str(&format!(
            r#"<div class="message-banner message-error">
                <span class="banner-icon">✕</span>
                <span>{}</span>
            </div>"#,
            e
        ));
    }

    Html(format!(
        r#"
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <title>{} - JDI Portfolio</title>
    <style>
        :root {{
            --primary-color: #0052D9;
            --primary-light: #E8F3FF;
            --bg-color: #F2F3F5;
            --card-bg: #FFFFFF;
            --text-main: #1D2129;
            --text-muted: #86909C;
            --border-color: #E5E6EB;
            --up-color: #F53F3F;
            --down-color: #00B42A;
            --warn-color: #FF7D00;
            --info-color: #165DFF;
            --nav-bg: rgba(255, 255, 255, 0.8);
            --radius: 12px;
            --shadow-sm: 0 2px 8px rgba(0,0,0,0.04);
            --shadow-md: 0 4px 16px rgba(0,0,0,0.08);
        }}
        * {{ box-sizing: border-box; -webkit-tap-highlight-color: transparent; }}
        body {{ 
            font-family: -apple-system, BlinkMacSystemFont, "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif; 
            line-height: 1.6; 
            color: var(--text-main); 
            background-color: var(--bg-color);
            margin: 0;
            padding: 0;
            padding-bottom: 80px; 
        }}
        
        .container {{ max-width: 1200px; margin: 0 auto; padding: 32px 24px; }}
        
        header {{ 
            background: var(--nav-bg); 
            backdrop-filter: blur(10px);
            -webkit-backdrop-filter: blur(10px);
            border-bottom: 1px solid var(--border-color); 
            position: sticky; 
            top: 0; 
            z-index: 1000; 
        }}
        .header-wrap {{ display: flex; align-items: center; justify-content: space-between; padding: 0 24px; height: 64px; max-width: 1200px; margin: 0 auto; }}
        .logo {{ font-weight: 900; font-size: 1.25rem; color: var(--primary-color); text-decoration: none; letter-spacing: -0.5px; display: flex; align-items: center; gap: 8px; }}
        .logo::before {{ content: '📈'; font-size: 1.4rem; }}
        
        .nav-desktop {{ display: flex; gap: 4px; }}
        .nav-desktop a {{ 
            color: var(--text-main); 
            text-decoration: none; 
            padding: 8px 16px; 
            font-size: 0.95rem; 
            font-weight: 600;
            border-radius: 8px;
            transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
        }}
        .nav-desktop a:hover {{ background: var(--bg-color); color: var(--primary-color); }}
        .nav-desktop a.active {{ color: var(--primary-color); background: var(--primary-light); }}
        
        .card {{ 
            background: var(--card-bg); 
            border-radius: var(--radius); 
            padding: 24px; 
            margin-bottom: 24px; 
            box-shadow: var(--shadow-sm); 
            border: 1px solid var(--border-color); 
            transition: transform 0.2s, box-shadow 0.2s;
        }}
        .card:hover {{ box-shadow: var(--shadow-md); }}
        .card-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; padding-bottom: 12px; border-bottom: 1px solid var(--bg-color); }}
        .card-title {{ font-size: 0.9rem; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; }}
        .card-value {{ font-size: 2.25rem; font-weight: 900; letter-spacing: -1.5px; line-height: 1; margin: 8px 0; font-variant-numeric: tabular-nums; }}
        .card-sub {{ font-size: 0.85rem; color: var(--text-muted); font-weight: 500; }}
        
        .dashboard-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 24px; margin-bottom: 24px; }}
        
        h1 {{ font-size: 2rem; font-weight: 900; margin: 0 0 32px 0; color: var(--text-main); letter-spacing: -0.75px; }}
        h2 {{ font-size: 1.5rem; font-weight: 800; margin: 48px 0 24px 0; letter-spacing: -0.5px; display: flex; align-items: center; gap: 12px; }}
        h3 {{ font-size: 1.15rem; font-weight: 700; margin: 24px 0 16px 0; }}

        .table-container {{ background: var(--card-bg); border-radius: var(--radius); overflow: hidden; border: 1px solid var(--border-color); margin-bottom: 32px; box-shadow: var(--shadow-sm); }}
        .table-wrap {{ overflow-x: auto; }}
        table {{ width: 100%; border-collapse: collapse; font-size: 0.95rem; }}
        th {{ background: var(--bg-color); color: var(--text-muted); font-weight: 700; text-align: left; padding: 14px 20px; border-bottom: 1px solid var(--border-color); font-size: 0.75rem; text-transform: uppercase; letter-spacing: 1px; }}
        td {{ padding: 16px 20px; border-bottom: 1px solid var(--bg-color); vertical-align: middle; }}
        tr:hover td {{ background-color: rgba(0, 82, 217, 0.02); }}
        tr:last-child td {{ border-bottom: none; }}
        .text-right {{ text-align: right; }}
        .tabular {{ font-variant-numeric: tabular-nums; }}
        
        .badge {{ display: inline-flex; align-items: center; justify-content: center; padding: 4px 10px; border-radius: 6px; font-size: 0.75rem; font-weight: 800; color: #fff; background: var(--text-muted); white-space: nowrap; }}
        .badge-red {{ background: var(--up-color); }}
        .badge-green {{ background: var(--down-color); }}
        .badge-blue {{ background: var(--info-color); }}
        .badge-orange {{ background: var(--warn-color); }}
        .badge-gray {{ background: var(--text-muted); }}
        .badge-outline {{ background: transparent; border: 1.5px solid currentColor; color: inherit; }}
        
        .text-up {{ color: var(--up-color); font-weight: 800; }}
        .text-down {{ color: var(--down-color); font-weight: 800; }}
        .text-warn {{ color: var(--warn-color); font-weight: 700; }}
        
        .message-banner {{ padding: 16px 24px; margin-bottom: 24px; border-radius: var(--radius); font-size: 1rem; border: 1px solid transparent; font-weight: 600; display: flex; align-items: center; gap: 16px; animation: slideIn 0.3s ease-out; }}
        @keyframes slideIn {{ from {{ transform: translateY(-10px); opacity: 0; }} to {{ transform: translateY(0); opacity: 1; }} }}
        .message-success {{ background: #EFFFF1; color: #008026; border-color: #B2F0C1; }}
        .message-error {{ background: #FFF1F0; color: #AD352F; border-color: #FFCCC7; }}
        .message-warning {{ background: #FFF7E6; color: #996000; border-color: #FFE7BA; }}
        .banner-icon {{ font-size: 1.2rem; font-weight: 900; }}
        
        .form-group {{ margin-bottom: 24px; }}
        .form-group label {{ display: block; margin-bottom: 8px; font-size: 0.9rem; font-weight: 700; color: var(--text-main); }}
        input, select, textarea {{ 
            width: 100%; padding: 12px 16px; border: 1px solid var(--border-color); border-radius: 8px; font-size: 1rem; outline: none; transition: all 0.2s; background: #FFF; font-weight: 500;
        }}
        input:focus, select:focus, textarea:focus {{ border-color: var(--primary-color); box-shadow: 0 0 0 4px rgba(0, 82, 217, 0.1); }}
        
        .btn {{ 
            display: inline-flex; align-items: center; justify-content: center; padding: 10px 24px; background: var(--primary-color); color: #fff; text-decoration: none; border-radius: 8px; 
            font-size: 0.95rem; font-weight: 700; border: none; cursor: pointer; transition: all 0.2s; gap: 8px;
        }}
        .btn:hover {{ opacity: 0.9; transform: translateY(-1px); }}
        .btn:active {{ transform: translateY(0); }}
        .btn:disabled {{ opacity: 0.5; cursor: not-allowed; transform: none; }}
        .btn-sm {{ padding: 6px 16px; font-size: 0.85rem; border-radius: 6px; }}
        .btn-outline {{ background: transparent; border: 1px solid var(--border-color); color: var(--text-main); }}
        .btn-outline:hover {{ background: var(--bg-color); border-color: var(--text-muted); }}
        .btn-ghost {{ background: transparent; color: var(--primary-color); padding: 0; box-shadow: none; font-weight: 600; border: none; cursor: pointer; }}
        .btn-ghost:hover {{ text-decoration: underline; background: transparent; transform: none; }}

        .nav-bottom {{ 
            display: none; position: fixed; bottom: 0; left: 0; right: 0; height: 64px; background: var(--nav-bg); 
            backdrop-filter: blur(10px); -webkit-backdrop-filter: blur(10px); border-top: 1px solid var(--border-color); z-index: 1000;
            justify-content: space-around; align-items: center; padding-bottom: env(safe-area-inset-bottom);
        }}
        .nav-item {{ display: flex; flex-direction: column; align-items: center; text-decoration: none; color: var(--text-muted); font-size: 0.7rem; font-weight: 700; }}
        .nav-item.active {{ color: var(--primary-color); }}
        .nav-icon {{ font-size: 1.25rem; margin-bottom: 2px; }}

        .empty-state {{ text-align: center; padding: 80px 24px; background: var(--card-bg); border-radius: var(--radius); border: 2px dashed var(--border-color); }}
        .empty-state-icon {{ font-size: 3rem; margin-bottom: 16px; display: block; opacity: 0.5; }}
        .empty-state-text {{ color: var(--text-muted); font-size: 1rem; }}
        
        .action-group {{ display: flex; gap: 12px; flex-wrap: wrap; margin-top: 24px; }}

        .ranking-row {{
            display: flex; justify-content: space-between; align-items: center;
            padding: 16px; border-bottom: 1px solid var(--bg-color);
            background: #FFF; border-radius: var(--radius); margin-bottom: 8px;
            box-shadow: var(--shadow-sm); border: 1px solid var(--border-color);
        }}
        .ranking-row:last-child {{ margin-bottom: 0; }}
        .metric-pill {{
            background: var(--bg-color); padding: 4px 10px; border-radius: 6px;
            font-size: 0.75rem; font-weight: 700; color: var(--text-muted);
        }}

        @media (max-width: 768px) {{
            .container {{ padding: 24px 16px; }}
            .nav-desktop {{ display: none; }}
            .nav-bottom {{ display: flex; }}
            .dashboard-grid {{ grid-template-columns: 1fr; gap: 16px; }}
            h1 {{ font-size: 1.75rem; }}
            .card {{ padding: 20px; }}
            .card-value {{ font-size: 2rem; }}
            td, th {{ padding: 12px 16px; }}
        }}
        {}
    </style>
</head>
<body>
    <header>
        <div class="header-wrap">
            <a href="/" class="logo">JDI PORTFOLIO</a>
            <nav class="nav-desktop">
                <a href="/overview">概览</a>
                <a href="/market">市场</a>
                <a href="/holdings">持仓</a>
            </nav>
        </div>
    </header>

    <main class="container">
        {}
        {}
    </main>

    <nav class="nav-bottom">
        <a href="/overview" class="nav-item">
            <span class="nav-icon">📊</span>
            <span>概览</span>
        </a>
        <a href="/market" class="nav-item">
            <span class="nav-icon">📈</span>
            <span>市场</span>
        </a>
        <a href="/holdings" class="nav-item">
            <span class="nav-icon">💰</span>
            <span>持仓</span>
        </a>
    </nav>

    <script>
        document.querySelectorAll('.nav-desktop a, .nav-bottom a').forEach(link => {{
            const path = window.location.pathname;
            const href = link.getAttribute('href');
            if (path === href || (href !== '/' && path.startsWith(href))) {{
                link.classList.add('active');
            }}
        }});
    </script>
<script>
    async function refreshMarket(btn) {{
        const originalText = btn ? btn.innerText : '刷新';
        if (btn) {{
            btn.disabled = true;
            btn.innerText = '⏳ 正在刷新...';
        }}

        try {{
            const res = await fetch('/api/jobs/market/refresh', {{ method: 'POST' }});
            const jr = await res.json();
            if (jr.status !== 'error') {{
                if (btn) btn.innerText = '✔️ 刷新成功';
                setTimeout(() => location.reload(), 800);
            }} else {{
                alert('刷新失败: ' + (jr.message || ''));
                if (btn) {{
                    btn.disabled = false;
                    btn.innerText = originalText;
                }}
            }}
        }} catch (e) {{
            alert('网络错误: ' + e);
            if (btn) {{
                btn.disabled = false;
                btn.innerText = originalText;
            }}
        }}
    }}

    async function autoClassify(btn) {{
        if (btn) {{
            btn.disabled = true;
            btn.innerText = '⏳ 处理中...';
        }}
        try {{
            const res = await fetch('/api/jobs/assets/auto-classify', {{ method: 'POST' }});
            const data = await res.json();
            if (data.success) {{
                location.reload();
            }} else {{
                alert('分类失败: ' + (data.message || ''));
                if (btn) btn.disabled = false;
            }}
        }} catch (e) {{
            alert('网络错误: ' + e);
            if (btn) btn.disabled = false;
        }}
    }}

    async function refreshNav(btn) {{
        if (btn) {{
            btn.disabled = true;
            btn.innerText = '⏳ 正在刷新...';
        }}
        try {{
            const res = await fetch('/api/jobs/nav/refresh', {{ method: 'POST' }});
            const jr = await res.json();
            if (jr.status !== 'error') {{
                if (btn) btn.innerText = '✔️ 刷新成功';
                setTimeout(() => location.reload(), 800);
            }} else {{
                alert('刷新失败: ' + (jr.message || ''));
                if (btn) {{
                    btn.disabled = false;
                    btn.innerText = '💰 刷新净值';
                }}
            }}
        }} catch (e) {{
            alert('网络错误: ' + e);
            if (btn) {{
                btn.disabled = false;
                btn.innerText = '💰 刷新净值';
            }}
        }}
    }}

    async function runDueDca(btn) {{
        if (!confirm('确定要执行今日到期的定投计划吗？')) return;
        if (btn) {{
            btn.disabled = true;
            btn.innerText = '⏳ 正在执行...';
        }}
        try {{
            const res = await fetch('/api/dca/run-due', {{ method: 'POST' }});
            const result = await res.json();
            if (result.success) {{
                if (btn) btn.innerText = '✔️ 执行成功';
                setTimeout(() => location.reload(), 500);
            }} else {{
                alert('执行失败: ' + result.message);
                if (btn) {{
                    btn.disabled = false;
                    btn.innerText = '🤖 执行定投';
                }}
            }}
        }} catch (e) {{
            alert('网络错误: ' + e);
            if (btn) {{
                btn.disabled = false;
                btn.innerText = '🤖 执行定投';
            }}
        }}
    }}
</script>
</body>
</html>
"#,
        title,
        product_extra_css(),
        msg_html,
        content
    ))
}
