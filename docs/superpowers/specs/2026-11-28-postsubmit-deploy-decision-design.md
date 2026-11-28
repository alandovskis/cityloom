# Postsubmit: deploy only what changes the site

## Intent

Every push to `main` that is not documentation alone runs `.github/workflows/postsubmit.yml` in full and then deploys to
Railway, even when the merge changed nothing the site serves (a tests-only PR from the factory's tests flow, CI
configuration, agent files). Those deploys rebuild and upload the same product. The goal is to skip them.

## Goals, non-goals, anti-goals

- **Goal:** Postsubmit gets a first job that decides whether the deploy runs; it runs only when the push changes an input
  of the deployed site.
- **Goal:** the test code lives under one root, `tests/`, so "is this a test" is a path question: `e2e/` moves to
  `tests/e2e/`.
- **Non-goal:** narrowing the checks on `main`. Every Postsubmit check keeps running every crate and every spec; the new
  job's only consumer is the deploy job.
- **Non-goal:** moving the Rust test files (`**/tests.rs`, `crates/*/tests/`): Cargo decides where they live.
- **Anti-goal:** a push that changes the site and is not deployed. A wasted deploy is acceptable; a missed one is not.
- **Anti-goal:** breaking the factory's merge and deploy loops, which wait on each merge's deploy.

## Constraints

