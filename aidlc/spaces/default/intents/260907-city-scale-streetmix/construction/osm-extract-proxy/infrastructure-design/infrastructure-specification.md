# Infrastructure Specification — `osm-extract-proxy` (U9)

Upstream inputs: `performance-design.md`, `security-design.md`,
`scalability-design.md`, `reliability-design.md`, `observability-design.md`
and `logical-components.md` (nfr-design, this Unit), `functional-spec.md`
(functional-design, this Unit), `components.md` (domain-design),
`contract-summary.md` (contract-design), `unit-of-work.md`
(units-generation), `infrastructure-design-questions.md` (this stage),
`team.md` and `project.md` (practices).

Design elements are numbered `ID-n`; `traceability.json` maps each
infrastructure-relevant `NFRx.y` decision from the NFR design to the
resource or setting that meets it. This is a design: it names the
resources, their settings and the reasons; the `Dockerfile`, the workflow
files and the Railway configuration file are code generation's.

Every platform fact below was checked against Railway's documentation on
2026-09-14 and is marked *(checked)*; anything unverifiable before the first
deploy is under "Assumptions & Open Questions".

## What is placed, and where

```
 GitHub (public repository)                   Railway (Hobby, us-east4)
 +-----------------------------+              +---------------------------+
 | main --------- push ------->|------------->| service: cityloom         |
 |  |                          |  GitHub app  |  builder: DOCKERFILE      |
 |  | data/current-build.toml  |              |  image = binary + LC-2    |
 |  |   (pointer, committed by |              |  /readyz gates rollover   |
 |  |    the weekly workflow)  |              |  X-Real-IP -> limiter     |
 |  |                          |              |  stdout -> log explorer   |
 | Actions: weekly data build  |              +------------+--------------+
 |  (LC-3, free runner) ------>| release      | edge: TLS, X-Real-IP      |
 |  publishes                  | assets       +------------+--------------+
 | Releases: data-<buildId>    |<--- Dockerfile fetches ---+
 |   store.bin, manifest.json  |
 +-----------------------------+              uptime monitor ---> GET /readyz
```

<!-- Text fallback: the public GitHub repository holds the code, a committed pointer file naming the current data release, and the weekly Actions workflow that runs the region-build tool and publishes the store and manifest as release assets. Railway's GitHub integration builds the service from a Dockerfile on every push to main; the Dockerfile fetches the pinned release assets so the image carries the binary and the cell store together. Railway's edge terminates TLS and sets X-Real-IP; the health check on /readyz gates rollover; stdout goes to Railway's log explorer; an external uptime monitor probes /readyz. -->

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute model | One container on **Railway**, one replica, the single service `unit-of-work.md` names for U9–U12 and the served client bundle (**ID-1**) | `team.md` Deployment: one environment, deploy on merge, no staging tier; NFR3.3.1 forbids a volume; NFR5.1.5's ≤ $2.50 allocation buys no second process (the arithmetic is under "Cost estimate" below) |
| Region | `us-east4` (Virginia) (**ID-1**) | The closest of Railway's four regions *(checked: US West, US East, Europe, Asia)* to the Canadian provinces the first data build carries (NFR2.1.4); no Canadian region exists; the 3-second budget (NFR1.1.1) is insensitive to the ~20 ms difference between US regions but the choice is made once, at creation |
| Builder | `DOCKERFILE`, a committed multi-stage `Dockerfile` (**ID-2**) | Q1: the cell store is copied into the image at build time, which only a project-supplied `Dockerfile` can do *(checked: Railway offers `RAILPACK`, legacy `NIXPACKS`, `DOCKERFILE`)*. This settles for the server the builder question `team.md` left to environment-provisioning; the client bundle's build path is U6's and is not decided here |
| Image contents | Stage 1 builds the binary on the toolchain `rust-toolchain.toml` pins with `cargo build --release --locked` (NFR7.2.1); stage 2 fetches `store.bin` and `manifest.json` from the GitHub Release the pointer names and verifies each asset's SHA-256 against the pointer; stage 3 is a minimal Debian-based runtime image, non-root user, the two files at `/data/` read-only, the binary as the entrypoint (**ID-2**) | LC-2 is read-only data the process only reads (RD-1, SD-5); a restart needs nothing outside the image (RD-6); no credential is needed to fetch a public release asset (NFR6.1.1); layer order puts the data stage's inputs (the pointer only) before the source tree so a code-only deploy reuses the cached data layer |
| Networking | Railway's edge terminates TLS 1.2/1.3 *(checked)* and forwards plain HTTP to the container on the injected `PORT`; the Railway-provided `*.up.railway.app` domain is the Stage 1 hostname; no private networking is needed (nothing calls this Unit inside Railway) (**ID-3**) | `security-requirements.md` L0 (`security-design.md`); NFR6.4.3's `Strict-Transport-Security` header is meaningful because the public hostname is TLS-only *(checked: inbound traffic must be TLS-encrypted)* |
| Client address | The limiter hashes the whole value of the `X-Real-IP` header, which Railway's edge sets to the client's remote address *(checked)*; a client-supplied `X-Forwarded-For` is never read; if the header is absent (tests, a local run) the peer address is used (**ID-4**) | NFR6.3.3, SD-1: "the header Infrastructure Design designates, in the position it specifies" — one header, one value |
| Storage | None attached. The image carries the store and manifest; the process writes nothing to disk; ephemeral disk is unused (**ID-2**) | NFR3.3.1; RD-1 opens the store read-only; the Hobby plan's 100 GB ephemeral ceiling *(checked)* is irrelevant by design |
| Environments | One: production. Pull-request environments are off (**ID-1**) | `team.md`: no staging; a PR environment would build the 500 MB image per PR for a reviewer who does not exist |
| Configuration as code | A committed Railway configuration file at the repository root carrying builder, `dockerfilePath`, `healthcheckPath` `/readyz`, `healthcheckTimeout` 120, restart policy `ON_FAILURE` with a bounded retry count, one replica, and the deployment variables below (**ID-5**) | Settings in a committed file survive a recreated service and are reviewed like code; `team.md`'s "infrastructure is code" reading of a solo project |
| Deployment variables | `RAILWAY_DEPLOYMENT_DRAINING_SECONDS=5`; `RAILWAY_DEPLOYMENT_OVERLAP_SECONDS` left at its default 0 (**ID-6**) | *(checked)* Railway's default is **0 seconds** between `SIGTERM` and `SIGKILL`, which would kill in-flight requests inside the 3-second budget; 5 s exceeds the budget RD-7 drains for. Overlap is unnecessary: the new deployment is already serving when the old one is signalled |
| Service variables (non-secret) | `CITYLOOM_STORE_PATH=/data/store.bin`, `CITYLOOM_MANIFEST_PATH=/data/manifest.json`, the grid, the two window limits, the slot count, the request budget, the cache ceiling, the log cadence — each with the provisional value from the NFR requirements as its default in code, so the variables exist to move a number after B-3 measures it (**ID-7**) | SD-9: typed, fail-fast, none secret; SC-2 makes the permit count configuration; `security-requirements.md` NFR6.1.1 |
| Build stamp | `current_build` (BR1.1, SD-6) is Railway's injected `RAILWAY_GIT_COMMIT_SHA`, a 40-character hexadecimal string inside `[A-Za-z0-9._-]{1,64}` (**ID-7**) | The client bundle built from the same commit carries the same value, which is exactly the "one deployable" rollover `contract-summary.md`'s build stamp exists for |
| Resource sizing | No instance size to pick: Railway meters actual usage *(checked: $10/GB-month memory, $20/vCPU-month, $0.05/GB egress)*. Expected for this Unit: ≤ 128 MB typical (NFR5.1.2) and ≤ 0.05 vCPU averaged (NFR5.1.4); the plan's per-service maxima (48 GB, 48 vCPU) are never approached (**ID-8**) | The bound on spend is the spending limit (ID-9), not a size; the bounds on consumption are the slots, the cache ceiling and the windows (SC-1 to SC-3) |
| Spending limit | Workspace usage limit: **soft $4** (email) and **hard $5** (services stop) *(checked: both thresholds exist; the hard limit interrupts services)* (**ID-9**) | NFR5.1.6: "the account usage limit is the last line of defence"; `project.md` forbids growth past ~$5/month being absorbed; the Hobby subscription's included $5 of usage is the whole budget |
| IaC approach | Railway's configuration file plus the `Dockerfile` plus the two GitHub Actions workflows (CI, weekly data build), all committed; the service itself is created once by hand at environment-provisioning and everything about it is thereafter in the repository (**ID-5**) | One-time creation of one service is not worth a provisioning tool the budget would pay for; drift is limited to the domain and the spending limit, both recorded in the runbook |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Railway service `cityloom` (ID-1) | compute | `us-east4`; builder `DOCKERFILE`; one replica; health check `GET /readyz`, 120 s; restart `ON_FAILURE`; draining 5 s | Shared with U10–U12 and the client bundle; this Unit's figures are its contribution to the process (`logical-components.md`, shared resources) |
| Railway edge (LC-4) | load-balancer / TLS termination / dns | Railway-provided domain; TLS 1.2/1.3; `X-Real-IP` set; 32 KB combined header limit *(checked)*; requests closed after 5 minutes without data *(checked)* | The edge's 32 KB header limit sits above the listener's own tighter limit (SD-2, NFR6.4.4); the edge's 5-minute idle timeout is far outside the 3-second budget, so the proxy's deadline is always the one that fires |
| Railway log explorer (LC-5) | logs | stdout, one JSON object per line (OD-1); retention as the plan provides | The counters, failure and lifecycle rows are read here; nothing is shipped elsewhere (NFR3.1.12–NFR3.1.15) |
| GitHub Releases (ID-10) | artifact store for LC-2 | One release per data build tagged `data-<buildId>`, assets `store.bin` and `manifest.json`; public; each file under 2 GiB, no bandwidth limit *(checked)* | The one-province store is ≤ 500 MB (NFR2.1.6); a release is immutable once published, so the pointer pins bytes, not a moving name; old releases are the data rollback path |
| GitHub Actions (ID-11) | scheduled job runner for LC-3 | Free public-repository runner; weekly schedule plus manual dispatch; concurrency group `data-build` | `scalability-requirements.md` NFR2.1.6 already sizes the ≤ 30-minute build against this runner; the job authenticates to nothing but GitHub itself, through the workflow's own token |
| External uptime monitor (ID-12) | availability probe | Free tier; `GET https://<domain>/readyz` every 5 minutes; expects 200; one notification on down | Q2; `monitoring-design.md`; the readiness route is public, limiter-exempt and discloses nothing (NFR3.2.5), so the probe leaks nothing |
| Cache | cache | In-process `moka`, 32 MiB, LRU (PD-5) — **no cache service** | Rejected at functional design Q2 and `scalability-design.md`; a cache service is a dependency at request time this Unit is designed not to have |
| Database, queue, search, CDN | — | **None for this Unit** | No persistent state (NFR3.3.1); no downstream call at request time (RD-3); the served bytes are keyed on a build the URL does not name, so a CDN could cache nothing correctly (NFR6.4.3's `Cache-Control: private, no-store`) |

## Shared Infrastructure

| Shared Resource | Owner Unit | Consumer Units | Access Boundary |
|---|---|---|---|
| Railway service `cityloom` (process, memory, CPU, domain) | U9 (created and configured first, at B-3) | U10 `design-storage`, U11 `accounts-sharing`, U12 `data-rights`, U6 `client-surfaces` (served bundle) | One binary, one port; each later Unit mounts its own routes under `/api/`; this Unit's memory and CPU figures are its share, and Infrastructure Design for each later Unit adds to the sum (`logical-components.md`, "Isolation and shared resources"). Nothing calls this Unit in-process; U4 reaches it only over Contract 1 |
| Spending limit (ID-9) | U9 (set at environment-provisioning) | Every Unit on the workspace | Workspace-wide; a later Unit's PostgreSQL service (U10) draws on the same $5, which is why this Unit's allocation is ≤ $2.50 (NFR5.1.5) |
| GitHub Actions CI (fast and slow tiers) | `team.md` (workspace-wide practice) | Every Unit | `cicd-pipeline.md` |
| The `Dockerfile` and Railway configuration file | U9 | U10–U12, U6 | Later Units extend the same image and the same configuration file; the image's data stage is this Unit's and stays a separate stage so the 500 MB layer is cached independently of code changes |

## Data build placement (ID-10, ID-11)

The weekly build of `functional-spec.md` W1 runs as a GitHub Actions workflow:

| Step | What | Fixed by |
|---|---|---|
| Trigger | `schedule` weekly (Monday, early UTC) and `workflow_dispatch` for the on-demand run BR8.5 names; a `concurrency` group so a slow run is never overlapped by the next | BR8.5; NFR3.1.4 |
| Runner | The free public-repository Linux runner; the region-build tool runs with `cargo run --release --locked` on the pinned toolchain | NFR2.1.6 (≤ 30 minutes, memory per SC-5); NFR7.2.1 |
| Build | Download each configured region and its `.md5` over TLS, verify, filter, slice, write `store.bin` and `manifest.json` to a temporary directory (RD-8) | BR8.1–BR8.4; SD-10; NFR7.2.2 |
| Publish decision | Compare the new `buildId` with the pointer file's; if equal, exit without publishing (BR8.5, NFR3.1.4) | `functional-spec.md` W1 step 7 |
| Publish | Create the GitHub Release `data-<buildId>` with the two assets and their SHA-256 sums in the release notes; then commit `data/current-build.toml` (release tag, `buildId`, asset SHA-256s, published timestamp) to `main` with the workflow's token | ID-10; the pointer is the only thing the deploy reads |
| Deploy | Railway's GitHub integration builds the pushed commit; the `Dockerfile`'s data stage fetches the assets the pointer names and refuses any SHA-256 mismatch; the service verifies every cell's BLAKE3 digest at start (RD-1) and takes traffic only when Ready (RD-2) | W2, W4; NFR3.2.1, NFR3.2.4 |
| Rollback of data | Revert the pointer commit; the previous release still exists; the redeploy carries the previous store | RD-9 |

Two properties of the pointer commit are recorded rather than hidden: it is
a bot push to `main`, so the branch rules that require a pull request must
carry a bypass for the workflow on that one path; and a commit authored by
the workflow's own token starts no CI run *(a GitHub rule, not a choice)*,
which is acceptable for a one-line data pointer because the code is
unchanged and the deploy's own verification (SHA-256 at build, BLAKE3 at
start) is the gate the data needs. The practice question — a bot commit
on the trunk `team.md` says all work merges to by pull request — is
raised at this stage's learnings step, not decided here.

