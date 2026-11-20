//! The editing controls of a page that shows its project read-only (#222): offered, so the page still shows what a
//! piece or a place is, and disabled.

use leptos::prelude::*;

/// `controls`, editing controls. On a page that shows its project read-only (`read_only`) they are inside a disabled
/// fieldset: each button, field and choice is greyed, takes no press and no focus, and is said to be unavailable. The
/// fieldset takes no room of its own (`.read-only-controls`, `web/app.css`). On any other page the controls are as they always
/// are, with nothing around them.
pub fn locked(read_only: bool, controls: impl IntoView + 'static) -> AnyView {
    if read_only { view! { <fieldset class="read-only-controls" disabled>{controls}</fieldset> }.into_any() } else { controls.into_any() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::testing::html;

    #[test]
    fn read_only_the_controls_are_in_a_disabled_fieldset_and_otherwise_as_they_are() {
        let shown = |read_only: bool| html(move || locked(read_only, view! { <button type="button">"Go"</button> }));
        let closed = shown(true);
        assert!(closed.starts_with(r#"<fieldset disabled class="read-only-controls""#), "{closed}");
        assert!(closed.contains("<button"), "{closed}");
        let open = shown(false);
        assert!(!open.contains("fieldset"), "{open}");
        assert!(open.contains("<button"), "{open}");
    }
}
