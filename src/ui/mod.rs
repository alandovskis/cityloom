//! The page, drawn from the model by Leptos. Each component reads the model's
//! view and calls the model's edits; the page holds no rules of its own.

pub mod turns;

use leptos::web_sys::HtmlElement;
use wasm_bindgen::prelude::*;

/// Draws the junction's turn table into `el`, a `<table>`.
#[wasm_bindgen]
pub fn mount_turns(el: HtmlElement) {
    leptos::mount::mount_to(el, turns::Turns).forget();
}
