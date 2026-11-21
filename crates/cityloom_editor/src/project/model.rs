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

/// The share links made on this device of a project (#221), which must stop working before the project goes, so that
/// whoever made them can still end them: once the project is gone, its list of links is gone with it. The share slice
/// keeps them; the composition root (`lib.rs`) hands them to the home page, so the project slice uses no other slice.
pub trait ProjectLinks {
    /// How many links made on this device of project `id` still work by this device's clock: what the question says.
    fn live(&self, id: &str) -> usize;
    /// How many links made on this device of project `id` are listed, expired here or not: any of them is sent to the
    /// server before the project goes, since this device's clock may be wrong.
    fn listed(&self, id: &str) -> usize;
    /// Deletes them on the server, one after the other; `progress` hears each step (a link ended, or a wait the
    /// server asked for), `done` hears `Ok` once none is left, or why one could not be deleted (data: a `Said`), the
    /// rest then left as they are.
    fn end(&self, id: &str, progress: Box<dyn Fn(Ending)>, done: Box<dyn FnOnce(Result<(), crate::shared::said::Said>)>);
}

/// A step of ending a project's links.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// A link was ended; the next is on its way.
    Going,
    /// The server asked to wait (its rate limit): the same link is asked again shortly.
    Waiting,
}

/// No links: a page that makes none (and the tests that need none).
pub struct NoLinks;

impl ProjectLinks for NoLinks {
    fn live(&self, _: &str) -> usize {
        0
    }
    fn listed(&self, _: &str) -> usize {
        0
    }
    fn end(&self, _: &str, _: Box<dyn Fn(Ending)>, done: Box<dyn FnOnce(Result<(), crate::shared::said::Said>)>) {
        done(Ok(()));
    }
}
