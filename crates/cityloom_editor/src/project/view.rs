//! The list of projects: it binds to `ProjectsVm` and decides nothing.

use std::rc::Rc;

use leptos::prelude::*;

use crate::project::vm::{ProjectsVm, Row};

type Vm = StoredValue<Rc<ProjectsVm>, LocalStorage>;

fn word(vm: Vm, key: &'static str) -> String {
    vm.with_value(|v| v.word(key))
}

/// The id of an element of the row of project `id` (project ids are `a-z`, `0-9` and `-`, safe in an id).
fn part(what: &str, id: &str) -> String {
    format!("project-{what}-{id}")
}

/// Moves focus where the view-model asks, once the page shows it: the effect runs after the view has been updated by
/// the same change. Effects do not run on the host, so the component tests do not depend on this.
#[cfg(target_arch = "wasm32")]
fn follow_focus(vm: Vm) {
    Effect::new(move |_| {
        use crate::project::vm::Focus;
        use wasm_bindgen::JsCast;
        let (_, to) = vm.with_value(|v| v.focus_request());
        let id = match to {
            Focus::None => return,
            Focus::Heading => "projects-heading".to_string(),
            Focus::CreateField => "project-name".to_string(),
            Focus::RenameField => "project-rename-field".to_string(),
            Focus::RenameButton(id) => part("rename", &id),
            Focus::DeleteButton(id) => part("delete", &id),
            Focus::DeleteSafe => "project-delete-safe".to_string(),
        };
        if let Some(el) = document().get_element_by_id(&id).and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
            let _ = el.focus();
        }
    });
}

/// The form that renames project `id`, in its row.
fn rename_form(vm: Vm, row: Row) -> impl IntoView {
    let field = NodeRef::<leptos::html::Input>::new();
    let (id, label) = (row.id.clone(), row.name.clone());
    let invalid = move || vm.with_value(|v| v.rename_error()).is_some();
    view! {
        <form
            class="project-rename"
            aria-label=move || vm.with_value(|v| v.named("project-rename-named", &label))
            on:submit=move |ev| {
                ev.prevent_default();
                let name = field.get().map(|f| f.value()).unwrap_or_default();
                vm.with_value(|v| v.rename(&id, &name));
            }
            on:keydown=move |e| {
                if e.key() == "Escape" {
                    e.stop_propagation();
                    vm.with_value(|v| v.cancel_rename());
                }
            }
        >
            <label for="project-rename-field">{move || word(vm, "project-create-name")}</label>
            <input
                id="project-rename-field"
                type="text"
                autocomplete="off"
                node_ref=field
                value=row.name.clone()
                aria-invalid=move || if invalid() { "true" } else { "false" }
                aria-describedby="project-rename-error"
            />
            <p id="project-rename-error" class="project-error">
                {move || vm.with_value(|v| v.rename_error())}
            </p>
            <div class="project-buttons">
                <button type="submit" class="btn">{move || word(vm, "project-rename-save")}</button>
                <button type="button" class="btn" on:click=move |_| vm.with_value(|v| v.cancel_rename())>
                    {move || word(vm, "project-cancel")}
                </button>
            </div>
        </form>
    }
}

/// The question asked in a row before its project is deleted.
fn delete_question(vm: Vm) -> impl IntoView {
    view! {
        <div
            role="group"
            class="confirm"
            aria-labelledby="project-delete-question"
            on:keydown=move |e| {
                if e.key() == "Escape" {
                    e.stop_propagation();
                    vm.with_value(|v| v.cancel_delete());
                }
            }
        >
            <p id="project-delete-question">{move || vm.with_value(|v| v.delete_question())}</p>
            <div class="project-buttons">
                <button type="button" id="project-delete-safe" class="btn" on:click=move |_| vm.with_value(|v| v.cancel_delete())>
                    {move || word(vm, "project-cancel")}
                </button>
                <button type="button" id="project-delete-yes" class="btn danger" on:click=move |_| {
                    vm.with_value(|v| v.confirm_delete());
                }>
                    {move || word(vm, "project-delete-yes")}
                </button>
            </div>
        </div>
    }
}

