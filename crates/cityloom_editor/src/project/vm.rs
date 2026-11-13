//! The home page: the person's projects, to open, create, rename and delete. The view binds to this and decides
//! nothing. It works on the ROOT storage: the list belongs to no project.

use std::rc::{Rc, Weak};

use leptos::prelude::*;

use crate::place::area::{Area, Metro, default_metro, metro, metro_of, metros};
use crate::project::migrate::move_legacy_work;
use crate::project::model::Project;
use crate::project::store::{ProjectStore, last_changed};
use crate::project::text::{changed_text, refused_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::ports::Ports;
use crate::shared::project_id::{FIRST, link};
use crate::shared::said::say;
use crate::shared::units::Units;
use crate::sync::hub;

/// A project as the list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub name: String,
    /// The name of the project's area.
    pub area_label: String,
    /// When it last changed, in the page's language; empty where that is not known.
    pub changed: String,
}

/// A metropolitan area the create form offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetroOption {
    /// Its catalogue id, the option's value.
    pub id: &'static str,
    /// Its name, in the page's language.
    pub label: String,
    /// Whether a new project is made in it.
    pub chosen: bool,
}

/// Where focus should go after what the person was using was removed or moved. The view-model cannot touch the page,
/// so the view moves focus when `ProjectsVm::focus_request` changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Focus {
    None,
    /// The list's heading, which is always there.
    Heading,
    /// The name field of the create form.
    CreateField,
    /// The name field of the rename form, which just opened.
    RenameField,
    /// The Rename button of the project.
    RenameButton(String),
    /// The Delete button of the project.
    DeleteButton(String),
    /// "Cancel", the safe button of the question before deleting.
    DeleteSafe,
}

pub struct ProjectsVm {
    ports: Ports,
    i18n: Rc<I18n>,
    store: ProjectStore,
    /// Changed whenever the list may have: read by `rows` so the view follows.
    version: ArcRwSignal<u32>,
    /// The key of the message saying why the last create was refused.
    create_error: ArcRwSignal<Option<&'static str>>,
    renaming: ArcRwSignal<Option<String>>,
    rename_error: ArcRwSignal<Option<&'static str>>,
    deleting: ArcRwSignal<Option<String>>,
    /// The catalogue id of the metropolitan area a new project is made in.
    metro: ArcRwSignal<&'static str>,
    /// Whether the page was sent here because its project is no longer here, until the person does something.
    missing: ArcRwSignal<bool>,
    focus: ArcRwSignal<(u32, Focus)>,
}

/// Whether the page's query string says it was sent here from a project that is not here (`?missing=1`).
fn was_missing(path: &str) -> bool {
    path.split_once('?').is_some_and(|(_, query)| query.split('#').next().unwrap_or("").split('&').any(|pair| pair == "missing=1"))
}

impl ProjectsVm {
    /// The list of the browser whose ROOT storage `ports` has. The work kept from before projects is first moved into
    /// the first project, named in the page's language; that project is then the one the pages open.
    pub fn new(ports: Ports, i18n: Rc<I18n>) -> Rc<ProjectsVm> {
        let store = ProjectStore::new(ports.storage.clone());
        // A page sent here because it named no project (a bookmark from before projects) has its project once the
        // work is moved: nothing is gone then.
        let moved = move_legacy_work(&ports.storage, &i18n);
        let missing = was_missing(&ports.page.path()) && !moved;
        let vm = Rc::new(ProjectsVm {
            ports,
            i18n,
            store,
            version: ArcRwSignal::new(0),
            create_error: ArcRwSignal::new(None),
            renaming: ArcRwSignal::new(None),
            rename_error: ArcRwSignal::new(None),
            deleting: ArcRwSignal::new(None),
            metro: ArcRwSignal::new(default_metro().id),
            missing: ArcRwSignal::new(missing),
            focus: ArcRwSignal::new((0, Focus::None)),
        });
        // What a sync brings or takes (a project made, renamed or deleted elsewhere, a place changed) shows at once.
        let me: Weak<ProjectsVm> = Rc::downgrade(&vm);
        hub::register(
            || {},
            move |_| {
                if let Some(vm) = me.upgrade() {
                    vm.refresh();
                }
            },
        );
        vm
    }

    pub fn i18n(&self) -> Rc<I18n> {
        self.i18n.clone()
    }

