//! The Account page's view: it binds to `AccountVm` and decides nothing.

use std::rc::Rc;

use leptos::prelude::*;

use crate::account::text::identity_text;
use crate::account::vm::{AccountVm, State};
use crate::shared::i18n::Args;

type Vm = StoredValue<Rc<AccountVm>, LocalStorage>;

fn tr(vm: Vm, key: &'static str) -> String {
    vm.with_value(|v| v.i18n().tr(key, &Args::new()))
}

fn radio(vm: Vm, value: &'static str, store: bool, label: &'static str, help: &'static str) -> impl IntoView {
    view! {
        <label class="choice">
            <input
                type="radio"
                name="storage"
                value=value
                checked=move || vm.with_value(|v| v.storage()) == Some(store)
                prop:checked=move || vm.with_value(|v| v.storage()) == Some(store)
                on:change=move |_| vm.with_value(|v| v.choose_storage(store))
            />
            <span class="choice-body">
                <span class="choice-label">{move || tr(vm, label)}</span>
                <span class="choice-help">{move || tr(vm, help)}</span>
            </span>
        </label>
    }
}

fn storage(vm: Vm) -> impl IntoView {
    view! {
        <section class="account-section" aria-labelledby="h-storage">
            <h2 id="h-storage" tabindex="-1">
                {move || tr(vm, "account-storage-heading")}
            </h2>
            <p class="hint">{move || tr(vm, "account-storage-lead")}</p>
            <div class="choices" role="radiogroup" aria-labelledby="h-storage">
                {radio(vm, "device", false, "account-storage-device", "account-storage-device-help")}
                {radio(vm, "server", true, "account-storage-server", "account-storage-server-help")}
            </div>
        </section>
    }
}

/// The list is the only part that changes, so its heading stays put and can take focus when a row is gone.
fn linked(vm: Vm) -> impl IntoView {
    view! {
        <section class="account-section" aria-labelledby="h-linked">
            <h2 id="h-linked" tabindex="-1">
                {move || tr(vm, "account-linked-heading")}
            </h2>
            <p class="hint">{move || tr(vm, "account-linked-note")}</p>
            {move || {
                let rows = vm.with_value(|v| v.identities());
                let only_one = rows.len() == 1;
                let items = rows
                    .into_iter()
                    .map(|row| {
                        let text = vm.with_value(|v| identity_text(&v.i18n(), &row));
                        let button = row.can_unlink.then(|| {
                            let id = row.id.clone();
                            let named = vm.with_value(|v| v.i18n().tr("account-unlink-named", &Args::new().str("what", text.clone())));
                            view! {
                                <button type="button" class="btn" aria-label=named on:click=move |_| vm.with_value(|v| v.unlink(&id))>
                                    {tr(vm, "account-unlink")}
                                </button>
                            }
                        });
                        view! {
                            <li>
                                <span>{text}</span>
                                {button}
                            </li>
                        }
                    })
                    .collect_view();
                view! {
                    <ul class="linked">{items}</ul>
                    {only_one.then(|| view! { <p class="hint">{tr(vm, "account-only-one")}</p> })}
                }
            }}
        </section>
    }
}

fn links(vm: Vm) -> impl IntoView {
    view! {
        <section class="account-section" aria-labelledby="h-link">
            <h2 id="h-link">{move || tr(vm, "account-link-heading")}</h2>
            <p class="hint">{move || tr(vm, "account-link-note")}</p>
            {move || {
                vm.with_value(|v| v.link_options())
                    .into_iter()
                    .map(|o| {
                        let slug = o.slug.clone();
                        let label = vm.with_value(|v| v.i18n().tr("account-link-with", &Args::new().str("provider", o.name)));
                        view! {
                            <p class="menu-act">
                                <button type="button" class="btn" data-provider=o.slug on:click=move |_| vm.with_value(|v| v.link(&slug))>
                                    {label}
                                </button>
                            </p>
                        }
                    })
                    .collect_view()
            }}
        </section>
    }
}