## Cost estimate (ID-8, ID-9)

The provisional figures, added up correctly, against NFR5.1.5's ≤ $2.50
all-in allocation for this Unit *(rates checked 2026-09-14)*:

| Term | Basis | Monthly |
|---|---|---|
| Memory | 128 MB typical (NFR5.1.2) × $10 per GB-month | **$1.25** |
| CPU | 0.05 vCPU averaged (NFR5.1.4) × $20 per vCPU-month | **$1.00** |
| Fixed terms | | **$2.25** |
| Headroom for egress inside $2.50 | $0.25 at $0.05 per GB | **5 GB a month** — about 33,000 clips at the 150 KB working estimate, roughly 1,100 a day |
| GitHub Actions, GitHub Releases, the uptime monitor | free tiers on a public repository | $0 |

So the allocation holds only if Stage 1 egress stays under about 5 GB a
month. That figure is **not** a design bound — no egress ceiling exists by an
approved decision (functional design Q4; NFR5.1.6 makes the spending limit
the last line) — it is the number B-3's measurement is read against. The
consolidated summary this stage confirmed stated an egress term of
"≤ $0.50", which would sum to $2.80; that sum breaches the allocation and is
corrected here rather than carried: the egress term is whatever 5 GB of
headroom leaves, and a measured Stage 1 egress above it is the constraint
change NFR5.1.5 and `project.md` require, raised at B-3's gate, never
absorbed. For scale: a single relentless client at the per-requester limit
alone is 1.08 GB a day (`scalability-requirements.md` NFR5.1.3), which is
why the spending limit (ID-9) exists.

