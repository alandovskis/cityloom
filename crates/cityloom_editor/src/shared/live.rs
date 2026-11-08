//! What the page says to a screen reader, through the status element the page
//! keeps for it.

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static SAID: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Says `text` in the page's `#live` status element. It is cleared first so the
/// same words said twice are heard twice.
#[cfg(target_arch = "wasm32")]
pub fn say(text: &str) {
    use std::cell::Cell;
    use std::time::Duration;

    use leptos::prelude::{document, set_timeout_with_handle};

    thread_local!(static PENDING: Cell<Option<leptos::leptos_dom::helpers::TimeoutHandle>> = const { Cell::new(None) });
    let Some(live) = document().get_element_by_id("live") else { return };
    if let Some(handle) = PENDING.with(|p| p.take()) {
        handle.clear();
    }
    live.set_text_content(Some(""));
    let text = text.to_string();
    let handle = set_timeout_with_handle(move || live.set_text_content(Some(&text)), Duration::from_millis(40)).ok();
    PENDING.with(|p| p.set(handle));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn say(text: &str) {
    SAID.with(|s| s.borrow_mut().push(text.to_string()));
}

/// What was said since this was last called, on the host.
#[cfg(test)]
pub fn take_said() -> Vec<String> {
    SAID.with(|s| std::mem::take(&mut *s.borrow_mut()))
}
