//! The copies of shared designs this browser has opened (#221). A copy is a project of its own (its documents under
//! `<key>@<id>`, as any project's), so the pages open it as they open any: `context::find` knows it (the person's
//! list, `store::find` and `ProjectStore`, does not), and `context` opens it only to read. It is not one of the person's projects: `ProjectStore::list` (the home
//! page, the sync, the move from before projects) does not hold it, nothing of it is ever marked or sent, and it is
//! kept in a document of its own, `cityloom-shared-copies` (`{"version":1,"copies":[…]}`, oldest first), which a page
//! showing a design read-only cannot write (`shared::scoped::DESIGN_KEYS`).
//!
//! Opening the same link again replaces its copy (the same id). At most `MAX_COPIES` are kept, holding at most
//! `MAX_COPIES_BYTES` together, and the browser's storage refusing a copy makes the oldest make room: the oldest goes
//! first, documents and all. Ids are `s-` and 12 hex digits where storage has randomness, else `s<n>`: never a project's own
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
/// The most text the copies' documents may hold together (two shares of the largest size the server keeps): a
/// browser keeps about 5 MB for a site, and the person's own work comes first. The newest copy is always kept.
pub const MAX_COPIES_BYTES: usize = 2 * 1024 * 1024;

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

/// How much text the documents of copy `id` hold.
fn size_of(storage: &Rc<dyn Storage>, id: &str) -> usize {
    SCOPED_KEYS.iter().filter_map(|key| storage.recall(&format!("{key}@{id}"))).map(|text| text.len()).sum()
}

/// Keeps a copy of the shared design behind `token`: its project's `name` and `area`, and `documents` (each a key of
/// `SCOPED_KEYS` and the text to keep under it). The copy that link made before is replaced, under the same id. The
/// oldest copies go, documents and all, past `MAX_COPIES` or `MAX_COPIES_BYTES` (the new one counted first), before
/// anything is written; and where the browser still refuses a write (its storage is full: the person's own work is
/// there too), the oldest copy left goes and the copy is written again, until none is left to go. Takes the root
/// storage. Nothing is marked. Where the copy cannot be kept, what was written of it is blanked and it is not listed
/// (`Unusable`), a replaced copy gone with it: every copy can be opened again from its link. Returns the copy's id.
pub fn keep(storage: &Rc<dyn Storage>, token: &str, name: &str, area: &str, documents: &[(&str, &str)]) -> Result<String, Refused> {
    let copies = list(&**storage);
    let id = copies.iter().find(|c| c.token == token).map(|c| c.id.clone()).unwrap_or_else(|| fresh_id(storage, &copies));
    blank(storage, &id);
    // The budget, newest first: once a copy does not fit, it and every older one go.
    let mut total: usize = documents.iter().map(|(_, text)| text.len()).sum();
    let (mut others, mut full) = (Vec::new(), false);
    for copy in copies.into_iter().rev().filter(|c| c.token != token) {
        let size = size_of(storage, &copy.id);
        full = full || others.len() + 1 >= MAX_COPIES || total + size > MAX_COPIES_BYTES;
        if full {
            blank(storage, &copy.id);
        } else {
            total += size;
            others.insert(0, copy);
        }
    }
    let copy = SharedCopy {
        id: id.clone(),
        token: token.to_string(),
        name: name.to_string(),
        area: area.to_string(),
        opened_at: crate::sync::model::stamp(storage.now_ms()),
    };
    let known = documents.iter().all(|(key, _)| SCOPED_KEYS.contains(key));
    let scoped = ScopedStorage::new(storage.clone(), &id);
    loop {
        let listed: Vec<SharedCopy> = others.iter().cloned().chain([copy.clone()]).collect();
        if known && documents.iter().all(|(key, text)| scoped.remember(key, text)) && write_list(storage, &listed) {
            return Ok(id);
        }
        blank(storage, &id);
        if !known || others.is_empty() {
            break;
        }
        // The browser's storage is full: the oldest copy left makes room.
        blank(storage, &others.remove(0).id);
    }
    write_list(storage, &others);
    Err(Refused::Unusable)
}

