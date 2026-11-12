//! Moves the work a browser kept before projects existed into the first project.
//!
//! Before projects, a browser kept one set of documents under bare keys (`cityloom-edits`, ...). Now each project keeps
//! its own under `<key>@<project>`; this moves the old ones under `first`, the project whose records keep bare keys on
//! the wire, so work already on the server is found as it is and two devices migrating apart make the same project.
//!
//! Nothing is lost or doubled on any failure: a value is written under its new key and read back before the old key is
//! blanked (`Storage` has no removal: an empty value reads as absent), the project is listed only once every document
//! is in place, and a document already there (pulled by the sync into a placeholder `first`) is merged with the old
//! one record by record, never replaced. The sync document is left alone: its entries for the records are the same
//! under bare wire keys, so what was synced stays synced and what was never marked is claimed by the sync, each with
//! its own time. Only the project's own record, which never existed before, is given an entry (the oldest possible).

use std::rc::Rc;

use serde_json::Value;

use crate::city::edits::{self, EDITS_KEY, JUNCTIONS, STREETS};
use crate::city::store::AREA_KEY;
use crate::place::area::{Area, default_area, is_montreal};
use crate::project::model::{Project, clean_name};
use crate::project::store::ProjectStore;
use crate::restriction::store::{RESTRICTIONS_KEY, RestrictionStore};
use crate::shared::i18n::{Args, I18n};
use crate::shared::ports::Storage;
use crate::shared::project_id::FIRST;
use crate::shared::scoped::{SCOPED_KEYS, ScopedStorage};
use crate::sync::model::{Id, Kind, Loaded, claim_at, load, stamp};

/// What is kept under `key`, where it is more than blank.
fn kept(storage: &Rc<dyn Storage>, key: &str) -> Option<String> {
    storage.recall(key).filter(|value| !value.trim().is_empty())
}

/// Whether the device holds an edit of the record that has not been sent: what the sync's own `decide` would keep over
/// a pulled copy.
fn pending(storage: &Rc<dyn Storage>, kind: Kind, key: &str) -> bool {
    matches!(load(&**storage), Loaded::Current(doc) if doc.entry(&Id::new(kind, key)).is_some_and(|entry| entry.pending))
}

/// How a record found in both the old and the new document is settled: the old one wins only where the device holds an
/// unsent edit of it; otherwise the copy already in the project stands.
fn old_wins(storage: &Rc<dyn Storage>, kind: Kind, key: &str, old: &Value, new: Option<&Value>) -> bool {
    match new {
        None => true,
        Some(new) if new == old => false,
        Some(_) => pending(storage, kind, key),
    }
}

/// The edits documents merged record by record, as text to keep; None where either cannot be read as a document of
/// this version (damaged, or a newer build's): neither is ever written over.
fn merge_edits(storage: &Rc<dyn Storage>, old: &str, new: &str) -> Option<String> {
    let old = edits::draft(Some(old)).ok()?;
    let mut merged = edits::draft(Some(new)).ok()?;
    for (section, kind) in [(STREETS, Kind::Street), (JUNCTIONS, Kind::Junction)] {
        for key in old.raw_keys(section) {
            let Some(record) = old.get_raw(section, &key) else { continue };
            if old_wins(storage, kind, &key, record, merged.get_raw(section, &key)) && !merged.put_raw(section, &key, record.clone()) {
                return None;
            }
        }
    }
    Some(merged.to_json())
}

/// The rules merged the same way, written into the project's own document (the store's write is a whole, valid
/// document each time). False where either document cannot be read or a write failed.
fn merge_rules(storage: &Rc<dyn Storage>, scoped: &Rc<dyn Storage>) -> bool {
    let (old, merged) = (RestrictionStore::new(storage.clone()), RestrictionStore::new(scoped.clone()));
    if !old.is_usable() || !merged.is_usable() {
        return false;
    }
    for id in old.raw_ids() {
        let Some(record) = old.raw(&id) else { continue };
        if old_wins(storage, Kind::Rule, &id, &record, merged.raw(&id).as_ref())
            && !(merged.put_raw(&id, record.clone()) && merged.raw(&id).as_ref() == Some(&record))
        {
            return false;
        }
    }
    true
}

