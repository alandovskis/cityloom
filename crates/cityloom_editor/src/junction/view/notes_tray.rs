//! The junction page's notes: the shell's tray (`shell::tray`) with the junction's three tabs, each a function giving
//! its tab and its panel together, as the street page's (`street::view::notes_tray`). The panels hold the empty slots
//! `mount_page` fills next (getting across, the turns, where paths meet, the checks, the goal, the changes, the title
//! block), so it is mounted first.

use std::rc::Rc;

use leptos::prelude::*;

use crate::junction::vm::JunctionVm;
use crate::shared::i18n::I18n;
use crate::shell::tray::{Tray, TrayPage, TrayTab, Words, changes_page, checks_page, tr};

#[component]
pub fn JunctionNotes(vm: Rc<JunctionVm>) -> impl IntoView {
    let i18n: Rc<I18n> = vm.i18n().clone();
    let words = StoredValue::new_local(i18n.clone());
    let pages = vec![streets(words), checks(words), changes(words)];
    view! {
        <Tray i18n=i18n pages=pages status=vec!["fit-slot"]>
            <p class="fine">{tr(words, "jn-page-fine-print")}</p>
            <div class="title-block" id="title-block" aria-label=tr(words, "jn-page-about")></div>
        </Tray>
    }
}

/// Streets: how each street is crossed, the turns allowed from it, and where the paths through the junction meet.
fn streets(words: Words) -> TrayPage {
    let tab = TrayTab { id: "t-space", panel: "p-space", label: "jn-page-tab-streets", badge: None };
    TrayPage::new(
        tab,
        view! {
            <section aria-labelledby="h-across">
                <h2 id="h-across" class="note-h">{tr(words, "jn-page-across")}</h2>
                <table class="nt" id="across"></table>
            </section>
            <section aria-labelledby="h-turns">
                <h2 id="h-turns" class="note-h">{tr(words, "jn-page-turns")}</h2>
                <p class="hint">{tr(words, "jn-page-turns-hint")}</p>
                <table class="nt turns" id="turns"></table>
            </section>
            <section aria-labelledby="h-conf">
                <h2 id="h-conf" class="note-h">{tr(words, "jn-page-conflicts")}</h2>
                <table class="nt" id="conflicts"></table>
                <p class="fine" id="conflict-note"></p>
            </section>
        },
    )
}

/// Checks: whether the junction works.
fn checks(words: Words) -> TrayPage {
    checks_page(words, tr(words, "jn-page-does-it-work").into_any(), "src-atlas-junction-label", None)
}

/// Changes: the person's changes to the junction.
fn changes(words: Words) -> TrayPage {
    changes_page(words, "jn-page-your-changes")
}
