# Security Requirements — `osm-extract-proxy` (U9)

Upstream inputs: `functional-spec.md`, `rules.md` and `entities.md`
(functional-design, this Unit), `requirements.md` (requirements-analysis),
`contract-summary.md` (contract-design), `nfr-requirements-questions.md`
(this stage), `decisions.md` ADR-004 (domain-design), `team.md` and
`project.md` (practices).

Ids follow `performance-requirements.md`: a third level refines an inception
requirement. Four requirements here (NFR6.4.1–NFR6.4.4) refine no numbered
inception sub-requirement: they are boundary hardening — input validation,
output encoding, response headers, transport — which `requirements.md` NFR6
does not itemise. They are numbered under a proposed `NFR6.4` and the
addition is recorded under "Amendments required" below, so their origin is
documented rather than invented.

## What this Unit protects, and from whom

The proxy holds **no user data, no design data and no account data**
(`unit-of-work.md` U9 boundary). Everything it serves is public
OpenStreetMap data under ODbL, cut from a publisher's extract. So the assets
are not secrets; they are these:

| Asset | Classification | Where it is |
|---|---|---|
| The requester's network address | **Personal data** — the one datum in this Unit that identifies a device; transient, never stored | In the request only; hashed on arrival (BR2.3) |
| The pair (address, requested area) | **Location data about an identifiable device** (`decisions.md` ADR-004) — never assembled, never written | Nowhere by construction: no entity can hold both (`entities.md`, Summary) |
| `RequesterWindow` (keyed hash, start times, counts) | Pseudonymous, per-process, memory-only | Process memory; gone at window end and process end (BR7.3) |
| `FailureRecord` rows and `ServiceCounters` | Operational, no personal data (BR7.1, BR7.2, BR7.4) | Process memory and stdout |
| `RegionalDataBuild` — cells, manifest, coverage | **Public**, integrity-sensitive: a wrong or tampered cell is a wrong street served as real (BR8.1, BR9.1) | Read-only in the deploy artifact |
| The **$5 budget** | Not data, but the thing an unauthenticated public endpoint most plausibly costs the owner (NFR5.1, `project.md` budget rule) | Bounded by NFR5.1.3 and the span bound |

