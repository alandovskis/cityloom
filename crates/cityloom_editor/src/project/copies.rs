//! The copies of shared designs this browser has opened (#221). A copy is a project of its own (its documents under
//! `<key>@<id>`, as any project's), so the pages open it as they open any: `context::find` knows it (the person's
//! list, `store::find` and `ProjectStore`, does not), and `context` opens it only to read. It is not one of the person's projects: `ProjectStore::list` (the home
//! page, the sync, the move from before projects) does not hold it, nothing of it is ever marked or sent, and it is
//! kept in a document of its own, `cityloom-shared-copies` (`{"version":1,"copies":[…]}`, oldest first), which a page
//! showing a design read-only cannot write (`shared::scoped::DESIGN_KEYS`).
//!
//! Opening the same link again replaces its copy (the same id). At most `MAX_COPIES` are kept: the oldest goes first,
//! documents and all. Ids are `s-` and 12 hex digits where storage has randomness, else `s<n>`: never a project's own
//! (`p-…`, `p<n>`, `first`), so no record the sync brings can land in a copy. No wording here.

use std::rc::Rc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::project::model::{Project, Refused};
use crate::project::store::ProjectStore;
use crate::shared::ports::Storage;
use crate::shared::project_id::valid;
use crate::shared::scoped::{SCOPED_KEYS, ScopedStorage};

/// Where the copies are listed.
pub const COPIES_KEY: &str = "cityloom-shared-copies";
/// The shape of the document.
pub const VERSION: u32 = 1;
/// The most copies kept on one device: the browser's storage also holds the person's own work.
pub const MAX_COPIES: usize = 10;

/// A copy as listed: the project it opens as, and the link it came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SharedCopy {
    pub id: String,
    /// The share link's token, so the same link opens the same copy. Kept on this device only.
    pub token: String,
    pub name: String,
    /// `Area::to_query()`, as the shared design's project named it.
    pub area: String,
    /// When this device opened it (`sync::model::stamp`).
    pub opened_at: String,
}

impl SharedCopy {
    /// The copy as the pages open a project.
    pub fn as_project(&self) -> Project {
        Project { id: self.id.clone(), name: self.name.clone(), area: self.area.clone(), created_at: self.opened_at.clone(), placeholder: false }
    }
}

/// The copies listed; none where the document is missing, damaged or a newer build's (a copy can always be opened
/// again from its link: nothing of the person's is lost by starting the list over).
pub fn list(storage: &dyn Storage) -> Vec<SharedCopy> {
    let Some(raw) = storage.recall(COPIES_KEY).filter(|s| !s.trim().is_empty()) else { return Vec::new() };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else { return Vec::new() };
    if value["version"].as_u64() != Some(u64::from(VERSION)) {
        return Vec::new();
    }
    serde_json::from_value(value["copies"].clone()).unwrap_or_default()
}

/// The copy `id` as a project, if this browser keeps one.
pub fn find(storage: &dyn Storage, id: &str) -> Option<Project> {
    list(storage).into_iter().find(|c| c.id == id).map(|c| c.as_project())
}

/// Whether `id` is a copy of a shared design (a page opens it only to read).
pub fn is_copy(storage: &dyn Storage, id: &str) -> bool {
    list(storage).iter().any(|c| c.id == id)
}

/// The id a new copy takes: random where the storage has randomness, else `s<n>` past the highest listed; never one a
/// project or a copy has.
fn fresh_id(storage: &Rc<dyn Storage>, copies: &[SharedCopy]) -> String {
    let taken = |id: &str| copies.iter().any(|c| c.id == id) || ProjectStore::new(storage.clone()).list().iter().any(|p| p.id == id);
    if let Some(hex) = storage.random_hex() {
        let id = format!("s-{}", &hex[..hex.len().min(12)]);
        if valid(&id) && !taken(&id) {
            return id;
        }
    }
    let highest = copies.iter().filter_map(|c| c.id.strip_prefix('s')?.parse::<u64>().ok()).max().unwrap_or(0);
    format!("s{}", highest.saturating_add(1))
}

/// Blanks every document of copy `id` (`Storage` has no removal: an empty value reads as a key never kept).
fn blank(storage: &Rc<dyn Storage>, id: &str) {
    for key in SCOPED_KEYS {
        let name = format!("{key}@{id}");
        if storage.recall(&name).is_some_and(|v| !v.is_empty()) {
            storage.remember(&name, "");
        }
    }
}

fn write_list(storage: &Rc<dyn Storage>, copies: &[SharedCopy]) -> bool {
    storage.remember(COPIES_KEY, &json!({ "version": VERSION, "copies": copies }).to_string())
}

