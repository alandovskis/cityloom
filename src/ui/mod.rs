//! The page, drawn from the model by Leptos. Each component reads the model's
//! view and calls the model's edits; the page holds no rules of its own.

pub mod inspector;
pub mod notes;
pub mod shared;
pub mod turns;
pub mod watch;

use leptos::prelude::*;
use leptos::web_sys::HtmlElement;
use wasm_bindgen::prelude::*;

/// Draws the junction's notes into the elements the page has for them: the
/// turn table, how far it is to cross, where paths meet, the checks, and the
/// changes made.
#[wasm_bindgen]
pub fn mount_notes(plan: &crate::Plan) {
    let at = |id: &str| -> HtmlElement {
        leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into()
    };
    let shared = plan.0.clone();
    let mount = |id: &str, view: AnyView| {
        leptos::mount::mount_to(at(id), move || view).forget();
    };
    mount("turns", view! { <turns::Turns shared=shared.clone()/> }.into_any());
    mount("across", view! { <notes::Across shared=shared.clone()/> }.into_any());
    mount("conflicts", view! { <notes::Conflicts shared=shared.clone()/> }.into_any());
    mount("conflict-note", view! { <notes::ConflictNote shared=shared.clone()/> }.into_any());
    mount("checks", view! { <notes::Checks shared=shared.clone()/> }.into_any());
    mount("inspector", view! { <inspector::Inspector shared=shared.clone()/> }.into_any());
    mount("measures", view! { <notes::Measures shared=shared.clone()/> }.into_any());
    mount("revs", view! { <notes::Revisions shared=shared/> }.into_any());
}
