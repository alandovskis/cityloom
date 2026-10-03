//! The page, drawn from the model by Leptos. Each component reads the model's
//! view and calls the model's edits; the page holds no rules of its own.

pub mod announce;
pub mod inspector;
pub mod keys;
pub mod live;
pub mod notes;
pub mod page;
pub mod plan;
pub mod plan_svg;
pub mod shared;
pub mod turns;
pub mod watch;

#[cfg(test)]
mod tests;

use leptos::prelude::*;
use leptos::web_sys::HtmlElement;
use wasm_bindgen::prelude::*;

/// Draws the junction page into the elements it keeps for them: the plan, its
/// heading and status, undo and redo, the key, the streets and samples to start
/// from, and the notes beside it.
#[wasm_bindgen]
pub fn mount_page(plan: &crate::Plan) {
    // Components create effects as they are built, before any is mounted.
    let _ = any_spawner::Executor::init_wasm_bindgen();
    let at = |id: &str| -> HtmlElement {
        leptos::prelude::document().get_element_by_id(id).unwrap_or_else(|| panic!("the page has no #{id}")).unchecked_into()
    };
    let shared = plan.0.clone();
    let mount = |id: &str, view: AnyView| {
        leptos::mount::mount_to(at(id), move || view).forget();
    };
    mount("scroll", view! { <plan::PlanDrawing shared=shared.clone()/> }.into_any());
    mount("street", view! { <page::Header shared=shared.clone()/> }.into_any());
    mount("title-block", view! { <page::TitleBlock shared=shared.clone()/> }.into_any());
    mount("fit-slot", view! { <page::Fit shared=shared.clone()/> }.into_any());
    mount("history", view! { <page::History shared=shared.clone()/> }.into_any());
    mount("plan-key", view! { <page::Key shared=shared.clone()/> }.into_any());
    mount("palette", view! { <page::Palette shared=shared.clone()/> }.into_any());
    mount("samples", view! { <page::Samples shared=shared.clone()/> }.into_any());
    mount("turns", view! { <turns::Turns shared=shared.clone()/> }.into_any());
    mount("inspector", view! { <inspector::Inspector shared=shared.clone()/> }.into_any());
    mount("across", view! { <notes::Across shared=shared.clone()/> }.into_any());
    mount("conflicts", view! { <notes::Conflicts shared=shared.clone()/> }.into_any());
    mount("conflict-note", view! { <notes::ConflictNote shared=shared.clone()/> }.into_any());
    mount("checks", view! { <notes::Checks shared=shared.clone()/> }.into_any());
    mount("measures", view! { <notes::Measures shared=shared.clone()/> }.into_any());
    mount("revs", view! { <notes::Revisions shared=shared/> }.into_any());
    // Streets that belong to a city are not added, and there is no street page to go to.
    if plan.0.view().linked {
        for selector in [".lower", ".surface[href=\"index.html\"]"] {
            if let Ok(Some(el)) = leptos::prelude::document().query_selector(selector) {
                el.unchecked_into::<HtmlElement>().set_hidden(true);
            }
        }
    }
}

/// Says `text` to a screen reader.
#[wasm_bindgen]
pub fn say(text: &str) {
    live::say(text);
}
