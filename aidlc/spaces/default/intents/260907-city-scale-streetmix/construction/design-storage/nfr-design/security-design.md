# Security Design — `design-storage` (U10)

Upstream inputs: `security-requirements.md`, `performance-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md` and
`tech-stack-decisions.md` (nfr-requirements, this Unit), `functional-spec.md`,
`rules.md` and `entities.md` (functional-design, this Unit),
`contract-summary.md` Contract 2, `decisions.md` ADR-006 (domain-design),
`osm-extract-proxy`'s own security design precedent (this workspace's one
other server Unit).

Design elements are numbered `SD-n`; `traceability.json` maps each
`NFRx.y.z` from `security-requirements.md` to the elements that meet it.
The threat model, the requirements and their verification are in
`security-requirements.md`; this file is how they are met, as layers.

## Defence in depth, as layers a request passes through

```
 internet
    |
    v
 [L0] platform edge ......... TLS termination (shared with U9's Railway
    |                          service)
    v
 [L1] listener + router ..... POST/GET/DELETE only on /api/designs*;
    |                          catch-panic boundary; body size limit
    v
 [L2] requester limiter ..... identifier-keyed windows -> 429 + Retry-After
    |                          (BR4.1)
    v
 [L3] input validation ...... payloadVersion + size cap checked before
    |                          any write (BR6.1) -> 400/413
    v
 [L4] authorization check ... identifier resolves to an existing,
    |                          non-expired row (BR3.1) -> 404/403
    v
 [L5] typed data access ..... sqlx compile-time-checked queries; no
    |                          request-derived text in SQL beyond a bound
    |                          parameter
    v
 [L6] typed response ........ StorageFailure | UploadReceipt | payload;
    |                          fixed header set
    v
 [L7] emission discipline ... no per-request row; closed-field failure
                               and sweep rows only
```

<!-- Text fallback: eight layers in order. L0 the platform edge terminates
TLS, shared with the U9 proxy's own Railway service. L1 the listener and
router accept only POST/GET/DELETE on the designs routes, catch panics, and
cap request body size. L2 applies an identifier-keyed rate-limiting window.
L3 validates payloadVersion and size before any write. L4 checks the
identifier resolves to an existing, unexpired row before returning anything.
L5 accesses the database only through compile-time-checked sqlx queries with
bound parameters, never interpolated text. L6 returns only the typed
StorageFailure, UploadReceipt, or payload shapes with a fixed header set. L7
emits no per-request row and only closed-field failure and sweep rows. -->

No layer trusts the one above it: L4's authorization check runs even
though L2 already rate-limited the request; L5's bound parameters are
used even though L3 already validated the payload shape. A single
layer's bug leaves the others standing.

## Design elements

### SD-1 — Order of checks (L2 before L3 before L4)

The rate limiter runs before payload validation and before the
authorization check, for the same reason `osm-extract-proxy`'s SD-3
orders its own layers: the limiter is the cheapest check and rejects a
scripted flood before the more expensive validation or a database round
trip is spent on it. Validation (L3) runs before authorization (L4) on
the upload path specifically because BR6.1 requires rejecting a bad
payload before any row exists to authorize against; on the fetch/remove
paths, L3 does not apply (there is no payload in the request) and L4
runs directly after L2.

### SD-2 — Requester-identifier rate limiting (L2, T1, NFR5.3.1, NFR6.4.1)