/// Keeps a copy of the shared design behind `token`: its project's `name` and `area`, and `documents` (each a key of
/// `SCOPED_KEYS` and the text to keep under it). The copy that link made before is replaced, under the same id; the
/// oldest copy goes past `MAX_COPIES`. Takes the root storage. Nothing is marked. Where a write is refused, what was
/// written is blanked and the list left as it was, a replaced copy gone with it (`Unusable`): the link can be opened
/// again. Returns the copy's id.
pub fn keep(storage: &Rc<dyn Storage>, token: &str, name: &str, area: &str, documents: &[(&str, &str)]) -> Result<String, Refused> {
    let mut copies = list(&**storage);
    let id = copies.iter().find(|c| c.token == token).map(|c| c.id.clone()).unwrap_or_else(|| fresh_id(storage, &copies));
    blank(storage, &id);
    let scoped = ScopedStorage::new(storage.clone(), &id);
    let written = documents.iter().all(|(key, text)| SCOPED_KEYS.contains(key) && scoped.remember(key, text));
    copies.retain(|c| c.token != token);
    copies.push(SharedCopy {
        id: id.clone(),
        token: token.to_string(),
        name: name.to_string(),
        area: area.to_string(),
        opened_at: crate::sync::model::stamp(storage.now_ms()),
    });
    let dropped: Vec<String> = if copies.len() > MAX_COPIES { copies.drain(..copies.len() - MAX_COPIES).map(|c| c.id).collect() } else { Vec::new() };
    if !(written && write_list(storage, &copies)) {
        blank(storage, &id);
        let before: Vec<SharedCopy> = list(&**storage).into_iter().filter(|c| c.id != id).collect();
        write_list(storage, &before);
        return Err(Refused::Unusable);
    }
    for gone in dropped {
        blank(storage, &gone);
    }
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::MemoryStorage;

    const EDITS: &str = "cityloom-edits";
    const RULES: &str = "cityloom-restrictions";

    fn root() -> (Rc<MemoryStorage>, Rc<dyn Storage>) {
        let memory = Rc::new(MemoryStorage::default());
        (memory.clone(), memory)
    }

    #[test]
    fn a_copy_keeps_its_documents_under_its_own_id_and_is_found_but_not_listed_as_a_project() {
        let (memory, storage) = root();
        let store = ProjectStore::new(storage.clone());
        store.create("Mine").unwrap();
        let id = keep(&storage, "tok-a", "Plateau", "area=45.5,-73.5&name=Plateau", &[(EDITS, "{\"version\":5}"), (RULES, "{\"version\":1}")]).unwrap();
        assert_eq!(id, "s1");
        assert_eq!(memory.recall("cityloom-edits@s1").as_deref(), Some("{\"version\":5}"));
        assert_eq!(memory.recall("cityloom-restrictions@s1").as_deref(), Some("{\"version\":1}"));
        let found = crate::project::context::find(&*storage, "s1").expect("the pages find it");
        assert_eq!((found.name.as_str(), found.area.as_str(), found.placeholder), ("Plateau", "area=45.5,-73.5&name=Plateau", false));
        assert!(store.get("s1").is_none(), "the list's own lookup knows only the person's projects");
        assert!(is_copy(&*storage, "s1") && !is_copy(&*storage, "p1"));
        assert_eq!(store.list().iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["p1"], "not one of the person's projects");
        assert!(crate::sync::model::open(&*storage).is_none_or(|doc| doc.records.keys().all(|k| !k.contains("s1"))), "nothing marked");
    }

    #[test]
    fn the_same_link_replaces_its_copy_and_another_link_makes_another() {
        let (memory, storage) = root();
        let first = keep(&storage, "tok-a", "A", "q", &[(EDITS, "one"), (RULES, "rules")]).unwrap();
        let again = keep(&storage, "tok-a", "A2", "q", &[(EDITS, "two")]).unwrap();
        assert_eq!(first, again);
        assert_eq!(memory.recall("cityloom-edits@s1").as_deref(), Some("two"));
        assert_eq!(memory.recall("cityloom-restrictions@s1").as_deref(), Some(""), "what the new copy lacks is gone");
        assert_eq!(list(&*storage).len(), 1);
        assert_eq!(list(&*storage)[0].name, "A2");
        assert_eq!(keep(&storage, "tok-b", "B", "q", &[]).unwrap(), "s2");
        memory.with_randomness(true);
        let random = keep(&storage, "tok-c", "C", "q", &[]).unwrap();
        assert!(random.starts_with("s-") && valid(&random), "{random}");
    }

    #[test]
    fn past_ten_copies_the_oldest_goes_with_its_documents() {
        let (memory, storage) = root();
        for n in 0..MAX_COPIES {
            keep(&storage, &format!("tok-{n}"), "X", "q", &[(EDITS, "e")]).unwrap();
        }
        keep(&storage, "tok-new", "X", "q", &[(EDITS, "e")]).unwrap();
        let ids: Vec<String> = list(&*storage).into_iter().map(|c| c.id).collect();
        assert_eq!(ids.len(), MAX_COPIES);
        assert!(!ids.contains(&"s1".to_string()));
        assert_eq!(memory.recall("cityloom-edits@s1").as_deref(), Some(""));
    }

    #[test]
    fn a_refused_write_leaves_no_copy() {
        let (memory, storage) = root();
        memory.refuse("cityloom-restrictions@s1");
        assert_eq!(keep(&storage, "tok-a", "A", "q", &[(EDITS, "e"), (RULES, "r")]), Err(Refused::Unusable));
        assert!(list(&*storage).is_empty());
        assert_eq!(memory.recall("cityloom-edits@s1").as_deref(), Some(""));
        memory.refuse_nothing();
        memory.refuse(COPIES_KEY);
        assert_eq!(keep(&storage, "tok-a", "A", "q", &[(EDITS, "e")]), Err(Refused::Unusable));
        assert_eq!(memory.recall("cityloom-edits@s1").as_deref(), Some(""));
        assert_eq!(keep(&storage, "tok-a", "A", "q", &[("cityloom-sync", "{}")]), Err(Refused::Unusable), "only a project's documents");
    }

    #[test]
    fn a_damaged_or_newer_list_holds_no_copy() {
        let (memory, storage) = root();
        for raw in ["{", "{\"version\":2,\"copies\":[]}", "{\"version\":1,\"copies\":7}"] {
            memory.remember(COPIES_KEY, raw);
            assert!(list(&*storage).is_empty(), "{raw}");
        }
    }
}
