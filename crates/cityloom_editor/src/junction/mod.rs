//! The junction page: a junction of streets edited in 2.5D. Its model, view-model,
//! words and views live together.

pub mod model;
pub mod view;
pub mod vm;

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::HtmlElement;
use wasm_bindgen::prelude::*;

use crate::city::binding::{CityBinding, Place};
use crate::city::store::CityStore;
use crate::shared::i18n::Resources;
use crate::shared::platform::browser_ports;
use crate::shared::ports::Ports;
use crate::shared::project_id::link_for;

/// What the junction says, in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };

/// One junction being edited. The page drives it through the components the
/// module mounts; the script only starts it up and hands it what the shell
/// chooses (units, region).
#[wasm_bindgen]
pub struct Editor(Rc<vm::JunctionVm>);

#[wasm_bindgen]
impl Editor {
    /// Binds the shell every page shares (settings menu, sidebars, notes tabs)
    /// to this page.
    pub fn mount_shell(&self) {
        crate::shell::mount(self.0.i18n().clone(), self.0.clone(), "details-details", crate::shell::NotesAtStart::OpenWhenRoomy);
    }

    /// The page is being left: what is waiting to be kept in the city is kept now.
    pub fn flush(&self) {
        self.0.flush();
    }

    /// The whole drawable state as JSON.
    pub fn view(&self) -> String {
        crate::json(&*self.0.view_now())
    }
}

/// Draws the junction page into the elements it keeps for them: the drawing, its
/// heading and status, undo and redo, the key, and the notes beside it.
#[wasm_bindgen]
pub fn mount_page(editor: &Editor) {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let at =
        |id: &str| -> HtmlElement { leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into() };
    let vm = editor.0.clone();
    let mount = |id: &str, view: AnyView| {
        leptos::mount::mount_to(at(id), move || view).forget();
    };
    mount("scroll", view! { <view::drawing::Drawing vm=vm.clone()/> }.into_any());
    mount("street", view! { <view::page::Header vm=vm.clone()/> }.into_any());
    mount("title-block", view! { <view::page::TitleBlock vm=vm.clone()/> }.into_any());
    mount("fit-slot", view! { <view::page::Fit vm=vm.clone()/> }.into_any());
    mount("history", view! { <view::page::History vm=vm.clone()/> }.into_any());
    mount("scenario-slot", view! { <view::page::Scenarios vm=vm.clone()/> }.into_any());
    mount("plan-key", view! { <view::page::Key vm=vm.clone()/> }.into_any());
    mount("turns", view! { <view::turns::Turns vm=vm.clone()/> }.into_any());
    mount("inspector", view! { <view::inspector::Inspector vm=vm.clone()/> }.into_any());
    mount("across", view! { <view::notes::Across vm=vm.clone()/> }.into_any());
    mount("conflicts", view! { <view::notes::Conflicts vm=vm.clone()/> }.into_any());
    mount("conflict-note", view! { <view::notes::ConflictNote vm=vm.clone()/> }.into_any());
    mount("goal-slot", view! { <view::notes::GoalGuide vm=vm.clone()/> }.into_any());
    mount("checks", view! { <view::notes::Checks vm=vm.clone()/> }.into_any());
    mount("checks-fold-slot", view! { <view::notes::PassingChecks vm=vm.clone()/> }.into_any());
    mount("atlas-source", view! { <view::notes::AtlasSource vm=vm.clone()/> }.into_any());
    mount("revs", view! { <view::notes::Revisions vm=vm/> }.into_any());
}

/// The words of the junction page, and of the page of a junction that cannot be drawn: the stored language, or
/// the browser's, which the document is then told it is in.
fn page_i18n(ports: &crate::shared::ports::Ports) -> Rc<crate::shared::i18n::I18n> {
    crate::i18n_browser(ports)
}

/// Whether the city of the page's project (`project::context`) has junction `node`. Where it has not, the page has
/// gone to that project's map; where the project is not here, to the list.
#[wasm_bindgen]
pub fn enter_junction(node: u32) -> bool {
    enter_junction_in(browser_ports(), node)
}

