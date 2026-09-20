# NFR Design Questions — `accounts-sharing` (U11)

All answers self-answered under standing autonomous authorization
(2026-09-20), grounded in `nfr-requirements/*.md` (READY, iteration 2) and
the `design-storage` (U10) sibling precedent.

**Mode:** self-answered (autonomous, user-authorized 2026-09-20)

## Q1 — Caching / performance (NFR1.5.1-3)

Any caching layer, or plain indexed queries?

Options:
- A. A cache layer (Redis or in-process) fronting account/session lookups
- B. No cache — indexed Postgres queries only, same as `design-storage`
- X. Other (please specify)

[Answer]: B. `Session.token_hash` and `Account.email` both carry unique
indexes (NFR2.4.2); at this project's single-instance, no-second-person
scale (`project.md` OC-4's ~$5/month budget), an indexed point-lookup
comfortably clears the 300ms p95 targets without adding a cache layer's
own invalidation surface. `design-storage` made the same call for its own
lookups.

## Q2 — Sharing-state list-query shape (NFR1.5.2)

How is the per-design sharing-state computed for a 100-row list without
N+1 queries?

Options:
- A. One query with a `LEFT JOIN`/subquery computing Grant count and
  ShareLink.enabled per row
- B. One query per SavedDesign row (N+1)
- X. Other (please specify)

[Answer]: A. A single query joins `SavedDesign` to an aggregated `Grant`
count (`GROUP BY stored_design_id`) and `ShareLink.enabled`, resolving
`performance-requirements.md`'s open assumption that this must not be an
N+1 shape.

## Q3 — Account-deletion cascade mechanism (NFR3.5.1)

Options:
- A. One database transaction deleting all four tables' rows for an
  `account_id`, committed or rolled back atomically
- B. Four separate deletes with application-level compensation on failure
- X. Other (please specify)

[Answer]: A. Matches `reliability-requirements.md`'s own stated assumption
and Postgres's native transactional guarantee — the simplest mechanism
that actually satisfies AC11.1.4's all-or-nothing property, no
compensation logic needed. Per the `[flagged gap, review R-02]` note in
`nfr-requirements/security-requirements.md`, this is designed as a
capability `cityloom-accounts-sharing` exposes (an internal function, not
a public route — `data-rights`/U12 is the one that decides when to call
it); no BR exists for it in this Unit's functional-design, so this design
element cites the NFR/Contract 6 directly rather than a nonexistent BR.

## Q4 — Fail-closed access-gate default (NFR6.2.7)

How does "unset reads as off" actually get implemented in code?

Options:
- A. Config loading uses a typed `Option<bool>` (or equivalent) with no
  default-true fallback anywhere in the loading path — absence is
  `None`, mapped to `false` at the one point the flag is read
- B. A boolean with `#[serde(default)]` set to `true`, relying on callers
  to override it
- X. Other (please specify)

[Answer]: A. Option B is exactly the fail-open bug NFR6.2.7 exists to
prevent — a default-`true` annotation is invisible at the call site and
one missed env var away from a public accounts surface. Config loading
never assigns a default; the flag's effective value is computed once at
startup as `config.access_gate_enabled.unwrap_or(false)`.

## Q5 — Session revocation check point (NFR6.2.8)

Options:
- A. Every authenticated route re-queries the `Session` table by token
  hash on each request (no in-process session cache)
- B. Sessions are cached in-process after first validation, invalidated
  on an explicit revoke event
- X. Other (please specify)

[Answer]: A. A cache reintroduces exactly the "grace window" NFR6.2.8
explicitly forbids (a revoked session cached for even a few seconds is a
window where a stolen or logged-out session token still works). The
per-request query is the same single indexed lookup NFR2.4.2 already
requires to be fast; no separate cache invalidation mechanism is needed.

## Q6 — Observability export mechanism

Options:
- A. Same stdout-JSON-rows pattern `osm-extract-proxy`/`design-storage`
  established
- B. A new mechanism (structured logging library, external metrics
  service)
- X. Other (please specify)

[Answer]: A. `observability-requirements.md`'s own stated assumption;
consistent with this workspace's established pattern, no new dependency.

## Ambiguity / contradiction check

No contradictions found between the six requirement files or against
`functional-spec.md`/`rules.md`. NFR3.5.1's missing functional-design BR
(the `[flagged gap, review R-02]` item) is a known, human-acknowledged gap
carried forward from `nfr-requirements` — not a new ambiguity this stage
introduces; this stage designs the capability without retroactively
inventing a BR ID for it.

## Consolidated Summary Confirmation

- Looks correct
- Request changes

[Answer]: Looks correct