    /// The message `key`, for a view: drawn again when the language is switched.
    pub fn word(&self, key: &str) -> String {
        self.i18n.tr(key, &Args::new())
    }

    /// The message `key` naming the project `name`, for a view.
    pub fn named(&self, key: &str, name: &str) -> String {
        self.i18n.tr(key, &Args::new().str("name", name.to_string()))
    }

    /// Reads the list again: another tab or the sync changed it.
    pub fn refresh(&self) {
        self.version.update(|v| *v = v.wrapping_add(1));
    }

    fn say(&self, key: &str, name: &str) {
        self.ports.announcer.say(&self.i18n.tr_now(key, &Args::new().str("name", name.to_string())));
    }

    /// Says why a name was refused. The reason is also shown beside the field, but a person who submitted with Enter
    /// is still in the field, and moving focus there again tells a screen reader nothing.
    fn refuse(&self, key: &str) {
        self.ports.announcer.say(&self.i18n.tr_now(key, &Args::new()));
    }

    fn ask_focus(&self, to: Focus) {
        self.focus.update(|f| *f = (f.0.wrapping_add(1), to));
    }

    /// Where focus is asked to go, with a count so that asking twice for the same place is seen; tracked.
    pub fn focus_request(&self) -> (u32, Focus) {
        self.focus.get()
    }

    // ---- the list ----

    /// The projects, the one changed last first (creation order between equals); tracked.
    pub fn rows(&self) -> Vec<Row> {
        self.version.track();
        let mut projects: Vec<(String, Project)> = self.store.list().into_iter().map(|p| (last_changed(&*self.ports.storage, &p), p)).collect();
        projects.sort_by(|a, b| b.0.cmp(&a.0));
        projects
            .into_iter()
            .map(|(when, p)| Row { changed: changed_text(&self.i18n, &when), area_label: self.area_label(&p), name: self.name_of(&p), id: p.id })
            .collect()
    }

    /// The name of the metropolitan area a project is in, in the page's language (tracked): the one that holds the
    /// place its record names, whatever build made it. A place no metropolitan area holds is named by its own name;
    /// an area that cannot be read opens the default area, so it is named by the default metropolitan area.
    fn area_label(&self, project: &Project) -> String {
        match Area::from_query(&project.area) {
            Some(area) => match metro_of(&area) {
                Some(metro) => self.metro_name(metro),
                None if area.name.is_empty() => area.key(),
                None => area.name,
            },
            None => self.metro_name(default_metro()),
        }
    }

    fn metro_name(&self, metro: &Metro) -> String {
        // Nothing a metropolitan area's name says has a length in it.
        say(&self.i18n, Units::Metres, &metro.name())
    }

