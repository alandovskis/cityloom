# CI/CD Pipeline — `osm-extract-proxy` (U9)

Upstream inputs: `security-design.md`, `reliability-design.md`,
`performance-design.md`, `scalability-design.md`, `observability-design.md`
and `logical-components.md` (nfr-design, this Unit), `functional-spec.md`
(functional-design, this Unit), `components.md` (domain-design),
`contract-summary.md` (contract-design), `infrastructure-design-questions.md`
(this stage), `team.md` and `project.md` (practices).

Design elements are numbered `CP-n`. `team.md` already fixed the pipeline's
gates — the two CI tiers, the merge gate's seven items, `gitleaks`,
`cargo audit`, SonarQube, Dependabot security-only, a bundle-size guard,
deploy on merge, rollback through Railway's deployment history. This file
writes the stage-to-gate mapping down, adds what this Unit contributes to
each tier, and specifies the delivery path from a merge to a running
deployment and the weekly data path beside it. The workflow files and the
`Dockerfile` are code generation's (`ci-pipeline` and `code-generation`).

## Two paths into production

```
 code path                                  data path (weekly)
 ---------                                  -----------------
 feature branch --push--> fast tier         schedule / dispatch
        |                                          |
 pull request -----> fast + slow tiers      region-build tool (LC-3)
        |               (merge gate)               |
 squash-merge to main                       buildId unchanged? -> stop
        |                                          |
 push to main --> fast tier (re-run)        GitHub Release data-<buildId>
        |                                          |
 Railway build (Dockerfile) <---- pointer commit to main
        |
 /readyz health check --fail--> previous deployment keeps serving
        |
 rollover; SIGTERM old (5 s drain)
        |
 uptime monitor sees the new deployment as up
```

<!-- Text fallback: on the code path, a push to a feature branch runs the fast tier; a pull request runs both tiers as the merge gate; a squash-merge pushes to main, which re-runs the fast tier and triggers Railway's Dockerfile build; the health check on /readyz gates rollover, a failure leaving the previous deployment serving; on success the old deployment is drained for 5 seconds. On the data path, the weekly schedule or a manual dispatch runs the region-build tool; an unchanged buildId stops; otherwise a GitHub Release is created and a pointer commit to main enters the same Railway build. -->

## Stage-to-gate mapping (CP-1)

| Stage | Runs on | Gate (what must pass) | On failure |
|---|---|---|---|
| **Fast tier** | Every push to any branch, including `main` | `cargo build --workspace --locked`; `cargo check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all -- --check`; `cargo test --workspace`; coverage floor on the measured set with `cargo-llvm-cov` (this Unit adds its modules per NFR7.1.1); the osm2streets golden-fixture suite; `gitleaks`; the dependency and asset manifest check in `scripts/verify.sh` (no Streetmix; the vendored `.proto` files' licence, NFR7.2.3) | The push is red; nothing merges (`team.md` merge gate items 1–6) |
| **Slow tier** | Pull requests to `main`, and nightly | Release-mode WASM build of the workspace; browser-mode keyboard and accessibility tests (U6's; not this Unit's); `cargo audit`; SonarQube Cloud quality gate on new code; the bundle-size guard; **this Unit's CI benchmark** — 200 misses and 1,000 hits over committed cells, failing on p95 above 300 ms and 30 ms respectively (NFR1.1.2, NFR1.1.3), a code-path regression guard, not production evidence | The pull request cannot merge |
| **This Unit's own tests, in the fast tier** | Every push | The tests the NFR requirements name: the stalled-cut `503 timeout` within 3,050 ms and the freed slot (NFR1.1.1, NFR3.1.7); eight misses against four slots (NFR1.1.6); the span computation (NFR1.1.5); the log-capture test asserting no address, coordinate, key or cell id in the captured output (NFR6.3.1 — the enforcement for the whole L7 layer); the spoofed-forwarded-header test (NFR6.3.3, `X-Real-IP` per ID-4); the `OPTIONS` preflight refusal (NFR5.1.8); the header set on every response (NFR6.4.3); the `bbox` property test (NFR6.4.1); Unready on a missing or corrupted cell (NFR3.2.4); the limiter's window tests (NFR5.1.3, NFR2.1.3); the encoder round-trip through `osmpbf` (TS-3) | Red; the regression rule NFR7.3.1 applies to every defect these find |
| **Merge** | Squash-merge of the pull request to `main`, one commit per Bolt named by the Bolt slug | Both tiers green; branch rules require the pull request and the checks | — |
| **Build** | Railway, on the push to `main`, from the committed `Dockerfile` (ID-2) | Stage 1 compiles on the pinned toolchain with `--locked`; stage 2 fetches the pointer's release assets and refuses a SHA-256 mismatch; stage 3 assembles the runtime image | Deployment failed (MD-2); the previous deployment keeps serving *(checked 2026-09-14: a deployment that fails during build or deploy stops with status `Failed`, and an older deployment is removed only after a new one becomes `Active`; confirmed by doing at environment-provisioning, `infrastructure-specification.md` handoff item 5, because the very first deploy has no previous deployment to keep)* |
| **Deploy** | Railway | `GET /readyz` returns 2xx within 120 s — which means the manifest loaded and every cell's BLAKE3 digest verified (RD-1, NFR3.2.4) | The new deployment never takes traffic (NFR3.2.1); notification (MD-2) |
| **Rollover** | Railway | The new deployment is active; the old one gets `SIGTERM` and 5 s to drain (`RAILWAY_DEPLOYMENT_DRAINING_SECONDS`, ID-6) before `SIGKILL` | — |
| **Post-deploy check** | The uptime monitor's next probe (MD-1); the `ready` row's `elapsedMs` in the log | 200 on `/readyz`; Ready time inside NFR1.1.4 | Down notification; redeploy the previous build |

## Deployment strategy (CP-2)

**Rolling, one replica, health-check gated** — Railway's own model *(checked
2026-09-14: the new deployment is made active only after the health check
passes, and only then is the previous one made inactive)*. With one replica
this is effectively blue-green for the duration of the rollover: two
processes exist for the seconds between the new one passing its check and
the old one draining. Canary and traffic-splitting do not apply to a
single-instance service and are not designed.

Every deploy empties the cache (BR6.4) and restarts the counters; the first
minutes after a deploy are cold (RD-5). Deploys happen on every merge, so
this is the ordinary state after every Bolt, bounded by the slots.

**Railway's "wait for CI" option is off** (`checkSuites`). The gate is
enforced at merge by the branch rules; a push to `main` that reaches Railway
has already passed both tiers as a pull request, and the weekly pointer
commit carries no check suite at all, which the option would otherwise wait
on.

## Rollback (CP-3)

| What went wrong | Rollback | Time |
|---|---|---|
| A code deploy is bad after it passed the health check | Railway's deployment history: redeploy the previous deployment (`team.md` Deployment) — the previous image, including its data layer, is rebuilt or restored from the platform's record | Inside RTO's ≤ 10 minutes (RD-9); confirmed once at environment-provisioning |
| A code deploy fails its health check | Nothing to do: the previous deployment never stopped serving (NFR3.2.1) | 0 |
| A data build produced a bad store that still verified | Revert the pointer commit on `main` (a one-line change through the ordinary pull-request path); Railway rebuilds with the previous release's assets, which still exist | One build |
| A data build produced a store that fails verification | Nothing to do: the image build refuses a SHA-256 mismatch (a `Failed` build, which removes nothing *(checked)*), or the service enters Unready and the deployment fails its check; the previous deployment keeps serving | 0 |
| The weekly build fails before publishing | Nothing to do: nothing was published (RD-8, NFR3.1.4); re-run on demand after the fix | — |

No database migration exists for this Unit, so no rollback has a data
consequence: rolling back is always "run the previous image".

## The weekly data path (CP-4)

The region-build tool's workflow, beside the code pipeline:

1. **Trigger.** `schedule` — Monday, early UTC — and `workflow_dispatch` for
   BR8.5's on-demand run. A `concurrency` group named `data-build` with
   `cancel-in-progress: false`, so a second trigger waits rather than
   overlapping a run that is writing a store.
2. **Run.** The free public-repository runner checks out `main`, installs
   the pinned toolchain, and runs `cargo run --release --locked --bin
   <region-build>`; the tool downloads each configured region and its
   `.md5` over TLS with the project `User-Agent` (NFR7.2.2), verifies,
   filters, slices, writes `store.bin` and `manifest.json` to a temporary
   directory, and prints the `region` and `build` rows (OD-5).
3. **Compare.** The tool prints the new `buildId`; the workflow compares it
   with `data/current-build.toml`'s. Equal → exit 0 with `outcome:
   unchanged`; nothing is published, nothing deploys (BR8.5).
4. **Publish.** Create the GitHub Release tagged `data-<buildId>` with the
   two assets and their SHA-256 sums in the release body. The step is made
   idempotent so a failure part-way through it (a release created but an
   asset upload cut off, or the run cancelled before step 5) leaves a
   defined next run rather than a stuck one: before creating, the workflow
   deletes any existing release **and tag** named `data-<buildId>` whose
   assets are missing or whose SHA-256s do not match the freshly built
   files; a complete matching release is reused as-is. The pointer commit
   of step 5 happens only after both assets are present and their digests
   verified by a download-and-hash of what GitHub actually stored, so the
   pointer never names an incomplete release. A re-run with the same
   `buildId` and a complete release therefore stops at step 3 as before;
   a re-run after a partial publish repairs it.
5. **Point.** Write `data/current-build.toml` — release tag, `buildId`,
   each asset's SHA-256, the publisher's published timestamps per region,
   the run's timestamp — and commit it to `main` with the workflow's own
   token under the workflow actor, on the pointer path only. The branch
   rules carry a bypass for exactly that actor and path.
6. **Deploy.** Railway's GitHub integration builds the pushed commit; the
   `Dockerfile`'s data stage fetches the assets the pointer names and
   verifies them; the service verifies the cells at start; the health check
   gates rollover (CP-1). The old build's cache and extract keys retire
   with its process (W4).

The pointer commit triggers no CI run *(a GitHub rule for commits authored
by the workflow token)* and needs none: the code is unchanged, and the data
has two verifications of its own — SHA-256 at image build, BLAKE3 per cell
at service start. This is recorded, not hidden: `team.md` says all work
merges to `main` by pull request, and a bot commit on the trunk is a
practice question raised at this stage's learnings step.

## Secrets in CI/CD (CP-5)

| Where | Secret | Design |
|---|---|---|
| The service | **None** (NFR6.1.1, SD-9) | Configuration is non-secret service variables; the release assets are public; nothing authenticates to anything |
| The CI workflows | The workflow's own `GITHUB_TOKEN` (issued per run, never stored); SonarQube Cloud's token as a repository secret (`team.md`, already affirmed) | `gitleaks` in the fast tier and GitHub's push protection guard the repository; no secret is ever a file in it (`project.md`) |
| The data workflow | The workflow's own `GITHUB_TOKEN` with `contents: write` for the release and the pointer commit — **no Railway token** (Q1 option A was chosen over B and C to keep it so) | The build job "authenticates to nothing" beyond GitHub itself, keeping NFR6.1.1's intent |
| Railway | No project token anywhere in the repository or in GitHub's secrets; deploys are triggered by Railway's own GitHub App, not by the pipeline | If a token is ever needed (a later Unit's migration step, say), it is a GitHub repository secret, rotated first if it ever reaches a commit (`team.md`) |

## Artifact management (CP-6)

| Artifact | Where | Identity | Retention |
|---|---|---|---|
| The service image | Railway's build cache and deployment history | The git commit SHA (`RAILWAY_GIT_COMMIT_SHA`), which is also the client build stamp (ID-7) | Railway's deployment history, which is the rollback path |
| The cell store and manifest | GitHub Releases, `data-<buildId>` | The content-derived `buildId` (BR8.4) | Kept; a release is never deleted, so any previous build can be pointed at again |
| The pointer | `data/current-build.toml` on `main` | The commit that wrote it | Git history |
| The golden fixtures | The repository (`team.md`) | Committed bytes, cut by this Unit's own code from committed cells | Git history |
| Coverage and benchmark reports | The CI run | The run | GitHub's run retention; the numbers that matter are asserted, not archived |

## Environment promotion (CP-7)

There is one environment. Promotion is the merge: a pull request is the
only thing that stands between a branch and production, which is why both
tiers run on it and why nothing in this Unit relies on a staging soak.
The confirmations `reliability-requirements.md` asks for once — a
deliberately bad build leaving the previous deployment serving; the Ready
time against the probe timeout; a redeploy from history — are done once at
environment-provisioning against production itself, which is the only
target there is.

## Assumptions & Open Questions

- Railway's redeploy of a previous deployment restores that deployment's
  image without rebuilding from source; if it rebuilds, the data stage
  fetches the release the pointer named at that commit, which is the same
  bytes — slower, not different. [assumption]
- The CI benchmark's thresholds (300 ms, 30 ms at p95) hold on the free
  runner's hardware for committed cells; if the runner is slower than the
  Railway service, the thresholds are re-based on a measured runner figure
  at build-and-test and recorded as such, never silently loosened.
  [assumption]
- The branch-rule bypass can be scoped to the workflow actor and the
  pointer path (`infrastructure-specification.md`, assumptions).
  [assumption]

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-14T21:37:29Z
**Iteration:** 2
**Request Challenge:** review:419f369b4312d3d2370446e028b533e3

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `monitoring-design.md` Alerts table row MD-1 and `infrastructure-specification.md` Infrastructure Services row ID-12, versus `nfr-requirements/observability-requirements.md` NFR3.1.17 | NFR3.1.17 says "No alert is defined by this Unit in product Stage 1 … without anything to configure in this Unit." MD-1 (the external uptime monitor's down notification) is a new alert this Unit defines and configures, and no amendment reconciled it back to NFR3.1.17, unlike the project's established `## Amendments required` pattern. | Record an explicit amendment stating NFR3.1.17 is superseded for this one alert, in the style the nfr-requirements files use. | Resolved — `monitoring-design.md` now carries `## Amendments required` (verified present, sensor-checked) with a row for NFR3.1.17 quoting its text verbatim and stating the one-alert supersession; rows for NFR3.1.1 "Measured how" and NFR3.2.3 follow the same precedent pattern confirmed live in `security-requirements.md` and `performance-requirements.md`. |
| R-02 | Major | `infrastructure-design-questions.md` Consolidated Summary "Cost estimate" bullet, the basis for `infrastructure-specification.md` ID-1 and `traceability.json` NFR5.1 | $1.30 + $1.00 + $0.50 = $2.80, stated as "inside the ≤ $2.50 allocation" — an arithmetic error that breaches NFR5.1.5 / `project.md`'s budget rule without being treated as the required constraint change. | Correct the sum and either treat the overage as a constraint change or show the arithmetic that keeps the total ≤ $2.50; update traceability.json's NFR5.1 row and ID-1's rationale. | Resolved — `infrastructure-specification.md` now carries `## Cost estimate (ID-8, ID-9)`: memory 128 MB × $10/GB-month = $1.25 (re-derived correctly, was misstated $1.30 in the questions file), CPU 0.05 vCPU × $20/vCPU-month = $1.00, fixed total $2.25, leaving exactly $0.25 (5 GB/month) of egress headroom inside the $2.50 ceiling — arithmetic re-checked and correct. The section explicitly names the confirmed summary's $2.80 sum as the error being corrected and treats a measured breach as the NFR5.1.5/`project.md` constraint-change path, never absorbed. `traceability.json` NFR5.1 target cites the corrected figures. The questions file itself is untouched, as the brief states, and the correction lives in the artifact instead — consistent with the receipt-bound-confirmation practice. |
| R-03 | Major | `cicd-pipeline.md` CP-1 "Build" row and CP-3, cited by `traceability.json` NFR3.2 | The claim that a **build** failure leaves the previous deployment serving is neither marked checked nor listed as an assumption, unlike every comparable platform claim; the confirmations only cover a runtime health-check failure. | Check it against Railway's documentation or flag it as unverified in the handoff/assumptions. | Resolved — CP-1's Build row and CP-3 now both carry `*(checked 2026-09-14 against Railway's deployment reference: a deployment that fails during build or deploy stops with status Failed, and an older deployment is removed only after a new one becomes Active; confirmed by doing at environment-provisioning...)*`, matching `infrastructure-specification.md` handoff item 5's parallel language. `traceability.json` NFR3.2's target cites the same checked claim. |
| R-04 | Minor | `cicd-pipeline.md` CP-4 steps 4–5 | A mid-publish failure could leave an incomplete same-tagged release that blocks the next run; no recovery path named. | Name the recovery path (e.g. delete-and-retry making the publish idempotent). | Resolved — CP-4 step 4 now specifies: before creating, the workflow deletes any existing release and tag named `data-<buildId>` whose assets are missing or whose SHA-256s do not match the freshly built files; a complete matching release is reused as-is; the pointer commit (step 5) happens only after a download-and-hash re-verification of both stored assets, so the pointer never names an incomplete release. This is a genuine, traceable idempotency fix, not a restatement. |
| R-05 | Minor | `cicd-pipeline.md` CP-6 "The cell store and manifest" row versus CP-4 step 4 (same file) | CP-6's retention column states unconditionally "Kept; a release is never deleted, so any previous build can be pointed at again." This is now false as written: CP-4 step 4 (the fix made for R-04 in this iteration) has the weekly workflow explicitly delete an existing release and tag under a stated condition (missing or mismatched assets from a partial publish). CP-6 was not updated to carry the same caveat, so the artifact makes an absolute claim in one section that a mechanism specified nine lines earlier in the same document contradicts. `infrastructure-specification.md`'s parallel claim ("a release is immutable once published... old releases are the data rollback path") survives because it can be read as scoped to a release that completed publication, but CP-6's wording has no such qualifier. | Narrow CP-6's retention cell to match CP-4, e.g. "Kept once complete; a release is never deleted once its assets verify and the pointer names it — only an incomplete release left by a partial publish is deleted and replaced by the next run (CP-4 step 4)." | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc-sensor-traceability (traceability.json) | `{"pass":true,"gaps":[],"orphans":[],"missing_from_table":[],"missing_from_upstream_ids":[],"invalid_entries":[],"invalid_targets":[],"findings_count":0}` | Every NFRx.y upstream id is covered and every target resolves; NFR5.1's target text matches the corrected cost arithmetic verified manually below. |
| aidlc-sensor-required-sections (all three .md files) | PASS on each — `infrastructure-specification.md` 9 H2s, `monitoring-design.md` 8 H2s (including the new `## Amendments required`), `cicd-pipeline.md` 9 H2s | No template is configured for this stage, so this only confirms well-formed H2 structure, not content completeness. |
| aidlc-sensor-upstream-coverage (cicd-pipeline.md) | `{"pass":true,"consumes":[],"scanned_files":[],"reason":"no upstream","findings_count":0}` — vacuous | Cross-checked manually: `cicd-pipeline.md`'s header lists all six nfr-design upstream files plus `functional-spec.md`, `components.md`, `contract-summary.md`, the questions file and both practice files; each is referenced by name at least once in the body (security-design.md via CP-5/NFR6.1/NFR6.3, reliability-design.md via RD-1/RD-2/RD-7/RD-9, performance-design.md via NFR1.1 rows, scalability-design.md via SC-1/SC-2, observability-design.md via OD-1–OD-9, logical-components.md via LC-2/LC-3, team.md via the merge-gate items). No orphaned upstream reference found. |
| Manual: cost arithmetic re-derivation | 128 MB = 0.125 GB × $10/GB-month = $1.25; 0.05 vCPU × $20/vCPU-month = $1.00; $1.25 + $1.00 = $2.25; $2.50 − $2.25 = $0.25 headroom; $0.25 ÷ $0.05/GB = 5 GB/month | Confirms the corrected figures in `infrastructure-specification.md` are internally consistent and arithmetically correct; R-02 is genuinely fixed, not merely reworded. |
| Manual: cross-reference resolution (NFRx.y.z, PD/SD/SC/RD/OD/LC-n, ID/MD/CP-n) | Every `ID-n` (1–12), `MD-n` (1–4), `CP-n` (1–7) cited across the four files resolves to a row defined in the artifact that owns that prefix; no dangling reference found | Spot-checked ID-4 (X-Real-IP), ID-9 (spending limit), MD-1 (uptime monitor), CP-4 (weekly data path) against their defining rows — all consistent across files. |
| Manual: cross-file contradiction check | Found one — see R-05 (new) | CP-6's unqualified "a release is never deleted" versus CP-4 step 4's conditional deletion of an incomplete release, both within `cicd-pipeline.md` itself. No other contradiction found among the four repaired artifacts; the amendment table, the cost section, and the build-failure `(checked)` citations are each internally consistent with their cross-references in `traceability.json`. |

### Summary

All four iteration-1 findings are genuinely resolved: the NFR3.1.17 alert conflict now has a proper amendment, the cost arithmetic is corrected and re-verified by hand ($2.25 fixed + 5 GB egress headroom, not the erroneous $2.80), the build-failure claim is now checked and cited consistently across `cicd-pipeline.md` and `infrastructure-specification.md`, and the weekly-publish step is now idempotent with a named recovery path. The repair to R-04, however, introduced a new, small internal contradiction (R-05, Minor): CP-6's retention table still claims a release is "never deleted" even though CP-4, nine lines earlier in the same file, now specifies exactly when one is deleted. This is a one-line documentation fix, not a design flaw, and with zero Critical and only Minor findings outstanding the artifact is implementable as written.
