//! Legacy URL redirects to the three-page product.

use axum::response::Redirect;

pub async fn root_redirect() -> Redirect {
    Redirect::permanent("/overview")
}

pub async fn redirect_to_overview() -> Redirect {
    Redirect::permanent("/overview")
}

pub async fn redirect_to_holdings() -> Redirect {
    Redirect::permanent("/holdings")
}

pub async fn redirect_to_market() -> Redirect {
    Redirect::permanent("/market")
}

pub async fn redirect_admin_hidden() -> Redirect {
    Redirect::permanent("/overview")
}
