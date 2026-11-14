//! The Profiles section: it binds to `StandardsVm` and decides nothing.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::catalogue::kind_key;
use crate::shared::i18n::Args;
use crate::standards::model::Field;
use crate::standards::text::{length_label, say};
use crate::standards::vm::{Form, FormView, Row, StandardsVm};

type Vm = StoredValue<Rc<StandardsVm>, LocalStorage>;

fn word(vm: Vm, key: &str) -> String {
    vm.with_value(|v| v.word(key))
}

fn named(vm: Vm, key: &str, name: &str) -> String {
    vm.with_value(|v| v.named(key, name))
}

/// A file as a `data:` address that a link can download, the text written as percent escapes.
pub fn data_address(text: &str) -> String {
    let mut out = String::from("data:application/json;charset=utf-8,");
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A name for the downloaded file: letters and digits of the profile's name, else `standards`.
pub fn file_name(name: &str) -> String {
    let slug: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    let slug = slug.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    format!("{}.json", if slug.is_empty() { "standards" } else { &slug })
}

fn list_row(vm: Vm, row: Row) -> impl IntoView {
    let (id, name) = (row.id.clone(), row.name.clone());
    let (edit_id, export_id, remove_id, ask_id) = (id.clone(), id.clone(), id.clone(), id.clone());
    let (n1, n2, n3) = (name.clone(), name.clone(), name.clone());
    let asking = {
        let id = id.clone();
        move || vm.with_value(|v| v.removing()).as_deref() == Some(id.as_str())
    };
    let question = name.clone();
    view! {
        <li class="standards-row" class:in-force=row.in_force>
            <span class="standards-name">{name.clone()}</span>
            {row.in_force.then(|| view! { <span class="standards-badge">{move || word(vm, "standards-in-force-label")}</span> })}
            <button type="button" class="secondary" id=format!("standards-edit-{id}") aria-label=move || named(vm, "standards-edit-named", &n1)
                on:click=move |_| vm.with_value(|v| v.begin_edit(&edit_id))>
                {move || word(vm, "standards-edit")}
            </button>
            <button type="button" class="secondary" id=format!("standards-export-{id}") aria-label=move || named(vm, "standards-export-named", &n2)
                on:click=move |_| { vm.with_value(|v| v.export(&export_id)); }>
                {move || word(vm, "standards-export")}
            </button>
            <button type="button" class="secondary" id=format!("standards-remove-{id}") aria-label=move || named(vm, "standards-remove-named", &n3)
                on:click=move |_| vm.with_value(|v| v.ask_remove(&ask_id))>
                {move || word(vm, "standards-remove")}
            </button>
            <Show when=asking>
                <div class="standards-ask" role="group" aria-label=question.clone()>
                    <p>{
                        let question = question.clone();
                        move || named(vm, "standards-remove-ask", &question)
                    }</p>
                    <button type="button" id="standards-remove-yes" on:click={
                        let remove_id = remove_id.clone();
                        move |_| { vm.with_value(|v| v.remove(&remove_id)); }
                    }>{move || word(vm, "standards-remove-confirm")}</button>
                    <button type="button" class="secondary" id="standards-remove-no" on:click=move |_| vm.with_value(|v| v.cancel_remove())>
                        {move || word(vm, "standards-cancel")}
                    </button>
                </div>
            </Show>
        </li>
    }
}

fn edit_form(vm: Vm, f: FormView) -> impl IntoView {
    let id = f.id.clone();
    let name_ref = NodeRef::<leptos::html::Input>::new();
    let length_refs: Vec<NodeRef<leptos::html::Input>> = f.lengths.iter().map(|_| NodeRef::new()).collect();
    let speeds_ref = NodeRef::<leptos::html::Input>::new();
    let target_refs: Vec<NodeRef<leptos::html::Input>> = f.targets.iter().map(|_| NodeRef::new()).collect();
    let (kind_ref, min_ref, max_ref) = (NodeRef::<leptos::html::Select>::new(), NodeRef::<leptos::html::Input>::new(), NodeRef::<leptos::html::Input>::new());
    let value = |r: &NodeRef<leptos::html::Input>| r.get().map(|i| i.value()).unwrap_or_default();
    let (save_id, width_id) = (id.clone(), id.clone());
    let (length_refs2, target_refs2) = (length_refs.clone(), target_refs.clone());
    let on_save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let form = Form {
            name: value(&name_ref),
            lengths: length_refs2.iter().map(value).collect(),
            speeds: value(&speeds_ref),
            targets: target_refs2.iter().map(value).collect(),
        };
        vm.with_value(|v| v.save(&save_id, &form));
    };
    let editing_name = f.name.clone();
    let lengths = f
        .lengths
        .iter()
        .zip(&length_refs)
        .map(|(l, r)| {
            let (field, hint, shown, r) = (l.field, l.builtin.clone(), l.value.clone(), *r);
            let input_id = format!("standards-{}", slug(field));
            let label_for = input_id.clone();
            view! {
                <div class="standards-field">
                    <label for=label_for>{move || vm.with_value(|v| length_label(&v.i18n(), v.units(), field))}</label>
                    <input type="text" inputmode="decimal" id=input_id node_ref=r value=shown/>
                    <span class="standards-hint">{move || vm.with_value(|v| say(&v.i18n(), v.units(), &hint))}</span>
                </div>
            }
        })
        .collect_view();
    let targets = f
        .targets
        .iter()
        .zip(&target_refs)
        .map(|(t, r)| {
            let (class, class_key, shown, r) = (t.class, t.class_key, t.value.clone(), *r);
            let input_id = format!("standards-target-{class}");
            let label_for = input_id.clone();
            view! {
                <div class="standards-field">
                    <label for=label_for>{move || vm.with_value(|v| {
                        let class = v.word(class_key);
                        v.i18n().tr("standards-target-label", &Args::new().str("class", class))
                    })}</label>
                    <input type="text" inputmode="numeric" id=input_id node_ref=r value=shown/>
                </div>
            }
        })
        .collect_view();
    let widths = f
        .widths
        .iter()
        .map(|w| {
            let (kind, said, clear_id) = (w.kind, w.said.clone(), id.clone());
            view! {
                <li>
                    <span>{move || vm.with_value(|v| say(&v.i18n(), v.units(), &said))}</span>
                    <button type="button" class="secondary" id=format!("standards-width-clear-{kind}")
                        on:click=move |_| { vm.with_value(|v| v.clear_widths(&clear_id, kind)); }>
                        {move || vm.with_value(|v| {
                            let k = v.word(&kind_key(kind));
                            v.i18n().tr("standards-width-clear", &Args::new().str("kind", k.to_lowercase()))
                        })}
                    </button>
                </li>
            }
        })
        .collect_view();
    let none = f.widths.is_empty();
    let warnings = f.warnings.clone();
    let has_warnings = !warnings.is_empty();
    let kinds = vm.with_value(|v| v.kinds());
    view! {
        <form class="standards-edit" id="standards-edit-form" aria-labelledby="standards-editing" on:submit=on_save>
            <h4 id="standards-editing">{move || named(vm, "standards-editing", &editing_name)}</h4>
            <div class="standards-field">
                <label for="standards-rename">{move || word(vm, "standards-rename-label")}</label>
                <input type="text" id="standards-rename" node_ref=name_ref value=f.name.clone() maxlength="60"/>
            </div>
            {lengths}
            <div class="standards-field">
                <label for="standards-speeds">{move || word(vm, "standards-speeds-label")}</label>
                <input type="text" id="standards-speeds" node_ref=speeds_ref value=f.speeds.clone()/>
            </div>
            {targets}
            <button type="submit" id="standards-save">{move || word(vm, "standards-save")}</button>
            <button type="button" class="secondary" id="standards-close" on:click=move |_| vm.with_value(|v| v.close_edit())>
                {move || word(vm, "standards-close")}
            </button>
            {has_warnings.then(|| view! {
                <div class="standards-warnings" id="standards-warnings">
                    <h5>{move || word(vm, "standards-warnings-heading")}</h5>
                    <ul>{warnings.iter().cloned().map(|w| view! { <li>{move || vm.with_value(|v| say(&v.i18n(), v.units(), &w))}</li> }).collect_view()}</ul>
                </div>
            })}
            <div class="standards-widths">
                <h5>{move || word(vm, "standards-width-heading")}</h5>
                {none.then(|| view! { <p>{move || word(vm, "standards-width-none")}</p> })}
                <ul id="standards-widths">{widths}</ul>
                <div class="standards-field">
                    <label for="standards-width-kind">{move || word(vm, "standards-width-kind-label")}</label>
                    <select id="standards-width-kind" node_ref=kind_ref>
                        {kinds.into_iter().map(|k| view! { <option value=k>{move || word(vm, &kind_key(k))}</option> }).collect_view()}
                    </select>
                </div>
                <div class="standards-field">
                    <label for="standards-width-min">{move || vm.with_value(|v| v.i18n().tr("standards-width-min-label", &Args::new().str("unit", v.units().word())))}</label>
                    <input type="text" inputmode="decimal" id="standards-width-min" node_ref=min_ref/>
                </div>
                <div class="standards-field">
                    <label for="standards-width-max">{move || vm.with_value(|v| v.i18n().tr("standards-width-max-label", &Args::new().str("unit", v.units().word())))}</label>
                    <input type="text" inputmode="decimal" id="standards-width-max" node_ref=max_ref/>
                </div>
                <button type="button" class="secondary" id="standards-width-set" on:click=move |_| {
                    let kind = kind_ref.get().map(|s| s.value()).unwrap_or_default();
                    vm.with_value(|v| v.set_widths(&width_id, &kind, &value(&min_ref), &value(&max_ref)));
                }>{move || word(vm, "standards-width-set")}</button>
            </div>
        </form>
    }
}

