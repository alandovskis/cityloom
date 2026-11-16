//! The notes as a tray under a page's drawing (the street page): a handle that stays in view (the page's status
//! line, the notes' tabs, the toggle) over a body holding the tabs' panels. The tray only draws: the shell binds its
//! toggle (`]`, `data-notes`) and its tabs (`show_tab`, `tab_key`) as it binds any page's notes, by their ids, so a
//! page mounts its tray before `mount_shell`. Its styles are `web/tray.css`.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::{Args, I18n};

/// A tab of the notes: its id, the id of its panel, the message that names it, and the id of an empty slot after the
/// name for what the page adds to it (the street's count of failing checks).
#[derive(Clone, Copy, Debug)]
pub struct TrayTab {
    pub id: &'static str,
    pub panel: &'static str,
    pub label: &'static str,
    pub badge: Option<&'static str>,
}

/// The tray: `status` names the empty slots the page mounts its status line into, at the start of the handle; the
/// first of `tabs` is chosen until the shell shows the one remembered; `children` are the tabs' panels.
#[component]
pub fn Tray(i18n: Rc<I18n>, tabs: Vec<TrayTab>, status: Vec<&'static str>, children: Children) -> impl IntoView {
    let i18n = StoredValue::new_local(i18n);
    let tr = move |key: &'static str| move || i18n.with_value(|i| i.tr(key, &Args::new()));
    let slots = status.into_iter().map(|id| view! { <div id=id style="display: contents"></div> }).collect_view();
    let tabs = tabs
        .into_iter()
        .enumerate()
        .map(|(n, t)| {
            let first = n == 0;
            view! {
                <button
                    type="button"
                    role="tab"
                    class="tab"
                    id=t.id
                    aria-controls=t.panel
                    aria-selected=if first { "true" } else { "false" }
                    tabindex=(!first).then_some("-1")
                >
                    <span>{tr(t.label)}</span>
                    {t.badge.map(|id| view! { <span id=id style="display: contents"></span> })}
                </button>
            }
        })
        .collect_view();
    view! {
        <aside class="notes tray" id="notes" aria-label=tr("ui-notes")>
            <div class="tray-handle">
                <div class="fit-row">{slots}</div>
                <div class="tabs" role="tablist" aria-label=tr("ui-notes-sections")>
                    {tabs}
                </div>
                <button
                    type="button"
                    id="notes-toggle"
                    class="tray-toggle"
                    aria-controls="notes-body"
                    aria-expanded="true"
                    aria-keyshortcuts="]"
                    title=tr("ui-notes-toggle-title")
                >
                    <span>{tr("ui-notes")}</span>
                    <svg class="tray-chevron" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
                        <path d="M4 10l4-4 4 4"></path>
                    </svg>
                </button>
            </div>
            <div class="tray-body" id="notes-body">
                {children()}
            </div>
        </aside>
    }
}
