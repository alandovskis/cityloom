//! The map's details, one level at a time under a path, as the street and junction pages have them: the city (the
//! Checks and Changes tabs, `#map-city`), the place a press shows (`#map-place`), the area rules (`#map-areas`) or a
//! network check (`#map-check`, `network_view::NetworkCheckPane`) or the selected streets (`#map-selection`). A
//! first press on a place shows it here; a second press on it, or Open, opens its editor (`MapVm::pick`,
//! `open_picked`). The city leads to the area rules by a row of its own (`AreasRow`).

use std::rc::Rc;

use leptos::prelude::*;

use crate::map::vm::{MapLevel, MapVm};
use crate::shared::bind::Bound;

/// The id of the path's current step, which takes the focus after its City step is pressed.
const CURRENT: &str = "map-path-current";

/// Moves the focus to the path's current step once the details are drawn again: from there Tab enters the level.
pub(crate) fn focus_current() {
    #[cfg(target_arch = "wasm32")]
    request_animation_frame(|| {
        use leptos::wasm_bindgen::JsCast;
        if let Some(step) = document().get_element_by_id(CURRENT).and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
            let _ = step.focus();
        }
    });
}

/// The page's element of each level, and whether it is the level `level`.
fn levels(level: &MapLevel) -> [(&'static str, bool); 5] {
    [
        ("map-city", *level == MapLevel::City),
        ("map-place", matches!(level, MapLevel::Place(_))),
        ("map-areas", *level == MapLevel::Areas),
        ("map-check", matches!(level, MapLevel::Check(_))),
        ("map-selection", *level == MapLevel::Selection),
    ]
}

/// The path: City, or City › the place shown, or City › Area rules. It also shows the level it names and hides the
/// others. While an outline is drawn or named the city is not a step back: Finish, Save or Cancel come first.
#[component]
pub fn MapPath(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let level = vm.with(|v| v.level());
        let doc = document();
        for (id, shown) in levels(&level) {
            if let Some(el) = doc.get_element_by_id(id) {
                let _ = if shown { el.remove_attribute("hidden") } else { el.set_attribute("hidden", "") };
            }
        }
    });
    #[cfg(not(target_arch = "wasm32"))]
    let _ = levels;
    move || {
        let label = vm.with(|v| v.word("map-path-label"));
        let city = vm.with(|v| v.word("map-path-city"));
        let here = match vm.with(|v| v.level()) {
            MapLevel::City => None,
            MapLevel::Place(_) => vm.with(|v| v.picked_row().map(|row| v.say(&row.name))),
            MapLevel::Areas => Some(vm.with(|v| v.word("restriction-panel"))),
            MapLevel::Check(_) => vm.with(|v| v.shown_check().map(|c| c.label)),
            MapLevel::Selection => Some(vm.with(|v| v.word("sel-heading"))),
        };
        match here {
            None => view! {
                <nav aria-label=label class="panel-path">
                    <ol><li><span id=CURRENT class="path-here" aria-current="location" tabindex="-1">{city}</span></li></ol>
                </nav>
            }
            .into_any(),
            Some(here) => {
                let back = if vm.with(|v| v.can_leave_areas()) {
                    view! {
                        <button type="button" class="path-step" id="map-path-city" on:click=move |_| { vm.with(|v| v.show_city()); focus_current(); }>
                            {city}
                        </button>
                    }
                    .into_any()
                } else {
                    view! { <span class="path-still">{city}</span> }.into_any()
                };
                view! {
                    <nav aria-label=label class="panel-path">
                        <ol>
                            <li>{back}</li>
                            <li><span id=CURRENT class="path-here" aria-current="location" tabindex="-1">{here}</span></li>
                        </ol>
                    </nav>
                }
                .into_any()
            }
        }
    }
}

/// The city's row to the selected streets: how many there are.
#[component]
pub fn SelectionRow(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    view! {
        <ul class="places-list map-go">
            <li>
                <button type="button" class="place-row" id="map-go-selection" on:click=move |_| { vm.with(|v| v.show_selected_streets()); focus_current(); }>
                    <b>{move || vm.with(|v| v.word("sel-heading"))}</b>
                    <small>{move || vm.with(|v| v.selection_summary())}</small>
                </button>
            </li>
        </ul>
    }
}

/// The city's row to the area rules: how many there are, and how many streets break them.
#[component]
pub fn AreasRow(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    view! {
        <ul class="places-list map-go">
            <li>
                <button type="button" class="place-row" id="map-go-areas" on:click=move |_| { vm.with(|v| v.show_areas()); focus_current(); }>
                    <b>{move || vm.with(|v| v.word("restriction-panel"))}</b>
                    <small>{move || vm.with(|v| v.areas_summary())}</small>
                </button>
            </li>
        </ul>
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
