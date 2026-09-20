# Security Design — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._

Upstream inputs: `security-requirements.md` (nfr-requirements, this Unit,
READY iteration 2), `functional-spec.md`/`rules.md`/`entities.md`
(functional-design, this Unit), `contract-summary.md` Contract 4/5,
`design-storage/security-design.md` (sibling precedent — layered defence,
`SD-n` numbering convention), `tech-stack-decisions.md` (argon2, session
token hashing).

Design elements are numbered `SD-n`; `traceability.json` maps each
`NFR6.x.y` from `security-requirements.md` to the elements that meet it.

## Defence in depth, as layers a request passes through

```
 internet
    |
    v
 [L0] platform edge ......... TLS termination (shared Railway service,
    |                          same as U9/U10)
    v
 [L1] access gate ........... SD-1: fail-closed config flag check, first
    |                          in the handler chain, before routing logic
    |                          runs — refuses while off/unset (NFR6.2.5,
    |                          NFR6.2.7)
    v
 [L2] listener + router ..... method/path matching only for this Unit's
    |                          routes; catch-panic boundary
    v
 [L3] session resolution .... SD-4: token hash lookup, fail if missing,
    |                          expired, or the row was deleted (NFR6.2.8)
    v
 [L4] authorization check ... SD-5: Grant/ShareLink check before any
    |                          DesignRepository read (NFR6.2.3)
    v
 [L5] typed data access ..... sqlx compile-time-checked queries; no
                               request-derived text in SQL beyond a bound
                               parameter (SD-7)
```

## SD-1 — Fail-closed access gate

Resolves NFR6.2.5/NFR6.2.7. Config loading (`tech-stack-decisions.md`)
reads the gate flag into `Option<bool>`; the ONE place its effective value
is computed is `config.access_gate_enabled.unwrap_or(false)` — no
`#[serde(default = "...")]` or similar default-true annotation exists
anywhere in the loading path (`nfr-design-questions.md` Q4). This computed
`bool` is checked first in every route handler, before any other logic,
via a shared middleware/extractor rather than duplicated per-handler
(reduces the chance one new route forgets the check).

## SD-2 — Password hashing

Resolves NFR6.2.1 (password half). `argon2` (RFC 9106 defaults,
`tech-stack-decisions.md`); verification uses the crate's own
constant-time `verify_password` function, never a manual byte comparison.

## SD-3 — Session token hashing and lookup

Resolves NFR6.2.1 (session half), NFR6.2.8. A session token is a random
256-bit value; only its SHA-256 hash is stored (`Session.token_hash`,
unique-indexed per `scalability-design.md` SC-2). Every authenticated
request re-queries this table by hash — no in-process cache
(`nfr-design-questions.md` Q5) — so a deleted `Session` row is
unreachable on the very next request, satisfying NFR6.2.8's no-grace-window
requirement directly as a consequence of the lookup path, not a separate
invalidation mechanism.

## SD-4 — Session cookie flags

Resolves NFR6.2.2. `HttpOnly`, `Secure`, `SameSite` are fixed by
`contract-summary.md` Contract 4 — this Unit's `Set-Cookie` response
construction sets all three unconditionally, never behind a config flag.

## SD-5 — Authorization before read (BR8.1)

Resolves NFR6.2.3. `SharingService`'s shared-design-read handler:
resolve caller (session `Grant` or `ShareLink` token) → authorize →
only then call `DesignRepository::fetch`. No code path constructs the
`DesignRepository` call before the authorization check returns `Ok`.

## SD-6 — ShareLink token entropy

Resolves NFR6.2.4. A `ShareLink.token` is a random 128-bit-or-greater
value (matching `design-storage`'s `anonymousDesignId` treatment,
`security-requirements.md` T4) — never derived from `stored_design_id`,
`account_id`, or any other correlatable input.

## SD-7 — Typed errors, no leaked driver text

Resolves NFR6.2.6. Same discipline as `design-storage`'s SD-6: every
`sqlx::Error` maps to a small closed `AccountsSharingFailure` enum before
crossing the handler boundary; the underlying error is logged once
server-side (with no credential/token value in the log line, `entities.md`
classification table) and never rendered to a client response.

## SD-8 — Secrets

Resolves NFR6.1.2. No credential, connection string, or the database
password is committed; supplied only as Railway environment variables
(`team.md` Deployment) — same as every other Unit.

## Traceability

See `traceability.json` in this directory.

