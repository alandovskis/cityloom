# CLAUDE.md

CityLoom is a street, junction and city-map editor in Rust, compiled to WebAssembly and drawn with Leptos 0.8 (client-side rendering). Pages: `web/index.html` (home), `web/map.html`, `web/street.html`, `web/intersection.html`. Streets come from OpenStreetMap; the catalogue's rules, rates and thresholds are synthetic placeholders.

Layout: every crate lives in `crates/` (`cityloom_editor`, the wasm editor, whose `src/` the paths below name as `src/…`; `cityloom_model`, `cityloom_report`, `cityloom_report_pdf`, `cityloom_server`, `citymodel_builder`, which also holds the OpenStreetMap importer); the root `Cargo.toml` is only the workspace. The product's `version` is in `crates/cityloom_editor/Cargo.toml`. `web/`, `e2e/`, `scripts/`, `website/` and `data/` stay at the root. Other paths below that start with a crate name (`cityloom_server/…`, `citymodel_builder/…`) are inside `crates/`.

## Commands

`just` lists the recipes. The ones used most:

```sh
just test            # cargo test --workspace (test-provider feature) + wasm32 build of the rest (cfg(wasm32) code is not built by `cargo test`); needs DATABASE_URL
just db              # throwaway Postgres in Docker (127.0.0.1:55432), prints the URL; `just db-stop` removes it
cargo test shell::vm # one module; a full test name also works
just check-no-fake   # release server must hold no fake provider (CI's server-release job)
just build           # release wasm + wasm-bindgen into web/pkg (gitignored)
just prepare         # downloads Geofabrik's Quebec extract (~1.2 GB) into data/, makes the network tiles in web/data/metro (osmium-tool keeps the roads, the importer reads
                     # them once, the network is cut into JSON tiles), packs them into data/montreal.citymodel; all gitignored; args pass through (--refresh). A place no tile holds is "not covered": without tiles no place opens
just pack            # citymodel_builder pack only; `cargo run -p citymodel_builder --release -- tiles|pack` runs one step (`fixture` and `import` turn one OSM file into a tile or a network)
just e2e-fixtures    # imports e2e/fixtures/*.osm.pbf into e2e/fixtures/generated/*.json (gitignored); the browser recipes run it first
just test-scripts    # scripts/include/tests (the issue system's consistency test and the CI selector's); CI's test job runs it
just e2e-ui          # UI tier of the browser tests (e2e/tiers.json): one page, network stubbed, static server only; no database, no archive; builds first
just e2e-integration # integration tier: the real server and a Postgres (needs DATABASE_URL): account-server and sync specs; builds first
just e2e-archive     # E2E tier: packs the e2e fixtures into target/e2e-archive and runs archive, delivery and serve specs, and the map and street specs again, against a fresh server that unpacks it (E2E_ARCHIVE=1); builds first; E2E_PORT moves the port
just e2e             # builds wasm and server, then the three tiers; with arguments only those specs (`just e2e tests/street.spec.ts`), whatever their tier; the integration tier needs DATABASE_URL
just deploy          # build and upload to Railway (needs the CLI logged in, `just prepare` tiles, zig; musl build; refuses a fake-provider server)
just check           # everything
just format          # rustfmt + Prettier; `just format-check` only checks, as CI does
just serve           # server on http://127.0.0.1:8137/ (E2E_PORT moves it); without DATABASE_URL the account side answers 503
just docs            # Docusaurus site in website/ (English, fr-CA) on :3000; `just docs-build` fails on a broken link; `just setup-docs` once
just setup           # once: wasm-bindgen-cli (version from Cargo.toml), `npm ci` at the root and in e2e, Playwright's Chromium
just setup-deploy    # once, for deploy: cargo-zigbuild, musl target; zig via `brew install zig` or `pip install ziglang`
```

