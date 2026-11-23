---
id: test-inventory
title: Test inventory
---

# Test inventory

This page counts CityLoom's tests by level, by crate and tier, and by feature, and says which tests belong to no
feature. The levels themselves are described in [Testing](testing.md). The counts are those of commit `0d9cb8b9`
(version 0.113.0). No script makes these counts or checks them, so they stay those of that commit until
the tests are counted again.

There are 5,385 tests: 4,552 Rust tests, 607 Playwright tests and 226 script tests. They are the tests these commands list:

```sh
cargo test --workspace --features cityloom_server/test-provider -- --list   # Rust: 4,552
cd e2e && npx playwright test --list                                        # Playwright: 607
python3 -m unittest discover -s scripts/include/tests                       # scripts: 226 (it runs them)
```

The Rust list needs no database. The Playwright list needs `npm ci` in `e2e/` and the server that
`scripts/pin-server.sh` copies to `target/e2e/cityloom_server`: `e2e/playwright.config.ts` throws without that file.

The paths are from the repository's root, or relative to `crates/cityloom_editor/src/` where the text names the editor.

How the tests of each feature spread over the levels is in the [Coverage matrix](coverage-matrix.md).

## How tests are counted

### One test

- **Rust**: one line of the list that ends in `: test`. The two `#[ignore]` tests of
  `crates/cityloom_editor/src/street/view/iso/mod.rs` (`markup_size` and `preview`) are in the list, so they are
  counted. No doc-test is in it.
- **Playwright**: one line of the list, that is one test of one project of `e2e/playwright.config.ts`. A test declared in
  a loop counts once for each turn. With `E2E_ARCHIVE=1`, the E2E project lists the specs of `archive_also` in
  `e2e/tiers.json` too: `map` (38 tests) and `street` (71 tests). These 109 are the UI tier's tests run again, and
  are not counted again.
- **Script**: one test method of `scripts/include/tests/test_*.py`.

