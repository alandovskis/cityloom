//! The map's details, one level at a time under a path, as the street and junction pages have them: the city (the
//! Checks and Changes tabs, `#map-city`), or the place a press shows (`#map-place`). A first press on a place shows it
//! here; a second press on it, or Open, opens its editor (`MapVm::pick`, `open_picked`).

use std::rc::Rc;

use leptos::prelude::*;

use crate::map::vm::MapVm;
use crate::shared::bind::Bound;

/// The id of the path's current step, which takes the focus after its City step is pressed.
const CURRENT: &str = "map-path-current";

/// Moves the focus to the path's current step once the details are drawn again: from there Tab enters the level.
fn focus_current() {
    #[cfg(target_arch = "wasm32")]
    request_animation_frame(|| {
        use leptos::wasm_bindgen::JsCast;
        if let Some(step) = document().get_element_by_id(CURRENT).and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
            let _ = step.focus();
        }
    });
}

/// The path: City, or City › the place shown. It also shows the level it names and hides the other.
#[component]
pub fn MapPath(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let place = vm.with(|v| v.picked()).is_some();
        let doc = document();
        for (id, hidden) in [("map-city", place), ("map-place", !place)] {
            if let Some(level) = doc.get_element_by_id(id) {
                let _ = if hidden { level.set_attribute("hidden", "") } else { level.remove_attribute("hidden") };
            }
        }
    });
    move || {
        let label = vm.with(|v| v.word("map-path-label"));
        let city = vm.with(|v| v.word("map-path-city"));
        match vm.with(|v| v.picked_row()) {
            None => view! {
                <nav aria-label=label class="panel-path">
                    <ol><li><span id=CURRENT class="path-here" aria-current="location" tabindex="-1">{city}</span></li></ol>
                </nav>
            }
            .into_any(),
            Some(row) => {
                let name = vm.with(|v| v.say(&row.name));
                view! {
                    <nav aria-label=label class="panel-path">
                        <ol>
                            <li>
                                <button type="button" class="path-step" id="map-path-city" on:click=move |_| { vm.with(|v| v.pick(None)); focus_current(); }>
                                    {city}
                                </button>
                            </li>
                            <li><span id=CURRENT class="path-here" aria-current="location" tabindex="-1">{name}</span></li>
                        </ol>
                    </nav>
                }
                .into_any()
            }
        }
    }
}

/// The place a press shows: its name and small print, the checks it fails (or that every one passes), and how to open
/// it or, for a street, add it to the selection.
#[component]
pub fn PlacePane(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    move || {
        let Some(row) = vm.with(|v| v.picked_row()) else { return ().into_any() };
        let street = row.hot.starts_with("s-");
        let uid = row.hot.get(2..).and_then(|u| u.parse::<u32>().ok()).unwrap_or(0);
        let (name, sub) = vm.with(|v| (v.say(&row.name), v.say(&row.sub)));
        let failing = vm.with(|v| v.picked_failing().iter().map(|s| v.say(s)).collect::<Vec<_>>());
        let checks = if failing.is_empty() {
            view! { <p class="insp-range map-place-ok">{vm.with(|v| v.word("map-place-ok"))}</p> }.into_any()
        } else {
            view! {
                <h3 class="note-h">{vm.with(|v| v.word("map-place-failing"))}</h3>
                <ul class="map-place-failing">{failing.into_iter().map(|f| view! { <li>{f}</li> }).collect_view()}</ul>
            }
            .into_any()
        };
        let open = vm.with(|v| v.word(if street { "map-open-street" } else { "map-open-junction" }));
        let select = street.then(|| {
            let hot = row.hot.clone();
            let label = move || vm.with(|v| v.word(if v.selection().contains(uid) { "map-drop-selection" } else { "map-add-selection" }));
            view! {
                <button type="button" class="btn" id="map-place-select" on:click=move |_| vm.with(|v| v.toggle_selected(Some(&hot)))>
                    {label}
                </button>
            }
        });
        view! {
            <div class="insp-head"><div><h2 class="insp-name">{name}</h2><p class="insp-sub">{sub}</p></div></div>
            <section class="insp-sec">{checks}</section>
            <div class="map-place-actions">
                <a class="btn primary" id="map-open-place" href=row.href.clone()>{open}</a>
                {select}
            </div>
        }
        .into_any()
    }
}
