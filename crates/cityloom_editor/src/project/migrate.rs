//! Moves the work a browser kept before projects existed into the first project.
//!
//! Before projects, a browser kept one set of documents under bare keys (`cityloom-edits`, ...). Now each project keeps
//! its own under `<key>@<project>`; this moves the old ones under `first`, the project whose records keep bare keys on
//! the wire, so work already on the server is found as it is and two devices migrating apart make the same project.
//!
//! Nothing is lost or doubled on any failure: a value is written under its new key and read back before the old key is
//! blanked (`Storage` has no removal: an empty value reads as absent), the project is listed only once every value is
//! copied, and a copy already in place (a pulled one, or one an earlier run made) is never overwritten.

use std::rc::Rc;

use crate::city::edits::EDITS_KEY;
use crate::city::store::AREA_KEY;
use crate::place::area::{Area, default_area};
use crate::project::model::{Project, clean_name};
use crate::project::store::{ProjectStore, records};
use crate::restriction::store::RESTRICTIONS_KEY;
use crate::shared::ports::Storage;
use crate::shared::project_id::FIRST;
use crate::shared::scoped::{SCOPED_KEYS, ScopedStorage};
use crate::sync::model::{Kind, mark, mark_all, stamp};

/// The metropolitan area the catalogue covers (the island and its surroundings), as a box of degrees.
const MONTREAL_LAT: (f64, f64) = (45.2, 45.8);
const MONTREAL_LON: (f64, f64) = (-74.3, -73.3);

fn in_montreal(area: &Area) -> bool {
    (MONTREAL_LAT.0..=MONTREAL_LAT.1).contains(&area.lat) && (MONTREAL_LON.0..=MONTREAL_LON.1).contains(&area.lon)
}

/// What is kept under `key`, where it is more than blank.
fn kept(storage: &Rc<dyn Storage>, key: &str) -> Option<String> {
    storage.recall(key).filter(|value| !value.trim().is_empty())
}

/// Moves the work from before projects into project `first`, named `first_name` (words the caller renders), and marks
/// it for the sync. Runs where the list can be changed and holds no project (or only the placeholder the sync made for
/// `first`, which it adopts) and a legacy `cityloom-edits` or `cityloom-restrictions` is kept: an area alone starts
/// nothing. Says whether it moved everything it found; false leaves what it did not move where it was.
pub fn migrate_legacy(storage: &Rc<dyn Storage>, first_name: &str) -> bool {
    let store = ProjectStore::new(storage.clone());
    if !store.is_usable() {
        return false;
    }
    match store.list().as_slice() {
        [] => {}
        [only] if only.id == FIRST && only.placeholder => {}
        _ => return false,
    }
    if kept(storage, EDITS_KEY).is_none() && kept(storage, RESTRICTIONS_KEY).is_none() {
        return false;
    }
    let Ok(name) = clean_name(first_name) else { return false };
    // An area that is not Montreal is ignored: the project is in the default one and the stored value stays untouched.
    let area = kept(storage, AREA_KEY).and_then(|query| Area::from_query(&query)).filter(in_montreal);

    // Copy first, confirming each copy; the project is listed and the old keys blanked only after.
    let mut complete = true;
    let mut copied = Vec::new();
    for key in SCOPED_KEYS.into_iter().filter(|key| *key != AREA_KEY || area.is_some()) {
        let Some(value) = kept(storage, key) else { continue };
        let new = format!("{key}@{FIRST}");
        match kept(storage, &new) {
            // Already there (an earlier run got this far): nothing to write.
            Some(existing) if existing == value => copied.push(key),
            // Another copy is there (pulled, or made since): it stands, and the old value stays for the person.
            Some(_) => complete = false,
            None if storage.remember(&new, &value) && storage.recall(&new).as_deref() == Some(value.as_str()) => copied.push(key),
            None => return false,
        }
    }

    let project = Project { id: FIRST.to_string(), name, area: area.unwrap_or_else(default_area).to_query(), created_at: stamp(0), placeholder: false };
    if !store.put_raw(project) {
        return false;
    }
    // Marked as an edit is: the project's record and every record the moved documents hold now travel to the server.
    mark(&**storage, Kind::Project, FIRST, false);
    let scoped = ScopedStorage::new(storage.clone(), FIRST);
    mark_all(&scoped, &records(storage, FIRST), false);

    for key in copied {
        complete &= storage.remember(key, "");
    }
    complete
}
