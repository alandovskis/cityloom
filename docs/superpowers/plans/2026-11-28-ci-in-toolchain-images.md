# CI in the toolchain images Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** CI's build and test jobs run inside the `dockerfiles/` toolchain images. The images come from GHCR, tagged by a hash of what they are built from, and are built and pushed only when that tag is missing.

**Architecture:** A reusable workflow, `.github/workflows/images.yml`, computes the tag with `scripts/ci/image_tag.py`. If GHCR already has all four images at that tag it does nothing; otherwise it bakes `dockerfiles/docker-bake.hcl` and pushes them. presubmit.yml, postsubmit.yml and docs.yml call it as a first job named `images`. Their toolchain jobs then run in `container: ghcr.io/<owner>/cityloom-<image>:<tag>` and drop their own toolchain setup steps. The jobs that need no toolchain stay on the runner: `changes`, `scripts`, `version`, `ci`, the deploys and sonar.yml.

**Tech Stack:** GitHub Actions (reusable workflows, `container:` jobs, service containers), Docker Buildx bake, GHCR, Python 3 (standard library only, as the other `scripts/ci` tools).

**Spec:** this conversation's decisions, restated here as the spec:
- Images come from GHCR, tagged by a hash of `dockerfiles/` and the two pins (the `wasm-bindgen` line of `crates/cityloom_editor/Cargo.toml`, `@playwright/test` in `tests/e2e/package-lock.json`). They are built only when the tag is missing.
- In scope are the build and test jobs: Format, Unit tests, Wasm type-check, Release server check, Build for the browser tests, the UI, integration and E2E tiers, and the docs workflow's build. The Railway deploy, Sonar and the Script tests stay as they are.
- The change lands as a new PR stacked on #332 (base branch `docker-toolchain-images`).

## Global Constraints

- Actions are pinned to a full commit SHA with the version in a comment, as every workflow here does (`uses: owner/action@<sha> # vX`). New ones take the latest release.
- The image name is `ghcr.io/${{ github.repository_owner }}/cityloom-<rust|js|docs|e2e>`, and the tag is the 16-hex-character output of `scripts/ci/image_tag.py`.
- Workflow-level `permissions` stay `contents: read`, plus `packages: read`. Only the `images` job gets `packages: write`.
- Images are built for `linux/amd64` only, which is what the runners are.
- `RUSTFLAGS: -D warnings` and the other existing `env` stay as they are. In a container job, `DATABASE_URL`'s host becomes `postgres`, the service's name.
- The change is CI configuration and documentation only (`.github/**`, `scripts/ci/**`, `scripts/include/**`, `docs/**`), so it needs no version bump.
- `just test-scripts` passes, and every test in `scripts/include/tests/test_ci_changes.py` keeps holding. In particular, each workflow's `ci` job lists every other job, `images` included.

## Review Focus

1. **A Dependabot PR that moves Playwright or wasm-bindgen** changes the tag, so the `images` job must push. Dependabot's token is read-only unless the workflow grants it more. Expect `permissions: packages: write` on the `images` job to be enough, and check it on the first such PR. Task 2 makes the job say so plainly when the push is refused.
2. **A pull request from a fork** cannot push either. With the tag present it must pass, since nothing is pushed. With the tag missing, it must fail with a message naming the tag. Task 2.
3. **The Postgres service from inside a container** is reached as `postgres:5432`, not `localhost`. `test`, `integration` and postsubmit's `e2e` tier must use that host. Task 3 tests it.
4. **The server-release positive control** builds into `/target` inside the image, but the step reads `target/release/cityloom_server`. It must ask `scripts/target-dir.sh` instead. Task 3 tests it.
5. **Two runs racing to build the same missing tag** both push the same content under the same tag. That's harmless, and neither run may fail because the other pushed first. `bake --push` overwrites, so no step may depend on the tag being absent after the check. Task 2.

---

### Task 1: The image tag

**Files:**
- Create: `scripts/ci/image_tag.py`
- Test: `scripts/include/tests/test_image_tag.py`

**Interfaces:**
- Produces: `image_tag(root: Path) -> str`, 16 lowercase hex characters. Running `scripts/ci/image_tag.py [--root <dir>]` prints the tag. With `GITHUB_OUTPUT` set, it also appends `tag=<tag>`.

- [ ] **Step 1: Write the failing tests** in `test_image_tag.py`. Each test copies a minimal tree into a temporary directory: `dockerfiles/` (two files and a README.md), `crates/cityloom_editor/Cargo.toml` with `version` and `wasm-bindgen = "0.2.130"`, and `tests/e2e/package-lock.json` with `packages["node_modules/@playwright/test"].version` and one other package.
  - `test_the_same_tree_gives_the_same_tag`: two calls give equal tags matching `^[0-9a-f]{16}$`.
  - `test_a_change_to_a_dockerfile_changes_the_tag`
  - `test_a_change_to_the_dockerfiles_readme_keeps_the_tag`: any `*.md` under `dockerfiles/` is left out.
  - `test_raising_the_editors_version_keeps_the_tag`
  - `test_moving_wasm_bindgen_changes_the_tag`
  - `test_moving_playwright_changes_the_tag`
  - `test_moving_another_e2e_package_keeps_the_tag`
  - `test_a_missing_pin_fails`: no `wasm-bindgen` line, or no `@playwright/test` entry, raises `SystemExit` with a message naming the file.
- [ ] **Step 2:** Run `python3 -m unittest scripts/include/tests/test_image_tag.py`. Expected: they fail because the module is missing.
- [ ] **Step 3: Implement `image_tag`.** Hash with sha256, in this order: each file under `dockerfiles/` except `*.md`, sorted by its POSIX relative path, as the path, a NUL, the content and a NUL; then `wasm-bindgen=<v>\n`; then `playwright=<v>\n`. Read the pin with the regex `rust.Dockerfile` uses (`^wasm-bindgen = "(.*)"$`, multiline). Keep the first 16 hex characters.
- [ ] **Step 4:** Run the tests again, then `just test-scripts`. Expected: they pass, apart from failures already present from untracked local folders.
- [ ] **Step 5: Commit** "Tag the toolchain images by what they are built from".

### Task 2: The images workflow

**Files:**
- Create: `.github/workflows/images.yml`
- Modify: `dockerfiles/docker-bake.hcl` (a `REGISTRY` and `TAG` variable for the tags, and the `org.opencontainers.image.source` label)
- Test: `scripts/include/tests/test_ci_changes.py` (class `Pipelines`)

**Interfaces:**
- Consumes: `scripts/ci/image_tag.py` (Task 1).
- Produces: a workflow triggered by `workflow_call` with the output `tag`. A caller writes `images: { uses: ./.github/workflows/images.yml, permissions: { contents: read, packages: write } }` and reads `needs.images.outputs.tag`. Bake variables: `REGISTRY` (default empty, which gives the local names `cityloom-*` as today) and `TAG` (default `latest`). With `REGISTRY=ghcr.io/<owner>/` the tags are `${REGISTRY}cityloom-<name>:${TAG}`.

- [ ] **Step 1: Write the failing tests** in `Pipelines`:
  - `test_the_images_workflow_is_called_and_gives_the_tag`: images.yml has `workflow_call:` and `tag:` under `outputs`, and runs `scripts/ci/image_tag.py`.
  - `test_the_images_are_pushed_only_when_missing`: images.yml runs `docker buildx imagetools inspect` before `docker buildx bake`, the bake step has an `if:` on the inspect result, and the bake has `--push`.
  - `test_only_the_images_job_may_write_packages`: in presubmit.yml, postsubmit.yml and docs.yml, `packages: write` appears only in the `images` job.
- [ ] **Step 2:** Run `python3 -m unittest scripts/include/tests/test_ci_changes.py -k images`. Expected: they fail.
- [ ] **Step 3:** Add the bake variables and the label. Confirm that `docker buildx bake -f dockerfiles/docker-bake.hcl --print` still names `cityloom-rust` etc. when no variables are set, so the README's commands don't change.
- [ ] **Step 4: Write images.yml.** It has one job, `images`, `runs-on: ubuntu-26.04`. Its steps:
  1. Check out.
  2. Compute the tag with `scripts/ci/image_tag.py`.
  3. Check whether the tag is there: `docker buildx imagetools inspect` for each of the four images. The step's output says whether one is missing.
  4. If one is missing: `docker/login-action` to `ghcr.io` with `github.actor` and `GITHUB_TOKEN`, then `docker/setup-buildx-action`, then `docker buildx bake -f dockerfiles/docker-bake.hcl --push --set '*.platform=linux/amd64' --set '*.cache-from=type=gha' --set '*.cache-to=type=gha,mode=max'` with `REGISTRY` and `TAG` in the environment.

  If the push is refused, the run fails with "could not push the toolchain images at tag <tag>: a pull request from a fork, or a token without packages: write, cannot build them; a maintainer's run will". Pin each action to its latest release's SHA.
