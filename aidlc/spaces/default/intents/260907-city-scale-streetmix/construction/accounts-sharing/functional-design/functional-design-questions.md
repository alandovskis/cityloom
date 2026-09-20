# Functional Design Questions — `accounts-sharing` (U11)

Upstream inputs: `unit-of-work.md` U11, `unit-of-work-story-map.md` (US9.1-9.3,
US10.1-10.3), `requirements.md` FR7/FR8/NFR6.2, `components.md`
(`AccountService`, `SharingService`), `contract-summary.md` Contract 4/5,
`project.md` (access-gate mandate).

**Mode:** self-answered (autonomous, user-authorized 2026-09-20). The user
gave standing permission to continue Construction for the remaining Units
without a per-stage human gate; every answer below is grounded in an
upstream artifact, cited inline, exactly as the completed Units' own
functional-design stages were.

## Q1 — Where does a saved design's name live?

`components.md`'s `AccountService` entity list (Account, Session) has no
entity for a name↔design association, and `design-storage`'s already-built
`StoredDesign` (U10, already code-generated — not being reopened) has no
`name` field. But US9.3 ("save a design with a name") is this Unit's story,
and `AccountService.depends_on` already declares the exact interaction:
"Attach a migrated or saved design to an account."

[Answer]: Introduce a new entity, `SavedDesign`, owned by `AccountService`,
holding `accountId` + `storedDesignId` (an opaque reference to U10's
`StoredDesign`, by ID only — no cross-crate foreign key, matching the
precedent U10 itself already set by storing `owner_account_id` as an opaque
string with no FK back to `Account`) + `name` + `savedAt`. Uniqueness for
AC9.3.3's duplicate-name check is scoped per-account (a name is only
required to be unique within one person's own list — nothing in US9.3
requires global uniqueness, and requiring it would leak information about
other users' design names).

## Q2 — What triggers the sign-in invitation, and what happens on decline?

AC9.1.4 is explicit: triggered only by an action that requires an account
(saving under a name, sharing, or reaching a design from another device),
never by an edit, and a decline must not be asked again in the same session.

[Answer]: The invitation is a client-surface concern (`AppShell` per
`components.md`'s `dependents`) that this Unit's API supports by simply
returning a `401`/gate-style response when an account-requiring endpoint is
called unauthenticated — this Unit does not implement "ask once per
session" itself (that is client-side UI state, `client-surfaces`/U6's
concern), it only guarantees it never *requires* sign-in for anything but
the three named actions.

## Q3 — Does a named-grant recipient need an account?

Explicitly settled already: `components.md` states "A named grant requires
the recipient to hold an account," and `stories.md` US10.2's carried-forward
open question (OQ-US5) about whether *readers* need an account was about
link-based readers, not named-grant recipients — a public link needs no
account (Contract 5, "an unauthenticated reader resolves a share link").

[Answer]: Confirmed as designed: named grants require the grantee to have
an account (`Grant.granteeAccountId` resolves via `AccountService`, per
`components.md`'s `SharingService.depends_on` on `AccountService` to
"resolve the account a named grant belongs to"); a share link requires no
account for the reader.

## Q4 — What does the object-level authorization boundary actually check, given U10 already owns "not-found/forbidden" enforcement?

`design-storage`'s own `rules.md` BR3.1 already enforces "never return the
design to a non-granted requester" as a response-shape rule, and
Contract 5's ownership row says "U11 owns link resolution; U10 owns the
design it returns."

[Answer]: This Unit's rules describe the *authorization decision* (does a
named `Grant` exist for this account, or does a valid enabled `ShareLink`
token accompany the request) as a `SharingService` responsibility;
enforcing the resulting refusal as an HTTP response is U10's already-built
job, referenced here rather than re-specified. This avoids duplicating
BR3.1 under a new ID in this Unit.

## Q5 — Is there a frontend-components.md for this Unit?

[Answer]: No. `unit-of-work.md` lists U11 as `kind: service` (not `ui`), and
`produces_kinds.frontend-components` is scoped to `[ui]` only — the actual
sign-in/sharing screens belong to `client-surfaces`/U6, which consumes this
Unit's API. Skipping this optional artifact.

## Ambiguity / contradiction check

No contradictions found between FR7/FR8, the US9.x/US10.x acceptance
criteria, and `components.md`'s stated component boundaries. The one real
gap (Q1, the missing name-holding entity) is a domain-design omission this
stage is entitled to fill within its own Unit's boundary, not a
contradiction between existing artifacts — flagged above with its
rationale for the reviewer to check.

## Consolidated Summary Confirmation

- Looks correct
- Request changes

[Answer]: Looks correct
