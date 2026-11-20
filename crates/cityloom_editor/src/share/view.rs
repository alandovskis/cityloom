//! The share page: it binds to `ShareVm` (a project's links) or `OpenVm` (a link being opened) and decides nothing.

use std::rc::Rc;

use leptos::prelude::*;

use crate::share::vm::{Account, LinkRow, OpenVm, Opening, ShareVm};

type Vm = StoredValue<Rc<ShareVm>, LocalStorage>;

fn word(vm: Vm, key: &'static str) -> String {
    vm.with_value(|v| v.word(key))
}

/// The id of an element of the row of the link `token` (base64url, safe in an id).
fn part(what: &str, token: &str) -> String {
    format!("sharing-{what}-{token}")
}

/// Moves focus where the view-model asks, once the page shows it. Effects do not run on the host.
#[cfg(target_arch = "wasm32")]
fn follow_focus(vm: Vm) {
    Effect::new(move |_| {
        use crate::share::vm::Focus;
        use wasm_bindgen::JsCast;
        let (_, to) = vm.with_value(|v| v.focus_request());
        let id = match to {
            Focus::None => return,
            Focus::Link(token) => part("link", &token),
            Focus::Create => "sharing-create".to_string(),
        };
        if let Some(el) = document().get_element_by_id(&id).and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
            let _ = el.focus();
        }
    });
}

/// One link made here: the link itself, to select or copy, until when it works, and its two buttons.
fn link_view(vm: Vm, row: LinkRow) -> impl IntoView {
    let (field_id, expiry_id) = (part("link", &row.token), part("expiry", &row.token));
    let (copy_token, delete_token) = (row.token.clone(), row.token.clone());
    view! {
        <li class="share-row" class:share-expired=row.expired>
            <label for=field_id.clone()>{move || word(vm, "share-link-label")}</label>
            <input
                id=field_id
                class="share-link"
                type="text"
                readonly
                value=row.link.clone()
                aria-describedby=expiry_id.clone()
                on:focus=move |ev| {
                    use wasm_bindgen::JsCast;
                    if let Some(input) = ev.target().and_then(|t| t.dyn_into::<leptos::web_sys::HtmlInputElement>().ok()) {
                        input.select();
                    }
                }
            />
            <p class="share-expiry" id=expiry_id>{row.expiry.clone()}</p>
            <div class="share-buttons">
                <button
                    type="button"
                    class="btn"
                    id=part("copy", &row.token)
                    disabled=row.expired
                    on:click=move |_| vm.with_value(|v| v.copy(&copy_token))
                >
                    {move || word(vm, "share-copy")}
                </button>
                <button
                    type="button"
                    class="btn danger"
                    id=part("delete", &row.token)
                    disabled=move || vm.with_value(|v| v.busy())
                    on:click=move |_| vm.with_value(|v| v.delete(&delete_token))
                >
                    {move || word(vm, "share-delete")}
                </button>
            </div>
        </li>
    }
}

/// The page that shares a project: what a share is, the button that makes one, and the links made here.
#[component]
pub fn ShareView(vm: Rc<ShareVm>) -> impl IntoView {
    let vm: Vm = StoredValue::new_local(vm);
    #[cfg(target_arch = "wasm32")]
    follow_focus(vm);
    // The window title is the page's, in its language (the shell leaves `<title>` to the page).
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| document().set_title(&word(vm, "share-page-title")));
    let blocked = move || vm.with_value(|v| v.blocked());
    view! {
        <p class="share-back-row">
            <a class="share-back" href=vm.with_value(|v| v.back_href())>{move || word(vm, "share-back")}</a>
        </p>
        <h1 id="sharing-heading" tabindex="-1">{move || vm.with_value(|v| v.heading())}</h1>
        <p class="share-intro">{move || word(vm, "share-intro")}</p>
        <p class="share-intro">{move || vm.with_value(|v| v.retention())}</p>
        <p id="sharing-blocked" class="sharing-blocked" hidden=move || blocked().is_none()>{blocked}</p>
        <button
            type="button"
            id="sharing-create"
            class="btn sharing-create"
            aria-describedby="sharing-blocked"
            disabled=move || vm.with_value(|v| v.busy() || v.blocked().is_some() || v.account() == Account::Unknown)
            on:click=move |_| vm.with_value(|v| v.create())
        >
            {move || word(vm, "share-create")}
        </button>
        {move || {
            let outcome = vm.with_value(|v| v.outcome());
            let (error, none) = (outcome.as_ref().is_some_and(|(_, e)| *e), outcome.is_none());
            view! {
                <p id="sharing-outcome" class="sharing-outcome" class:share-error=error hidden=none>
                    {outcome.map(|(text, _)| text)}
                </p>
            }
        }}
        <h2 id="sharing-links-heading">{move || word(vm, "share-links-heading")}</h2>
        {move || {
            let rows = vm.with_value(|v| v.links());
            if rows.is_empty() {
                view! { <p class="share-empty">{move || word(vm, "share-links-empty")}</p> }.into_any()
            } else {
                view! {
                    <ul class="share-list" aria-labelledby="sharing-links-heading">
                        {rows.into_iter().map(|row| link_view(vm, row)).collect_view()}
                    </ul>
                }
                    .into_any()
            }
        }}
    }
}

/// The page while it opens a link, and why it could not.
#[component]
pub fn OpenView(vm: Rc<OpenVm>) -> impl IntoView {
    let vm = StoredValue::new_local(vm);
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| document().set_title(&vm.with_value(|v| v.word("share-page-title"))));
    let failed = move || matches!(vm.with_value(|v| v.state()), Opening::Failed(_));
    view! {
        <h1 id="sharing-heading" tabindex="-1">{move || vm.with_value(|v| v.word("share-open-heading"))}</h1>
        // One node each for the page's life: a status while it works, an alert when it cannot.
        <p id="sharing-open-status" role="status" class="share-intro">
            {move || (!failed()).then(|| vm.with_value(|v| v.message()))}
        </p>
        <p id="sharing-open-error" role="alert" class="sharing-outcome share-error">
            {move || failed().then(|| vm.with_value(|v| v.message()))}
        </p>
        <p class="share-back-row" hidden=move || !failed()>
            <a class="share-back" href="index.html">{move || vm.with_value(|v| v.word("share-open-projects"))}</a>
        </p>
    }
}