/// `enter_junction` from the page's own ports (root storage).
pub fn enter_junction_in(ports: Ports, node: u32) -> bool {
    let Some(ports) = crate::project::context::enter(ports) else { return false };
    // No junction is numbered 0: the page's script gives 0 for an address with no junction.
    let here = node != 0 && CityStore::current(ports.storage.clone()).open().junction_name(node).is_some();
    if !here {
        ports.navigator.replace(&link_for("map.html", &*ports.storage));
    }
    here
}

/// The page of junction `node` when the streets that meet there can no longer be drawn as a junction: it says
/// so under the junction's name, and keeps the shell every page shares (the settings menu and the sidebars).
#[wasm_bindgen]
pub fn mount_bare_shell(node: u32) {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    // `open_junction` already watches for the project's removal.
    let Some(ports) = crate::project::context::enter(browser_ports()) else { return };
    let project = ports.storage.project();
    let map = link_for("map.html", &*ports.storage);
    let i18n = page_i18n(&ports);
    let name = CityStore::current(ports.storage.clone()).open().junction_name(node);
    let doc = leptos::prelude::document();
    let find = |selector: &str| doc.query_selector(selector).ok().flatten();
    // Nothing here can be drawn or changed: the drawing, its tools and its status make way for the panel.
    if let Some(tools) = find(".tools") {
        let _ = tools.set_attribute("hidden", "");
    }
    for selector in [".plan-drawing", ".statusbar"] {
        if let Some(el) = find(selector) {
            el.remove();
        }
    }
    if let (Some(at), Some(name)) = (find("#street"), name) {
        let i18n = i18n.clone();
        leptos::mount::mount_to(at.unchecked_into(), move || view! { <view::page::StuckHeader i18n=i18n name=name/> }).forget();
    }
    if let Some(at) = find(".stage-main") {
        let i18n = i18n.clone();
        leptos::mount::mount_to(at.unchecked_into(), move || view! { <view::page::Stuck i18n=i18n map=map/> }).forget();
    }
    crate::shell::mount(i18n, Rc::new(crate::shell::Bare::in_project(project)), "details-details", crate::shell::NotesAtStart::OpenWhenRoomy);
}

/// The junction editor on one junction of the city of the page's project, which writes what is made back to the
/// city. Nothing when there is no such junction or it cannot be drawn with the streets as they now are, or when the
/// project is not here (the page has then gone to the list).
#[wasm_bindgen]
pub fn open_junction(node: u32) -> Option<Editor> {
    open_junction_in(browser_ports(), node)
}

/// `open_junction` from the page's own ports (root storage).
pub fn open_junction_in(ports: Ports, node: u32) -> Option<Editor> {
    let ports = crate::project::context::open_page(ports)?;
    let store = CityStore::current(ports.storage.clone());
    let junction = store.open().junction_editor(node, store.region())?;
    let place = Place::Junction(node);
    let i18n = page_i18n(&ports);
    let vm = vm::JunctionVm::new(ports, i18n, junction, Some(CityBinding { store, place }));
    vm.watch_sync();
    Some(Editor(vm))
}

/// The junction comparison page's junction half (`junction/view/compare.rs`), mounted into `#compare-slot`: the
/// designs of junction `node` of the city of the page's project, side by side and read-only. `ports` are already
/// scoped to the project (`project::context::peek`); nothing is written. Gives the view-model, for the shell to tell it
/// the units chosen; none when the junction cannot be opened (not a junction of the city, or not drawable with its
/// streets as they now are), and nothing is mounted. The
/// caller (`lib.rs::mount_compare`) runs `any_spawner::Executor::init_wasm_bindgen()` first, as every `mount_*` must.
/// No standards profile is activated: the junction page never does.
pub fn mount_compare_junction(ports: Ports, i18n: Rc<crate::shared::i18n::I18n>, node: u32) -> Option<Rc<dyn crate::shared::compare::CompareHost>> {
    let binding = CityBinding { store: CityStore::current(ports.storage.clone()), place: Place::Junction(node) };
    let vm = vm::compare::JunctionCompareVm::new(ports, i18n, binding)?;
    let slot = leptos::prelude::document().get_element_by_id("compare-slot")?;
    let host: Rc<dyn crate::shared::compare::CompareHost> = vm.clone();
    let slot: HtmlElement = slot.unchecked_into();
    leptos::mount::mount_to(slot, move || view! { <view::compare::JunctionCompare vm=vm/> }).forget();
    Some(host)
}
