//! The street page's details panel: which of its levels shows (the page's own markup, `#p-<level>`), and the path to
//! that level above it (`Street › Driving lane`). A step before the last is a button that shows its level;
//! the last is the level shown. The views show what the view-model says; nothing here decides which level that is.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::Args;
use crate::street::vm::level::Level;
use crate::street::vm::watch::SheetWatch;
use crate::street::vm::{Crumb, StreetVm};

/// The id of the step of the path that is the level shown, which takes the focus when a step is pressed.
pub const CURRENT_ID: &str = "panel-path-current";

/// The id of a level's part of the panel.
pub fn level_id(level: Level) -> String {
    format!("p-{}", level.id())
}

#[component]
pub fn PanelPath(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    // The levels are static markup of the page, not this component's: they are shown and hidden to follow the level.
    // A level hidden while it holds the focus gives the focus to the path, so it never stays on a control out of sight.
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let shown = w.level();
        let document = leptos::prelude::document();
        let focused = document.active_element();
        let mut lost = false;
        for level in Level::ALL {
            if let Some(part) = document.get_element_by_id(&level_id(level)) {
                if level == shown {
                    let _ = part.remove_attribute("hidden");
                } else {
                    lost |= focused.as_ref().is_some_and(|f| part.contains(Some(f)));
                    let _ = part.set_attribute("hidden", "");
                }
            }
        }
        if lost {
            focus_current();
        }
    });
    let step = move |c: Crumb| {
        if c.current {
            view! { <li><span id=CURRENT_ID class="path-here" aria-current="location" tabindex="-1">{c.label}</span></li> }.into_any()
        } else {
            let level = c.level;
            let on_press = move |_| {
                w.show_level(level);
                focus_current();
            };
            view! { <li><button type="button" class="path-step" data-level=level.id() on:click=on_press>{c.label}</button></li> }.into_any()
        }
    };
    view! {
        <nav class="panel-path" aria-label=move || w.i18n().tr("st-path-label", &Args::new())>
            <ol>{move || w.path().into_iter().map(step).collect_view()}</ol>
        </nav>
    }
}

/// Moves the focus to the step that is the level shown, once the path is drawn again: from there Tab enters the level.
fn focus_current() {
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::wasm_bindgen::JsCast;
        request_animation_frame(|| {
            if let Some(step) = leptos::prelude::document().get_element_by_id(CURRENT_ID) {
                if let Ok(step) = step.dyn_into::<leptos::web_sys::HtmlElement>() {
                    let _ = step.focus();
                }
            }
        });
    }
}
