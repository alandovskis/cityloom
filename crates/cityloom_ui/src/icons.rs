//! The marks the controls draw.

use leptos::prelude::*;

/// The tick of an option that is chosen.
pub fn tick() -> AnyView {
    view! { <svg class="tick" viewBox="0 0 16 16" aria-hidden="true"><path d="M2.5 8.5 6.5 12.5 13.5 3.5"/></svg> }.into_any()
}