    /// The name a project is shown by. A placeholder (its places came from the sync before its record, or with none, as
    /// an older CityLoom's rows do for the first project) holds no name of the person's: it is named in the page's
    /// language, the first project as the migration names it.
    fn name_of(&self, project: &Project) -> String {
        match (project.placeholder, project.id == FIRST) {
            (false, _) => project.name.clone(),
            (true, true) => self.word("project-first-name"),
            (true, false) => self.word("project-placeholder-name"),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.version.track();
        self.store.list().is_empty()
    }

    /// What the page says first when it was sent here from a project that is no longer here, until the person's first
    /// command; tracked.
    pub fn notice(&self) -> Option<String> {
        self.missing.get().then(|| self.word("project-gone"))
    }

    /// The person did something: the notice of a project that is gone has been seen.
    fn acted(&self) {
        if self.missing.get_untracked() {
            self.missing.set(false);
        }
    }

    /// The metropolitan areas a new project can be made in, the chosen one marked; tracked.
    pub fn metro_options(&self) -> Vec<MetroOption> {
        let chosen = self.metro.get();
        metros().iter().map(|m| MetroOption { id: m.id, label: self.metro_name(m), chosen: m.id == chosen }).collect()
    }

    /// Makes new projects in the metropolitan area the catalogue lists as `id`. False, and nothing changes, where it
    /// lists none.
    pub fn choose_metro(&self, id: &str) -> bool {
        let Some(metro) = metro(id) else { return false };
        self.metro.set(metro.id);
        true
    }

    // ---- create ----

    /// Keeps a new project named `name` in the chosen metropolitan area; says why where it is refused. True where it was kept.
    pub fn create(&self, name: &str) -> bool {
        self.acted();
        let metro = metro(self.metro.get_untracked()).unwrap_or_else(default_metro);
        match self.store.create_in(name, metro) {
            Ok(project) => {
                self.create_error.set(None);
                self.refresh();
                self.say("project-created", &project.name);
                true
            }
            Err(refused) => {
                self.create_error.set(Some(refused_key(refused)));
                self.refuse(refused_key(refused));
                self.ask_focus(Focus::CreateField);
                false
            }
        }
    }

    /// Why the last create was refused; tracked.
    pub fn create_error(&self) -> Option<String> {
        self.create_error.get().map(|key| self.word(key))
    }

    // ---- rename ----

    /// Opens the rename form of project `id`; any question before deleting is dropped.
    pub fn start_rename(&self, id: &str) {
        self.deleting.set(None);
        self.rename_error.set(None);
        self.renaming.set(Some(id.to_string()));
        self.ask_focus(Focus::RenameField);
    }

    pub fn cancel_rename(&self) {
        if let Some(id) = self.renaming.get_untracked() {
            self.renaming.set(None);
            self.rename_error.set(None);
            self.ask_focus(Focus::RenameButton(id));
        }
    }

    /// The project whose rename form is open; tracked.
    pub fn renaming(&self) -> Option<String> {
        self.renaming.get()
    }

    /// Why the last rename was refused; tracked.
    pub fn rename_error(&self) -> Option<String> {
        self.rename_error.get().map(|key| self.word(key))
    }

    /// Gives project `id` the name `name`; says why where it is refused, and the form stays open. True where it was kept.
    pub fn rename(&self, id: &str, name: &str) -> bool {
        self.acted();
        match self.store.rename(id, name) {
            Ok(()) => {
                self.renaming.set(None);
                self.rename_error.set(None);
                self.refresh();
                let kept = self.store.get(id).map(|p| p.name).unwrap_or_default();
                self.say("project-renamed", &kept);
                self.ask_focus(Focus::RenameButton(id.to_string()));
                true
            }
            Err(refused) => {
                self.rename_error.set(Some(refused_key(refused)));
                self.refuse(refused_key(refused));
                self.ask_focus(Focus::RenameField);
                false
            }
        }
    }

    // ---- delete ----

    /// Asks before deleting project `id`; any rename form is closed.
    pub fn ask_delete(&self, id: &str) {
        if self.store.get(id).is_none() {
            return;
        }
        self.renaming.set(None);
        self.deleting.set(Some(id.to_string()));
        self.ask_focus(Focus::DeleteSafe);
    }

    /// The project the question is about, while it is asked; tracked.
    pub fn deleting(&self) -> Option<Row> {
        let id = self.deleting.get()?;
        self.rows().into_iter().find(|r| r.id == id)
    }

    /// The question asked before deleting; tracked.
    pub fn delete_question(&self) -> Option<String> {
        self.deleting().map(|row| self.named("project-delete-confirm", &row.name))
    }

    pub fn cancel_delete(&self) {
        if let Some(id) = self.deleting.get_untracked() {
            self.deleting.set(None);
            self.ask_focus(Focus::DeleteButton(id));
        }
    }

    /// Deletes the project the question is about, with everything in it. False where nothing was asked or the list
    /// could not be changed.
    pub fn confirm_delete(&self) -> bool {
        let Some(id) = self.deleting.get_untracked() else { return false };
        self.acted();
        self.deleting.set(None);
        let name = self.store.get(&id).map(|p| self.name_of(&p)).unwrap_or_default();
        let done = self.store.delete(&id);
        self.refresh();
        if done {
            self.say("project-deleted", &name);
        } else {
            self.ports.announcer.say(&self.i18n.tr_now("project-refused-unusable", &Args::new()));
        }
        self.ask_focus(Focus::Heading);
        done
    }

    // ---- open ----

    /// The address of project `id`'s map.
    pub fn href(&self, id: &str) -> String {
        link("map.html", Some(id))
    }

    /// Remembers project `id` as the one opened last and goes to its map. Nothing where it is not listed.
    pub fn open(&self, id: &str) {
        self.acted();
        if self.store.open(id) {
            self.ports.navigator.go(&self.href(id));
        }
    }
}
