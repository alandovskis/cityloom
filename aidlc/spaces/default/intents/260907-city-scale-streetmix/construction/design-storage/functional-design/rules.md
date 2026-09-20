# Business Rules — `design-storage` (U10)

_Confirmed._

Upstream inputs: `entities.md` (this stage), `components.md`,
`contract-summary.md` Contract 2, `stories.md` US8.3, US11.3, US10.1.

```yaml
rules:
  # BR1 — Explicit upload only, nothing implicit
  - id: BR1.1
    statement: "A design is stored only when the client explicitly uploads it (Contract 2's uploadDesign); this Unit never receives or stores a design the user did not ask to leave the device."
    category: constraint
    applies_to: StoredDesign
    trigger: "POST /api/designs"
    logic: "IF a design is stored THEN it arrived via an explicit uploadDesign call; there is no code path in this Unit that stores a design without one."
    violation_behaviour: "AC8.3.2 — nothing about a design leaves the device unless the user explicitly asks. This Unit's obligation is simply to never invent a second, implicit intake path."
    source: "AC8.3.2; Contract 2"

  # BR2 — Retention differs by ownership
  - id: BR2.1
    statement: "An anonymous uploaded design expires 30 days after creation; an account-owned design is retained until the account is deleted; a design held only on a device receives no action from this Unit, because it was never received."
    category: policy
    applies_to: StoredDesign
    trigger: "Time passing after upload, or an expiry sweep running"
    logic: "IF stored_design.anonymous_design_id is set THEN expires_at = created_at + 30 days, and the row is removed once expires_at passes; IF stored_design.owner_account_id is set THEN no automatic expiry applies — retention ends only when the account is deleted (a Stage 2 / u12-data-rights operation); a design never uploaded has no StoredDesign row and this Unit takes no action regarding it."
    violation_behaviour: "AC11.3.1, AC11.3.2, AC11.3.3. Letting an anonymous upload persist indefinitely accumulates content nobody can claim or delete — exactly the harm US11.3 names; conversely, expiring an account-owned design would violate the different promise Stage 2 makes."
    source: "AC11.3.1; AC11.3.2; AC11.3.3; FR9.3, FR9.4"

  # BR3 — Object-level authorization, never the design on refusal
  - id: BR3.1
    statement: "A request for a design the requester has not been granted access to receives a not-found or forbidden response and never the design itself — asserted as a response, not a UI-layer decision."
    category: validation
    applies_to: StoredDesign
    trigger: "GET /api/designs/{anonymousDesignId}"
    logic: "IF the requested identifier does not resolve to a stored row, OR the requester has not been granted access THEN respond 404 (design_not_found) or 403 (not_granted) and the response body never contains the payload; the check happens in this Unit's own handler, not left to a caller to enforce."
    violation_behaviour: "AC10.1.2, AC10.1.3. An unauthenticated or ungranted request that returns the design in any form — even behind a client-side check — is exactly the object-level authorization failure this rule exists to prevent."
    source: "AC10.1.2; AC10.1.3; Contract 2 fetchDesign"

  # BR4 — Rate limiting by identifier
  - id: BR4.1
    statement: "Access attempts against a given anonymousDesignId are rate-limited, because the identifier is the only protection an anonymous design has."
    category: constraint
    applies_to: StoredDesign
    trigger: "Repeated GET/DELETE requests against the same or guessed identifiers"
    logic: "IF requests against an identifier (or from a source attempting many identifiers) exceed a stated threshold THEN further requests receive 429 (rate_limited) until the window resets."
    violation_behaviour: "Without this, a 128-bit random identifier's security rests entirely on an attacker not being willing to guess — rate limiting is what makes guessing actually impractical rather than merely inconvenient."
    source: "Contract 2 (rate limiting note); components.md DesignRepository"

  # BR5 — Removal requires no account
  - id: BR5.1
    statement: "Removing an uploaded design never requires an account, because the upload that created it required none."
    category: policy
    applies_to: StoredDesign
    trigger: "DELETE /api/designs/{anonymousDesignId}"
    logic: "IF a DELETE request carries a valid anonymousDesignId THEN the row is removed (or the response is 204 if already absent) — no authentication header or account context is required or checked beyond the identifier itself."
    violation_behaviour: "AC8.3.4. Requiring an account for removal would contradict the same anonymous, no-account-required design the upload path already commits to."
    source: "AC8.3.4; Contract 2 removeDesign"

  # BR6 — Failed upload leaves the device copy intact
  - id: BR6.1
    statement: "A failed upload never partially stores a design; the client's device copy is unaffected, and the failure is a typed, closed reason — never a raw dependency or database error."
    category: constraint
    applies_to: StorageFailure
    trigger: "An upload request that cannot be completed"
    logic: "IF an upload fails for any reason THEN this Unit either commits the full StoredDesign row or none of it (no partial write), and returns one of the closed StorageFailure reasons; it never lets a database-native error type reach the response."
    violation_behaviour: "AC8.3.3 places the recovery obligation on the client (device copy retained, user told to retry), which only holds if this Unit's own failure is atomic and cleanly typed — a partial write or a leaked internal error would corrupt that contract."
    source: "AC8.3.3; team.md Code Style (typed results, not exceptions)"

  # BR7 — Stored identifier carries no personal signal
  - id: BR7.1
    statement: "An anonymous upload is stored against an identifier that carries no name, no contact detail, and no cross-site linkage — it is opaque and minted by the client, not derived from anything about the user or device."
    category: constraint
    applies_to: StoredDesign
    trigger: "Storing an anonymous upload"
    logic: "IF a StoredDesign is created for an anonymous upload THEN its anonymous_design_id is exactly the client-supplied value from Contract 2's UploadRequest (never re-derived, hashed from, or supplemented with any device/request signal); this Unit stores no IP address, user agent, or other requester-identifying field alongside the row."
    violation_behaviour: "AC8.3.5. Storing even one additional identifying field alongside an otherwise-anonymous identifier would defeat the whole point of the anonymous path."
    source: "AC8.3.5; decisions.md ADR-006"
```

## Summary

| Rule | Category | What it protects |
|---|---|---|
| BR1.1 | Explicit upload only | Nothing leaves the device without being asked |
| BR2.1 | Retention differs by ownership | Anonymous content doesn't accumulate forever; account content isn't lost early |
| BR3.1 | Object-level authorization | A private design is never returned to the wrong requester |
| BR4.1 | Rate limiting | An anonymous identifier can't be found by guessing |
| BR5.1 | Removal requires no account | The no-account promise holds for deletion too |
| BR6.1 | Atomic, typed upload failure | The client's recovery story (retry, no data loss) actually holds |
| BR7.1 | No personal signal on the identifier | The anonymous path stays genuinely anonymous |

## Traceability

See `traceability.json` in this directory.