The attacker this Unit is designed against is not a targeted adversary; it is
the ordinary internet: scanners, a looping client, a hostile page that makes
visitors' browsers fetch from us, a corrupted download. There is no
authentication to break because there is none (see "No authentication, by
design").

## Trust boundaries and attack surface

```
+-----------------+     +------------------+     +---------------------------+
| Public internet |---->| Railway edge     |---->| osm-extract-proxy process |
| (browsers,      | TLS | (terminates TLS, | HTTP| GET /api/extract          |
|  scanners)      |     |  asserts address)|     | readiness endpoint        |
+-----------------+     +------------------+     |   |                       |
                                                 |   v  read-only            |
                                                 | cells + manifest (disk)   |
                                                 |   |                       |
                                                 |   v                       |
                                                 | stdout (failure rows,     |
                                                 |         counters)         |
                                                 +---------------------------+

+------------------+   TLS + .md5   +----------------+   artifact   +--------+
| Geofabrik        |<---------------| build runner   |------------->| deploy |
| (public extract) |                | (W1, weekly)   |              +--------+
+------------------+                +----------------+
```

<!-- Text fallback: two trust boundaries. At request time the public internet reaches the Railway edge over TLS; the edge terminates TLS, asserts the client address in a header, and forwards plain HTTP to the proxy process, which exposes GET /api/extract and a readiness endpoint, reads cells and the manifest from local disk read-only, and writes only failure rows and counters to stdout. At build time a runner downloads the publisher's extract and checksum over TLS, produces cells and a manifest, and they become part of the deploy artifact. -->

Entry points, all of them:

1. `GET /api/extract?bbox=…` with `x-client-build` — public, unauthenticated.
2. The readiness endpoint — public, answers ready or not-ready and nothing else.
3. The platform's address header on every request — trusted only as
   Infrastructure Design specifies (NFR6.3.3).
4. The weekly download from the publisher (W1 step 2) — outbound, build time.
5. The deploy artifact — cells and manifest, produced by the build and
   verified at start (BR9.1).

Nothing else listens, and the service has no write path to anything but
stdout.

## Threat model (STRIDE)

Each row is a threat against an element or flow above, its treatment, and
the requirement that carries it.

| # | Element / flow | Threat | Category | Likelihood × impact | Treatment | Requirement |
|---|---|---|---|---|---|---|
| T1 | Request → stdout | The request's address or area, or a value derived from either, is written to a log, a metric label or a span, and a log line becomes location data about a device | Information disclosure | Medium × High = **High** (the default request span of the HTTP framework logs the full URI including `bbox`) | No per-request row for a successful request; failure rows carry exactly BR7.2's five fields; the framework's default request span is not installed on this route; a test greps the captured log output of a full request cycle for the address, the box and the key | NFR6.3.1 |
| T2 | `RequesterWindow` | The requester hash is linkable across restarts or to another record, making it an identifier | Information disclosure | Low × High = **Medium** | Keyed hash with a random key minted at process start, held only in memory, never derived from configuration (BR2.3); the hash function and key size are fixed below | NFR6.3.2 |
| T3 | Address header | A client sets its own forwarded-address header and escapes the per-requester limit by rotating fake addresses, or pins the limit on someone else | Spoofing → Denial of service / cost | Medium × Medium = **Medium** | The address used is the one the platform asserts, read from the header Infrastructure Design designates in the position it specifies; a client-supplied value never overrides it | NFR6.3.3 |
| T4 | `GET /api/extract` | A looping client, or many, drives egress against the budget | Denial of service (cost) | High × Medium = **High** | Per-requester windows (NFR5.1.3), span bound (NFR1.1.5), PBF encoding (BR5.5); `429` carries `Retry-After` | NFR5.1.7 |
| T5 | `GET /api/extract` | A hostile page makes its visitors' browsers fetch clips from us, spending our egress and their allowance | Denial of service (cost) | Medium × Medium = **Medium** | The endpoint sets no CORS headers and requires a custom header (`x-client-build`, BR1.1), so a cross-origin browser request is preflighted and refused before the GET is sent | NFR5.1.8 |
| T6 | `GET /api/extract` | Many concurrent misses exhaust CPU or memory | Denial of service | Medium × Medium = **Medium** | Four cutting slots, the 3-second budget, ≤ 8 MB per cut (NFR1.1.6, NFR1.1.1, NFR5.1.2); overload becomes `timeout` | NFR5.1.7 |
| T7 | `bbox` parameter | A crafted box reaches the file system as a path, or reaches arithmetic as an infinity or NaN | Tampering / Elevation of privilege | Low × High = **Medium** | BR3.1 rejects anything but four finite in-range numbers before any read; cell ids are integers computed from the canonical box, and cell paths are built from those integers only — never from request text | NFR6.4.1 |
| T8 | Cells on disk | A tampered or truncated cell serves a wrong street as real | Tampering | Low × High = **Medium** | Every cell is digest-verified at start against the manifest (BR9.1); a failure is Unready, never partial service; the readiness signal keeps the previous deployment serving | NFR3.2.4 |
| T9 | Publisher download | A truncated or altered download is sliced into cells | Tampering | Low × High = **Medium** | TLS with certificate verification to the publisher plus the publisher's `.md5` (BR8.1); any mismatch fails the whole build and publishes nothing | NFR7.2.2 |
| T10 | Publisher download | The publisher itself is compromised and serves plausible but wrong data; the checksum matches the wrong file | Tampering (supply chain) | Very low × High = **Low** | Accepted: the checksum is integrity of transfer, not authenticity of content; detection is the golden-fixture suite (`team.md`), which moves the day the served bytes for a fixture street change | NFR7.2.2 |
| T11 | Error responses | A panic, a dependency's error text or a file path reaches the response body | Information disclosure | Medium × Low = **Low** | BR10.1 (project-authored detail only), BR10.4 (a panic is caught at the handler boundary and answered `internal`); response bodies are the typed `ApiError` and nothing else | NFR6.4.2 |
| T12 | Response headers | A value in `x-extract-key` or `current_build` carries a line break or control character (header injection) | Tampering | Very low × Medium = **Low** | Both values are constrained: the key is a lowercase hex digest (BR6.1), the build id is validated against a fixed character set at start | NFR6.4.2 |
| T13 | Readiness endpoint | Used to enumerate configuration or as a cheap request flood | Information disclosure / Denial of service | Low × Low = **Low** | Answers a status and nothing else; no body beyond a fixed token; it is the platform's probe, exempt from requester limiting so a limited client cannot make a deploy look unhealthy | NFR3.2.5 |
| T14 | Dependencies | A vulnerable crate in the tree | Tampering (supply chain) | Medium × Medium = **Medium** | `Cargo.lock` committed and built `--locked`; `cargo audit` as a merge gate; Dependabot security updates only; osm2streets-related crates pinned by commit (`project.md`) | NFR7.2.1 (`tech-stack-decisions.md`) |
| T15 | Repository | A secret reaches the public repository | Information disclosure | Low × High = **Medium** | This Unit needs no secret at all — the publisher is public, the service reads local files, the platform supplies the address; push protection and `gitleaks` per `team.md` remain the backstop | NFR6.1.1 |
| T16 | Any | An action cannot be attributed later | Repudiation | — | Not applicable: there are no user actions, no accounts, and by design no record of who requested what. The audit trail this Unit keeps is of *what failed* (BR7.2) and *how much was served* (BR7.4), which is the trade ADR-004 made deliberately | — |

## Requirements

| ID | Refines | Requirement | Verified by |
|---|---|---|---|
| NFR6.1.1 | NFR6.1 | **No secret exists for this Unit.** The service reads configuration (region list, grid, limits, budget, cache ceiling, build id) from environment variables or a committed non-secret configuration file; none of them is a credential. The build job authenticates to nothing. If a later region source ever requires a credential it is a Railway environment variable and a constraint change, never a file in the repository. | The dependency and asset manifest (`team.md`) lists no credentialed source; `gitleaks` in CI; a test that the service starts with an empty environment plus the documented non-secret variables. |
| NFR6.3.1 | NFR6.3 | **Nothing identifying is ever written.** (BR7.1) No log row, span, metric label, panic message, error body or file name contains a network address, a requested or canonical box, an extract key, a cell id derived from a request, or any value derived from one of them. Concretely: a successful request emits **no row at all** (BR7.4); a failed request emits exactly one `FailureRecord` with `occurredAt`, `reason`, `status`, `phase`, `detail` (BR7.2) and nothing else; the HTTP framework's default request span, which records the full URI, is **not** installed on the extract route; spans used internally for the budget and error mapping are never emitted by the subscriber. | A test drives one hit, one miss and one of each failure reason through the service with log capture, then asserts the captured output contains none of: the address used, any coordinate of the box, the returned `x-extract-key`, any touched cell id. This test is part of the merge gate for any change to this Unit. |
| NFR6.3.2 | NFR6.3 | **The requester hash is unlinkable across processes.** `requesterHash` = a 64-bit keyed hash (SipHash-1-3, the standard library's `RandomState`) of the asserted address, keyed with a 128-bit random key created at process start, held only in that process's memory, never logged, never written, never derived from configuration or from the build id. Collisions merge two requesters' counts and are accepted at 2^-64. | A test asserts the same address hashes differently in two service instances; a code-review assertion (no `clippy` lint exists) that the key is constructed from the operating system's random source and stored nowhere. |
| NFR6.3.3 | NFR6.3 | **The address is the platform's assertion, not the client's.** The value hashed is read from the header Infrastructure Design designates as the platform-asserted client address, in the position it specifies; a client-supplied forwarded header never replaces it. The raw value lives only for the hashing call: it is not stored in the request extension, the span, or any struct that outlives the limiting phase. | A test sends a request with a spoofed forwarded header and asserts the hash equals the one computed from the platform-asserted value; the NFR6.3.1 log test covers retention. Header name and position are confirmed at infrastructure-design. |
| NFR6.3.4 | NFR6.3 | **No cookies, no identifiers issued.** The endpoint sets no cookie and issues no client identifier; `x-extract-key` identifies the extract (BR6.1), never the requester. | A header assertion in the deterministic suite (`team.md`, the no-DAST replacement). |
| NFR5.1.7 | NFR5.1 | **Abuse is bounded, and the bounds are the numbers already set.** Per-requester windows 30 per minute and 300 per hour (NFR5.1.3); at most 12 touched cells (NFR1.1.5); four cutting slots (NFR1.1.6); a 3-second budget (NFR1.1.1); ≤ 8 MB per cut (NFR5.1.2). A `429 rate_limited` response carries `Retry-After` in whole seconds until the earlier of the two windows clears. No global egress ceiling exists — functional design Q4 rejected one — so the budget's last line of defence is the platform's account usage limit (`reliability-requirements.md` NFR5.1.6). | Tests named under each referenced requirement; a test that `Retry-After` on a minute-limited request is between 1 and 60. |
| NFR5.1.8 | NFR5.1 | **Cross-origin browsers cannot make requests.** No CORS response headers are ever set on this route, and `x-client-build` is mandatory (BR1.1). Because a custom request header makes a browser preflight a cross-origin request and the preflight fails without CORS headers, a page on another origin cannot make its visitors' browsers fetch clips. This costs nothing: every legitimate consumer is this project's own client on the same origin (`contract-summary.md`, "The condition this rests on"). | A test sends an `OPTIONS` preflight with `Origin` set to another site and `Access-Control-Request-Headers: x-client-build` and asserts no `Access-Control-Allow-*` header is present; a header assertion that no response carries one. |
| NFR6.4.1 | NFR6.4 (proposed — see Amendments) | **Request text never reaches the file system or arithmetic unvalidated.** `bbox` is parsed as exactly four finite decimal numbers in range with each minimum strictly below its maximum (BR3.1); anything else is `400 invalid_area` before any cell is read. Cell ids are integers computed from the canonical box and the `GridSpec`; cell paths are built from those integers only. No other request parameter is read; unknown query parameters are ignored, never echoed. | Property test over random strings and edge values (`NaN`, `inf`, `-0`, exponents, 180.0000001, a box with min equal to max, five numbers, three numbers, empty) asserting either a canonical box or `invalid_area`, never a panic; a test that the path of every cell read for a request is inside the cells directory. |
| NFR3.2.4 | NFR3.2 | **Only a verified build is served.** At start, the manifest is loaded and every listed cell is checked against its recorded digest (BLAKE3, `tech-stack-decisions.md`); any absence or mismatch is Unready (BR9.1) and the readiness signal stays not-ready, so the previous deployment keeps serving (NFR3.2). There is no partial-service mode and no runtime path that loads a cell not named in the manifest. | Tests: a manifest naming a missing cell → Unready; a cell with one flipped byte → Unready; a valid build → Ready; an extract request while Unready → `503 upstream_unavailable`. |
| NFR7.2.2 | NFR7.2 | **Build inputs are verified in transfer and recorded.** The build downloads each configured region and its `.md5` from the publisher over TLS with certificate verification, verifies the file against the checksum before reading a byte of it (BR8.1), records the publisher's path, published timestamp and digest in the manifest (BR8.4), and identifies itself with a project `User-Agent` carrying a contact address. A mismatch or a download failure fails the whole build; nothing is published (W1). The checksum is integrity of transfer, not authenticity of content (T10): authenticity rests on TLS to the publisher, and the golden-fixture suite is the detection for a content change. | Build tests with a corrupted download and a mismatched checksum both fail the build with nothing written; the manifest of a good build carries every field. |
| NFR6.4.2 | NFR6.4 (proposed — see Amendments) | **Responses carry only what the contract names.** Error bodies are the typed `ApiError` with a project-authored `detail` (BR10.1); a panic in any phase is caught at the handler boundary and answered `503 internal` with one `FailureRecord` (BR10.4); `x-extract-key` is a lowercase hexadecimal digest and `current_build` matches `[A-Za-z0-9._-]{1,64}`, validated at start, so neither can carry a header line break. | A test that a handler panic yields `503 internal` with the contract body and one failure row; property tests on the two header values. |
| NFR3.2.5 | NFR3.2 | **The readiness endpoint discloses nothing and cannot be starved.** It answers a fixed ready / not-ready token with no build, region or version detail, and is exempt from requester limiting so a limited client cannot make a healthy instance look unhealthy to the platform. | Header and body assertions; a test that a requester at its limit still gets the readiness answer. |
| NFR6.4.3 | NFR6.4 (proposed — see Amendments); `team.md` Deployment | **Response headers per `team.md`.** Every response from this Unit carries `Strict-Transport-Security`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Content-Security-Policy: default-src 'none'` (an API response renders nothing, so the strictest policy costs nothing), and `Cache-Control: private, no-store` (the served bytes are keyed on a build the URL does not name, so no shared cache may hold them; the client's own local store is where a design keeps what it needs). TLS itself terminates at the platform edge and is Infrastructure Design's to confirm. | The deterministic header-assertion test `team.md` names as the no-DAST replacement, run on every merge. |
| NFR7.2.3 | NFR7.2 | **Supply chain.** `Cargo.lock` is committed and every build is `--locked`; `cargo audit` runs as a merge gate; the vendored OSM `.proto` files carry their MIT licence and upstream commit in the asset manifest (`team.md`); no crate in this Unit's tree keys anything on the raw client address (`tech-stack-decisions.md`, TS-8). | `scripts/verify.sh`; the asset manifest check. |
| NFR6.4.4 | NFR6.4 (proposed — see Amendments) | **The listener is hardened for a public edge.** A header read timeout and a maximum header size are configured on the server; the extract route accepts `GET` only and no request body; the request budget (NFR1.1.1) applies from the handler. Values are fixed at code generation from the HTTP stack's documented options. | Tests: a `POST` to the route is rejected by the router; a request whose headers exceed the limit is refused before the handler runs. |

## No authentication, by design

The extract endpoint is public and unauthenticated. This is not an omission:

- The data is public (ODbL) and the same for every requester.
- Product Stage 1 has no accounts (`requirements.md` FR7 is Stage 2) and
  ADR-004's whole point is that the proxy holds nothing about anybody; an
  API key per browser would be exactly the identifier it refuses to hold.
- What authentication would protect against — cost and abuse — is handled
  by the bounds in NFR5.1.7 and NFR5.1.8 instead.

If a credentialed consumer is ever wanted (a programmatic API for outside
callers), `contract-summary.md` already says both the shared-crate contract
and the build-stamp check must be reopened; this section is reopened with
them.

## Privacy and compliance notes

- **The address is personal data** (an IP address identifies a device under
  GDPR and, in Canada, PIPEDA's reading of personal information). This Unit
  processes it transiently to enforce a fair-use limit — a legitimate
  interest that a public, free, unauthenticated service has — and does not
  store it. The keyed hash is not personal data after the process ends,
  because the key that produced it is gone. No consent surface is needed for
  this endpoint; it sets no cookie and issues no identifier (NFR6.3.4).
- **No special-category data and no payment data** are touched (NFR6.3 as
  written).
- **ODbL attribution** for the served OpenStreetMap data is a product
  obligation carried by the client surface that displays it (U6), not by
  the bytes; this Unit's contribution is provenance — the manifest records
  the publisher's source, its published timestamp and digest (BR8.4), so
  every clip can be traced to the extract it was cut from.
- **Publisher politeness**: one download per region per week plus on-demand
  runs (BR8.5), the `-latest` file with its checksum, and an identifying
  `User-Agent` (NFR7.2.2). The publisher's own usage guidance is read and
  quoted in the build's documentation at code generation rather than
  paraphrased here.

## Amendments required

| Artifact | What must change | Why |
|---|---|---|
| `entities.md` `RequesterWindow` | One `RequesterWindow` per requester **per window length**: the entity gains a `windowLength` discriminator (minute or hour), or the model holds two windows per hash | NFR5.1.3 is two-tiered (Q1); the approved entity models one window per hash |
| `contract-summary.md` Contract 1 | The `429` response gains a `Retry-After` header | NFR5.1.7; the contract's `429` currently names only the typed body |
| `requirements.md` NFR6 | Gains **NFR6.4** — "Inputs at every system boundary are validated, outputs encoded, and responses carry the security headers `team-practices.md` names" — as the inception home of NFR6.4.1–NFR6.4.4 | The construction-phase guardrails and `team.md` require these controls, but no inception requirement names them; the inception guardrail that a new requirement must document its origin is met by this row |

## Assumptions & Open Questions

- The platform asserts the client address in a header whose name and
  position (first value, last value) Infrastructure Design will confirm;
  until then the limiter is written against an injected "asserted address"
  so the header choice is one function. [assumption]
- A cross-origin browser really is stopped by the preflight: this holds for
  every browser in `requirements.md` NFR4.6's matrix and for current desktop
  browsers, which all implement the fetch specification's preflight for
  non-safelisted headers; a non-browser client is unaffected and is bounded
  by NFR5.1.7 instead. [assumption]
- The publisher's checksum stays MD5 (verified 2026-09-12: a `.osm.pbf.md5`
  exists beside each extract). If the publisher adds a stronger digest the
  build verifies both. [assumption]
- No egress ceiling exists in this Unit by an approved decision
  (functional design Q4, option C rejected); the account-level usage limit
  is the only hard stop and is Environment Provisioning's to set. [assumption]

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-12T21:53:50Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `security-requirements.md` > `## Requirements` rows NFR6.3.1–NFR6.3.4, versus `inception/requirements-analysis/requirements.md` NFR6.3 and `security-requirements.md` > `## Privacy and compliance notes` | The `## Requirements` table labels NFR6.3.1–NFR6.3.4 as refining inception `NFR6.3`, following the stated scheme ("a third level refines an inception requirement"). But `requirements.md` NFR6.3 reads verbatim "No payment data and no special-category personal data shall be collected or stored" — a requirement about payment/special-category data, not about network-address privacy, hash unlinkability, or cookies, which is what NFR6.3.1–NFR6.3.4 actually specify. The document's own later prose confirms the mismatch rather than resolving it: `## Privacy and compliance notes` separately states "No special-category data and no payment data are touched (NFR6.3 as written)" — i.e. NFR6.3 as literally written is satisfied trivially and is not the source of the four address-privacy requirements above it. The document handles the analogous case correctly elsewhere: NFR6.4.1–NFR6.4.4 have no matching inception sub-requirement and are honestly numbered under a *proposed* `NFR6.4`, recorded in `## Amendments required`. NFR6.3.1–NFR6.3.4 needed the same treatment — a proposed new inception requirement (or NFR6.4) — and instead were silently attached to an unrelated existing id, breaking the traceability chain the whole numbering scheme depends on for four requirements, not one. | Recharacterise NFR6.3.1–NFR6.3.4 as refining a proposed new inception requirement (e.g. fold them into the same proposed `NFR6.4` group already used for NFR6.4.1–NFR6.4.4, or propose a distinct `NFR6.5` for "processing of the requester's network address is minimised and never linked to a location"), and add the corresponding row to `## Amendments required` the way NFR6.4 was added, so every derived id in this file actually refines the inception requirement it claims to. | New |
| R-02 | Minor | `security-requirements.md` > Threat model (STRIDE) row T14, versus `tech-stack-decisions.md` > "What was already decided elsewhere" | T14's Treatment column lists "osm2streets-related crates pinned by commit (`project.md`)" as one of this Unit's mitigations for a vulnerable dependency. `tech-stack-decisions.md`'s own "What was already decided elsewhere" table states the opposite for this exact rule: "Every osm2streets-related dependency is pinned by commit — project.md — Not this Unit's tree — the proxy does not depend on osm2streets (`functional-spec.md`: U1's fixtures are cut *by* this Unit, not with it)." Citing an osm2streets pin as a mitigation this Unit relies on contradicts the same stage's own tech-stack decision that this Unit has no osm2streets dependency at all. | Drop the osm2streets clause from T14's treatment (the remaining `Cargo.lock`/`--locked`/`cargo audit`/Dependabot mitigations are sufficient and accurate on their own), or replace it with a clause that actually applies to this Unit's tree (e.g. the vendored OSM `.proto` files' pinned upstream commit, TS-3). |  New |
| R-03 | Minor | `security-requirements.md` > `## Amendments required` row for `entities.md` `RequesterWindow` | The amendment gives two alternative fixes — "the entity gains a `windowLength` discriminator (minute or hour), or the model holds two windows per hash" — as though the choice were still open. But this stage's own `tech-stack-decisions.md` TS-8 and `scalability-requirements.md` NFR2.1.3 already commit to the second option: TS-8 specifies "two `(windowStartedAt, requestCount)` pairs" per requester hash in one `HashMap` entry, and NFR2.1.3 sizes the entry as "a 64-bit hash, two start times and two counts." Leaving the amendment phrased as an open "or" when the rest of this stage's own output has already decided is a documentation gap the developer must resolve by cross-referencing two other files. | Narrow the amendment to name the option the stage actually decided (two `(windowStartedAt, requestCount)` pairs per `RequesterWindow`, no `windowLength` discriminator), matching TS-8 and NFR2.1.3, so `entities.md`'s update has one unambiguous target. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `aidlc-sensor-traceability.ts` | `{"pass":true,"gaps":[],"orphans":[],"findings_count":0}` | All seven inception NFR groups (NFR1–NFR7) are accounted for in `traceability.json`, every listed target id resolves, and no gaps or orphans exist. |
| `aidlc-sensor-required-sections.ts` (×6 artifacts) | PASS on all six deliverable `.md` files | No template is configured for this stage, so the sensor reports the H2 structure found; each file's headings are consistent with its stated purpose (Requirements/Amendments/Assumptions sections present throughout). |
| `aidlc-sensor-upstream-coverage.ts` | `{"pass":true,"unreferenced":[]}` | All four declared `consumes` (`functional-spec`, `rules`, `requirements`, `contract-summary`) are referenced by name in `security-requirements.md`. |
| Manual: every `BR<n>.<m>` cited across the six `.md` deliverables vs. `rules.md`'s 32 rule ids | PASS, no unresolved citations | `comm` diff of the cited-id set against the defined-id set is empty — every cited BR id exists in `rules.md`. |
| Manual: every two-level inception `NFR<n>.<m>` used as a "Refines" parent vs. `requirements.md`'s defined ids | PASS on existence, FAIL on semantic match for one group | Every cited parent id (`NFR1.1, NFR2.1, NFR3.1–3.3, NFR5.1, NFR5.2, NFR6.1, NFR6.3, NFR7.1–7.3`) exists as a defined inception requirement — no dangling reference. However, `NFR6.3`'s defined text does not match the content of the four derived requirements that cite it as their parent (see R-01): existence of the id is not the same as correctness of the refinement, and this check only tests existence. |
| Manual: entity/attribute names cited in `security-requirements.md` vs. `entities.md`'s catalogue | PASS | `RequesterWindow`, `CachedExtract`, `Cell`, `FailureRecord`, `ServiceCounters`, `RegionalDataBuild`, `ExtractKey`, `BoundingBox` and their cited attributes (`requesterHash`, `sizeBytes`, `bytesServed`, etc.) all exist in `entities.md` with matching shapes. |
| Manual: the Q&A's Consolidated Summary Confirmation numbers vs. all six deliverables | PASS | Cell size 0.01°, span bound 12, 5-decimal outward-rounded precision, two fixed windows (30/min, 300/hour), 4 cutting slots, ≤30s Ready target, BLAKE3/MD5 split, and the named crate set (axum 0.8, tokio, tower-http, osmpbf, moka, blake3, md-5, tracing, reqwest) all appear identically across `performance-requirements.md`, `scalability-requirements.md`, `security-requirements.md` and `tech-stack-decisions.md`; no drifted number found. |
| Manual: arithmetic checks (egress/relentless-client cost, area cap, memory-per-cut, requester-window memory, manifest size) | PASS | 300/hour × 24h = 7,200 clips/day × 150 KB ≈ 1.08 GB/day × $0.05/GB × 30 days ≈ $1.62/month (stated as "$1.60/month", consistent); 128 MB × $10/GB-month ≈ $1.28/month (stated "$1.30", consistent); 8 MB/cut × 4 cuts = 32 MB in-flight (NFR5.1.2, consistent); 64 bytes/requester × 10,000 = 640,000 bytes = 640 KB (NFR2.1.3, exact); ≤80 bytes/cell × ~200,000 cells ≈ 16 MB (NFR2.1.5, exact). No arithmetic error found. |
| Manual: Contract 1 amendment claims (`429` gains `Retry-After`) vs. `contract-design/contract-summary.md`'s actual OpenAPI block | PASS | Contract 1's `429` response today refers only to the generic `TypedError` schema with no header; the amendment recorded in `## Amendments required` is real and necessary, and no other wire-affecting change (e.g. the `NFR6.4.3` response headers, which are existing `team.md`-level obligations rather than new contract fields) was found unrecorded. |
| Manual: STRIDE table completeness against the five named entry points in `## Trust boundaries and attack surface` | PASS except T14 (R-02) | Every one of the five listed entry points has at least one STRIDE row addressing it (extract endpoint: T1, T4–T7, T11, T12; readiness endpoint: T13; address header: T3; weekly download: T9, T10; deploy artifact: T8), and every row's stated Requirement column delivers the behaviour the Treatment column describes, except T14 as noted in R-02. Likelihood × impact classifications are internally consistent across all 16 rows (the same combinations always yield the same risk level). |

### Summary

The six deliverables are numerically and cross-referentially consistent with each other, with `rules.md`, `entities.md`, the Q&A's confirmed parameters, and `contract-summary.md`'s actual OpenAPI block; every validation tool passes clean, and no BR/entity citation is dangling. The one Major finding (R-01) is a real defect in the derived-id scheme this stage relies on for traceability: four requirements central to this Unit's privacy story (NFR6.3.1–NFR6.3.4) are labelled as refining an inception requirement whose own text — and the artifact's own later paragraph — is about something else entirely (payment/special-category data), when the document already knew how to handle an unmatched derivation correctly for the adjacent NFR6.4 group. This does not block a developer from building the described behaviour (the row content itself is sound and testable), but it does mean the stated traceability contract is broken for a quarter of this file's security requirements, so it is recorded as Major rather than Minor. Two Minor findings round out the pass: a stale osm2streets mitigation copied into a threat-model row for a Unit that has no osm2streets dependency, and an amendment left phrased as an open choice the stage's own other artifacts have already resolved. None of the three findings requires rework of the underlying design; all three are corrections to what a handful of table cells claim.
