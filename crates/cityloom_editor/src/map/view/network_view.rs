//! The network checks of the map's details: a row per check with its count at the city's level (`NetworkChecks`), and
//! the check a row leads to at a level of its own (`NetworkCheckPane`): its findings, each saying why it failed and
//! naming the places it concerns, which a press shows in the details. The views show what the map's
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

/// The six checks, in a fixed order, one row each with its count; a row leads to the check's own level.
#[component]
pub fn NetworkChecks(vm: Rc<MapVm>) -> impl IntoView {
    let vm: Vm = Bound::new(vm);
    move || {
        let rows = vm.with(|v| v.network_rules()).into_iter().map(|rule| row(vm, rule)).collect_view();
        view! { <ul class="places-list net-rows">{rows}</ul> }
    }
}

/// Whether the person's goal bears on the check `id`, and the words that say so.
fn goal_mark(vm: Vm, id: &str) -> Option<String> {
    let marked = vm.with(|v| v.goal().get()).is_some_and(|g| g.bears_on_network(id));
    marked.then(|| vm.with(|v| v.word("goal-check-mark")))
}

fn row(vm: Vm, rule: NetworkCheck) -> impl IntoView {
    let mark = goal_mark(vm, rule.id);
    let (id, marked) = (rule.id, mark.is_some());
    view! {
        <li class="net-rule" class:goal=marked data-rule=id>
            <button type="button" class="place-row" id=format!("net-go-{id}") on:click=move |_| {
                vm.with(|v| v.show_check(id));
                super::details::focus_current();
            }>
                <b>{rule.label}</b>
                <small class="fig net-count" data-count=rule.count>{rule.count_text}</small>
                {mark.map(|m| view! { <span class="goal-mark">{m}</span> })}
            </button>
        </li>
    }
}

/// The check shown: its name and count, what the network checks are, its source, and its findings.
#[component]
pub fn NetworkCheckPane(vm: Rc<MapVm>) -> impl IntoView {
    let vm: Vm = Bound::new(vm);
    move || {
        let Some(rule) = vm.with(|v| v.shown_check()) else { return ().into_any() };
        let source = vm.with(|v| crate::shared::sources_view::source_note(rule.source, &rule.label, v.i18n(), v.units()));
        let mark = goal_mark(vm, rule.id);
        let items = rule.items.into_iter().map(|item| finding(vm, item)).collect_view();
        view! {
            <div class="insp-head">
                <div>
                    <h2 id=format!("net-h-{}", rule.id) class="insp-name">{rule.label}</h2>
                    <p class="insp-sub fig net-count">{rule.count_text}</p>
                </div>
            </div>
            {mark.map(|m| view! { <span class="goal-mark">{m}</span> })}
            <p class="hint">{vm.with(|v| v.word("net-note"))}</p>
            {source}
            <ul class="checks places-list net-findings" data-rule=rule.id data-count=rule.count>{items}</ul>
            {rule.more.map(|m| view! { <p class="hint net-more">{m}</p> })}
        }
        .into_any()
    }
}

fn finding(vm: Vm, item: NetworkItem) -> impl IntoView {
    let class = if item.severity == "problem" { "bad" } else { "" };
    let icon = if item.severity == "problem" {
        view! { <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 3l10 10M13 3 3 13"/></svg> }.into_any()
    } else {
        view! { <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="4.5" y="4.5" width="7" height="7"/></svg> }.into_any()
    };
    // A list of its own, so a screen reader says how many places the finding concerns. A place pressed is shown in the
    // details, as a press on the map shows it, from where Open opens it.
    let places = item
        .places
        .into_iter()
        .map(|p| {
            let hot = p.hot.clone();
            view! {
                <li>
                    <button type="button" class="net-link" data-hot=p.hot on:click=move |_| {
                        vm.with(|v| v.pick(Some(&hot)));
                        super::details::focus_current();
                    }>{p.name}</button>
                </li>
            }
        })
        .collect_view();
    let more = item.more_places.map(|m| view! { <li class="net-more-places">{m}</li> });
    view! {
        <li class=class data-severity=item.severity data-rule=item.rule>
            {icon}
            <div><b>{item.text}</b><span class="net-severity">{item.severity_word}</span><ul class="net-links">{places}{more}</ul></div>
        </li>
    }
}
