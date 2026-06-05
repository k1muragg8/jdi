//! Shared admin-panel CSS/JS (modals, fetch helpers, toasts).

pub fn admin_extra_css() -> &'static str {
    r#"
        .admin-toolbar { display:flex; flex-wrap:wrap; gap:8px; align-items:center; margin-bottom:14px; }
        .admin-toolbar .btn { white-space:nowrap; }
        .metric-editable { cursor:pointer; border-bottom:1px dashed var(--primary-color); }
        .metric-editable:hover { color:var(--primary-color); }
        .source-hint { font-size:0.72rem; color:var(--text-muted); margin-top:4px; }
        .drawer-overlay { display:none; position:fixed; inset:0; background:rgba(0,0,0,.45); z-index:3000; align-items:center; justify-content:center; padding:16px; }
        .drawer-overlay.open { display:flex; }
        .drawer-panel { background:#fff; border-radius:12px; padding:20px; width:100%; max-width:640px; max-height:calc(100vh - 64px); overflow-y:auto; box-shadow:var(--shadow-md); }
        .form-grid { display:grid; grid-template-columns:1fr 1fr; gap:12px; }
        .form-grid label { display:flex; flex-direction:column; gap:4px; font-size:0.8rem; color:var(--text-muted); }
        .form-grid input, .form-grid select, .form-grid textarea { padding:8px 10px; border:1px solid var(--border-color); border-radius:6px; font-size:0.9rem; }
        .form-actions { display:flex; gap:8px; margin-top:16px; flex-wrap:wrap; }
        #adminToast { position:fixed; bottom:90px; left:50%; transform:translateX(-50%); z-index:4000; display:none; padding:10px 16px; border-radius:8px; font-size:0.88rem; box-shadow:var(--shadow-md); max-width:90%; }
        #adminToast.ok { background:#E8FFEA; color:#008026; display:block; }
        #adminToast.err { background:#FFECE8; color:#CB2634; display:block; }
    "#
}

pub fn admin_js_core() -> &'static str {
    r#"
    function showToast(msg, ok) {
        const t = document.getElementById('adminToast');
        if (!t) return alert(msg);
        t.textContent = msg;
        t.className = ok ? 'ok' : 'err';
        setTimeout(() => { t.className = ''; t.style.display = 'none'; }, 4000);
        t.style.display = 'block';
    }
    async function adminFetch(url, opts) {
        const res = await fetch(url, opts || {});
        let data = {};
        try { data = await res.json(); } catch(e) {}
        if (!res.ok && !data.message) data.message = res.statusText;
        return data;
    }
    function openDrawer(id) {
        const el = document.getElementById(id);
        if (el) el.classList.add('open');
    }
    function closeDrawer(id) {
        const el = document.getElementById(id);
        if (el) el.classList.remove('open');
    }
    async function runBtnAction(btn, fn) {
        if (btn) { btn.disabled = true; const t = btn.innerText; btn.dataset.orig = t; btn.innerText = '处理中…'; }
        try {
            const r = await fn();
            if (r && r.success === false) throw new Error(r.message || '操作失败');
            if (r && r.message) showToast(r.message, true);
            else showToast('完成', true);
            setTimeout(() => location.reload(), 600);
            return r;
        } catch(e) {
            showToast(String(e.message || e), false);
            if (btn) { btn.disabled = false; btn.innerText = btn.dataset.orig || '重试'; }
            return null;
        }
    }
    "#
}
