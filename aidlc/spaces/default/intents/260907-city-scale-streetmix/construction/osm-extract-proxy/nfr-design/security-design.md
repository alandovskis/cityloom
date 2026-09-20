# Security Design — `osm-extract-proxy` (U9)

Upstream inputs: `security-requirements.md`, `performance-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md` and
`tech-stack-decisions.md` (nfr-requirements, this Unit),
`functional-spec.md`, `rules.md` and `entities.md` (functional-design, this
Unit), `contract-summary.md` (contract-design), `decisions.md` ADR-004
(domain-design), `nfr-design-questions.md` (this stage), `team.md` and
`project.md` (practices).

Design elements are numbered `SD-n`; `traceability.json` maps each
`NFRx.y.z` from `security-requirements.md` to the elements that meet it.
The threat model, the requirements and their verification are in
`security-requirements.md`; this file is how they are met, as layers.

## Defence in depth, as layers a request passes through

```
 internet
    |
    v
 [L0] platform edge ......... TLS termination; asserts the client address
    |
    v
 [L1] listener + router ..... GET only; header size and read-timeout limits;
    |                          no CORS; catch-panic boundary
    v
 [L2] build stamp ........... x-client-build first (BR1.1) -> 426
    |
    v
 [L3] requester limiter ..... hash(address) with the per-process key;
    |                          two windows -> 429 + Retry-After
    v
 [L4] typed parsing ......... bbox -> integer box or 400 (BR3.1);
    |                          nothing else read from the request
    v
 [L5] integer addressing .... cell ids from the box; offsets from the
    |                          index; a read-only store
    v
 [L6] typed response ........ ApiError | bytes; fixed header set;
    |                          x-extract-key is a hex digest
    v
 [L7] emission discipline ... no per-request row; five-field failure rows;
                               span events off
```

<!-- Text fallback: eight layers in order. L0 the platform edge terminates TLS and asserts the client address. L1 the listener and router accept GET only, apply header limits and a read timeout, set no CORS headers, and catch panics. L2 checks x-client-build first. L3 hashes the address with the per-process key and applies two windows. L4 parses bbox into an integer box or answers 400, reading nothing else. L5 addresses cells by integer id and offset into a read-only store. L6 answers only the typed error or the bytes with a fixed header set. L7 emits no per-request row and only five-field failure rows. -->

No layer trusts the one above it to have done its job: L4 validates even
though L0 asserted an address; L6 constrains header values even though L5
produced them from integers. A single layer failing (a bug in one) leaves
the others standing, which is the point of the arrangement.

## Design elements

### SD-1 — Trust the platform's address, and only for one call (L0, L3)

The asserted client address is read from the header Infrastructure Design
designates (NFR6.3.3), in the position it specifies. The value is passed
to exactly one function, `requester_hash(address) -> u64`, which hashes it
with the process's `RandomState` (SipHash-1-3, 128-bit key from the
operating system at start, NFR6.3.2; TS-8) and returns. The raw value is a
borrowed string slice with the lifetime of the header map; it is stored in
no struct, no extension, no span. A client-supplied forwarded header is not
consulted; if the platform header is absent (a direct connection in a
test), the peer address is used so the limiter still applies.

### SD-2 — The listener (L1)

- Routes: the extract route accepts `GET` only; the readiness route accepts
  `GET`; everything else is the router's 404/405 with an empty body.
- Header read timeout and maximum header size set on the HTTP server
  (NFR6.4.4); values from the stack's documented options at code
  generation. No request body is read on any route.
- No CORS layer, no CORS headers, ever (NFR5.1.8). The mandatory
  `x-client-build` header (BR1.1) makes a cross-origin browser request
  preflight, and the preflight fails.
- A catch-panic layer around the router maps a panic to the failure mapping
  with reason `internal`, phase `unexpected`, and one failure row (BR10.4,
  NFR6.4.2). The panic payload is never rendered.

### SD-3 — Order of checks (L2 before L3 before L4)

`functional-spec.md` W3 fixes the order and this design keeps it, for a
security reason as well as a functional one: the build-stamp check costs
nothing and rejects every stale or scripted request that lacks the header
before the limiter spends a map entry on it, and the limiter runs before
validation so a client cannot probe the parser without spending its
allowance.

### SD-4 — Requester windows (L3)

One `Mutex<HashMap<u64, Windows>>` where `Windows` is four integers:
minute start, minute count, hour start, hour count (two fixed windows per
hash, the `entities.md` amendment recorded in `security-requirements.md`,
resolved to this shape). One critical section per request: look up or
insert, reset a window whose start is older than its length, compare both
counts to 30 and 300, increment both, release. A limited request answers
`429 rate_limited` with `Retry-After` = seconds until the earlier of the two
windows ends (NFR5.1.7). A background task sweeps entries whose hour window
has ended, every 60 seconds (NFR2.1.3). The map holds nothing but hashes,
starts and counts (BR7.3); it is never serialised.

