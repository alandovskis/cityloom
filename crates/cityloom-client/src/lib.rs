//! CityLoom wasm client: document state, WebGL2 renderer, camera, and the
//! `window.__cityloom` test seam that Playwright asserts against.
//!
//! Task 024 adds the wasm-bindgen surface; task 027 adds the renderer.

/// Name of the global the client installs itself on for E2E assertions.
///
/// Playwright reads `window.__cityloom.scene_state()` (WASM-006), so this
/// string is part of the test contract, not an implementation detail.
pub const GLOBAL_NAME: &str = "__cityloom";

#[cfg(test)]
mod tests {
    use super::GLOBAL_NAME;

    /// Skeleton smoke test. Replace with binding tests in task 024.
    #[test]
    fn global_name_is_dunder_prefixed() {
        assert!(std::hint::black_box(GLOBAL_NAME).starts_with("__"));
    }
}
