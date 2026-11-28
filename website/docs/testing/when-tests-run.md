---
title: When tests run
---

# When tests run

This page says what each workflow runs, and what must pass before a merge. The levels they run are in the
[overview](index.md).

## Presubmit

`.github/workflows/presubmit.yml` runs on every pull request, with `RUSTFLAGS=-D warnings`, so a new compiler warning
fails it. A change to documentation alone (`website/**`, `docs/**`, `**/*.md`, `LICENSE*`) does not start it.

Its `changes` job runs `scripts/ci/ci_changes.py`, which decides what runs from the files the pull request changes:

- The Rust tests of the crates it touches and of the crates that depend on them. A change to something every crate
  reads, such as `web/`, `Cargo.lock`, the `justfile` or a workflow, runs every crate. The wasm check and the
  release-server check follow the same crates.
  A version bump alone (the version line of `crates/cityloom_editor/Cargo.toml` and its entry in `Cargo.lock`) is not a
  change to those files.
- Only the browser specs its files can affect: the first rule of `scripts/ci/spec-map.json` that matches a file
  decides, by group. A file no rule matches runs every spec.
  A page's own files in `web/` run that page's group and `@cross`, the cross-page specs. What every page loads
  (`app.css`, `boot.js`, `city.js`), the server and `crates/citymodel_builder/**`, which makes the fixtures' tiles, run
  every spec.
- Each chosen spec runs in its tier, and a tier with no chosen spec does not run. There is no E2E job.

The jobs are Format, Script tests, Unit tests, Wasm type-check, Release server has no fake provider, Build for the
browser tests, UI tests, Integration tests and Version bump, and `CI`, which gathers them.

## Postsubmit

`.github/workflows/postsubmit.yml` runs on every push to `main` (with the same `paths-ignore`) and by hand. It has no
`changes` job: it runs every job and every spec, with the UI tier in six Playwright shards, and the E2E tier through
`scripts/e2e-archive.sh`. It also sets `RUSTFLAGS=-D warnings`, and its runs are never cancelled. Its `All checks` job
fails when a job failed or was cancelled, and `Deploy to Railway` waits for it. The deploy runs on a push to `main`
only; a run by hand never deploys.

## Documentation

`.github/workflows/docs.yml` runs when a change touches `website/**` or the workflow. It checks `website/` with
Prettier (from `e2e/`), then builds the site in both languages; a broken link or anchor fails the build. It publishes
the site to GitHub Pages from `main`. A change to `docs/**` or to a `*.md` outside `website/` starts no workflow.

## SonarCloud

`.github/workflows/sonar.yml` runs on pull requests and on `main`, with the same `paths-ignore`. It runs
`just sonar-reports`: coverage from the host tests (without `cityloom_server`) and Clippy's findings with
`--cap-lints warn`. It has no `-D warnings`.

## What must pass before a merge

- **`CI`**, presubmit's last job. It fails when any job failed or was cancelled, and passes the ones `changes` skipped.
- **SonarCloud.** It is not part of `CI` and never blocks a merge. The merge agent (`.claude/agents/merge.md`) still
  requires no open issue and no hotspot on the pull request, and the quality gate `OK`. The gate's other conditions are
  coverage of new code of at least 80 % and duplication of at most 3 %.
- **The version.** `scripts/check-version.sh` requires the version in `crates/cityloom_editor/Cargo.toml` to be above
  the base's. A change made only of documentation and CI configuration (`CI_CONFIGURATION` in `ci_changes.py`:
  `.github/**`, `scripts/ci/**`, `scripts/include/**`, `.claude/**`) needs none, and the job is skipped.
- **The Documentation check**, for a change to `website/`. `CI` does not cover it.
- **The merge itself.** The integrate agent (`.claude/agents/integrate.md`) merges only when every check passed or was
  skipped, GitHub says `CLEAN` and `main`'s last Postsubmit run succeeded. The deploy agent (`.claude/agents/deploy.md`)
  then watches the deploy and checks the live site. In the software factory, a person approves the pull request's head
  first.
- **The factory's gates** (`.claude/factory/gate.py`, described in `.claude/factory/README.md`), for an issue the
  factory builds:
  - `red`: the new tests compile, every new test fails, and every old test passes;
  - `green`: every test passes with `-D warnings`, formatting and the version hold, and no test file, `#[cfg(test)]`
    block, check or factory file changed since the red commit;
  - `docs`: only documentation changed, every page has its page in the other language, and the site passes Prettier
    and builds;
  - `tests`, for an issue that asks only for tests of code that already works: only tests changed (and the version
    line), every existing test is as it was, every test passes as for `green`, and each new test fails under at least
    one fault of the code the plan names (mutation: the gate puts each fault in, from `.claude/factory/faults.py` and
    `cargo-mutants`, runs the new tests and takes it back);
  - `faults`: before a plan of that flow is posted, every function, message and declaration its faults name is there
    and yields a fault. It builds nothing.