- `ci_changes.py` and its tests are standard-library Python (`just test-scripts`, CI's Script tests job).
- The deploy job keeps its `deploy-production` concurrency group and only ever runs for a push to `main`.
- A PR that moves `e2e/` is not CI configuration only, so it raises the patch version (`scripts/check-version.sh`).
- `target/e2e/` (the pinned server binary, `scripts/pin-server.sh`) is a build output, not the folder: it keeps its name.

## Approaches

Considered, with the person's choice:

- **A. A list of what does not change the site** (anything else deploys). Fails safe, but a test inside a
  `#[cfg(test)]` block of an ordinary source file still deploys.
- **B. A list of what the deploy is built from** (anything else does not deploy). **Chosen.** It states what the deploy
  depends on directly. Its risk is a new input nobody adds to the list, which would silently not deploy; the
  classification test below (my call) turns that into a failing test on the PR that adds the file.
- **C. Hashing the build** and skipping an identical upload. Dropped: the version is compiled into the wasm, so every PR's
  bump changes the hash.

## Part 1: move `e2e/` to `tests/e2e/` (its own PR, first)

`git mv e2e tests/e2e`, keeping its inner layout (`tests/e2e/tests/*.spec.ts`, `tests/e2e/fixtures/`,
`tests/e2e/tiers.json`, `tests/e2e/playwright.config.ts`, `tests/e2e/CLAUDE.md`). Then every reference follows. At least:

- `justfile` recipes (`cd e2e`, `e2e/fixtures/generated`), `scripts/*.sh` and `scripts/*.mjs`.
- `.github/workflows/presubmit.yml`, `postsubmit.yml`, `docs.yml`, `.github/actions/browser-tests/action.yml`
  (`working-directory`, `cache-dependency-path`, `hashFiles`, artifact paths).
- `scripts/ci/spec-map.json`, `scripts/ci/features.json`, `scripts/ci/ci_changes.py` (`RUST_NEUTRAL`, the tiers path)
  and `scripts/include/tests/*` that name the path.
- `.claude/factory/gate.py` (its test globs, `/e2e/tests/` in report paths), `guard.py` if it names the path, the agents
  in `.claude/agents/`, the factory README and skills.
- Paths relative to the folder itself, now one level deeper: anything in `tests/e2e/` that reaches the root with `../`.
- Rust tests and `build.rs` that read the fixtures (`crates/citymodel_builder/tests`, `crates/cityloom_server`).
- `.gitignore`, `.prettierignore`, `sonar-project.properties`, `package.json` scripts if any.
- `.github/dependabot.yml` (its npm entry for `/e2e`).
- `CLAUDE.md` (root and folder ones), `website/docs/**` and its fr-CA copy that name `e2e/`.

Left as they are (my call): `docs/superpowers/plans/` and `docs/superpowers/specs/`, which record what was true when
written.

Done when `git grep -nE '(^|[^a-z_/.-])e2e/'` outside `docs/superpowers/` finds only `target/e2e` and the tier name, and
`just check`, `just test-scripts` and `just docs-build` pass.

**Work in flight on the old layout (the person's choice: rebase after).** The move merges while factory PRs (#312, #326,
#327 and others), unpushed factory worktrees (`../worktrees/clm-<n>`) and Dependabot's #316 (`/e2e`) are on the old
layout. Once it is merged, each is brought onto it by hand, one at a time: rebase onto `main` (git follows the renames;
a spec the branch added under `e2e/tests/` moves to `tests/e2e/tests/`), fix any path the branch itself added, and re-run
its gate from the main checkout (whose `gate.py` now names the new paths) before it goes back to the factory. Dependabot's
PR is closed for Dependabot to open again against the new directory. The merge and integrate agents still stop on such a
conflict; this rebase is done by a person or this session, not by them. The builder lists the branches open at the time.

## Part 2: the deploy decision (second PR)

### `ci_changes.py`

- A new mode, `--event push --base <last deployed commit> [--head HEAD]`, with one output, `deploy` (`true`/`false`),
  written to `GITHUB_OUTPUT`. It uses the existing `changed_files`, so a bare version bump (the editor's `Cargo.toml`
  version and its `Cargo.lock` line) is not a change.
- **The base is the last deployed commit, not the push's `before`** (the person's choice). Diffing from `before` would
  lose a site change whose own run failed: merge A changes `web/`, its checks or its deploy fail, merge B (tests only)
  makes `main` green, and B's diff from A says nothing to deploy. Diffing from the last deployed commit deploys A with B,
  as today, and keeps the deploy agent's **after a cleared failure** rule true.
- `deploy` is true when any changed file matches `DEPLOY_INPUTS`. It is also true when the base is unknown: empty, not a
  commit in the history, or not an ancestor of HEAD (`git merge-base --is-ancestor`).
- The script stays standard-library and offline: the workflow finds the base and passes it.
- `DEPLOY_INPUTS`, what `just deploy` and the deploy job are built from (the builder confirms each against
  `scripts/deploy.sh`, `scripts/build.sh` and the `prepare` recipe):
  - `crates/**`, except `**/tests.rs`, `crates/*/tests/**` and the editor's guards (`crates/cityloom_editor/src/*_guard.rs`).
    Every crate is served: the server links `citymodel_builder`, and the wasm is built from the rest.
  - `web/**`, `deploy/**`, `cities/**`.
  - `Cargo.toml`, `Cargo.lock`, `package.json`, `package-lock.json`, `justfile`.
  - The scripts the deploy runs: `scripts/deploy.sh`, `scripts/build.sh`, `scripts/target-dir.sh`, `scripts/retry.sh`,
    `scripts/check-no-fake.sh`, `scripts/precompress.mjs`, and whatever `just prepare` runs.
  - `.github/workflows/postsubmit.yml` (the deploy job's own steps).
- `NOT_SERVED`, the list on the other side: `tests/**`, the Rust test paths above, `.claude/**`, `.github/**` other than
  `postsubmit.yml`, `scripts/ci/**`, `scripts/include/**`, `scripts/e2e-archive.sh`, `scripts/check-version.sh`,
  `scripts/pin-server.sh`, `.impeccable/**`, `sonar-project.properties`, `rustfmt.toml`, `.prettierrc.json`,
  `.prettierignore`, `.gitignore`, `.env.example`, and documentation (the `docs` rule).

### The classification test (my call)

`test_ci_changes.py` lists every tracked file (`git ls-files`) and fails on a file that is in neither list, or in both,
naming it. The test paths inside `crates/` (and Markdown there) are a third, small list, `CARVED_OUT`: inside an input
folder, not served. A new kind of file must be classified in the PR that adds it, which closes
approach B's gap.

### `postsubmit.yml`

- A first job, `changes` ("Changes"), with `permissions: { contents: read, actions: read }`, checks out with
  `fetch-depth: 0`, finds the last deployed commit and runs `scripts/ci/ci_changes.py --event push --base "$BASE"`. For
  `workflow_dispatch` it runs too; the deploy job already never runs then.
- **Finding the last deployed commit** (a step with `gh` and the `GITHUB_TOKEN`; how exactly is the builder's): the
  newest Postsubmit run on `main`, for a push, older than this one, whose `Deploy to Railway` job concluded `success`;
  its `head_sha` is the base. A run that succeeded with its deploy skipped is not a deploy, so the job's conclusion is
  read, not the run's. When the call fails or finds none, the base is empty, and the push deploys.
- No check job depends on it: they keep running in full.
- `All checks` (`ci`) adds `changes` to its `needs`, so a failed decision fails the run.
- The deploy job: `needs: [changes, ci]`, and
  `if: github.event_name == 'push' && github.ref == 'refs/heads/main' && needs.changes.outputs.deploy == 'true'`.
  A run whose deploy is skipped still concludes `success`.
- The workflow's header comment says what decides the deploy.

### The deploy agent and the factory loops

- `.claude/agents/deploy.md`: a run whose `Deploy to Railway` is `skipped` while every other job succeeded, and whose
  Changes job decided `deploy=false`, is a new result, `not deployed: nothing served changed`. The agent still checks the
  site (200, `/api/me`) and reports it as a success line.
- `.claude/skills/factory-deploy/SKILL.md` records that result as `**Factory · deployed**`, with the reason, and closes
  the issue as for a deploy. Then the merge loop's "deploy is clear" holds unchanged (it reads that record and the run's
  `success`).
- A skipped deploy job does not enter the `deploy-production` concurrency group, so it never cancels a pending deploy of
  an earlier merge. The builder checks this against GitHub's documentation, and adjusts the agent's "cancelled while a
  later run deploys" rule if it does not hold.

### Documentation

`website/docs/testing/when-tests-run.md` and its fr-CA copy say when postsubmit deploys. The CI paragraph of `CLAUDE.md`
gets one line on it.

### Accepted consequence

The live footer shows the version of the last deploy, which can be older than `main`'s after a tests-only merge.

## Left to the builder

- The exact globs of `DEPLOY_INPUTS` and `NOT_SERVED`, within the classification test's rule.
- The wording of the agent's new result line and of the documentation.
- The test cases for the push mode: a tests-only push, a version bump alone, an unknown or non-ancestor base, one deploy
  input among test files, `postsubmit.yml` alone, a site change two commits back behind a tests-only one.
- Part 1 and Part 2 each follow the repository's test-first rule: the failing test comes first where there is one (the
  classification test, the push mode's cases).
