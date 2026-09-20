# NFR Requirements Questions — `accounts-sharing` (U11)

**Mode:** self-answered (autonomous, user-authorized 2026-09-20)

## Q1 — Performance targets

What are this Unit's latency targets?

[Answer]: Sign-up/sign-in p95 under 300ms (an interactive-feeling
credential check against a small `Account` table); the design list with
computed sharing state (US10.3) p95 under 400ms, since it aggregates
`Grant`/`ShareLink` state per row rather than a flat read; grant/link
create or revoke p95 under 300ms. All three are ordinary single-instance
Postgres query budgets, one order of magnitude looser than `design-storage`'s
own targets are not needed here since this Unit's tables are small
(accounts, not designs) — grounded in `requirements.md` NFR1 and the same
proportionate-to-workload reasoning `design-storage/performance-requirements.md`
used.

## Q2 — Security / threat model

What does this Unit protect, and what's the credential/session mechanism?

[Answer]: Protects: account credentials, session tokens, and the
grant/link authorization decision gating access to someone else's design
(`requirements.md` NFR6.2, this Unit's own confirmed requirement per
`project.md`'s access-gate mandate). Credentials: a password, hashed at
rest (never compared or stored in plaintext) — `tech-stack-decisions.md`
below picks the specific hashing algorithm. Sessions are server-side
(`Session` rows in Postgres, matching `entities.md`), not a
self-contained JWT — this allows immediate revocation (sign-out, or a
future admin/erasure action) by deleting the row, which a stateless
signed token cannot offer without a separate revocation list. The
session's own token value is stored hashed in the `Session` table
(BR-equivalent to how `design-storage` never stores a raw secret
comparably), so a database read alone does not yield a usable session
credential. `Contract 4`'s stated cookie flags (`HttpOnly`, `Secure`,
`SameSite`) are a hard requirement, not a suggestion.

## Q3 — Scalability

What's the expected load and how does this Unit share the single Railway
instance/database with `design-storage`?

[Answer]: Same single-instance, ~$5/month budget constraint as every
other server Unit (`project.md` OC-4, `team.md` Deployment). This Unit's
own tables (`Account`, `Session`, `SavedDesign`, `Grant`, `ShareLink`)
live in the same Postgres database as `design-storage`'s
`stored_designs` table, in their own migration-created tables — one
database, one connection pool shared across both crates' `sqlx::PgPool`
once merged at the composition root (`infrastructure-design`'s decision,
not this stage's). Session lookups are the highest-frequency query this
Unit runs (once per authenticated request) and must stay indexed
(`Session.token_hash` unique index) so they don't degrade as the account
count grows.

## Q4 — Reliability

What happens on a deploy/restart, and how does this Unit participate in
account erasure (`data-rights`/U12)?

[Answer]: Sessions are DB-backed, so a Railway redeploy (which restarts
the process) does not sign anyone out — a purely in-memory session store
would fail this the moment the org's own deploy-on-merge practice
(`team.md` Deployment) runs. For erasure: `project.md` Mandated makes
data subject rights a functional requirement gating Stage 2's public
release, but the erasure *trigger and policy* belong to `data-rights`/U12
(`contract-summary.md` Contract 6/7) — this Unit's own obligation is
narrower: deleting an `Account` must cascade to its `Session`,
`SavedDesign`, and `Grant` rows (and `ShareLink` rows for designs it
owned) without leaving orphans, and either complete or leave the account
intact, matching Contract 6's stated all-or-nothing erasure behavior
(AC11.1.4) — this Unit exposes that cascade capability; it does not
decide when to invoke it.

## Q5 — Observability

What does an operator need to see?

[Answer]: Aggregate counters (no per-request/per-account row, matching
`design-storage`'s established stdout-JSON-rows precedent and
`osm-extract-proxy`'s original pattern): sign-up/sign-in outcomes
(success/failure by reason), grant/link create and revoke counts, and a
count of requests refused because the access-gate flag is off — the last
one lets an operator confirm the gate is actually being hit as off
(not merely configured off) before Stage 2 goes live, which is exactly
the check `project.md`'s access-gate mandate needs to be verifiable
rather than asserted. Plus a readiness signal distinguishing "database
reachable" from "process running," same shape as `design-storage`'s
NFR7.4.4.

## Q6 — Tech stack

Same stack as `design-storage`, or different?

[Answer]: Same crate-level stack (`axum` `=0.8.9`, `tokio` `=1.53.1`,
`sqlx` postgres/runtime-tokio/tls-rustls, `thiserror`, `tracing` +
`tracing-subscriber`, all pinned to the versions `design-storage`
already established) — this Unit is the same kind of server crate in the
same shared Railway service, and there is no reason to introduce a
second set of pins. New dependency this Unit needs that `design-storage`
does not: a password-hashing crate. See `tech-stack-decisions.md` for
the specific choice and rationale.

## Consolidated Summary Confirmation

- Looks correct
- Request changes

[Answer]: Looks correct
