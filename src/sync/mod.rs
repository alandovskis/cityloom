//! Cloud edits on the app side: the sync document that marks what waits to be sent, the pure rules of merging and
//! batching, and the view-model that runs a sync when a page opens, after a write, when the tab shows again, when the
//! network comes back and when the page is left.

use crate::shared::i18n::Resources;

pub mod hub;
pub mod local;
pub mod model;
#[cfg(test)]
mod tests;
pub mod text;
pub mod vm;

pub use vm::{Status, SyncVm};

/// What syncing says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };
