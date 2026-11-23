//! The room an editor's column leaves its drawing (`.tray-page` in `web/editor.css`, the street and junction pages):
//! the column under the title (`.stage-scroll`) less what else the drawing's card holds around the picture. Each page
//! measures its own picture and lays the drawing out to the room (`street::view`, `junction::view::drawing`).

use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{Element, HtmlElement};

/// The column's own padding, top and bottom (`web/editor.css`, `.stage-scroll`: 2 px each, room for the focus outline
/// of what is at its edges).
const COLUMN_PADDING: f64 = 4.0;

/// How high the drawing's picture may be: the `column`'s height less what its `card` holds besides a picture now
/// `picture` high. Where the column does not scroll (under 900 px: the page scrolls), or is not there, nothing limits
/// it: `INFINITY`.
pub fn room_in_column(column: Option<&Element>, card: Option<&Element>, picture: f64) -> f64 {
    match (column, card) {
        (Some(column), Some(card)) if scrolls(column) => column.client_height() as f64 - (height(card) - picture) - COLUMN_PADDING,
        _ => f64::INFINITY,
    }
}

/// The height an element takes in the page, 0 for one that is not an HTML element.
pub fn height(el: &Element) -> f64 {
    el.dyn_ref::<HtmlElement>().map_or(0.0, |h| h.offset_height() as f64)
}

fn scrolls(el: &Element) -> bool {
    leptos::prelude::window()
        .get_computed_style(el)
        .ok()
        .flatten()
        .and_then(|s| s.get_property_value("overflow-y").ok())
        .is_some_and(|o| o == "auto" || o == "scroll")
}
