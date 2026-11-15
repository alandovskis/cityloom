//! The street page's notes: the shell's tray (`shell::tray`) with the street's four tabs and their panels. The
//! panels hold the empty slots `mount_street_page` fills next (the Last change card, the width and capacity tables,
//! the goals, the checks, the changes, where the street came from, the title block), so it is mounted first.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::{Args, I18n};
use crate::shell::tray::{Tray, TrayTab};
use crate::street::vm::StreetVm;

/// The street's tabs: Impact, Checks (with the count of failing checks after its name), Changes, Provenance.
const TABS: [TrayTab; 4] = [
    TrayTab { id: "t-space", panel: "p-space", label: "ui-tab-impact", badge: None },
    TrayTab { id: "t-checks", panel: "p-checks", label: "ui-tab-checks", badge: Some("checks-n-slot") },
    TrayTab { id: "t-changes", panel: "p-changes", label: "ui-tab-changes", badge: None },
    TrayTab { id: "t-provenance", panel: "p-provenance", label: "ui-tab-provenance", badge: None },
];

#[component]
pub fn StreetNotes(vm: Rc<StreetVm>) -> impl IntoView {
    let i18n: Rc<I18n> = vm.i18n().clone();
    let words = StoredValue::new_local(i18n.clone());
    let tr = move |key: &'static str| move || words.with_value(|i| i.tr(key, &Args::new()));
    view! {
        <Tray i18n=i18n tabs=TABS.to_vec() status=vec!["fit-slot", "fit-checks-slot"]>
            <div class="panel-tab" id="p-space" role="tabpanel" aria-labelledby="t-space">
                <section class="impact" id="impact-card" aria-labelledby="h-impact" aria-live="polite" aria-atomic="true">
                    <h2 id="h-impact" class="note-h">{tr("st-impact-heading")}</h2>
                    <div id="impact"></div>
                </section>
                <section aria-labelledby="h-space">
                    <h2 id="h-space" class="note-h">{tr("st-space-heading")}</h2>
                    <table class="nt" id="space"></table>
                </section>
                <section aria-labelledby="h-cap">
                    <h2 id="h-cap" class="note-h">
                        <span>{tr("st-people-moved")}</span>
                        " "
                        <span class="sample">{tr("st-sample-numbers")}</span>
                    </h2>
                    <table class="nt" id="cap"></table>
                </section>
                <p class="fine" id="time-note"></p>
            </div>
            <div class="panel-tab" id="p-checks" role="tabpanel" aria-labelledby="t-checks" hidden>
                <section aria-labelledby="h-goal">
                    <h2 id="h-goal" class="note-h">{tr("goal-heading")}</h2>
                    <div id="goal-slot"></div>
                </section>
                <section aria-labelledby="h-checks">
                    <h2 id="h-checks" class="note-h">
                        <span>{tr("st-does-it-work")}</span>
                        " "
                        <span class="sample">{tr("st-sample-limits")}</span>
                    </h2>
                    <ul class="checks" id="checks"></ul>
                </section>
                <section aria-labelledby="h-atlas">
                    <h2 id="h-atlas" class="note-h">{tr("st-transit-heading")}</h2>
                    <div id="atlas-source"></div>
                </section>
            </div>
            <div class="panel-tab" id="p-changes" role="tabpanel" aria-labelledby="t-changes" hidden>
                <section aria-labelledby="h-rev" class="revs">
                    <h2 id="h-rev" class="note-h">{tr("st-your-changes")}</h2>
                    <table class="nt rev" id="revs"></table>
                </section>
            </div>
            <div class="panel-tab" id="p-provenance" role="tabpanel" aria-labelledby="t-provenance" hidden>
                <section aria-labelledby="h-provenance">
                    <h2 id="h-provenance" class="note-h">{tr("st-provenance-heading")}</h2>
                    <div id="provenance"></div>
                </section>
            </div>
            <div class="title-block" id="title-block" aria-label=tr("st-about-street")></div>
        </Tray>
    }
}
