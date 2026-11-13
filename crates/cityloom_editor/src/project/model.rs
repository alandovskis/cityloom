//! A project and why a change to the list is refused. No wording here: `Refused` is data.

use serde::{Deserialize, Serialize};

/// The longest name, in characters.
pub const NAME_MAX: usize = 80;

/// A named container tied to an area.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// `first`, `p-<hex>` or `p<n>` (`shared::project_id::valid`).
    pub id: String,
    pub name: String,
    /// `Area::to_query()`.
    pub area: String,
    /// `sync::model::stamp`.
    pub created_at: String,
    /// Made by the sync for a project whose places came before its own record (`ProjectStore::ensure`): kept on this
    /// device only and never sent, so it cannot replace the project's name elsewhere; the project's record replaces it
    /// when it comes, and a rename makes it the person's own.
    #[serde(default, skip_serializing_if = "is_false")]
    pub placeholder: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Why a change was not taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    Empty,
    TooLong,
    Unknown,
    /// The list is damaged or a newer build's.
    Unusable,
}

/// The name as kept: trimmed, not empty, not over `NAME_MAX` characters.
pub fn clean_name(name: &str) -> Result<String, Refused> {
    let name = name.trim();
    if name.is_empty() {
        Err(Refused::Empty)
    } else if name.chars().count() > NAME_MAX {
        Err(Refused::TooLong)
    } else {
        Ok(name.to_string())
    }
}
