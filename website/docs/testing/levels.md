---
title: Levels of tests
sidebar_label: Levels
---

# Levels of tests

This page describes each level of tests, and the guards. The recipe and the CI job that run each level, where a new
test goes and the root of the editor's paths are in the [overview](index.md).

## Model unit tests

Each model file keeps its tests beside the code, in a `#[cfg(test)] mod tests`: for example the files of
`street/model/`, and `crates/cityloom_model/src/cut.rs`, `graph.rs`, `osm.rs`, `importer.rs` and `corridor.rs`. A
model holds no Leptos and no wording. An edit returns `bool`, and a refusal carries a reason, so a test calls the rule
and reads what it returns. `cargo test shell::vm` runs one module; a full test name also works.

## View-model tests

A view-model reaches the browser only through the ports, so its tests run natively against fakes of them, in
`shared/ports.rs`:

- `test_ports()` gives a set of ports with a `RecordingAnnouncer` (what a screen reader was told) and a
  `MemoryStorage`, which the test can read.
- `test_ports_with_time()` adds a `ManualScheduler`: delayed work runs only when the test calls `advance(ms)`.
- Other helpers in the same file add the rest, such as `test_ports_with_fetcher()`, whose `FakeFetcher` keeps each
  request until the test answers it.

The view-models' tests are the `mod tests` of `street/vm/mod.rs`, `junction/vm/mod.rs` and `map/vm/mod.rs`, among
others. The sync is tested the same way against a server in memory: `ServerSim`, `Device` and `device()` in
`sync/tests.rs`.

## Component tests

A component test renders a view to HTML on the host, with `shared::testing::html` (`shared/testing.rs`), and reads the
markup. They are in each slice's `tests.rs`: `account`, `archive`, `junction/view`, `map/view`, `project`, `share`,
`shell`, `standards`, `street/view`, `sync` and `timeline`.

The shared controls' component tests are inline, in `crates/cityloom_ui/src/`: the `mod tests` of `radio.rs`,
`stepper.rs`, `section.rs` and `confirm.rs`, with that crate's own `testing::html` (`testing.rs`).
`cargo test -p cityloom_ui` runs them.

## The wasm32 build check

`just test-wasm` runs
`cargo build --target wasm32-unknown-unknown --workspace --exclude cityloom_server --exclude citymodel_builder`. It
runs no test: it type-checks the code under `cfg(target_arch = "wasm32")`, which `cargo test` on the host does not
compile.

## Server tests

The server's tests are `crates/cityloom_server/tests/`: `api`, `areas`, `auth_flow`, `data_mount`, `db`,
`no_plaintext`, `providers`, `rotate`, `share_api`, `share_db`, `static_files`, `unavailable`, `work_api` and
`work_db`, with their helpers in `tests/support/mod.rs`.

- A test that needs the database is a `#[sqlx::test]`. It gets a fresh database on the Postgres that `DATABASE_URL`
  names. `areas`, `data_mount`, `providers`, `static_files` and `unavailable` have none.
- `TestApp` (in `tests/support/mod.rs`) is the server's router over a `PgRepo` with fixed test keys and a fixed clock.
  Its `Browser` keeps cookies in front of the router, and `sign_in` signs it in through the fake provider.
- `api`, `auth_flow`, `providers`, `rotate`, `share_api` and `work_api` need the server's `test-provider` feature.
  `just test-host` turns it on whenever the server is tested.

`just test-host` refuses to run without `DATABASE_URL`, whichever crates it tests.

## `citymodel_builder` and `cityloom_model` tests

`crates/citymodel_builder/tests/` holds the pipeline's tests (`archive`, `city`, `commands`, `ctrl_c`, `download`,
`prepare`, `progress` and `tiles`) and the importer's, which are one test crate, `tests/importer/`: `import.rs`, `real_city.rs`,
`clip_agrees.rs` and `projection.rs`, with small OpenStreetMap extracts such as `boulevard.osm` and `split_block.osm`.
`cityloom_model`'s tests are inline (see [Model unit tests](#model-unit-tests)). `cityloom_model::IMPORTER_VERSION` is
raised whenever the importer's output changes for the same data.

## Report and PDF tests

