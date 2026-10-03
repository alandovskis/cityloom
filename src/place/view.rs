//! The box that finds a place by name: the home page's main action.

use std::rc::Rc;

use leptos::ev::KeyboardEvent;
use leptos::prelude::*;

use super::vm::{AreaVm, Status};
use crate::shared::bind::Bound;

type Vm = Bound<AreaVm>;

/// A search box, a list of what it finds and a line saying how it is going. Enter searches; once there
/// are places, the arrow keys move down them and Enter opens the one they are on.
#[component]
pub fn AreaSearch(vm: Rc<AreaVm>) -> impl IntoView {
    let owner = vm.clone();
    let vm = Bound::new(vm);
    let input = NodeRef::<leptos::html::Input>::new();
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::wasm_bindgen::JsCast;
        let handle = window_event_listener(leptos::ev::keydown, move |e: KeyboardEvent| {
            let in_a_field = e
                .target()
                .and_then(|t| t.dyn_into::<leptos::web_sys::Element>().ok())
                .is_some_and(|el| el.closest("input, select, textarea").ok().flatten().is_some());
            if e.key() == "/" && !e.meta_key() && !e.ctrl_key() && !e.alt_key() && !in_a_field {
                e.prevent_default();
                if let Some(i) = input.get_untracked() {
                    let _ = i.focus();
                }
            }
        });
        on_cleanup(move || handle.remove());
    }
    let open = move || vm.with(|v| v.note().is_some());
    let active = move || vm.with(|v| v.active());
    let keydown = move |e: KeyboardEvent| match e.key().as_str() {
        "ArrowDown" => {
            e.prevent_default();
            vm.with(|v| v.move_active(1));
        }
        "ArrowUp" => {
            e.prevent_default();
            vm.with(|v| v.move_active(-1));
        }
        "Escape" if open() => {
            e.prevent_default();
            vm.with(|v| v.set_text(""));
        }
        _ => {}
    };
    let owner = StoredValue::new_local(owner);
    view! {
        <form
            class="search"
            role="search"
            on:submit=move |e| {
                e.prevent_default();
                let vm = owner.get_value();
                if vm.status() == Status::Found { vm.choose_active() } else { vm.search() }
            }
        >
            <svg class="search-ico" viewBox="0 0 20 20" width="20" height="20" aria-hidden="true" focusable="false"><circle cx="8.5" cy="8.5" r="5.5"/><path d="m12.8 12.8 4 4"/></svg>
            <input
                id="search"
                type="search"
                name="q"
                node_ref=input
                placeholder="Find a place, like Kreuzberg, Berlin"
                aria-label="Find a place"
                role="combobox"
                aria-autocomplete="list"
                aria-controls="area-results"
                aria-expanded=move || open().to_string()
                aria-activedescendant=move || active().map(|i| format!("ar-{i}"))
                autocomplete="off"
                spellcheck="false"
                enterkeyhint="search"
                prop:value=move || vm.with(|v| v.text())
                on:input=move |e| vm.with(|v| v.set_text(&event_target_value(&e)))
                on:keydown=keydown
            />
            <kbd class="search-key" aria-hidden="true">"/"</kbd>
            <button type="submit" class="search-go" aria-disabled=move || vm.with(|v| v.busy()).to_string()>
                {move || if vm.with(|v| v.status()) == Status::Found { "Open" } else { "Find" }}
            </button>
            <div class="search-pop" hidden=move || !open()>
                <ul id="area-results" role="listbox" aria-label="Places found">
                    {move || vm.with(|v| v.rows()).into_iter().enumerate().map(|(i, r)| row(vm, owner, r.name, r.detail, i)).collect_view()}
                </ul>
                <p class="search-note" role="status">{move || vm.with(|v| v.note())}</p>
            </div>
        </form>
    }
}

fn row(vm: Vm, owner: StoredValue<Rc<AreaVm>, LocalStorage>, name: String, detail: String, i: usize) -> impl IntoView {
    let on = move || vm.with(|v| v.active()) == Some(i);
    view! {
        <li role="option" id=format!("ar-{i}") aria-selected=move || on().to_string()>
            <button type="button" class=move || if on() { "place-row on" } else { "place-row" } tabindex="-1" on:click=move |_| owner.get_value().choose(i)>
                <b>{name}</b>
                <small>{detail}</small>
            </button>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::test_ports_with_fetcher;
    use crate::shared::testing::{count, html};

    fn page(vm: &Rc<AreaVm>) -> String {
        let vm = vm.clone();
        html(move || view! { <AreaSearch vm=vm/> }.into_any())
    }

    #[test]
    fn the_box_is_a_labelled_combobox_with_a_find_button_and_a_closed_list() {
        let (ports, ..) = test_ports_with_fetcher();
        let h = page(&AreaVm::new(ports));
        assert!(h.contains("role=\"combobox\"") && h.contains("aria-label=\"Find a place\""));
        assert!(h.contains(">Find</button>") && !h.contains(">Open</button>"));
        assert!(h.contains("aria-expanded=\"false\""));
        assert_eq!(count(&h, "role=\"option\""), 0);
    }

    #[test]
    fn what_was_found_is_listed_with_the_first_chosen_and_the_button_offers_to_open_it() {
        let (ports, fetcher, _) = test_ports_with_fetcher();
        let vm = AreaVm::new(ports);
        vm.set_text("Kreuzberg");
        vm.search();
        fetcher
            .answer(Ok(br#"[{"display_name":"Kreuzberg, Berlin, Germany","lat":"52.5","lon":"13.4","boundingbox":["52.4","52.6","13.3","13.5"]}]"#.to_vec()));
        let h = page(&vm);
        assert_eq!(count(&h, "role=\"option\""), 1);
        assert!(h.contains("<b>Kreuzberg, Berlin</b>") && h.contains("<small>Germany</small>"));
        assert!(h.contains("aria-selected=\"true\"") && h.contains("aria-activedescendant=\"ar-0\""));
        assert!(h.contains(">Open</button>") && h.contains("aria-expanded=\"true\""));
        assert!(h.contains("1 place found"));
    }

    #[test]
    fn while_the_streets_are_got_the_button_is_off_and_the_note_says_so() {
        let (ports, fetcher, _) = test_ports_with_fetcher();
        let vm = AreaVm::new(ports);
        vm.set_text("x");
        vm.search();
        fetcher.answer(Ok(br#"[{"display_name":"Kreuzberg, Berlin","lat":"52.5","lon":"13.4","boundingbox":["52.4","52.6","13.3","13.5"]}]"#.to_vec()));
        vm.choose(0);
        let h = page(&vm);
        assert!(h.contains("aria-disabled=\"true\""));
        assert!(h.contains("Getting the streets of Kreuzberg, Berlin…"));
    }
}
