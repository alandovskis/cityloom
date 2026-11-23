---
id: coverage-matrix
title: Coverage matrix
---

# Coverage matrix

This page sets CityLoom's features against the levels of tests, with the number of tests in each cell. It also gives
the tests that belong to no feature, and the line coverage that SonarCloud measures. The counts are copied from the
[Test inventory](test-inventory.md), which says how tests are counted and names their files. They are those of commit
`0d9cb8b9` (version 0.113.0). No script makes these figures or checks them, so they stay those of that commit until
the tests are counted again. The gaps in the tests are in [Known gaps and risks](testing.md#known-gaps-and-risks).

## Features against levels

A row is one of the 45 features of `scripts/ci/features.json`, in its order, and links to the test files of the
feature in the inventory. A column is one of the 11 [levels](test-inventory.md#by-level). A cell is the number of the
feature's tests at that level.

- A test of several features counts in each of them.
- A zero is not by itself a gap. The level of a test comes from where its file is, and its features from the code and
  the specs that `scripts/ci/features.json` names, so what a feature does can be tested by tests that its row does not
  count. For example:
  - The Component cells of every `map` feature, and of every `street` feature but `street.standards`, are 0. The
    component tests of the two pages are `street/view/tests.rs` (185 tests) and `map/view/tests.rs` (122 tests) in
    `crates/cityloom_editor/src/`, which are unmapped.
  - `pages.credits` is 0 at every level. `e2e/tests/home.spec.ts` and `e2e/tests/about.spec.ts`, which belong to other
    features, open `/credits.html`.
  - The Server cells of `projects.share`, `account.sync` and `account.sign-in` leave out `share_api.rs`, `work_api.rs`
    and `auth_flow.rs` of `crates/cityloom_server/tests/`, which are unmapped.

| Feature                                                                    |     Model | View-model | Component |  Guard | Server | `citymodel_builder` and `cityloom_model` | Report and PDF | Script |   UI tier | Integration tier | E2E tier |     Total |
| -------------------------------------------------------------------------- | --------: | ---------: | --------: | -----: | -----: | ---------------------------------------: | -------------: | -----: | --------: | ---------------: | -------: | --------: |
| [`street.pieces`](test-inventory.md#streetpieces)                          |       164 |        178 |         0 |      0 |      0 |                                       19 |              0 |      0 |        97 |                0 |        0 |       458 |
| [`street.drawing`](test-inventory.md#streetdrawing)                        |       240 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        14 |                0 |        0 |       254 |
| [`street.stretches`](test-inventory.md#streetstretches)                    |        95 |         23 |         0 |      0 |      0 |                                        0 |              0 |      0 |        92 |                0 |        0 |       210 |
| [`street.speed`](test-inventory.md#streetspeed)                            |        14 |          0 |         0 |      0 |      0 |                                        1 |              0 |      0 |        10 |                0 |        0 |        25 |
| [`street.curbside`](test-inventory.md#streetcurbside)                      |        68 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |         5 |                0 |        0 |        73 |
| [`street.stops`](test-inventory.md#streetstops)                            |        52 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |         6 |                0 |        0 |        58 |
| [`street.calming`](test-inventory.md#streetcalming)                        |        58 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        10 |                0 |        0 |        68 |
| [`street.green`](test-inventory.md#streetgreen)                            |        64 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |         7 |                0 |        0 |        71 |
| [`street.checks`](test-inventory.md#streetchecks)                          |        17 |         15 |         0 |      0 |      0 |                                        0 |              0 |      0 |        82 |                0 |        0 |       114 |
| [`street.templates`](test-inventory.md#streettemplates)                    |        27 |         23 |         0 |      0 |      0 |                                        0 |              0 |      0 |         5 |                0 |        0 |        55 |
| [`street.standards`](test-inventory.md#streetstandards)                    |        23 |          0 |        13 |      0 |      0 |                                        0 |              0 |      0 |         7 |                0 |        0 |        43 |
| [`junction.layout`](test-inventory.md#junctionlayout)                      |       469 |         93 |       235 |      0 |      0 |                                        0 |              0 |      0 |       112 |                0 |        0 |       909 |
| [`junction.roundabout`](test-inventory.md#junctionroundabout)              |        40 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        81 |                0 |        0 |       121 |
| [`junction.signal.phases`](test-inventory.md#junctionsignalphases)         |        46 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        81 |                0 |        0 |       127 |
| [`junction.signal.plans`](test-inventory.md#junctionsignalplans)           |        14 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        86 |                0 |        0 |       100 |
| [`junction.signal.calls`](test-inventory.md#junctionsignalcalls)           |        33 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        86 |                0 |        0 |       119 |
| [`junction.signal.actuation`](test-inventory.md#junctionsignalactuation)   |        16 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        86 |                0 |        0 |       102 |
| [`junction.signal.bicycles`](test-inventory.md#junctionsignalbicycles)     |        14 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        86 |                0 |        0 |       100 |
| [`junction.signal.transit`](test-inventory.md#junctionsignaltransit)       |        16 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        86 |                0 |        0 |       102 |
| [`junction.signal.preemption`](test-inventory.md#junctionsignalpreemption) |        19 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        86 |                0 |        0 |       105 |
| [`design.scenarios`](test-inventory.md#designscenarios)                    |        10 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |         9 |                0 |        0 |        19 |
| [`design.compare`](test-inventory.md#designcompare)                        |        44 |         21 |         0 |      0 |      0 |                                        0 |              0 |      0 |         8 |                0 |        0 |        73 |
| [`map.city`](test-inventory.md#mapcity)                                    |         6 |        191 |         0 |      0 |      0 |                                        0 |              0 |      0 |        39 |                0 |        0 |       236 |
| [`map.search`](test-inventory.md#mapsearch)                                |         2 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        13 |                0 |        0 |        15 |
| [`map.goals`](test-inventory.md#mapgoals)                                  |        13 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        38 |                0 |        0 |        51 |
| [`map.network-checks`](test-inventory.md#mapnetwork-checks)                |        30 |          0 |         0 |      0 |      0 |                                        4 |              0 |      0 |         8 |                0 |        0 |        42 |
| [`map.area-rules`](test-inventory.md#maparea-rules)                        |        64 |         44 |         0 |      0 |      0 |                                        0 |              0 |      0 |        18 |                0 |        0 |       126 |
| [`map.report`](test-inventory.md#mapreport)                                |         0 |         12 |         0 |      0 |      0 |                                        0 |              1 |      0 |         7 |                0 |        0 |        20 |
| [`city.area`](test-inventory.md#cityarea)                                  |       185 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |         4 |                0 |        0 |       189 |
| [`city.edits`](test-inventory.md#cityedits)                                |       160 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        10 |                0 |        0 |       170 |
| [`data.importer`](test-inventory.md#dataimporter)                          |         0 |          0 |         0 |      0 |      0 |                                      114 |              0 |      0 |         0 |                0 |        0 |       114 |
| [`data.archive`](test-inventory.md#dataarchive)                            |         0 |          0 |         0 |      0 |      0 |                                       50 |              0 |      0 |         0 |                0 |        3 |        53 |
| [`projects.list`](test-inventory.md#projectslist)                          |        22 |          0 |       117 |      0 |      0 |                                        0 |              0 |      0 |        17 |                0 |        0 |       156 |
| [`projects.timeline`](test-inventory.md#projectstimeline)                  |        23 |          0 |        20 |      0 |      0 |                                        0 |              0 |      0 |         7 |                0 |        0 |        50 |
| [`projects.design-archive`](test-inventory.md#projectsdesign-archive)      |         0 |          0 |        29 |      0 |      0 |                                        0 |              0 |      0 |         4 |                0 |        0 |        33 |
| [`projects.share`](test-inventory.md#projectsshare)                        |        15 |          0 |        37 |      0 |      8 |                                        0 |              0 |      0 |         0 |                9 |        0 |        69 |
| [`projects.read-only`](test-inventory.md#projectsread-only)                |         1 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |         5 |                0 |        0 |         6 |
| [`account.sign-in`](test-inventory.md#accountsign-in)                      |        20 |         25 |        15 |      0 |     45 |                                        0 |              0 |      0 |         7 |               12 |        0 |       124 |
| [`account.sync`](test-inventory.md#accountsync)                            |        50 |          0 |        54 |      0 |      9 |                                        0 |              0 |      0 |         0 |               13 |        0 |       126 |
| [`shell.settings`](test-inventory.md#shellsettings)                        |        44 |         30 |        44 |      0 |      0 |                                        0 |              0 |      0 |        84 |                0 |        0 |       202 |
| [`shell.language`](test-inventory.md#shelllanguage)                        |        23 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        21 |                0 |        0 |        44 |
| [`shell.accessibility`](test-inventory.md#shellaccessibility)              |         0 |          0 |         0 |     28 |      0 |                                        0 |              0 |      0 |        31 |                0 |        0 |        59 |
| [`pages.about`](test-inventory.md#pagesabout)                              |        14 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |        16 |                0 |        0 |        30 |
| [`pages.credits`](test-inventory.md#pagescredits)                          |         0 |          0 |         0 |      0 |      0 |                                        0 |              0 |      0 |         0 |                0 |        0 |         0 |
| [`server.delivery`](test-inventory.md#serverdelivery)                      |         0 |          0 |         0 |      0 |     22 |                                        0 |              0 |      0 |         0 |                0 |       18 |        40 |
| **Sum**                                                                    | **2,215** |    **655** |   **564** | **28** | **84** |                                  **188** |          **1** |  **0** | **1,471** |           **34** |   **21** | **5,261** |
| **Distinct mapped tests**                                                  | **2,164** |    **644** |   **564** | **28** | **84** |                                  **181** |          **1** |  **0** |   **552** |           **34** |   **21** | **4,273** |

The sum is 988 above the distinct mapped tests, because [9 Rust files and 8 specs](test-inventory.md#by-feature)
belong to several features: 51 at the model level, 11 at the view-model level, 7 at the `citymodel_builder` and
`cityloom_model` level, and 919 in the UI tier.

## Unmapped tests

1,112 tests belong to no feature. The inventory names their files in [Unmapped](test-inventory.md#unmapped).

| Level                                    | Distinct mapped tests | Unmapped tests | All tests |
| ---------------------------------------- | --------------------: | -------------: | --------: |
| Model                                    |                 2,164 |            164 |     2,328 |
| View-model                               |                   644 |             20 |       664 |
| Component                                |                   564 |            307 |       871 |
| Guard                                    |                    28 |             25 |        53 |
| Server                                   |                    84 |            180 |       264 |
| `citymodel_builder` and `cityloom_model` |                   181 |            167 |       348 |
| Report and PDF                           |                     1 |             23 |        24 |
| Script                                   |                     0 |            226 |       226 |
| UI tier                                  |                   552 |              0 |       552 |
| Integration tier                         |                    34 |              0 |        34 |
| E2E tier                                 |                    21 |              0 |        21 |
| **Total**                                |             **4,273** |      **1,112** | **5,385** |

## Line coverage

The figures were read from SonarCloud (the project `alandovskis_cityloom`) on 23 November 2026 at 08:22 UTC. The last
analysis was then the one of 23 November 2026 at 07:25:52 UTC, of revision `0d9cb8b9`, the commit of the counts above.

- They are the coverage of the host tests only, without `cityloom_server` (`just sonar-reports`, which
  `.github/workflows/sonar.yml` runs). The browser tests and the script tests are not in them.
- `cityloom_server`, `cityloom_report` and `cityloom_report_pdf` are not measured: `sonar.sources` in
  `sonar-project.properties` leaves them out. The code under `cfg(target_arch = "wasm32")` is not measured either.
- The 0.0 of `web/` is not a measure of its tests: the only coverage report is the Rust one, so nothing measures
  `web/`.
- `sonar.coverage.exclusions` leaves out `**/tests.rs`, `i18n_guard.rs` and `scripts/**`. The lines of an inline
  `#[cfg(test)]` module count as lines.

### By crate

| Crate                 | Line coverage (%) | Lines to cover | Uncovered lines |
| --------------------- | ----------------: | -------------: | --------------: |
| `cityloom_editor`     |              95.3 |         79,034 |           3,689 |
| `cityloom_model`      |              99.2 |          1,851 |              15 |
| `citymodel_builder`   |              97.6 |          3,147 |              77 |
| `cityloom_server`     |      not measured |   not measured |    not measured |
| `cityloom_report`     |      not measured |   not measured |    not measured |
| `cityloom_report_pdf` |      not measured |   not measured |    not measured |
| `web/` (not a crate)  |               0.0 |            365 |             365 |
| **The whole project** |          **95.1** |     **84,397** |       **4,146** |

### By slice of the editor

A slice's figures take in its subfolders.

| Slice                                     | Line coverage (%) | Lines to cover | Uncovered lines |
| ----------------------------------------- | ----------------: | -------------: | --------------: |
| `account`                                 |              94.5 |            763 |              42 |
| `archive`                                 |              93.8 |            464 |              29 |
| `city`                                    |              99.3 |          6,246 |              45 |
| `junction`                                |              96.7 |         21,473 |             717 |
| `map`                                     |              96.6 |          8,348 |             287 |
| `place`                                   |             100.0 |            606 |               0 |
| `project`                                 |              92.7 |          1,206 |              88 |
| `restriction`                             |              97.9 |          1,119 |              24 |
| `share`                                   |              93.6 |            629 |              40 |
| `shared` (the shared kernel, not a slice) |              95.0 |          5,791 |             287 |
| `shell`                                   |              79.3 |          1,731 |             358 |
| `standards`                               |              87.8 |          1,152 |             141 |
| `street`                                  |              94.8 |         26,494 |           1,365 |
| `sync`                                    |              96.9 |          1,554 |              48 |
| `timeline`                                |              91.4 |          1,133 |              98 |
| The files at the root of `src/`           |                 – |            325 |             120 |
| **`cityloom_editor`**                     |          **95.3** |     **79,034** |       **3,689** |

The files at the root of `src/` are what the folders leave of the crate's lines. They are mostly `lib.rs`: 180 lines
to cover, 119 of them uncovered, a line coverage of 33.9 %.

## The gaps

A gap is a break of [Where a new test goes](testing.md#where-a-new-test-goes), that is of a row of its table or of its
list of what only the browser tests reach, confirmed in the code. The 23 gaps, each with its issue, are in [Known gaps
and risks](testing.md#known-gaps-and-risks).