/// Whether `migrate_legacy` has work to move: the list can be changed and holds no project but `first` (none, the
/// placeholder the sync made for it, or the real one a sync brought from a device that moved its work already), and a
/// legacy `cityloom-edits` or `cityloom-restrictions` is kept (an area alone starts nothing). While it is so the sync
/// leaves the places alone (`sync::local::usable`): their entries name bare keys, which are now `first`'s, and a body
/// read from `first`'s still empty documents would go up as a removal.
pub fn legacy_waiting(storage: &Rc<dyn Storage>) -> bool {
    let store = ProjectStore::new(storage.clone());
    store.is_usable() && store.list().iter().all(|p| p.id == FIRST) && (kept(storage, EDITS_KEY).is_some() || kept(storage, RESTRICTIONS_KEY).is_some())
}

/// `migrate_legacy` for a page that opens in no project (the home page, the Account page), before it builds anything:
/// the first project is named in the page's language and, where nothing was opened before, opened, so the pages open
/// the old work as they did. Says whether the move took the work into `first` now (a page sent to the list because it
/// named no project then has its project after all).
pub fn move_legacy_work(storage: &Rc<dyn Storage>, i18n: &I18n) -> bool {
    if !legacy_waiting(storage) {
        return false;
    }
    let store = ProjectStore::new(storage.clone());
    let real = |store: &ProjectStore| store.get(FIRST).is_some_and(|p| !p.placeholder);
    let was_real = real(&store);
    let complete = migrate_legacy(storage, &i18n.tr_now("project-first-name", &Args::new()));
    // A first project made now, or a real one that took the work.
    let moved = real(&store) && (!was_real || complete);
    if moved && store.last_opened().is_none() {
        store.open(FIRST);
    }
    moved
}

/// Moves the work from before projects into project `first`, named `first_name` (words the caller renders), where
/// `legacy_waiting`. A placeholder `first` is adopted (made real, with this name); a real one keeps its record and takes
/// the work as a placeholder would. Says whether it moved everything it found; false leaves what it did not move where
/// it was. Where a document cannot be merged, nothing is changed and the project is not listed, so a later run tries
/// again.
pub fn migrate_legacy(storage: &Rc<dyn Storage>, first_name: &str) -> bool {
    let store = ProjectStore::new(storage.clone());
    if !legacy_waiting(storage) {
        return false;
    }
    // A record a sync brought from a device that moved its work first: its name and area are the person's.
    let listed = store.get(FIRST).is_some_and(|p| !p.placeholder);
    let Ok(name) = clean_name(first_name) else { return false };
    // An area that is not Montreal is ignored: the project is in the default one and the stored value stays untouched.
    let area = kept(storage, AREA_KEY).and_then(|query| Area::from_query(&query)).filter(is_montreal);
    let scoped: Rc<dyn Storage> = Rc::new(ScopedStorage::new(storage.clone(), FIRST));

    // Put every document in place first, confirming each; the project is listed and the old keys blanked only after.
    let mut complete = true;
    let mut settled = Vec::new();
    for key in SCOPED_KEYS.into_iter().filter(|key| *key != AREA_KEY || area.is_some()) {
        let Some(old) = kept(storage, key) else { continue };
        let name = format!("{key}@{FIRST}");
        let present = kept(storage, &name);
        // What the project's key should hold, where it must be written.
        let write = match present.as_deref() {
            None => Some(old.clone()),
            Some(present) if present == old => None,
            Some(_) if key == AREA_KEY => None,
            Some(present) if key == EDITS_KEY => match merge_edits(storage, &old, present) {
                Some(merged) => Some(merged),
                None => return false,
            },
            Some(_) if key == RESTRICTIONS_KEY => {
                if !merge_rules(storage, &scoped) {
                    return false;
                }
                None
            }
            // A salvage copy of a document that could not be read: the one already there stands, the old one stays.
            Some(_) => {
                complete = false;
                continue;
            }
        };
        if let Some(value) = write {
            let wrote = storage.remember(&name, &value);
            if !wrote || storage.recall(&name).as_deref() != Some(value.as_str()) {
                if wrote {
                    // Undo what was written so a later run finds what was there before, not a copy that differs.
                    storage.remember(&name, present.as_deref().unwrap_or(""));
                }
                return false;
            }
        }
        settled.push(key);
    }

    let project = Project { id: FIRST.to_string(), name, area: area.unwrap_or_else(default_area).to_query(), created_at: stamp(0), placeholder: false };
    if !listed && !store.put_raw(project) {
        return false;
    }
    // The project's record is new: it waits to be sent, older than any rename of it made anywhere.
    complete &= claim_at(&**storage, Kind::Project, FIRST, &stamp(0));

    for key in settled {
        complete &= storage.remember(key, "");
    }
    complete
}
