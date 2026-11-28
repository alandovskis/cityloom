# Postsubmit deploy decision Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `e2e/` to `tests/e2e/`, then make Postsubmit deploy only when a push changes something the deployed site is built from, compared with the last commit that actually went live.

**Architecture:** Two PRs. PR 1 is a pure move with every reference following it. PR 2 adds a `push` mode to `scripts/ci/ci_changes.py` (an offline, standard-library decision over a list of deploy inputs), a small `scripts/ci/last_deploy.py` that asks GitHub for the last successful `Deploy to Railway` job, a `changes` job in `postsubmit.yml` that gates only the deploy, and a third result ("nothing to deploy") in the deploy agent and the factory's deploy loop.

**Tech Stack:** GitHub Actions, Python 3 standard library (`unittest`, `just test-scripts`), `gh` CLI, Playwright config (TypeScript), Rust integration tests that read the fixtures.

**Spec:** `docs/superpowers/specs/2026-11-28-postsubmit-deploy-decision-design.md`

## Global Constraints

- Every Postsubmit check keeps running every crate and every spec; the `changes` job's only consumer is the deploy job.
- A push that changes the site must never go undeployed; when in doubt (unknown base, failed lookup), deploy.
- `ci_changes.py` stays standard-library and offline; `last_deploy.py` is standard-library plus the `gh` CLI.
- `target/e2e/` and `target/e2e-archive/` are build outputs and keep their names; so does the tier name `e2e`.
- `docs/superpowers/plans/**` and `docs/superpowers/specs/**` written before this plan are not edited.
- PR 1 raises the patch version of `crates/cityloom_editor/Cargo.toml` and runs `cargo update -p cityloom_editor`. PR 2 is documentation and CI configuration only and raises nothing.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`; PR bodies end with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- `just format` before each commit; `RUSTFLAGS=-D warnings` holds in CI.
- Never push `main`; each PR goes up from its own branch.

## Review Focus

1. **A site change behind a failed run:** merge A changes `web/`, its run fails, tests-only merge B goes green. Expect B to deploy (base is the last deployed commit, before A). Pinned by `test_a_site_change_behind_a_tests_only_one_deploys` (Task 4).
2. **A brand-new kind of file:** a PR adds a top-level file nobody classified. Expect `just test-scripts` to fail and name it. Pinned by `test_every_tracked_file_is_classified` (Task 4).
3. **Lookup returns nothing usable:** the API fails, there is no earlier deploy, or the commit has been force-pushed away. Expect an empty base, so the push deploys. Pinned by `test_no_deploy_found_gives_no_base` and `test_a_base_not_behind_head_deploys` (Tasks 4–5).
4. **A skipped-deploy run's own record:** its run succeeds with `Deploy to Railway` skipped. Expect the deploy agent to report `nothing to deploy` and factory-deploy to record it as deployed, so the merge loop keeps going. Covered by the agent and skill text in Task 6. The reviewer checks it against `factory-merge` §2.
5. **A deploy still running when the next push lands:** A's deploy is in progress and B (tests only) arrives. The last deployed commit is older than A, so B also deploys: redundant, never missing. Pinned by `test_a_run_whose_deploy_was_skipped_is_not_a_deploy` and the ordering cases in Task 5.

---

## PR 1: move `e2e/` to `tests/e2e/`

Branch `claude/move-e2e-to-tests` from `origin/main` (`git fetch origin && git switch -c claude/move-e2e-to-tests origin/main`), in this worktree. Commit the spec and this plan in its first commit.

### Task 1: Move the folder and everything that runs from it

**Files:**
- Move: `e2e/` → `tests/e2e/` (`git mv e2e tests/e2e`; `tests/` does not exist yet)
- Modify: `tests/e2e/playwright.config.ts` (`../target/…` → `../../target/…`, `WEB_ROOT: "../web"` → `"../../web"`), `tests/e2e/tests/report.spec.ts:14` (`../../crates` → `../../../crates`). `tests/e2e/tests/fixtures.ts:65` stays (`../fixtures` is inside the folder).
- Modify: `justfile` (every `cd e2e`, `e2e/fixtures/…`; leave `target/e2e`), `scripts/e2e-archive.sh`, `scripts/pin-server.sh` (only paths to the folder, not `target/e2e`)
- Modify: `.github/workflows/presubmit.yml`, `postsubmit.yml`, `docs.yml`, `.github/actions/browser-tests/action.yml` (`working-directory`, `cache-dependency-path`, `hashFiles`, artifact `path:` entries `e2e/fixtures/generated`, `e2e/playwright-report/`, `e2e/test-results/`), `.github/dependabot.yml` (`directory: /e2e` → `/tests/e2e`), `.github/issue-flow.json`
- Modify: `.gitignore`, `.prettierignore`, `sonar-project.properties`
- Modify: `scripts/ci/ci_changes.py` (`RUST_NEUTRAL` `"e2e/tests/**"` → `"tests/e2e/tests/**"`; `main()`'s `ROOT / "e2e"` paths → `ROOT / "tests" / "e2e"`), `scripts/ci/spec-map.json`, `scripts/ci/features.json`
- Modify: `scripts/include/tests/test_ci_changes.py` (`TIERS`, `SPECS`, every `e2e/…` literal, the top-level-entry test: `tests/x` must match a rule), `test_factory_gate.py`, `test_factory_guard.py`, `test_features.py`
- Modify: `.claude/factory/gate.py` (globs at lines 78–80 and 324, `/e2e/tests/` at 213–223), and `.claude/factory/guard.py` only if it names the path
- Modify: `crates/citymodel_builder/tests/{city,commands,prepare,tiles}.rs`, `crates/citymodel_builder/tests/importer/real_city.rs` (`../../e2e/fixtures` → `../../tests/e2e/fixtures`)
- Test: `scripts/include/tests/test_ci_changes.py`

**Interfaces:**
- Produces: the layout `tests/e2e/{tests/*.spec.ts, fixtures/, tiers.json, playwright.config.ts, package.json, CLAUDE.md}`, which PR 2's `NOT_SERVED` names as `tests/**`.

- [ ] **Step 1: Write the failing test** in `Coverage` of `test_ci_changes.py`:

```python
    def test_the_browser_tests_live_under_tests(self):
        self.assertTrue((ROOT / "tests" / "e2e" / "tiers.json").is_file())
        self.assertFalse((ROOT / "e2e").exists())
```

- [ ] **Step 2: Run it.** `python3 -m unittest discover -s scripts/include/tests -k test_the_browser_tests_live_under_tests`. Expected: FAIL (`tests/e2e/tiers.json` missing).
- [ ] **Step 3: `git mv e2e tests/e2e`,** then fix every file listed above. Also remove the stale untracked `e2e/` left behind (`node_modules`, `test-results`, `fixtures/generated`) after checking `git status --ignored e2e` shows only ignored files. Then run `cd tests/e2e && npm ci`.
- [ ] **Step 4: Check that no old path is left.** `git grep -nE '(^|[^a-z_/.-])e2e/' -- ':!docs/superpowers/**' ':!website/**' ':!*.md'`. Expected: only `target/e2e` hits. The documentation is Task 2's.
- [ ] **Step 5: Run the suites.** `just test-scripts`, `just test`, then `just e2e`. Expected: all pass (with `DATABASE_URL` from `just db`).
- [ ] **Step 6: `just format`, then commit** with the message `Move the browser tests to tests/e2e`.

### Task 2: The documentation follows

**Files:**
- Modify: `CLAUDE.md` (the `e2e/` mentions and the "Where the details are" line, `e2e/CLAUDE.md` → `tests/e2e/CLAUDE.md`), `README.md`, `tests/e2e/CLAUDE.md`, `crates/cityloom_editor/src/{archive,junction,map,street}/CLAUDE.md`, `crates/cityloom_server/CLAUDE.md`, `crates/cityloom_server/tests/static_files.rs:1` (doc comment)
- Modify: `.claude/agents/{implement,merge,planner,test-writer,triage,writer}.md`, `.claude/skills/{factory-build,factory-evidence,factory-plan}/SKILL.md`, `.claude/factory/README.md`
- Modify: `website/docs/{architecture/runtime,coding-style,getting-started,project-layout}.md`, `website/docs/testing/*.md`, and the same files under `website/i18n/fr-CA/docusaurus-plugin-content-docs/current/`

- [ ] **Step 1: Find the old paths.** `git grep -nE '(^|[^a-z_/.-])e2e/' -- ':!docs/superpowers/**'`. Expected: only `target/e2e` hits once this task is done.
- [ ] **Step 2: Update each hit.** Rewrite in place, English and fr-CA alike. A path that names `e2e/` as the folder becomes `tests/e2e/`. Wording that names the tier ("the e2e tier") stays.
- [ ] **Step 3: Run the checks.** `just docs-build`, `just test-scripts` and `just format-check`. Expected: pass, with no broken link.
- [ ] **Step 4: Commit** with the message `Point the documentation and agents at tests/e2e`.

### Task 3: Version, PR, and the work in flight

- [ ] **Step 1: Bump the version.** Raise the patch of `crates/cityloom_editor/Cargo.toml`, run `cargo update -p cityloom_editor`, then commit `Raise the version to <x.y.z>`.
- [ ] **Step 2: Open the PR.** Push the branch and open the PR with `gh pr create`. Title: `Move the browser tests to tests/e2e (<x.y.z>)`. The body says what moved, the paths that kept their names (`target/e2e`), and that open factory branches will be rebased afterwards. Then bind it with the ccd_pr tools.
- [ ] **Step 3: List the work on the old layout.** After the user's merge, list the open PRs that touch `e2e/` (`gh pr list --state open --json number,headRefName,files`) and the worktrees under `../worktrees/clm-*`. Give the list to the user, then rebase each one on their go-ahead. For each: `git rebase origin/main`, fix any path the branch itself added under the old folder, and re-run its gate from the main checkout. Close Dependabot's `/e2e` PR, and Dependabot opens it again against `/tests/e2e`.

---

## PR 2: the deploy decision

Branch `claude/postsubmit-deploy-decision` from `origin/main` once PR 1 is merged.

### Task 4: `ci_changes.py` decides whether a push deploys

**Files:**
- Modify: `scripts/ci/ci_changes.py`
- Test: `scripts/include/tests/test_ci_changes.py` (new class `Deploy`)

**Interfaces:**
- Produces, in `ci_changes`:
  - `DEPLOY_INPUTS: tuple[str, ...]`: `crates/**`, `web/**`, `deploy/**`, `cities/**`, `Cargo.toml`, `Cargo.lock`, `package.json`, `package-lock.json`, `justfile`, `scripts/build.sh`, `scripts/deploy.sh`, `scripts/target-dir.sh`, `scripts/retry.sh`, `scripts/check-no-fake.sh`, `scripts/precompress.mjs`, `.github/workflows/postsubmit.yml`.
  - `CARVED_OUT: tuple[str, ...]`, which sits inside an input folder and is not served: `**/tests.rs`, `crates/*/tests/**`, `crates/cityloom_editor/src/*_guard.rs`, `**/*.md`.
  - `NOT_SERVED: tuple[str, ...]`, which holds everything else that is tracked: `tests/**`, `website/**`, `docs/**`, `LICENSE*`, `*.md`, `.claude/**`, `.impeccable/**`, `.github/actions/**`, `.github/ISSUE_TEMPLATE/**`, `.github/dependabot.yml`, `.github/issue-flow.json`, `.github/issue-labeler.yml`, `.github/labels.yml`, `.github/workflows/presubmit.yml`, `.github/workflows/docs.yml`, `.github/workflows/sonar.yml`, `.github/workflows/labels.yml`, `.github/workflows/issue-labels.yml`, `scripts/ci/**`, `scripts/include/**`, `scripts/e2e-archive.sh`, `scripts/check-version.sh`, `scripts/pin-server.sh`, `sonar-project.properties`, `rustfmt.toml`, `.prettierrc.json`, `.prettierignore`, `.gitignore`, `.env.example`. Adjust the list until the classification test passes, putting each file on the side the spec's Intent puts it.
  - `def served(path: str) -> bool`: the path matches `DEPLOY_INPUTS` and no `CARVED_OUT` pattern.
  - `def deploys(files: list[str] | None) -> bool`: `True` when `files is None` (base unknown) or any file is `served`.
  - `def behind(base: str, head: str) -> bool`: `git merge-base --is-ancestor base head`; `False` on any git error.
  - In `main()`, when `--event push`: `files = changed_files(base, head) if behind(base, head) else None`, and the output gets `"deploy": str(deploys(files)).lower()`. The other outputs stay as they are (all `true`, every spec).

- [ ] **Step 1: Write the failing tests** in a new class `Deploy`:

```python
class Deploy(unittest.TestCase):
    def test_tests_alone_do_not_deploy(self):
        self.assertFalse(ci_changes.deploys(["tests/e2e/tests/map.spec.ts", "crates/cityloom_editor/src/street/view/tests.rs",
                                              "crates/citymodel_builder/tests/tiles.rs", "crates/cityloom_editor/src/ui_guard.rs"]))

    def test_ci_configuration_alone_does_not_deploy(self):
        self.assertFalse(ci_changes.deploys([".claude/agents/deploy.md", "scripts/ci/spec-map.json", ".github/workflows/presubmit.yml"]))

    def test_one_input_among_tests_deploys(self):
        self.assertTrue(ci_changes.deploys(["tests/e2e/tests/map.spec.ts", "web/app.css"]))

    def test_each_kind_of_input_deploys(self):
        for f in ("crates/cityloom_server/src/main.rs", "crates/cityloom_server/migrations/0007_x.sql", "web/map.html",
                  "deploy/Dockerfile", "cities/montreal.toml", "Cargo.lock", "package-lock.json", "justfile",
                  "scripts/deploy.sh", "scripts/precompress.mjs", ".github/workflows/postsubmit.yml"):
            with self.subTest(f):
                self.assertTrue(ci_changes.deploys([f]))

    def test_an_unknown_base_deploys(self):
        self.assertTrue(ci_changes.deploys(None))

    def test_a_bare_version_bump_does_not_deploy(self):
        self.assertFalse(ci_changes.deploys([]))  # changed_files has already left the bump out

    def test_a_site_change_behind_a_tests_only_one_deploys(self):
        # base = last deployed commit, so A's web/ change is still in the diff when B adds only tests
        self.assertTrue(ci_changes.deploys(["web/app.js", "tests/e2e/tests/street.spec.ts"]))

    def test_a_base_not_behind_head_deploys(self):
        self.assertFalse(ci_changes.behind("0" * 40, "HEAD"))
        self.assertFalse(ci_changes.behind("HEAD", "HEAD~1"))
        self.assertTrue(ci_changes.behind("HEAD~1", "HEAD"))

    def test_every_tracked_file_is_classified(self):
        known = lambda f: ci_changes.served(f) or any(ci_changes.glob(p, f) for p in ci_changes.NOT_SERVED + ci_changes.CARVED_OUT)
        loose = [f for f in ci_changes.git("ls-files").splitlines() if not known(f)]
        self.assertEqual(loose, [], "classify these in DEPLOY_INPUTS or NOT_SERVED (scripts/ci/ci_changes.py)")

    def test_what_is_not_served_is_no_input(self):
        both = [f for f in ci_changes.git("ls-files").splitlines()
                if any(ci_changes.glob(p, f) for p in ci_changes.NOT_SERVED) and ci_changes.served(f)]
        self.assertEqual(both, [])

    def test_the_tile_cache_key_reads_only_inputs(self):
        # every file the deploy job's tile cache hashes is a deploy input
        key = re.search(r"key: tiles-\$\{\{ hashFiles\(([^)]*)\)", (ROOT / ".github/workflows/postsubmit.yml").read_text())
        for pattern in re.findall(r"'([^'!][^']*)'", key.group(1)):
            with self.subTest(pattern):
                self.assertTrue(any(ci_changes.glob(p, pattern.replace("**", "x.rs")) for p in ci_changes.DEPLOY_INPUTS))
```

- [ ] **Step 2: Run them.** `python3 -m unittest discover -s scripts/include/tests -k Deploy`. Expected: FAIL with `AttributeError: module 'ci_changes' has no attribute 'deploys'`.
- [ ] **Step 3: Implement the constants and functions** from Interfaces in `ci_changes.py`, and extend the module docstring's output list with `deploy` (push only). `behind` checks `REVISION.fullmatch` on both arguments before it calls git, as `changed_files` does.
- [ ] **Step 4: Run the tests.** `just test-scripts`. Expected: all pass.
- [ ] **Step 5: Try it by hand.** `python3 scripts/ci/ci_changes.py --event push --base HEAD~1`. Expected: the JSON holds `"deploy": …`.
- [ ] **Step 6: Commit** with the message `Decide in ci_changes.py whether a push to main deploys`.

### Task 5: Find the last deployed commit, and gate the deploy on it

**Files:**
- Create: `scripts/ci/last_deploy.py`
- Modify: `.github/workflows/postsubmit.yml`, `scripts/ci/spec-map.json` (if `scripts/ci/**` is not already covered by a rule)
- Test: `scripts/include/tests/test_last_deploy.py` (new), `scripts/include/tests/test_ci_changes.py` (`Pipelines`)

**Interfaces:**
- Consumes: `ci_changes.py --event push --base <sha>` and its `deploy` output (Task 4).
- Produces: `last_deploy.last_deployed(runs: list[dict], deploy_conclusion: Callable[[int], str | None], before: int) -> str`. It returns the `head_sha` of the newest run with `id < before` whose deploy conclusion is `"success"`, and `""` when there is none. `runs` is GitHub's `workflow_runs` list (newest first). `deploy_conclusion(run_id)` gives the `conclusion` of that run's job named `Deploy to Railway`, or `None`.
- `main()`: reads `GITHUB_REPOSITORY` and `GITHUB_RUN_ID`, then calls `gh api "repos/<repo>/actions/workflows/postsubmit.yml/runs?branch=main&event=push&per_page=30"`, plus `gh api repos/<repo>/actions/runs/<id>/jobs` for each candidate it checks, newest first, stopping at the first success. It prints the sha, or nothing. **On any error it prints nothing and exits 0.**

- [ ] **Step 1: Write the failing tests** in `test_last_deploy.py`, with `sys.path` set up as `test_ci_changes.py` does:

```python
RUNS = [{"id": 30, "head_sha": "c" * 40}, {"id": 20, "head_sha": "b" * 40}, {"id": 10, "head_sha": "a" * 40}]

class LastDeployed(unittest.TestCase):
    def test_the_newest_earlier_successful_deploy(self):
        self.assertEqual(last_deploy.last_deployed(RUNS, {30: "success", 20: "success"}.get, before=30), "b" * 40)

    def test_a_run_whose_deploy_was_skipped_is_not_a_deploy(self):
        self.assertEqual(last_deploy.last_deployed(RUNS, {20: "skipped", 10: "success"}.get, before=30), "a" * 40)

    def test_failed_and_cancelled_deploys_are_passed_over(self):
        self.assertEqual(last_deploy.last_deployed(RUNS, {20: "failure", 10: "cancelled"}.get, before=99), "")

    def test_no_deploy_found_gives_no_base(self):
        self.assertEqual(last_deploy.last_deployed([], {}.get, before=30), "")

    def test_this_run_and_later_ones_are_not_candidates(self):
        self.assertEqual(last_deploy.last_deployed(RUNS, {30: "success"}.get, before=30), "")
```
Then add these to `Pipelines` in `test_ci_changes.py`. Also change `test_the_deploy_waits_for_every_check` to match `needs: \[changes, ci\]`. `changes` is in `ci`'s needs, so its set equality still holds.

```python
    def test_main_decides_whether_to_deploy(self):
        jobs = self.jobs("postsubmit.yml")
        self.assertIn("scripts/ci/last_deploy.py", jobs["changes"])
        self.assertIn("ci_changes.py --event push", jobs["changes"])
        self.assertIn("actions: read", jobs["changes"])
        self.assertIn("needs.changes.outputs.deploy == 'true'", jobs["deploy"])

    def test_no_check_waits_for_the_decision(self):
        jobs = self.jobs("postsubmit.yml")
        for name, text in jobs.items():
            if name not in ("ci", "deploy", "changes"):
                with self.subTest(name):
                    self.assertNotIn("changes", re.search(r"needs: (.*)", text).group(1) if "needs:" in text else "")
```

- [ ] **Step 2: Run them.** `just test-scripts`. Expected: FAIL (`last_deploy` not found; no `changes` job).
- [ ] **Step 3: Write `scripts/ci/last_deploy.py`.** Standard library, `subprocess.run(["gh", "api", …])`, `json.loads`, with a module docstring like `ci_changes.py`'s.
- [ ] **Step 4: Add the `changes` job to `postsubmit.yml`,** first among the jobs, named `Changes`:
  - `runs-on: ubuntu-26.04`, `timeout-minutes: 5`, `permissions: { contents: read, actions: read }`.
  - Steps: checkout (`fetch-depth: 0`, `persist-credentials: false`), then `id: base` running `echo "sha=$(scripts/ci/last_deploy.py)" >> "$GITHUB_OUTPUT"` with `GH_TOKEN: ${{ github.token }}`, then `id: select` running `scripts/ci/ci_changes.py --event push --base "$BASE"` with `BASE: ${{ steps.base.outputs.sha }}`.
  - `outputs: deploy: ${{ steps.select.outputs.deploy }}`.

  Add `changes` to `ci`'s `needs`. On the deploy job, set `needs: [changes, ci]` and add `&& needs.changes.outputs.deploy == 'true'` to its `if`. Then update the workflow's header comment to say the deploy runs only when a deploy input changed since the last deploy (`scripts/ci/ci_changes.py`, `DEPLOY_INPUTS`).
- [ ] **Step 5: Run the tests.** `just test-scripts`. Expected: pass. Also run `chmod +x scripts/ci/last_deploy.py`. Then try it by hand: `GITHUB_REPOSITORY=alandovskis/cityloom GITHUB_RUN_ID=999999999999 scripts/ci/last_deploy.py`. Expected: prints the sha of main's last successful deploy.
- [ ] **Step 6: Check the concurrency behaviour.** Look in GitHub's documentation on job-level `concurrency` for whether a job skipped by its `if` joins its concurrency group. If it does, a skipped deploy could cancel a pending one: then update `deploy.md`'s "cancelled while a later run deploys" rule so it follows such a merge to the next run that actually deploys, and say so in the PR.
- [ ] **Step 7: Commit** with the message `Deploy main only when a deploy input changed since the last deploy`.

### Task 6: The deploy agent, the deploy loop and the documentation learn "nothing to deploy"

**Files:**
- Modify: `.claude/agents/deploy.md` (step 3's result cases and the report examples), `.claude/skills/factory-deploy/SKILL.md` (§3)
- Modify: `website/docs/testing/when-tests-run.md`, `website/i18n/fr-CA/docusaurus-plugin-content-docs/current/testing/when-tests-run.md`, `CLAUDE.md` (the CI paragraph)

- [ ] **Step 1: Add the new case to `deploy.md` step 3.** When every job succeeded and `Deploy to Railway` is `skipped` (the `Changes` job decided `deploy=false`), the merge changed nothing the site is built from. The agent still checks the site (step 4) and reports `nothing to deploy, <run url>; site 200, /api/me 401`. Add that line to the report examples.
- [ ] **Step 2: Handle the new line in `factory-deploy` §3.** A `nothing to deploy, <run url>` line is recorded and closed exactly like `deployed`: `**Factory · deployed** <run url> (nothing to deploy)`. Check `factory-merge` §2 still reads true with no edit. It does, because the record exists and the run concluded `success`.
- [ ] **Step 3: Update `when-tests-run.md`.** In English and fr-CA (glossary `docs/glossary-fr-ca.md`; OQLF spacing), say that Postsubmit always runs every check and deploys only when a file the site is built from changed since the last successful deploy. The list is `DEPLOY_INPUTS` in `scripts/ci/ci_changes.py`, and a new file must be classified there. Add one line saying the same to the CI paragraph of `CLAUDE.md`.
- [ ] **Step 4: Run the checks.** `just docs-build`, `just test-scripts` and `just format-check`. Expected: pass.
- [ ] **Step 5: Commit** with the message `Teach the deploy agent and loop that a merge may have nothing to deploy`.

### Task 7: PR 2

- [ ] **Step 1: Check that no version bump is needed.** `scripts/ci/ci_changes.py --event pull_request --base origin/main` must print `"version": "false"`, because every file is documentation or CI configuration. If it says `true`, find the file that isn't and raise the patch version.
- [ ] **Step 2: Open the PR.** Push and run `gh pr create`. Title: `Deploy main only when the site changed since the last deploy`. The body gives the base rule, the classification test, the new agent result, and the accepted footer lag. Bind it with the ccd_pr tools.
- [ ] **Step 3: After the user's merge, watch the PR's own Postsubmit run.** That run changes `postsubmit.yml`, a deploy input, so it must deploy. On the next tests-only merge, its `Changes` job must decide `deploy=false` and the run must still conclude `success`.