One bounded map (in-process, matching NFR2.3.3's "no cross-request
coordination state" — see `scalability-design.md` SC-1 for why this does
not conflict with the single-instance deployment) keyed on the
`anonymousDesignId` from the request path or body, the same
two-fixed-window shape `osm-extract-proxy`'s SD-4 already established
for this workspace (a short window and a longer window, both compared
per request). Unlike U9's proxy, the key here is the identifier itself,
not a hashed requester address — BR4.1 protects the identifier from
guessing, not the requester from being profiled, so no address is read
or hashed for this check at all (NFR6.3.1's "no requester-identifying
field" extends naturally to the rate limiter's own state, which the U9
precedent's address-hashing approach would otherwise have suggested).
A limited request answers `429 rate_limited` with `Retry-After` (BR4.1,
NFR5.3.1). Numeric thresholds are fixed at `tech-stack-decisions.md`
per that file's own stated assumption. This is the design element that
closes T1 (identifier guessing/enumeration): the STRIDE table's own
stated treatment for T1 is the 128-bit identifier's unguessable space
combined with rate limiting making a guessing campaign impractical, and
this map is the rate-limiting half of that pair. NFR6.4.1's own
verification text — that the identifier is never derived from the
design's content and is compared only as an opaque value — is the other
half of what closes T1; this Unit never computes, hashes, or derives an
`anonymousDesignId` from anything (`SD-7` below states the same
non-derivation property for the different reason of not storing an
identifying field, T5) — the identifier this Unit compares is always
exactly the opaque value the client minted (ADR-006) and supplied.

### SD-3 — Fail-closed input validation before any write (L3)

`payloadVersion` is checked via `cityloom-design-payload`'s existing
`DesignPayload::from_json` fail-closed version check (U3) — this Unit
does not re-implement that check, only calls it, per
`tech-stack-decisions.md`'s stated reuse decision. The request body's
raw byte length is checked against the size cap (`tech-stack-decisions.md`
fixes the cap value) before the body is even deserialized into a
`DesignPayload`, so an oversized payload never reaches the parser at all
— `413 payload_too_large` is returned from a byte-length check, not a
parse-then-reject. Both checks run, and both must pass, before the
`INSERT` in `performance-design.md` PD-1 is issued (NFR6.4.3).

### SD-4 — Object-level authorization on every read (L4)

`GET /api/designs/{id}` and `DELETE /api/designs/{id}` both resolve the
path identifier against the `stored_designs` table before anything else
happens; a `SELECT`/`DELETE` that finds no matching, non-expired row
answers `404 design_not_found` (BR3.1). The Stage 1 authorization model
is exactly "the identifier itself is the credential" (`entities.md`'s
stated assumption) — there is no separate grant table to consult yet, so
this check is the row lookup itself, not a check layered on top of it.
No code path constructs a response body containing `payload` before this
lookup has succeeded (NFR6.4.2); the `SELECT` that returns the payload
and the `SELECT` that authorizes are the same query, so there is no
window between "authorized" and "payload read" for a race to open.

### SD-5 — Typed data access (L5)

Every query is a `sqlx::query!`/`query_as!` compile-time-checked macro
call against the schema (`infrastructure-design`'s migration), with
every request-derived value (the identifier, the payload bytes) passed
as a bound parameter — never string-interpolated into SQL text. This is
`sqlx`'s default mode (`tech-stack-decisions.md`'s stated reason for
choosing it over a hand-built query string), so SQL injection is
foreclosed by the query-construction mechanism itself rather than by a
sanitization step that could be forgotten.

### SD-6 — Typed responses, no leaked driver error (L6, T4)

Every `sqlx::Error` the pool or a query can return (a connection
failure, a timeout, a genuine constraint violation on a column other
than the conflict-handled one below) is mapped, once, in one function,
to a `StorageFailure` with reason `internal` and a project-authored
`detail`; no `sqlx::Error`'s `Display` output or a raw Postgres error
message ever reaches a response body (BR6.1, T4, NFR6.4.4). The
upload-conflict case (`performance-design.md` PD-1's `INSERT ...
ON CONFLICT (anonymous_design_id) DO NOTHING`) is not an error at all
under this mapping — it is an expected, named outcome detected from the
statement's own rows-affected count: `0` rows affected means a row with
that identifier already existed, and the handler maps that specific,
non-error result to `409` (per Contract 2) directly, before this
function's error-mapping path is ever reached. No unique-violation
`sqlx::Error` is caught or expected on this path, because `ON CONFLICT
DO NOTHING` never raises one.

### SD-7 — No identifying field stored (T5, NFR6.3.1)

The `stored_designs` schema (`infrastructure-design`) has exactly the
columns `entities.md` names for `StoredDesign` — no `ip_address`,
`user_agent`, or similar column exists for this Unit to accidentally
populate. `anonymous_design_id` is stored exactly as received from the
client (BR7.1); no handler code derives, hashes, or supplements it with
anything read from the request beyond that one field.

### SD-8 — Configuration and secrets (T6)

The database connection string is read once at process start from a
Railway environment variable (`DATABASE_URL`, `team.md` Deployment); it
is never logged, and connection failures are logged with a fixed message
(`observability-design.md` OD-4's readiness signal), never the
connection string itself.

### SD-9 — Supply chain

`Cargo.lock` committed, `--locked` builds, `cargo audit` as a merge gate
(`team.md` Deployment), the same as every other Unit in this workspace.
`sqlx`'s compile-time query checking additionally requires a reachable
database (or its offline query cache, `.sqlx/`) at build time — the
offline cache is committed so CI does not need a live database just to
compile (a build-reproducibility concern this Unit is the first in the
workspace to introduce, flagged for `infrastructure-design`).

## What is deliberately absent

| Control | Why not here |
|---|---|
| Authentication (accounts, sessions) | Stage 1 has no accounts (`entities.md`); the identifier itself is the Stage 1 credential, per `components.md` `DesignRepository`'s own stated design |
| Encryption of the payload column at rest | Postgres-managed disk encryption (Railway's platform default) covers this; a second application-level layer would need its own key management for no threat this STRIDE table names |
| A global per-IP rate limiter (as opposed to per-identifier) | BR4.1 and `security-requirements.md`'s T7 are both scoped to identifier-guessing; a broader IP-based limiter is `osm-extract-proxy`'s own concern for its distinct unauthenticated-fetch threat model, not duplicated here |
| Audit logging of who accessed which design | No requester identity exists to log in Stage 1, and NFR6.3.1 forbids storing one just to have it |

## Assumptions & Open Questions

- **[assumption]** The rate limiter's numeric thresholds and the
  payload size cap are fixed at `tech-stack-decisions.md`/code
  generation, not here — this design fixes the mechanism, not the
  numbers, consistent with `security-requirements.md`'s own stated
  deferral.
- **[assumption]** Whether the rate limiter's in-process map survives a
  process restart (it does not, by construction) is acceptable for
  Stage 1's threat model — a restart-reset limiter still bounds
  sustained guessing, and a persistent limiter would need its own
  storage and its own threat model, which this Unit's cost/complexity
  budget does not justify for the threat named.

## Traceability

See `traceability.json` in this directory.

_Confirmed._

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-19T02:14:18Z
**Iteration:** 1
**Request Challenge:** review:34f47bffc08fc490d97d260ab1b83edc

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `traceability.json` `upstream_ids`, versus this Unit's five `nfr-requirements` files and the `osm-extract-proxy` precedent | `traceability.json` lists only the detailed three-segment `NFRx.y.z` ids. It omits every coarse two-segment id these same `nfr-requirements` files themselves cite: the `Refines` column values for already-existing ids and the group ids each file's own "Amendments required" table proposes. | Add the 11 coarse ids (`NFR1.4`, `NFR2.3`, `NFR3.1`, `NFR3.2`, `NFR3.3`, `NFR3.4`, `NFR5.3`, `NFR6.1`, `NFR6.3`, `NFR6.4`) to `traceability.json`'s `upstream_ids` array, matching the precedent's own group-id-plus-detail-id shape, and re-run the sensor to confirm a clean pass. | Resolved — all 11 named ids are now present in `upstream_ids` with matching `coverage` rows. Re-running `aidlc-sensor-traceability.ts --output-path <this file> --stage nfr-design` now returns `missing_from_upstream_ids":["NFR1.1"]` only — the 11 ids this finding named are gone from the gap list. The one remaining `NFR1.1` hit is a sensor false-positive: `performance-requirements.md` mentions `NFR1.1` only to state it does *not* apply to this Unit ("No inception NFR fixes a numeric budget for this Unit's three endpoints directly — NFR1.1's 10-second budget is scoped to a full osm2streets import (U4/U9)") and cites `osm-extract-proxy`'s own `NFR1.1.1`–`NFR1.1.6` purely as a stated-discipline precedent, not as an id this Unit refines or covers; adding it to `upstream_ids` would misrepresent this Unit as covering another Unit's requirement. Recorded as a new Minor observation (see below), not re-opened as part of R-01. |
| R-02 | Major | `security-design.md` SD-2/SD-7, versus `security-requirements.md`'s STRIDE table (T1) and `traceability.json`'s entry for `NFR6.4.1` | `traceability.json` mapped `NFR6.4.1` (the requirement that closes threat T1) to `SD-7`, a different design element for a different threat (T5) and requirement (`NFR6.3.1`). The STRIDE table's own stated T1 treatment is the rate limiter, `SD-2`, which nothing tied to `NFR6.4.1` or T1. | Correct `traceability.json`'s `NFR6.4.1` target to cite `SD-2`, and add text to `SD-2` addressing `NFR6.4.1`'s own opacity/non-derivation verification text. | Resolved. `SD-2`'s heading now reads "Requester-identifier rate limiting (L2, T1, NFR5.3.1, NFR6.4.1)" and its body states explicitly "This is the design element that closes T1... this map is the rate-limiting half of that pair. NFR6.4.1's own verification text — that the identifier is never derived from the design's content and is compared only as an opaque value — is the other half of what closes T1", cross-referencing `SD-7`'s parallel (but distinct-purpose, T5) non-derivation statement. `traceability.json`'s `NFR6.4.1` entry now cites both `SD-2 (guessing-campaign mitigation, T1)` and `SD-7 (identifier held/compared opaquely, never re-derived)`. Re-reading the STRIDE T1 row, `SD-2`, and `SD-7` together: the chain now resolves end to end — T1's stated mitigation (rate limiting) is the same design element the traceability table cites, and the opacity property `NFR6.4.1`'s own text asks for is separately and correctly attributed. |
| R-03 | Major | `performance-design.md` PD-1, versus `security-design.md` SD-6, on the upload-conflict (`409`) path | PD-1 (rows-affected via `ON CONFLICT DO NOTHING`) and SD-6 (a caught unique-constraint-violation error) described two mutually exclusive SQL mechanisms for the same `409` outcome. | Align both files to one mechanism. | Resolved. `SD-6`'s second half now reads: "The upload-conflict case (`performance-design.md` PD-1's `INSERT ... ON CONFLICT (anonymous_design_id) DO NOTHING`) is not an error at all under this mapping — it is an expected, named outcome detected from the statement's own rows-affected count: `0` rows affected means a row with that identifier already existed, and the handler maps that specific, non-error result to `409`... No unique-violation `sqlx::Error` is caught or expected on this path, because `ON CONFLICT DO NOTHING` never raises one." This now matches PD-1's mechanism exactly (rows-affected count, no caught error), with the generic error-mapping function in the first half of SD-6 explicitly scoped to "a genuine constraint violation on a column *other than* the conflict-handled one below." No remaining contradiction between the two files. |
| R-04 | Minor | `traceability.json` `upstream_ids`, versus `performance-requirements.md`'s prose | The sensor's re-run still reports `missing_from_upstream_ids":["NFR1.1"]`. `NFR1.1` is `osm-extract-proxy`'s (U9's) requirement, mentioned in this Unit's `performance-requirements.md` only as an explicit statement that it does *not* apply here. Adding it to this Unit's `upstream_ids` would falsely claim this Unit refines/covers another Unit's requirement, so the sensor's residual flag is a known false-positive rather than a real gap — but it does mean the sensor cannot be run to a clean `pass:true` on this file as it stands. | No action required for readiness; optionally rephrase the `performance-requirements.md` sentence to avoid citing the bare `NFR1.1` token if a fully clean sensor run is ever wanted (e.g. spell it as "U9's NFR1.1" in prose only), but this is cosmetic and does not affect any developer's ability to build from these files. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-traceability.ts --output-path traceability.json --stage nfr-design` (this Unit, re-run) | `{"pass":false,"gaps":[],"orphans":[],"missing_from_table":[],"missing_from_upstream_ids":["NFR1.1"],"invalid_entries":[],"invalid_targets":[],"findings_count":1}` | Confirms R-01's fix: all 11 previously-missing coarse ids are now present. Only `NFR1.1` remains, which is a false-positive (R-04) rather than a real gap — see R-04's rationale. |
| Manual re-read: `security-design.md` SD-2, SD-7 and `security-requirements.md`'s STRIDE T1 row / `NFR6.4.1` text, against `traceability.json`'s `NFR6.4.1` entry | Chain resolves end to end | Confirms R-02's fix. |
| Manual re-read: `performance-design.md` PD-1 and `security-design.md` SD-6 together | One consistent mechanism (rows-affected via `ON CONFLICT DO NOTHING`; no caught unique-violation error) | Confirms R-03's fix. |
| Sanity pass: re-checked that the SD-2 retitle and SD-6 rewrite did not disturb any other cross-reference (the L2 layer-diagram line, `SD-3`'s ordering claim, the STRIDE table's T4/T5 rows, `traceability.json`'s `NFR6.4`/`NFR6.3.1`/`NFR5.3.1` entries) | PASS | No new inconsistency introduced by the three fixes. |

### Summary

All three Major findings from iteration 1 are resolved and verified against the actual files and the traceability sensor: the coarse NFR ids are now in `traceability.json` (R-01), the T1/`NFR6.4.1` chain now correctly cites the rate limiter with the opacity property attributed separately (R-02), and `performance-design.md`/`security-design.md` now describe one consistent upload-conflict mechanism (R-03). One new Minor, non-blocking observation (R-04) is recorded: the traceability sensor still reports one residual false-positive (`NFR1.1`, a different Unit's requirement cited only as explicitly out-of-scope context) that does not affect buildability. No fix broke any other cross-reference checked in this pass.
