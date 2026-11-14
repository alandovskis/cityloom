//! The timeline: it binds to `TimelineVm` and decides nothing.

use std::rc::Rc;

use leptos::prelude::*;

use crate::timeline::vm::{Comparison, Row, TimelineVm};

type Vm = StoredValue<Rc<TimelineVm>, LocalStorage>;

fn word(vm: Vm, key: &'static str) -> String {
    vm.with_value(|v| v.word(key))
}

/// The id of an element of the row of milestone `id` (ids are `m-` and hex digits or `m<n>`, safe in an id).
fn part(what: &str, id: &str) -> String {
    format!("timeline-{what}-{id}")
}

/// Moves focus where the view-model asks, once the page shows it. Effects do not run on the host, so the component
/// tests do not depend on this.
#[cfg(target_arch = "wasm32")]
fn follow_focus(vm: Vm) {
    Effect::new(move |_| {
        use crate::timeline::vm::Focus;
        use wasm_bindgen::JsCast;
        let (_, to) = vm.with_value(|v| v.focus_request());
        let id = match to {
            Focus::None => return,
            Focus::Heading => "timeline-heading".to_string(),
            Focus::CreateField => "timeline-name".to_string(),
            Focus::CompareButton(id) => part("compare", &id),
            Focus::RestoreButton(id) => part("restore", &id),
            Focus::DeleteButton(id) => part("delete", &id),
            Focus::RestoreSafe => "timeline-restore-safe".to_string(),
            Focus::DeleteSafe => "timeline-delete-safe".to_string(),
        };
        if let Some(el) = document().get_element_by_id(&id).and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
            let _ = el.focus();
        }
    });
}

/// The question asked in a row before the work is put back.
fn restore_question(vm: Vm) -> impl IntoView {
    view! {
        <div
            role="group"
            class="confirm"
            aria-labelledby="timeline-restore-question"
            on:keydown=move |e| {
                if e.key() == "Escape" {
                    e.stop_propagation();
                    vm.with_value(|v| v.cancel_restore());
                }
            }
        >
            <p id="timeline-restore-question">{move || vm.with_value(|v| v.restore_question())}</p>
            <div class="timeline-buttons">
                <button type="button" id="timeline-restore-safe" class="btn" on:click=move |_| vm.with_value(|v| v.cancel_restore())>
                    {move || word(vm, "timeline-cancel")}
                </button>
                <button type="button" id="timeline-restore-yes" class="btn" on:click=move |_| {
                    vm.with_value(|v| v.confirm_restore());
                }>
                    {move || word(vm, "timeline-restore-yes")}
                </button>
            </div>
        </div>
    }
}

/// The question asked in a row before the milestone is deleted.
fn delete_question(vm: Vm) -> impl IntoView {
    view! {
        <div
            role="group"
            class="confirm"
            aria-labelledby="timeline-delete-question"
            on:keydown=move |e| {
                if e.key() == "Escape" {
                    e.stop_propagation();
                    vm.with_value(|v| v.cancel_delete());
                }
            }
        >
            <p id="timeline-delete-question">{move || vm.with_value(|v| v.delete_question())}</p>
            <div class="timeline-buttons">
                <button type="button" id="timeline-delete-safe" class="btn" on:click=move |_| vm.with_value(|v| v.cancel_delete())>
                    {move || word(vm, "timeline-cancel")}
                </button>
                <button type="button" id="timeline-delete-yes" class="btn danger" on:click=move |_| {
                    vm.with_value(|v| v.confirm_delete());
                }>
                    {move || word(vm, "timeline-delete-yes")}
                </button>
            </div>
        </div>
    }
}

