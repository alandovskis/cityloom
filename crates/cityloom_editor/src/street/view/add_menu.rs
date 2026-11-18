//! The street page's Add a piece menu: a search box over every item, and under it the eight topics beside the chosen
//! topic's items (one row of tabs across the top on a narrow screen). Which topic it opens on, what each topic holds,
//! what matches and what is greyed with which reason all come from the view-model (`street::vm::add_menu`), and where
//! a key moves the focus from `keys::add_menu_key`; this binds them.

use std::rc::Rc;

use leptos::either::Either;
use leptos::prelude::*;

use super::page::{MenuParts, say};
use crate::shared::catalogue::KINDS;
use crate::shared::i18n::Args;
use crate::shared::said::{self, Said};
use crate::shared::symbols::icon;
use crate::street::vm::StreetVm;
use crate::street::vm::add_menu::{Entry, Item, Run, RunView, Topic, runs};
use crate::street::vm::keys::{AddFocus, add_menu_key};
use crate::street::vm::watch::SheetWatch;

/// The button that opens the menu, and the menu: a disclosure (not a WAI-ARIA menu, since it holds a search box and
/// tabs). It opens on the topic of what is selected with the focus in the search box; the arrows move through the
/// topics (showing each) and through the items, ArrowDown in the search box goes to the first item shown, Escape or
/// leaving it closes it.
#[component]
pub fn AddMenu(vm: Rc<StreetVm>) -> impl IntoView {
    use leptos::wasm_bindgen::JsCast;
    use leptos::web_sys::{FocusEvent, HtmlElement, KeyboardEvent};

    let w = SheetWatch::new(vm);
    let parts = MenuParts::new(".add-item");
    let MenuParts { open, button, menu, .. } = parts;
    let topic = RwSignal::new(w.add_menu_opening());
    let query = RwSignal::new(String::new());
    let narrow = RwSignal::new(is_narrow());
    let search = NodeRef::<leptos::html::Input>::new();
    // The widget is the menu and its button.
    let inside = move |t: &leptos::web_sys::Element| {
        menu.get_untracked().is_some_and(|m| m.contains(Some(t))) || button.get_untracked().is_some_and(|b| b.contains(Some(t)))
    };
    parts.listen("#add-search", inside);
    #[cfg(target_arch = "wasm32")]
    window_event_listener(leptos::ev::resize, move |_| narrow.set(is_narrow()));
    // Each opening starts again: the topic of what is selected, an empty search.
    let open_menu = move || {
        topic.set(w.add_menu_opening());
        query.set(String::new());
        open.set(true);
    };
    let focus_out = move |e: FocusEvent| {
        if MenuParts::left_by(&e, inside) {
            open.set(false);
        }
    };
    let keys = move |e: KeyboardEvent| {
        let Some(m) = menu.get_untracked() else { return };
        let (tabs, items) = (all_in(&m, ".add-topic"), parts.items());
        let active = document().active_element();
        let is = |el: &HtmlElement| active.as_ref() == Some(el.unchecked_ref::<leptos::web_sys::Element>());
        let at = if search.get_untracked().is_some_and(|s| is(&s)) {
            AddFocus::Search
        } else if let Some(i) = tabs.iter().position(is) {
            AddFocus::Topic(i)
        } else if let Some(i) = items.iter().position(is) {
            AddFocus::Item(i)
        } else {
            return;
        };
        let Some(to) = add_menu_key(&e.key(), at, tabs.len(), items.len()) else { return };
        e.prevent_default();
        let next = match to {
            AddFocus::Search => search.get_untracked().map(HtmlElement::from),
            AddFocus::Topic(i) => {
                topic.set(Topic::ALL[i]);
                tabs.get(i).cloned()
            }
            AddFocus::Item(i) => items.get(i).cloned(),
        };
        if let Some(el) = next {
            let _ = el.focus();
        }
    };
    let typed = move |e: leptos::ev::Event| {
        let q = event_target_value(&e);
        query.set(q.clone());
        if !q.trim().is_empty() {
            w.say_matches(w.add_search(&q).len());
        }
    };
    // The search's results replace the topics while the box holds text; clearing it shows the topic again.
    let searching = Memo::new(move |_| !query.get().trim().is_empty());
    // The chosen topic's tab is brought into the row when the row scrolls sideways (a narrow screen): on opening, on
    // a topic chosen, and when clearing the search shows the topics again. Its own scroll, never the page's.
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let (shown, t) = (open.get() && !searching.get(), topic.get());
        if !shown {
            return;
        }
        // Once the menu is shown, so that the tab has a place to measure.
        leptos::task::spawn_local(async move {
            leptos::task::tick().await;
            let Some(m) = menu.get_untracked() else { return };
            let (Ok(Some(row)), Ok(Some(tab))) = (m.query_selector(".add-topics"), m.query_selector(&format!("#add-topic-{}", t.id()))) else { return };
            let (c, s) = (tab.get_bounding_client_rect(), row.get_bounding_client_rect());
            let by = crate::street::view::strip::scroll_into_view_px(c.left(), c.right(), s.left(), s.right());
            if by != 0.0 {
                row.set_scroll_left(row.scroll_left() + by.round() as i32);
            }
        });
    });
    let body = move || {
        if searching.get() { Either::Left(move || results(w, parts, query.get())) } else { Either::Right(topics(w, parts, topic, narrow)) }
    };
    view! {
        <button
            type="button"
            id="add-btn"
            class="btn"
            aria-expanded=move || open.get().to_string()
            aria-controls="add-menu"
            node_ref=button
            on:click=move |_| if open.get_untracked() { open.set(false) } else { open_menu() }
            on:keydown=move |e: KeyboardEvent| {
                if e.key() == "ArrowDown" && !open.get_untracked() {
                    e.prevent_default();
                    open_menu();
                }
            }
        >
            <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M3 8h10M8 3v10"/></svg>
            {say(w, "add-button")}
        </button>
        <div id="add-menu" class="menu add-menu" hidden=move || !open.get() node_ref=menu on:keydown=keys on:focusout=focus_out>
            <p class="menu-h">{say(w, "add-button")}</p>
            <label class="sr-only" for="add-search">{say(w, "add-search-label")}</label>
            <input
                type="search"
                id="add-search"
                class="add-search"
                autocomplete="off"
                spellcheck="false"
                placeholder=say(w, "add-search-label")
                node_ref=search
                prop:value=move || query.get()
                on:input=typed
            />
            <div class="add-body">{body}</div>
        </div>
    }
}

