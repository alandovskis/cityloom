//! The Selected streets panel on the map page: the streets picked on the map or from the search, each with a
//! button to drop it, and one to clear them all. It shows what the map's view-model says and sends it what the
//! person does.

use std::rc::Rc;

use leptos::prelude::*;

use crate::map::vm::MapVm;
use crate::shared::bind::Bound;

type Vm = Bound<MapVm>;

/// A message of the page, for a view: it follows a switch of language.
fn word(vm: Vm, key: &'static str) -> impl Fn() -> String + Copy + 'static {
    move || vm.with(|v| v.word(key))
}

#[component]
pub fn SelectedStreets(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    let streets = move || vm.with(|v| v.selection().streets());
    let lead = move || {
        let n = streets().len();
        if n == 0 { vm.with(|v| v.word("sel-empty")) } else { vm.with(|v| v.selection_lead(n)) }
    };
    view! {
        <p class="hint" id="selection-lead">{lead}</p>
        <ul class="checks places-list" id="selection-list" aria-label=word(vm, "sel-list-label") hidden=move || streets().is_empty()>
            {move || {
                streets()
                    .into_iter()
                    .map(|uid| {
                        let name = vm.with(|v| v.street_label(uid));
                        let label = vm.with(|v| v.selection_remove_label(&name));
                        view! {
                            <li class="selection-item">
                                <span>{name}</span>
                                <button type="button" class="btn quiet" aria-label=label on:click=move |_| vm.with(|v| v.deselect(uid))>
                                    {word(vm, "sel-remove-short")}
                                </button>
                            </li>
                        }
                    })
                    .collect_view()
            }}
        </ul>
        {move || {
            (!streets().is_empty())
                .then(|| {
                    view! {
                        <button type="button" class="btn quiet" id="selection-clear" on:click=move |_| vm.with(|v| v.clear_selection())>
                            {word(vm, "sel-clear")}
                        </button>
                    }
                })
        }}
    }
}