### SD-5 — Typed parsing and integer-only addressing (L4, L5)

`bbox` is parsed into `[f64; 4]` with a strict parser: exactly four
comma-separated decimals, finite, in range, minimum strictly below maximum
(BR3.1); anything else is `400 invalid_area` before any allocation beyond
the four numbers. The box becomes an integer box (PD-1) and every later
value — cell ids, index offsets, the key — is derived from integers. A
cell is read by an offset and length from the manifest-derived index, so no
request-derived text is ever part of a path or a file name (NFR6.4.1). The
store is opened read-only at start; the process holds no writable handle
to any file.

### SD-6 — Typed responses and the header set (L6)

Every response passes through one response layer that sets, unconditionally:
`Strict-Transport-Security`, `X-Content-Type-Options: nosniff`,
`Referrer-Policy: no-referrer`, `Content-Security-Policy: default-src
'none'`, `Cache-Control: private, no-store` (NFR6.4.3). The extract
response body is either the clip bytes with `application/octet-stream` and
`x-extract-key` (a lowercase hex BLAKE3 digest, so it cannot carry a line
break), or the typed `ApiError` JSON from the shared contract crate with a
project-authored `detail` (BR10.1, NFR6.4.2). `current_build` is validated
against `[A-Za-z0-9._-]{1,64}` when configuration is loaded, so a bad value
fails start-up rather than reaching a header.

### SD-7 — Emission discipline (L7)

The `tracing` subscriber is configured once, at start: JSON to stdout, span
events off. The only events the proxy emits are the catalogued ones
(`observability-design.md` OD-1 to OD-5), each built from a fixed set of
fields. The HTTP framework's request-tracing layer is not installed
(NFR6.3.1). Request-scoped spans, if code generation uses them for the
deadline, carry no fields at all. The log-capture test named under
NFR6.3.1 runs one hit, one miss and every failure reason and asserts that
the captured output contains none of the address, any coordinate, the key
or any cell id — this test is the enforcement for the whole layer.

### SD-8 — Readiness endpoint (L1, L6)

A fixed body (`ready` / `not-ready`) and status (200 / 503) read from the
Ready flag (`reliability-design.md` RD-1). It bypasses L2 and L3 — no build
stamp, no limiting — so the platform's probe can never be limited or
rejected for lacking a client header (NFR3.2.5). It carries the same header
set as every other response and nothing else.

### SD-9 — Configuration and secrets

Configuration is read once at start from environment variables (region
list, grid, limits, budget, cache ceiling, build id, store and manifest
paths) with typed parsing and a fail-fast on any invalid value; none is a
secret and none is logged as a whole (the Ready row names the `buildId` and
counts only). There is no secrets store to integrate because there is no
secret (NFR6.1.1). The build tool reads the same configuration shape plus
the publisher's base URL.

### SD-10 — Build-time integrity

The region-build tool (`logical-components.md` LC-3) downloads each region
and its `.md5` over TLS with certificate verification (`reqwest` with
`rustls`, TS-7), streams the body to disk while computing MD5, compares,
and aborts the whole build on any mismatch or transport error — nothing is
written to the store on failure (BR8.1, NFR7.2.2). The manifest records the
publisher path, published timestamp and digest per region (BR8.4). Every
cell's BLAKE3 digest is written by the build and verified by the service at
start (BR9.1, NFR3.2.4), so a corrupted or tampered artifact never serves.

### SD-11 — Supply chain

`Cargo.lock` committed, `--locked` builds, `cargo audit` as a merge gate,
Dependabot security updates only, the vendored `.proto` files with their
MIT licence and upstream commit in the asset manifest (NFR7.2.3, TS-3). No
crate in the tree keys anything on the raw address (TS-8).

## What is deliberately absent

| Control | Why not here |
|---|---|
| Authentication on the extract route | Public data, no accounts in Stage 1, and an identifier per browser would be exactly what ADR-004 refuses to hold (`security-requirements.md`, "No authentication, by design") |
| A web application firewall or bot detection | Would need to inspect and often log the address; the limiter and the preflight refusal cover the realistic abuse at this scale |
| A global egress ceiling | Rejected at functional design Q4; the account usage limit is the hard stop (NFR5.1.6) |
| Encryption at rest | The only data at rest is the public cell store; encrypting public data protects nothing |
| Audit logging of who requested what | Forbidden by design (BR7.1); the audit trail is what failed and how much was served |

## Assumptions & Open Questions

- The platform-asserted address header and its position are confirmed at
  infrastructure-design; SD-1 isolates the choice in one function.
  [assumption]
