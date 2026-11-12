//! Which project a page is opened in. The map and the editors work inside one project: its id comes in the page's
//! address (`?project=<id>`), else it is the project opened last; a page whose project is not here (a bad link, a
//! project deleted elsewhere) goes to the list of projects, which says so, and never opens on an empty city.

use std::rc::Rc;

use crate::place::area::Area;
use crate::project::store::ProjectStore;
use crate::shared::ports::{Ports, Storage};
use crate::shared::scoped::{ScopedStorage, in_project};
use crate::sync::hub;
use crate::sync::model::{Change, Kind};

/// Where a page goes when its project is not here: the list, which says that project is no longer here.
pub const MISSING_HREF: &str = "index.html?missing=1";

/// The project a page is opened in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolved {
    Project(String),
    /// The address names a project that is not here, or names none and none was opened before.
    Missing,
}

/// The project of a page whose query string is `search` (with or without its `?`): the one `project=` names when it
/// is listed; with no `project=`, the one opened last when it is still listed. A `project=` naming a project that is
/// not listed is `Missing`: the page does not open another project in its place. Takes the root storage.
pub fn resolve(storage: &Rc<dyn Storage>, search: &str) -> Resolved {
    let store = ProjectStore::new(storage.clone());
    let named = search.trim_start_matches('?').split('&').find_map(|pair| match pair.split_once('=') {
        Some(("project", id)) => Some(id),
        _ => (pair == "project").then_some(""),
    });
    let id = match named {
        Some(id) => store.get(id).map(|p| p.id),
        None => store.last_opened(),
    };
    id.map_or(Resolved::Missing, Resolved::Project)
}

/// The query string of the page `ports.page` is.
fn search_of(ports: &Ports) -> String {
    ports.page.path().split_once('?').map(|(_, search)| search.to_string()).unwrap_or_default()
}

/// `enter`, keeping the project's storage so that it can be shut.
fn enter_scoped(ports: Ports) -> Option<(Ports, Rc<ScopedStorage>)> {
    match resolve(&ports.storage, &search_of(&ports)) {
        Resolved::Project(id) => {
            ProjectStore::new(ports.storage.clone()).open(&id);
            let scoped = Rc::new(ScopedStorage::new(ports.storage.clone(), &id));
            let storage: Rc<dyn Storage> = scoped.clone();
            Some((Ports { storage, ..ports }, scoped))
        }
        Resolved::Missing => {
            ports.navigator.replace(MISSING_HREF);
            None
        }
    }
}

/// The ports of the project the page is opened in, from the page's own (root storage): its storage is the project's
/// and the project is remembered as the one opened last. Where the project is not here, the page is sent to the list
/// (`MISSING_HREF`) and there is none: nothing of the project is read or written.
pub fn enter(ports: Ports) -> Option<Ports> {
    enter_scoped(ports).map(|(ports, _)| ports)
}

/// The area the record of `storage`'s project names (`Storage::project`; the list is read through the scoped storage,
/// which passes its key to the root). None over the root storage, or where the project is not listed, its record is a
/// placeholder the sync made before the real one came, or its area cannot be read: the caller then falls back.
pub fn area_of(storage: &dyn Storage) -> Option<Area> {
    let project = crate::project::store::find(storage, &storage.project()?)?;
    if project.placeholder { None } else { Area::from_query(&project.area) }
}

/// The ports of the project the page is opened in, without sending the page away or remembering the project (the
/// roads to get ahead of the mount, the archive, the home page). None where no project resolves: there is then no
/// project to write into, and what would write stays inert rather than write the root storage.
pub fn peek(ports: Ports) -> Option<Ports> {
    match resolve(&ports.storage, &search_of(&ports)) {
        Resolved::Project(id) => Some(in_project(ports, &id)),
        Resolved::Missing => None,
    }
}

/// `enter`, for the one call that opens a page's map or editor: should a sync remove its project (deleted on another
/// device), the page also goes to the list and its storage is shut.
pub fn open_page(ports: Ports) -> Option<Ports> {
    let root = ports.clone();
    let (ports, scoped) = enter_scoped(ports)?;
    leave_when_removed(&root, &scoped);
    Some(ports)
}

/// When a sync changes the record of `scoped`'s project and the project is no longer listed, shuts `scoped` (nothing
/// more is written or marked through it: an editor's last flush cannot bring the work back) and sends the page to the
/// list (`MISSING_HREF`). `ports` has the root storage.
pub fn leave_when_removed(ports: &Ports, scoped: &Rc<ScopedStorage>) {
    let (storage, navigator, scoped) = (ports.storage.clone(), ports.navigator.clone(), scoped.clone());
    let Some(id) = scoped.project() else { return };
    hub::register(
        || {},
        move |changes: &[Change]| {
            let about = changes.iter().any(|c| c.id.kind == Kind::Project && c.id.key == id);
            if about && ProjectStore::new(storage.clone()).get(&id).is_none() {
                scoped.close();
                navigator.replace(MISSING_HREF);
            }
        },
    );
}
