//! The piece list, under the drawing: every piece of the street as a cell of a WAI-ARIA listbox, left to right, with
//! its kind and its width. It is the way to a piece the picture hides (too narrow where it stands, or behind a taller
//! one) and the list of widths that a perspective cannot read off. One tab stop; the arrows, Home and End choose a
//! piece by the same call a press on the drawing makes.

use std::rc::Rc;

use leptos::prelude::*;

use super::keys::piece_target;
use super::text::piece_text;
use super::vm::StreetVm;
use super::watch::SheetWatch;
use crate::shared::catalogue::{KINDS, kind_key};
use crate::shared::i18n::Args;

/// The id of the cell of the piece `uid`.
pub fn cell_id(uid: u32) -> String {
    format!("piece-cell-{uid}")
}

#[component]
pub fn PieceList(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let heading = move || w.i18n().tr("st-pieces-heading", &Args::new());
    let label = move || w.i18n().tr("st-pieces-label", &Args::new());
    // The chosen cell, or else the first, so that the list has a place for the keyboard to land.
    let active = move || {
        let v = w.view();
        v.selected.or_else(|| v.segments.first().map(|s| s.uid)).map(cell_id)
    };
    let on_key = move |ev: leptos::ev::KeyboardEvent| {
        if ev.alt_key() || ev.ctrl_key() || ev.meta_key() {
            return;
        }
        let v = w.view_now();
        let current = v.selected.and_then(|u| v.segments.iter().position(|s| s.uid == u));
        if let Some(i) = piece_target(&ev.key(), current, v.segments.len()) {
            ev.prevent_default();
            w.select(Some(v.segments[i].uid));
        }
    };
    let cells = move || {
        let pieces: Vec<(u32, usize)> = w.view().segments.iter().map(|s| (s.uid, s.kind)).collect();
        pieces
            .into_iter()
            .enumerate()
            .map(|(i, (uid, kind))| {
                view! {
                    <div
                        class="piece-cell"
                        id=cell_id(uid)
                        role="option"
                        aria-selected=move || (w.view().selected == Some(uid)).to_string()
                        aria-label=move || {
                            let i18n = w.i18n();
                            let _ = i18n.locale();
                            piece_text(&w.view(), i, &i18n, w.units())
                        }
                        on:click=move |_| w.select(Some(uid))
                    >
                        <span class="piece-name">{move || w.i18n().tr(&kind_key(KINDS[kind].id), &Args::new())}</span>
                        <span class="piece-width">
                            {move || w.segment(uid, |g| w.units().length_fine_in(g.width_mm, w.i18n().locale())).unwrap_or_default()}
                        </span>
                    </div>
                }
            })
            .collect_view()
    };
    view! {
        <section class="piece-list-section" aria-labelledby="h-pieces">
            <h2 id="h-pieces" class="sr-only">{heading}</h2>
            <div
                id="piece-list"
                class="piece-list"
                role="listbox"
                aria-orientation="horizontal"
                tabindex="0"
                aria-label=label
                aria-activedescendant=active
                on:keydown=on_key
            >
                {cells}
            </div>
        </section>
    }
}
