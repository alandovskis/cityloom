# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

CityLoom is a street, junction and city-map editor written in Rust, compiled to WebAssembly and drawn with Leptos 0.8 (client-side rendering). The three pages are `web/map.html`, `web/index.html` (street) and `web/intersection.html` (junction). All catalogue widths, rules and rates are synthetic placeholders.

## Commands

```sh
cargo test                                   # everything runs on the host, no browser
cargo test vm::shell                         # one module; a full test name also works
cargo install wasm-bindgen-cli --version 0.2.129 --root .tools   # once; must match Cargo.toml
./scripts/build.sh                           # release wasm + wasm-bindgen into web/pkg (gitignored)
python3 -m http.server 8137 --directory web  # http://127.0.0.1:8137/
cargo build --target wasm32-unknown-unknown  # quick type-check of the wasm-only code paths
```

There is no linter or formatter configuration. `build.sh` strips the name and producers sections (without that the wasm is ~7.6 MB instead of ~2 MB). The `proc-macro-error2` future-incompat warning comes from Leptos dependencies and is harmless.

## Architecture

The layers are strict, and each layer only talks to the one below it:

- **Model** (`src/model.rs` street, `src/junction.rs` junction, `src/city.rs` the shared sample city, `*_view.rs` read models, `catalogue.rs`, `atlas.rs`, `units.rs`). It owns every editing rule. It has no DOM, no signals, and no I/O. Edits return `bool` and a refusal carries a reason.
- **View-models** (`src/vm/`): one per page (`map.rs`, `junction.rs`, `street.rs`) plus `shell.rs` for what every page shares (units, region, theme, settings menu, sidebars, notes tabs). They own the model, expose presentation-ready properties as reactive getters, and take commands. They reach the browser **only through the ports** in `ports.rs` (`Announcer`, `Storage`, `Scheduler`), so they are tested natively against the fakes `test_ports()` / `test_ports_with_time()`. The words said to a screen reader live in `junction_text.rs` and `street_text.rs`. `Core<M: Presents>` is the shared model-plus-version-signal-plus-cached-view; reading `view()` is tracked, `view_now()` is not (use it in commands).
- **Views** (`src/ui/`): Leptos components that bind to a view-model and forward events. They decide nothing. Rendering is by pure functions returning SVG strings (`plan_svg.rs`, `street_svg.rs`, `map_svg.rs`) set with `inner_html`; pointer and keyboard logic are pure state machines with tests. `ui/shell.rs` is the exception: it binds the static markup the pages share to `ShellVm` with web-sys listeners and effects.
- **Entry points** (`src/lib.rs`, `ui/mod.rs`): wasm-bindgen exports. A page script (`web/app.js`, `junction.js`, `map.js`, each tiny) only loads the module, sets up the hatch `<defs>`, and calls `mount_*` and `.mount_shell()`. `open_junction(id)` / `open_street(id)` open a place of the city from browser storage and return a view-model already bound to write back to it.

Persistence: a page opened from the map (`?junction=3`, `?street=7`) edits one place of the city. `CityBinding` writes it back through `CityStore` (against the city as stored *now*, so another tab's change is not overwritten) behind a 250 ms debounced `Keeper`; `flush()` is called on `pagehide`. Without the query parameter the page is a sandbox on the sample streets and keeps nothing. A write failure is said once.

## Conventions and gotchas

- Logic belongs in Rust; JS in `web/` stays start-up glue. Do not add logic there.
- Add view-model behaviour with a native test against the port fakes. Component tests render to HTML on the host (`ui/tests.rs`, Leptos `ssr` as a dev-dependency); `ui::platform::browser_ports()` also works natively (thread-local recorders), which is how those tests read what was announced.
- Non-`Send` values inside reactive closures go in `StoredValue::new_local`; views take a `Copy` handle (`Watch`, `SheetWatch`) over an `Rc<…Vm>`.
- `any_spawner::Executor::init_wasm_bindgen()` must run at the top of every `mount_*` function before any component creates an effect.
- `#![recursion_limit = "1024"]` is needed for the Leptos `view!` macros. New web-sys APIs need their feature added in `Cargo.toml`.
- Test-driven: write the failing test first. Commit each change separately once the tests pass.
- Checking in a real browser: a tab driven in the background throttles timers to ~1 s and pauses `requestAnimationFrame`, and `.focus()` does nothing (no document focus). Use microtask waits rather than timers, and expect focus-dependent behaviour to be unverifiable there. After rebuilding, bust the cache with `fetch(url, {cache: 'reload'})` for the wasm, the glue JS and the page script, then reload.
