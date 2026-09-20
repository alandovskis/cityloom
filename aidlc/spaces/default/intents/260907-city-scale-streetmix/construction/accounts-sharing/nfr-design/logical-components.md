# Logical Components — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._

Upstream inputs: `components.md` `AccountService`/`SharingService`
(domain-design), `tech-stack-decisions.md` (nfr-requirements, this Unit),
all five other `nfr-design` artifacts in this directory, `team.md`'s "port
and composition root are one fix, not two" corollary,
`design-storage/logical-components.md` (sibling precedent — same
three-inward-pointing-layers discipline).

Design elements are numbered `LC-n`.

## LC-1 — What is inside `cityloom-accounts-sharing`

`components.md` names two components for this Unit, `AccountService` and
`SharingService`. Inside the crate:

| Module | Responsibility |
|---|---|
| `handlers` | Route handlers for sign-up/sign-in, save-with-name/list/reopen, grant/link operations, and the shared-design read — orchestrates the access-gate check (SD-1), session resolution, authorization, and one query per operation; no SQL itself |
| `repository` | `sqlx` query functions and the `sqlx::Error` → `AccountsSharingFailure` mapping (SD-7) |
| `auth` | Password hashing/verification (SD-2) and session-token hashing/issuance/lookup (SD-3) — plain Rust logic, no `axum`/`sqlx` dependency, unit-testable in isolation (`team.md` TDD ordering) |
| `deletion` | The account-deletion cascade transaction (RD-2) — an internal function, not a route; the one place `data-rights`/U12 will eventually call into |
| `failure` | The `AccountsSharingFailure` type (`thiserror`) |
| `observability` | Counter and log-row emission points (`observability-design.md`) |

`handlers` is the only module depending on `axum`; `repository` and
`deletion` are the only modules depending on `sqlx`. `auth` depends on
neither — same isolation discipline `design-storage`'s `rate_limit`/
`sweep` modules established.

## LC-2 — Dependencies on other Units' crates

`cityloom-accounts-sharing` depends on:
- `cityloom-api-types` (U9's shared wire-types crate, extended with this
  Unit's request/response shapes).
- `cityloom-design-storage` (U10) — `SharingService`'s in-process read of
  `DesignRepository` (`components.md`'s already-declared dependent edge;
  `functional-spec.md` workflow step 5, `security-design.md` SD-5). This
  edge is already declared in `unit-of-work-dependency.md`
  (`accounts-sharing: depends_on: [design-storage]`), confirmed at
  `functional-design`'s iteration-2 review.

It depends on nothing from `street-import`, `design-editing`, or any
client-side crate — server-only, no reason to link WASM-target code.

## LC-3 — Composition root: the router merge point

Same pattern as `design-storage/logical-components.md` LC-3: this Unit
exposes exactly one public constructor from its `lib.rs`, taking a shared
`PgPool` (and, per LC-2, a handle to `cityloom-design-storage`'s
`DesignRepository`) and returning an `axum::Router`. The binary that owns
process start constructs both the pool and the `DesignRepository` handle
once and `.merge()`s this Unit's router with the others' — no other
component in this crate constructs a `PgPool` or reaches into U10's crate
directly outside the one call site `handlers` makes through the passed-in
handle.

## LC-4 — What this Unit does not own

Per `components.md`'s ownership boundaries: design storage/authorization-
by-identifier mechanics (U10), erasure/export policy and the *decision* of
when to delete an account (U12, `data-rights`) — this Unit exposes the
deletion *capability* (RD-2) but never decides to invoke it itself — and
the `DesignPayload` shape (U3) are all out of scope.

## Traceability

Not applicable — architectural inventory; see the other five `nfr-design`
files' `traceability.json` entries for the numbered requirements this
component structure satisfies.

_Confirmed (consolidated summary confirmed 2026-09-20)._
