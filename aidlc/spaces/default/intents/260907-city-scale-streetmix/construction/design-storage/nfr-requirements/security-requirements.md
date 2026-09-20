# Security Requirements — `design-storage` (U10)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md`, `entities.md`
(functional-design, this Unit); `requirements.md` NFR6;
`contract-summary.md` Contract 2; `components.md` `DesignRepository`.

## What this Unit protects, and from whom

Unlike `osm-extract-proxy` (U9), this Unit **does** hold user content —
a design's lane edits, corrections, and (via the payload) provenance
data. It holds no account/identity data in Stage 1 (no accounts exist
yet), but the design content itself is the asset an attacker would want:
reading someone's unpublished proposal, or deleting/tampering with it.

| Asset | Classification | Where it is |
|---|---|---|
| `StoredDesign.payload` | User content (design/lane data, not PII) | Database, at rest |
| `StoredDesign.anonymous_design_id` | Pseudonymous identifier — the sole credential for Stage 1 access | Database; also held by the client (U7) |
| `StoredDesign.owner_account_id` | Reserved for Stage 2; unset in this Unit's implementation scope | Database column, always null in Stage 1 |

## Threat model (STRIDE)

| # | Threat | Category | Treatment | Requirement |
|---|---|---|---|---|
| T1 | An attacker guesses or enumerates `anonymousDesignId` values to read others' designs | Information disclosure | 128-bit identifier (2^128 space) is unguessable by brute force alone; rate limiting (BR4.1) makes even a targeted guessing campaign impractical | NFR6.4.1 |
| T2 | An unauthenticated or ungranted request is answered with the design instead of a refusal | Information disclosure / Elevation of privilege | BR3.1 — every read is authorization-checked before the payload is ever placed in a response body | NFR6.4.2 |
| T3 | A malformed or oversized payload is used to exhaust storage or crash the service | Denial of service | Payload size cap and `payloadVersion` fail-closed check (Contract 3) applied before any write (BR6.1) | NFR6.4.3 |
| T4 | A dependency's native database error (e.g. a SQL error containing schema detail) reaches the client | Information disclosure | BR6.1 — typed `StorageFailure` only, no native error type crosses the response boundary (same discipline as `osm-extract-proxy`'s BR10.1/BR10.4) | NFR6.4.4 |
| T5 | An anonymous design is stored with an identifying field (IP, cookie, fingerprint) alongside it, defeating the anonymous path | Information disclosure | BR7.1 — the stored row carries only the client-supplied identifier and the payload, nothing else | NFR6.3.1 |
| T6 | A secret (database credential) reaches the public repository | Information disclosure | No secret in the repository; the database connection string is a Railway environment variable (`team.md` Deployment) | NFR6.1.1 |
| T7 | Repeated access/delete attempts against one identifier, or scanning many identifiers, go unbounded | Denial of service | BR4.1 rate limiting | NFR5.3.1 |

## Requirements

| ID | Refines | Requirement | Verified by |
|---|---|---|---|
| NFR6.1.1 | NFR6.1 | No credential or connection string is committed; the database connection string is supplied as a Railway environment variable. | `gitleaks` in CI (`team.md`); the asset/dependency manifest lists no credentialed source in this Unit's own configuration. |
| NFR6.3.1 | NFR6.3 | An anonymous `StoredDesign` row carries no field beyond `anonymous_design_id`, `payload`, `created_at`, `expires_at` — no IP address, cookie, or device fingerprint is stored alongside it. | A test inspects the schema/row shape and asserts no such column exists; code review checks no handler code reads or persists a request-identifying value. |
| NFR6.4.1 | (proposed — see Amendments) | An `anonymousDesignId` is never derived from, or reveals anything about, the design's content or the requester; it is stored and compared as an opaque 128-bit value. | A test that two designs with identical payloads have unrelated stored identifiers. |
| NFR6.4.2 | (proposed) | Every read (`GET /api/designs/{id}`) is authorization-checked (BR3.1) before any payload byte is placed in the response; a 403/404 response body never contains payload data. | A test asserts the byte content of a 403/404 response contains no payload field. |
| NFR6.4.3 | (proposed) | A payload exceeding the size cap, or failing `payloadVersion` validation, is rejected (`413`/`400`) before any database write is attempted. | A test with an oversized payload asserts no row is created; a test with a bad `payloadVersion` asserts the same. |
| NFR6.4.4 | (proposed) | No database-native error type or message reaches a response body; every failure maps to one of `StorageFailure`'s closed reasons. | A test that a simulated database error (e.g. a forced connection failure) yields `internal` with no leaked driver text. |
| NFR5.3.1 | NFR5 (cost, restated as an abuse bound) | Access and delete attempts against a given identifier are rate-limited (BR4.1); a client exceeding the threshold receives `429` with `Retry-After`. | A test drives repeated requests against one identifier past the threshold and asserts `429`. |

## Amendments required

| Artifact | What must change | Why |
|---|---|---|
| `requirements.md` NFR6 | Gains a proposed **NFR6.4** — "Design storage inputs are validated, outputs are authorization-checked before release, and no dependency-native error reaches a response" — as the inception home for NFR6.4.1–NFR6.4.4, following the precedent `osm-extract-proxy` set for its own NFR6.4 group | No inception requirement currently names these storage-specific boundary controls |

## Assumptions & Open Questions

- **[assumption]** The exact rate-limiting threshold (NFR5.3.1) and
  payload size cap (NFR6.4.3) numeric values are fixed at
  `tech-stack-decisions.md`/code-generation, consistent with this
  project's practice of deferring unmeasured numbers rather than
  inventing them here.

## Traceability

See `traceability.json` in this directory.