/// Whether the screen is as narrow as the one where `web/editor.css` lays the topics across the top.
fn is_narrow() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        window().match_media("(max-width: 560px)").ok().flatten().is_some_and(|m| m.matches())
    }
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// The elements under `root` that `selector` picks, in order.
fn all_in(root: &leptos::web_sys::Element, selector: &str) -> Vec<leptos::web_sys::HtmlElement> {
    use leptos::wasm_bindgen::JsCast;
    let Ok(list) = root.query_selector_all(selector) else { return Vec::new() };
    (0..list.length()).filter_map(|i| list.item(i)?.dyn_into().ok()).collect()
}

/// An item the street can take is added and the menu closes, the focus back on its button; a greyed one adds nothing,
/// says why, and the menu stays open.
fn choose(w: SheetWatch, parts: MenuParts, item: Item, greyed: bool) {
    if !greyed {
        parts.close(true);
    }
    w.add_item(item);
}

/// The topics, a tablist (vertical beside the items, horizontal across the top on a narrow screen), and the chosen
/// topic's items in its tabpanel. The chosen tab is the only one in the tab order.
fn topics(w: SheetWatch, parts: MenuParts, topic: RwSignal<Topic>, narrow: RwSignal<bool>) -> impl IntoView {
    let tabs = Topic::ALL
        .into_iter()
        .map(|t| {
            view! {
                <button
                    type="button"
                    role="tab"
                    class="add-topic"
                    id=format!("add-topic-{}", t.id())
                    aria-selected=move || (topic.get() == t).to_string()
                    tabindex=move || if topic.get() == t { "0" } else { "-1" }
                    aria-controls="add-panel"
                    on:click=move |_| topic.set(t)
                >
                    {move || w.i18n().tr(&t.name_key(), &Args::new())}
                </button>
            }
        })
        .collect_view();
    view! {
        <div
            class="add-topics"
            role="tablist"
            aria-orientation=move || if narrow.get() { "horizontal" } else { "vertical" }
            aria-label=say(w, "add-topics-label")
        >
            {tabs}
        </div>
        <div id="add-panel" class="add-panel" role="tabpanel" aria-labelledby=move || format!("add-topic-{}", topic.get().id())>
            {move || panel(w, parts, w.add_entries(topic.get()))}
        </div>
    }
}

