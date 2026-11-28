---
id: project-layout
title: Project layout
sidebar_position: 4
---

# Project layout

This page says where things are in the repository. [Architecture](architecture/index.md) explains how the code inside them
fits together.

## The top level

| Folder       | What it holds                                                                                                                                                                                                                                                                    |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/`    | Every Rust crate (see [The crates](#the-crates)). The root `Cargo.toml` is only the workspace: it lists `crates/*` and sets the release profile.                                                                                                                                 |
| `cities/`    | One configuration file per city that `just prepare <city>` can prepare, `cities/<id>.toml`: its OpenStreetMap extract's address, its grid of tiles and their margin.                                                                                                             |
| `web/`       | What the server sends to the browser: the pages, their start-up scripts, stylesheets, fonts and icons (see [`web/`](#web)).                                                                                                                                                      |
| `tests/e2e/` | The Playwright browser tests: the specs in `tests/`, small OpenStreetMap extracts in `fixtures/`, and `tiers.json`, which puts each spec in one tier (UI, integration or E2E).                                                                                                   |
| `scripts/`   | The shell and Node scripts the `justfile` and CI run: building, deploying and checking the version. `scripts/ci/` holds CI's change selector and its rules (`spec-map.json`); `scripts/include/tests/` tests them.                                                               |
| `website/`   | This documentation (Docusaurus). The English pages are in `website/docs/`, the French ones in `website/i18n/fr-CA/docusaurus-plugin-content-docs/current/`.                                                                                                                      |
| `docs/`      | Working documents that are not on this site: the French glossary (`glossary-fr-ca.md`) and the design specs and implementation plans in `superpowers/`. The data model is drawn on this site: [Data model: streets and junctions](architecture/data-model-streets-junctions.md). |
| `deploy/`    | The `Dockerfile` of the image that `just deploy` uploads to Railway. Nothing is compiled in it.                                                                                                                                                                                  |
| `.github/`   | The CI workflows, the issue forms, the labels (`labels.yml`), the rules that turn issue answers into labels, Dependabot's settings and the `browser-tests` action that every browser tier shares.                                                                                |
| `.claude/`   | The agents used by Claude Code, such as the ones that merge pull requests and watch their deploys.                                                                                                                                                                               |

The files at the top level:

- `Cargo.toml` and `Cargo.lock`: the workspace and its locked dependencies.
- `justfile`: every task; run `just` to list them.
- `package.json`: what the pages load from npm at run time (MapLibre), copied into `web/vendor/` by the build.
- `.env.example`: every variable the server reads, with how to make each key.
- `README.md`, and `CLAUDE.md` with one more in most folders: notes for people and coding agents working in that part of
  the code.
- `DESIGN.md`, `PRODUCT.md` and `.impeccable/`: the design system and the product brief that the design tools read.
- `rustfmt.toml`, `.prettierrc.json` and `.prettierignore`: the formatting rules (`just format`).
- `sonar-project.properties`: SonarCloud's settings.

### Made by the tools, not kept in git

| Path                            | Made by               | What it is                                                                                                                                                                         |
| ------------------------------- | --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `target/`                       | `cargo`               | Build output.                                                                                                                                                                      |
| `.tools/`                       | `just setup`          | `wasm-bindgen-cli`, at the version pinned in `crates/cityloom_editor/Cargo.toml`.                                                                                                  |
| `node_modules/`                 | `just setup`          | MapLibre, from `package.json`.                                                                                                                                                     |
| `web/pkg/`                      | `just build`          | The WebAssembly modules and their JavaScript glue.                                                                                                                                 |
| `web/vendor/`                   | `just build`          | MapLibre's script, worker and stylesheet, copied from `node_modules/`.                                                                                                             |
| `web/data/metro/`               | `just prepare <city>` | The network tiles (`<x>_<y>.json`) of the city prepared last, and their `index.json`.                                                                                              |
| `data/`                         | `just prepare <city>` | Each city's extract, its osmium work files in `metro-work/<id>/`, and its packed area archive, `<id>.citymodel` (Montréal's: Geofabrik's Quebec extract and `montreal.citymodel`). |
| `tests/e2e/fixtures/generated/` | `just e2e-fixtures`   | The test fixtures made into network tiles.                                                                                                                                         |
| `website/build/`                | `just docs-build`     | This site, built.                                                                                                                                                                  |

## The crates

The workspace has seven crates, all in `crates/`. Those that run in the browser are built to WebAssembly.

| Crate                 | Runs in                   | What it is                                                                                                                                                                                                                                                   |
| --------------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `cityloom_editor`     | The browser               | The editor: every page's logic and views. Its `Cargo.toml` holds the product's version, which every pull request that changes the app raises.                                                                                                                |
| `cityloom_ui`         | The browser               | The controls the editor's pages share: the radio group, the stepper and its number field, the section of the details and the question asked before something is done. It holds no wording and uses no other crate of the workspace.                          |
| `cityloom_model`      | Browser, builder          | The plain street network that `citymodel_builder` produces and the editor reads (serde only).                                                                                                                                                                |
| `cityloom_report`     | The browser               | The design-review report as plain data (serde only). The editor fills it and hands it to the PDF writer.                                                                                                                                                     |
| `cityloom_report_pdf` | The browser               | The PDF writer. It is a module of its own, `web/pkg/cityloom_report_pdf*`, that `web/report.js` loads the first time a report is asked for, so its large dependency (krilla) stays out of the editor's module. The build fails if the editor links it.       |
| `cityloom_server`     | The server                | The server: it sends `web/`, signs people in and keeps accounts and stored work in Postgres. `migrations/` holds the database schema, `tests/` its integration tests.                                                                                        |
| `citymodel_builder`   | A developer's machine, CI | The data preparation behind `just prepare` and `just pack`: it downloads the extract, imports it once with osm2streets (`src/importer/`), cuts the network into tiles and packs them into a `.citymodel` archive (`src/archive/`), which the server unpacks. |

## The editor's folders

The editor's source, `crates/cityloom_editor/src/`, has one folder per feature (a slice). [Architecture](architecture/index.md)
says which slice may use which.

| Folder         | What it holds                                                                                                        |
| -------------- | -------------------------------------------------------------------------------------------------------------------- |
| `street/`      | The street editor.                                                                                                   |
| `junction/`    | The junction editor, with its geometry, its 2.5D drawing (`iso/`), stops and signals.                                |
| `map/`         | The city map drawn by MapLibre, the Area rules panel and the Network checks.                                         |
| `shell/`       | What every page shares: the menu, the language switch, shortcuts and the Sign in menu.                               |
| `account/`     | The Account page.                                                                                                    |
| `city/`        | The city: its layout, storage, projection onto the earth, the edits document, and the import of a network.           |
| `place/`       | Where a city comes from: place search, and fetching an area's streets.                                               |
| `restriction/` | The rules declared over an area of the map.                                                                          |
| `project/`     | The projects: the list on the home page and which project a page is opened in.                                       |
| `standards/`   | Project-specific design standards: local thresholds that the street page's checks use in place of the built-in ones. |
| `timeline/`    | The design timeline: named milestones of a project's work.                                                           |
| `sync/`        | Keeps a signed-in person's work on the server, when they chose it, and merges it between devices.                    |
| `archive/`     | The design archive: export and import of a person's work as one file.                                                |
| `shared/`      | The kernel: catalogue, units, ports, translations, test helpers. It depends on no slice.                             |

Inside an editing slice, `model.rs` holds the rules, `vm.rs` the view-model, `text.rs` the words for a screen reader,
`tests.rs` the tests and `i18n/en.ftl` and `i18n/fr.ftl` the messages; the views sit beside them. In the three largest
slices, `street/`, `junction/` and `map/`, each layer has a folder of its own: `model/`, `vm/` and `view/` (with
`model/mod.rs`, `vm/mod.rs` and `view/mod.rs` in place of `model.rs`, `vm.rs` and `view.rs`, and the tests in
`view/tests.rs`); only `mod.rs`, `i18n/` and `CLAUDE.md` stay at the slice's root. Six files sit at the top of `src/`:

- `lib.rs`: the entry points the pages call (`mount_*`) and the registry of every slice's messages.
- `dependency_guard.rs`: a test that fails when a slice uses another slice in a way its `ALLOWED` table does not allow,
  or when a layer uses what its rules forbid.
- `i18n_guard.rs`: a test that fails when a translated file holds wording of its own instead of a message.
- `layout_guard.rs`: a test that fails when a file is left at the root of `street/`, `junction/` or `map/` instead of in
  one of its layers.
- `ui_guard.rs`: a test that fails when a slice draws its own radio group, stepper, headed section of the details or
  question instead of `cityloom_ui`'s.
- `atlas_contract.rs`: a test that keeps what is built and the Atlas table's list of measures in step.

## `web/`

The server sends these files. A page loads its script and stylesheets from `v/<build>/`, an address tied to the build,
so a browser never mixes files from two deploys.

| Page                | Script        | What it is                                                              |
| ------------------- | ------------- | ----------------------------------------------------------------------- |
| `index.html`        | `home.js`     | Home: the person's projects.                                            |
| `about.html`        | `about.js`    | What CityLoom is for.                                                   |
| `map.html`          | `map.js`      | The city map. `mapcanvas.js` is the adapter that owns the MapLibre map. |
| `street.html`       | `app.js`      | The street editor.                                                      |
| `intersection.html` | `junction.js` | The junction editor.                                                    |
| `account.html`      | `account.js`  | The Account page.                                                       |
| `timeline.html`     | `timeline.js` | The design timeline.                                                    |
| `compare.html`      | `compare.js`  | Two designs of a street or junction side by side.                       |
| `credits.html`      |               | The credits, in English only.                                           |

The page scripts are start-up glue: they load the WebAssembly module and call its `mount_*` function. The other files:

- `boot.js`: the message shown when a page's code cannot be loaded or started. It is the one script outside `v/<build>/`.
- `city.js`: reads which place a page was opened for from its address (`?street=7`, `?junction=3`).
- `report.js`: loads the PDF writer and its fonts the first time a report is asked for.
- `app.css` (every page), `editor.css` (the street and junction editors), `home.css` (home and About), and
  `about.css`, `account.css`, `compare.css` and `timeline.css` for their pages. The map page uses MapLibre's stylesheet from
  `vendor/`.
- `fonts/`: the Noto Sans fonts the PDF report embeds. `glyphs/`: the map's label glyphs. `licenses/`: the licences of
  what is shipped from others.
- The logo, icons and `favicon.svg`.
