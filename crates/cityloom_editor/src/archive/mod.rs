//! The portable design archive (issue #74): export a person's design work as one file, and import such a file
//! safely. `model` holds the format and the rules with no I/O, `vm` the Export and Import commands, `view` the Account
//! page's section, `text` the words of what was found.

use crate::shared::i18n::Resources;

pub mod model;
#[cfg(test)]
mod tests;
pub mod text;
pub mod view;
pub mod vm;

/// What the archive section says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };
