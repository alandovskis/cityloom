# Business Rules — `accounts-sharing` (U11)

_Confirmed (consolidated summary confirmed 2026-09-20)._

Upstream inputs: `entities.md` (this stage), `components.md`,
`contract-summary.md` Contract 4/5, `stories.md` US9.1-US9.3,
US10.1-US10.3, `project.md` (access gate mandate).

```yaml
rules:
  # BR1 — Account creation and cross-device continuity
  - id: BR1.1
    statement: "Completing sign-up immediately establishes a signed-in session; signing in on any device with valid credentials reaches the same account's saved designs."
    category: authorization
    applies_to: Account, Session
    trigger: "Sign-up completion, or sign-in on any device"
    logic: "IF sign-up completes THEN a Session is created for the new Account; IF sign-in succeeds on device X THEN the returned Session resolves to the same Account regardless of which device created it."
    violation_behaviour: "AC9.1.1, AC9.1.2. A sign-up that does not leave the caller signed in, or a second device that reaches a different account's designs, breaks the single-identity premise FR7.1 states."
    source: "AC9.1.1; AC9.1.2; FR7.1"

  # BR2 — Server-side access gate
  - id: BR2.1
    statement: "The whole accounts and sharing surface is unreachable by the public while the access-gate configuration flag is off, enforced in this Unit's own handlers, never by omitting a client-side link to it."
    category: authorization
    applies_to: Account, Session, SavedDesign, Grant, ShareLink
    trigger: "Any request to an accounts/sharing endpoint"
    logic: "IF the access-gate flag reads off THEN every endpoint in this Unit refuses the request server-side, independent of whether the caller knows the URL or the client UI exposes it."
    violation_behaviour: "AC9.1.3; project.md's Mandated access-gate rule. A gate enforced only by hiding a UI element is not enforced at all — it fails the moment someone calls the endpoint directly, which is exactly the failure this rule and project.md's mandate both name."
    source: "AC9.1.3; project.md Mandated"

  # BR3 — Sign-in invitation scope
  - id: BR3.1
    statement: "This Unit requires an account only for the three account-gated actions (naming/saving under an account, sharing, reaching a design from another device) and never rejects an anonymous edit or anonymous local save."
    category: constraint
    applies_to: Account
    trigger: "Any request this Unit receives"
    logic: "IF a request is for save-with-name, sharing, or cross-device reopen THEN it requires a valid Session; IF a request is any other design operation (e.g. an edit, which this Unit never even receives, per its own boundary) THEN this Unit imposes no account requirement."
    violation_behaviour: "AC9.1.4. This Unit's contribution to AC9.1.4 is the endpoint-level requirement itself; the client's one-per-session prompt behaviour is client-surfaces/U6's concern (functional-design-questions.md Q2) and is out of this Unit's scope to re-specify."
    source: "AC9.1.4"

  # BR4 — Device-design migration
  - id: BR4.1
    statement: "Migrating a device design to a new account preserves lane order, lane types, widths, and every provenance state exactly as they were on the device."
    category: constraint
    applies_to: SavedDesign
    trigger: "Sign-up with a device design present"
    logic: "IF a device design accompanies sign-up THEN the migrated SavedDesign's payload is byte-identical to the device copy in every field AC9.2.2 names — this Unit performs no transformation of the payload itself, only attachment to the new Account."
    violation_behaviour: "AC9.2.1, AC9.2.2. A migration that drops or alters a provenance state converts a measured value into an unlabelled one somewhere along the way, the exact failure class team.md's provenance-typing rule exists to prevent."
    source: "AC9.2.1; AC9.2.2; FR7.2"

  - id: BR4.2
    statement: "A failed migration leaves the device copy untouched and reports the failure; it never leaves the design in a half-migrated or lost state."
    category: constraint
    applies_to: SavedDesign
    trigger: "Migration failure at any step"
    logic: "IF the migration attempt does not complete successfully THEN no partial SavedDesign row is left committed, the device copy is not deleted or marked migrated, and the caller receives an explicit failure response naming that the design was not moved."
    violation_behaviour: "AC9.2.3. Silently losing the one design that convinced the user to sign up is worse than the migration never having been offered."
    source: "AC9.2.3"

  # BR5 — Save, name, reopen
  - id: BR5.1
    statement: "Saving a design under a name makes it appear in the signed-in account's design list under that exact name."
    category: constraint
    applies_to: SavedDesign
    trigger: "Save-with-name request from a signed-in session"
    logic: "IF a signed-in caller saves a design with name N THEN a SavedDesign with name = N is created and appears in that account's list."
    violation_behaviour: "AC9.3.1. A saved design missing from its own owner's list defeats the entire point of naming it."
    source: "AC9.3.1; FR7.3"

  - id: BR5.2
    statement: "Saving under a name that duplicates one already used by the same account is refused with an explicit message, never a silent overwrite of the other design."
    category: validation
    applies_to: SavedDesign
    trigger: "Save-with-name request where name already exists for this account"
    logic: "IF SavedDesign.name already exists for this account_id THEN the request is refused with a duplicate-name response; the existing SavedDesign is left unmodified."
    violation_behaviour: "AC9.3.3. Silently overwriting a same-named design under a different one destroys work with no warning — a far worse failure than asking the user to pick a different name."
    source: "AC9.3.3"

  - id: BR5.3
    statement: "Reopening a saved design returns lane order, types, widths and every provenance state exactly as they were when saved."
    category: constraint
    applies_to: SavedDesign
    trigger: "Reopen request for a SavedDesign"
    logic: "IF a SavedDesign is reopened THEN the returned payload is identical to what was saved, unchanged by this Unit in transit."
    violation_behaviour: "AC9.3.2. Same provenance-fidelity failure class as BR4.1, at the reopen path instead of the migration path."
    source: "AC9.3.2"

  - id: BR5.4
    statement: "An account with no saved designs sees an explanatory empty state offering the route to start a design, never a bare empty list."
    category: constraint
    applies_to: SavedDesign
    trigger: "List request for an account with zero SavedDesign rows"
    logic: "IF the SavedDesign count for this account is zero THEN the list response indicates the empty-state condition explicitly rather than returning an ambiguous empty array with no further signal."
    violation_behaviour: "AC9.3.4. This Unit's obligation is to make the empty condition legible to the client, not to render the empty-state UI itself (that is client-surfaces/U6's job)."
    source: "AC9.3.4"

  # BR6 — Private by default
  - id: BR6.1
    statement: "A newly saved design has no Grant and no enabled ShareLink; sharing is never on by default."
    category: policy
    applies_to: SavedDesign, Grant, ShareLink
    trigger: "SavedDesign creation"
    logic: "IF a SavedDesign is created THEN no Grant row references its stored_design_id and any ShareLink row that exists for it has enabled = false."
    violation_behaviour: "AC10.1.1; NFR6.2. An unfinished proposal that is discoverable the moment it is saved is exactly the harm US10.1's private-by-default promise exists to prevent — Policy-priority per stories.md's MoSCoW table, since NFR6.2 is stated as a commitment."
    source: "AC10.1.1; NFR6.2; FR8.1"

  # BR7 — Named grants
  - id: BR7.1
    statement: "A named grant can only be issued to an account holder; there is no email-only or bare-identifier invite path."
    category: authorization
    applies_to: Grant
    trigger: "Grant-issuance request naming a recipient"
    logic: "IF a grant is issued THEN grantee_account_id resolves to an existing Account (via this Unit's own AccountService); a recipient with no account cannot be granted access by name."
    violation_behaviour: "components.md SharingService behaviour statement; supports AC10.2.1's 'named access' semantics being genuine authorization rather than a shared secret."
    source: "components.md SharingService; AC10.2.1; FR8.2"

  - id: BR7.2
    statement: "Granting named access and enabling/disabling a link are independent operations in both directions: neither changes the other's state."
    category: constraint
    applies_to: Grant, ShareLink
    trigger: "Any grant-issuance or link-enable/disable request"
    logic: "IF a link is enabled THEN existing Grants are unchanged; IF a link is disabled THEN existing Grants remain; IF a Grant is issued or revoked THEN ShareLink.enabled is unchanged in either direction."
    violation_behaviour: "AC10.2.1, AC10.2.2, AC10.2.3. Collapsing the two into one 'visibility' setting is the exact failure contract-summary.md Contract 4 calls out as why there is no single visibility field."
    source: "AC10.2.1; AC10.2.2; AC10.2.3; contract-summary.md Contract 4; FR8.2"

  - id: BR7.3
    statement: "Disabling a link stops it working immediately; no grace window."
    category: constraint
    applies_to: ShareLink
    trigger: "Link-disable request"
    logic: "IF ShareLink.enabled transitions to false THEN a subsequent request presenting that token is refused starting with the very next request."
    violation_behaviour: "AC10.2.4. A link that still works for some interval after being disabled means 'disable' does not mean what the person who clicked it thought it meant."
    source: "AC10.2.4"

  # BR8 — Authorization decision AND enforcement (both this Unit's job)
  - id: BR8.1
    statement: "Whether a given account or link token is authorized to read a shared design is this Unit's decision, and this Unit is also the one that enforces it on the read itself — it is not delegated to design-storage's (U10) own public upload/fetch endpoint, which is keyed on anonymousDesignId only and has no grant/account concept (design-storage/functional-design/entities.md's own Assumptions section states the account/grant check is a Stage 2 extension point U10 does not implement)."
    category: authorization
    applies_to: Grant, ShareLink
    trigger: "A shared-design read request (Contract 5) accompanied by an account session or a link token"
    logic: "IF the caller's account has a matching Grant OR presents a token matching an enabled ShareLink THEN this Unit's SharingService reads the payload directly from DesignRepository (U10) — an in-process call within the single shared Railway service, per components.md's own dependency edge (DesignRepository dependents: SharingService, 'Read and authorise the design being shared') — and returns it; OTHERWISE this Unit itself responds not-found/forbidden and never reads or returns the payload. U10's own `/api/designs/{anonymousDesignId}` endpoint (Contract 2) is a separate, narrower path this workflow does not use."
    violation_behaviour: "AC10.1.2, AC10.1.3. Treating U10's identifier-only endpoint as if it already enforced grant/account authorization — as an earlier draft of this rule did — would ship a decision with no corresponding enforcement anywhere in the system, exactly the object-level authorization gap this rule exists to close."
    source: "AC10.1.2; AC10.1.3; contract-summary.md Contract 5 (U10 / U11 joint ownership); components.md DesignRepository dependents (SharingService)"

  # BR9 — Sharing state visibility
  - id: BR9.1
    statement: "Every design list response includes each design's current sharing state, and that state matches the design's actual Grant/ShareLink rows at the moment of the request."
    category: constraint
    applies_to: SavedDesign, Grant, ShareLink
    trigger: "Design-list request"
    logic: "IF a list of an account's SavedDesigns is returned THEN each entry carries a sharing-state field reflecting its current Grant count and ShareLink.enabled value, computed at request time rather than cached from creation."
    violation_behaviour: "AC10.3.1, AC10.3.2. A stale or absent sharing indicator forces the person back into each design just to find out whether it is public — the exact friction US10.3 exists to remove."
    source: "AC10.3.1; AC10.3.2; FR8.3"

  - id: BR9.2
    statement: "Sharing state is exposed as a named, announceable field, never as an icon-only signal with no accessible name."
    category: constraint
    applies_to: SavedDesign
    trigger: "Design-list response shape"
    logic: "IF the sharing-state field is rendered by a client THEN it carries a value a screen reader can announce (e.g. 'private' / 'shared with N people' / 'link enabled'), not a bare boolean or icon code with no textual equivalent."
    violation_behaviour: "AC10.3.3. This is an API-shape obligation (a named, human-readable value) that makes the accessible announcement possible; the announcement itself is client-surfaces/U6's rendering responsibility."
    source: "AC10.3.3"
```

## Summary

| Group | Rules | Covers |
|---|---|---|
| BR1 | Account creation, cross-device continuity | US9.1 |
| BR2 | Server-side access gate | US9.1 (AC9.1.3), project.md |
| BR3 | Sign-in invitation scope | US9.1 (AC9.1.4) |
| BR4 | Device-design migration fidelity and failure handling | US9.2 |
| BR5 | Save/name/duplicate-name/reopen/empty-state | US9.3 |
| BR6 | Private by default | US10.1 |
| BR7 | Named grants and link independence | US10.2 |
| BR8 | Authorization decision (enforcement is U10's) | US10.1 (AC10.1.2/.3) |
| BR9 | Sharing-state visibility, including accessibility | US10.3 |
