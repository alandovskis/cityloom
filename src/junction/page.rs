//! The rest of the junction page around the plan: its heading, the status
//! line, undo and redo and the key to the strips.

use std::rc::Rc;

use leptos::prelude::*;

use crate::junction::read_model::JView;
use crate::junction::text::{arms_text, fit_text};
use crate::junction::vm::JunctionVm;
use crate::junction::watch::Watch;
use crate::shared::catalogue::{KINDS, kind_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::provenance::Sources;
use crate::shared::said::{Said, say};
use crate::shared::tick::Tick;
use crate::shared::units::Units;

/// The kinds of piece that appear in the plan, in the order the catalogue lists them.
pub fn key_kinds(v: &JView) -> Vec<usize> {
    let mut kinds: Vec<usize> = v.arms.iter().flat_map(|a| a.pieces.iter().map(|p| p.kind)).collect();
    kinds.sort_unstable();
    kinds.dedup();
    kinds
}

/// The tint and hatch of a kind of piece, in a box of `width` by 22.
fn swatch_rects(id: &str, x: f64, width: f64) -> impl IntoView {
    view! {
        <rect class=format!("k-{id}") x=x width=width height="22" stroke="none"/>
        <rect x=x width=width height="22" fill=format!("url(#h-{id})") stroke="none"/>
    }
}

#[component]
pub fn Header(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    let name = move || w.say(&w.view().name);
    let linked = move || w.view().linked;
    let arms = move || arms_text(&w.view(), &w.i18n());
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        if w.view().linked {
            document().set_title(&w.i18n().tr("jn-title", &Args::new().str("name", w.say(&w.view().name))));
        }
    });
    view! {
        <h1 id="street-name">{name}</h1>
        <p class="street-sub" id="street-sub">
            {move || linked().then(|| view! {
                <a class="back" href="map.html">
                    <svg viewBox="0 0 14 14" width="14" height="14" aria-hidden="true" focusable="false"><path d="M12 7H2M6 3 2 7l4 4"/></svg>
                    {move || w.tr("jn-city-map")}
                </a>
                " "
                <span aria-hidden="true">"·"</span>
                " "
            })}
            <span>{move || w.tr("jn-plan")}</span>
            " "
            <span aria-hidden="true">"·"</span>
            " "
            <span><b id="arm-count" class="fig">{arms}</b></span>
        </p>
    }
}

#[component]
pub fn TitleBlock(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    view! {
        <div class="tb-cell tb-wide"><span>{move || w.tr("jn-block-junction")}</span><b id="tb-street">{move || w.say(&w.view().name)}</b></div>
        <div class="tb-cell"><span>{move || w.tr("jn-block-streets")}</span><b id="tb-row" class="fig">{move || w.view().arms.len().to_string()}</b></div>
        <div class="tb-cell"><span>{move || w.tr("jn-block-changes")}</span><b id="tb-changes" class="fig">{move || w.view().revisions.len().to_string()}</b></div>
        {move || {
            let source = w.view().source.clone();
            (!source.is_empty())
                .then(|| {
                    view! {
                        <div class="tb-cell tb-full"><span>{move || w.tr("jn-block-osm")}</span><b class="tb-src"><Sources kind="node" refs=source/></b></div>
                        <div class="tb-cell tb-full">
                            <span>{move || w.tr("jn-block-data")}</span>
                            <b id="tb-state">{move || w.tr(if w.view().changed { "jn-block-edited" } else { "jn-block-imported" })}</b>
                        </div>
                    }
                })
        }}
    }
}

/// The status line: whether the junction works.
#[component]
pub fn Fit(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    let works = Signal::derive(move || w.view().checks.iter().all(|c| c.ok));
    view! {
        <p id="fit" class=move || if works.get() { "fit" } else { "fit bad" } role="status">
            <Tick works=works/>
            {move || fit_text(&w.view(), &w.i18n())}
        </p>
    }
}

#[component]
pub fn History(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    view! {
        <button type="button" id="undo" class="btn" disabled=move || !w.view().can_undo on:click=move |_| { w.undo(); }>
            <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M5.5 3 2.5 6l3 3M2.5 6H10a3.5 3.5 0 0 1 0 7H6"/></svg>
            {move || w.tr("jn-undo")}
        </button>
        <button type="button" id="redo" class="btn" disabled=move || !w.view().can_redo on:click=move |_| { w.redo(); }>
            <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M10.5 3l3 3-3 3M13.5 6H6a3.5 3.5 0 0 0 0 7h4"/></svg>
            {move || w.tr("jn-redo")}
        </button>
        <button type="button" id="reset" class="btn" disabled=move || !w.view().changed on:click=move |_| { w.reset(); }>
            <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M2.5 8a5.5 5.5 0 1 0 1.8-4.1M2.5 2.5v3h3"/></svg>
            {move || w.tr("jn-reset")}
        </button>
    }
}

/// A key to the strips: the tint and hatch of each piece that appears in the plan.
#[component]
pub fn Key(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    move || {
        key_kinds(&w.view())
            .into_iter()
            .map(|k| {
                view! {
                    <li>
                        <svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false">{swatch_rects(KINDS[k].id, 0.0, 44.0)}</svg>
                        {w.tr(&kind_key(KINDS[k].id))}
                    </li>
                }
            })
            .collect_view()
    }
}

/// The window title and the heading of the page of a junction that cannot be drawn, in the language of the
/// page. It asks with the tracked `tr` and `say`, so a view that calls it follows a switch of language.
pub fn stuck_words(i18n: &I18n, name: &Said) -> (String, String) {
    // A junction's name holds no length, so the units do not matter.
    let name = say(i18n, Units::default(), name);
    (i18n.tr("jn-title", &Args::new().str("name", name.clone())), name)
}

/// The heading of the page of a junction that cannot be drawn: its name, which also names the window.
#[component]
pub fn StuckHeader(i18n: Rc<I18n>, name: Said) -> impl IntoView {
    let held = StoredValue::new_local((i18n, name));
    let words = move || held.with_value(|(i18n, name)| stuck_words(i18n, name));
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| document().set_title(&words().0));
    view! { <h1 id="street-name">{move || words().1}</h1> }
}

/// What the page of a junction says when the streets that meet there no longer make a junction.
#[component]
pub fn Stuck(i18n: Rc<I18n>) -> impl IntoView {
    let held = StoredValue::new_local(i18n);
    let tr = move |key: &'static str| move || held.with_value(|i18n| i18n.tr(key, &Args::new()));
    view! {
        <div class="stuck">
            <h2 class="note-h">{tr("jn-stuck-title")}</h2>
            <p>{tr("jn-stuck-text")}</p>
            <p><a class="back" href="map.html">{tr("jn-city-map")}</a></p>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::*;

    #[test]
    fn the_key_lists_each_piece_in_the_plan_once_in_catalogue_order() {
        let kinds = key_kinds(&Junction::new(0).view());
        assert!(kinds.windows(2).all(|w| w[0] < w[1]));
        assert!(!kinds.is_empty());
        let names: Vec<&str> = kinds.iter().map(|k| KINDS[*k].id).collect();
        assert!(names.contains(&"sidewalk") && names.contains(&"travel"));
    }

    #[test]
    fn a_junction_without_a_bike_lane_does_not_key_one() {
        let names: Vec<&str> = key_kinds(&Junction::new(0).view()).iter().map(|k| KINDS[*k].id).collect();
        assert!(!names.contains(&"bikeshare"));
    }
}