/// The documents a fork takes from its copy: the design as it was shared (`share::model`'s two documents; the share
/// slice's tests check every document a share holds is here).
pub const FORKED_KEYS: [&str; 2] = [crate::city::edits::EDITS_KEY, crate::restriction::store::RESTRICTIONS_KEY];

/// Why a copy of a shared design was not made one of the person's projects (`fork`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unforked {
    /// No such copy: never kept, or gone since (the oldest of more than ten copies makes room).
    Gone,
    /// The browser's storage refused the project or its design (most likely full).
    NotKept,
    /// The list of projects is a newer CityLoom's: this build may not change it.
    Newer,
}

/// Makes the copy `id` of a shared design one of the person's projects, to edit (#223): a new project, listed, with
/// the copy's name and area and a copy of its design, marked for the sync with each of its places as the person's own
/// work. The copy stays as it was shared, still a copy (its link opens it again, and replaces it, never the fork).
/// Where it cannot be made (`Unforked`), nothing of it is left listed. Takes the root storage.
pub fn fork(storage: &Rc<dyn Storage>, id: &str) -> Result<Project, Unforked> {
    let copy = list(&**storage).into_iter().find(|c| c.id == id).ok_or(Unforked::Gone)?;
    let shared = ScopedStorage::new(storage.clone(), id);
    let documents: Vec<(&str, String)> =
        FORKED_KEYS.iter().filter_map(|key| shared.recall(key).filter(|text| !text.is_empty()).map(|text| (*key, text))).collect();
    let store = ProjectStore::new(storage.clone());
    if store.is_newer() {
        return Err(Unforked::Newer);
    }
    let project = store.create_at(&copy.name, &copy.area).map_err(|_| Unforked::NotKept)?;
    let forked = ScopedStorage::new(storage.clone(), &project.id);
    if !documents.iter().all(|(key, text)| forked.remember(key, text)) {
        store.delete(&project.id);
        return Err(Unforked::NotKept);
    }
    let records = crate::project::store::records(storage, &project.id);
    crate::sync::model::mark_all(&forked, &records, false);
    Ok(project)
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
    fn past_the_size_budget_the_oldest_copies_go_with_their_documents() {
        let (memory, storage) = root();
        let big = "x".repeat(MAX_COPIES_BYTES / 2 - 10);
        let first = keep(&storage, "tok-a", "A", "q", &[(EDITS, &big)]).unwrap();
        let second = keep(&storage, "tok-b", "B", "q", &[(EDITS, &big)]).unwrap();
        assert_eq!(list(&*storage).len(), 2, "two halves of the budget fit");
        let third = keep(&storage, "tok-c", "C", "q", &[(EDITS, &big)]).unwrap();
        let ids: Vec<String> = list(&*storage).into_iter().map(|c| c.id).collect();
        assert_eq!(ids, [second.clone(), third.clone()], "the oldest went for the newest");
        assert_eq!(memory.recall(&format!("cityloom-edits@{first}")).as_deref(), Some(""));
        // Replacing a copy counts its new documents, not its old ones too.
        keep(&storage, "tok-c", "C", "q", &[(EDITS, &big)]).unwrap();
        assert_eq!(list(&*storage).len(), 2);
        // A copy over the whole budget alone is still kept: the others make room for it.
        keep(&storage, "tok-d", "D", "q", &[(EDITS, &"y".repeat(MAX_COPIES_BYTES + 1))]).unwrap();
        assert_eq!(list(&*storage).into_iter().map(|c| c.token).collect::<Vec<_>>(), ["tok-d"]);
    }

    /// The browser's storage is shared with the person's own work: when it refuses a copy, the oldest copies make room,
    /// one at a time, before the copy is given up.
    #[test]
    fn a_full_storage_drops_the_oldest_copies_to_keep_a_new_one() {
        let (memory, storage) = root();
        let doc = "x".repeat(1000);
        let first = keep(&storage, "tok-a", "A", "q", &[(EDITS, &doc)]).unwrap();
        let second = keep(&storage, "tok-b", "B", "q", &[(EDITS, &doc)]).unwrap();
        // Room for what is kept now and a little more, not for a third copy.
        memory.hold_at_most(memory.held() + 500);
        let third = keep(&storage, "tok-c", "C", "q", &[(EDITS, &doc)]).expect("the oldest made room");
        let ids: Vec<String> = list(&*storage).into_iter().map(|c| c.id).collect();
        assert_eq!(ids, [second.clone(), third.clone()]);
        assert_eq!(memory.recall(&format!("cityloom-edits@{first}")).as_deref(), Some(""));
        assert_eq!(memory.recall(&format!("cityloom-edits@{third}")).as_deref(), Some(doc.as_str()));
        // With no other copy to drop, a copy too large for what is left is given up and nothing of it is kept.
        assert_eq!(keep(&storage, "tok-d", "D", "q", &[(EDITS, &"z".repeat(10_000))]), Err(Refused::Unusable));
        assert!(list(&*storage).iter().all(|c| c.token != "tok-d"));
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

    /// An edits document holding one street, as an editor writes it.
    fn edits_with_street(key: &str) -> String {
        let mut draft = crate::city::edits::Draft::new();
        assert!(draft.put_raw(crate::city::edits::STREETS, key, json!({ "v": 1 })));
        draft.to_json()
    }

    fn pending(storage: &dyn Storage) -> Vec<String> {
        crate::sync::model::open(storage).map(|doc| doc.records.into_iter().filter(|(_, e)| e.pending).map(|(name, _)| name).collect()).unwrap_or_default()
    }

    #[test]
    fn forking_a_copy_makes_one_of_the_persons_projects_holding_the_same_design() {
        let (memory, storage) = root();
        let edits = edits_with_street("w1~n2~n3");
        let area = "area=45.5,-73.5&name=Plateau";
        let copy = keep(&storage, "tok-a", "Plateau", area, &[(EDITS, &edits), (RULES, "{\"version\":1}")]).unwrap();
        memory.with_randomness(true);
        let forked = fork(&storage, &copy).expect("forked");
        assert_ne!(forked.id, copy);
        assert!(forked.id.starts_with("p-"), "an id of the person's own: {}", forked.id);
        assert_eq!((forked.name.as_str(), forked.area.as_str(), forked.placeholder), ("Plateau", area, false));
        let listed = ProjectStore::new(storage.clone()).list();
        assert_eq!(listed, [forked.clone()], "listed among the person's projects");
        assert!(!is_copy(&*storage, &forked.id), "not a copy: a page opens it to edit");
        assert!(!crate::project::context::read_only(&*storage, Some(&forked.id), &format!("project={}", forked.id)));
        assert_eq!(memory.recall(&format!("cityloom-edits@{}", forked.id)), Some(edits.clone()));
        assert_eq!(memory.recall(&format!("cityloom-restrictions@{}", forked.id)).as_deref(), Some("{\"version\":1}"));
        // The fork is the person's work: the sync sends the project and its places.
        let marked = pending(&*storage);
        assert!(marked.contains(&format!("project/{}", forked.id)), "{marked:?}");
        assert!(marked.contains(&format!("street/{}:w1~n2~n3", forked.id)), "{marked:?}");
        // The copy stays as it was shared, still a copy.
        assert!(is_copy(&*storage, &copy));
        assert_eq!(memory.recall(&format!("cityloom-edits@{copy}")), Some(edits));
        assert!(marked.iter().all(|name| !name.contains(&copy)), "nothing of the copy is sent: {marked:?}");
    }

    #[test]
    fn editing_a_fork_leaves_the_shared_copy_as_it_was() {
        let (memory, storage) = root();
        let edits = edits_with_street("w1~n2~n3");
        let copy = keep(&storage, "tok-a", "A", "area=45.5,-73.5&name=A", &[(EDITS, &edits)]).unwrap();
        let forked = fork(&storage, &copy).unwrap();
        let scoped = ScopedStorage::new(storage.clone(), &forked.id);
        assert!(scoped.remember(EDITS, &edits_with_street("w9~n8~n7")));
        assert_eq!(memory.recall(&format!("cityloom-edits@{copy}")), Some(edits), "the copy keeps the shared design");
        // Opening the same link again replaces the copy, never the fork.
        keep(&storage, "tok-a", "A", "area=45.5,-73.5&name=A", &[(EDITS, "{}")]).unwrap();
        assert_eq!(memory.recall(&format!("cityloom-edits@{}", forked.id)), Some(edits_with_street("w9~n8~n7")));
    }

    #[test]
    fn forking_twice_makes_two_projects() {
        let (_, storage) = root();
        let copy = keep(&storage, "tok-a", "A", "area=45.5,-73.5&name=A", &[]).unwrap();
        let one = fork(&storage, &copy).unwrap();
        let two = fork(&storage, &copy).unwrap();
        assert_ne!(one.id, two.id);
        assert_eq!(ProjectStore::new(storage.clone()).list().len(), 2);
    }

    #[test]
    fn only_a_copy_can_be_forked() {
        let (_, storage) = root();
        let mine = ProjectStore::new(storage.clone()).create("Mine").unwrap();
        assert_eq!(fork(&storage, &mine.id), Err(Unforked::Gone), "a project of the person's is not a copy");
        assert_eq!(fork(&storage, "s9"), Err(Unforked::Gone));
        assert_eq!(ProjectStore::new(storage.clone()).list().len(), 1);
    }

    #[test]
    fn a_fork_the_browser_cannot_keep_leaves_no_project() {
        let (memory, storage) = root();
        let copy = keep(&storage, "tok-a", "A", "area=45.5,-73.5&name=A", &[(EDITS, &edits_with_street("w1~n2~n3"))]).unwrap();
        // The new project's id is `p1` (no randomness): its edits cannot be written.
        memory.refuse("cityloom-edits@p1");
        assert_eq!(fork(&storage, &copy), Err(Unforked::NotKept));
        assert!(ProjectStore::new(storage.clone()).list().is_empty(), "nothing listed");
        assert!(pending(&*storage).iter().all(|name| !name.starts_with("street/")), "none of its places is sent");
        assert!(is_copy(&*storage, &copy), "the copy is still there");
        // A list of projects the browser cannot write keeps no fork either.
        memory.refuse_nothing();
        memory.refuse(crate::project::store::PROJECTS_KEY);
        assert_eq!(fork(&storage, &copy), Err(Unforked::NotKept));
    }

    /// A list of projects a newer CityLoom wrote cannot take the fork, and no free space would change that: it is a
    /// refusal of its own, so the page can say to reload rather than to free space.
    #[test]
    fn a_fork_into_a_newer_builds_list_of_projects_is_refused_as_such() {
        let (memory, storage) = root();
        let copy = keep(&storage, "tok-a", "A", "area=45.5,-73.5&name=A", &[(EDITS, &edits_with_street("w1~n2~n3"))]).unwrap();
        let newer = "{\"version\":99,\"projects\":[]}";
        memory.remember(crate::project::store::PROJECTS_KEY, newer);
        assert_eq!(fork(&storage, &copy), Err(Unforked::Newer));
        assert_eq!(memory.recall(crate::project::store::PROJECTS_KEY).as_deref(), Some(newer), "the newer list is left as it was");
        // A damaged list is not a newer one: it is copied aside and replaced, and the fork is made.
        memory.remember(crate::project::store::PROJECTS_KEY, "{");
        assert!(fork(&storage, &copy).is_ok());
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
