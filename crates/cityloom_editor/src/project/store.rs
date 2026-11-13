//! The projects as this browser keeps them: one document, `{"version":1,"projects":[…]}`, in creation order.
//! Written one change at a time from what is kept now. A damaged document is copied aside only by the next
//! successful change; one a newer build wrote is never touched.

use std::collections::BTreeSet;
use std::rc::Rc;

use serde_json::{Value, json};

use crate::city::edits::{self, EDITS_KEY, JUNCTIONS, STREETS};
use crate::city::store::AREA_KEY;
use crate::place::area::{Area, Metro, default_area, default_metro};
use crate::project::model::{Project, Refused, clean_name};
use crate::restriction::store::RestrictionStore;
use crate::shared::ports::Storage;
use crate::shared::project_id::{round_trips, split_key, valid, wire_key};
use crate::shared::scoped::{SCOPED_KEYS, ScopedStorage};
use crate::sync::model::{AREA_RECORD, Id, Kind, Loaded, load, mark, mark_all, stamp};

/// The shape of the document.
pub const VERSION: u32 = 1;
/// Where the document is kept.
pub const PROJECTS_KEY: &str = "cityloom-projects";
/// Where a document that could not be read is copied before the next change replaces it.
pub const DAMAGED_KEY: &str = "cityloom-projects-damaged";
/// The id of the project opened last.
pub const LAST_KEY: &str = "cityloom-project";
/// The name a project gets when the sync brings one of its records before the project's own: data only, replaced by
/// the project's record when that comes.
pub const PLACEHOLDER_NAME: &str = "Project";

enum Document {
    Empty,
    Current(Vec<Project>),
    Future,
    Damaged(String),
}

fn parse(stored: Option<&str>) -> Document {
    let Some(raw) = stored.filter(|s| !s.trim().is_empty()) else { return Document::Empty };
    let damaged = || Document::Damaged(raw.to_string());
    let Ok(value) = serde_json::from_str::<Value>(raw) else { return damaged() };
    match value["version"].as_u64() {
        Some(v) if v == u64::from(VERSION) => {}
        Some(v) if v > u64::from(VERSION) => return Document::Future,
        _ => return damaged(),
    }
    match serde_json::from_value::<Vec<Project>>(value["projects"].clone()) {
        Ok(projects) => Document::Current(projects),
        Err(_) => damaged(),
    }
}

/// The listed project `id`; none where it is not listed or the list is damaged or a newer build's. Only reads, so it
/// takes any storage: a `ScopedStorage` passes the list's key through to the root.
pub fn find(storage: &dyn Storage, id: &str) -> Option<Project> {
    match parse(storage.recall(PROJECTS_KEY).as_deref()) {
        Document::Current(projects) => projects.into_iter().find(|p| p.id == id),
        _ => None,
    }
}

/// The person's projects. Takes the root storage, never a `ScopedStorage`: the list belongs to no project.
#[derive(Clone)]
pub struct ProjectStore {
    storage: Rc<dyn Storage>,
}

impl ProjectStore {
    pub fn new(storage: Rc<dyn Storage>) -> ProjectStore {
        ProjectStore { storage }
    }

    fn document(&self) -> Document {
        parse(self.storage.recall(PROJECTS_KEY).as_deref())
    }

    /// Every project, in creation order; none where the document is damaged or a newer build's.
    pub fn list(&self) -> Vec<Project> {
        match self.document() {
            Document::Current(projects) => projects,
            _ => Vec::new(),
        }
    }

    pub fn get(&self, id: &str) -> Option<Project> {
        find(&*self.storage, id)
    }

    /// Whether the document can be read and changed: none kept, or one of this version.
    pub fn is_usable(&self) -> bool {
        matches!(self.document(), Document::Empty | Document::Current(_))
    }

