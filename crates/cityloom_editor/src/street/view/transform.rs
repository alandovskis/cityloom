//! The Transform dialog: the things that change the whole street at once, each previewed before it is applied.
//!
//! A button opens a modal `<dialog>` with three columns: the groups (a vertical tablist: Make pedestrian, Transit
//! priority, Street templates, each with how many choices it has), the open group's choices (a listbox; the transit
//! measures under one heading per Atlas family) and the preview of the choice pointed at or chosen (the street now and
//! after at one scale, and what changes). Nothing changes until Apply, which is one undoable step and hands the focus
//! to the drawing; Cancel, the close button and Escape hand it back to the button.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::atlas::name_key;
use crate::shared::catalogue::{KINDS, kind_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::symbols::icon;
use crate::street::model::{TemplateRefusal, View};
use crate::street::view::drawing;
use crate::street::view::iso::{IsoSvg, iso_pair, iso_svg};
use crate::street::view::notes::{SubHeading, impact_details};
use crate::street::vm::StreetVm;
use crate::street::vm::text::say;
use crate::street::vm::transform::{Choice, DialogState, Group, ListKey, Outcome, Preview, TransformChoices};
use crate::street::vm::watch::SheetWatch;

/// How wide the preview draws the street, in pixels; the picture is scaled to its column.
pub const TRANSFORM_PREVIEW_WIDTH: f64 = 360.0;

/// A word of the dialog, asked again when the language is switched.
fn word(w: SheetWatch, key: &'static str) -> impl Fn() -> String + Send + Sync + 'static {
    move || w.i18n().tr(key, &Args::new())
}

/// The end of the ids of a group's tab: `transform-tab-<id>`.
fn group_id(g: Group) -> &'static str {
    match g {
        Group::Pedestrian => "ped",
        Group::Transit => "transit",
        Group::Templates => "templates",
    }
}

/// The message naming a group, and the one saying what its choices do.
fn group_words(g: Group) -> (&'static str, &'static str) {
    match g {
        Group::Pedestrian => ("make-pedestrian-button", "make-pedestrian-hint"),
        Group::Transit => ("st-transit-heading", "transform-transit-hint"),
        Group::Templates => ("transform-templates-heading", "transform-templates-hint"),
    }
}

/// What a choice is called: a kind's name, a measure's code and name, a template's name.
fn choice_name(i18n: &I18n, c: Choice) -> String {
    match c {
        Choice::Pedestrian(k) => i18n.tr(&kind_key(KINDS[k].id), &Args::new()),
        Choice::Measure(code) => {
            let name = i18n.tr(&name_key(code), &Args::new());
            i18n.tr("transform-measure-title", &Args::new().str("code", code).str("name", name))
        }
        Choice::Template(t) => i18n.tr(t.name_key(), &Args::new()),
    }
}

/// The key of the list of choices that moves through it, if it is one.
pub fn list_key(key: &str) -> Option<ListKey> {
    match key {
        "ArrowUp" => Some(ListKey::Up),
        "ArrowDown" => Some(ListKey::Down),
        "Home" => Some(ListKey::Home),
        "End" => Some(ListKey::End),
        _ => None,
    }
}

/// The previews the dialog has made, kept until the street changes, so the live line, the drawing and Apply share
/// one: the street does not change while the dialog is open, but for a reload from another device, after which they
/// are made again.
#[derive(Clone, Copy)]
pub struct Previews {
    w: SheetWatch,
    kept: StoredValue<(Option<Rc<View>>, Vec<(Choice, Rc<Preview>)>), LocalStorage>,
}

impl Previews {
    pub fn new(w: SheetWatch) -> Previews {
        Previews { w, kept: StoredValue::new_local((None, Vec::new())) }
    }

    /// What `c` would do to the street as it is; a reader is drawn again when the street changes.
    pub fn of(&self, c: Choice) -> Rc<Preview> {
        let street = self.w.view();
        self.kept.update_value(|(at, kept)| {
            if !at.as_ref().is_some_and(|at| Rc::ptr_eq(at, &street)) {
                *at = Some(street);
                kept.clear();
            }
        });
        if let Some(p) = self.kept.with_value(|(_, kept)| kept.iter().find(|(k, _)| *k == c).map(|(_, p)| p.clone())) {
            return p;
        }
        let p = Rc::new(self.w.preview(c));
        self.kept.update_value(|(_, kept)| kept.push((c, p.clone())));
        p
    }

    /// Lets go of every preview, as the dialog closes.
    pub fn clear(&self) {
        self.kept.set_value((None, Vec::new()));
    }
}

/// Whether Apply can apply `chosen`: something is chosen and the edit does not refuse it.
pub fn can_apply(previews: Previews, chosen: Option<Choice>) -> bool {
    chosen.is_some_and(|c| previews.of(c).can_apply())
}

/// The dialog as it opens and as it is left when it closes: its first group with something in it, nothing chosen or
/// pointed, so the closed dialog holds no preview.
pub fn opening(choices: &TransformChoices) -> DialogState {
    DialogState::open(choices).unwrap_or(DialogState::of(Group::Templates))
}

/// A tab for each group: its name and how many choices it has; the open group's tab is the one the Tab key reaches.
fn tabs_markup(w: SheetWatch, choices: &TransformChoices, state: RwSignal<DialogState>) -> impl IntoView + use<> {
    choices
        .groups()
        .into_iter()
        .map(|g| {
            let (name, _) = group_words(g);
            let selected = move || state.with(|s| s.group == g);
            // How many choices it has; for the templates, how many of them fit.
            let (n, fit, all) = (choices.in_group(g).len(), choices.templates.iter().filter(|t| t.refusal.is_none()).count(), choices.templates.len());
            let n = move || match g {
                Group::Templates => w.i18n().tr("transform-templates-fit", &Args::new().num("fit", fit as i64).num("all", all as i64)),
                _ => n.to_string(),
            };
            view! {
                <button
                    type="button"
                    role="tab"
                    id=format!("transform-tab-{}", group_id(g))
                    class="transform-tab"
                    aria-selected=move || selected().to_string()
                    aria-controls="transform-panel"
                    tabindex=move || if selected() { "0" } else { "-1" }
                    on:click=move |_| state.update(|s| s.set_group(g))
                >
                    <span class="transform-tab-name">{word(w, name)}</span>
                    " "
                    <span class="count">{n}</span>
                </button>
            }
        })
        .collect_view()
}

/// One choice of the list: pointing at it previews it, a click chooses it. A template that does not fit is
/// `aria-disabled` (still reached by the arrows, still previewed) with its reason under its name; the option is named
/// by what it shows, its name and that reason (WCAG 2.5.3: the visible words are in the name, so a screen reader
/// reads the reason once).
fn option_markup(w: SheetWatch, c: Choice, refusal: Option<TemplateRefusal>, state: RwSignal<DialogState>) -> AnyView {
    let id = format!("transform-opt-{}", c.key());
    let selected = move || state.with(|s| s.chosen == Some(c)).to_string();
    let active = move || state.with(|s| s.active == Some(c));
    let point = move |_| state.update(|s| s.point(Some(c)));
    let choose = move |_| state.update(|s| s.choose(c));
    let name = move || choice_name(&w.i18n(), c);
    match c {
        Choice::Pedestrian(k) => view! {
            <div role="option" id=id class="transform-item ped-item" class:transform-active=active aria-selected=selected on:mouseenter=point on:click=choose>
                <span class="add-icon" aria-hidden="true" inner_html=icon(KINDS[k].id)></span>
                " "
                <b>{name}</b>
            </div>
        }
        .into_any(),
        Choice::Measure(code) => {
            let name = move || w.i18n().tr(&name_key(code), &Args::new());
            view! {
                <div role="option" id=id class="transform-item transit-item" class:transform-active=active aria-selected=selected on:mouseenter=point on:click=choose>
                    <span class="dirtag">{code}</span>
                    " "
                    <b>{name}</b>
                </div>
            }
            .into_any()
        }
        Choice::Template(t) => {
            let reason_id = format!("template-reason-{}", t.id());
            let disabled = refusal.is_some();
            let reason = refusal.map(|r| {
                let text = move || say(&w.i18n(), w.units(), &r.said(t));
                view! { <p class="template-reason" id=reason_id>{text}</p> }
            });
            view! {
                <div
                    role="option"
                    id=id
                    class="transform-item template-item"
                    class:template-disabled=disabled
                    class:transform-active=active
                    aria-selected=selected
                    aria-disabled=disabled.then_some("true")
                    data-template=t.id()
                    on:mouseenter=point
                    on:click=choose
                >
                    <b>{name}</b>
                    {reason}
                </div>
            }
            .into_any()
        }
    }
}

/// The choices of `group`, in the tab panel of the group's tab: its hint, then the list of them, labelled by the group's tab; the transit measures under
/// one heading per Atlas family (a group of the list named by its heading). The active choice (`DialogState::active`)
/// is the list's active descendant; taking the focus makes the first one active; leaving the list stops pointing.
pub fn choices_markup(w: SheetWatch, choices: &TransformChoices, group: Group, state: RwSignal<DialogState>) -> impl IntoView + use<> {
    let (_, hint) = group_words(group);
    let options = match group {
        Group::Pedestrian => choices.pedestrian.iter().map(|k| option_markup(w, Choice::Pedestrian(*k), None, state)).collect_view().into_any(),
        Group::Transit => choices
            .transit
            .iter()
            .map(|(family, codes)| {
                let heading = format!("transform-family-{family}-h");
                let labelled = heading.clone();
                let key = format!("transform-family-{family}");
                let items = codes.iter().map(|code| option_markup(w, Choice::Measure(code), None, state)).collect_view();
                view! {
                    <div role="group" aria-labelledby=labelled>
                        <p class="transform-family" id=heading aria-hidden="true">{move || w.i18n().tr(&key, &Args::new())}</p>
                        {items}
                    </div>
                }
            })
            .collect_view()
            .into_any(),
        Group::Templates => choices.templates.iter().map(|t| option_markup(w, Choice::Template(t.template), t.refusal, state)).collect_view().into_any(),
    };
    let focus_choices = choices.clone();
    let tab = format!("transform-tab-{}", group_id(group));
    let panel_tab = tab.clone();
    view! {
        <div role="tabpanel" id="transform-panel" class="transform-panel" aria-labelledby=panel_tab>
        <p class="hint" id="transform-hint">{word(w, hint)}</p>
        <div
            id="transform-choices"
            class="transform-list"
            role="listbox"
            tabindex="0"
            aria-labelledby=tab
            aria-describedby="transform-hint"
            aria-activedescendant=move || state.with(|s| s.active.map(|c| format!("transform-opt-{}", c.key())))
            on:focus=move |_| state.update(|s| s.focus(&focus_choices))
            on:mouseleave=move |_| state.update(|s| s.point(None))
        >
            {options}
        </div>
        </div>
    }
}

/// `markup` with every id it defines put after `prefix`, and its references to them (`url(#…)`, `href="#…"`) with
/// them, so that a drawing set beside another (the page's own) neither shadows the other's patterns and filters nor
/// is shadowed by them. References to ids it does not define (the page's hatches) are left alone.
pub fn scoped_ids(markup: &str, prefix: &str) -> String {
    let ids: Vec<&str> = markup.split(" id=\"").skip(1).filter_map(|rest| rest.split('"').next()).collect();
    let mut out = markup.to_string();
    for id in ids {
        out = out.replace(&format!(" id=\"{id}\""), &format!(" id=\"{prefix}{id}\""));
        out = out.replace(&format!("url(#{id})"), &format!("url(#{prefix}{id})"));
        out = out.replace(&format!("href=\"#{id}\""), &format!("href=\"#{prefix}{id}\""));
    }
    out
}

/// One drawing of the preview: its caption, and the street in 2.5D named for a screen reader, its ids after `prefix`.
fn figure(i18n: &I18n, caption: &'static str, prefix: &str, svg: IsoSvg, alt: String) -> impl IntoView {
    let markup = scoped_ids(&svg.markup, prefix);
    view! {
        <figure class="transform-side">
            <figcaption>{i18n.tr(caption, &Args::new())}</figcaption>
            <div class="transform-figure" role="img" aria-label=alt>
                <svg class="iso" xmlns="http://www.w3.org/2000/svg" viewBox=format!("0 0 {} {}", svg.width, svg.height) inner_html=markup></svg>
            </div>
        </figure>
    }
}

/// What the preview shows for `shown`: nothing yet, a line saying to choose; otherwise its name, the street now and
/// after at one scale, and what changes in words (with a template's report, where it has one); or, for a choice the
/// edit refuses, the street now and why.
pub fn preview_markup(w: SheetWatch, shown: Option<(Choice, Rc<Preview>)>) -> AnyView {
    let i18n = w.i18n();
    let Some((c, preview)) = shown else {
        return view! { <p class="transform-empty">{i18n.tr("transform-preview-empty", &Args::new())}</p> }.into_any();
    };
    let units = w.units();
    let Preview { before, outcome } = Preview::clone(&preview);
    let title = view! { <h3 id="transform-preview-h">{choice_name(&i18n, c)}</h3> };
    match outcome {
        Outcome::Changed { after, impact, report } => {
            let (now, then) = iso_pair(&before, &after, &i18n, TRANSFORM_PREVIEW_WIDTH);
            let report = report.filter(|r| !r.applied.is_empty() || !r.skipped.is_empty()).map(|r| {
                let items = r.applied.iter().chain(&r.skipped).map(|s| view! { <li>{say(&i18n, units, s)}</li> }).collect_view();
                view! { <ul class="transform-report">{items}</ul> }
            });
            view! {
                {title}
                <div class="transform-pair">
                    {figure(&i18n, "transform-now", "transform-now-", now, drawing::alt(&before, &i18n, units))}
                    {figure(&i18n, "transform-after", "transform-after-", then, drawing::alt(&after, &i18n, units))}
                </div>
                <h4>{i18n.tr("transform-what-changes", &Args::new())}</h4>
                {impact_details(&i18n, units, &impact, SubHeading::H5)}
                {report}
            }
            .into_any()
        }
        Outcome::Refused(said) => {
            let now = iso_svg(&before, &i18n, TRANSFORM_PREVIEW_WIDTH);
            view! {
                {title}
                <div class="transform-pair">{figure(&i18n, "transform-now", "transform-now-", now, drawing::alt(&before, &i18n, units))}</div>
                <p class="transform-no">{say(&i18n, units, &said)}</p>
            }
            .into_any()
        }
    }
}

/// What the preview's live line says of `shown`: its name, and what it would do in one sentence (the verdict on the
/// checks, or that it has no effect) or why it cannot be applied. Nothing while nothing is shown.
pub fn said_line(w: SheetWatch, shown: Option<(Choice, Rc<Preview>)>) -> Option<(String, String)> {
    let (c, preview) = shown?;
    let (i18n, units) = (w.i18n(), w.units());
    let outcome = match &preview.outcome {
        Outcome::Changed { impact, .. } if impact.is_empty() => i18n.tr("impact-no-effect", &Args::new()),
        Outcome::Changed { impact, .. } => say(&i18n, units, &impact.verdict_said()),
        Outcome::Refused(said) => say(&i18n, units, said),
    };
    Some((choice_name(&i18n, c), outcome))
}

/// Puts the keyboard on the element whose id is `id`, if the page has it.
fn focus_id(id: &str) {
    if let Some(el) = document().get_element_by_id(id) {
        let _ = leptos::wasm_bindgen::JsCast::unchecked_into::<leptos::web_sys::HtmlElement>(el).focus();
    }
}

/// Brings the element whose id is `id` into its scrolled list's view.
fn reveal_id(id: &str) {
    if let Some(el) = document().get_element_by_id(id) {
        let o = leptos::web_sys::ScrollIntoViewOptions::new();
        o.set_block(leptos::web_sys::ScrollLogicalPosition::Nearest);
        el.scroll_into_view_with_scroll_into_view_options(&o);
    }
}

/// The Transform button and its dialog (see the module's documentation). The button is shown while there is
/// something to offer, which is always, since the templates are always listed.
#[component]
pub fn TransformDialog(vm: Rc<StreetVm>) -> impl IntoView {
    use leptos::web_sys::KeyboardEvent;

    let w = SheetWatch::new(vm);
    let choices = Memo::new(move |_| w.transform_choices());
    // A street shown read-only cannot be transformed: the button is not offered (and the dialog never opens).
    let offered = Memo::new(move |_| !choices.with(|c| c.is_empty()) && !w.read_only());
    let state = RwSignal::new(opening(&choices.get_untracked()));
    let group = Memo::new(move |_| state.with(|s| s.group));
    let shown = Memo::new(move |_| state.with(|s| s.shown()));
    let chosen = Memo::new(move |_| state.with(|s| s.chosen));
    let previews = Previews::new(w);
    // Made once for the shown choice, shared by the live line and the drawing.
    let shown_preview = move || shown.get().map(|c| (c, previews.of(c)));
    let ok = Memo::new(move |_| can_apply(previews, chosen.get()));
    // A reload from another device can take away the open group or the chosen choice (`DialogState::reconcile`).
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        choices.with(|c| {
            let mut s = state.get_untracked();
            s.reconcile(c);
            if s != state.get_untracked() {
                state.set(s);
            }
        })
    });
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    // Whether the dialog closes on an Apply that changed the street: the focus then goes to the drawing.
    let applied = StoredValue::new(false);

    let open = move |_| {
        state.set(opening(&choices.get_untracked()));
        if let Some(d) = dialog.get_untracked() {
            let _ = d.show_modal();
        }
        // Once the list is drawn for the group that opens, so that it can take the focus.
        leptos::task::spawn_local(async move {
            leptos::task::tick().await;
            focus_id("transform-choices");
        });
    };
    let close = move || {
        if let Some(d) = dialog.get_untracked() {
            d.close();
        }
    };
    let apply = move || {
        let Some(c) = state.get_untracked().chosen else { return };
        if ok.get_untracked() {
            applied.set_value(w.apply_choice(c));
            close();
        }
    };
    // Every way the dialog closes (Apply, Cancel, the close button, Escape) ends here.
    let closed = move |_| {
        state.set(opening(&choices.get_untracked()));
        previews.clear();
        if applied.get_value() {
            applied.set_value(false);
            focus_id("wrap");
        } else {
            focus_id("transform-btn");
        }
    };
    let tab_keys = move |e: KeyboardEvent| {
        let Some(k) = list_key(&e.key()) else { return };
        e.prevent_default();
        choices.with_untracked(|c| state.update(|s| s.step_group(c, k)));
        focus_id(&format!("transform-tab-{}", group_id(state.get_untracked().group)));
    };
    // Enter applies the chosen choice (and chooses the active one when it is not); Space chooses the active one.
    let list_keys = move |e: KeyboardEvent| {
        if e.key() == "Enter" {
            e.prevent_default();
            let mut applies = false;
            state.update(|s| applies = s.press_enter());
            if applies {
                apply();
            }
            return;
        }
        if e.key() == " " {
            e.prevent_default();
            state.update(|s| s.choose_active());
            return;
        }
        let Some(k) = list_key(&e.key()) else { return };
        e.prevent_default();
        choices.with_untracked(|c| state.update(|s| s.step_choice(c, k)));
        if let Some(c) = state.get_untracked().chosen {
            reveal_id(&format!("transform-opt-{}", c.key()));
        }
    };

    view! {
        {move || {
            offered
                .get()
                .then(|| {
                    view! {
                        <button type="button" id="transform-btn" class="btn" aria-haspopup="dialog" on:click=open>
                            {word(w, "transform-button")}
                        </button>
                    }
                })
        }}
        <dialog id="transform-dialog" class="transform-dialog" aria-labelledby="transform-dialog-h" node_ref=dialog on:close=closed>
            <div class="transform-head">
                <h2 id="transform-dialog-h">{word(w, "transform-dialog-title")}</h2>
                <button type="button" id="transform-close" class="btn quiet transform-x" aria-label=word(w, "transform-close") on:click=move |_| close()>
                    <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
                        <path d="M4 4l8 8M12 4l-8 8"/>
                    </svg>
                </button>
            </div>
            <div class="transform-body">
                <div
                    id="transform-groups"
                    class="transform-groups"
                    role="tablist"
                    aria-orientation="vertical"
                    aria-labelledby="transform-dialog-h"
                    on:keydown=tab_keys
                >
                    {move || choices.with(|c| tabs_markup(w, c, state))}
                </div>
                <div class="transform-pick" on:keydown=list_keys>
                    {move || {
                        let g = group.get();
                        choices.with(|c| choices_markup(w, c, g, state))
                    }}
                </div>
                <section
                    id="transform-preview"
                    class="transform-preview"
                    aria-labelledby=move || shown.get().map(|_| "transform-preview-h")
                >
                    // The one live region: the shown choice's name and what it would do, not the figures and details,
                    // which are redrawn on every move.
                    <p id="transform-said" class="sr-only" aria-live="polite" aria-atomic="true">
                        {move || said_line(w, shown_preview()).map(|(name, outcome)| view! { <span>{name}</span> " " <span>{outcome}</span> })}
                    </p>
                    {move || preview_markup(w, shown_preview())}
                </section>
            </div>
            <div class="transform-foot">
                <button type="button" id="transform-cancel" class="btn" on:click=move |_| close()>
                    {word(w, "transform-cancel")}
                </button>
                <button type="button" id="transform-apply" class="btn primary" disabled=move || !ok.get() on:click=move |_| apply()>
                    {word(w, "transform-apply")}
                </button>
            </div>
        </dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arrows_home_and_end_move_through_a_list_and_other_keys_do_not() {
        assert_eq!(list_key("ArrowUp"), Some(ListKey::Up));
        assert_eq!(list_key("ArrowDown"), Some(ListKey::Down));
        assert_eq!(list_key("Home"), Some(ListKey::Home));
        assert_eq!(list_key("End"), Some(ListKey::End));
        assert_eq!(list_key("ArrowLeft"), None);
        assert_eq!(list_key("Enter"), None);
    }
}
