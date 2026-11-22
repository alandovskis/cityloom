//! CityLoom cross-section editor core, compiled to WebAssembly.
//!
//! The model owns every editing rule. The page only draws the view it
//! returns and relays pointer and keyboard input.

#![recursion_limit = "1024"]

pub mod account;
pub mod archive;
pub mod city;
pub mod junction;
pub mod map;
pub mod place;
pub mod project;
pub mod restriction;
pub mod share;
pub mod shared;
pub mod shell;
pub mod standards;
pub mod street;
pub mod sync;
pub mod timeline;

use shared::{atlas, symbols};
use wasm_bindgen::prelude::*;

use shared::catalogue::{CURBS, DIRECTIONS, KINDS, MATERIALS, REGIONS};

pub(crate) fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("view serialises")
}

/// The segment catalogue as JSON.
#[wasm_bindgen]
pub fn catalogue() -> String {
    json(&KINDS)
}

/// The surface and curb material tables as JSON.
#[wasm_bindgen]
pub fn materials() -> String {
    json(&serde_json::json!({ "surfaces": MATERIALS, "curbs": CURBS, "directions": DIRECTIONS, "regions": REGIONS }))
}

/// The hatch patterns of the pieces, surfaces and curbs, as JSON: `{HATCH, MATERIAL_HATCH, CURB_HATCH}`,
/// each from a name to its SVG `<pattern>`.
#[wasm_bindgen]
pub fn hatches() -> String {
    symbols::hatches_json()
}

/// The Transit Priority Atlas toolbox measures and where each is modelled, as JSON.
#[wasm_bindgen]
pub fn atlas() -> String {
    json(&atlas::MEASURES)
}

/// Says `text` to a screen reader.
#[wasm_bindgen]
pub fn say(text: &str) {
    shared::live::say(text);
}

/// Draws the Profiles section into the street page's `#standards-slot`: the browser's standards profiles, to create,
/// edit, choose, export, import and remove. The street's checks are drawn again whenever one changes. The street page's
/// script calls it after `mount_street_page`.
#[wasm_bindgen]
pub fn mount_standards(sheet: &street::Sheet) {
    use wasm_bindgen::JsCast;
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let Some(slot) = leptos::prelude::document().get_element_by_id("standards-slot").and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok()) else {
        return;
    };
    let vm = sheet.view_model();
    let (for_units, for_change) = (vm.clone(), vm.clone());
    standards::mount_profiles(
        slot,
        shared::platform::browser_ports(),
        vm.i18n().clone(),
        std::rc::Rc::new(move || for_units.units()),
        std::rc::Rc::new(move || {
            use leptos::prelude::Update;
            for_change.version().update(|v| *v = v.wrapping_add(1));
        }),
    );
}

/// The home page as its script sees it: the list of projects (`project`), and nothing else. It is in no project.
#[wasm_bindgen]
pub struct HomePage(std::rc::Rc<shared::i18n::I18n>);

#[wasm_bindgen]
impl HomePage {
    /// Binds the shell every page shares (settings menu, sign-in menu, language) to this page.
    pub fn mount_shell(&self) {
        shell::mount(self.0.clone(), std::rc::Rc::new(shell::Bare::default()), "details-details", shell::NotesAtStart::Open);
    }
}

/// Draws the home page, the list of projects, which first moves the work kept from before projects into the first
/// project, in one set of words the shell switches.
#[wasm_bindgen]
pub fn mount_projects() -> HomePage {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let ports = shared::platform::browser_ports();
    let i18n = i18n_browser(&ports);
    // A project's share links made here end before it does (#221): the share slice keeps them.
    let links = std::rc::Rc::new(share::links::ProjectShares::new(ports.clone()));
    project::mount_list(ports, i18n.clone(), links);
    HomePage(i18n)
}

/// The timeline page as its script sees it: the milestones of the project it is opened in (`timeline`).
#[wasm_bindgen]
pub struct TimelinePage {
    i18n: std::rc::Rc<shared::i18n::I18n>,
    project: Option<String>,
}

