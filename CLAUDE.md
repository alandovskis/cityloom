# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

CityLoom is a street, junction and city-map editor written in Rust, compiled to WebAssembly and drawn with Leptos 0.8 (client-side rendering). The three pages are `web/map.html`, `web/street.html` (street) and `web/intersection.html` (junction). All catalogue widths, rules and rates are synthetic placeholders.

## Commands

`just` lists the recipes. The ones used most:

```sh
just test            # cargo test + `cargo build --target wasm32-unknown-unknown` (cfg(target_arch = "wasm32") code is not compiled by plain `cargo test`)
cargo test shell::vm # one module; a full test name also works
just build           # release wasm + wasm-bindgen into web/pkg (gitignored); scripts/build.sh does the work
just e2e             # builds, then runs the Playwright tests in e2e/; `just e2e tests/street.spec.ts` for one spec
just check           # everything
just format          # rustfmt (rustfmt.toml) + Prettier; `just format-check` only checks, as CI does
just serve           # http://127.0.0.1:8137/
just setup           # once: wasm-bindgen-cli (version from Cargo.toml, into .tools), `npm ci`, Playwright's Chromium
```

CI (`.github/workflows/ci.yml`) runs `just format-check`, `just test` and `just e2e` with `RUSTFLAGS=-D warnings`, so a new compiler warning fails it; run `just format` before committing. There is no clippy configuration. `build.sh` strips the name and producers sections (without that the wasm is ~7.6 MB instead of ~2 MB). The `proc-macro-error2` future-incompat warning comes from Leptos dependencies and is harmless.

## Architecture

The code is organised by vertical slice, one folder per feature, and MVVM is used inside each slice:

- `street/`, `junction/`, `map/`, `shell/` each hold what that feature needs: `model.rs` (street, junction: the editing rules, no DOM, no signals, no I/O; edits return `bool` and a refusal carries a reason), `vm.rs` (the view-model), `text.rs` (the words said to a screen reader), the Leptos views, and `tests.rs`. `junction/` also has `geometry.rs`, `read_model.rs` and `frame.rs`.
- `city/` is the sample city (`model.rs`), how it is kept in storage (`store.rs`) and the `CityBinding` a page writes back through (`binding.rs`).
- `shared/` is the kernel: catalogue, units, atlas, symbols, the ports (`Announcer`, `Storage`, `Scheduler` and their fakes in `ports.rs`), `platform.rs` (their browser side), `core.rs` (`Core<M: Presents>`, the model plus a version signal plus a cached view; `view()` is tracked, `view_now()` is not, so use it in commands), `keeper.rs`, `shortcut.rs`, `bind.rs`, and `testing.rs` (test-only HTML helpers). `shared` must not depend on any slice.
- **Dependency rule:** a slice may use another slice's `model` (junction uses the street model; city uses both), `city::store` / `city::binding`, `shell::Target` and `shared`; it must never reach into another slice's `vm`, views or text. New code goes in the slice that owns the feature, not in a layer folder.
- **View-models** expose presentation-ready reactive getters and take commands. They reach the browser **only through the ports**, so they are tested natively against `test_ports()` / `test_ports_with_time()`.
- **Views** bind to a view-model and forward events; they decide nothing. Rendering is by pure functions returning SVG strings (`plan_svg.rs`, `street/svg.rs`, `map/svg.rs`) set with `inner_html`; pointer and keyboard logic are pure state machines with tests. `shell/view.rs` is the exception: it binds the static markup the pages share to `ShellVm` with web-sys listeners and effects.
- **Entry points** are in each slice's `mod.rs` (wasm-bindgen exports: `Plan`/`open_junction`/`mount_page`, `Sheet`/`open_street`/`mount_street_page`, `mount_map`) and `lib.rs`. A page script (`web/app.js`, `junction.js`, `map.js`, each tiny) only loads the module, sets up the hatch `<defs>`, and calls `mount_*` and `.mount_shell()`.

Persistence: a page opened from the map (`?junction=3`, `?street=7`) edits one place of the city. `CityBinding` writes it back through `CityStore` (against the city as stored *now*, so another tab's change is not overwritten) behind a 250 ms debounced `Keeper`; `flush()` is called on `pagehide`. Without the query parameter the page is a sandbox on the sample streets and keeps nothing. A write failure is said once.

## Conventions and gotchas

- E2E tests (`e2e/tests/*.spec.ts`, Playwright) cover each page: `shell.spec.ts` runs the same checks on all three, plus one spec per page. `fixtures.ts` makes every test fail on a page error and gives each a fresh browser context (empty storage). They run against the built `web/pkg`, so rebuild after changing Rust (`just e2e` does). Pointer behaviour (drags) is only covered here, not by `cargo test`.
- Logic belongs in Rust; JS in `web/` stays start-up glue. Do not add logic there.
- Add view-model behaviour with a native test against the port fakes. Component tests render to HTML on the host (each slice's `tests.rs`, Leptos `ssr` as a dev-dependency; helpers in `shared/testing.rs`); `shared::platform::browser_ports()` also works natively (thread-local recorders), which is how those tests read what was announced.
- Non-`Send` values inside reactive closures go in `StoredValue::new_local`; views take a `Copy` handle (`junction::watch::Watch`, `street::watch::SheetWatch`) over an `Rc<…Vm>`.
- `any_spawner::Executor::init_wasm_bindgen()` must run at the top of every `mount_*` function before any component creates an effect.
- `#![recursion_limit = "1024"]` is needed for the Leptos `view!` macros. New web-sys APIs need their feature added in `Cargo.toml`.
- Test-driven: write the failing test first. Commit each change separately once the tests pass.
- Checking in a real browser: a tab driven in the background throttles timers to ~1 s and pauses `requestAnimationFrame`, and `.focus()` does nothing (no document focus). Use microtask waits rather than timers, and expect focus-dependent behaviour to be unverifiable there. After rebuilding, bust the cache with `fetch(url, {cache: 'reload'})` for the wasm, the glue JS and the page script, then reload.