/// One project: its name, which opens it, its area and when it changed, and what can be done to it.
fn row_view(vm: Vm, row: Row) -> impl IntoView {
    let (name_id, area_id) = (part("name", &row.id), part("area", &row.id));
    let named_by = format!("{name_id} {area_id}");
    let href = vm.with_value(|v| v.href(&row.id));
    let (open_id, rename_id, delete_id) = (row.id.clone(), row.id.clone(), row.id.clone());
    let (here_rename, here_delete) = (row.id.clone(), row.id.clone());
    let (rename_label, delete_label) = (row.name.clone(), row.name.clone());
    let form_row = row.clone();
    view! {
        <li class="project-row" aria-labelledby=named_by>
            <a
                class="project-open"
                id=name_id
                href=href
                on:click=move |ev| {
                    // A click that asks for a new tab or window is the browser's to follow.
                    if ev.button() == 0 && !(ev.ctrl_key() || ev.meta_key() || ev.shift_key() || ev.alt_key()) {
                        ev.prevent_default();
                        vm.with_value(|v| v.open(&open_id));
                    }
                }
            >
                {row.name.clone()}
            </a>
            <p class="project-meta">
                <span class="project-area-label">{move || word(vm, "project-area")}</span>
                " "
                <span id=area_id>{row.area_label.clone()}</span>
                {(!row.changed.is_empty()).then(|| view! { <span class="project-changed">{row.changed.clone()}</span> })}
            </p>
            <div class="project-buttons">
                <button
                    type="button"
                    class="btn"
                    id=part("rename", &row.id)
                    aria-label=move || vm.with_value(|v| v.named("project-rename-named", &rename_label))
                    on:click=move |_| vm.with_value(|v| v.start_rename(&rename_id))
                >
                    {move || word(vm, "project-rename")}
                </button>
                <button
                    type="button"
                    class="btn"
                    id=part("delete", &row.id)
                    aria-label=move || vm.with_value(|v| v.named("project-delete-named", &delete_label))
                    on:click=move |_| vm.with_value(|v| v.ask_delete(&delete_id))
                >
                    {move || word(vm, "project-delete")}
                </button>
            </div>
            {move || (vm.with_value(|v| v.renaming()).as_deref() == Some(here_rename.as_str())).then(|| rename_form(vm, form_row.clone()))}
            {move || (vm.with_value(|v| v.deleting()).is_some_and(|r| r.id == here_delete)).then(|| delete_question(vm))}
        </li>
    }
}

/// The form that makes a new project, in the default area, which it shows.
fn create_form(vm: Vm) -> impl IntoView {
    let field = NodeRef::<leptos::html::Input>::new();
    let invalid = move || vm.with_value(|v| v.create_error()).is_some();
    view! {
        <form
            class="project-create"
            aria-labelledby="project-create-heading"
            on:submit=move |ev| {
                ev.prevent_default();
                let Some(input) = field.get() else { return };
                if vm.with_value(|v| v.create(&input.value())) {
                    input.set_value("");
                }
            }
        >
            <h2 id="project-create-heading">{move || word(vm, "project-create-heading")}</h2>
            <div class="project-field">
                <label for="project-name">{move || word(vm, "project-create-name")}</label>
                <input
                    id="project-name"
                    type="text"
                    autocomplete="off"
                    node_ref=field
                    aria-invalid=move || if invalid() { "true" } else { "false" }
                    aria-describedby="project-create-error"
                />
            </div>
            <dl class="project-area">
                <dt>{move || word(vm, "project-area")}</dt>
                <dd>{vm.with_value(|v| v.default_area_label())}</dd>
            </dl>
            <p id="project-create-error" class="project-error">
                {move || vm.with_value(|v| v.create_error())}
            </p>
            <button type="submit" class="btn project-create-go">{move || word(vm, "project-create-submit")}</button>
        </form>
    }
}

/// The list of projects with its heading, the notice of a project that is gone, and the form to make one.
#[component]
pub fn ProjectsView(vm: Rc<ProjectsVm>) -> impl IntoView {
    let vm: Vm = StoredValue::new_local(vm);
    #[cfg(target_arch = "wasm32")]
    follow_focus(vm);
    // The window title is the page's, in its language (the shell leaves `<title>` to the page).
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| document().set_title(&word(vm, "project-page-title")));
    view! {
        <h1 id="projects-heading" tabindex="-1">{move || word(vm, "project-list-heading")}</h1>
        // One status for the page's life: its words follow a switch of language and it empties once the person acts.
        <p class="project-notice" role="status" hidden=move || vm.with_value(|v| v.notice()).is_none()>
            {move || vm.with_value(|v| v.notice())}
        </p>
        {move || {
            let rows = vm.with_value(|v| v.rows());
            if rows.is_empty() {
                view! { <p class="project-empty">{move || word(vm, "project-empty")}</p> }.into_any()
            } else {
                view! { <ul class="project-list">{rows.into_iter().map(|row| row_view(vm, row)).collect_view()}</ul> }.into_any()
            }
        }}
        {create_form(vm)}
    }
}
