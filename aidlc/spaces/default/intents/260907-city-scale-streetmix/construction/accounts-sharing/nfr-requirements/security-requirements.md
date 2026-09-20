# Security Requirements — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._


Upstream inputs: `functional-spec.md`, `rules.md`, `entities.md`
(functional-design, this Unit); `requirements.md` NFR6; `contract-summary.md`
Contract 4/5; `components.md` `AccountService`/`SharingService`;
`project.md` Mandated (access gate, data subject rights).

## What this Unit protects, and from whom

Unlike `design-storage` (U10), this Unit **does** hold identity data —
credentials and sessions — plus the authorization decision (BR8.1) that
gates access to another Unit's content. It is also the one Unit
`project.md` names directly: its whole surface must stay behind a
server-side access-gate flag until `data-rights`/U12 exists.

| Asset | Classification | Where it is |
|---|---|---|
| `Account.password_hash` | Credential | Database, at rest — never the plaintext password |
| `Session.token_hash` | Session credential | Database, at rest; the raw token is held only by the client as a cookie |
| `Grant`, `ShareLink.token` | Authorization state | Database — a `ShareLink.token` is a bearer credential for public link readers |
| The access-gate configuration flag | Release-control state | Server configuration (Railway environment variable), not a database row |

## Threat model (STRIDE)