fn delete(vm: Vm) -> impl IntoView {
    view! {
        <section class="account-section" aria-labelledby="h-delete">
            <h2 id="h-delete">{move || tr(vm, "account-delete-heading")}</h2>
            {move || {
                if vm.with_value(|v| v.confirming_delete()) {
                    view! {
                        <div
                            role="group"
                            aria-labelledby="h-confirm"
                            aria-describedby="delete-warning"
                            id="delete-confirm"
                            class="confirm"
                            on:keydown=move |e| {
                                if e.key() == "Escape" {
                                    e.stop_propagation();
                                    vm.with_value(|v| v.cancel_delete());
                                }
                            }
                        >
                            <h3 id="h-confirm">{tr(vm, "account-delete-confirm-heading")}</h3>
                            <p id="delete-warning">{tr(vm, "account-delete-warning")}</p>
                            <button type="button" id="delete-no" class="btn" on:click=move |_| vm.with_value(|v| v.cancel_delete())>
                                {tr(vm, "account-delete-no")}
                            </button>
                            <button type="button" class="btn danger" on:click=move |_| vm.with_value(|v| v.confirm_delete())>
                                {tr(vm, "account-delete-yes")}
                            </button>
                        </div>
                    }
                        .into_any()
                } else {
                    view! {
                        <button type="button" id="delete-btn" class="btn" on:click=move |_| vm.with_value(|v| v.ask_delete())>
                            {tr(vm, "account-delete")}
                        </button>
                    }
                        .into_any()
                }
            }}
        </section>
    }
}

fn ready(vm: Vm) -> impl IntoView {
    // Where the choice sits is settled when the page loads, so that making it does not move it from under the person.
    let choice_first = vm.with_value(|v| v.choice_first());
    view! {
        <p>{move || vm.with_value(|v| v.i18n().tr("account-signed-in-as", &Args::new().str("name", v.name())))}</p>
        {choice_first.then(|| storage(vm))}
        {linked(vm)}
        {links(vm)}
        {(!choice_first).then(|| storage(vm))}
        {delete(vm)}
        <p class="menu-act">
            <button type="button" class="btn" on:click=move |_| vm.with_value(|v| v.sign_out())>
                {move || tr(vm, "account-sign-out")}
            </button>
        </p>
    }
}

fn signed_out(vm: Vm) -> impl IntoView {
    let buttons = vm
        .with_value(|v| v.providers())
        .into_iter()
        .map(|p| {
            let slug = p.slug.clone();
            view! {
                <p class="menu-act">
                    <button type="button" class="btn" data-provider=p.slug on:click=move |_| vm.with_value(|v| v.sign_in(&slug))>
                        {p.name}
                    </button>
                </p>
            }
        })
        .collect_view();
    view! {
        <p>{tr(vm, "account-signed-out")}</p>
        <p class="hint">{tr(vm, "account-signed-out-help")}</p>
        <h2>{tr(vm, "account-sign-in-with")}</h2>
        {buttons}
    }
}

#[component]
pub fn AccountView(vm: Rc<AccountVm>) -> impl IntoView {
    let vm: Vm = StoredValue::new_local(vm);
    // Whatever the person was using was removed or moved: put focus where they can carry on. The effect runs after
    // the view has been updated by the same change. Effects do not run on the host, so the component tests do not
    // depend on this.
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        use crate::account::vm::Focus;
        use wasm_bindgen::JsCast;
        let (_, to) = vm.with_value(|v| v.focus_request());
        let id = match to {
            Focus::None => return,
            Focus::Page => "account-title",
            Focus::Storage => "h-storage",
            Focus::Linked => "h-linked",
            Focus::DeleteButton => "delete-btn",
            Focus::DeleteSafe => "delete-no",
        };
        if let Some(el) = document().get_element_by_id(id).and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
            let _ = el.focus();
        }
    });
    view! {
        <div class="account-view">
            <h1 id="account-title" tabindex="-1">
                {move || tr(vm, "account-page-heading")}
            </h1>
            {move || match vm.with_value(|v| v.state()) {
                State::Loading => view! { <p role="status">{tr(vm, "account-loading")}</p> }.into_any(),
                State::Unavailable => view! { <p role="status">{tr(vm, "account-unavailable")}</p> }.into_any(),
                State::SignedOut => signed_out(vm).into_any(),
                State::Ready => ready(vm).into_any(),
            }}
        </div>
    }
}
