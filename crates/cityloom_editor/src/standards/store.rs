//! The standards profiles this browser keeps: one document, `cityloom-standards`, with the profiles and which one is
//! active. Browser-only: it belongs to no project and is never marked for the sync. Each change re-reads the document,
//! so another tab's profiles survive a write. A profile is exported as a standalone JSON file with its own schema
//! version and imported through the same validation as an edit.

use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::shared::ports::Storage;
use crate::standards::model::{Active, Refusal, Standards, clean_name, set_active};

/// The shape of the document. Raised when the shape changes.
pub const VERSION: u32 = 1;
/// Where the document is kept.
pub const STANDARDS_KEY: &str = "cityloom-standards";
/// Where a document that could not be read is copied before the first write replaces it.
pub const DAMAGED_KEY: &str = "cityloom-standards-damaged";
/// The most profiles a browser keeps.
pub const MAX_PROFILES: usize = 20;
/// What an exported file says it is.
pub const EXPORT_SCHEMA: &str = "cityloom-standards";
/// The version of the exported file's shape. Raised when the shape changes.
pub const EXPORT_VERSION: u32 = 1;

/// A named set of thresholds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub standards: Standards,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Doc {
    version: u32,
    #[serde(default)]
    active: Option<String>,
    #[serde(default)]
    next_id: u32,
    #[serde(default)]
    profiles: Vec<Profile>,
}

impl Doc {
    fn new() -> Doc {
        Doc { version: VERSION, active: None, next_id: 1, profiles: Vec::new() }
    }
}

enum Loaded {
    Empty,
    Current(Doc),
    Future,
    Damaged(String),
}

/// An exported profile.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Export {
    schema: String,
    schema_version: u32,
    name: String,
    standards: Standards,
}

/// The browser's profiles.
#[derive(Clone)]
pub struct StandardsStore {
    storage: Rc<dyn Storage>,
}

impl StandardsStore {
    /// Over `storage`: the key is not one of a project's, so a project's scoped storage reads the same document.
    pub fn new(storage: Rc<dyn Storage>) -> StandardsStore {
        StandardsStore { storage }
    }

    fn load(&self) -> Loaded {
        let Some(raw) = self.storage.recall(STANDARDS_KEY).filter(|r| !r.trim().is_empty()) else { return Loaded::Empty };
        // A newer build's document is recognised by its version alone.
        let version = serde_json::from_str::<serde_json::Value>(&raw).ok().and_then(|v| v.get("version")?.as_u64());
        match version {
            Some(v) if v > u64::from(VERSION) => Loaded::Future,
            _ => match serde_json::from_str::<Doc>(&raw) {
                Ok(doc) if doc.version == VERSION => Loaded::Current(doc),
                _ => Loaded::Damaged(raw),
            },
        }
    }

    /// Every profile, in creation order; none where the document is damaged or a newer build's.
    pub fn list(&self) -> Vec<Profile> {
        match self.load() {
            Loaded::Current(doc) => doc.profiles,
            _ => Vec::new(),
        }
    }

    pub fn get(&self, id: &str) -> Option<Profile> {
        self.list().into_iter().find(|p| p.id == id)
    }

    /// The id of the profile in force, when the document names one that is still there.
    pub fn active_id(&self) -> Option<String> {
        match self.load() {
            Loaded::Current(doc) => doc.active.filter(|id| doc.profiles.iter().any(|p| &p.id == id)),
            _ => None,
        }
    }

    /// The profile in force.
    pub fn active(&self) -> Option<Profile> {
        self.active_id().and_then(|id| self.get(&id))
    }

    /// Whether the document can be changed: none kept, or one of this version (a damaged one is replaced).
    pub fn is_usable(&self) -> bool {
        !matches!(self.load(), Loaded::Future)
    }

    /// Applies `change` to the document as it is kept now and keeps the result, and makes the active profile the one
    /// the street model reads.
    fn write<R>(&self, change: impl FnOnce(&mut Doc) -> Result<R, Refusal>) -> Result<R, Refusal> {
        let (mut doc, damaged) = match self.load() {
            Loaded::Future => return Err(Refusal::Newer),
            Loaded::Empty => (Doc::new(), None),
            Loaded::Current(doc) => (doc, None),
            Loaded::Damaged(raw) => (Doc::new(), Some(raw)),
        };
        let done = change(&mut doc)?;
        if let Some(raw) = damaged
            && self.storage.recall(DAMAGED_KEY).is_none()
            && !self.storage.remember(DAMAGED_KEY, &raw)
        {
            return Err(Refusal::Storage);
        }
        let text = serde_json::to_string(&doc).expect("the document serialises");
        if !self.storage.remember(STANDARDS_KEY, &text) {
            return Err(Refusal::Storage);
        }
        self.activate();
        Ok(done)
    }