    /// Applies `change` to the list as it is kept now and keeps the result. A damaged document is copied aside once the
    /// change is taken, before it is replaced.
    fn write<R>(&self, change: impl FnOnce(&mut Vec<Project>) -> Result<R, Refused>) -> Result<R, Refused> {
        let (mut projects, damaged) = match self.document() {
            Document::Future => return Err(Refused::Unusable),
            Document::Empty => (Vec::new(), None),
            Document::Current(projects) => (projects, None),
            Document::Damaged(raw) => (Vec::new(), Some(raw)),
        };
        let done = change(&mut projects)?;
        // The first copy may hold what can be recovered: a later one does not replace it.
        if let Some(raw) = damaged
            && self.storage.recall(DAMAGED_KEY).is_none()
            && !self.storage.remember(DAMAGED_KEY, &raw)
        {
            return Err(Refused::Unusable);
        }
        let doc = json!({ "version": VERSION, "projects": projects }).to_string();
        if self.storage.remember(PROJECTS_KEY, &doc) { Ok(done) } else { Err(Refused::Unusable) }
    }

    /// The id a new project takes: random where the storage has randomness, else `p<n>` past the highest number ever
    /// used. Neither takes an id the list or the sync document knows (a deleted project's tombstone must stay dead).
    fn fresh_id(&self, projects: &[Project]) -> String {
        let mut known: BTreeSet<String> = projects.iter().map(|p| p.id.clone()).collect();
        if let Loaded::Current(doc) = load(&*self.storage) {
            known.extend(doc.records.keys().filter_map(|name| Id::from_name(name)).filter(|id| id.kind == Kind::Project).map(|id| id.key));
        }
        if let Some(hex) = self.storage.random_hex() {
            let id = format!("p-{}", &hex[..hex.len().min(12)]);
            if valid(&id) && !known.contains(&id) {
                return id;
            }
        }
        let highest = known.iter().filter_map(|id| id.strip_prefix('p')?.parse::<u64>().ok()).max().unwrap_or(0);
        format!("p{}", highest.saturating_add(1))
    }

    /// Keeps `project` as the sync brought it: replaces the one with its id in place, or adds it at the end. Not an
    /// edit: nothing is marked. False where the list is damaged or a newer build's (the sync never replaces it).
    pub fn put_raw(&self, project: Project) -> bool {
        self.is_usable()
            && self
                .write(|projects| {
                    match projects.iter_mut().find(|p| p.id == project.id) {
                        Some(kept) => *kept = project,
                        None => projects.push(project),
                    }
                    Ok(())
                })
                .is_ok()
    }

    /// Adds a placeholder for `id` where the list has no such project, so the places the sync brings for it are
    /// reachable; its own record replaces it when it comes. True where the project is (now) listed.
    pub fn ensure(&self, id: &str) -> bool {
        if self.get(id).is_some() {
            return true;
        }
        valid(id)
            && self.put_raw(Project {
                id: id.to_string(),
                name: PLACEHOLDER_NAME.to_string(),
                area: default_area().to_query(),
                created_at: stamp(self.storage.now_ms()),
                placeholder: true,
            })
    }

    /// Removes the project as the sync was told: its entry, its scoped keys, the last-opened id if it was this one.
    /// Not an edit: nothing is marked. False where the list is damaged or a newer build's.
    pub fn remove_raw(&self, id: &str) -> bool {
        if !self.is_usable() {
            return false;
        }
        if self.get(id).is_some() && self.write(|projects| Ok(projects.retain(|p| p.id != id))).is_err() {
            return false;
        }
        self.forget(id);
        true
    }

    /// Blanks the project's scoped keys, and the last-opened id where it is this project.
    fn forget(&self, id: &str) {
        for key in SCOPED_KEYS {
            // `Storage` has no removal: an empty value reads as a key never kept, to every store.
            let name = format!("{key}@{id}");
            if self.storage.recall(&name).is_some() {
                self.storage.remember(&name, "");
            }
        }
        if self.storage.recall(LAST_KEY).as_deref() == Some(id) {
            self.storage.remember(LAST_KEY, "");
        }
    }

    /// Keeps a new project in the default metropolitan area, and marks it for the sync.
    pub fn create(&self, name: &str) -> Result<Project, Refused> {
        self.create_in(name, default_metro())
    }

