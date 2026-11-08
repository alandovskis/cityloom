//! The Provenance tab of the map page's Notes panel: where the imported OpenStreetMap data may be missing or wrong,
//! one heading per kind with its count, and under it the findings, each saying what looks doubtful, where the doubt
//! comes from and which design reading it could mislead, and linking the places it concerns. The views show what the
//! map's view-model says; nothing here decides anything.

use std::rc::Rc;

use leptos::prelude::*;

use crate::map::vm::{MapVm, SourceGroup, SourceItem};
use crate::shared::bind::Bound;

type Vm = Bound<MapVm>;

/// The line above the findings: how many there are in all.
#[component]
pub fn SourceLead(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    move || vm.with(|v| v.source_lead())
}

/// The kinds of finding, in a fixed order, each with its count and its findings.
#[component]
pub fn SourceFindings(vm: Rc<MapVm>) -> impl IntoView {
    let vm: Vm = Bound::new(vm);
    move || vm.with(|v| v.source_groups()).into_iter().map(group).collect_view()
}

fn group(g: SourceGroup) -> impl IntoView {
    let heading = format!("src-h-{}", g.id);
    let items = g.items.into_iter().map(finding).collect_view();
    // The name and the count are two spans with a hidden ": " between them, so a screen reader hears "Names:
    // 3 possible gaps" and the heading reads as two parts.
    view! {
        <div class="net-rule src-group" data-kind=g.id>
            <h3 id=heading class="net-h"><span>{g.label}</span><span class="sr-only">": "</span><span class="fig net-count">{g.count_text}</span></h3>
            <ul class="checks places-list" data-count=g.count>{items}</ul>
            {g.more.map(|m| view! { <p class="hint net-more">{m}</p> })}
        </div>
    }
}

fn finding(item: SourceItem) -> impl IntoView {
    // Not the mark of a failure: an open square, as a notice has, and no "bad" class.
    let places = item.places.into_iter().map(|p| view! { <li><a class="net-link" href=p.href data-hot=p.hot>{p.name}</a></li> }).collect_view();
    let more = item.more_places.map(|m| view! { <li class="net-more-places">{m}</li> });
    view! {
        <li data-kind=item.kind>
            <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="4.5" y="4.5" width="7" height="7"/></svg>
            <div>
                <b>{item.text}</b>
                <p class="src-line"><span class="src-label">{item.source_label}</span>" "{item.source}</p>
                <p class="src-line"><span class="src-label">{item.impact_label}</span>" "{item.impact}</p>
                <ul class="net-links">{places}{more}</ul>
            </div>
        </li>
    }
}
