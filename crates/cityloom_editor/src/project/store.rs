//! The projects as this browser keeps them: one document, `{"version":1,"projects":[…]}`, in creation order.
//! Written one change at a time from what is kept now. A damaged document is copied aside only by the next
//! successful change; one a newer build wrote is never touched.

use std::rc::Rc;

use serde_json::{Value, json};

use crate::place::area::default_area;
use crate::project::model::{Project, Refused, clean_name};
use crate::shared::ports::Storage;
use crate::shared::project_id::{split_key, valid};
use crate::shared::scoped::SCOPED_KEYS;
use crate::sync::model::{Id, Kind, Loaded, load, mark, stamp};

/// The shape of the document.
pub const VERSION: u32 = 1;
/// Where the document is kept.
pub const PROJECTS_KEY: &str = "cityloom-projects";
/// Where a document that could not be read is copied before the next change replaces it.
pub const DAMAGED_KEY: &str = "cityloom-projects-damaged";
/// The id of the project opened last.
pub const LAST_KEY: &str = "cityloom-project";

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
        self.list().into_iter().find(|p| p.id == id)
    }

    /// Whether the document can be read and changed: none kept, or one of this version.
    pub fn is_usable(&self) -> bool {
        matches!(self.document(), Document::Empty | Document::Current(_))
    }

    /// Applies `change` to the list as it is kept now and keeps the result. A damaged document is copied aside first.
    fn write<R>(&self, change: impl FnOnce(&mut Vec<Project>) -> Result<R, Refused>) -> Result<R, Refused> {
        let mut projects = match self.document() {
            Document::Future => return Err(Refused::Unusable),
            Document::Empty => Vec::new(),
            Document::Current(projects) => projects,
            Document::Damaged(raw) => {
                // The first copy may hold what can be recovered: a later one does not replace it.
                if self.storage.recall(DAMAGED_KEY).is_none() && !self.storage.remember(DAMAGED_KEY, &raw) {
                    return Err(Refused::Unusable);
                }
                Vec::new()
            }
        };
        let done = change(&mut projects)?;
        let doc = json!({ "version": VERSION, "projects": projects }).to_string();
        if self.storage.remember(PROJECTS_KEY, &doc) { Ok(done) } else { Err(Refused::Unusable) }
    }

    /// The id a new project takes: random where the storage has randomness, else the next unused `p<n>`.
    fn fresh_id(&self, projects: &[Project]) -> String {
        if let Some(hex) = self.storage.random_hex() {
            let id = format!("p-{}", &hex[..hex.len().min(12)]);
            if valid(&id) && projects.iter().all(|p| p.id != id) {
                return id;
            }
        }
        (1..).map(|n| format!("p{n}")).find(|id| projects.iter().all(|p| &p.id != id)).expect("some number is unused")
    }

    /// Keeps a new project in the default area, and marks it for the sync.
    pub fn create(&self, name: &str) -> Result<Project, Refused> {
        let name = clean_name(name)?;
        let created = self.write(|projects| {
            let project = Project { id: self.fresh_id(projects), name, area: default_area().to_query(), created_at: stamp(self.storage.now_ms()) };
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
            let changed = project.name != name;
            project.name = name;
            Ok(changed)
        })?;
        if changed {
            mark(&*self.storage, Kind::Project, id, false);
        }
        Ok(())
    }

    /// Removes the project, its scoped keys, and marks the project record deleted. Its streets, junctions, rules and
    /// area records are not marked here. False where there is no such project or the list cannot be changed.
    pub fn delete(&self, id: &str) -> bool {
        let removed = self.write(|projects| projects.iter().position(|p| p.id == id).map(|at| drop(projects.remove(at))).ok_or(Refused::Unknown));
        if removed.is_err() {
            return false;
        }
        for key in SCOPED_KEYS {
            // `Storage` has no removal: an empty value reads as a key never kept, to every store.
            self.storage.remember(&format!("{key}@{id}"), "");
        }
        if self.storage.recall(LAST_KEY).as_deref() == Some(id) {
            self.storage.remember(LAST_KEY, "");
        }
        mark(&*self.storage, Kind::Project, id, true);
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
