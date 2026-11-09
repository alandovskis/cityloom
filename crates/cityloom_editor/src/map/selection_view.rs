//! The Selected streets panel on the map page: the streets picked on the map or from the search, each with a
//! button to drop it, and one to clear them all. It shows what the map's view-model says and sends it what the
//! person does.

use std::rc::Rc;

use leptos::prelude::*;

use crate::map::corridor::Op;
use crate::map::vm::{MapVm, Stage};
use crate::shared::bind::Bound;

type Vm = Bound<MapVm>;

/// Puts the keyboard's focus on the panel's heading: the button that was pressed is gone (the street left the
/// list, or the list is empty), and focus must not fall to the page.
fn focus_heading() {
    focus("h-selection");
}

/// Puts the keyboard's focus on the element `id`, once the page has drawn what the press changed.
fn focus(id: &'static str) {
    #[cfg(target_arch = "wasm32")]
    request_animation_frame(move || {
        use leptos::wasm_bindgen::JsCast;

        if let Some(h) = leptos::prelude::document().get_element_by_id(id) {
            let _ = h.unchecked_into::<leptos::web_sys::HtmlElement>().focus();
        }
    });
    #[cfg(not(target_arch = "wasm32"))]
    let _ = id;
}

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
                                <button type="button" class="btn quiet" aria-label=label on:click=move |_| {
                                    vm.with(|v| v.deselect(uid));
                                    focus_heading();
                                }>
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
                        <button type="button" class="btn quiet" id="selection-clear" on:click=move |_| {
                                vm.with(|v| v.clear_selection());
                                focus_heading();
                            }>
                            {word(vm, "sel-clear")}
                        </button>
                    }
                })
        }}
        {move || (!streets().is_empty()).then(|| change(vm))}
        {move || {
            vm.with(|v| v.corridor_undo_label())
                .map(|label| {
                    view! {
                        <button type="button" class="btn quiet" id="corridor-undo" on:click=move |_| {
                            vm.with(|v| v.undo_corridor());
                            // the button goes when the change is undone or can never be: focus must not fall with it
                            if vm.with(|v| v.corridor_undo_label()).is_none() {
                                focus_heading();
                            }
                        }>
                            {label}
                        </button>
                    }
                })
        }}
    }
}

/// The change to make on the selected streets: pick it, review it street by street, then make it on the streets
/// that can take it; afterwards, what was done. The streets that cannot take it say why and are left as they are.
fn change(vm: Vm) -> impl IntoView {
    let stage = move || vm.with(|v| v.corridor_stage());
    let picked = move || vm.with(|v| v.corridor_op().id());
    let rows = move |label: &'static str| {
        view! {
            <ul class="corridor-rows" aria-label=word(vm, label)>
                {move || {
                    vm.with(|v| v.corridor_rows())
                        .into_iter()
                        .map(|r| {
                            view! {
                                <li class=if r.ready { "corridor-row ok" } else { "corridor-row bad" }>
                                    <span class="corridor-street">{r.name}</span>
                                    " "
                                    <span>{r.text}</span>
                                </li>
                            }
                        })
                        .collect_view()
                }}
            </ul>
        }
    };
    view! {
        <section class="corridor" aria-labelledby="h-corridor">
            <h3 id="h-corridor" class="note-h">{word(vm, "corridor-heading")}</h3>
            <label for="corridor-op">{word(vm, "corridor-op-label")}</label>
            <select id="corridor-op" on:change=move |e| vm.with(|v| v.set_corridor_op(&event_target_value(&e)))>
                {Op::ALL
                    .into_iter()
                    .map(|op| {
                        view! {
                            <option value=op.id() selected=move || picked() == op.id()>
                                {move || vm.with(|v| v.say_op(op))}
                            </option>
                        }
                    })
                    .collect_view()}
            </select>
            <div id="corridor-result" tabindex="-1">
                {move || match stage() {
                    Stage::Idle => {
                        view! {
                            <button type="button" class="btn" id="corridor-review" on:click=move |_| {
                                vm.with(|v| v.review_corridor());
                                focus("corridor-result");
                            }>
                                {word(vm, "corridor-review-button")}
                            </button>
                        }
                            .into_any()
                    }
                    Stage::Reviewing(_) => {
                        let ready = move || vm.with(|v| v.corridor_ready());
                        view! {
                            {rows("corridor-review-list")}
                            <button type="button" class="btn" id="corridor-apply" aria-disabled=move || (ready() == 0).to_string() on:click=move |_| {
                                if ready() == 0 {
                                    return;
                                }
                                vm.with(|v| v.apply_corridor());
                                focus("corridor-result");
                            }>
                                {move || vm.with(|v| v.corridor_apply_label(ready()))}
                            </button>
                            <button type="button" class="btn quiet" id="corridor-cancel" on:click=move |_| {
                                vm.with(|v| v.cancel_corridor());
                                focus("corridor-review");
                            }>
                                {word(vm, "corridor-cancel")}
                            </button>
                        }
                            .into_any()
                    }
                    Stage::Done(_) => {
                        view! {
                            {rows("corridor-report-list")}
                            <button type="button" class="btn quiet" id="corridor-close" on:click=move |_| {
                                vm.with(|v| v.cancel_corridor());
                                focus("corridor-review");
                            }>
                                {word(vm, "corridor-close")}
                            </button>
                        }
                            .into_any()
                    }
                }}
            </div>
        </section>
    }
}
