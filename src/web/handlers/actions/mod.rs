//! POST form actions and templates (redirect responses).

mod assets;
mod cash;
mod dca_admin;
mod instruments;
mod reconcile;
mod templates;
mod types;

pub use crate::web::handlers::forms::{AssetIdForm, CashAdjustForm, CashSetForm};
pub use assets::*;
pub use cash::*;
pub use dca_admin::*;
pub use instruments::*;
pub use reconcile::*;
pub use templates::*;
pub use types::*;
