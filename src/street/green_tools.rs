//! The commands beside the Green row: add a tree or planting (its kind, a tree's size, where along the street, the
//! piece it stands on and how far from that piece's middle, its length and width), change or remove the chosen one.
//! A field left blank takes its default when an element is added. With an element chosen the fields show it. One
//! named group, like the stops' commands; every control is a tab stop.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::{HtmlInputElement, HtmlSelectElement};

use super::curb_tools::typed_length;
use super::model::{GreenCell, GreenForm, GreenKind, TreeSize};
use super::vm::StreetVm;
use super::watch::SheetWatch;
use super::zone_tools::focus_later;
use crate::shared::i18n::Args;
use crate::shared::said::say;
use crate::shared::units::parse_number;

/// The sizes a tree can be, in the order the select lists them.
const SIZES: [(TreeSize, &str); 3] = [(TreeSize::S, "green-size-s"), (TreeSize::M, "green-size-m"), (TreeSize::L, "green-size-l")];

fn kind_index(k: GreenKind) -> usize {
    GreenKind::ALL.iter().position(|x| *x == k).unwrap_or(0)
}

fn size_index(s: Option<TreeSize>) -> usize {
    SIZES.iter().position(|(x, _)| Some(*x) == s).unwrap_or(1)
}

/// What the fields hold: what is picked and what is typed.
#[derive(Clone, Copy)]
struct Fields {
    kind: RwSignal<usize>,
    size: RwSignal<usize>,
    at: RwSignal<String>,
    host: RwSignal<String>,
    fine: RwSignal<String>,
    length: RwSignal<String>,
    width: RwSignal<String>,
}

impl Fields {
    fn new() -> Fields {
        Fields {
            kind: RwSignal::new(0),
            size: RwSignal::new(1),
            at: RwSignal::new(String::new()),
            host: RwSignal::new(String::new()),
            fine: RwSignal::new(String::new()),
            length: RwSignal::new(String::new()),
            width: RwSignal::new(String::new()),
        }
    }

    /// Shows element `c`.
    fn show(self, w: SheetWatch, c: &GreenCell) {
        let (units, locale) = (w.units_now(), w.i18n().locale_now());
        self.kind.set(kind_index(c.kind));
        self.size.set(size_index(c.size));
        self.at.set(units.number_in(c.from_mm, locale));
        self.host.set(c.host.map(|h| h.to_string()).unwrap_or_default());
        self.fine.set(fine_shown(c.fine_mm));
        self.length.set(units.number_in(c.to_mm - c.from_mm, locale));
        self.width.set(units.number_in(c.width_mm, locale));
    }

    /// What the fields ask for. `None`, with it said, when a field holds something that is not a number.
    fn form(self, w: SheetWatch) -> Option<GreenForm> {
        let field = |text: String| -> Option<Option<i32>> { if text.trim().is_empty() { Some(None) } else { typed_length(w, &text, None).map(Some) } };
        let kind = GreenKind::ALL.get(self.kind.get_untracked()).copied().unwrap_or(GreenKind::Tree);
        let at_mm = field(self.at.get_untracked())?;
        let hosts: Vec<usize> = w.hosts_at(at_mm.unwrap_or(0)).into_iter().map(|(i, _)| i).collect();
        let (host, fell_back) = effective_host(&hosts, &self.host.get_untracked());
        let fine_mm = if fell_back {
            self.fine.set(String::new());
            None
        } else {
            let typed = self.fine.get_untracked();
            if typed.trim().is_empty() {
                None
            } else {
                match fine_typed(&typed) {
                    Some(mm) => Some(mm),
                    None => {
                        w.say_bad_number();
                        return None;
                    }
                }
            }
        };
        Some(GreenForm {
            kind,
            size: (kind == GreenKind::Tree).then(|| SIZES[self.size.get_untracked().min(SIZES.len() - 1)].0),
            length_mm: field(self.length.get_untracked())?,
            width_mm: field(self.width.get_untracked())?,
            at_mm,
            host,
            fine_mm,
        })
    }
}

/// The piece the host select shows and the form acts on: the one held when the section at this place has it, else the
/// first that can hold an element. The second is true when a piece was held and the place does not have it, so that
/// the offset from that piece's middle means nothing here.
fn effective_host(hosts: &[usize], held: &str) -> (Option<usize>, bool) {
    match held.parse::<usize>().ok().filter(|h| hosts.contains(h)) {
        Some(h) => (Some(h), false),
        None => (hosts.first().copied(), !held.trim().is_empty()),
    }
}

