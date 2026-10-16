//! The Sign in button of the header and its menu, drawn into the `#session-slot` of every page. Its open and
//! closed state is bound by `view::bind_menu`; the words and the choices are this component's.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::Args;
use crate::shell::session::{Session, SessionVm};

#[component]
pub fn SessionMenu(vm: Rc<SessionVm>) -> impl IntoView {
    let vm = StoredValue::new_local(vm);
    let tr = move |key: &'static str| vm.with_value(|v| v.i18n().tr(key, &Args::new()));
    // The button, or the menu it opens, was hidden under the focus: put focus back where the person can carry on.
    // The effect runs after the box's `hidden` is updated, so the button can take it.
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        use crate::shell::session::Focus;
        use wasm_bindgen::JsCast;
        let (_, to) = vm.with_value(|v| v.focus_request());
        let target = match to {
            Focus::None => return,
            Focus::Button => "#session-btn",
            Focus::Page => "main",
        };
        if let Some(el) = document().query_selector(target).ok().flatten().and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
            if to == Focus::Page {
                el.set_tab_index(-1);
            }
            let _ = el.focus();
        }
    });
    view! {
        <div class="session-box" hidden=move || !vm.with_value(|v| v.visible())>
            <button
                type="button"
                id="session-btn"
                class="btn quiet"
                aria-haspopup="true"
                aria-expanded="false"
                aria-controls="session-menu"
                aria-label=move || vm.with_value(|v| v.label())
                title=move || vm.with_value(|v| v.title())
                disabled=move || vm.with_value(|v| v.state() == Session::Unavailable)
            >
                <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
                    <circle cx="8" cy="5.5" r="2.5"></circle>
                    <path d="M3 13.5c.6-2.6 2.6-4 5-4s4.4 1.4 5 4"></path>
                </svg>
                <span id="session-btn-label">{move || vm.with_value(|v| v.label())}</span>
            </button>
            <div id="session-menu" class="menu" hidden>
                {move || match vm.with_value(|v| v.state()) {
                    Session::SignedIn { name } => {
                        view! {
                            <p class="menu-h" id="session-heading">{name}</p>
                            <p class="menu-act">
                                <a class="btn" href="account.html">{move || tr("session-account")}</a>
                            </p>
                            <p class="menu-act">
                                <button type="button" class="btn" on:click=move |_| vm.with_value(|v| v.sign_out())>
                                    {move || tr("session-sign-out")}
                                </button>
                            </p>
                        }
                            .into_any()
                    }
                    _ => {
                        view! {
                            <p class="menu-h" id="session-heading">{move || tr("session-sign-in-with")}</p>
                            {vm.with_value(|v| v.providers())
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
                                .collect_view()}
                        }
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}