- The stack's catch-panic layer can be configured to produce the typed
  `ApiError` body rather than its default; if not, a project-owned
  equivalent is a small piece of code with the same shape. [assumption]
- `[A-Za-z0-9._-]{1,64}` is wide enough for whatever build identifier the
  deploy pipeline mints (a commit hash or a date-stamped tag); confirmed at
  ci-pipeline. [assumption]

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-14T19:08:32Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | `performance-design.md` > PD-4, versus `nfr-requirements/performance-requirements.md` NFR1.1.1 | NFR1.1.1 defines the 3,000 ms budget as "measured from the request being received by the handler to the first response byte being written" — i.e. the clock starts when the handler receives the request. PD-4 instead starts the `tokio::time::timeout` "after the build-stamp check", explicitly excluding phase 1 (BR1.1) from the timed window on the stated ground that it "needs no budget". Functionally immaterial (the build-stamp check is a header comparison, sub-microsecond), but the design's actual measurement point does not match the metric's own stated definition, and `security-design.md` SD-3 repeats the same framing ("the build-stamp check costs nothing... before the limiter"). A future implementer measuring NFR1.1.1 by wall-clock from request receipt (as literally specified) would include a phase the design says is excluded from the budget. | Either amend NFR1.1.1's measured-how text (at the next `nfr-requirements` touch) to say the clock starts after the build-stamp check, or change PD-4 to start the single deadline at request receipt (wrapping phase 1 too) so the design's timing point matches the requirement's stated definition verbatim. |  New |
| R-02 | Minor | `scalability-design.md` SC-4 table row "Manifest row (JSON)", versus `nfr-requirements/scalability-requirements.md` NFR2.1.5 | NFR2.1.5's target is "Manifest ≤ 16 MB **in memory** per province" (its own text sizes a ~60-byte in-memory entry to ~10 MB, budgeting to 16 MB). SC-4 labels its "Manifest row (JSON)" figure of 15–20 MB as living on "disk", and `performance-design.md` PD-2/PD-7 describe only a separate 24-byte-per-entry compact index (~5 MB) as what is actually retained in memory after start-up. The two design files never state outright that the ~60-byte-style "manifest" NFR2.1.5 sizes and the 24-byte offset/length index PD-2 keeps are different structures serving the same requirement, or that the on-disk JSON's digest/count fields are deliberately dropped after start-up verification. A reader checking NFR2.1.5's 16 MB ceiling against SC-4's 15–20 MB figure alone would wrongly read it as a near-miss or a breach, when the actual retained structure (PD-2's index) is comfortably under budget. | Add one sentence to SC-4 (or PD-2) stating explicitly that the on-disk JSON manifest (15–20 MB, digest/count fields included) is parsed once at start and only its offset/length index (PD-2, ~5 MB) is retained in memory for the life of the process — so NFR2.1.5's 16 MB in-memory ceiling is measured against the retained index, not the on-disk JSON, and is met with margin. |  New |
| R-03 | Minor | `observability-design.md` OD-4 "unready" row, versus `reliability-design.md` RD-1's start-up sequence | OD-4 names five closed `check` values for the `unready` lifecycle row: `manifest-missing`, `manifest-unreadable`, `store-missing`, `cell-missing`, `cell-digest-mismatch`. RD-1's start-up sequence has only three failure points: load manifest (missing/unreadable → Unready), open the store (missing → Unready), and "verify every cell's range... any digest mismatch → Unready" — the last step conflates a listed cell being absent from the store with a listed cell whose bytes fail their digest check into one Unready path, with no step described as distinguishing them. OD-4 assumes that distinction is made (`cell-missing` vs `cell-digest-mismatch` as separate enum values) without RD-1 describing where in the sequence a missing-cell condition is detected separately from a digest mismatch. | Add a sentence to RD-1's verification step (or a note in OD-4) naming the check that distinguishes "the manifest lists cell N but the store has no entry at its recorded offset" (`cell-missing`) from "the store has bytes at cell N's offset but they do not match the recorded digest" (`cell-digest-mismatch`), so the two `unready` check values OD-4 promises are traceable to an actual step in RD-1's sequence. |  New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-traceability.ts` | `{"pass":true,"gaps":[],"orphans":[],"missing_from_table":[],"missing_from_upstream_ids":[],"invalid_entries":[],"invalid_targets":[],"findings_count":0}` | Every detailed NFR id in the five `nfr-requirements` files this Unit produced is enumerated in `traceability.json` and every `coverage[].target` design-element citation resolves; no gaps, no orphans. |
| `aidlc-sensor-required-sections.ts` (×6 artifacts) | PASS on all six (`performance-design.md`, `security-design.md`, `scalability-design.md`, `reliability-design.md`, `observability-design.md`, `logical-components.md`) | No template is configured for this stage, so the sensor reports the H2 structure found; each file's headings are internally consistent with its own stated purpose. |
| `aidlc-sensor-upstream-coverage.ts` (on `security-design.md`) | `{"pass":true,"consumes":[],"unreferenced":[],"scanned_files":[],"reason":"no upstream"}` | The sensor found no configured `consumes` list to check against for this artifact under this invocation shape and reported a vacuous pass; not independent evidence either way, so cross-checked manually instead (see below). |
| `aidlc-sensor-linter.ts` / `aidlc-sensor-type-check.ts` | Not applicable | `security-design.md` contains no fenced TypeScript/JavaScript code blocks (its one fenced block is the ASCII layer diagram, with no language tag); the other five design artifacts likewise carry only ASCII diagrams and Rust-shaped illustrative snippets outside fenced-code form, so there is nothing for these two sensors to lint or type-check. |
| Manual: every `NFRx.y.z` cited in the six design artifacts vs. the five `nfr-requirements` files | PASS, with one pre-existing exception already disclosed | Every numeric target cited (3,000 ms budget, p95/p99 figures, 0.01° cells, span bound 12, 5-decimal outward canonicalisation, 30/min and 300/hour, 4 slots, 32 MB cache, 128/192 MB memory, ≤8 MB/cut, 30 s Ready, the nine-bucket histograms, 60 s counters cadence) matches its requirements-file source exactly. `NFR6.3.1`–`NFR6.3.4`'s parent-id mismatch (documented in `security-requirements.md`'s own carried-forward `## Review` R-01) is correctly reflected rather than silently resolved: `traceability.json`'s note for `NFR6.3` names the review finding instead of asserting a clean refinement. |
| Manual: every `BR<n>.<m>` cited in the design artifacts vs. `functional-design/rules.md`'s 32 rule ids | PASS | Every cited rule id (BR1.1, BR2.1–2.3, BR3.1–3.3, BR4.1–4.2, BR5.1–5.5, BR6.1–6.4, BR7.1–7.4, BR8.1, BR8.4, BR9.1, BR10.1–10.4) exists in `rules.md` with matching statement content. |
| Manual: the read path, cache/single-flight, limiter, start-up, security-layer and observability decisions in `nfr-design-questions.md`'s Consolidated Summary Confirmation vs. all six deliverables | PASS | Every confirmed design decision (packed single-store layout, integer cell addressing, four-permit slots with per-cell deadline checks, `moka`/`try_get_with` single-flight, one mutex-guarded requester-window map with a 60 s sweep, the eight-layer defence-in-depth order, the closed observability catalogue) appears identically across the six artifacts with no drift from the confirmed answer. |
| Manual: `logical-components.md` LC-1 inventory and the "four Units, one deployable" / `OsmExtractProxy` claims vs. `inception/domain-design/components.md` and `inception/units-generation/unit-of-work.md` | PASS | `components.md`'s `OsmExtractProxy` entry (fetches and caches OSM extracts, logs neither addresses nor locations) is the component `logical-components.md` says it is the inside of; `unit-of-work.md`'s own "Four Units, one deployable" heading and its U9–U12 rows confirm the shared-Railway-service claim in LC-1's shared-resource note verbatim. |
| Manual: team practice cross-check (`team.md` Code Style — typed `Result`/no `panic!` reaching a caller, `rustfmt`/`clippy`; `project.md` Forbidden/Mandated — no secret committed, no OSM edit, budget-growth-as-constraint-change) | PASS | `reliability-design.md` RD-4 and `security-design.md` SD-2 route every panic through a catch-panic boundary into a typed `internal` result rather than letting one propagate; `security-design.md` SD-9 and SD-11 keep configuration non-secret and dependencies pinned/audited; no design element in any of the six files edits OSM data, and `scalability-design.md`'s capacity-threshold table explicitly routes a memory/CPU/second-instance increase through "a constraint change" rather than silent absorption. |

### Summary

The six artifacts are numerically and cross-referentially sound: every cited NFR, BR, entity and confirmed Q&A decision resolves to real, matching content, `traceability.json` enumerates the full upstream NFR set with no gaps or orphans, and the design keeps to its declared boundary (no user/design/account data, no osm2streets dependency, no fabricated circuit breaker or retry policy where none is warranted) while handing the right five items to Infrastructure Design. The three findings recorded are all Minor and none blocks implementation: a sub-microsecond discrepancy between where PD-4's timer starts and NFR1.1.1's literal measurement-point wording, an under-explained relationship between the on-disk JSON manifest's size and the smaller in-memory index that actually meets NFR2.1.5's ceiling, and an observability enum (`cell-missing` vs `cell-digest-mismatch`) that assumes a start-up distinction RD-1's prose does not spell out. A developer could build this Unit from these six files without further architectural guidance.