| # | Threat | Category | Treatment | Requirement |
|---|---|---|---|---|
| T1 | An attacker who reads the database recovers a usable password or session token | Information disclosure | Both are stored hashed, never in a reversible or comparable-in-the-clear form (BR1.1/entities.md `Account`/`Session`) | NFR6.2.1 |
| T2 | A session cookie is readable by client-side script, sent over plaintext HTTP, or replayed cross-site | Tampering / Information disclosure | Contract 4's fixed cookie flags: `HttpOnly`, `Secure`, `SameSite` | NFR6.2.2 |
| T3 | A read request reaches the shared-design payload without passing BR8.1's authorization check | Elevation of privilege | Every read authorizes before it reaches `DesignRepository` (BR8.1) — no code path returns payload bytes before that check | NFR6.2.3 |
| T4 | A `ShareLink.token` is guessable or enumerable, letting a non-recipient discover a shared design | Information disclosure | Token is a high-entropy opaque value (matching `design-storage`'s `anonymousDesignId` treatment — unguessable by brute force) | NFR6.2.4 |
| T5 | The accounts/sharing surface is reachable in production before data subject rights exist | Elevation of privilege (of the product's own release posture) | BR2.1 — the whole surface refuses every request server-side while the access-gate flag is off, verifiably (see `observability-requirements.md` NFR7.5.3) | NFR6.2.5 |
| T6 | A secret (session-signing input, database credential) reaches the public repository | Information disclosure | No secret is committed; supplied only as a Railway environment variable, same discipline as every other Unit (`team.md` Deployment) | NFR6.1.2 |
| T7 | A database-native error (constraint violation, connection failure) leaks schema or driver detail to a client | Information disclosure | Every failure maps to a closed, typed error set before crossing the response boundary — same discipline `design-storage`'s NFR6.4.4 established | NFR6.2.6 |

## Requirements

| ID | Refines | Requirement | Verified by |
|---|---|---|---|
| NFR6.1.2 | NFR6.1 | No credential, connection string, or session-signing secret is committed to the repository; each is supplied as a Railway environment variable. | `gitleaks` in CI; the asset/dependency manifest lists no credentialed source in this Unit's configuration. |
| NFR6.2.1 | NFR6.2 | `Account.password_hash` and `Session.token_hash` are stored only as one-way hashes; no code path stores, logs, or compares either in plaintext. | A test asserts the stored value differs from the input and that comparison uses the hashing library's constant-time verify function, not `==`. |
| NFR6.2.2 | NFR6.2 | Every `Set-Cookie` response for a session carries `HttpOnly`, `Secure`, and an explicit `SameSite` value. | A test asserts all three flags are present on every response that sets the session cookie. |
| NFR6.2.3 | NFR6.2 | A shared-design read is authorized (BR8.1) before any payload byte is read from `DesignRepository` or placed in a response. | A test for an unauthorized caller asserts the response contains no payload data and that no `DesignRepository` read was attempted (a mock/spy call-count assertion). |
| NFR6.2.4 | NFR6.2 | A `ShareLink.token` is a high-entropy value (128 bits of randomness or greater), never derived from or correlated with the design's content or the granting account. | A test that two links for different designs by the same account have statistically unrelated token values (same treatment class as `design-storage`'s NFR6.4.1). |
| NFR6.2.5 | NFR6.2 | While the access-gate flag reads off, every endpoint in this Unit refuses every request server-side, independent of the caller's knowledge of the URL or any client-side UI state. | A test with the flag off asserts every route returns a refusal, including routes a client would never normally expose a link to. |
| NFR6.2.6 | NFR6.2 | No database-native error type or message reaches a response body; every failure maps to a closed, typed failure reason. | A test with a simulated database error (e.g. forced connection failure) asserts the response contains no driver-originated text. |
| NFR6.2.7 | NFR6.2 | The access-gate configuration flag's absence or unset state is treated as off (fail-closed) — there is no code path where a missing/unset flag reads as enabled. | A test that unsets/omits the config variable entirely (not merely setting it false) asserts every route in this Unit still refuses the request, exactly as when the flag is explicitly off (NFR6.2.5). |
| NFR6.2.8 | NFR6.2 | A `Session` row that has been deleted or otherwise revoked causes the very next request bearing that session's token to be refused, with no grace window — the DB-backed-session design's whole rationale over a stateless token is this immediate revocability. | A test deletes an active `Session` row, then presents its token on a subsequent request, and asserts the request is refused exactly as an unauthenticated one would be. |

## Amendments required

None — NFR6.1.2 and NFR6.2.1–NFR6.2.6 elaborate `requirements.md`'s
existing NFR6.1 and NFR6.2 items; they do not add a new top-level NFR
(unlike `design-storage`'s NFR6.4, which extended the group because no
existing item covered its guessing-resistance concern — here NFR6.2
already exists and covers this Unit's concern directly).

## Assumptions & Open Questions

- **[assumption]** The specific password-hashing algorithm (argon2id vs.
  bcrypt/scrypt) is a `tech-stack-decisions.md` choice, not fixed here —
  this stage fixes the requirement (one-way, no plaintext), not the
  algorithm.
- **[assumption]** T5/NFR6.2.5's verification depends on
  `observability-requirements.md`'s access-gate-refusal counter existing
  so the "verifiably off" claim is checkable in production, not just in
  tests — cross-referenced there, not duplicated as a separate
  requirement here.
- **[flagged gap, review R-02]** `reliability-requirements.md`'s
  NFR3.5.1 (atomic account-deletion cascade) is grounded in
  `contract-summary.md` Contract 6 and AC11.1.4 (`stories.md` US11.1,
  owned by `data-rights`/U12), but this Unit's own already-reviewed
  `functional-design` (`rules.md`, `functional-spec.md`) never specified
  a cascade-delete capability as a BR or workflow step — only
  `entities.md`'s note that U12 sets `deleted_at`. The capability is
  real and needed (U12 must be able to call it), but it was missed at
  functional-design time rather than invented here. Recorded here as a
  known gap between stages rather than silently patched back into the
  closed, already-reviewed `functional-design` artifacts — a human
  should confirm this Unit's functional-design gets a follow-up BR for
  it (or that `nfr-design`/`code-generation` is sufficient authority to
  build it from this NFR alone).

## Traceability

See `traceability.json` in this directory.

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-20T15:29:38Z
**Iteration:** 2
**Request Challenge:** review:de17b92e327cfb179c70202e78d3bb5e

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `security-requirements.md` NFR6.2.7, cross-checked against `project.md` Mandated ("a single server-side flag... defaulting to off") | Fixed. NFR6.2.7 now states: "The access-gate configuration flag's absence or unset state is treated as off (fail-closed) — there is no code path where a missing/unset flag reads as enabled," with a real test-based verification method: "A test that unsets/omits the config variable entirely (not merely setting it false) asserts every route in this Unit still refuses the request, exactly as when the flag is explicitly off (NFR6.2.5)." This is a genuine requirement row in the table, not prose, and it directly closes the default-off gap `project.md`'s Mandated rule exists to guard against. | None. | Resolved |
| R-02 | Major | `reliability-requirements.md` NFR3.5.1; `security-requirements.md` § "Assumptions & Open Questions" | Fixed, via the disclosed third option (flag the gap rather than silently invent or delete it). (a) NFR3.5.1's grounding is genuine: `contract-summary.md` Contract 6 row states "a deletion that fails partway is retried to completion or leaves the account intact and says so, never a half-deleted account with orphaned designs (AC11.1.4)" — this is exactly the all-or-nothing cascade property NFR3.5.1 restates, so the NFR is not invented, only its functional basis (a BR in `rules.md`) is still missing. (b) `security-requirements.md`'s Assumptions section now carries an explicit `[flagged gap, review R-02]` note naming the mismatch (Contract 6/AC11.1.4 grounds the NFR; `rules.md`/`functional-spec.md` never specify the cascade as a BR or workflow step; `entities.md` only notes U12 sets `deleted_at`), stating the capability is real and needed, and asking a human to confirm whether a follow-up BR is required in functional-design or whether `nfr-design`/`code-generation` has sufficient authority to build it from the NFR alone. This is clear enough for a human at the gate summary to understand both the gap and the decision it needs. Note: the disposition lives in `security-requirements.md`'s Assumptions section rather than `reliability-requirements.md`'s own "Amendments required" section (which still reads "None" and does not cross-reference the flag) — a human scanning `reliability-requirements.md` alone would not find it, though the traceability chain (`security-requirements.md` is reviewed in the same gate) makes it discoverable. | None blocking; optional polish — a one-line cross-reference from `reliability-requirements.md`'s own "Amendments required" section to the `[flagged gap, review R-02]` note would make the gap visible from the file that actually carries NFR3.5.1, not only from a sibling file's Assumptions section. | Resolved |
| R-03 | Major | `security-requirements.md` NFR6.2.8 | Fixed. NFR6.2.8 states: "A `Session` row that has been deleted or otherwise revoked causes the very next request bearing that session's token to be refused, with no grace window — the DB-backed-session design's whole rationale over a stateless token is this immediate revocability," verified by "A test deletes an active `Session` row, then presents its token on a subsequent request, and asserts the request is refused exactly as an unauthenticated one would be." This is not circular (it tests observable request-refusal behaviour, not an implementation detail) and is directly executable against the `Session` entity already defined in `entities.md`. It also now ties the revocability rationale from `nfr-requirements-questions.md` Q2 to a checkable requirement. | None. | Resolved |
| R-04 | Minor | `traceability.json` `upstream_ids` / `coverage`, cross-checked against `rules.md` BR9.2 | Fixed. `upstream_ids` now includes `"NFR4"`, with `coverage` target: `"functional-design/rules.md BR9.2 (named, screen-reader-announceable sharing-state string) — this Unit's server-side contribution to NFR4.1/NFR4.3; the remaining NFR4 sub-items (keyboard operability, viewport widths, mobile matrix) apply to client-surfaces/U6's rendering, not this server Unit."` Verified against `rules.md` BR9.2 directly: its statement ("Sharing state is exposed as a named, announceable field, never as an icon-only signal with no accessible name") and its `violation_behaviour` ("This is an API-shape obligation... the announcement itself is client-surfaces/U6's rendering responsibility") match the traceability claim exactly. The scoping is honest, not overclaiming: it explicitly limits this Unit's contribution to NFR4.1/NFR4.3 and defers the rest to U6. | None. | Resolved |
| R-05 | Minor | Review-scope limitation on dispatch item 1 (cross-Unit NFR-numbering collision) | Unchanged from iteration 1 — this iteration's dispatch brief repeats the same per-unit read-scope restriction ("this Unit's artifacts plus the listed upstream files only... don't repeat the blocked cross-unit grep"), so `design-storage`'s own NFR files remain unverifiable from this session for the same reason. Not a defect in the artifact under review. | No action against this artifact; the human approving this gate should note that full cross-Unit ID-collision coverage still rests on indirect evidence (cross-references within this Unit's own artifacts), not a direct sibling read. | Accepted risk |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| Access-gate default-off requirement check against `project.md` Mandated | PASS — NFR6.2.7 exists, states the fail-closed default, has a test-based verification method | Closes R-01 |
| Functional-grounding check of `reliability-requirements.md` NFR3.5.1 against `contract-summary.md` Contract 6 / AC11.1.4 | PASS — Contract 6's stated erasure behaviour (retried to completion or intact, never half-deleted, AC11.1.4) matches NFR3.5.1's all-or-nothing cascade claim | Confirms R-02(a); the NFR is grounded, only its `rules.md` BR is still missing, which is what the flagged-gap note now discloses |
| Gap-disclosure clarity check of the `[flagged gap, review R-02]` note in `security-requirements.md` | PASS — names the mismatch, the source artifacts on both sides, and the decision needed from a human | Confirms R-02(b) |
| Session-revocation testability check of NFR6.2.8 | PASS — asserts an observable, non-circular outcome (next request refused) against an entity (`Session`) that already exists in `entities.md` | Closes R-03 |
| NFR4 traceability-target check against `rules.md` BR9.2 | PASS — BR9.2's statement and violation_behaviour match the `traceability.json` target description exactly, and the scoping to NFR4.1/NFR4.3 (not the full NFR4 group) is stated, not implied | Closes R-04 |
| Escaped-pipe check on NFR6.2.7 and NFR6.2.8 table rows, and on the new `[flagged gap, review R-02]` bullet | PASS — no unescaped literal `\|` found in any new table cell or bullet | Confirms no new Markdown table corruption was introduced by this iteration's edits |
| `traceability.json` internal consistency | PASS — `upstream_ids` (`NFR1, NFR2, NFR3, NFR4, NFR6, NFR7`) has exactly one `coverage` entry per id, and the `NFR6` coverage target (`NFR6.1.2, NFR6.2.1-NFR6.2.8`) already reflects the two new requirements | Confirms the traceability file was updated consistently with the new NFR6.2.7/NFR6.2.8 rows |

### Summary

All three Major findings from iteration 1 are independently verified as fixed: R-01's fail-closed default is now a real, test-verifiable requirement (NFR6.2.7); R-02's cascade-delete NFR is confirmed genuinely grounded in Contract 6/AC11.1.4 and its functional-design gap is now disclosed clearly enough for a human to act on at the gate, rather than silently invented or dropped; R-03's session-revocation property is now a testable, non-circular requirement (NFR6.2.8). R-04's NFR4 traceability gap is closed with an honestly scoped target verified against `rules.md` BR9.2 directly. No new unescaped table pipes were introduced, and `traceability.json` remains internally consistent. R-05 (the per-unit review-scope limitation on the cross-Unit ID-collision check) persists unchanged under this iteration's identical scope restriction and is not a defect in this artifact; it is recorded as an accepted risk for the human at the gate. With zero Critical and zero unresolved Major findings, this artifact is READY.