/// A topic's items in runs (its pieces, then its green elements, stops or traffic calming elements), each under the
/// hint that says where its items go and, when the street cannot take them, over the reason that describes each one.
fn panel(w: SheetWatch, parts: MenuParts, entries: Vec<Entry>) -> impl IntoView {
    runs(&entries)
        .into_iter()
        .map(|RunView { run: r, entries, unavailable: reason }| {
            let reason_id = format!("add-reason-{}", r.id());
            let described = reason.as_ref().map(|_| reason_id.clone());
            let items = entries.into_iter().map(|e| item(w, parts, e, described.clone(), None, false)).collect_view();
            view! {
                <p class="hint" id=format!("add-hint-{}", r.id())>{say(w, r.hint_key())}</p>
                <ul class="add-sub">{items}</ul>
                {reason.map(|s| reason_line(w, reason_id, s))}
            }
        })
        .collect_view()
}

/// Every item whose name holds `query`, in topic order, each with its topic's name. Where an item goes is not shown
/// (one hint over a list mixing pieces and stops would be wrong for half of it): it describes each item instead, from
/// a hidden line per kind of item.
fn results(w: SheetWatch, parts: MenuParts, query: String) -> impl IntoView {
    let found = w.add_search(&query);
    if found.is_empty() {
        return Either::Left(view! { <p class="add-none">{move || w.i18n().tr("add-no-match", &Args::new().str("query", query.clone()))}</p> });
    }
    let places: Vec<Run> = runs(&found).into_iter().map(|r| r.run).collect();
    let items = found
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            let place = format!("add-place-{}", e.item.run().id());
            let reason = e.unavailable.clone().map(|s| (format!("add-reason-r{i}"), s));
            let described = reason.as_ref().map_or_else(|| place.clone(), |(id, _)| format!("{place} {id}"));
            item(w, parts, e, Some(described), reason, true)
        })
        .collect_view();
    let places = places.into_iter().map(|r| view! { <p id=format!("add-place-{}", r.id()) hidden>{say(w, r.hint_key())}</p> }).collect_view();
    Either::Right(view! {
        <ul id="add-results" class="add-sub">{items}</ul>
        {places}
    })
}

/// Why the street cannot take a run of items, or one item, as text the greyed items point at.
fn reason_line(w: SheetWatch, id: String, reason: Said) -> impl IntoView {
    view! { <p class="add-reason" id=id>{move || said::say(&w.i18n(), w.units(), &reason)}</p> }
}

/// One item: its icon if a piece, its name, its topic's name (in the search's results), the size it is added at.
/// Greyed (`aria-disabled`, still reached by the keys) when the street cannot take it. `described` names what
/// describes it; `reason`, the reason to show under it.
fn item(w: SheetWatch, parts: MenuParts, e: Entry, described: Option<String>, reason: Option<(String, Said)>, with_topic: bool) -> impl IntoView {
    let (it, t) = (e.item, e.topic);
    let greyed = e.unavailable.is_some();
    let mark = match it {
        Item::Piece(k) => Some(view! { <span class="add-icon" inner_html=icon(KINDS[k].id)></span> }),
        _ => None,
    };
    let topic_name = with_topic.then(|| view! { <span class="add-topic-name">{move || w.i18n().tr(&t.name_key(), &Args::new())}</span> });
    let size = it.size_mm().map(|mm| view! { <span class="dw">{move || w.units().length_fine_in(mm, w.i18n().locale())}</span> });
    view! {
        <li>
            <button
                type="button"
                class="add-item"
                data-item=it.name_key()
                aria-disabled=greyed.then_some("true")
                aria-describedby=described
                on:click=move |_| choose(w, parts, it, greyed)
            >
                {mark}
                <b>{move || w.i18n().tr(&it.name_key(), &Args::new())}</b>
                {topic_name}
                {size}
            </button>
            {reason.map(|(id, s)| reason_line(w, id, s))}
        </li>
    }
}
