# Entities — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._

Upstream inputs: `unit-of-work.md` U11, `components.md` (`AccountService`,
`SharingService`), `contract-summary.md` Contract 4/5, `stories.md`
US9.1-US9.3, US10.1-US10.3.

This Unit owns accounts, sessions, the server-side access gate, migration
of a device design on sign-up, saved-design naming, and the two independent
sharing mechanisms (named grants and a public link). It does not own
storage mechanics (U10 `design-storage`) or erasure/export (U12
`data-rights`).

```yaml
entities:
  - name: Account
    description: >
      A signed-up user (components.md AccountService). The whole accounts
      surface stays behind a server-side access gate until data subject
      rights exist (AC9.1.3, project.md).
    attributes:
      - name: account_id
        type: string
        required: true
        constraints: ["server-minted primary key"]
      - name: email
        type: string
        required: true
        unique: true
      - name: created_at
        type: string
        required: true
      - name: deleted_at
        type: string
        required: false
        constraints: ["set by u12-data-rights on account erasure; this Unit never sets it itself"]

  - name: Session
    description: "An authenticated session for one Account (components.md AccountService)."
    attributes:
      - name: session_id
        type: string
        required: true
        constraints: ["server-minted, carried in an HttpOnly/Secure cookie with a SameSite policy (stories.md 'Verification notes'; contract-summary.md Contract 4)"]
      - name: account_id
        type: string
        required: true
        references: Account
      - name: issued_at
        type: string
        required: true
      - name: expires_at
        type: string
        required: true
    relationships:
      - to: Account
        cardinality: "many Sessions to one Account"

  - name: SavedDesign
    description: >
      [assumption — see functional-design-questions.md Q1] Associates an
      Account with a design it has saved under a name. Not present in
      components.md's original entity list; introduced here because
      AccountService already declares the interaction this entity backs
      ("Attach a migrated or saved design to an account") and US9.3
      requires naming, which design-storage's already-built StoredDesign
      does not carry.
    attributes:
      - name: saved_design_id
        type: string
        required: true
        constraints: ["server-minted primary key"]
      - name: account_id
        type: string
        required: true
        references: Account
      - name: stored_design_id
        type: string
        required: true
        constraints: ["opaque reference to design-storage's (U10) StoredDesign.stored_design_id — a value only, never a cross-crate database foreign key, matching the precedent U10 itself set by storing owner_account_id as an opaque string with no FK back to Account"]
      - name: name
        type: string
        required: true
        constraints: ["unique per account_id (AC9.3.3) — not globally unique; a duplicate within one account's own list is refused rather than silently overwriting"]
      - name: saved_at
        type: string
        required: true
    relationships:
      - to: Account
        cardinality: "many SavedDesigns to one Account"

  - name: Grant
    description: "A named-access authorization to one stored design (components.md SharingService)."
    attributes:
      - name: grant_id
        type: string
        required: true
        constraints: ["server-minted primary key"]
      - name: stored_design_id
        type: string
        required: true
        constraints: ["opaque reference to U10's StoredDesign, same convention as SavedDesign.stored_design_id"]
      - name: grantee_account_id
        type: string
        required: true
        references: Account
        constraints: ["a named grant requires the recipient to hold an account (components.md SharingService) — never a bare identifier or email-only invite"]
      - name: granted_at
        type: string
        required: true
    relationships:
      - to: Account
        cardinality: "many Grants to one grantee Account"

  - name: ShareLink
    description: "A public, revocable link opening one stored design without requiring an account to read it (components.md SharingService; Contract 5)."
    attributes:
      - name: share_link_id
        type: string
        required: true
        constraints: ["server-minted primary key"]
      - name: stored_design_id
        type: string
        required: true
        constraints: ["opaque reference to U10's StoredDesign, same convention as above"]
      - name: enabled
        type: boolean
        required: true
        defaults: false
        constraints: ["a new design has sharing off (AC10.1.1); enabling/disabling is a separate explicit action from Grant, independent in both directions (AC10.2.1-AC10.2.4)"]
      - name: token
        type: string
        required: true
        constraints: ["128-bit, cryptographically random — the only credential an unauthenticated reader presents (Contract 5)"]
      - name: enabled_at
        type: string
        required: false
        constraints: ["present only while enabled; cleared on disable so a stale timestamp cannot be read as 'currently shared'"]
```

## Summary

Five entities across the two components this Unit owns. `Account` and
`Session` are the identity surface. `SavedDesign` is this stage's one
design addition beyond `components.md` — it is where a human-chosen name
lives, since neither `AccountService`'s original entity list nor U10's
already-built `StoredDesign` carries one. `Grant` and `ShareLink` are the
two independent sharing mechanisms US10.2 keeps deliberately separate: a
grant always resolves to an `Account`, a link never does. Every reference
to a design (`SavedDesign`, `Grant`, `ShareLink`'s `stored_design_id`) is an
opaque value, never a foreign key into U10's own tables — this Unit and
`design-storage` are separate crates in the same deployable service, and
neither reaches into the other's schema.