## Environment-provisioning handoff

Named so `environment-provisioning` has a list rather than a search:

1. Create the Railway project and service in `us-east4`; connect the GitHub
   repository; set `DOCKERFILE` and the committed configuration file (ID-1,
   ID-2, ID-5).
2. Set the deployment and service variables (ID-6, ID-7) and confirm the
   injected `PORT` and `RAILWAY_GIT_COMMIT_SHA`.
3. Set the workspace spending limit, soft $4 and hard $5 (ID-9).
4. Create the uptime monitor on the Railway-provided domain's `/readyz`
   (ID-12) and route its notification to the maintainer.
5. Confirm by doing, once each: a deploy of an artifact with a corrupted
   cell leaves the previous deployment serving (NFR3.2.1's verification);
   a deploy whose image build fails (a deliberately wrong asset SHA-256 in
   the pointer) also leaves the previous deployment serving *(checked
   2026-09-14 against Railway's deployment reference: a deployment that
   fails during build or deploy stops with status `Failed`, and an older
   deployment is removed only after a new one becomes `Active` — confirmed
   by doing because the first deploy of all has no previous deployment to
   keep)*; the measured Ready time sits well inside the 120-second probe
   timeout (NFR3.2.2); a redeploy from Railway's deployment history
   restores the previous build (RD-9); a `SIGTERM` drains for 5 seconds
   (RD-7).
6. Record the image size Railway accepted and whether the data layer was
   reused on a code-only deploy (the two open items below).

## Rejected alternatives

- **A Railway volume for the store.** Forbidden by NFR3.3.1, and it would
  turn every deploy into a volume hand-off rather than a rollover.
- **Fetching the store at service start** (Q1 option B). Couples every
  restart to GitHub's reachability and transfer rate, puts a Railway token
  into the build job, and makes NFR1.1.4's 30 seconds a network figure.
- **A Railway cron service and bucket** (Q1 option C). Two credentials this
  Unit has none of, on a public repository; the platform's own cron would
  otherwise fit *(checked: 5-minute minimum, must exit, skips an overlapping
  run)*.
- **`RAILPACK` for the server.** Cannot place the data in the image, and
  the toolchain pin would live in a builder variable rather than in
  `rust-toolchain.toml`.
- **A custom domain in Stage 1.** Nothing needs it before there is a
  public release; the Railway domain is TLS-terminated and free.

## Assumptions & Open Questions

- Railway accepts and builds an image of roughly 600 MB (binary plus a
  ≤ 500 MB store) without a documented size limit being hit; confirmed by
  the first deploy at environment-provisioning. [assumption]
- Railway's build layer cache persists between builds of the same service,
  so a code-only deploy does not re-download the store; if it does not,
  every deploy costs one 500 MB download from GitHub during the build —
  slower, never incorrect. [assumption]
- The pointer commit's bypass of the pull-request rule can be scoped to the
  workflow actor and the pointer path with GitHub's branch rules; if it
  cannot be scoped that narrowly, the alternative is a scheduled workflow
  that opens and auto-merges a pull request, which needs a token beyond the
  workflow's own. [assumption]
- The uptime monitor's free tier keeps at least a calendar month of history
  at a 5-minute interval, which is the resolution NFR3.1.1's monthly figure
  is read at. [assumption]
- `RAILWAY_GIT_COMMIT_SHA` is injected for a deployment built from the
  GitHub integration; for a deployment created another way the service
  falls back to a `CITYLOOM_BUILD_ID` variable and fails start-up if
  neither exists (SD-9's fail-fast). [assumption]