#[wasm_bindgen]
impl TimelinePage {
    /// Binds the shell every page shares (settings menu, sign-in menu, language) to this page, in its project.
    pub fn mount_shell(&self) {
        shell::mount(self.i18n.clone(), std::rc::Rc::new(shell::Bare::in_project(self.project.clone())), "details-details", shell::NotesAtStart::Open);
    }
}

/// Draws the timeline of the project the page is opened in (`project::context::open_page`). The page is put together
/// here, where slices may meet: the project slice says which project and what it is called, the timeline slice draws
/// its milestones. Where the project is not here the page has gone to the list, and there is nothing to draw.
#[wasm_bindgen]
pub fn mount_timeline() -> Option<TimelinePage> {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let ports = project::context::open_page(shared::platform::browser_ports())?;
    let i18n = i18n_browser(&ports);
    let project = ports.storage.project();
    let name = project.as_deref().and_then(|id| project::context::find(&*ports.storage, id)).map(|p| p.name).unwrap_or_default();
    let context = timeline::vm::Context { project_name: name, app_version: env!("CARGO_PKG_VERSION").to_string() };
    timeline::mount_timeline(ports, i18n.clone(), context);
    Some(TimelinePage { i18n, project })
}

/// The comparison page as its script sees it: two designs of one place of the project it is opened in, side by side.
#[wasm_bindgen]
pub struct ComparePage {
    i18n: std::rc::Rc<shared::i18n::I18n>,
    project: Option<String>,
    host: std::rc::Rc<dyn shared::compare::CompareHost>,
}

#[wasm_bindgen]
impl ComparePage {
    /// Binds the shell every page shares (settings menu, sign-in menu, language) to this page, in its project: the
    /// units chosen go to the comparison. The shell says by itself that the page is read-only when its address asks
    /// (`view=1`).
    pub fn mount_shell(&self) {
        shell::mount(
            self.i18n.clone(),
            std::rc::Rc::new(CompareTarget::new(self.project.clone(), self.host.clone())),
            "details-details",
            shell::NotesAtStart::Open,
        );
    }
}

/// The shell's target on the comparison page: the units go to the comparison, which draws and says its lengths in
/// them; the region is only kept, as on a bare page, since the designs keep their own drive side. The comparison
/// starts in the units the shell starts in (`Units::default()`, as every page: the choice is not remembered).
struct CompareTarget {
    bare: shell::Bare,
    host: std::rc::Rc<dyn shared::compare::CompareHost>,
}

impl CompareTarget {
    fn new(project: Option<String>, host: std::rc::Rc<dyn shared::compare::CompareHost>) -> CompareTarget {
        CompareTarget { bare: shell::Bare::in_project(project), host }
    }
}

impl shell::Target for CompareTarget {
    fn set_units(&self, units: shared::units::Units) {
        self.host.set_units(units);
    }

    fn apply_region(&self, region: usize) -> bool {
        self.bare.apply_region(region)
    }

    fn region_id(&self) -> String {
        self.bare.region_id()
    }

    fn project(&self) -> Option<String> {
        self.bare.project()
    }
}

/// Draws the comparison of two designs of the street (`?street=<edge>`) or the junction (`?junction=<node>`) the page
/// is opened for, in the project it is opened in. Read-only: the project is only peeked at (`project::context::peek`),
/// nothing is remembered or written. The page is put together here, where slices may meet: the project slice says which
/// project, the street or junction slice draws its half. Where the project is not here the page goes to the list, and
/// where the place is not one of the city's, to the project's map; there is nothing to draw in either case.
#[wasm_bindgen]
pub fn mount_compare() -> Option<ComparePage> {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let root = shared::platform::browser_ports();
    let Some(ports) = project::context::peek(root.clone()) else {
        root.navigator.replace(project::context::MISSING_HREF);
        return None;
    };
    let path = ports.page.path();
    let place = |name: &str| compared_place(&path, name);
    let i18n = i18n_browser(&ports);
    let mounted = match (place("street"), place("junction")) {
        (Some(edge), _) => street::mount_compare_street(ports.clone(), i18n.clone(), edge),
        (None, Some(node)) => junction::mount_compare_junction(ports.clone(), i18n.clone(), node),
        (None, None) => None,
    };
    let Some(host) = mounted else {
        ports.navigator.replace(&shared::project_id::link_for("map.html", &*ports.storage));
        return None;
    };
    Some(ComparePage { i18n, project: ports.storage.project(), host })
}