    /// Keeps a new project in the metropolitan area `metro`, and marks it for the sync. The record names the place in
    /// it that the project's map opens on (`Metro::start`), as an area query, the form every build reads; the
    /// metropolitan area is the one that holds it (`place::area::metro_of`), and nothing changes it later.
    pub fn create_in(&self, name: &str, metro: &Metro) -> Result<Project, Refused> {
        let name = clean_name(name)?;
        let created = self.write(|projects| {
            let project =
                Project { id: self.fresh_id(projects), name, area: metro.start().to_query(), created_at: stamp(self.storage.now_ms()), placeholder: false };
            projects.push(project.clone());
            Ok(project)
        })?;
        mark(&*self.storage, Kind::Project, &created.id, false);
        Ok(created)
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<(), Refused> {
        let name = clean_name(name)?;
        let changed = self.write(|projects| {
            let project = projects.iter_mut().find(|p| p.id == id).ok_or(Refused::Unknown)?;
            // Renaming a placeholder makes it the person's project: from now on it is sent.
            let changed = project.name != name || project.placeholder;
            project.name = name;
            project.placeholder = false;
            Ok(changed)
        })?;
        if changed {
            mark(&*self.storage, Kind::Project, id, false);
        }
        Ok(())
    }

    /// Removes the project and its scoped keys, and marks deleted the project record and every record of the project
    /// (those its documents hold and those the sync document knows), so the server and other devices drop them too.
    /// False where there is no such project or the list cannot be changed.
    pub fn delete(&self, id: &str) -> bool {
        let removed = self.write(|projects| projects.iter().position(|p| p.id == id).map(|at| drop(projects.remove(at))).ok_or(Refused::Unknown));
        if removed.is_err() {
            return false;
        }
        // Read before the keys are blanked: afterwards nothing says what the project held.
        let mut gone: BTreeSet<(Kind, String)> = records(&self.storage, id).into_iter().map(|(kind, key)| (kind, wire_key(id, &key))).collect();
        if let Loaded::Current(doc) = load(&*self.storage) {
            gone.extend(
                doc.records
                    .iter()
                    .filter(|(_, entry)| !entry.deleted)
                    .filter_map(|(name, _)| Id::from_name(name))
                    .filter(|record| record.kind != Kind::Project && split_key(&record.key).0 == id)
                    .map(|record| (record.kind, record.key)),
            );
        }
        self.forget(id);
        gone.insert((Kind::Project, id.to_string()));
        mark_all(&*self.storage, &gone.into_iter().collect::<Vec<_>>(), true);
        true
    }

    /// The project opened last, if it still exists.
    pub fn last_opened(&self) -> Option<String> {
        self.storage.recall(LAST_KEY).filter(|id| self.get(id).is_some())
    }

    /// Remembers `id` as the project opened last. False where there is no such project.
    pub fn open(&self, id: &str) -> bool {
        self.get(id).is_some() && self.storage.remember(LAST_KEY, id)
    }
}

/// The project's streets, junctions, rules and area, as kinds and the keys its own documents hold them under (not
/// wire keys). A document that is damaged or a newer build's gives none. Takes the root storage.
pub fn records(storage: &Rc<dyn Storage>, project: &str) -> Vec<(Kind, String)> {
    let scoped: Rc<dyn Storage> = Rc::new(ScopedStorage::new(storage.clone(), project));
    let mut out = Vec::new();
    if let Ok(draft) = edits::draft(scoped.recall(EDITS_KEY).as_deref()) {
        out.extend(draft.raw_keys(STREETS).into_iter().map(|key| (Kind::Street, key)));
        out.extend(draft.raw_keys(JUNCTIONS).into_iter().map(|key| (Kind::Junction, key)));
    }
    out.extend(RestrictionStore::new(scoped.clone()).raw_ids().into_iter().map(|key| (Kind::Rule, key)));
    if scoped.recall(AREA_KEY).is_some_and(|query| Area::from_query(&query).is_some()) {
        out.push((Kind::Area, AREA_RECORD.to_string()));
    }
    // A key whose wire key would name another project (`p-abc:w1` in `first`) is that project's, not this one's.
    out.retain(|(_, key)| round_trips(project, key));
    out
}

/// When the project last changed: the latest `edited_at` among the sync entries of its records (its own
/// `project/<id>` entry too), else when it was created.
pub fn last_changed(storage: &dyn Storage, project: &Project) -> String {
    let Loaded::Current(doc) = load(storage) else { return project.created_at.clone() };
    doc.records
        .iter()
        .filter(|(name, _)| {
            Id::from_name(name).is_some_and(|id| if id.kind == Kind::Project { id.key == project.id } else { split_key(&id.key).0 == project.id })
        })
        .map(|(_, entry)| entry.edited_at.clone())
        .max()
        .unwrap_or_else(|| project.created_at.clone())
}
