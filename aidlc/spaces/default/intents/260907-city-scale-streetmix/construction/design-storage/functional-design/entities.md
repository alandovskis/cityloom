# Entities — `design-storage` (U10)

_Confirmed._

Upstream inputs: `unit-of-work.md` U10, `components.md` (`DesignRepository`),
`contract-summary.md` Contract 2, `stories.md` US8.3, US11.3, US10.1
(the object-level authorization half only).

This Unit owns stored designs — anonymous uploads now, account-owned
designs from product Stage 2 — their retention, object-level
authorisation on every read, and rate limiting against identifier
guessing. It does not own accounts, grants, or erasure (those are U11,
U11, U12 respectively).

```yaml
entities:
  - name: StoredDesign
    description: >
      A design the service holds, keyed by either an anonymous identifier
      or an account, never both at once in Stage 1 (account-owned storage
      is a Stage 2 capability this shape already accommodates via a
      nullable owner). Retention differs by which key applies (BR2.1).
    attributes:
      - name: stored_design_id
        type: string
        required: true
        constraints: ["server-minted primary key, distinct from the client-minted anonymousDesignId"]
      - name: anonymous_design_id
        type: string
        required: false
        constraints: ["128-bit, hex-encoded, client-minted from a cryptographic source (decisions.md ADR-006); present for an anonymous upload, absent once/if the design becomes account-owned"]
      - name: owner_account_id
        type: string
        required: false
        constraints: ["present only for a Stage-2 account-owned design; this Unit's Stage 1 scope never sets it — reserved so Stage 2 does not require a schema migration"]
      - name: payload
        type: reference
        required: true
        constraints: ["the DesignPayload shape (design-payload-spec, U3, Contract 3) — this Unit stores it opaquely and does not interpret its contents beyond the payloadVersion check Contract 3 already requires of any reader"]
      - name: created_at
        type: string
        required: true
      - name: expires_at
        type: string
        required: false
        constraints: ["present only for an anonymous design: created_at + 30 days (AC11.3.1, BR2.1); absent for an account-owned design, which is retained until the account is deleted (AC11.3.2)"]

  - name: UploadReceipt
    description: >
      What the service returns on a successful upload (Contract 2's
      response shape) — confirms the design now exists off the device
      and states when an anonymous upload will expire.
    attributes:
      - name: anonymous_design_id
        type: string
        required: true
      - name: uploaded_at
        type: string
        required: true
      - name: expires_at
        type: string
        required: true
        constraints: ["30 days after uploaded_at for an anonymous design (AC11.3.1)"]

  - name: StorageFailure
    description: >
      The typed, closed failure surface this Unit's API produces — mirrors
      Contract 2's ApiError/reason enum exactly, reusing the same
      "typed result, not exception" discipline `osm-extract-proxy` (U9)
      already established for this workspace's one other server Unit.
    attributes:
      - name: reason
        type: enum
        required: true
        allowed_values: [design_not_found, not_granted, payload_too_large, unsupported_payload_version, rate_limited, internal]
      - name: detail
        type: string
        required: false
```

## Summary

Three types. `StoredDesign` is this Unit's own record — deliberately
shaped so a Stage 2 account-owned design is the same row type with
`owner_account_id` set and `anonymous_design_id`/`expires_at` cleared,
not a second table. `UploadReceipt` and `StorageFailure` are the two
possible outcomes of a write, mirroring Contract 2's response shapes
exactly.

## Assumptions & Open Questions

- **[assumption]** `payload`'s shape is `design-payload-spec`'s (U3)
  `DesignPayload` — this Unit is one of that spec's three named
  consumers (`contract-summary.md` Contract 3) and stores it as an
  opaque blob (or a typed column, decided at `nfr-requirements`/
  `tech-stack-decisions`), never re-deriving its own copy of the shape.
- **[assumption]** Grant-based authorization (AC10.1.2/AC10.1.3's full
  form, checking a named grant or share link) cannot be built in this
  Unit alone — `SharingService`/`AccountService` (U11) do not exist yet
  in Stage 1. This Unit's Stage 1 scope implements the authorization
  check `DesignRepository` itself owns per `components.md` — refusing
  any request that doesn't carry a valid `anonymousDesignId` matching a
  stored row, treating the identifier itself as the Stage 1 credential
  (per Contract 2's `fetchDesign`) — and rate-limits guessing attempts
  against it. The full grant/account check is a Stage 2 extension point
  this Unit's design leaves room for (`owner_account_id`) but does not
  implement.

## Traceability

See `traceability.json` in this directory.