/// The place the comparison page's address `path` names by `name` (`street` or `junction`): a number other than 0.
fn compared_place(path: &str, name: &str) -> Option<u32> {
    shared::compare::params(path).find(|(key, _)| *key == name).and_then(|(_, value)| value.parse::<u32>().ok()).filter(|&n| n != 0)
}

/// The share page as its script sees it (#221): the links of the project it is opened in, or a link being opened.
#[wasm_bindgen]
pub struct SharePage {
    i18n: std::rc::Rc<shared::i18n::I18n>,
    project: Option<String>,
}

#[wasm_bindgen]
impl SharePage {
    /// Binds the shell every page shares (settings menu, sign-in menu, language) to this page.
    pub fn mount_shell(&self) {
        shell::mount(self.i18n.clone(), std::rc::Rc::new(shell::Bare::in_project(self.project.clone())), "details-details", shell::NotesAtStart::Open);
    }
}

/// Draws the share page. Its address carries either a share link's token (`share.html#<token>`, or a fragment where
/// no project is named: `share::model::opens_a_link`): the page is in no project, moves the work from before projects first as such a page does, and opens the link (`share::mount_open`);
/// or a project (`share.html?project=<id>`, `project::context::open_page`): the page makes and lists that project's
/// links (`share::mount_share`), and where the project is not here it has gone to the list and draws nothing. Put
/// together here, where slices may meet: the project slice says which project, what it is called and where it is.
#[wasm_bindgen]
pub fn mount_share() -> Option<SharePage> {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let root = shared::platform::browser_ports();
    let search = root.page.path().split_once('?').map(|(_, search)| search.to_string()).unwrap_or_default();
    if share::model::opens_a_link(&root.page.hash(), &search) {
        let i18n = i18n_browser(&root);
        project::migrate::move_legacy_work(&root.storage, &i18n);
        share::mount_open(root, i18n.clone());
        return Some(SharePage { i18n, project: None });
    }
    let ports = project::context::open_page(root)?;
    let i18n = i18n_browser(&ports);
    let project = ports.storage.project();
    let record = project.as_deref().and_then(|id| project::context::find(&*ports.storage, id));
    let context = share::vm::Context {
        project_name: record.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        project_area: record.map(|p| p.area).unwrap_or_else(|| place::area::default_area().to_query()),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    share::mount_share(ports, i18n.clone(), context);
    Some(SharePage { i18n, project })
}

/// The About page as its script sees it: what CityLoom is for, with the city of the project opened last drawn behind
/// it (the map's About part). The page is put together here, where slices may meet: the map slice draws it, and the
/// project slice moves the work from before projects first.
#[wasm_bindgen]
pub struct AboutPage(map::MapPage);

#[wasm_bindgen]
impl AboutPage {
    /// Binds the shell every page shares (settings menu, sign-in menu, language) to this page.
    pub fn mount_shell(&self) {
        self.0.mount_shell();
    }

    /// The city behind the About card, once its roads are got (`prepare_city`).
    pub fn mount_city(&self) {
        self.0.mount_city();
    }
}

/// Draws the About page: its window title now, the city when `AboutPage::mount_city` is called. Its shell syncs as soon
/// as it knows who is signed in, as the home page's does, so the work from before projects is moved into the first
/// project before anything is read: the city drawn is that project's when it is the one opened last.
#[wasm_bindgen]
pub fn mount_about() -> AboutPage {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let ports = shared::platform::browser_ports();
    let i18n = i18n_browser(&ports);
    project::migrate::move_legacy_work(&ports.storage, &i18n);
    AboutPage(map::mount_about(i18n))
}

/// Every slice's messages, in one place: a slice's `.ftl` files are registered here.
const RESOURCES: &[shared::i18n::Resources] = &[
    shell::RESOURCES,
    account::RESOURCES,
    street::RESOURCES,
    city::RESOURCES,
    map::RESOURCES,
    place::RESOURCES,
    junction::RESOURCES,
    restriction::RESOURCES,
    sync::RESOURCES,
    archive::RESOURCES,
    project::RESOURCES,
    timeline::RESOURCES,
    share::RESOURCES,
    standards::RESOURCES,
];

/// The words of the app in `locale`.
pub fn i18n_for(locale: shared::i18n::Locale) -> std::rc::Rc<shared::i18n::I18n> {
    shared::i18n::I18n::new(locale, RESOURCES)
}

/// The words of the app in the language a page starts in, which the document then says it is in.
pub fn i18n_browser(ports: &shared::ports::Ports) -> std::rc::Rc<shared::i18n::I18n> {
    let locale = shared::i18n::detect(ports);
    ports.page.set_lang(locale.tag());
    i18n_for(locale)
}

#[cfg(test)]
mod i18n_guard;

#[cfg(test)]
mod layout_guard;

#[cfg(test)]
mod atlas_contract;

#[cfg(test)]
mod tests {
    use super::*;
    use shared::i18n::Locale;
    use shared::ports::test_ports_with_page;

    #[test]
    fn the_page_is_told_the_language_it_starts_in() {
        let (ports, page, _) = test_ports_with_page();
        assert_eq!(i18n_browser(&ports).locale_now(), Locale::En);
        assert_eq!(*page.lang.borrow(), "en");
        *page.languages.borrow_mut() = vec!["fr".into()];
        assert_eq!(i18n_browser(&ports).locale_now(), Locale::FrCa);
        assert_eq!(*page.lang.borrow(), "fr-CA");
    }

    #[test]
    fn the_compared_place_is_read_from_the_query_alone() {
        assert_eq!(compared_place("/compare.html?street=7&b=2", "street"), Some(7));
        assert_eq!(compared_place("/compare.html?project=p1&junction=12", "junction"), Some(12));
        assert_eq!(compared_place("/compare.html?street=7#top", "street"), Some(7));
        assert_eq!(compared_place("/compare.html?street=0", "street"), None);
        assert_eq!(compared_place("/compare.html?streets=7", "street"), None);
        assert_eq!(compared_place("/compare.html", "street"), None);
    }

    #[test]
    fn the_comparison_shows_lengths_in_the_units_the_shell_chooses() {
        use shared::compare::CompareHost;
        use shared::units::Units;
        use shell::Target;
        let (mut ports, _, storage, _) = shared::ports::test_ports_with_time();
        let page = std::rc::Rc::new(shared::ports::FakePage::default());
        ports.page = page.clone();
        let own: std::rc::Rc<dyn shared::ports::Storage> = std::rc::Rc::new(shared::scoped::ScopedStorage::new(storage, "p1"));
        let store = city::store::CityStore::sample(own.clone());
        let edge = store.open().view(0).edges[0].uid;
        *page.path.borrow_mut() = format!("/compare.html?street={edge}");
        ports.storage = own;
        let binding = city::binding::CityBinding { store, place: city::binding::Place::Street(edge) };
        let vm = street::vm::compare::StreetCompareVm::new(ports, i18n_for(Locale::En), binding).unwrap();
        let target = CompareTarget::new(Some("p1".into()), vm.clone());
        assert_eq!(vm.units(), Units::Metres, "the shell starts in metres, as every page does");
        target.set_units(Units::Feet);
        assert_eq!(vm.units(), Units::Feet);
        assert_eq!(target.project().as_deref(), Some("p1"));
        assert!(target.apply_region(0));
        assert_eq!(target.region_id(), shared::catalogue::REGIONS[0].id);
    }
}