_Confirmed (consolidated summary confirmed 2026-09-20)._

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-20T15:48:18Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | `security-design.md` — no `SD-n` element addresses sign-in brute-force/credential-stuffing resistance | Every confirmed NFR6.x requirement (hashing, cookie flags, authorization-before-read, token entropy, fail-closed gate, typed errors, secrets, session revocation) has a concrete `SD-n` behind it — `traceability.json`'s coverage table is complete and every target resolves to a real design element in this or a sibling `nfr-design` file. No NFR in `security-requirements.md` requires rate-limiting or lockout on repeated sign-in failures, so this is not a coverage gap against a confirmed requirement and does not block READY. It is a design decision not made either way (accept the residual brute-force exposure, or add a throttle) on a surface (`handlers`, per `logical-components.md` LC-1) that is new attack surface for this project (U9/U10 have no credential-guessing surface of their own, per `scalability-design.md` SC-1's own contrast). | Optional: record the decision explicitly (accept residual risk given `project.md` OC-4's budget constraint, or add a per-account/per-IP throttle) so it reads as a decision rather than an omission. | New |
| R-02 | Minor | `security-design.md` SD-6 | SD-6 specifies token length ("128-bit-or-greater") but not the entropy source. `nfr-design-questions.md` does not ask this as a separate question, and `security-requirements.md` NFR6.2.4's own verified-by clause only tests statistical unrelatedness across tokens, not the generation mechanism, so this does not fail any confirmed requirement. | Optional: name the CSPRNG (e.g. the `rand` crate's `OsRng`/`rand::rngs::ThreadRng` seeded from the OS) at `code-generation` time if not already implied by a workspace-wide convention. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| Cross-check every confirmed NFR in `nfr-requirements/*.md` (NFR1.5.1-3, NFR2.4.1-2, NFR3.2.2, NFR3.5.1, NFR4, NFR6.1.2, NFR6.2.1-8, NFR7.5.1-4) against `traceability.json`'s `coverage` array | PASS — 20 upstream ids, 20 coverage rows, 1:1, every target (`PD-1/2/3`, `SC-1/2/3`, `RD-1/2/3`, `SD-1..8`, `OD-1/2`) resolves to a real, non-restated design element in `performance-design.md`, `scalability-design.md`, `reliability-design.md`, `security-design.md`, or `observability-design.md` | No orphan targets, no unaddressed confirmed NFR |
| NFR6.2.7 fail-closed mechanism check | PASS — `security-design.md` SD-1 states a concrete mechanism: `config.access_gate_enabled.unwrap_or(false)` computed once, no default-true annotation anywhere in the loading path, checked via shared middleware before route logic; matches `nfr-design-questions.md` Q4's self-answer verbatim | Not a restated requirement; a real, code-level commitment |
| NFR6.2.8 vs. performance/scalability cache consistency check | PASS — `security-design.md` SD-3 ("no in-process cache... `nfr-design-questions.md` Q5") and `performance-design.md` PD-1 ("no cache layer, matching `design-storage`'s own resolution") and `scalability-design.md` SC-1 (no `HashMap`-backed session store) all independently state the same no-cache decision; no file implicitly requires a cache that would contradict SD-3 | No cross-file inconsistency found on this specific axis |
| `logical-components.md` LC-2 dependency-declaration check | PASS — read `unit-of-work-dependency.md` directly (not trusted from the claim): line 48-50 declares `- name: accounts-sharing / kind: service / depends_on: [design-storage]`, and line 92 of the same file's diagram shows `accounts-sharing --> design-storage` | LC-2's claimed edge is a real, already-approved part of the unit topology, not a new undeclared cross-unit dependency |
| RD-2 / LC-4 scope-boundary check against `unit-of-work.md` | PASS — `unit-of-work.md`'s U11 section states "**Boundary.** Identity and authorisation grants. Erasure and export are U12." `RD-2` and `LC-4` both describe the deletion cascade as an internal, non-route capability this Unit exposes but does not decide when to invoke, and `LC-4` explicitly lists "the *decision* of when to delete an account (U12, `data-rights`)" as out of this Unit's scope | The carried-forward RD-2 gap is disclosed honestly; no silent scope expansion into U12's territory |
| Internal consistency across all 7 files on shared concepts (DB-backed sessions, access-gate flag, no-cache) | PASS — `security-design.md`, `performance-design.md`, `scalability-design.md`, `reliability-design.md` all describe the same DB-row session model (RD-1, SD-3, SC-1, PD-1) with no contradiction found; `logical-components.md` LC-1's module table matches the SD-numbered elements it names | No contradiction found among the seven artifacts |
| Escaped-pipe check on this review's own table cells | PASS — no unescaped literal `\|` introduced | Table renders correctly |

### Summary

Every confirmed NFR from `nfr-requirements/` has a concrete, non-restated design element behind it in `traceability.json`, and every cross-file claim checked against evidence — the fail-closed gate mechanism, the no-cache consistency between security/performance/scalability, the `cityloom-design-storage` dependency edge (verified directly in `unit-of-work-dependency.md`, not trusted from the claim), and the RD-2/LC-4 scope boundary against U12 (verified directly in `unit-of-work.md`) — held up. Two Minor findings are recorded as optional decisions to make explicit (brute-force resistance on sign-in, token-entropy source), neither backed by an unmet confirmed NFR, so neither blocks READY.
