//! The street page's notes: the shell's tray (`shell::tray`) with the street's three tabs, each a function giving its
//! tab and its panel together. The panels hold the empty slots `mount_street_page` fills next (the Last change card,
//! the width and capacity tables, the goals, the checks, the changes, the title block), so it is mounted first.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::I18n;
use crate::shell::tray::{Tray, TrayPage, TrayTab, Words, changes_page, checks_page, tr};
use crate::street::vm::StreetVm;

#[component]
pub fn StreetNotes(vm: Rc<StreetVm>) -> impl IntoView {
    let i18n: Rc<I18n> = vm.i18n().clone();
    let words = StoredValue::new_local(i18n.clone());
    let pages = vec![impact(words), checks(words), changes(words)];
    view! {
        <Tray i18n=i18n pages=pages status=vec!["fit-slot", "fit-checks-slot"]>
            <div class="title-block" id="title-block" aria-label=tr(words, "st-about-street")></div>
        </Tray>
    }
}

/// Impact: one band of three parts side by side (stacked where narrow): the Last change card, the width drawn as bars
/// (the table read instead), People moved as one figure (the table read instead).
fn impact(words: Words) -> TrayPage {
    let tab = TrayTab { id: "t-space", panel: "p-space", label: "ui-tab-impact", badge: None };
    TrayPage::new(
        tab,
        view! {
            <div class="impact-band">
                <section class="impact" id="impact-card" aria-labelledby="h-impact" aria-live="polite" aria-atomic="true">
                    <h2 id="h-impact" class="note-h">{tr(words, "st-impact-heading")}</h2>
                    <div id="impact"></div>
                </section>
                <section class="impact-width" aria-labelledby="h-space">
                    <h2 id="h-space" class="note-h">{tr(words, "st-space-heading")}</h2>
                    <div id="space-bars"></div>
                    <table class="nt sr-only" id="space"></table>
                    <p class="fine" id="time-note"></p>
                </section>
                <section class="impact-people" aria-labelledby="h-cap">
                    <h2 id="h-cap" class="note-h">
                        <span>{tr(words, "st-people-moved")}</span>
                        " "
                        <span class="sample">{tr(words, "st-sample-numbers")}</span>
                    </h2>
                    <div id="cap-figure"></div>
                    <table class="nt sr-only" id="cap"></table>
                </section>
            </div>
        },
    )
}

/// Checks, with the count of failing checks after its name, and the sample limits said in its heading.
fn checks(words: Words) -> TrayPage {
    let heading = view! {
        <span>{tr(words, "st-does-it-work")}</span>
        " "
        <span class="sample">{tr(words, "st-sample-limits")}</span>
    };
    checks_page(words, heading.into_any(), "st-transit-heading", Some("checks-n-slot"))
}

/// Changes: the person's changes to the street.
fn changes(words: Words) -> TrayPage {
    changes_page(words, "st-your-changes")
}
