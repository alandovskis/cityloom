# Infrastructure Design — Questions — `osm-extract-proxy` (U9)

Upstream inputs: `performance-design.md`, `security-design.md`,
`scalability-design.md`, `reliability-design.md`, `observability-design.md`
and `logical-components.md` (nfr-design, this Unit), `functional-spec.md`
(functional-design, this Unit), `components.md` (domain-design),
`contract-summary.md` (contract-design), `unit-of-work.md`
(units-generation), `bolt-plan.md` (delivery-planning), `team.md` and
`project.md` (practices).

**What this stage decides for this Unit:** how the logical components of
`logical-components.md` are placed on Railway — the service, its image,
where the cell store lives and how a weekly build reaches production, the
health probe and rollover settings, the platform-asserted address header,
what is monitored and how the service level is measured, and the delivery
pipeline from a merge to a running deployment.

Most of it is already fixed and is confirmed at the summary rather than
asked:

- **The platform and cadence.** One Railway service, deploy on merge to
  `main`, no staging tier, rollback through Railway's deployment history
  (`team.md`, Deployment). The four server Units share that one service
  (`unit-of-work.md`, "Four Units, one deployable").
- **The pipeline gates.** The two CI tiers, `gitleaks`, `cargo audit`,
  Dependabot security-only, SonarQube in the slow tier, a bundle-size
  guard — all affirmed in `team.md` and inherited here unchanged.
- **The build runner for the weekly data build.** A free public-repository
  GitHub Actions runner (7 GB of memory, 14 GB of disk) is what
  `scalability-requirements.md` NFR2.1.6 already sizes the ≤ 30-minute
  build against.
- **Secrets.** None for this Unit's service (`security-requirements.md`
  NFR6.1.1); configuration is environment variables.
- **Facts checked on 2026-09-14 against Railway's documentation** that the
  design below relies on: the edge sets `X-Real-IP` for the client's remote
  address and terminates TLS 1.2/1.3; the health check is called only at
  deploy time to gate rollover (a `2xx` from the configured path, default
  timeout 300 s, hostname `healthcheck.railway.app`) and is **not
  continuous monitoring**; the old deployment gets `SIGTERM` and, by
  default, **0 seconds** before `SIGKILL` unless
  `RAILWAY_DEPLOYMENT_DRAINING_SECONDS` is set; cron services run their
  start command on a schedule (5-minute minimum) and must exit; buckets are
  S3-compatible, private only (credentials required), $0.015 per GB-month
  with free egress; the Hobby plan is $5/month including $5 of usage, at
  $10/GB-month memory, $20/vCPU-month, $0.05/GB egress, with a 100 GB
  ephemeral disk ceiling per service; a workspace spending limit can be
  set with a soft (email) and hard (stop services) threshold. GitHub
  release assets on a public repository are free, each file under 2 GiB,
  with no bandwidth limit.

Two things are genuinely open. Both are placement decisions no prior stage
could take, and both change what the service depends on to start.

---

## Q1. Where does the cell store live, and how does a weekly build reach production?

`logical-components.md` hands this over as item 1: LC-2 is one packed store
file (≤ 500 MB for one province, NFR2.1.6) plus a manifest, produced weekly
by LC-3 (the region-build tool, W1). `functional-spec.md` left two
placements open — inside the image, or fetched to ephemeral disk at start —
and NFR3.3.1 forbids an attached volume. The choice fixes what a restart
needs, whether this Unit acquires a credential it currently has none of
(NFR6.1.1), and how the weekly refresh (BR8.5, W4) triggers a deploy.

