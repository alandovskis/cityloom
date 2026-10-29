//! The Network checks section of the map page's left panel: one heading per check with its count, and under it
//! the findings, each saying why it failed and linking the places it concerns. The views show what the map's
//! view-model says; nothing here decides anything.

use std::rc::Rc;

use leptos::prelude::*;

use crate::map::vm::{MapVm, NetworkCheck, NetworkItem};
use crate::shared::bind::Bound;

type Vm = Bound<MapVm>;

/// The line above the checks: how many findings there are in all.
#[component]
pub fn NetworkLead(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    move || vm.with(|v| v.network_lead())
}

/// The five checks, in a fixed order, each with its count and its findings.
#[component]
pub fn NetworkChecks(vm: Rc<MapVm>) -> impl IntoView {
    let vm: Vm = Bound::new(vm);
    move || vm.with(|v| v.network_rules()).into_iter().map(check).collect_view()
}

fn check(rule: NetworkCheck) -> impl IntoView {
    let heading = format!("net-h-{}", rule.id);
    let items = rule.items.into_iter().map(finding).collect_view();
    // The name and the count are two spans with a hidden ": " between them, so a screen reader hears "Cycling links:
    // 3 findings" and the heading reads as two parts.
    view! {
        <div class="net-rule" data-rule=rule.id>
            <h3 id=heading class="net-h"><span>{rule.label}</span><span class="sr-only">": "</span><span class="fig net-count">{rule.count_text}</span></h3>
            <ul class="checks places-list" data-count=rule.count>{items}</ul>
            {rule.more.map(|m| view! { <p class="hint net-more">{m}</p> })}
        </div>
    }
}

fn finding(item: NetworkItem) -> impl IntoView {
    let class = if item.severity == "problem" { "bad" } else { "" };
    let icon = if item.severity == "problem" {
        view! { <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 3l10 10M13 3 3 13"/></svg> }.into_any()
    } else {
        view! { <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="4.5" y="4.5" width="7" height="7"/></svg> }.into_any()
    };
    // A list of its own, so a screen reader says how many places the finding concerns.
    let places = item.places.into_iter().map(|p| view! { <li><a class="net-link" href=p.href data-hot=p.hot>{p.name}</a></li> }).collect_view();
    let more = item.more_places.map(|m| view! { <li class="net-more-places">{m}</li> });
    view! {
        <li class=class data-severity=item.severity data-rule=item.rule>
            {icon}
            <div><b>{item.text}</b><span class="net-severity">{item.severity_word}</span><ul class="net-links">{places}{more}</ul></div>
        </li>
    }
}
