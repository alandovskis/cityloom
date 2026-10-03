//! What the page remembers in the browser's storage between visits. Storage can
//! be blocked, so a choice then lasts for this visit only.

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static KEPT: std::cell::RefCell<std::collections::HashMap<String, String>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Keeps `value` under `key`; says whether it was kept.
#[cfg(target_arch = "wasm32")]
pub fn remember(key: &str, value: &str) -> bool {
    leptos::web_sys::window().and_then(|w| w.local_storage().ok().flatten()).is_some_and(|s| s.set_item(key, value).is_ok())
}

#[cfg(target_arch = "wasm32")]
pub fn recall(key: &str) -> Option<String> {
    leptos::web_sys::window().and_then(|w| w.local_storage().ok().flatten()).and_then(|s| s.get_item(key).ok().flatten())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn remember(key: &str, value: &str) -> bool {
    KEPT.with(|k| k.borrow_mut().insert(key.to_string(), value.to_string()));
    true
}

#[cfg(not(target_arch = "wasm32"))]
pub fn recall(key: &str) -> Option<String> {
    KEPT.with(|k| k.borrow().get(key).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_remembered_is_recalled_and_what_is_not_is_not() {
        assert_eq!(recall("cityloom-test-nothing"), None);
        assert!(remember("cityloom-test-key", "seen"));
        assert_eq!(recall("cityloom-test-key").as_deref(), Some("seen"));
        assert!(remember("cityloom-test-key", "show"));
        assert_eq!(recall("cityloom-test-key").as_deref(), Some("show"));
    }
}