- `cityloom_report` has its tests inline, in `crates/cityloom_report/src/lib.rs`.
- `crates/cityloom_report_pdf/tests/pdf.rs` writes PDFs with `write_pdf` and reads them back with lopdf.
- The editor's tests write the report through the PDF writer: `cityloom_report_pdf` is one of the editor's
  dev-dependencies, so it stays out of the editor's own build. `scripts/build.sh` fails if the editor's JavaScript
  exports `report_pdf`.
- In the browser, `e2e/tests/report.spec.ts` reads the PDF a page makes with `pdfjs-dist`.

## Script tests

`just test-scripts` runs `python3 -m unittest discover -s scripts/include/tests`, with the standard library only. The
tests are `test_ci_changes.py` (CI's change selector, and that every spec is in one tier and one group),
`test_factory_config.py`, `test_factory_gate.py`, `test_factory_guard.py` and `test_factory_faults.py` (the software
factory; the last, the faults its tests flow puts into the code),
`test_issue_system.py` (the labels, the issue forms and the labeler rules agree), `test_architecture_docs.py` (the
Architecture pages name every slice, storage key and server table, in both languages) and `test_features.py` (the
feature list is whole, and the share of Rust tests that no feature names stays within its limit).

## Browser tests

The browser tests are Playwright specs, `e2e/tests/*.spec.ts`, run against the built `web/pkg` and a server built with
the fake provider. `e2e/tiers.json` gives each spec exactly one tier:

- **UI** (35 specs): one page, the network stubbed, the static server only; no database, no archive. `just e2e-ui`.
- **Integration**: `account-server`, `share` and `sync`, against the real server and a Postgres (`DATABASE_URL`).
  `just e2e-integration`.
- **E2E**: `archive`, `delivery` and `serve`, over a server that unpacks the packed archive, and the real server's
  delivery of the site. `just e2e-archive`.
- **`archive_also`**: `map` and `street` are UI specs that the E2E tier runs again over the unpacked archive.

`e2e/playwright.config.ts` makes one Playwright project per tier, and throws when a spec is in no tier or in two. With
`E2E_ARCHIVE=1` only the E2E project runs, with the `archive_also` specs added. Every project is Desktop Chrome at 1440
by 900. In CI a failed test is retried once, `test.only` fails the run, and two workers run. The server is
`target/e2e/cityloom_server`, the copy `scripts/pin-server.sh` makes, started with `FAKE_PROVIDER=1` and fixed test
keys. Outside CI and archive mode, Playwright reuses a server already on the port. `E2E_PORT` moves the port.

`just e2e` builds the wasm and the server and makes the fixtures' tiles, then runs the three tiers. Given arguments, it
runs those specs whatever their tier: `just e2e tests/street.spec.ts`.

## The guards

The guards are tests about the code itself. They run with the host tests.

- **`i18n_guard.rs`** fails on wording in the files of its `MIGRATED` list (`LEFT_OUT` names the few left out, each
  with its reason), on a message id used that is not in both languages, on a file of a translated slice in neither
  list, and on a dead entry of `ALLOWED_LITERALS` or `NOT_MESSAGE_IDS`.
- **`dependency_guard.rs`** fails on a use of another slice that its `ALLOWED` table does not allow, on a break of a
  layer or library rule that its `EXCEPTIONS` table does not let pass, on an entry that nothing uses or that has no
  reason, and on a folder of `src/` that is not in `SLICES`. Test code is exempt.
- **`layout_guard.rs`** keeps the files of `street/`, `junction/` and `map/` in `model/`, `vm/` and `view/`: their
  roots hold only `mod.rs`, `CLAUDE.md` and `i18n/`.
- **`ui_guard.rs`** fails when a slice draws its own radio group, stepper, headed section of the details or question
  instead of `cityloom_ui`'s.
- **`atlas_contract.rs`** holds the Atlas table (`shared::atlas::MEASURES`) as a contract: every measure it lists is
  built, nothing else is, and every measure is named in both languages.
- **`shared/contrast.rs`** measures the colour tokens of the CSS against WCAG 1.4.3 (text) and 1.4.11 (the rest).
- **`no_plaintext.rs`** (in the server's tests) reads every column of `users`, `identities`, `sessions`, `work` and
  `shares` as raw bytes and finds nothing the person signed in with.

The first six are `#[cfg(test)]` modules of the editor.