The [wasm32 build check](testing.md#the-wasm32-build-check) runs no test, so it is not here.

### The file of a test

A Rust test's file is the deepest file its module path names under its crate's `src/`: `a/b.rs`, `a/b/mod.rs` or
`a/tests.rs`, and `lib.rs` when the path names none. A test of `crates/*/tests/<name>.rs` has that file. The importer's
tests are one test crate (`crates/citymodel_builder/tests/importer/main.rs`), and each has the file of its module beside
`main.rs`. A Playwright test's file is its spec. A file with no test is not listed.

### The level of a test

A Rust test's level comes from where its file is. The first rule that matches decides:

1. **Guard**: `i18n_guard.rs`, `layout_guard.rs`, `atlas_contract.rs` and `shared/contrast.rs` in
   `crates/cityloom_editor/src/`, and `crates/cityloom_server/tests/no_plaintext.rs`.
2. **Server**: `crates/cityloom_server/`.
3. **`citymodel_builder` and `cityloom_model`**: `crates/citymodel_builder/` and `crates/cityloom_model/`.
4. **Report and PDF**: `crates/cityloom_report/` and `crates/cityloom_report_pdf/`.
5. **Component**: in the editor, a file named `tests.rs`. `sync/tests.rs`, which tests the sync, is
   therefore here.
6. **View-model**: in the editor, a file under a `vm/` folder, or named `vm.rs`.
7. **Model**: every other unit test of the editor. It includes the pure SVG renderers of `view/` and `shared/` (such as
   `street/view/iso/`, `street/view/drawing.rs` and `shared/iso/`), and the other files of `shared/`.

A Playwright test's level is the tier of its spec in `e2e/tiers.json`: UI, integration or E2E. A script test's level is
Script.

### The features of a test

The 45 features are those of `scripts/ci/features.json`.

- A Rust test belongs to every feature with a `code` glob that matches its file. In a glob, `*` stays within one folder
  and `**` crosses folders.
- A Playwright test belongs to every feature that names its spec in `specs`.
- A test can belong to several features, and is counted in each, so the counts of the features add up to more than the
  tests there are.
- A test that belongs to none is in [Unmapped](#unmapped).

## By level

| Level                                    | Runner       | Files                                                                                                                                      |     Tests |
| ---------------------------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------ | --------: |
| Model                                    | `cargo test` | Every other unit test of the editor                                                                                                        |     2,328 |
| View-model                               | `cargo test` | In the editor, a file under a `vm/` folder or named `vm.rs`                                                                                |       664 |
| Component                                | `cargo test` | In the editor, a file named `tests.rs`                                                                                                     |       871 |
| Guard                                    | `cargo test` | The editor's `i18n_guard.rs`, `layout_guard.rs`, `atlas_contract.rs`, `shared/contrast.rs`, `crates/cityloom_server/tests/no_plaintext.rs` |        53 |
| Server                                   | `cargo test` | `crates/cityloom_server/`, without `no_plaintext.rs`                                                                                       |       264 |
| `citymodel_builder` and `cityloom_model` | `cargo test` | `crates/citymodel_builder/`, `crates/cityloom_model/`                                                                                      |       348 |
| Report and PDF                           | `cargo test` | `crates/cityloom_report/`, `crates/cityloom_report_pdf/`                                                                                   |        24 |
| Script                                   | `unittest`   | `scripts/include/tests/`                                                                                                                   |       226 |
| UI tier                                  | Playwright   | The specs of `ui` in `e2e/tiers.json`                                                                                                      |       552 |
| Integration tier                         | Playwright   | The specs of `integration` in `e2e/tiers.json`                                                                                             |        34 |
| E2E tier                                 | Playwright   | The specs of `e2e` in `e2e/tiers.json`                                                                                                     |        21 |
| **Total**                                |              |                                                                                                                                            | **5,385** |

## By crate and by tier

### Rust tests by crate

One row per test binary, named by its root file in the crate's folder. The library of `cityloom_report_pdf`
and the programs of `cityloom_server` and `citymodel_builder` (`src/main.rs`) are test binaries too, with no test.

| Crate                   | Test binary              |     Tests |
| ----------------------- | ------------------------ | --------: |
| `cityloom_editor`       | `src/lib.rs`             |     3,915 |
| `citymodel_builder`     | `src/lib.rs`             |       101 |
| `citymodel_builder`     | `tests/archive.rs`       |        33 |
| `citymodel_builder`     | `tests/commands.rs`      |        14 |
| `citymodel_builder`     | `tests/ctrl_c.rs`        |         2 |
| `citymodel_builder`     | `tests/download.rs`      |        17 |
| `citymodel_builder`     | `tests/importer/main.rs` |        71 |
| `citymodel_builder`     | `tests/prepare.rs`       |         7 |
| `citymodel_builder`     | `tests/progress.rs`      |        14 |
| `citymodel_builder`     | `tests/tiles.rs`         |         9 |
| **`citymodel_builder`** | **Total**                |   **268** |
| `cityloom_server`       | `src/lib.rs`             |        96 |
| `cityloom_server`       | `tests/api.rs`           |        10 |
| `cityloom_server`       | `tests/areas.rs`         |         4 |
| `cityloom_server`       | `tests/auth_flow.rs`     |        25 |
| `cityloom_server`       | `tests/data_mount.rs`    |         9 |
| `cityloom_server`       | `tests/db.rs`            |        10 |
| `cityloom_server`       | `tests/no_plaintext.rs`  |         1 |
| `cityloom_server`       | `tests/providers.rs`     |         8 |
| `cityloom_server`       | `tests/rotate.rs`        |         4 |
| `cityloom_server`       | `tests/share_api.rs`     |        13 |
| `cityloom_server`       | `tests/share_db.rs`      |        10 |
| `cityloom_server`       | `tests/static_files.rs`  |        27 |
| `cityloom_server`       | `tests/unavailable.rs`   |         3 |
| `cityloom_server`       | `tests/work_api.rs`      |        20 |
| `cityloom_server`       | `tests/work_db.rs`       |        25 |
| **`cityloom_server`**   | **Total**                |   **265** |
| `cityloom_model`        | `src/lib.rs`             |        80 |
| `cityloom_report_pdf`   | `tests/pdf.rs`           |        23 |
| `cityloom_report`       | `src/lib.rs`             |         1 |
| **Total**               |                          | **4,552** |

The levels of each crate:

| Crate                 | Tests by level                                               |
| --------------------- | ------------------------------------------------------------ |
| `cityloom_editor`     | 3,915 (model 2,328, view-model 664, component 871, guard 52) |
| `citymodel_builder`   | 268 (`citymodel_builder` and `cityloom_model`)               |
| `cityloom_server`     | 265 (guard 1, server 264)                                    |
| `cityloom_model`      | 80 (`citymodel_builder` and `cityloom_model`)                |
| `cityloom_report_pdf` | 23 (report and PDF)                                          |
| `cityloom_report`     | 1 (report and PDF)                                           |

### Playwright tests by tier

One row per Playwright project, with its specs (`e2e/tests/<name>.spec.ts`) and the tests of each.

| Project       |  Specs | Tests of each spec                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |   Tests |
| ------------- | -----: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------: |
| `ui`          |     35 | `a11y` 21, `about` 16, `account` 7, `calming` 10, `compare` 8, `curb` 5, `design-archive` 4, `editors` 26, `edits` 6, `focus-touch` 10, `footer` 11, `green` 7, `home` 5, `impact` 11, `junction` 81, `junction-a11y` 21, `language` 21, `map` 38, `network` 8, `projects` 12, `read-only` 5, `report` 7, `restrictions` 18, `scenarios` 9, `search` 13, `shell` 47, `speed` 10, `standards` 7, `stops` 6, `street` 71, `street-iso` 14, `templates` 5, `timeline` 7, `webgl` 1, `world` 4 |     552 |
| `integration` |      3 | `account-server` 12, `share` 9, `sync` 13                                                                                                                                                                                                                                                                                                                                                                                                                                                  |      34 |
| `e2e`         |      3 | `archive` 3, `delivery` 14, `serve` 4                                                                                                                                                                                                                                                                                                                                                                                                                                                      |      21 |
| **Total**     | **41** |                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | **607** |

## By feature

The features are in the order of `scripts/ci/features.json`. Each links to its test files and specs.

| Feature                                                   | Name                                                                                                                                                        | Rust tests | Playwright tests |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------: | ---------------: |
| [`street.pieces`](#streetpieces)                          | A street's cross-section is laid out from pieces that are added, removed, resized and rearranged, with Undo and Start over                                  |        361 |               97 |
| [`street.drawing`](#streetdrawing)                        | A street is drawn in 2.5D, and a piece is chosen by pressing it on the drawing                                                                              |        240 |               14 |
| [`street.stretches`](#streetstretches)                    | A street is split into stretches, each with its own layout, whose boundaries are dragged on the street's plan                                               |        118 |               92 |
| [`street.speed`](#streetspeed)                            | A street and each of its stretches have a speed limit, imported from OpenStreetMap and changed by the designer                                              |         15 |               10 |
| [`street.curbside`](#streetcurbside)                      | Curbside uses (parking, loading, trees, patios, bike parking, platforms) are placed along the curb                                                          |         68 |                5 |
| [`street.stops`](#streetstops)                            | Bus stops are added along a street, at the curb, on a bulb or on an island, with or without a shelter                                                       |         52 |                6 |
| [`street.calming`](#streetcalming)                        | Raised elements calm traffic: raised tables, raised crossings, chicanes and modal filters                                                                   |         58 |               10 |
| [`street.green`](#streetgreen)                            | Street trees and green infrastructure (tree pits, planting beds, rain gardens, bioswales) are placed on a street                                            |         64 |                7 |
| [`street.checks`](#streetchecks)                          | A street's checks say whether its pieces fit and meet the standards, and the Impact card says what the last change did                                      |         32 |               82 |
| [`street.templates`](#streettemplates)                    | A street is transformed by a template, such as a transit street or a school street, in one step that Undo takes back                                        |         50 |                5 |
| [`street.standards`](#streetstandards)                    | A standards profile replaces the built-in thresholds that a street's checks read                                                                            |         36 |                7 |
| [`junction.layout`](#junctionlayout)                      | A junction's arms, lanes, turns, crossings and control are drawn and edited, and every part is reached by keyboard or by a tap                              |        797 |              112 |
| [`junction.roundabout`](#junctionroundabout)              | A junction is made a roundabout or a turbo roundabout                                                                                                       |         40 |               81 |
| [`junction.signal.phases`](#junctionsignalphases)         | A signalled junction's phases are chosen from ready-made patterns or set by hand, with their timing                                                         |         46 |               81 |
| [`junction.signal.plans`](#junctionsignalplans)           | A signal follows time-of-day plans on a weekly schedule                                                                                                     |         14 |               86 |
| [`junction.signal.calls`](#junctionsignalcalls)           | A signal serves pedestrians on call, by push button or by passive detection                                                                                 |         33 |               86 |
| [`junction.signal.actuation`](#junctionsignalactuation)   | A signal's green responds to vehicle demand (actuation)                                                                                                     |         16 |               86 |
| [`junction.signal.bicycles`](#junctionsignalbicycles)     | A signal detects bicycles and gives them their green                                                                                                        |         14 |               86 |
| [`junction.signal.transit`](#junctionsignaltransit)       | A signal gives transit priority by early green or green extension                                                                                           |         16 |               86 |
| [`junction.signal.preemption`](#junctionsignalpreemption) | A junction's signal pre-empts for emergency calls                                                                                                           |         19 |               86 |
| [`design.scenarios`](#designscenarios)                    | A street or a junction has named scenarios beside its baseline, made, renamed, duplicated, deleted and switched between                                     |         10 |                9 |
| [`design.compare`](#designcompare)                        | Two designs of one street or junction are compared side by side, with what differs in the second                                                            |         65 |                8 |
| [`map.city`](#mapcity)                                    | The city map shows the project's area with its streets and junctions, and opens each in its editor                                                          |        197 |               39 |
| [`map.search`](#mapsearch)                                | Places of the map are found by name from the search box                                                                                                     |          2 |               13 |
| [`map.goals`](#mapgoals)                                  | A planning goal chosen on the map is followed on the street and junction pages                                                                              |         13 |               38 |
| [`map.network-checks`](#mapnetwork-checks)                | The network checks find dead ends, missing crossings, broken cycling links and transit corridors across the area                                            |         34 |                8 |
| [`map.area-rules`](#maparea-rules)                        | Rules are declared over an area drawn on the map (car-free, bus and bike only, speed limit, no parking or loading), and streets that break them are flagged |        108 |               18 |
| [`map.report`](#mapreport)                                | The design review report is downloaded as a PDF, in the page's language                                                                                     |         13 |                7 |
| [`city.area`](#cityarea)                                  | A project's area is fetched from its tiles, cut out and made into a city of streets and junctions, and made again when its tile changes                     |        185 |                4 |
| [`city.edits`](#cityedits)                                | A street's or junction's edits are kept in the browser under its OpenStreetMap ids, and survive a reload                                                    |        160 |               10 |
| [`data.importer`](#dataimporter)                          | OpenStreetMap's roads become the street network the editor reads                                                                                            |        114 |                0 |
| [`data.archive`](#dataarchive)                            | The network is cut into tiles and packed into the city archive, which the server serves                                                                     |         50 |                3 |
| [`projects.list`](#projectslist)                          | The home page lists the person's projects; a project is made in a metropolitan area and opens on its map                                                    |        139 |               17 |
| [`projects.timeline`](#projectstimeline)                  | A project's milestones keep its work at a moment, and are compared with the work now                                                                        |         43 |                7 |
| [`projects.design-archive`](#projectsdesign-archive)      | A project's design is exported as one file and imported on another device                                                                                   |         29 |                4 |
| [`projects.share`](#projectsshare)                        | A project is shared by a link that opens it read-only, and a shared design is copied into one's own projects                                                |         60 |                9 |
| [`projects.read-only`](#projectsread-only)                | A design opened read-only can be looked at and chosen in, and no control changes it                                                                         |          1 |                5 |
| [`account.sign-in`](#accountsign-in)                      | A person signs in with a provider, links or unlinks sign-ins, chooses where the work is kept, and deletes the account                                       |        105 |               19 |
| [`account.sync`](#accountsync)                            | A signed-in person's work is kept on the server and merged between devices, also after working offline                                                      |        113 |               13 |
| [`shell.settings`](#shellsettings)                        | Every page has the top bar, the settings menu (theme, units, region, keyboard shortcuts), the details panel and the notes tray                              |        118 |               84 |
| [`shell.language`](#shelllanguage)                        | Every app page can be read in English or in Canadian French, and the choice is kept                                                                         |         23 |               21 |
| [`shell.accessibility`](#shellaccessibility)              | Every page is usable by keyboard and screen reader, at any zoom and width, and its colours meet WCAG 2.2 AA                                                 |         28 |               31 |
| [`pages.about`](#pagesabout)                              | The About page says what CityLoom is for, over the city of the project opened last                                                                          |         14 |               16 |
| [`pages.credits`](#pagescredits)                          | The Credits page names the data and the software CityLoom is made from                                                                                      |          0 |                0 |
| [`server.delivery`](#serverdelivery)                      | The server delivers the site and the city's data quickly: versioned addresses, compression and byte ranges                                                  |         22 |               18 |
| **Sum over the features**                                 |                                                                                                                                                             |  **3,735** |        **1,526** |
| **Distinct mapped tests**                                 |                                                                                                                                                             |  **3,666** |          **607** |

The sums are above the distinct mapped tests because 9 Rust files and 8 specs belong to several features:

| Test file or spec                                          | Tests | Features                                                                                                                                                                                                                               |
| ---------------------------------------------------------- | ----: | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/cityloom_editor/src/junction/view/phase_svg.rs`    |     9 | `junction.layout`, `junction.signal.phases`                                                                                                                                                                                            |
| `crates/cityloom_editor/src/junction/view/signal_folds.rs` |     8 | `junction.layout`, `junction.signal.phases`                                                                                                                                                                                            |
| `crates/cityloom_editor/src/junction/view/turns.rs`        |     1 | `junction.layout`, `junction.signal.phases`                                                                                                                                                                                            |
| `crates/cityloom_editor/src/junction/vm/compare.rs`        |    11 | `junction.layout`, `design.compare`                                                                                                                                                                                                    |
| `crates/cityloom_editor/src/project/copies.rs`             |    13 | `projects.list`, `projects.share`                                                                                                                                                                                                      |
| `crates/cityloom_editor/src/shell/session.rs`              |    20 | `account.sign-in`, `shell.settings`                                                                                                                                                                                                    |
| `crates/cityloom_model/src/catalogue.rs`                   |     2 | `street.pieces`, `data.importer`                                                                                                                                                                                                       |
| `crates/cityloom_model/src/corridor.rs`                    |     4 | `map.network-checks`, `data.importer`                                                                                                                                                                                                  |
| `crates/cityloom_model/src/street/speed.rs`                |     1 | `street.pieces`, `street.speed`                                                                                                                                                                                                        |
| `e2e/tests/a11y.spec.ts`                                   |    21 | `street.stretches`, `shell.accessibility`                                                                                                                                                                                              |
| `e2e/tests/editors.spec.ts`                                |    26 | `street.pieces`, `shell.settings`                                                                                                                                                                                                      |
| `e2e/tests/focus-touch.spec.ts`                            |    10 | `junction.layout`, `shell.accessibility`                                                                                                                                                                                               |
| `e2e/tests/junction.spec.ts`                               |    81 | `junction.layout`, `junction.roundabout`, `junction.signal.phases`, `junction.signal.plans`, `junction.signal.calls`, `junction.signal.actuation`, `junction.signal.bicycles`, `junction.signal.transit`, `junction.signal.preemption` |
| `e2e/tests/map.spec.ts`                                    |    38 | `map.city`, `map.goals`                                                                                                                                                                                                                |
| `e2e/tests/read-only.spec.ts`                              |     5 | `junction.signal.plans`, `junction.signal.calls`, `junction.signal.actuation`, `junction.signal.bicycles`, `junction.signal.transit`, `junction.signal.preemption`, `projects.read-only`                                               |
| `e2e/tests/street.spec.ts`                                 |    71 | `street.pieces`, `street.stretches`, `street.checks`                                                                                                                                                                                   |
| `e2e/tests/world.spec.ts`                                  |     4 | `city.area`, `city.edits`                                                                                                                                                                                                              |

### `street.pieces`

A street's cross-section is laid out from pieces that are added, removed, resized and rearranged, with Undo and Start over.

- Rust tests: 361 (model 164, view-model 178, `citymodel_builder` and `cityloom_model` 19)
- Playwright tests: 97 (UI tier)

| Test file or spec                                     | Level                                    | Tests |
| ----------------------------------------------------- | ---------------------------------------- | ----: |
| `crates/cityloom_editor/src/shared/catalogue.rs`      | Model                                    |     6 |
| `crates/cityloom_editor/src/street/model/mod.rs`      | Model                                    |   109 |
| `crates/cityloom_editor/src/street/model/pick.rs`     | Model                                    |    37 |
| `crates/cityloom_editor/src/street/model/pieces.rs`   | Model                                    |     2 |
| `crates/cityloom_editor/src/street/view/inspector.rs` | Model                                    |     5 |
| `crates/cityloom_editor/src/street/view/page.rs`      | Model                                    |     5 |
| `crates/cityloom_editor/src/street/vm/add_menu.rs`    | View-model                               |     5 |
| `crates/cityloom_editor/src/street/vm/keys.rs`        | View-model                               |    16 |
| `crates/cityloom_editor/src/street/vm/mod.rs`         | View-model                               |   157 |
| `crates/cityloom_model/src/catalogue.rs`              | `citymodel_builder` and `cityloom_model` |     2 |
| `crates/cityloom_model/src/street/from_road.rs`       | `citymodel_builder` and `cityloom_model` |    10 |
| `crates/cityloom_model/src/street/mod.rs`             | `citymodel_builder` and `cityloom_model` |     6 |
| `crates/cityloom_model/src/street/speed.rs`           | `citymodel_builder` and `cityloom_model` |     1 |
| `e2e/tests/street.spec.ts`                            | UI tier                                  |    71 |
| `e2e/tests/editors.spec.ts`                           | UI tier                                  |    26 |

### `street.drawing`

A street is drawn in 2.5D, and a piece is chosen by pressing it on the drawing.

- Rust tests: 240 (model)
- Playwright tests: 14 (UI tier)

| Test file or spec                                              | Level   | Tests |
| -------------------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/shared/iso/format.rs`              | Model   |     2 |
| `crates/cityloom_editor/src/shared/iso/ground.rs`              | Model   |     6 |
| `crates/cityloom_editor/src/shared/iso/paint.rs`               | Model   |     7 |
| `crates/cityloom_editor/src/shared/iso/shading.rs`             | Model   |     1 |
| `crates/cityloom_editor/src/street/view/iso/calming.rs`        | Model   |    17 |
| `crates/cityloom_editor/src/street/view/iso/curbside.rs`       | Model   |    14 |
| `crates/cityloom_editor/src/street/view/iso/green.rs`          | Model   |    12 |
| `crates/cityloom_editor/src/street/view/iso/joints.rs`         | Model   |     8 |
| `crates/cityloom_editor/src/street/view/iso/markings.rs`       | Model   |    18 |
| `crates/cityloom_editor/src/street/view/iso/mod.rs`            | Model   |    67 |
| `crates/cityloom_editor/src/street/view/iso/order.rs`          | Model   |     4 |
| `crates/cityloom_editor/src/street/view/iso/paint.rs`          | Model   |     9 |
| `crates/cityloom_editor/src/street/view/iso/projection.rs`     | Model   |    15 |
| `crates/cityloom_editor/src/street/view/iso/raised.rs`         | Model   |     9 |
| `crates/cityloom_editor/src/street/view/iso/scene/hits.rs`     | Model   |     2 |
| `crates/cityloom_editor/src/street/view/iso/scene/markings.rs` | Model   |     3 |
| `crates/cityloom_editor/src/street/view/iso/scene/objects.rs`  | Model   |     2 |
| `crates/cityloom_editor/src/street/view/iso/scene/raised.rs`   | Model   |     2 |
| `crates/cityloom_editor/src/street/view/iso/scene/surface.rs`  | Model   |     3 |
| `crates/cityloom_editor/src/street/view/iso/sign.rs`           | Model   |     2 |
| `crates/cityloom_editor/src/street/view/iso/solids.rs`         | Model   |    11 |
| `crates/cityloom_editor/src/street/view/iso/stops.rs`          | Model   |    12 |
| `crates/cityloom_editor/src/street/view/iso/surface.rs`        | Model   |     4 |
| `crates/cityloom_editor/src/street/view/iso/symbols.rs`        | Model   |     7 |
| `crates/cityloom_editor/src/street/view/iso/taper.rs`          | Model   |     3 |
| `e2e/tests/street-iso.spec.ts`                                 | UI tier |    14 |

### `street.stretches`

A street is split into stretches, each with its own layout, whose boundaries are dragged on the street's plan.

- Rust tests: 118 (model 95, view-model 23)
- Playwright tests: 92 (UI tier)

| Test file or spec                                       | Level      | Tests |
| ------------------------------------------------------- | ---------- | ----: |
| `crates/cityloom_editor/src/street/model/joint.rs`      | Model      |     9 |
| `crates/cityloom_editor/src/street/model/plan.rs`       | Model      |     9 |
| `crates/cityloom_editor/src/street/model/transition.rs` | Model      |    24 |
| `crates/cityloom_editor/src/street/model/zones.rs`      | Model      |    30 |
| `crates/cityloom_editor/src/street/view/plan/hit.rs`    | Model      |     7 |
| `crates/cityloom_editor/src/street/view/plan/mod.rs`    | Model      |    14 |
| `crates/cityloom_editor/src/street/view/plan/scale.rs`  | Model      |     2 |
| `crates/cityloom_editor/src/street/vm/plan.rs`          | View-model |    23 |
| `e2e/tests/street.spec.ts`                              | UI tier    |    71 |
| `e2e/tests/a11y.spec.ts`                                | UI tier    |    21 |

### `street.speed`

A street and each of its stretches have a speed limit, imported from OpenStreetMap and changed by the designer.

- Rust tests: 15 (model 14, `citymodel_builder` and `cityloom_model` 1)
- Playwright tests: 10 (UI tier)

| Test file or spec                                       | Level                                    | Tests |
| ------------------------------------------------------- | ---------------------------------------- | ----: |
| `crates/cityloom_editor/src/street/model/zone_speed.rs` | Model                                    |    14 |
| `crates/cityloom_model/src/street/speed.rs`             | `citymodel_builder` and `cityloom_model` |     1 |
| `e2e/tests/speed.spec.ts`                               | UI tier                                  |    10 |

### `street.curbside`

Curbside uses (parking, loading, trees, patios, bike parking, platforms) are placed along the curb.

- Rust tests: 68 (model)
- Playwright tests: 5 (UI tier)

| Test file or spec                                       | Level   | Tests |
| ------------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/street/model/curb.rs`       | Model   |    28 |
| `crates/cityloom_editor/src/street/model/curb_end.rs`   | Model   |     7 |
| `crates/cityloom_editor/src/street/model/curb_shown.rs` | Model   |    12 |
| `crates/cityloom_editor/src/street/model/map_uses.rs`   | Model   |    12 |
| `crates/cityloom_editor/src/street/view/curb_strip.rs`  | Model   |     9 |
| `e2e/tests/curb.spec.ts`                                | UI tier |     5 |

### `street.stops`

Bus stops are added along a street, at the curb, on a bulb or on an island, with or without a shelter.

- Rust tests: 52 (model)
- Playwright tests: 6 (UI tier)

| Test file or spec                                     | Level   | Tests |
| ----------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/street/model/stop.rs`     | Model   |    37 |
| `crates/cityloom_editor/src/street/model/stop_end.rs` | Model   |    15 |
| `e2e/tests/stops.spec.ts`                             | UI tier |     6 |

### `street.calming`

Raised elements calm traffic: raised tables, raised crossings, chicanes and modal filters.

- Rust tests: 58 (model)
- Playwright tests: 10 (UI tier)

| Test file or spec                                          | Level   | Tests |
| ---------------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/street/model/calming.rs`       | Model   |    52 |
| `crates/cityloom_editor/src/street/model/calming_shown.rs` | Model   |     6 |
| `e2e/tests/calming.spec.ts`                                | UI tier |    10 |

### `street.green`

Street trees and green infrastructure (tree pits, planting beds, rain gardens, bioswales) are placed on a street.

- Rust tests: 64 (model)
- Playwright tests: 7 (UI tier)

| Test file or spec                                       | Level   | Tests |
| ------------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/street/model/green.rs`      | Model   |    59 |
| `crates/cityloom_editor/src/street/view/green_strip.rs` | Model   |     3 |
| `crates/cityloom_editor/src/street/view/green_tools.rs` | Model   |     2 |
| `e2e/tests/green.spec.ts`                               | UI tier |     7 |

### `street.checks`

A street's checks say whether its pieces fit and meet the standards, and the Impact card says what the last change did.

- Rust tests: 32 (model 17, view-model 15)
- Playwright tests: 82 (UI tier)

| Test file or spec                                        | Level      | Tests |
| -------------------------------------------------------- | ---------- | ----: |
| `crates/cityloom_editor/src/shared/check_fold.rs`        | Model      |     4 |
| `crates/cityloom_editor/src/street/model/profile.rs`     | Model      |     7 |
| `crates/cityloom_editor/src/street/model/unavailable.rs` | Model      |     6 |
| `crates/cityloom_editor/src/street/vm/impact.rs`         | View-model |    15 |
| `e2e/tests/impact.spec.ts`                               | UI tier    |    11 |
| `e2e/tests/street.spec.ts`                               | UI tier    |    71 |

### `street.templates`

A street is transformed by a template, such as a transit street or a school street, in one step that Undo takes back.

- Rust tests: 50 (model 27, view-model 23)
- Playwright tests: 5 (UI tier)

| Test file or spec                                      | Level      | Tests |
| ------------------------------------------------------ | ---------- | ----: |
| `crates/cityloom_editor/src/street/model/school.rs`    | Model      |    17 |
| `crates/cityloom_editor/src/street/model/templates.rs` | Model      |    10 |
| `crates/cityloom_editor/src/street/vm/transform.rs`    | View-model |    23 |
| `e2e/tests/templates.spec.ts`                          | UI tier    |     5 |

### `street.standards`

A standards profile replaces the built-in thresholds that a street's checks read.

- Rust tests: 36 (model 23, component 13)
- Playwright tests: 7 (UI tier)

| Test file or spec                               | Level     | Tests |
| ----------------------------------------------- | --------- | ----: |
| `crates/cityloom_editor/src/standards/model.rs` | Model     |    10 |
| `crates/cityloom_editor/src/standards/store.rs` | Model     |    12 |
| `crates/cityloom_editor/src/standards/tests.rs` | Component |    13 |
| `crates/cityloom_editor/src/standards/view.rs`  | Model     |     1 |
| `e2e/tests/standards.spec.ts`                   | UI tier   |     7 |

### `junction.layout`

A junction's arms, lanes, turns, crossings and control are drawn and edited, and every part is reached by keyboard or by a tap.

- Rust tests: 797 (model 469, view-model 93, component 235)
- Playwright tests: 112 (UI tier)

| Test file or spec                                          | Level      | Tests |
| ---------------------------------------------------------- | ---------- | ----: |
| `crates/cityloom_editor/src/junction/model/beyond.rs`      | Model      |    10 |
| `crates/cityloom_editor/src/junction/model/control.rs`     | Model      |     4 |
| `crates/cityloom_editor/src/junction/model/curbside.rs`    | Model      |     5 |
| `crates/cityloom_editor/src/junction/model/geometry.rs`    | Model      |     9 |
| `crates/cityloom_editor/src/junction/model/mod.rs`         | Model      |   163 |
| `crates/cityloom_editor/src/junction/model/read_model.rs`  | Model      |   137 |
| `crates/cityloom_editor/src/junction/model/stops.rs`       | Model      |    12 |
| `crates/cityloom_editor/src/junction/view/drawing.rs`      | Model      |     3 |
| `crates/cityloom_editor/src/junction/view/frame.rs`        | Model      |     3 |
| `crates/cityloom_editor/src/junction/view/inspector.rs`    | Model      |     8 |
| `crates/cityloom_editor/src/junction/view/iso/mod.rs`      | Model      |    59 |
| `crates/cityloom_editor/src/junction/view/iso_marks.rs`    | Model      |    31 |
| `crates/cityloom_editor/src/junction/view/notes.rs`        | Model      |     5 |
| `crates/cityloom_editor/src/junction/view/page.rs`         | Model      |     2 |
| `crates/cityloom_editor/src/junction/view/phase_svg.rs`    | Model      |     9 |
| `crates/cityloom_editor/src/junction/view/signal_folds.rs` | Model      |     8 |
| `crates/cityloom_editor/src/junction/view/tests.rs`        | Component  |   235 |
| `crates/cityloom_editor/src/junction/view/turns.rs`        | Model      |     1 |
| `crates/cityloom_editor/src/junction/vm/compare.rs`        | View-model |    11 |
| `crates/cityloom_editor/src/junction/vm/keys.rs`           | View-model |    11 |
| `crates/cityloom_editor/src/junction/vm/mod.rs`            | View-model |    56 |
| `crates/cityloom_editor/src/junction/vm/text.rs`           | View-model |    15 |
| `e2e/tests/junction.spec.ts`                               | UI tier    |    81 |
| `e2e/tests/junction-a11y.spec.ts`                          | UI tier    |    21 |
| `e2e/tests/focus-touch.spec.ts`                            | UI tier    |    10 |

### `junction.roundabout`

A junction is made a roundabout or a turbo roundabout.

- Rust tests: 40 (model)
- Playwright tests: 81 (UI tier)

| Test file or spec                                           | Level   | Tests |
| ----------------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/junction/model/turbo.rs`        | Model   |    22 |
| `crates/cityloom_editor/src/junction/model/turbo_ring.rs`   | Model   |     2 |
| `crates/cityloom_editor/src/junction/model/turbo_spiral.rs` | Model   |    16 |
| `e2e/tests/junction.spec.ts`                                | UI tier |    81 |

### `junction.signal.phases`

A signalled junction's phases are chosen from ready-made patterns or set by hand, with their timing.

- Rust tests: 46 (model)
- Playwright tests: 81 (UI tier)

| Test file or spec                                          | Level   | Tests |
| ---------------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/junction/model/signal.rs`      | Model   |    28 |
| `crates/cityloom_editor/src/junction/view/phase_svg.rs`    | Model   |     9 |
| `crates/cityloom_editor/src/junction/view/signal_folds.rs` | Model   |     8 |
| `crates/cityloom_editor/src/junction/view/turns.rs`        | Model   |     1 |
| `e2e/tests/junction.spec.ts`                               | UI tier |    81 |

### `junction.signal.plans`

A signal follows time-of-day plans on a weekly schedule.

- Rust tests: 14 (model)
- Playwright tests: 86 (UI tier)

| Test file or spec                                    | Level   | Tests |
| ---------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/junction/model/plans.rs` | Model   |    14 |
| `e2e/tests/junction.spec.ts`                         | UI tier |    81 |
| `e2e/tests/read-only.spec.ts`                        | UI tier |     5 |

### `junction.signal.calls`

A signal serves pedestrians on call, by push button or by passive detection.

- Rust tests: 33 (model)
- Playwright tests: 86 (UI tier)

| Test file or spec                                    | Level   | Tests |
| ---------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/junction/model/calls.rs` | Model   |    33 |
| `e2e/tests/junction.spec.ts`                         | UI tier |    81 |
| `e2e/tests/read-only.spec.ts`                        | UI tier |     5 |

### `junction.signal.actuation`

A signal's green responds to vehicle demand (actuation).

- Rust tests: 16 (model)
- Playwright tests: 86 (UI tier)

| Test file or spec                                        | Level   | Tests |
| -------------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/junction/model/actuation.rs` | Model   |    16 |
| `e2e/tests/junction.spec.ts`                             | UI tier |    81 |
| `e2e/tests/read-only.spec.ts`                            | UI tier |     5 |

### `junction.signal.bicycles`

A signal detects bicycles and gives them their green.

- Rust tests: 14 (model)
- Playwright tests: 86 (UI tier)

| Test file or spec                                       | Level   | Tests |
| ------------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/junction/model/bicycles.rs` | Model   |    14 |
| `e2e/tests/junction.spec.ts`                            | UI tier |    81 |
| `e2e/tests/read-only.spec.ts`                           | UI tier |     5 |

### `junction.signal.transit`

A signal gives transit priority by early green or green extension.

- Rust tests: 16 (model)
- Playwright tests: 86 (UI tier)

| Test file or spec                                      | Level   | Tests |
| ------------------------------------------------------ | ------- | ----: |
| `crates/cityloom_editor/src/junction/model/transit.rs` | Model   |    16 |
| `e2e/tests/junction.spec.ts`                           | UI tier |    81 |
| `e2e/tests/read-only.spec.ts`                          | UI tier |     5 |

### `junction.signal.preemption`

A junction's signal pre-empts for emergency calls.

- Rust tests: 19 (model)
- Playwright tests: 86 (UI tier)

| Test file or spec                                         | Level   | Tests |
| --------------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/junction/model/preemption.rs` | Model   |    19 |
| `e2e/tests/junction.spec.ts`                              | UI tier |    81 |
| `e2e/tests/read-only.spec.ts`                             | UI tier |     5 |

### `design.scenarios`

A street or a junction has named scenarios beside its baseline, made, renamed, duplicated, deleted and switched between.

- Rust tests: 10 (model)
- Playwright tests: 9 (UI tier)

| Test file or spec                             | Level   | Tests |
| --------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/city/scenario.rs` | Model   |    10 |
| `e2e/tests/scenarios.spec.ts`                 | UI tier |     9 |

### `design.compare`

Two designs of one street or junction are compared side by side, with what differs in the second.

- Rust tests: 65 (model 44, view-model 21)
- Playwright tests: 8 (UI tier)

| Test file or spec                                      | Level      | Tests |
| ------------------------------------------------------ | ---------- | ----: |
| `crates/cityloom_editor/src/junction/model/compare.rs` | Model      |    13 |
| `crates/cityloom_editor/src/junction/vm/compare.rs`    | View-model |    11 |
| `crates/cityloom_editor/src/shared/compare.rs`         | Model      |     7 |
| `crates/cityloom_editor/src/shared/compare_view.rs`    | Model      |    11 |
| `crates/cityloom_editor/src/street/model/compare.rs`   | Model      |    13 |
| `crates/cityloom_editor/src/street/vm/compare.rs`      | View-model |    10 |
| `e2e/tests/compare.spec.ts`                            | UI tier    |     8 |

### `map.city`

The city map shows the project's area with its streets and junctions, and opens each in its editor.

- Rust tests: 197 (model 6, view-model 191)
- Playwright tests: 39 (UI tier)

| Test file or spec                                  | Level      | Tests |
| -------------------------------------------------- | ---------- | ----: |
| `crates/cityloom_editor/src/map/model/editable.rs` | Model      |     6 |
| `crates/cityloom_editor/src/map/vm/camera.rs`      | View-model |    11 |
| `crates/cityloom_editor/src/map/vm/mod.rs`         | View-model |   105 |
| `crates/cityloom_editor/src/map/vm/overlay.rs`     | View-model |    50 |
| `crates/cityloom_editor/src/map/vm/selection.rs`   | View-model |     4 |
| `crates/cityloom_editor/src/map/vm/style.rs`       | View-model |    21 |
| `e2e/tests/map.spec.ts`                            | UI tier    |    38 |
| `e2e/tests/webgl.spec.ts`                          | UI tier    |     1 |

### `map.search`

Places of the map are found by name from the search box.

- Rust tests: 2 (model)
- Playwright tests: 13 (UI tier)

| Test file or spec                             | Level   | Tests |
| --------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/shared/search.rs` | Model   |     2 |
| `e2e/tests/search.spec.ts`                    | UI tier |    13 |

### `map.goals`

A planning goal chosen on the map is followed on the street and junction pages.

- Rust tests: 13 (model)
- Playwright tests: 38 (UI tier)

| Test file or spec                                 | Level   | Tests |
| ------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/shared/goals.rs`      | Model   |     8 |
| `crates/cityloom_editor/src/shared/goals_view.rs` | Model   |     5 |
| `e2e/tests/map.spec.ts`                           | UI tier |    38 |

### `map.network-checks`

The network checks find dead ends, missing crossings, broken cycling links and transit corridors across the area.

- Rust tests: 34 (model 30, `citymodel_builder` and `cityloom_model` 4)
- Playwright tests: 8 (UI tier)

| Test file or spec                                  | Level                                    | Tests |
| -------------------------------------------------- | ---------------------------------------- | ----: |
| `crates/cityloom_editor/src/map/model/corridor.rs` | Model                                    |     7 |
| `crates/cityloom_editor/src/map/model/network.rs`  | Model                                    |    23 |
| `crates/cityloom_model/src/corridor.rs`            | `citymodel_builder` and `cityloom_model` |     4 |
| `e2e/tests/network.spec.ts`                        | UI tier                                  |     8 |

### `map.area-rules`

Rules are declared over an area drawn on the map (car-free, bus and bike only, speed limit, no parking or loading), and streets that break them are flagged.

- Rust tests: 108 (model 64, view-model 44)
- Playwright tests: 18 (UI tier)

| Test file or spec                                 | Level      | Tests |
| ------------------------------------------------- | ---------- | ----: |
| `crates/cityloom_editor/src/map/vm/areas.rs`      | View-model |    44 |
| `crates/cityloom_editor/src/restriction/draw.rs`  | Model      |    14 |
| `crates/cityloom_editor/src/restriction/mod.rs`   | Model      |     1 |
| `crates/cityloom_editor/src/restriction/model.rs` | Model      |    26 |
| `crates/cityloom_editor/src/restriction/store.rs` | Model      |    23 |
| `e2e/tests/restrictions.spec.ts`                  | UI tier    |    18 |

### `map.report`

The design review report is downloaded as a PDF, in the page's language.

- Rust tests: 13 (view-model 12, report and PDF 1)
- Playwright tests: 7 (UI tier)

| Test file or spec                                    | Level          | Tests |
| ---------------------------------------------------- | -------------- | ----: |
| `crates/cityloom_editor/src/map/vm/report.rs`        | View-model     |     5 |
| `crates/cityloom_editor/src/map/vm/report_review.rs` | View-model     |     7 |
| `crates/cityloom_report/src/lib.rs`                  | Report and PDF |     1 |
| `e2e/tests/report.spec.ts`                           | UI tier        |     7 |

### `city.area`

A project's area is fetched from its tiles, cut out and made into a city of streets and junctions, and made again when its tile changes.

- Rust tests: 185 (model)
- Playwright tests: 4 (UI tier)

| Test file or spec                                       | Level   | Tests |
| ------------------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/city/import.rs`             | Model   |    32 |
| `crates/cityloom_editor/src/city/model.rs`              | Model   |   105 |
| `crates/cityloom_editor/src/city/model/neighbours.rs`   | Model   |     6 |
| `crates/cityloom_editor/src/city/model/network_read.rs` | Model   |     2 |
| `crates/cityloom_editor/src/city/projection.rs`         | Model   |     5 |
| `crates/cityloom_editor/src/place/area.rs`              | Model   |     9 |
| `crates/cityloom_editor/src/place/loader.rs`            | Model   |    13 |
| `crates/cityloom_editor/src/place/mod.rs`               | Model   |     6 |
| `crates/cityloom_editor/src/place/tiles.rs`             | Model   |     7 |
| `e2e/tests/world.spec.ts`                               | UI tier |     4 |

### `city.edits`

A street's or junction's edits are kept in the browser under its OpenStreetMap ids, and survive a reload.

- Rust tests: 160 (model)
- Playwright tests: 10 (UI tier)

| Test file or spec                             | Level   | Tests |
| --------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/city/binding.rs`  | Model   |     6 |
| `crates/cityloom_editor/src/city/edits.rs`    | Model   |    76 |
| `crates/cityloom_editor/src/city/keys.rs`     | Model   |     9 |
| `crates/cityloom_editor/src/city/store.rs`    | Model   |    61 |
| `crates/cityloom_editor/src/shared/scoped.rs` | Model   |     7 |
| `crates/cityloom_editor/src/shared/store.rs`  | Model   |     1 |
| `e2e/tests/edits.spec.ts`                     | UI tier |     6 |
| `e2e/tests/world.spec.ts`                     | UI tier |     4 |

### `data.importer`

OpenStreetMap's roads become the street network the editor reads.

- Rust tests: 114 (`citymodel_builder` and `cityloom_model`)
- Playwright tests: 0

| Test file or spec                                 | Level                                    | Tests |
| ------------------------------------------------- | ---------------------------------------- | ----: |
| `crates/cityloom_model/src/catalogue.rs`          | `citymodel_builder` and `cityloom_model` |     2 |
| `crates/cityloom_model/src/corridor.rs`           | `citymodel_builder` and `cityloom_model` |     4 |
| `crates/cityloom_model/src/cut.rs`                | `citymodel_builder` and `cityloom_model` |    21 |
| `crates/cityloom_model/src/frame.rs`              | `citymodel_builder` and `cityloom_model` |     4 |
| `crates/cityloom_model/src/graph.rs`              | `citymodel_builder` and `cityloom_model` |     5 |
| `crates/cityloom_model/src/hours.rs`              | `citymodel_builder` and `cityloom_model` |     3 |
| `crates/cityloom_model/src/importer.rs`           | `citymodel_builder` and `cityloom_model` |     2 |
| `crates/cityloom_model/src/junction.rs`           | `citymodel_builder` and `cityloom_model` |     2 |
| `crates/cityloom_model/src/lane.rs`               | `citymodel_builder` and `cityloom_model` |     3 |
| `crates/cityloom_model/src/osm.rs`                | `citymodel_builder` and `cityloom_model` |     1 |
| `crates/cityloom_model/src/point.rs`              | `citymodel_builder` and `cityloom_model` |     3 |
| `crates/cityloom_model/src/provenance.rs`         | `citymodel_builder` and `cityloom_model` |     4 |
| `crates/cityloom_model/src/road.rs`               | `citymodel_builder` and `cityloom_model` |     7 |
| `crates/cityloom_model/src/road_type.rs`          | `citymodel_builder` and `cityloom_model` |     2 |
| `crates/citymodel_builder/src/importer/blocks.rs` | `citymodel_builder` and `cityloom_model` |    14 |
| `crates/citymodel_builder/src/importer/merge.rs`  | `citymodel_builder` and `cityloom_model` |    14 |
| `crates/citymodel_builder/src/importer/mod.rs`    | `citymodel_builder` and `cityloom_model` |    23 |

### `data.archive`

The network is cut into tiles and packed into the city archive, which the server serves.

- Rust tests: 50 (`citymodel_builder` and `cityloom_model`)
- Playwright tests: 3 (E2E tier)

| Test file or spec                                | Level                                    | Tests |
| ------------------------------------------------ | ---------------------------------------- | ----: |
| `crates/citymodel_builder/src/archive/index.rs`  | `citymodel_builder` and `cityloom_model` |    11 |
| `crates/citymodel_builder/src/download.rs`       | `citymodel_builder` and `cityloom_model` |     1 |
| `crates/citymodel_builder/src/metro_index.rs`    | `citymodel_builder` and `cityloom_model` |     3 |
| `crates/citymodel_builder/src/network_tiles.rs`  | `citymodel_builder` and `cityloom_model` |     5 |
| `crates/citymodel_builder/src/progress/frame.rs` | `citymodel_builder` and `cityloom_model` |    25 |
| `crates/citymodel_builder/src/progress/mod.rs`   | `citymodel_builder` and `cityloom_model` |     3 |
| `crates/citymodel_builder/src/progress/view.rs`  | `citymodel_builder` and `cityloom_model` |     2 |
| `e2e/tests/archive.spec.ts`                      | E2E tier                                 |     3 |

### `projects.list`

The home page lists the person's projects; a project is made in a metropolitan area and opens on its map.

- Rust tests: 139 (model 22, component 117)
- Playwright tests: 17 (UI tier)

| Test file or spec                                 | Level     | Tests |
| ------------------------------------------------- | --------- | ----: |
| `crates/cityloom_editor/src/project/copies.rs`    | Model     |    13 |
| `crates/cityloom_editor/src/project/tests.rs`     | Component |   117 |
| `crates/cityloom_editor/src/shared/project_id.rs` | Model     |     9 |
| `e2e/tests/home.spec.ts`                          | UI tier   |     5 |
| `e2e/tests/projects.spec.ts`                      | UI tier   |    12 |

### `projects.timeline`

A project's milestones keep its work at a moment, and are compared with the work now.

- Rust tests: 43 (model 23, component 20)
- Playwright tests: 7 (UI tier)

| Test file or spec                              | Level     | Tests |
| ---------------------------------------------- | --------- | ----: |
| `crates/cityloom_editor/src/timeline/model.rs` | Model     |     2 |
| `crates/cityloom_editor/src/timeline/store.rs` | Model     |     9 |
| `crates/cityloom_editor/src/timeline/tests.rs` | Component |    20 |
| `crates/cityloom_editor/src/timeline/work.rs`  | Model     |    12 |
| `e2e/tests/timeline.spec.ts`                   | UI tier   |     7 |

### `projects.design-archive`

A project's design is exported as one file and imported on another device.

- Rust tests: 29 (component)
- Playwright tests: 4 (UI tier)

| Test file or spec                             | Level     | Tests |
| --------------------------------------------- | --------- | ----: |
| `crates/cityloom_editor/src/archive/tests.rs` | Component |    29 |
| `e2e/tests/design-archive.spec.ts`            | UI tier   |     4 |

### `projects.share`

A project is shared by a link that opens it read-only, and a shared design is copied into one's own projects.

- Rust tests: 60 (model 15, component 37, server 8)
- Playwright tests: 9 (integration tier)

| Test file or spec                              | Level            | Tests |
| ---------------------------------------------- | ---------------- | ----: |
| `crates/cityloom_editor/src/project/copies.rs` | Model            |    13 |
| `crates/cityloom_editor/src/share/store.rs`    | Model            |     2 |
| `crates/cityloom_editor/src/share/tests.rs`    | Component        |    37 |
| `crates/cityloom_server/src/share.rs`          | Server           |     8 |
| `e2e/tests/share.spec.ts`                      | Integration tier |     9 |

### `projects.read-only`

A design opened read-only can be looked at and chosen in, and no control changes it.

- Rust tests: 1 (model)
- Playwright tests: 5 (UI tier)

| Test file or spec                             | Level   | Tests |
| --------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/shared/locked.rs` | Model   |     1 |
| `e2e/tests/read-only.spec.ts`                 | UI tier |     5 |

### `account.sign-in`

A person signs in with a provider, links or unlinks sign-ins, chooses where the work is kept, and deletes the account.

- Rust tests: 105 (model 20, view-model 25, component 15, server 45)
- Playwright tests: 19 (UI tier 7, integration tier 12)

| Test file or spec                                   | Level            | Tests |
| --------------------------------------------------- | ---------------- | ----: |
| `crates/cityloom_editor/src/account/tests.rs`       | Component        |    15 |
| `crates/cityloom_editor/src/account/vm.rs`          | View-model       |    25 |
| `crates/cityloom_editor/src/shell/session.rs`       | Model            |    20 |
| `crates/cityloom_server/src/accounts.rs`            | Server           |     9 |
| `crates/cityloom_server/src/crypto.rs`              | Server           |    10 |
| `crates/cityloom_server/src/db.rs`                  | Server           |     1 |
| `crates/cityloom_server/src/flow.rs`                | Server           |     6 |
| `crates/cityloom_server/src/providers.rs`           | Server           |     4 |
| `crates/cityloom_server/src/providers/apple.rs`     | Server           |     4 |
| `crates/cityloom_server/src/providers/microsoft.rs` | Server           |     3 |
| `crates/cityloom_server/src/session.rs`             | Server           |     8 |
| `e2e/tests/account.spec.ts`                         | UI tier          |     7 |
| `e2e/tests/account-server.spec.ts`                  | Integration tier |    12 |

### `account.sync`

A signed-in person's work is kept on the server and merged between devices, also after working offline.

- Rust tests: 113 (model 50, component 54, server 9)
- Playwright tests: 13 (integration tier)

| Test file or spec                          | Level            | Tests |
| ------------------------------------------ | ---------------- | ----: |
| `crates/cityloom_editor/src/sync/hub.rs`   | Model            |     4 |
| `crates/cityloom_editor/src/sync/local.rs` | Model            |    24 |
| `crates/cityloom_editor/src/sync/model.rs` | Model            |    22 |
| `crates/cityloom_editor/src/sync/tests.rs` | Component        |    54 |
| `crates/cityloom_server/src/work.rs`       | Server           |     9 |
| `e2e/tests/sync.spec.ts`                   | Integration tier |    13 |

### `shell.settings`

Every page has the top bar, the settings menu (theme, units, region, keyboard shortcuts), the details panel and the notes tray.

- Rust tests: 118 (model 44, view-model 30, component 44)
- Playwright tests: 84 (UI tier)

| Test file or spec                               | Level      | Tests |
| ----------------------------------------------- | ---------- | ----: |
| `crates/cityloom_editor/src/shared/shortcut.rs` | Model      |     1 |
| `crates/cityloom_editor/src/shared/units.rs`    | Model      |    17 |
| `crates/cityloom_editor/src/shell/header.rs`    | Model      |     1 |
| `crates/cityloom_editor/src/shell/popover.rs`   | Model      |     4 |
| `crates/cityloom_editor/src/shell/session.rs`   | Model      |    20 |
| `crates/cityloom_editor/src/shell/share.rs`     | Model      |     1 |
| `crates/cityloom_editor/src/shell/tests.rs`     | Component  |    44 |
| `crates/cityloom_editor/src/shell/vm.rs`        | View-model |    30 |
| `e2e/tests/shell.spec.ts`                       | UI tier    |    47 |
| `e2e/tests/editors.spec.ts`                     | UI tier    |    26 |
| `e2e/tests/footer.spec.ts`                      | UI tier    |    11 |

### `shell.language`

Every app page can be read in English or in Canadian French, and the choice is kept.

- Rust tests: 23 (model)
- Playwright tests: 21 (UI tier)

| Test file or spec                           | Level   | Tests |
| ------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/shared/i18n.rs` | Model   |    18 |
| `crates/cityloom_editor/src/shared/said.rs` | Model   |     5 |
| `e2e/tests/language.spec.ts`                | UI tier |    21 |

### `shell.accessibility`

Every page is usable by keyboard and screen reader, at any zoom and width, and its colours meet WCAG 2.2 AA.

- Rust tests: 28 (guard)
- Playwright tests: 31 (UI tier)

| Test file or spec                               | Level   | Tests |
| ----------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/shared/contrast.rs` | Guard   |    28 |
| `e2e/tests/a11y.spec.ts`                        | UI tier |    21 |
| `e2e/tests/focus-touch.spec.ts`                 | UI tier |    10 |

### `pages.about`

The About page says what CityLoom is for, over the city of the project opened last.

- Rust tests: 14 (model)
- Playwright tests: 16 (UI tier)

| Test file or spec                            | Level   | Tests |
| -------------------------------------------- | ------- | ----: |
| `crates/cityloom_editor/src/map/view/svg.rs` | Model   |    14 |
| `e2e/tests/about.spec.ts`                    | UI tier |    16 |

### `pages.credits`

The Credits page names the data and the software CityLoom is made from.

- Rust tests: 0
- Playwright tests: 0

### `server.delivery`

The server delivers the site and the city's data quickly: versioned addresses, compression and byte ranges.

- Rust tests: 22 (server)
- Playwright tests: 18 (E2E tier)

| Test file or spec                      | Level    | Tests |
| -------------------------------------- | -------- | ----: |
| `crates/cityloom_server/src/app.rs`    | Server   |     5 |
| `crates/cityloom_server/src/config.rs` | Server   |    17 |
| `e2e/tests/delivery.spec.ts`           | E2E tier |    14 |
| `e2e/tests/serve.spec.ts`              | E2E tier |     4 |

## Unmapped

These tests belong to no feature: no glob of `scripts/ci/features.json` matches their file. With them, every test is
on this page: 3,666 mapped and 886 unmapped Rust tests make the 4,552, the 607 Playwright tests are all mapped, and the
226 script tests are all unmapped. That is 4,273 mapped tests and 1,112 unmapped ones.

### Unmapped Rust tests

57 files, 886 tests (model 164, view-model 20, component 307, guard 25, server 180, `citymodel_builder` and `cityloom_model` 167, report and PDF 23).

| Test file                                                | Level                                    |   Tests |
| -------------------------------------------------------- | ---------------------------------------- | ------: |
| `crates/cityloom_editor/src/atlas_contract.rs`           | Guard                                    |       7 |
| `crates/cityloom_editor/src/city/mod.rs`                 | Model                                    |       5 |
| `crates/cityloom_editor/src/i18n_guard.rs`               | Guard                                    |      14 |
| `crates/cityloom_editor/src/layout_guard.rs`             | Guard                                    |       3 |
| `crates/cityloom_editor/src/lib.rs`                      | Model                                    |       3 |
| `crates/cityloom_editor/src/map/view/tests.rs`           | Component                                |     122 |
| `crates/cityloom_editor/src/shared/atlas.rs`             | Model                                    |       3 |
| `crates/cityloom_editor/src/shared/core.rs`              | Model                                    |       3 |
| `crates/cityloom_editor/src/shared/diagram/mod.rs`       | Model                                    |       2 |
| `crates/cityloom_editor/src/shared/diagram/render.rs`    | Model                                    |      13 |
| `crates/cityloom_editor/src/shared/flat.rs`              | Model                                    |      21 |
| `crates/cityloom_editor/src/shared/keeper.rs`            | Model                                    |       3 |
| `crates/cityloom_editor/src/shared/platform.rs`          | Model                                    |       5 |
| `crates/cityloom_editor/src/shared/ports.rs`             | Model                                    |      31 |
| `crates/cityloom_editor/src/shared/provenance.rs`        | Model                                    |       1 |
| `crates/cityloom_editor/src/shared/random.rs`            | Model                                    |       1 |
| `crates/cityloom_editor/src/shared/sources.rs`           | Model                                    |       8 |
| `crates/cityloom_editor/src/shared/sources_view.rs`      | Model                                    |       3 |
| `crates/cityloom_editor/src/shared/symbols.rs`           | Model                                    |      13 |
| `crates/cityloom_editor/src/shared/tick.rs`              | Model                                    |       3 |
| `crates/cityloom_editor/src/street/model/beyond.rs`      | Model                                    |      17 |
| `crates/cityloom_editor/src/street/model/matching.rs`    | Model                                    |       3 |
| `crates/cityloom_editor/src/street/model/replay.rs`      | Model                                    |      15 |
| `crates/cityloom_editor/src/street/view/notes.rs`        | Model                                    |       9 |
| `crates/cityloom_editor/src/street/view/scroll.rs`       | Model                                    |       1 |
| `crates/cityloom_editor/src/street/view/tests.rs`        | Component                                |     185 |
| `crates/cityloom_editor/src/street/view/transform.rs`    | Model                                    |       1 |
| `crates/cityloom_editor/src/street/vm/text.rs`           | View-model                               |      20 |
| `crates/cityloom_report_pdf/tests/pdf.rs`                | Report and PDF                           |      23 |
| `crates/cityloom_server/src/commands.rs`                 | Server                                   |       4 |
| `crates/cityloom_server/src/fake_provider.rs`            | Server                                   |       7 |
| `crates/cityloom_server/src/lib.rs`                      | Server                                   |       1 |
| `crates/cityloom_server/tests/api.rs`                    | Server                                   |      10 |
| `crates/cityloom_server/tests/areas.rs`                  | Server                                   |       4 |
| `crates/cityloom_server/tests/auth_flow.rs`              | Server                                   |      25 |
| `crates/cityloom_server/tests/data_mount.rs`             | Server                                   |       9 |
| `crates/cityloom_server/tests/db.rs`                     | Server                                   |      10 |
| `crates/cityloom_server/tests/no_plaintext.rs`           | Guard                                    |       1 |
| `crates/cityloom_server/tests/providers.rs`              | Server                                   |       8 |
| `crates/cityloom_server/tests/rotate.rs`                 | Server                                   |       4 |
| `crates/cityloom_server/tests/share_api.rs`              | Server                                   |      13 |
| `crates/cityloom_server/tests/share_db.rs`               | Server                                   |      10 |
| `crates/cityloom_server/tests/static_files.rs`           | Server                                   |      27 |
| `crates/cityloom_server/tests/unavailable.rs`            | Server                                   |       3 |
| `crates/cityloom_server/tests/work_api.rs`               | Server                                   |      20 |
| `crates/cityloom_server/tests/work_db.rs`                | Server                                   |      25 |
| `crates/citymodel_builder/tests/archive.rs`              | `citymodel_builder` and `cityloom_model` |      33 |
| `crates/citymodel_builder/tests/commands.rs`             | `citymodel_builder` and `cityloom_model` |      14 |
| `crates/citymodel_builder/tests/ctrl_c.rs`               | `citymodel_builder` and `cityloom_model` |       2 |
| `crates/citymodel_builder/tests/download.rs`             | `citymodel_builder` and `cityloom_model` |      17 |
| `crates/citymodel_builder/tests/importer/clip_agrees.rs` | `citymodel_builder` and `cityloom_model` |       3 |
| `crates/citymodel_builder/tests/importer/import.rs`      | `citymodel_builder` and `cityloom_model` |      56 |
| `crates/citymodel_builder/tests/importer/projection.rs`  | `citymodel_builder` and `cityloom_model` |       1 |
| `crates/citymodel_builder/tests/importer/real_city.rs`   | `citymodel_builder` and `cityloom_model` |      11 |
| `crates/citymodel_builder/tests/prepare.rs`              | `citymodel_builder` and `cityloom_model` |       7 |
| `crates/citymodel_builder/tests/progress.rs`             | `citymodel_builder` and `cityloom_model` |      14 |
| `crates/citymodel_builder/tests/tiles.rs`                | `citymodel_builder` and `cityloom_model` |       9 |
| **Total**                                                |                                          | **886** |

### Unmapped Playwright tests

None: every spec of `e2e/tiers.json` is in a feature, and `scripts/include/tests/test_features.py` fails otherwise.

### Unmapped script tests

No glob covers `scripts/`.

| Test file                                         |   Tests |
| ------------------------------------------------- | ------: |
| `scripts/include/tests/test_architecture_docs.py` |      19 |
| `scripts/include/tests/test_ci_changes.py`        |      71 |
| `scripts/include/tests/test_factory_config.py`    |       7 |
| `scripts/include/tests/test_factory_gate.py`      |      63 |
| `scripts/include/tests/test_factory_guard.py`     |      12 |
| `scripts/include/tests/test_features.py`          |      21 |
| `scripts/include/tests/test_issue_system.py`      |      33 |
| **Total**                                         | **226** |
