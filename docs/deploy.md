# Deploying `osm-extract-proxy` (U9)

## Runbook: a bad deploy or a bad data build

There is one recovery action (`reliability-design.md` RD-9): **redeploy the
last good build** from Railway's deployment history. Railway keeps the
previous deployment serving until a new one passes its health check
(`/readyz`, 200), so a broken image never takes over traffic; if a bad
deploy somehow does take over, redeploying the prior deployment restores
service immediately — no data migration, no volume, nothing else to undo,
because this Unit holds no state outside the image itself.

## Environment-provisioning handoff

Copied from `infrastructure-specification.md` so the next stage has it
beside the code, not just in a design document:

1. Create the Railway project and service in `us-east4`; connect the GitHub
   repository; confirm the builder is `DOCKERFILE` and the committed
   `railway.json` is picked up (ID-1, ID-2, ID-5).
2. Set the deployment variable `RAILWAY_DEPLOYMENT_DRAINING_SECONDS=5`
   (leave `RAILWAY_DEPLOYMENT_OVERLAP_SECONDS` at its default, `0`) and the
   service variables below; confirm the platform injects `PORT` and
   `RAILWAY_GIT_COMMIT_SHA` (ID-6, ID-7).
3. Set the workspace spending limit: soft $4 (email), hard $5 (services
   stop) (ID-9).
4. Create the uptime monitor on the Railway-provided domain's `/readyz`
   (every 5 minutes, expect 200) and route its notification to the
   maintainer (ID-12).
5. Confirm by doing, once each (see `infrastructure-specification.md` for
   the full list): a deploy with a corrupted cell leaves the previous
   deployment serving; a deploy whose image build fails (a deliberately
   wrong asset SHA-256 in the pointer) also leaves the previous deployment
   serving; the measured Ready time sits well inside the 120-second probe
   timeout; a redeploy from Railway's deployment history restores the
   previous build; a `SIGTERM` drains for 5 seconds.
6. Record the image size Railway accepted and whether the data layer was
   reused on a code-only deploy.

## Service variables (`CITYLOOM_*`, ID-7)

None of these are secrets; every one has a code default (`config.rs`), so
the variable exists only to move a number after B-3 measures a real one.

| Variable | Default | Meaning |
|---|---|---|
| `PORT` | `8080` | injected by Railway |
| `CITYLOOM_STORE_PATH` | `/data/store.bin` | set in the `Dockerfile` |
| `CITYLOOM_MANIFEST_PATH` | `/data/manifest.json` | set in the `Dockerfile` |
| `CITYLOOM_BUILD_ID` | — | fallback build stamp if `RAILWAY_GIT_COMMIT_SHA` is absent (a non-Railway deployment) |
| `CITYLOOM_CACHE_MAX_BYTES` | `33554432` (32 MiB) | the extract cache ceiling (BR6.2) |
| `CITYLOOM_REQUESTS_PER_MINUTE` | `30` | BR2.1 |
| `CITYLOOM_REQUESTS_PER_HOUR` | `300` | BR2.1 |
| `CITYLOOM_SPAN_BOUND` | `12` | BR3.3 |
| `CITYLOOM_CUTTING_SLOTS` | `4` | NFR1.1.6 |
| `CITYLOOM_REQUEST_BUDGET_MS` | `3000` | BR10.3 |

`RAILWAY_GIT_COMMIT_SHA` is injected automatically for a deployment built
from the GitHub integration and needs no configuration.
