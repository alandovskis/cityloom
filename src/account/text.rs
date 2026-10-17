//! The words of a row of the page.
use crate::account::vm::IdentityRow;
use crate::shared::i18n::{Args, I18n};

pub fn identity_text(i18n: &I18n, row: &IdentityRow) -> String {
    match &row.label {
        Some(label) => i18n.tr("account-identity", &Args::new().str("provider", row.provider.clone()).str("label", label.clone())),
        None => i18n.tr("account-identity-bare", &Args::new().str("provider", row.provider.clone())),
    }
}
