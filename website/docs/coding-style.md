---
id: coding-style
title: Coding style
---

# Coding style

This page describes the style the code of CityLoom already has. A rule here is what most of the code does, so some
files differ from it. Where the code has no settled way, the page states no rule.

Each rule ends with what enforces it:

- **Tool**: a formatter, the compiler or a script checks it.
- **Guard**: a test about the code checks it (see [The guards](testing.md#the-guards)); the rule names the test.
- **Review**: only the review of the pull request checks it.

How the code is organised (slices, layers, ports) is in [Architecture](architecture/index.md). How it is tested is in
[Testing](testing.md). The French terms and the spacing are in
[`docs/glossary-fr-ca.md`](https://github.com/alandovskis/cityloom/blob/main/docs/glossary-fr-ca.md). The paths of the
editor's files are relative to `crates/cityloom_editor/src/`.

## What tools decide

- rustfmt lays out the Rust, and Prettier lays out `e2e/`, `web/*.js` and `website/`. `just format` runs both and
  `just format-check` only checks, as CI does (see
  [Tests and formatting](getting-started.md#tests-and-formatting)). Example: `justfile` (`format`). **Tool**
- The code compiles with no warning: CI denies every compiler warning. Example: `.github/workflows/presubmit.yml`
  (`RUSTFLAGS`). **Tool**
- No formatter covers `web/*.css`, the HTML, `scripts/`, the SQL, the `.ftl` files, or the Markdown outside `website/`
  and `e2e/`: these are laid out by hand, like the lines around them. Example: `web/app.css` (`:root`). **Review**

Clippy denies nothing: it runs only for [SonarCloud](testing.md#sonarcloud), with its lints capped at warnings.

## Rust

### Names

- Types, functions and constants have the names Rust expects: the compiler warns about any other. Example:
  `street/model/pick.rs` (`PickRefusal`, `said`). **Tool**
- The type of a view-model ends in `Vm`. Example: `project/vm.rs` (`ProjectsVm`). **Review**
- A trait that adds the editor's rules to a type of `cityloom_model` has the type's name followed by `Ext`. Example:
  `street/model/curb.rs` (`UseExt`). **Review**

### Modules and files

- In the editor, the server, `citymodel_builder` and the two report crates, a file of `src/` opens with a `//!` comment that
  says what the file holds. The crate roots of the server and `citymodel_builder` (`lib.rs`, `main.rs`) have none. Example:
  `shared/iso/format.rs` (the file of `esc`). **Review**
- The imports at the top of a file are in three groups, with a blank line between them: the standard library, other
  crates, then this crate. Example: `project/vm.rs` (the imports above `ProjectsVm`). **Review**
- In `street/`, `junction/` and `map/`, each layer has its own folder (see
  [Layers inside a slice](architecture/index.md#layers-inside-a-slice)). Example: `layout_guard.rs`
  (`layered_slices_keep_their_files_in_model_vm_and_view`). **Guard** (`layout_guard.rs`)

### Errors and refusals

- There is no `unsafe`. Example: `crates/cityloom_server/src/crypto.rs` (`seal`) encrypts without it. **Review**
- The message of an `expect` states the fact that makes the failure impossible, in lower case. Example:
  `crates/cityloom_server/src/crypto.rs` (`seal`: `"a key is 32 bytes"`). **Review**
- Outside the tests, `unwrap()` is for a `write!` into a `String` and for the lock of a mutex. Input is never
  unwrapped. Examples: `map/view/svg.rs` (`map_svg`), `crates/cityloom_server/src/clock.rs` (`FixedClock`). **Review**
- A refusal of the model is an enum with one variant per reason. Example: `street/model/pick.rs` (`PickRefusal`).
  **Review**
- A refusal holds no wording: its words are a Fluent message, reached by key. Example: `street/model/pick.rs`
  (`PickRefusal::said`). **Guard** (`i18n_guard.rs`) for the files of its `MIGRATED` list, **Review** for any other.

### Ownership and shared state

- The editor shares a value with `Rc`; the server and `citymodel_builder` share it with `Arc`. Examples: `street/vm/watch.rs`
  (`SheetWatch`), `crates/cityloom_server/src/app.rs` (`Account`). **Review**
- A value that is not `Send` goes in a `StoredValue::new_local`; `StoredValue::new` holds plain values. Example:
  `street/vm/watch.rs` (`SheetWatch::new`). **Tool**
- A view keeps its view-model in a `Copy` handle over an `Rc`. Examples: `junction/vm/watch.rs` (`Watch`),
  `street/vm/watch.rs` (`SheetWatch`), `shared/bind.rs` (`Bound`). **Review**

### Constants and units

- A constant of a model that holds a length, a time or a speed ends in its unit: `_MM` or `_M`, `_S`, `_KMH`. Examples:
  `street/model/calming.rs` (`RAMP_MM`), `junction/model/mod.rs` (`GREEN_MIN_S`),
  `crates/citymodel_builder/src/importer/merge.rs` (`MAX_APART_M`). **Review**
- A length of a street or a junction, as a field or a parameter of the models, is a whole number of millimetres in an
  `i32`, and its name ends in `_mm`. The drawing code computes in `f64`. The lengths of the imported network are metres
  in `f64`, named `_m`. Examples: `crates/cityloom_model/src/street/calming.rs` (`length_mm`),
  `crates/cityloom_model/src/lane.rs` (`width_m`). **Review**
- A length is written for the person by `Units::*_in(…, locale)`, never by `format!("{:.1}")`, and a number the person
  types is read by `parse_number`. Example: `shared/units.rs` (`length_in`, `parse_number`). **Review**

## Leptos and SVG

- A view binds to its view-model and forwards events, and a view-model reaches the browser only through the ports (see
  [Layers inside a slice](architecture/index.md#layers-inside-a-slice) and [Ports](architecture/index.md#ports)).
  Example: `project/vm.rs` (`ProjectsVm`). **Review**
- A component is named in `PascalCase`. Example: `map/view/network_view.rs` (`NetworkChecks`). **Review**
- A renderer is a function from what is to be drawn to SVG markup, with no Leptos, no signal and no DOM. Example:
  `street/view/plan/mod.rs` (`plan_svg`). **Review**
- A view sets that markup with `inner_html`. Example: `street/view/plan/component.rs` (`inner_html`). **Review**
- Text put into markup goes through `esc`. Example: `street/view/plan/mod.rs` (`thing`). **Review**
- A coordinate is rounded before it is written into markup, by a helper such as `f1`. Example: `shared/iso/format.rs`
  (`f1`). **Review**

## Comments

- A documentation comment begins with a capital and ends with a period. Example: `street/view/plan/mod.rs`
  (`plan_svg`). **Review**
- It says what the thing is, with a noun phrase ("The…", "A…", "What…", "Whether…"), or what it does, with a verb in the
  third person ("Keeps…", "Draws…"). Example: `shared/iso/format.rs` (`f1`). **Review**
- The Rust has only line comments, no block comments. Example: `shared/iso/format.rs` (`esc`). **Review**
- No `TODO` or `FIXME` is left in the Rust. Example: `shared/iso/format.rs` (`f1`). **Review**

## Tests

Where a new test goes is in [Testing](testing.md#where-a-new-test-goes).

- The name of a Rust test is a sentence that states the behaviour, with no `test_` prefix. Example:
  `crates/cityloom_server/src/crypto.rs` (`two_seals_of_one_value_differ`). **Review**
- The unit tests of a file are in a `mod tests` at its end, with `use super::*`. Example: `shared/iso/format.rs`
  (`tests`). **Review**
- A fake of a port is named for what it does in place of the port: `Fake…`, `Recording…`, `Memory…`, `Manual…`,
  `Fixed…`. Examples: `shared/ports.rs` (`RecordingAnnouncer`), `crates/cityloom_server/src/clock.rs` (`FixedClock`).
  **Review**

## Other languages

### JavaScript

What belongs in JavaScript is in [Architecture](architecture/index.md#javascript).

- A script of `web/` opens with a comment that says what it does. Example: `web/mapcanvas.js` (the comment above
  `createMapCanvas`). **Review**
- The scripts are ES modules, except `boot.js`, which is a classic script. Example: `web/mapcanvas.js`
  (`createMapCanvas`). **Review**
- They declare with `const` and `let`, never `var`, and compare with `===`, never `==`. Example: `web/mapcanvas.js`
  (`here`, `themeNow`). **Review**

### TypeScript

- A spec takes `test` and `expect` from `./fixtures`. Example: `e2e/tests/shell.spec.ts` (`expect`, `test`). **Review**
- The title of a spec's test is a sentence that states the behaviour. It begins in lower case, unless its first word is
  a name such as a key, a control or a language, and has no final period. Example: `e2e/tests/street.spec.ts`
  (`a piece is selected with the arrow keys, announced, and shown in the details`). **Review**

### Python

- A Python file of `scripts/` has a docstring at its top and `from __future__ import annotations`, and uses only the standard
  library. Example: `scripts/ci/ci_changes.py` (`CI_CONFIGURATION`). **Review**
- A test is a method of a `unittest.TestCase`, named `test_` and a sentence. Example:
  `scripts/include/tests/test_factory_gate.py` (`test_the_red_gate_passes_a_red_commit_whose_new_test_fails`).
  **Review**

### Shell

- A script is POSIX `sh`, with a comment at its top that says what it does, and `set -eu`. Example:
  `scripts/check-version.sh` (`version_of`). **Review**

### CSS

- A stylesheet opens with a comment. Example: `web/app.css` (above `:root`). **Review**
- A colour is a custom property, used with `var(--…)`. Example: `web/app.css` (`--accent`). **Review**
- A colour pair for text or a control is measured in `shared/contrast.rs`. Example: `shared/contrast.rs` (`contrast`).
  **Guard** (`shared/contrast.rs`) for the pairs it lists, **Review** for any other.
- A class is named in kebab case. Example: `web/editor.css` (`insp-meter`). **Review**

### SQL migrations

- A migration is named `000N_name.sql` and opens with a comment. Its SQL is in lower case and its names are in snake
  case. Example: `crates/cityloom_server/migrations/0005_shares.sql` (`shares_user`). **Review**
- A merged migration is not changed: a new one is added. Example:
  `crates/cityloom_server/migrations/0006_share_bounds.sql` drops the index `shares_user`, which `0005_shares.sql` made.
  **Review**

### Fluent messages

- A `.ftl` file opens with a comment that says what it words. Example: `project/i18n/en.ftl` (the comment at its top).
  **Review**
- The id of a message is in kebab case and a variable is in snake case. Example: `junction/i18n/en.ftl`
  (`jn-calls-said`, `$not_served`). **Review**
- A message is a whole sentence with named placeables, never pieces put together, and a plural is a selector (`[one]` /
  `*[other]`). Example: `project/i18n/en.ftl` (`project-delete-confirm-links`). **Review**
- An id used in a file of the guard's `MIGRATED` list exists in both languages. Example: `i18n_guard.rs`
  (`every_message_id_used_in_a_migrated_file_exists_in_both_languages`). **Guard** (`i18n_guard.rs`)
- Both languages have the same ids, with the same variables. Example: `shared/i18n.rs` (`parity_problems`). **Guard**
  (the parity test of the slice), **Review** for `timeline` and `standards`, which have no parity test (see
  [Translation](testing.md#translation)).

## Commits and pull requests

- The title of a pull request is one line that says what the change does. It begins with a capital and has no final
  period and no type prefix such as `feat:`. Example: `.claude/skills/factory-evidence/SKILL.md`
  (`<what it does> (<version>) (CLM-<n>)`). **Review**
- After what the change does, the title gives the version, when the pull request raises it, and then the issue's key.
  GitHub adds the number. Example: `Keep a junction's folded options reachable by heading (0.113.1) (CLM-50) (#289)`.
  **Review**
- A pull request does what its issue asks and nothing else: no refactor, rename or fix elsewhere. Example:
  `.claude/agents/reviewer.md` (Scope). **Review**
- Each change is a commit of its own, made once its tests pass. Example: `CLAUDE.md` (Conventions and gotchas).
  **Review**
- A pull request raises the version in `crates/cityloom_editor/Cargo.toml` (major for a breaking change, minor for a
  feature, patch for a fix) and updates `Cargo.lock`, unless it changes only documentation and CI configuration (see
  [What must pass before a merge](testing.md#what-must-pass-before-a-merge)). Example: `scripts/check-version.sh`
  (`version_of`). **Tool** (presubmit's Version bump job) for the version, **Review** for `Cargo.lock`.