/// The id of a length figure's input.
fn slug(field: Field) -> &'static str {
    match field {
        Field::AccessLane => "access-lane",
        Field::CrossingClearance => "crossing-clearance",
        Field::DoorBuffer => "door-buffer",
        Field::PatioWalk => "patio-walk",
        Field::StopMin => "stop-min",
        Field::SpeedChoices => "speed-choices",
        Field::Width => "width",
        Field::TargetSpeed => "target-speed",
    }
}

fn exported_file(vm: Vm, name: String, text: String) -> impl IntoView {
    let href = data_address(&text);
    let download = file_name(&name);
    let label = name.clone();
    view! {
        <div class="standards-export" id="standards-export-file">
            <label for="standards-exported">{move || named(vm, "standards-exported-label", &label)}</label>
            <textarea id="standards-exported" readonly rows="8">{text}</textarea>
            <a id="standards-download" href=href download=download>{move || word(vm, "standards-download")}</a>
            <button type="button" class="secondary" id="standards-export-close" on:click=move |_| vm.with_value(|v| v.close_export())>
                {move || word(vm, "standards-close")}
            </button>
        </div>
    }
}

/// The Profiles section of the street page's Street tab.
#[component]
pub fn Profiles(vm: Rc<StandardsVm>) -> impl IntoView {
    let vm: Vm = StoredValue::new_local(vm);
    let create_ref = NodeRef::<leptos::html::Input>::new();
    let import_ref = NodeRef::<leptos::html::Textarea>::new();
    let select_ref = NodeRef::<leptos::html::Select>::new();
    let on_create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let name = create_ref.get().map(|i| i.value()).unwrap_or_default();
        if vm.with_value(|v| v.create(&name))
            && let Some(i) = create_ref.get()
        {
            i.set_value("");
        }
    };
    let on_import = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let text = import_ref.get().map(|i| i.value()).unwrap_or_default();
        if vm.with_value(|v| v.import(&text))
            && let Some(i) = import_ref.get()
        {
            i.set_value("");
        }
    };
    view! {
        <section class="standards" aria-labelledby="standards-heading">
            <h3 id="standards-heading">{move || word(vm, "standards-heading")}</h3>
            <p class="standards-intro">{move || word(vm, "standards-intro")}</p>
            <div class="standards-field">
                <label for="standards-in-force">{move || word(vm, "standards-in-force-label")}</label>
                <select
                    id="standards-in-force"
                    node_ref=select_ref
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        vm.with_value(|v| v.select((!value.is_empty()).then_some(value.as_str())));
                    }
                >
                    <option value="" selected=move || vm.with_value(|v| v.in_force()).is_none()>{move || word(vm, "standards-builtin")}</option>
                    {move || {
                        vm.with_value(|v| v.rows())
                            .into_iter()
                            .map(|r| view! { <option value=r.id.clone() selected=r.in_force>{r.name.clone()}</option> })
                            .collect_view()
                    }}
                </select>
            </div>
            <p id="standards-active" role="status">{move || vm.with_value(|v| v.in_force_text())}</p>
            <p id="standards-error" class="standards-error" role="alert" hidden=move || vm.with_value(|v| v.error()).is_none()>
                {move || vm.with_value(|v| v.error()).unwrap_or_default()}
            </p>
            <h4>{move || word(vm, "standards-profiles-label")}</h4>
            <ul id="standards-list" class="standards-list">
                {move || {
                    let rows = vm.with_value(|v| v.rows());
                    if rows.is_empty() {
                        view! { <li class="standards-none">{word(vm, "standards-none")}</li> }.into_any()
                    } else {
                        rows.into_iter().map(|r| list_row(vm, r)).collect_view().into_any()
                    }
                }}
            </ul>
            {move || vm.with_value(|v| v.exported()).map(|(name, text)| exported_file(vm, name, text))}
            {move || vm.with_value(|v| v.form()).map(|f| edit_form(vm, f))}
            <form class="standards-create" on:submit=on_create>
                <label for="standards-name">{move || word(vm, "standards-name-label")}</label>
                <input type="text" id="standards-name" node_ref=create_ref maxlength="60"/>
                <button type="submit" id="standards-create">{move || word(vm, "standards-create")}</button>
            </form>
            <form class="standards-import" on:submit=on_import>
                <label for="standards-import-text">{move || word(vm, "standards-import-label")}</label>
                <textarea id="standards-import-text" node_ref=import_ref rows="4"></textarea>
                <button type="submit" id="standards-import">{move || word(vm, "standards-import")}</button>
            </form>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_offered_as_a_data_address_and_named_after_its_profile() {
        assert_eq!(data_address("{\"a\": 1}"), "data:application/json;charset=utf-8,%7B%22a%22%3A%201%7D");
        assert_eq!(file_name("City of X (2026)"), "city-of-x-2026.json");
        assert_eq!(file_name("***"), "standards.json");
    }
}
