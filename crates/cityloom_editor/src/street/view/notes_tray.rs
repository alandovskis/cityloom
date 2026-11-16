//! The street page's notes: the shell's tray (`shell::tray`) with the street's four tabs, each a function giving its
//! tab and its panel together. The panels hold the empty slots `mount_street_page` fills next (the Last change card,
//! the width and capacity tables, the goals, the checks, the changes, where the street came from, the title block), so
//! it is mounted first.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::{Args, I18n};
use crate::shell::tray::{Tray, TrayPage, TrayTab};
use crate::street::vm::StreetVm;

/// A message in the page's language, following a switch.
type Words = StoredValue<Rc<I18n>, LocalStorage>;

fn tr(words: Words, key: &'static str) -> impl Fn() -> String + Copy + 'static {
    move || words.with_value(|i| i.tr(key, &Args::new()))
}

#[component]
pub fn StreetNotes(vm: Rc<StreetVm>) -> impl IntoView {
    let i18n: Rc<I18n> = vm.i18n().clone();
    let words = StoredValue::new_local(i18n.clone());
    let pages = vec![impact(words), checks(words), changes(words), provenance(words)];
    view! {
        <Tray i18n=i18n pages=pages status=vec!["fit-slot", "fit-checks-slot"]>
            <div class="title-block" id="title-block" aria-label=tr(words, "st-about-street")></div>
        </Tray>
    }
}

/// Impact: the Last change card, the width table, People moved.
fn impact(words: Words) -> TrayPage {
    let tab = TrayTab { id: "t-space", panel: "p-space", label: "ui-tab-impact", badge: None };
    TrayPage::new(
        tab,
        view! {
            <section class="impact" id="impact-card" aria-labelledby="h-impact" aria-live="polite" aria-atomic="true">
                <h2 id="h-impact" class="note-h">{tr(words, "st-impact-heading")}</h2>
                <div id="impact"></div>
            </section>
            <section aria-labelledby="h-space">
                <h2 id="h-space" class="note-h">{tr(words, "st-space-heading")}</h2>
                <table class="nt" id="space"></table>
            </section>
            <section aria-labelledby="h-cap">
                <h2 id="h-cap" class="note-h">
                    <span>{tr(words, "st-people-moved")}</span>
                    " "
                    <span class="sample">{tr(words, "st-sample-numbers")}</span>
                </h2>
                <table class="nt" id="cap"></table>
            </section>
            <p class="fine" id="time-note"></p>
        },
    )
}

/// Checks, with the count of failing checks after its name: the goal, the checks, the transit source.
fn checks(words: Words) -> TrayPage {
    let tab = TrayTab { id: "t-checks", panel: "p-checks", label: "ui-tab-checks", badge: Some("checks-n-slot") };
    TrayPage::new(
        tab,
        view! {
            <section aria-labelledby="h-goal">
                <h2 id="h-goal" class="note-h">{tr(words, "goal-heading")}</h2>
                <div id="goal-slot"></div>
            </section>
            <section aria-labelledby="h-checks">
                <h2 id="h-checks" class="note-h">
                    <span>{tr(words, "st-does-it-work")}</span>
                    " "
                    <span class="sample">{tr(words, "st-sample-limits")}</span>
                </h2>
                <ul class="checks" id="checks"></ul>
            </section>
            <section aria-labelledby="h-atlas">
                <h2 id="h-atlas" class="note-h">{tr(words, "st-transit-heading")}</h2>
                <div id="atlas-source"></div>
            </section>
        },
    )
}

/// Changes: the person's changes to the street.
fn changes(words: Words) -> TrayPage {
    let tab = TrayTab { id: "t-changes", panel: "p-changes", label: "ui-tab-changes", badge: None };
    TrayPage::new(
        tab,
        view! {
            <section aria-labelledby="h-rev" class="revs">
                <h2 id="h-rev" class="note-h">{tr(words, "st-your-changes")}</h2>
                <table class="nt rev" id="revs"></table>
            </section>
        },
    )
}

/// Provenance: where the street came from.
fn provenance(words: Words) -> TrayPage {
    let tab = TrayTab { id: "t-provenance", panel: "p-provenance", label: "ui-tab-provenance", badge: None };
    TrayPage::new(
        tab,
        view! {
            <section aria-labelledby="h-provenance">
                <h2 id="h-provenance" class="note-h">{tr(words, "st-provenance-heading")}</h2>
                <div id="provenance"></div>
            </section>
        },
    )
}
