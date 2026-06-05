//! POST form actions and templates (redirect responses).

mod assets;
mod cash;
mod instruments;
mod templates;
mod types;

pub use crate::web::handlers::forms::{AssetIdForm, CashAdjustForm, CashSetForm};
pub use assets::*;
pub use cash::*;
pub use instruments::*;
pub use templates::*;
pub use types::*;