/// A milestone set beside the work now: how many places differ, and each one with what about it changed.
fn comparison_view(vm: Vm, c: Comparison) -> impl IntoView {
    let heading_id = part("heading", &c.id);
    let labelled_by = heading_id.clone();
    view! {
        <section class="timeline-compare" aria-labelledby=labelled_by>
            <h3 id=heading_id>{c.heading}</h3>
            <p class="timeline-compare-summary">{c.summary}</p>
            {(!c.rows.is_empty())
                .then(|| {
                    view! {
                        <ul class="timeline-diff">
                            {c
                                .rows
                                .into_iter()
                                .map(|r| {
                                    view! {
                                        <li>
                                            <span class="timeline-diff-how">{r.how}</span>
                                            " "
                                            <span class="timeline-diff-place">{r.label}</span>
                                            {(!r.aspects.is_empty())
                                                .then(|| {
                                                    view! {
                                                        <ul class="timeline-aspects">
                                                            {r.aspects.into_iter().map(|a| view! { <li>{a}</li> }).collect_view()}
                                                        </ul>
                                                    }
                                                })}
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    }
                })}
            <button type="button" class="btn" on:click=move |_| vm.with_value(|v| v.close_compare())>
                {move || word(vm, "timeline-compare-close")}
            </button>
        </section>
    }
}

/// One milestone: its name, when and by whom it was saved, what it holds, and what can be done with it.
fn row_view(vm: Vm, row: Row) -> impl IntoView {
    let (name_id, meta_id) = (part("name", &row.id), part("meta", &row.id));
    let named_by = format!("{name_id} {meta_id}");
    let (compare_id, restore_id, delete_id) = (row.id.clone(), row.id.clone(), row.id.clone());
    let (here_compare, here_restore, here_delete) = (row.id.clone(), row.id.clone(), row.id.clone());
    let (compare_label, restore_label, delete_label) = (row.name.clone(), row.name.clone(), row.name.clone());
    view! {
        <li class="timeline-row" aria-labelledby=named_by>
            <h3 class="timeline-name" id=name_id>{row.name.clone()}</h3>
            {row.auto.then(|| view! { <p class="timeline-auto">{move || vm.with_value(|v| v.auto_label())}</p> })}
            <p class="timeline-meta" id=meta_id>
                <span class="timeline-when">{row.when.clone()}</span>
                {(!row.author.is_empty()).then(|| view! { <span class="timeline-author">{row.author.clone()}</span> })}
                <span class="timeline-version">{row.version.clone()}</span>
            </p>
            <p class="timeline-summary">{row.summary.clone()}</p>
            <div class="timeline-buttons">
                <button
                    type="button"
                    class="btn"
                    id=part("compare", &row.id)
                    aria-label=move || vm.with_value(|v| v.named("timeline-compare-named", &compare_label))
                    on:click=move |_| {
                        vm.with_value(|v| v.compare(&compare_id));
                    }
                >
                    {move || word(vm, "timeline-compare")}
                </button>
                <button
                    type="button"
                    class="btn"
                    id=part("restore", &row.id)
                    aria-label=move || vm.with_value(|v| v.named("timeline-restore-named", &restore_label))
                    on:click=move |_| vm.with_value(|v| v.ask_restore(&restore_id))
                >
                    {move || word(vm, "timeline-restore")}
                </button>
                <button
                    type="button"
                    class="btn"
                    id=part("delete", &row.id)
                    aria-label=move || vm.with_value(|v| v.named("timeline-delete-named", &delete_label))
                    on:click=move |_| vm.with_value(|v| v.ask_delete(&delete_id))
                >
                    {move || word(vm, "timeline-delete")}
                </button>
            </div>
            {move || {
                (vm.with_value(|v| v.comparing()).as_deref() == Some(here_compare.as_str()))
                    .then(|| vm.with_value(|v| v.comparison()))
                    .flatten()
                    .map(|c| comparison_view(vm, c))
            }}
            {move || (vm.with_value(|v| v.restoring()).as_deref() == Some(here_restore.as_str())).then(|| restore_question(vm))}
            {move || (vm.with_value(|v| v.deleting()).as_deref() == Some(here_delete.as_str())).then(|| delete_question(vm))}
        </li>
    }
}

/// The form that saves the work now as a milestone.
fn create_form(vm: Vm) -> impl IntoView {
    let field = NodeRef::<leptos::html::Input>::new();
    let invalid = move || vm.with_value(|v| v.create_error()).is_some();
    view! {
        <form
            class="timeline-create"
            aria-labelledby="timeline-create-heading"
            on:submit=move |ev| {
                ev.prevent_default();
                let Some(input) = field.get() else { return };
                if vm.with_value(|v| v.create(&input.value())) {
                    input.set_value("");
                }
            }
        >
            <h2 id="timeline-create-heading">{move || word(vm, "timeline-create-heading")}</h2>
            <div class="timeline-field">
                <label for="timeline-name">{move || word(vm, "timeline-create-name")}</label>
                <input
                    id="timeline-name"
                    type="text"
                    autocomplete="off"
                    node_ref=field
                    aria-invalid=move || if invalid() { "true" } else { "false" }
                    aria-describedby="timeline-create-hint timeline-create-error"
                />
            </div>
            <p id="timeline-create-hint" class="timeline-hint">{move || word(vm, "timeline-create-hint")}</p>
            <p id="timeline-create-error" class="timeline-error">{move || vm.with_value(|v| v.create_error())}</p>
            <button type="submit" class="btn timeline-create-go">{move || word(vm, "timeline-create-submit")}</button>
        </form>
    }
}

/// The page's heading, the milestones oldest first, and the form that saves one.
#[component]
pub fn TimelineView(vm: Rc<TimelineVm>) -> impl IntoView {
    let vm: Vm = StoredValue::new_local(vm);
    #[cfg(target_arch = "wasm32")]
    follow_focus(vm);
    // The window title is the page's, in its language (the shell leaves `<title>` to the page).
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| document().set_title(&word(vm, "timeline-page-title")));
    view! {
        <p class="timeline-back-row">
            <a class="timeline-back" href=vm.with_value(|v| v.back_href())>{move || word(vm, "timeline-back")}</a>
        </p>
        <h1 id="timeline-heading" tabindex="-1">{move || vm.with_value(|v| v.heading())}</h1>
        {create_form(vm)}
        {move || {
            let rows = vm.with_value(|v| v.rows());
            if rows.is_empty() {
                view! { <p class="timeline-empty">{move || word(vm, "timeline-empty")}</p> }.into_any()
            } else {
                view! {
                    <h2 id="timeline-list-heading">{move || word(vm, "timeline-list-heading")}</h2>
                    <ol class="timeline-list" aria-labelledby="timeline-list-heading">
                        {rows.into_iter().map(|row| row_view(vm, row)).collect_view()}
                    </ol>
                }
                    .into_any()
            }
        }}
    }
}
