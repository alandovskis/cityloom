//! The notes as a tray under a page's drawing (the street and junction pages): a handle that stays in view (the page's status
//! line, the notes' tabs, the toggle) over a body holding the tabs' panels. A page gives each tab as a `TrayPage`, the
//! tab and what its panel holds together; the tray draws both from it, so a tab and its panel always name each other.
//! The two editors' Checks and Changes tabs are the same but for their headings (`checks_page`, `changes_page`). The
//! tray only draws: the shell binds its toggle (`]`, `data-notes`) and its tabs (`show_tab`, `tab_key`) as it binds any
//! page's notes, by their ids, so a page mounts its tray before `mount_shell`. Its styles are `web/tray.css`.

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

/// A message in the page's language, following a switch.
pub type Words = StoredValue<Rc<I18n>, LocalStorage>;

/// The message `key` in the page's language, following a switch.
pub fn tr(words: Words, key: &'static str) -> impl Fn() -> String + Copy + 'static {
    move || words.with_value(|i| i.tr(key, &Args::new()))
}

/// Checks: the checks headed by `heading`, what fails first; beside them a column for the goal and its measures, then
/// where the transit measures' names come from, headed by the message `atlas`. `badge` is the slot after the tab's
/// name (the street's count of failing checks).
pub fn checks_page(words: Words, heading: AnyView, atlas: &'static str, badge: Option<&'static str>) -> TrayPage {
    let tab = TrayTab { id: "t-checks", panel: "p-checks", label: "ui-tab-checks", badge };
    TrayPage::new(
        tab,
        view! {
            <div class="checks-split">
                <section aria-labelledby="h-checks" class="checks-main">
                    <h2 id="h-checks" class="note-h">{heading}</h2>
                    <ul class="checks" id="checks"></ul>
                    <div id="checks-fold-slot"></div>
                </section>
                <div class="checks-side">
                    <section aria-labelledby="h-goal">
                        <h2 id="h-goal" class="note-h">{tr(words, "goal-heading")}</h2>
                        <div id="goal-slot"></div>
                    </section>
                    <section aria-labelledby="h-atlas">
                        <h2 id="h-atlas" class="note-h">{tr(words, atlas)}</h2>
                        <div id="atlas-source"></div>
                    </section>
                </div>
            </div>
        },
    )
}

/// Changes: the person's changes, headed by the message `heading`.
pub fn changes_page(words: Words, heading: &'static str) -> TrayPage {
    let tab = TrayTab { id: "t-changes", panel: "p-changes", label: "ui-tab-changes", badge: None };
    TrayPage::new(
        tab,
        view! {
            <section aria-labelledby="h-rev" class="revs">
                <h2 id="h-rev" class="note-h">{tr(words, heading)}</h2>
                <ol class="rev-cards" id="revs" aria-labelledby="h-rev"></ol>
            </section>
        },
    )
}

/// One tab of the tray and what its panel holds.
pub struct TrayPage {
    tab: TrayTab,
    body: AnyView,
}

impl TrayPage {
    pub fn new(tab: TrayTab, body: impl IntoView + 'static) -> Self {
        Self { tab, body: body.into_any() }
    }
}

/// The tray: `status` names the empty slots the page mounts its status line into, at the start of the handle; the
/// first of `pages` is chosen (its panel shown, the others hidden) until the shell shows the one remembered;
/// `children` follow the panels in the body.
#[component]
pub fn Tray(i18n: Rc<I18n>, pages: Vec<TrayPage>, status: Vec<&'static str>, children: Children) -> impl IntoView {
    let i18n = StoredValue::new_local(i18n);
    let tr = move |key: &'static str| move || i18n.with_value(|i| i.tr(key, &Args::new()));
    let slots = status.into_iter().map(|id| view! { <div id=id style="display: contents"></div> }).collect_view();
    let tabs = pages
        .iter()
        .map(|p| p.tab)
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
    let panels = pages
        .into_iter()
        .enumerate()
        .map(|(n, p)| {
            view! {
                <div class="panel-tab" id=p.tab.panel role="tabpanel" aria-labelledby=p.tab.id hidden=n != 0>
                    {p.body}
                </div>
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
                {panels}
                {children()}
            </div>
        </aside>
    }
}
