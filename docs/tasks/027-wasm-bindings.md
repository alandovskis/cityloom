---
id: 027
title: "Expose the wasm-bindgen client surface"
depends_on: [006, 002]
features: [WASM-001, WASM-002, WASM-003, WASM-004, WASM-008, WASM-011, WASM-012]
status: todo
acceptance:
  - "just verify"
  - "wasm-pack test --headless --chrome crates/cityloom-client"
model_hint: sonnet
---

# Context
The client crate is a stub. This task gives it a document and a JS-facing API,
without any rendering, so the data path can be proven before pixels exist.

# Scope
- `init()` returning a handle and touching no DOM globals, so the module loads
  in a Web Worker.
- `load_city_json(s)` and `city_json()` round-tripping the API's payload.
- Malformed input returns a JS error with a readable message and leaves prior
  state intact.
- A panic hook so Rust panics surface as JS exceptions carrying their message.
- Keep the release bundle under 3 MB gzipped; assert it in the build.

# Out of scope
- `scene_state()`. Task 028.
- `apply_op`. Task 029.
- WebGL. Task 030.

# Acceptance criteria (beyond the acceptance commands)
- A round-trip through `load_city_json` then `city_json` is asserted equal on a
  fixture city with shared cross-sections.
- A deliberate panic is asserted to reach JS with its message, not
  `unreachable`.
- The gzipped size assertion fails the build if exceeded, rather than warning.