A. **Baked into the image from a GitHub Release; the data build commits a
   pointer.** The weekly GitHub Actions job runs LC-3, publishes the store
   and manifest as assets of a GitHub Release tagged with the `buildId`,
   then commits a one-line pointer file (the release tag and the
   manifest's digest) to `main`. Railway's GitHub integration deploys the
   push; the `Dockerfile` downloads the pinned assets during the image
   build, verifies the digest, and copies them into the image beside the
   binary. The running service reads local files only; a restart needs
   nothing outside the image; no token or credential exists anywhere in
   this Unit; the image is reproducible from the pointer. Costs: the image
   grows by the store (≤ 500 MB), every deploy re-downloads it during the
   build, and the pointer commit is a bot push to `main` that the branch
   rules must allow (a GitHub-Actions-authored commit does not itself
   trigger CI, which is acceptable for a data pointer).

B. **Fetched at service start from a GitHub Release to ephemeral disk.** The
   image carries the binary only; at start the service downloads the assets
   the configured release tag names, verifies them (W2 step 3 as written),
   then becomes Ready. The weekly job publishes the release and must then
   restart the service — which needs a Railway token in the GitHub Actions
   secrets, so the build job authenticates to Railway (an amendment to
   NFR6.1.1's "the build job authenticates to nothing"). Every restart
   downloads ≤ 500 MB from GitHub before Ready, so NFR1.1.4's 30-second
   target depends on GitHub's transfer rate and the probe timeout must be
   raised; a restart while GitHub is unreachable leaves the new deployment
   Unready (the previous one keeps serving under the rollover). Smallest
   image, no rebuild for a data refresh.

C. **Railway-native: a cron service builds into a Railway bucket; the proxy
   fetches from the bucket at start.** LC-3 becomes a second Railway
   service with a weekly `cronSchedule`; it writes the store and manifest
   to a Railway bucket (about $0.01/month for one province, egress free)
   and triggers a restart of the proxy through Railway's API. The proxy
   downloads from the bucket at start as in B. Everything stays on one
   platform and the build's compute is metered in pennies, but this Unit
   acquires two credentials it has none of today — the bucket's access key
   in the proxy's environment and an API token in the cron job — which
   reverses NFR6.1.1 and adds a secret to rotate on a public-repository
   project.

X. Other (please specify)

[Answer]: A

---

## Q2. How is availability measured, given Railway's health check runs only at deploy time?

`reliability-requirements.md` NFR3.1.1 defines the availability SLI as "the
fraction of one-minute intervals in which the readiness signal answered
ready" and measures it from "the platform's own health-check history".
Checked on 2026-09-14: Railway's health check "is only called at the start
of the deployment" and is "not used for continuous monitoring", so that
history does not exist. The 99.5% target needs another source or stays
unverifiable. `team.md` sets the bar for anything added: it must block or
alert at the moment of the mistake with an actionable message, or it is
off — no dashboards, no periodic reports.

A. **An external uptime monitor on the readiness endpoint.** A free-tier
   monitor (for example UptimeRobot's free plan: 50 monitors at a 5-minute
   interval; or a comparable service) probes the public readiness path
   every 1–5 minutes, keeps the up/down history the SLI is computed from,
   and sends one notification when the service is down — the one alert
   the reliability design has no other source for, and actionable
   (redeploy the last good build). The readiness endpoint is already
   public, limiter-exempt and discloses nothing (NFR3.2.5), so the probe
   costs nothing and leaks nothing. Adds one third-party account with no
   credential in this Unit.

B. **Derived from the counters row cadence.** OD-2 emits one row every 60
   seconds while Ready; a gap in the rows is a minute of unavailability,
   so the SLI is computed from Railway's log history by a script at review
   time. Self-contained, no third party — but a log-derived figure with no
   alert at all, dependent on Railway's log retention covering a calendar
   month, and blind to the case where the process is up but the edge is
   not.

C. **Not measured in product Stage 1.** NFR3.1.1 stays a target with no
   measurement until Stage 2's server work adds one; the reliability
   requirement is amended to say so, and the only availability signals are
   Railway's own failed-deploy and crash notifications.

X. Other (please specify)

[Answer]: A

---

## Consolidated Summary Confirmation

**Your answers**

- **Cell store placement** (Q1): baked into the image from a GitHub
  Release. The weekly GitHub Actions job publishes the store and manifest
  as release assets tagged by `buildId` and commits a one-line pointer
  file to `main`; Railway deploys the push; the `Dockerfile` downloads the
  pinned assets, verifies their digests, and copies them beside the
  binary. No credential anywhere in this Unit; a restart needs nothing
  outside the image.
- **Availability measurement** (Q2): an external free-tier uptime monitor
  probes the public readiness path every 5 minutes, keeps the up/down
  history NFR3.1.1 is computed from, and sends the one down notification.

**Things I will design with rationale rather than ask — confirm them here**

- **The Railway service.** One service in the `us-east4` (Virginia) region
  — the closest Railway region to the Canadian provinces the first data
  build carries; no Canadian region exists. Its Railway-provided domain is
  the Stage 1 hostname; TLS terminates at the edge. The same service is
  the one U10–U12 and the served client bundle share.
- **The image, built from a committed `Dockerfile`.** Q1 needs the store
  copied into the image at build time, which only a project-supplied
  `Dockerfile` can do, so the builder question `team.md` left to
  environment-provisioning is settled here for the server: `DOCKERFILE`.
  Three stages — a build stage on the toolchain `rust-toolchain.toml`
  pins, running `cargo build --release --locked`; a data stage that fetches
  the two release assets the pointer names and checks their SHA-256
  against the pointer; a runtime stage on a minimal Debian-based image
  running as a non-root user with the store and manifest at a fixed
  read-only path. The data stage depends only on the pointer file, so an
  unchanged pointer lets the layer cache skip the 500 MB download on a
  code-only deploy.
- **Railway settings, committed as configuration.** Health check path
  `/readyz` with a 120-second timeout (four times NFR1.1.4's 30 s);
  `RAILWAY_DEPLOYMENT_DRAINING_SECONDS=5` so the old deployment gets more
  than the 3-second request budget between `SIGTERM` and `SIGKILL` (RD-7;
  Railway's default is 0); restart policy on failure with a bounded retry
  count; one replica; the Unit's own non-secret configuration as service
  variables (store and manifest paths, grid, limits, budget, cache
  ceiling), with the client build stamp taken from Railway's injected git
  commit SHA, which fits `[A-Za-z0-9._-]{1,64}` (SD-6).
- **The platform-asserted address** is `X-Real-IP`, set by Railway's edge
  (checked 2026-09-14), read as the whole header value; SD-1's one hashing
  call reads it, and the peer address is the fallback when the header is
  absent (tests, local runs). The readiness route accepts the
  `healthcheck.railway.app` hostname Railway probes with, because the
  service filters no hostnames.
- **The weekly data build** is a GitHub Actions workflow on a free
  public-repository runner: a Monday schedule plus manual dispatch (BR8.5's
  "weekly and on demand"), a concurrency group so runs never overlap, and
  the steps of W1 — run the region-build tool, compare the `buildId` with
  the pointer and stop if unchanged (nothing published), otherwise create
  the release with the assets, then commit the pointer with the workflow's
  own token. The branch rules that gate `main` allow that one bot commit
  on the pointer path; a commit authored by the workflow token triggers no
  CI run, which is acceptable for a data pointer and noted as a practice
  question for the learnings step.
- **The delivery pipeline** is `team.md`'s two CI tiers unchanged, with
  the stage-to-gate mapping written down: every push runs the fast tier;
  pull requests to `main` also run the slow tier; nothing merges without
  both; a squash-merge to `main` triggers Railway's build from the
  `Dockerfile`; the health check gates rollover; rollback is Railway's
  redeploy of the previous deployment, and a data rollback is reverting the
  pointer commit (the previous release still exists). Railway's own
  "wait for CI" option stays off, because the gate is enforced at merge
  and the pointer commit carries no check suite.
- **Monitoring.** The uptime monitor (Q2), Railway's own notifications for
  a failed deployment and a crash-restart, and the workspace spending
  limit with a soft (email) threshold at $4 and a hard (stop services)
  threshold at $5 — that limit is NFR5.1.6's "account usage limit". Logs
  are read in Railway's log explorer from the counters, failure and
  lifecycle rows OD-1 to OD-5 fix; the four SLIs are OD-7's arithmetic on
  two rows. No dashboard, no metrics agent, no tracing.
- **Cost estimate for this Unit** on the Hobby plan: memory 128 MB at
  $10/GB-month ≈ $1.30; CPU at 0.05 vCPU average ≈ $1.00; egress at the
  B-3 measurement (provisional ≤ $0.50); GitHub Actions and release
  storage free; monitor free — inside the ≤ $2.50 allocation of NFR5.1.5,
  with the whole workspace's usage under the $5 the subscription includes.

**Things I will record as open rather than answer**

- Railway's acceptance of a ~600 MB image and whether its layer cache
  persists across builds — confirmed at environment-provisioning by the
  first deploy.
- Which uptime-monitor service, and its exact interval and notification
  channel — the design fixes the properties (public readiness path, ≤ 5
  minutes, one down notification); the account is created at
  environment-provisioning.
- The exact bot-commit bypass rule for the pointer path, and the
  `Dockerfile`'s base-image tags — code-generation and ci-pipeline, within
  the shape above.

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
