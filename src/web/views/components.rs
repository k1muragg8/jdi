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