/// What the across field shows for an offset from the middle of the piece: whole centimetres, whatever the page's
/// units, so that the value shown is close enough to the offset (within 5 mm) for the 100 mm grid to give it back.
pub(crate) fn fine_shown(fine_mm: i32) -> String {
    (f64::from(fine_mm) / 10.0).round().to_string()
}

/// The offset in millimetres that a number of centimetres typed in the across field means; `None` when it is not one.
pub(crate) fn fine_typed(text: &str) -> Option<i32> {
    parse_number(text).map(|cm| (cm * 10.0).round() as i32)
}

fn chosen(w: SheetWatch) -> Option<GreenCell> {
    let v = w.view();
    v.green.selected.and_then(|i| v.green.cells.get(i).cloned())
}

/// The control to give the focus back to after a command: the row, or the Add button when the row is gone.
fn focus_after(w: SheetWatch) {
    focus_later(if w.view_now().green.cells.is_empty() { "green-add" } else { "green-strip" });
}

#[component]
pub fn GreenTools(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let text = move |key: &'static str| w.i18n().tr(key, &Args::new());
    let unit_text = move |key: &'static str| w.i18n().tr(key, &Args::new().str("unit", w.units().word()));
    let f = Fields::new();
    if let Some(c) = w.view_now().green.selected.and_then(|i| w.view_now().green.cells.get(i).cloned()) {
        f.show(w, &c);
    }
    // The fields follow the chosen element, as it is chosen and as it changes; the person's typing stays otherwise.
    #[cfg(target_arch = "wasm32")]
    {
        let shown = Memo::new(move |_| chosen(w));
        Effect::new(move |_| {
            if let Some(c) = shown.get() {
                f.show(w, &c);
            }
        });
    }
    let can_add = move || w.view().green.can_add;
    let is_tree = move || f.kind.get() == kind_index(GreenKind::Tree);
    let none_chosen = move || chosen(w).is_none();
    let at_mm = move || {
        let typed = f.at.get();
        if typed.trim().is_empty() { 0 } else { crate::shared::units::parse_length_mm(&typed, w.units()).unwrap_or(0) }
    };
    let host_ids = move || w.hosts_at(at_mm()).into_iter().map(|(i, _)| i).collect::<Vec<_>>();
    let host_value = move || effective_host(&host_ids(), &f.host.get()).0.map(|h| h.to_string()).unwrap_or_default();
    // A piece held for another place is let go of, with the offset from its middle.
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let (host, fell_back) = effective_host(&host_ids(), &f.host.get());
        if fell_back {
            f.host.set(host.map(|h| h.to_string()).unwrap_or_default());
            f.fine.set(String::new());
        }
    });
    let kind_options = move || {
        GreenKind::ALL
            .iter()
            .enumerate()
            .map(|(n, k)| {
                let k = *k;
                view! { <option value=n.to_string()>{move || w.i18n().tr(k.name_key(), &Args::new())}</option> }
            })
            .collect_view()
    };
    let size_options = move || {
        SIZES
            .iter()
            .enumerate()
            .map(|(n, (_, key))| {
                let key = *key;
                view! { <option value=n.to_string()>{move || w.i18n().tr(key, &Args::new())}</option> }
            })
            .collect_view()
    };
    let host_options = move || {
        w.hosts_at(at_mm())
            .into_iter()
            .map(|(i, said)| {
                let name = move || say(&w.i18n(), w.units(), &said);
                view! { <option value=i.to_string()>{name}</option> }
            })
            .collect_view()
    };
    let add = move || {
        if let Some(form) = f.form(w)
            && w.add_green(form)
        {
            focus_after(w);
        }
    };
    let change = move || {
        if none_chosen() {
            w.change_green(&GreenForm::new(GreenKind::Tree));
        } else if let Some(form) = f.form(w)
            && w.change_green(&form)
        {
            focus_after(w);
        }
    };
    let on_enter = move |e: leptos::ev::KeyboardEvent| {
        if e.key() == "Enter" {
            e.prevent_default();
            if none_chosen() { add() } else { change() }
        }
    };
    let field = move |id: &'static str, key: &'static str, value: RwSignal<String>| {
        view! {
            <div class="zone-tool">
                <label for=id>{move || unit_text(key)}</label>
                <input
                    id=id
                    type="text"
                    inputmode="decimal"
                    autocomplete="off"
                    value=move || value.get()
                    prop:value=move || value.get()
                    on:input=move |e| value.set(event_target::<HtmlInputElement>(&e).value())
                    on:keydown=on_enter
                />
            </div>
        }
    };
    view! {
        <Show when=can_add>
            <section class="insp-sec" aria-labelledby="h-green-tools">
                <h3 class="note-h" id="h-green-tools">{move || text("st-green-tools")}</h3>
                <div class="zone-tools curb-tools" role="group" aria-label=move || text("st-green-tools")>
                    <div class="zone-tool">
                        <label for="green-kind">{move || text("st-green-kind")}</label>
                        <select
                            id="green-kind"
                            prop:value=move || f.kind.get().to_string()
                            on:change=move |e| f.kind.set(event_target::<HtmlSelectElement>(&e).value().parse().unwrap_or(0))
                        >
                            {kind_options}
                        </select>
                    </div>
                    <Show when=is_tree>
                        <div class="zone-tool">
                            <label for="green-size">{move || text("st-green-size")}</label>
                            <select
                                id="green-size"
                                prop:value=move || f.size.get().to_string()
                                on:change=move |e| f.size.set(event_target::<HtmlSelectElement>(&e).value().parse().unwrap_or(1))
                            >
                                {size_options}
                            </select>
                        </div>
                    </Show>
                    {field("green-at", "st-green-at", f.at)}
                    <div class="zone-tool">
                        <label for="green-host">{move || text("st-green-host")}</label>
                        <select
                            id="green-host"
                            prop:value=host_value
                            on:change=move |e| {
                                f.host.set(event_target::<HtmlSelectElement>(&e).value());
                                // The offset from the middle of the old piece means nothing on another: the new one takes its default.
                                f.fine.set(String::new());
                            }
                        >
                            {host_options}
                        </select>
                    </div>
                    {field("green-fine", "st-green-fine", f.fine)}
                    {field("green-length", "st-green-length", f.length)}
                    {field("green-width", "st-green-width", f.width)}
                    <div class="zone-tool">
                        <button type="button" id="green-add" on:click=move |_| add()>
                            {move || text("st-green-add")}
                        </button>
                        <button type="button" id="green-change" aria-disabled=move || none_chosen().then_some("true") on:click=move |_| change()>
                            {move || text("st-green-change")}
                        </button>
                        <button
                            type="button"
                            id="green-remove"
                            aria-disabled=move || none_chosen().then_some("true")
                            on:click=move |_| {
                                if w.remove_green() {
                                    focus_after(w);
                                }
                            }
                        >
                            {move || text("st-green-remove")}
                        </button>
                    </div>
                </div>
            </section>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::{effective_host, fine_shown, fine_typed};
    use crate::street::model::{SNAP_MM, snap};

    #[test]
    fn the_fine_field_is_in_whole_centimetres_and_the_value_shown_gives_the_same_offset_back() {
        assert_eq!((fine_shown(950), fine_shown(-300), fine_shown(0), fine_shown(1_625)), ("95".into(), "-30".into(), "0".into(), "163".into()));
        assert_eq!((fine_typed("95"), fine_typed(" -30 "), fine_typed("4,5"), fine_typed(""), fine_typed("x")), (Some(950), Some(-300), Some(45), None, None));
        // Any offset on the grid, on any piece (its middle need not be on the grid): the value shown, typed back, is the offset.
        for host_mm in [1_500, 1_750, 3_000, 3_300, 3_350, 3_450] {
            let middle = host_mm / 2;
            for offset in (0..=host_mm).step_by(SNAP_MM as usize) {
                let shown = fine_shown(offset - middle);
                assert_eq!(snap(middle + fine_typed(&shown).unwrap()), offset, "{host_mm} {offset}: {shown}");
            }
        }
    }

    #[test]
    fn the_host_shown_is_the_one_held_when_the_place_has_it_and_else_the_first() {
        let hosts = [0, 3];
        assert_eq!(effective_host(&hosts, "3"), (Some(3), false));
        assert_eq!(effective_host(&hosts, ""), (Some(0), false), "nothing chosen: the first, and a typed offset counts");
        assert_eq!(effective_host(&hosts, "5"), (Some(0), true), "a piece the section here does not have");
        assert_eq!(effective_host(&[], "3"), (None, true), "no piece can hold it here");
    }
}