    /// Makes the browser's chosen profile the one in force on this page (the built-in figures where none is chosen).
    pub fn activate(&self) {
        set_active(self.active().map(|p| Active { name: p.name, standards: p.standards }));
    }

    fn taken(doc: &Doc, name: &str, except: Option<&str>) -> bool {
        doc.profiles.iter().any(|p| Some(p.id.as_str()) != except && p.name.to_lowercase() == name.to_lowercase())
    }

    fn add(doc: &mut Doc, name: &str, standards: Standards) -> Result<String, Refusal> {
        let name = clean_name(name)?;
        if Self::taken(doc, &name, None) {
            return Err(Refusal::NameTaken);
        }
        if doc.profiles.len() >= MAX_PROFILES {
            return Err(Refusal::TooMany { max: MAX_PROFILES });
        }
        standards.validate()?;
        // An id that was removed is not issued again.
        let highest = doc.profiles.iter().filter_map(|p| p.id.strip_prefix('s')?.parse::<u32>().ok()).max().unwrap_or(0);
        let id = format!("s{}", doc.next_id.max(highest + 1));
        doc.next_id = doc.next_id.max(highest + 1) + 1;
        doc.profiles.push(Profile { id: id.clone(), name, standards });
        Ok(id)
    }

    /// A new profile, with the built-in figures, to be edited. Its id.
    pub fn create(&self, name: &str) -> Result<String, Refusal> {
        self.write(|doc| Self::add(doc, name, Standards::default()))
    }

    /// Gives a profile another name.
    pub fn rename(&self, id: &str, name: &str) -> Result<(), Refusal> {
        self.write(|doc| {
            let name = clean_name(name)?;
            if Self::taken(doc, &name, Some(id)) {
                return Err(Refusal::NameTaken);
            }
            doc.profiles.iter_mut().find(|p| p.id == id).ok_or(Refusal::Unknown)?.name = name;
            Ok(())
        })
    }

    /// Replaces a profile's figures, which must be valid.
    pub fn update(&self, id: &str, standards: Standards) -> Result<(), Refusal> {
        self.write(|doc| {
            standards.validate()?;
            doc.profiles.iter_mut().find(|p| p.id == id).ok_or(Refusal::Unknown)?.standards = standards;
            Ok(())
        })
    }

    /// Chooses the profile in force, or none (the built-in figures).
    pub fn select(&self, id: Option<&str>) -> Result<(), Refusal> {
        self.write(|doc| {
            if let Some(id) = id
                && !doc.profiles.iter().any(|p| p.id == id)
            {
                return Err(Refusal::Unknown);
            }
            doc.active = id.map(str::to_string);
            Ok(())
        })
    }

    /// Removes a profile; if it was in force the built-in figures are again.
    pub fn remove(&self, id: &str) -> Result<(), Refusal> {
        self.write(|doc| {
            let before = doc.profiles.len();
            doc.profiles.retain(|p| p.id != id);
            if doc.profiles.len() == before {
                return Err(Refusal::Unknown);
            }
            if doc.active.as_deref() == Some(id) {
                doc.active = None;
            }
            Ok(())
        })
    }

    /// A profile as a standalone file: its name and figures under the file's own schema version.
    pub fn export(&self, id: &str) -> Option<String> {
        let p = self.get(id)?;
        let file = Export { schema: EXPORT_SCHEMA.into(), schema_version: EXPORT_VERSION, name: p.name, standards: p.standards };
        Some(serde_json::to_string_pretty(&file).expect("the export serialises"))
    }

    /// Adds the profile an exported file holds, as valid as an edited one must be. Its id. It is not chosen.
    pub fn import(&self, text: &str) -> Result<String, Refusal> {
        // The version first, so that a newer file is told apart from one that is no profile.
        let value: serde_json::Value = serde_json::from_str(text).map_err(|_| Refusal::ImportUnreadable)?;
        if value.get("schema").and_then(|s| s.as_str()) != Some(EXPORT_SCHEMA) {
            return Err(Refusal::ImportUnreadable);
        }
        let found = value.get("schema_version").and_then(|v| v.as_u64()).ok_or(Refusal::ImportUnreadable)?;
        if found != u64::from(EXPORT_VERSION) {
            return Err(Refusal::ImportVersion { found: u32::try_from(found).unwrap_or(u32::MAX) });
        }
        let file: Export = serde_json::from_value(value).map_err(|_| Refusal::ImportUnreadable)?;
        self.write(|doc| Self::add(doc, &file.name, file.standards))
    }
}

