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
pub mod shared;
pub mod shell;
pub mod standards;
pub mod street;
pub mod sync;

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

/// The home page as its script sees it: the list of projects (`project`), and nothing else. It is in no project.
#[wasm_bindgen]
pub struct HomePage(std::rc::Rc<shared::i18n::I18n>);

#[wasm_bindgen]
impl HomePage {
    /// Binds the shell every page shares (settings menu, sign-in menu, language) to this page.
    pub fn mount_shell(&self) {
        shell::mount(self.0.clone(), std::rc::Rc::new(shell::Bare::default()), "details-details", false);
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
    project::mount_list(ports, i18n.clone());
    HomePage(i18n)
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
}
