//! What every page shares: the settings menu (units, region, theme), the details
//! and notes sidebars and the notes tabs. The view-model decides; the view binds
//! the markup each page keeps for them.

#[cfg(test)]
mod tests;
pub mod view;
pub mod vm;

use crate::shared::i18n::Resources;

pub use view::mount;
pub use vm::{Bare, Target};

/// What the shell says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };
