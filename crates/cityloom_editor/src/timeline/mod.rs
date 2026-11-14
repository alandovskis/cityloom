//! The design timeline: named milestones of a project's work (see `CLAUDE.md`).

pub mod model;
pub mod store;
pub mod text;
pub mod vm;
pub mod work;

#[cfg(test)]
mod tests;

use crate::shared::i18n::Resources;

/// What the timeline says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };
