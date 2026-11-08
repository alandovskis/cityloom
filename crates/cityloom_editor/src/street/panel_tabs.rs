//! The tabs of the street page's left panel: Piece (the details of the selected piece and the tools that edit the
//! street's sections) and Street (what is set for the street as a whole, its speed limit). A WAI-ARIA tab list with one
//! tab stop: the arrows, Home and End choose a tab and move the focus to it, as the notes' tabs do. The panels are the
//! page's own markup (`#p-piece`, `#p-street`); this component says which of them shows.

use std::rc::Rc;

use leptos::prelude::*;

use super::keys::panel_tab_target;
use super::vm::{PanelTab, StreetVm};
use super::watch::SheetWatch;
use crate::shared::i18n::Args;

/// The id of a tab's button.
pub fn tab_id(tab: PanelTab) -> String {
    format!("t-{}", tab.id())
}

/// The id of a tab's panel.
pub fn panel_id(tab: PanelTab) -> String {
    format!("p-{}", tab.id())
}

#[component]
pub fn PanelTabs(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    // The panels are static markup of the page, not this component's: they are shown and hidden to follow the choice.
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let shown = w.panel_tab();
        let document = leptos::prelude::document();
        for tab in PanelTab::ALL {
            if let Some(panel) = document.get_element_by_id(&panel_id(tab)) {
                if tab == shown {
                    let _ = panel.remove_attribute("hidden");
                } else {
                    let _ = panel.set_attribute("hidden", "");
                }
            }
        }
    });
    let on_key = move |ev: leptos::ev::KeyboardEvent| {
        if ev.alt_key() || ev.ctrl_key() || ev.meta_key() {
            return;
        }
        let current = PanelTab::ALL.iter().position(|t| *t == w.panel_tab()).unwrap_or(0);
        if let Some(to) = panel_tab_target(&ev.key(), current, PanelTab::ALL.len()) {
            ev.prevent_default();
            let tab = PanelTab::ALL[to];
            w.show_panel_tab(tab);
            focus_tab(tab);
        }
    };
    view! {
        <div class="tabs" role="tablist" aria-label=move || w.i18n().tr("st-panel-tabs-label", &Args::new())>
            {PanelTab::ALL
                .into_iter()
                .map(|tab| {
                    view! {
                        <button
                            type="button"
                            role="tab"
                            class="tab"
                            id=tab_id(tab)
                            aria-controls=panel_id(tab)
                            aria-selected=move || (w.panel_tab() == tab).to_string()
                            tabindex=move || if w.panel_tab() == tab { "0" } else { "-1" }
                            on:click=move |_| w.show_panel_tab(tab)
                            on:keydown=on_key
                        >
                            {move || w.i18n().tr(
                                match tab {
                                    PanelTab::Piece => "st-panel-tab-piece",
                                    PanelTab::Street => "st-panel-tab-street",
                                },
                                &Args::new(),
                            )}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// Moves the focus to a tab's button, which the keys have just chosen.
fn focus_tab(tab: PanelTab) {
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::wasm_bindgen::JsCast;
        if let Some(button) = leptos::prelude::document().get_element_by_id(&tab_id(tab)) {
            if let Ok(button) = button.dyn_into::<leptos::web_sys::HtmlElement>() {
                let _ = button.focus();
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = tab;
}
