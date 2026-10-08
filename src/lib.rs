//! CityLoom cross-section editor core, compiled to WebAssembly.
//!
//! The model owns every editing rule. The page only draws the view it
//! returns and relays pointer and keyboard input.

#![recursion_limit = "1024"]

pub mod city;
pub mod junction;
pub mod map;
pub mod place;
pub mod shared;
pub mod shell;
pub mod street;

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

/// Every slice's messages, in one place: a slice's `.ftl` files are registered here.
const RESOURCES: &[shared::i18n::Resources] = &[shell::RESOURCES, street::RESOURCES];

/// The words of the app in `locale`.
pub fn i18n_for(locale: shared::i18n::Locale) -> std::rc::Rc<shared::i18n::I18n> {
    shared::i18n::I18n::new(locale, RESOURCES)
}

/// The words of the app in the language the page starts in, which the document then says it is in.
pub fn i18n_browser(ports: &shared::ports::Ports) -> std::rc::Rc<shared::i18n::I18n> {
    let locale = shared::i18n::detect(ports);
    ports.page.set_lang(locale.tag());
    i18n_for(locale)
}

#[cfg(test)]
mod i18n_guard;

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
        *page.language.borrow_mut() = Some("fr".into());
        assert_eq!(i18n_browser(&ports).locale_now(), Locale::FrCa);
        assert_eq!(*page.lang.borrow(), "fr-CA");
    }
}
