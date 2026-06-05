//! Reusable HTML fragments (badges, banners, tables).

pub mod admin_ui;

pub use admin_ui::{admin_extra_css, admin_js_core};

pub fn flash_success(message: &str) -> String {
    format!(
        r#"<div class="message-banner message-success"><span class="banner-icon">✓</span><span>{}</span></div>"#,
        message
    )
}

pub fn flash_error(message: &str) -> String {
    format!(
        r#"<div class="message-banner message-error"><span class="banner-icon">✕</span><span>{}</span></div>"#,
        message
    )
}

pub fn metric_card(title: &str, value: &str, sub: &str) -> String {
    format!(
        r#"<div class="card"><div class="card-header"><span class="card-title">{}</span></div><div class="card-value tabular">{}</div><div class="source-hint">{}</div></div>"#,
        title, value, sub
    )
}

pub fn source_hint(text: &str) -> String {
    format!(r#"<div class="source-hint">{}</div>"#, text)
}

pub fn drawer_shell(id: &str, title: &str, content: &str) -> String {
    format!(
        r#"<div id="{}" class="drawer-overlay" onclick="if(event.target===this)closeDrawer('{}')"><div class="drawer-panel" onclick="event.stopPropagation()"><h3 style="margin:0 0 12px;">{}</h3>{}</div></div>"#,
        id, id, title, content
    )
}