/// Makes the browser's chosen profile the one in force on this page.
pub fn activate(storage: Rc<dyn Storage>) {
    StandardsStore::new(storage).activate();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::MemoryStorage;
    use crate::standards::model::{self, ACCESS_LANE_MM, Field, WidthRange};

    fn store() -> (StandardsStore, Rc<MemoryStorage>) {
        let storage = Rc::new(MemoryStorage::default());
        (StandardsStore::new(storage.clone()), storage)
    }

    fn tidy() -> model::Restore {
        model::activate_for_test("", Standards::default())
    }

    #[test]
    fn a_profile_is_created_listed_renamed_edited_and_removed() {
        let _r = tidy();
        let (s, _) = store();
        assert!(s.list().is_empty());
        let id = s.create(" City of X ").unwrap();
        assert_eq!(s.list(), vec![Profile { id: id.clone(), name: "City of X".into(), standards: Standards::default() }]);
        s.rename(&id, "Region of Y").unwrap();
        let mut edited = Standards::default();
        edited.access_lane_mm = 3500;
        s.update(&id, edited.clone()).unwrap();
        assert_eq!(s.get(&id).unwrap().name, "Region of Y");
        assert_eq!(s.get(&id).unwrap().standards, edited);
        s.remove(&id).unwrap();
        assert!(s.list().is_empty());
        assert_eq!(s.remove(&id), Err(Refusal::Unknown));
    }

    #[test]
    fn invalid_names_and_figures_are_refused_and_nothing_is_kept() {
        let _r = tidy();
        let (s, storage) = store();
        assert_eq!(s.create("  "), Err(Refusal::NameEmpty));
        let id = s.create("A").unwrap();
        assert_eq!(s.create("a"), Err(Refusal::NameTaken));
        let other = s.create("B").unwrap();
        assert_eq!(s.rename(&other, "A"), Err(Refusal::NameTaken));
        assert_eq!(s.rename(&other, "B"), Ok(()));
        let bad = Standards { access_lane_mm: 10, ..Standards::default() };
        assert!(matches!(s.update(&id, bad), Err(Refusal::Length { field: Field::AccessLane, .. })));
        assert_eq!(s.get(&id).unwrap().standards, Standards::default());
        assert_eq!(storage.keys(), vec![STANDARDS_KEY.to_string()]);
    }

    #[test]
    fn selecting_a_profile_puts_it_in_force_and_clearing_or_removing_it_puts_the_built_in_figures_back() {
        let _r = tidy();
        let (s, _) = store();
        let id = s.create("City of X").unwrap();
        s.update(&id, Standards { access_lane_mm: 3500, ..Standards::default() }).unwrap();
        assert_eq!(model::access_lane_mm(), ACCESS_LANE_MM);
        s.select(Some(&id)).unwrap();
        assert_eq!(s.active_id(), Some(id.clone()));
        assert_eq!(model::access_lane_mm(), 3500);
        assert_eq!(model::active_name().as_deref(), Some("City of X"));
        s.select(None).unwrap();
        assert_eq!(model::access_lane_mm(), ACCESS_LANE_MM);
        s.select(Some(&id)).unwrap();
        s.remove(&id).unwrap();
        assert_eq!(s.active_id(), None);
        assert_eq!(model::active_name(), None);
        assert_eq!(s.select(Some("nope")), Err(Refusal::Unknown));
    }

    #[test]
    fn editing_the_profile_in_force_takes_effect_at_once() {
        let _r = tidy();
        let (s, _) = store();
        let id = s.create("X").unwrap();
        s.select(Some(&id)).unwrap();
        s.update(&id, Standards { access_lane_mm: 4000, ..Standards::default() }).unwrap();
        assert_eq!(model::access_lane_mm(), 4000);
    }

    #[test]
    fn the_chosen_profile_is_in_force_when_a_page_opens() {
        let _r = tidy();
        let (s, storage) = store();
        let id = s.create("X").unwrap();
        s.update(&id, Standards { access_lane_mm: 4000, ..Standards::default() }).unwrap();
        s.select(Some(&id)).unwrap();
        model::set_active(None);
        activate(storage);
        assert_eq!(model::access_lane_mm(), 4000);
    }

    #[test]
    fn an_exported_profile_imports_again_with_its_figures() {
        let _r = tidy();
        let (s, _) = store();
        let id = s.create("City of X").unwrap();
        let mut st = Standards::default();
        st.kind_widths.insert("travel".into(), WidthRange { min_mm: 2800, max_mm: 3600 });
        st.target_kmh.insert("local".into(), 30);
        s.update(&id, st.clone()).unwrap();
        let file = s.export(&id).unwrap();
        assert!(file.contains("\"schema\": \"cityloom-standards\"") && file.contains("\"schema_version\": 1"));
        let (other, _) = store();
        let imported = other.import(&file).unwrap();
        assert_eq!(other.get(&imported).unwrap().name, "City of X");
        assert_eq!(other.get(&imported).unwrap().standards, st);
        // not chosen by being imported
        assert_eq!(other.active_id(), None);
        // the same name again is refused, as when created
        assert_eq!(other.import(&file), Err(Refusal::NameTaken));
    }

    #[test]
    fn an_import_is_validated_like_an_edit() {
        let _r = tidy();
        let (s, _) = store();
        let file = |body: &str| format!(r#"{{"schema":"cityloom-standards","schema_version":1,"name":"X","standards":{body}}}"#);
        assert!(matches!(s.import(&file(r#"{"access_lane_mm":10}"#)), Err(Refusal::Length { field: Field::AccessLane, .. })));
        assert_eq!(s.import(&file(r#"{"speed_choices":[]}"#)), Err(Refusal::SpeedsEmpty));
        assert_eq!(s.import(&file(r#"{"target_kmh":{"lane":30}}"#)), Err(Refusal::UnknownClass("lane".into())));
        assert_eq!(s.import(&file(r#"{"unheard_of":1}"#)), Err(Refusal::ImportUnreadable));
        assert_eq!(s.import("not json"), Err(Refusal::ImportUnreadable));
        assert_eq!(s.import(r#"{"schema":"other","schema_version":1}"#), Err(Refusal::ImportUnreadable));
        assert_eq!(s.import(r#"{"schema":"cityloom-standards","schema_version":2,"name":"X","standards":{}}"#), Err(Refusal::ImportVersion { found: 2 }));
        assert_eq!(s.import(&file("{}")).map(|_| ()), Ok(()));
        assert_eq!(s.list().len(), 1);
    }

    #[test]
    fn a_blocked_storage_refuses_and_a_newer_document_is_left_alone() {
        let _r = tidy();
        let (s, storage) = store();
        storage.blocked(true);
        assert_eq!(s.create("X"), Err(Refusal::Storage));
        storage.blocked(false);
        storage.remember(STANDARDS_KEY, r#"{"version":2,"profiles":[]}"#);
        assert_eq!(s.create("X"), Err(Refusal::Newer));
        assert!(!s.is_usable());
        assert_eq!(storage.recall(STANDARDS_KEY).unwrap(), r#"{"version":2,"profiles":[]}"#);
    }

    #[test]
    fn a_damaged_document_is_copied_aside_and_replaced_by_the_next_change() {
        let _r = tidy();
        let (s, storage) = store();
        storage.remember(STANDARDS_KEY, "{oops");
        assert!(s.list().is_empty());
        s.create("X").unwrap();
        assert_eq!(storage.recall(DAMAGED_KEY).as_deref(), Some("{oops"));
        assert_eq!(s.list().len(), 1);
    }

    #[test]
    fn an_id_that_was_removed_is_not_issued_again() {
        let _r = tidy();
        let (s, _) = store();
        let a = s.create("A").unwrap();
        s.remove(&a).unwrap();
        assert_ne!(s.create("B").unwrap(), a);
    }

    #[test]
    fn a_browser_keeps_a_limited_number_of_profiles() {
        let _r = tidy();
        let (s, _) = store();
        for n in 0..MAX_PROFILES {
            s.create(&format!("P{n}")).unwrap();
        }
        assert_eq!(s.create("one more"), Err(Refusal::TooMany { max: MAX_PROFILES }));
    }

    #[test]
    fn the_profiles_are_kept_for_the_browser_and_not_for_the_sync() {
        let _r = tidy();
        let (s, storage) = store();
        s.create("X").unwrap();
        assert_eq!(storage.keys(), vec![STANDARDS_KEY.to_string()]);
    }
}
