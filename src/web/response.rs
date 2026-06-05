//! HTTP response helpers (redirects, flash query params).

use axum::response::Redirect;
use serde::Deserialize;

#[derive(Deserialize, Default, Clone)]
pub struct AdminQuery {
    pub success: Option<String>,
    pub error: Option<String>,
    pub filter: Option<String>,
}

pub fn redirect_with_flash(base: &str, query: &AdminQuery) -> Redirect {
    if let Some(s) = &query.success {
        return Redirect::to(&format!("{}?success={}", base, s));
    }
    if let Some(e) = &query.error {
        return Redirect::to(&format!("{}?error={}", base, e));
    }
    if let Some(f) = &query.filter {
        if base == "/holdings" || base == "/market" {
            return Redirect::to(&format!("{}?filter={}", base, f));
        }
    }
    Redirect::to(base)
}
