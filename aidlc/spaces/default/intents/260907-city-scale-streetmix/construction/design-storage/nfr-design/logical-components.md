# Logical Components — `design-storage` (U10)

Upstream inputs: `components.md` `DesignRepository` (domain-design),
`tech-stack-decisions.md` (nfr-requirements, this Unit), all five other
`nfr-design` artifacts in this directory, `team.md`'s "port and
composition root are one fix, not two" corollary (learned at
domain-design), `osm-extract-proxy`'s own logical-components precedent.

Design elements are numbered `LC-n`.

## LC-1 — What is inside `cityloom-design-storage`

`components.md` names one component, `DesignRepository`, for this Unit.
Inside the crate:

| Module | Responsibility |
|---|---|
| `handlers` | The three axum route handlers (upload/fetch/remove) — orchestrates validation, rate-limit check, authorization check, and the single query per `performance-design.md` PD-1; contains no SQL itself |
| `repository` | The `sqlx` query functions (one per operation) and the `sqlx::Error` → `StorageFailure` mapping (`security-design.md` SD-6) |
| `rate_limit` | The identifier-keyed window map (`security-design.md` SD-2) |
| `sweep` | The batched expiry-sweep task (`performance-design.md` PD-3, `reliability-design.md` RD-3) |
| `failure` | The `StorageFailure` type and its `thiserror` impl (`entities.md`) |
| `observability` | The counter and log-row emission points (`observability-design.md`) |

`handlers` is the only module that depends on `axum`; `repository` is
the only module that depends on `sqlx`. Neither `rate_limit` nor
`sweep` depends on `axum` — both are plain Rust state/logic the handlers
and the composition root drive, which keeps them unit-testable without
spinning up an HTTP server (`team.md` Testing Posture's TDD ordering
applies most directly to these two modules, since they carry the actual
business logic BR2.1 and BR4.1 name).

## LC-2 — Dependencies on other Units' crates

`cityloom-design-storage` depends on:
- `cityloom-api-types` (U9's shared wire-types crate, extended with this
  Unit's own request/response shapes, `tech-stack-decisions.md`).
- `cityloom-design-payload` (U3) for `DesignPayload::from_json`'s
  version check (`security-design.md` SD-3) — a normal crate dependency,
  not a re-implementation.

It depends on nothing from `street-import`, `design-editing`, or any
client-side crate — this Unit is server-only and has no reason to link
against WASM-target code.

## LC-3 — Composition root: the router merge point

Per `team.md`'s corollary (a port narrows what crosses a boundary, but
something still has to construct the implementor, and that construction
belongs at a composition root with no other behaviour), this Unit
exposes exactly one public function from its `lib.rs`: a constructor
taking a `PgPool` and returning an `axum::Router` with all three routes
mounted plus the sweep task's `JoinHandle` spawned. The binary that owns
process start (`tech-stack-decisions.md`'s open assumption: whether that
binary is `osm-extract-proxy`'s own or a new shared one is an
`infrastructure-design`/`environment-provisioning` decision) is the one
place that constructs the `PgPool` and calls this function, `.merge()`ing
the returned router with U9's own — no other component in this Unit's
own crate constructs a `PgPool` or reaches across to U9's crate directly.

## LC-4 — What this Unit does not own

Per `components.md`'s ownership boundaries: accounts and grants
(`SharingService`/`AccountService`, U11), erasure
(`u12-data-rights`, U12), and the `DesignPayload` shape itself (U3) are
all out of this Unit's scope — `cityloom-design-storage` stores and
authorizes-by-identifier, and calls into U3's existing validation; it
never re-implements a capability another Unit already owns.

## Traceability

Not applicable — this artifact is architectural inventory, not a set of
verifiable NFR targets; see the other five `nfr-design` files' own
`traceability.json` entries for the numbered requirements this
component structure exists to satisfy.

_Confirmed._ (nfr-design)
