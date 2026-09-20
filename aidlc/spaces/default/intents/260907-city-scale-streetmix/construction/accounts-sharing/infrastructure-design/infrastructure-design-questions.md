# Infrastructure Design Questions — `accounts-sharing` (U11)

## Q1: Deployment target — new service or the existing shared one?

`unit-of-work.md`'s "four Units, one deployable" (U9/U10/U11/U12 on one
Railway service, `logical-components.md` LC-3's composition root) already
fixes this. This Unit's router merges into the same `cityloom` service
`design-storage` (U10) already extended from `osm-extract-proxy` (U9) — no
new compute service.

[Answer]: The existing shared Railway service `cityloom`. No new compute
resource.

**Mode:** self-answered (autonomous, user-authorized 2026-09-20)

## Q2: Database — new Postgres plugin, or reuse design-storage's?

`scalability-design.md` SC-3 already fixed this at `nfr-design`: this Unit
does not construct its own `PgPool` — the composition root passes in the
one shared pool. `reliability-design.md` RD-1 also assumes "the shared
Postgres database."

[Answer]: Reuse `design-storage`'s existing Railway-managed PostgreSQL
plugin (provisioned at U10's `environment-provisioning`) — one shared
database, this Unit's own tables (`account`, `session`, `access_grant`,
`share_link`, `saved_design`) added via this Unit's own migration files.
(`access_grant`, not `grant` — `grant` is a reserved PostgreSQL keyword.)
No new Railway resource. This also keeps the account-deletion cascade
(RD-2) a single-database transaction — splitting into a second database
would break RD-2's one-transaction atomicity guarantee entirely.

**Mode:** self-answered (autonomous, user-authorized 2026-09-20)

## Q3: Migration ordering — does this Unit's migration depend on design-storage's?

`reliability-design.md` RD-2's cascade transaction references
`saved_design.stored_design_id`, which is an opaque value only — no
foreign key into `design-storage`'s `stored_designs` table
(`entities.md`'s no-cross-crate-FK convention, carried from
`functional-design`). So no migration ordering dependency exists: this
Unit's migration can run before, after, or interleaved with any future
`design-storage` migration, since neither references the other's schema
objects.

[Answer]: No ordering dependency. This Unit's migration file is
independent of `design-storage`'s migration history.

**Mode:** self-answered (autonomous, user-authorized 2026-09-20)

## Q4: CI — new workflow, or extend the existing two-tier pipeline?

`team.md` already fixed the two-tier CI shape workspace-wide;
`design-storage`'s `cicd-pipeline.md` already extended it once for a
Postgres-backed crate.

[Answer]: Extend the existing fast/slow tiers exactly as `design-storage`
did — this Unit's crate (`cityloom-accounts-sharing`) joins the fast
tier's build/clippy/fmt/test/coverage matrix and reuses the same Postgres
service container `design-storage` already added (one container, now
serving two crates' test suites — no second container).

**Mode:** self-answered (autonomous, user-authorized 2026-09-20)

## Q5: Secrets — any new secret beyond `DATABASE_URL`?

`security-design.md` SD-1/SD-8 named the access-gate flag and the
password-hashing scheme.

[Answer]: One new non-secret configuration value: the access-gate flag
(`ACCOUNTS_SHARING_ENABLED` or equivalent, Railway service variable,
**absent = off** per SD-1's fail-closed design — never a secret, since its
value is not sensitive, only its *absence* is meaningful). No new secret:
`DATABASE_URL` is already injected and shared; Argon2 hashing needs no
external key material (unlike, say, JWT signing, which this Unit's
DB-backed session design deliberately avoids per `nfr-requirements`
Q2/`security-design.md` SD-3).

**Mode:** self-answered (autonomous, user-authorized 2026-09-20)

## Consolidated Summary Confirmation

- Looks correct
- Request changes

[Answer]: Looks correct