CI (`.github/workflows/ci.yml`) runs `just format-check`, `just test` and `just e2e` with `RUSTFLAGS=-D warnings`, so a new compiler warning fails it; run `just format` before committing. Its `changes` job runs `scripts/ci/ci_changes.py` first: a change to documentation alone (`website/**`, `docs/**`, `*.md`) skips the unit, release-server, browser, version and Railway jobs (Format, Documentation and the documentation deploy still run and need no version bump); a pull request runs every unit test but only the browser specs its files can affect, by the first matching rule of `scripts/ci/spec-map.json` and in the tier that owns each spec (`e2e/tiers.json`: UI, integration, E2E, each its own job after one shared `build` job, and a tier with no chosen spec does not run, so editor code starts no Postgres and builds no archive; `crates/cityloom_server/**` and `crates/cityloom_editor/src/sync/**` run integration, `crates/citymodel_builder/**` every spec, since it packs the archive and makes the fixtures' tiles; a new spec goes in exactly one tier or `just test-scripts` and the Playwright config fail) (shared code, `web/`, `e2e/` helpers, the server, the build, the workflows and any file no rule names run every spec; a bare version bump is not a change to `Cargo.toml`); a push to main or a manual run always runs every spec before the deploy. A new spec goes in a group of that file and a new source folder gets a rule, or `just test-scripts` fails. SonarCloud ignores documentation-only changes; the `CI` job fails on a failed or cancelled job and passes a skipped one. Its unit-test and integration jobs run a Postgres service; docs, docs-deploy (GitHub Pages) and deploy run on push to main. No clippy configuration. `build.sh` strips the wasm's name and producers sections (~2 MB instead of ~7.6 MB). The `proc-macro-error2` future-incompat warning (from Leptos dependencies) is harmless.

Issues: labels (families `A-` area, `C-` category, `O-` browser, `P-` priority, `S-` status, plus `needs-triage`; names `<prefix>-<english> · <français>`, descriptions `<Family>: … / <Famille> : …`) live in `.github/labels.yml`, synced on push to main; the sync deletes a label not listed, so a label an automation applies must be in the file. Issue forms (`.github/ISSUE_TEMPLATE`) apply their `C-` label and `needs-triage`; `.github/issue-labeler.yml` turns dropdown answers into `A-`/`O-` labels; the maintainer sets `P-` and `S-`. `scripts/include/tests/test_issue_system.py` fails when the three disagree.

## Architecture

Vertical slices, one folder per feature, MVVM inside each:

- `street/`, `junction/`, `map/`, `shell/` each hold `model.rs` (street, junction: editing rules, no DOM, signals or I/O; edits return `bool`, a refusal carries a reason), `vm.rs`, `text.rs` (words for a screen reader), Leptos views and `tests.rs`. `junction/` also has `geometry.rs`, `read_model.rs`, `frame.rs`, `drawing.rs`, `iso/` (2.5D), `iso_marks.rs`, `stops.rs` and `signal.rs` (a signal's phases, one per street with entering lanes, each with a ready-made `Pattern` from `Junction::set_phase_pattern`, offered only where the lanes support it).
- **Dependency rule:** a slice may use another slice's `model` (junction uses the street model; city uses both), `city::store` / `city::binding` / `city::projection`, `restriction::store`, `project::context` (which project a page is opened in), `shell::Target` and `shared`; never another slice's `vm`, views or text. Stores and editors (`city/`, `restriction/`, `street/`, `junction/`) also use `sync::model` (`Id`, `Kind`, `Change`, `Origin`, `Report`, `mark`), which depends only on `shared`; the shell builds `sync::SyncVm`; every other slice reaches the sync only through `sync::hub` and `sync::model`. `shared` depends on no slice. New code goes in the slice that owns the feature, not in a layer folder.
- **View-models** expose presentation-ready reactive getters and take commands. They reach the browser only through the ports, so they are tested natively against `test_ports()` / `test_ports_with_time()`.
- **Views** bind to a view-model and forward events; they decide nothing. Rendering is by pure functions returning SVG strings (`junction/iso/`, `street/iso/`, `map/svg.rs` for the home hero only) set with `inner_html`; pointer and keyboard logic are pure state machines with tests. `shell/view.rs` binds the shared static markup to `ShellVm`. `map/` draws with MapLibre, no SVG.
- **Entry points** are in each slice's `mod.rs` (wasm-bindgen exports: `Editor`/`open_junction`/`mount_page`, `Sheet`/`open_street`/`mount_street_page`, `mount_map(canvas)`) and `lib.rs`. Each opens inside the page's project (`project::context::open_page`: `?project=<id>`, else the one opened last; none here sends the page to `index.html?missing=1` and gives nothing); links to `map.html`, `street.html`, `intersection.html` carry `project=<id>` (`shared::project_id::link`; the shell rewrites the markup's). A page script (`web/app.js`, `junction.js`, `map.js`) only loads the module, sets up the hatch `<defs>` (not `map.js`, which passes `createMapCanvas` from `web/mapcanvas.js` to `mount_map`) and calls `mount_*` and `.mount_shell()`.

Translation: English and fr-CA on all four pages and the shell.

- Engine `shared/i18n.rs` (`I18n` with `tr`/`tr_now`, `detect`, `LANG_KEY`, ...). Slices owning messages (shell, street, city, map, place, junction, restriction) have `i18n/{en,fr}.ftl` and a `RESOURCES` const, registered only in `lib.rs::i18n_for`. Text functions take `&I18n`; views use the tracked `tr`/`say` so they follow a switch, commands use `tr_now`/`say_now`.
- The model never holds wording: it says things as `Said` data (`shared/said.rs`: a key and `Arg`s) that `say` renders in the current language. Names the app invents (place, street title, ends, class words, "unnamed") are `Said` too, with messages in `city/i18n`.
- Message keys are global: a slice words another slice's messages only by key through `Said`/`Arg`, never by importing its text.
- The shell owns the switch (`ShellVm::set_locale` keeps `cityloom-lang`, sets `<html lang>`); page markup carries `data-i18n*` attributes filled by `shell/view.rs::apply_translations`. `web/junction.js` holds no wording (`mount_bare_shell`).

## Conventions and gotchas

- Test-driven: write the failing test first. Commit each change separately once the tests pass.
- Logic belongs in Rust; JS in `web/` is start-up glue only. The exception is `web/mapcanvas.js`, the adapter owning the MapLibre map (vendored MapLibre in `web/vendor`).
- Adding a string: add it to both `.ftl` files with the same id and variables (a parity test fails otherwise). Whole sentences with named placeables, never concatenated pieces; plurals use selectors (`[one]` / `*[other]`). Write an ordinary space before `:` in the `.ftl`. fr-CA follows the OQLF: no-break space before `:` and inside `« »`, none before `;` `!` `?`. Terms: `docs/glossary-fr-ca.md`. Lengths and typed numbers go through `Units::*_in(…, locale)` and `parse_number`, never `format!("{:.1}")`.
- `src/i18n_guard.rs` fails on any wording in the files listed in `MIGRATED` (add a file when its slice migrates; `LEFT_OUT` names the few left out; a map, place, junction or city file in neither list fails). A literal that only looks like a word goes in `ALLOWED_LITERALS`, a kebab-case literal that is not a message (an element id, a CSS class) in `NOT_MESSAGE_IDS`, each with its reason; any other kebab-case literal must be a message in both languages, and a dead entry fails.
- Still English: `web/credits.html`, OSM data, MapLibre's strings.
- Accessibility (WCAG 2.2 AA): `shared/contrast.rs` (tests only) measures the colour tokens of `web/app.css` and `editor.css`; a new colour pair for text or a control goes there. `shell/header.rs` keeps `--bar-stuck` and `html` has that much `scroll-padding-top`, so focus is never hidden under the sticky bar. Editors open with a skip link to `#wrap`; an `h2` heads the details panel.
- View-model behaviour gets a native test against the port fakes. Component tests render to HTML on the host (each slice's `tests.rs`, helpers in `shared/testing.rs`); `shared::platform::browser_ports()` also works natively.
- Non-`Send` values inside reactive closures go in `StoredValue::new_local`; views take a `Copy` handle (`junction::watch::Watch`, `street::watch::SheetWatch`) over an `Rc<…Vm>`.
- `any_spawner::Executor::init_wasm_bindgen()` must run at the top of every `mount_*` function before any component creates an effect.
- `#![recursion_limit = "1024"]` is needed for the Leptos `view!` macros. New web-sys APIs need their feature added in `Cargo.toml`.
- Versioning: `version` in `crates/cityloom_editor/Cargo.toml` (semver) is the product's version, shown in the app pages' footer. Every pull request raises it (major: breaking, minor: feature, patch: fix) and runs `cargo update -p cityloom_editor` for `Cargo.lock`; CI's `version` job (`scripts/check-version.sh`) fails the PR otherwise.
- Merging and deploying: `.claude/agents/deploy.md` (Haiku) merges the PRs it is told to, one at a time, by a fixed runbook (rebase on the version line only, tests, CI, a `CLEAN` merge, deploy, live site) and stops on anything needing judgment (another conflict, a failing or pending check, a failed deploy), handing it back to the session that sent it.
- Real-browser checks: a background tab throttles timers to ~1 s, pauses `requestAnimationFrame` and ignores `.focus()`, so use microtask waits and expect focus behaviour to be unverifiable. After rebuilding, bust the cache with `fetch(url, {cache: 'reload'})` for the wasm, glue JS and page script, then reload.

## Where the details are

Folder-level `CLAUDE.md` files hold the details of each area and load when you work there:

- `src/street/CLAUDE.md`: Street slice (`src/street/`)
- `src/junction/CLAUDE.md`: Junction slice (`src/junction/`)
- `src/map/CLAUDE.md`: Map slice (`src/map/`)
- `src/city/CLAUDE.md`: City, import and persistence (`src/city/`)
- `src/place/CLAUDE.md`: Place slice (`src/place/`)
- `src/restriction/CLAUDE.md`: Restriction slice (`src/restriction/`)
- `src/sync/CLAUDE.md`: Sync slice (`src/sync/`)
- `src/archive/CLAUDE.md`: Archive slice (`src/archive/`)
- `src/shared/CLAUDE.md`: Shared kernel (`src/shared/`)
- `src/account/CLAUDE.md`: Account slice (`src/account/`)
- `citymodel_builder/CLAUDE.md`: `citymodel_builder`
- `cityloom_server/CLAUDE.md`: `cityloom_server`
- `e2e/CLAUDE.md`: End-to-end tests (`e2e/`)