- [ ] **Step 5:** Run the tests again, then `just test-scripts`. Expected: they pass.
- [ ] **Step 6: Commit** "Build and push the toolchain images when their tag is missing".

### Task 3: Run the build and test jobs in the images

**Files:**
- Modify: `.github/workflows/presubmit.yml`, `.github/workflows/postsubmit.yml`, `.github/workflows/docs.yml`, `.github/actions/browser-tests/action.yml`
- Test: `scripts/include/tests/test_ci_changes.py` (class `Pipelines`)

**Interfaces:**
- Consumes: the `images` job and `needs.images.outputs.tag` (Task 2).
- Job to image: `format` → e2e (it needs rustfmt, Node and just together); `test`, `wasm`, `server-release` → rust; `build` → e2e (it needs Rust and Node: `just build` compresses); `ui`, `integration`, postsubmit's `e2e` → e2e; docs.yml's `build` → docs.

- [ ] **Step 1: Write the failing tests** in `Pipelines`:
  - `test_the_toolchain_jobs_run_in_the_images`: for each job and image above, the job text has `container:`, `cityloom-<image>:${{ needs.images.outputs.tag }}` and `images` in its `needs`.
  - `test_the_image_jobs_set_up_no_toolchain`: none of those jobs, and not action.yml either, has `dtolnay/rust-toolchain`, `actions/setup-node`, `extractions/setup-just`, `cargo install wasm-bindgen-cli` or `playwright install`.
  - `test_the_runner_jobs_stay_on_the_runner`: `changes`, `scripts`, `version`, `ci` and postsubmit's `deploy` have no `container:`.
  - `test_a_container_job_reaches_postgres_by_its_name`: `test` and `integration` in presubmit, and `test`, `integration` and `e2e` in postsubmit, have `@postgres:5432/` in `DATABASE_URL` and not `@localhost:`.
  - `test_the_positive_control_asks_where_the_target_is`: the `server-release` job runs `scripts/target-dir.sh` and has no `target/release/cityloom_server`.
- [ ] **Step 2:** Run `python3 -m unittest scripts/include/tests/test_ci_changes.py`. Expected: the new tests fail, and the old ones pass.
- [ ] **Step 3: Edit the workflows.**
  - Workflow `permissions` gain `packages: read`.
  - Each moved job gets `container: { image: ghcr.io/${{ github.repository_owner }}/cityloom-<image>:${{ needs.images.outputs.tag }}, credentials: { username: ${{ github.actor }}, password: ${{ secrets.GITHUB_TOKEN }} } }` and `images` in `needs`. `images` is also added to each `ci` job's `needs`.
  - Remove those jobs' toolchain, `setup-node`, `setup-just` and wasm-bindgen cache and install steps. Keep `Swatinem/rust-cache`.
  - In `server-release`, the positive control reads `dir=$(./scripts/target-dir.sh)` and checks `"$dir/release/cityloom_server"`.
  - `DATABASE_URL` becomes `postgres://postgres:postgres@postgres:5432/cityloom`, and the `ports:` mappings go.
  - action.yml keeps `download-artifact`, the `chmod` and `npm ci`, and drops the Node, Playwright cache and install steps. Chromium is in the image at `PLAYWRIGHT_BROWSERS_PATH`.
  - docs.yml gains the `images` job, and its `build` job runs in the docs image.
- [ ] **Step 4:** Run `just test-scripts`. Expected: it passes. Also run `actionlint` on the four files, if it can be run locally (`docker run --rm -v "$PWD":/repo -w /repo rhysd/actionlint:<latest>`). Expected: no findings.
- [ ] **Step 5: Commit** "Run CI's build and test jobs in the toolchain images".

### Task 4: The PR and its first runs

**Files:**
- Modify: `dockerfiles/README.md` (a short "In CI" section: the tag, GHCR, and when images are pushed), `CLAUDE.md` (one sentence in the CI paragraph), `website/docs/testing/when-tests-run.md` and its fr-CA page (one paragraph, in both languages)

- [ ] **Step 1:** Write the README, CLAUDE.md and docs sentences. Run `just docs-build`, or the docs image's build. Expected: both languages build.
- [ ] **Step 2:** Push the branch `ci-toolchain-images` and open the PR against `docker-toolchain-images`.
- [ ] **Step 3:** The first run must build and push all four images (Expected: `images` pushes), and every job must pass in the images. A second push with no change to `dockerfiles/` must skip the bake (Expected: `images` finishes without baking).
- [ ] **Step 4:** Once #332 merges, retarget the PR to `main`.
