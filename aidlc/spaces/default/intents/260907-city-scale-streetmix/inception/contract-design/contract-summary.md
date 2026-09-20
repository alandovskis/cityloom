# Contract Summary — Streetmix at City Scale

Upstream inputs: `unit-of-work.md` and `unit-of-work-dependency.md`
(units-generation), `components.md` (domain-design), `requirements.md`
(requirements-analysis), `contract-design-questions.md` (this stage).

A contract here is a formal agreement across a boundary: what data crosses it,
in what shape, over what protocol, and what happens when it goes wrong. This
file maps every boundary in the system at once and carries each spec inline.

## How these contracts are expressed

**Both ends of every HTTP boundary are Rust.** The client compiles to
WebAssembly from Rust; the server is Rust. So the request and response types
are **one shared crate** that both ends depend on and serialise with `serde`,
rather than two definitions kept in sync by hand (Q2). A drifted contract is a
compile error, not a runtime surprise — the same principle `team.md` applies to
the client's own layering, where boundaries are enforced "by crate boundaries
and module visibility, not by a linter plugin, so a boundary violation fails
the build itself rather than a review that has no reviewer."

The OpenAPI blocks below are **generated from those types, not hand-authored**.
They are in this document so a human can read the boundary; the crate is the
source of truth.

Because they are generated per crate, a reference that crosses a crate boundary
is written as a cross-file one — Contract 2 refers to
`./contract-3-design-payload.yaml#/DesignPayload` because the payload types
live in their own crate (`cityloom-design-payload`) and a generator run would
emit them to their own file. **In this document that reference resolves to
Contract 3's `DesignPayload` block below**, which is the same shape written
inline; the `.yaml` file it names does not exist here and is not expected to.

### The condition this rests on

Every consumer of every boundary here is this project's own WASM client.
Someone opening a shared design link (boundary 5) is not a third-party
programmatic caller — they load the same application in a browser, which then
fetches the design. That is what makes a shared Rust crate and a build-stamp
check safe rather than reckless.

**If a shared design is ever exposed as a programmatic API for outside
callers, both of those decisions break and this contract must be reopened.**
A non-Rust consumer cannot depend on the crate and cannot send a build stamp.
Recorded here rather than discovered later.

## Contracts

| # | Provider Unit | Consumer | Mechanism | Owner |
|---|---|---|---|---|
| 1 | U9 `osm-extract-proxy` | U4 `street-import` | Synchronous HTTP, JSON | U9 |
| 2 | U10 `design-storage` | U7 `local-persistence` | Synchronous HTTP, JSON | U10 |
| 3 | U3 `design-payload-spec` | U7, U10, U12 | Shared schema (Rust types + `serde`) | U3 |
| 4 | U11 `accounts-sharing` | U6 `client-surfaces` | Synchronous HTTP, JSON — product Stage 2 | U11 |
| 5 | U10 / U11 | `External: public web` | Synchronous HTTP, JSON — product Stage 2 | U11 |
| 6 | U12 `data-rights` | U6 `client-surfaces` | Synchronous HTTP, JSON — product Stage 2 | U12 |
| 7 | U12 `data-rights` | U10, U11 | In-process Rust — no separate spec | U12 |
| 8 | U2 `street-core` `StreetSource` port | U4 `street-import` | Rust trait — no separate spec | U2 |

Contracts 1, 2 and 3 are specified in full below: they are product Stage 1 and
needed for the walking skeleton and the first release. Contracts 4, 5 and 6
carry their mechanism, ownership, error shape and versioning policy, with
endpoint-level detail deferred until Stage 2 work starts (Q1).

Contracts 7 and 8 cross no process boundary. The compiler is the contract:
a trait signature and a function signature, checked at build time. Writing a
document for them would add a thing to drift from without adding a check.

Two further boundaries the system **consumes** rather than exposes are not
specified here because their shape is not ours to design: the public
OpenStreetMap API behind U9, and the basemap tile service behind `MapView` —
still unchosen, and constrained by having to supply stable OpenStreetMap way
identities for selection (AC2.2.4).

---

## Contract 1 — Extract fetch (U9 → U4)

The browser asks the proxy for OpenStreetMap extract bytes covering an area.
`components.md` gives `OsmExtractProxy` an obligation that is part of the
contract rather than an operational note: it **logs neither IP addresses nor
requested locations, and never a pair of the two**, because a request log
pairing those is location data about an identifiable device
(`decisions.md` ADR-004).

```yaml
# Generated from the shared types crate. Source of truth: crate `cityloom-api-types`.
openapi: 3.1.0
info:
  title: OSM extract proxy
  version: "0"        # see "Versioning" — no API version; build stamp instead
paths:
  /api/extract:
    get:
      operationId: fetchExtract
      parameters:
        - name: bbox
          in: query
          required: true
          description: >
            Bounding area, as min-longitude, min-latitude, max-longitude,
            max-latitude in WGS84 decimal degrees.
          schema: { type: string }
        - name: x-client-build
          in: header
          required: true
          schema: { type: string }
      responses:
        "200":
          description: Extract bytes for the requested area.
          content:
            application/octet-stream:
              schema: { type: string, format: binary }
          headers:
            x-extract-key:
              description: Cache key for this extract. Keyed on the extract, never on the requester.
              schema: { type: string }
        "400": { $ref: "#/components/responses/TypedError" }
        "404": { $ref: "#/components/responses/TypedError" }
        "429": { $ref: "#/components/responses/TypedError" }
        "503": { $ref: "#/components/responses/TypedError" }
        "426":
          description: Client build no longer served. The client must reload.
          content:
            application/json:
              schema: { $ref: "#/components/schemas/ReloadRequired" }
components:
  responses:
    TypedError:
      description: A project-owned failure reason.
      content:
        application/json:
          schema: { $ref: "#/components/schemas/ApiError" }
  schemas:
    ApiError:
      type: object
      required: [reason]
      properties:
        reason: { $ref: "#/components/schemas/FailureReason" }
        detail:
          type: string
          description: >
            Human-readable, project-authored. Never a dependency's native error
            text — team.md forbids letting one reach a caller.
    FailureReason:
      type: string
      enum:
        - area_not_found
        - area_too_large
        - upstream_unavailable
        - upstream_rate_limited
        - malformed_extract
        - timeout
        - internal
    ReloadRequired:
      type: object
      required: [reason, current_build]
      properties:
        reason: { type: string, const: reload_required }
        current_build: { type: string }
```

**Retryable classes.** `upstream_unavailable`, `upstream_rate_limited` and
`timeout` are transient and may be retried. `area_not_found`,
`area_too_large` and `malformed_extract` are not: retrying a badly tagged
street returns the same bytes (AC7.2.1, AC7.2.2).

**Timeout and retry values are owed, not set** (Q6). The contract commits that
a timeout exists and that exceeding it is a failure rather than an indefinite
wait (AC3.1.6), and to which classes above are retryable. The numbers belong to
`nfr-requirements`, which AC3.1.2 and AC3.1.6 already name as their owner —
no device, no network and no reference street are established yet. NFR1.1's
10 seconds for a full import stands as the target that budget must respect.

**Rate limiting toward upstream is the provider's obligation,** not the
consumer's: a server fetching for many browsers presents differently to a free
public service than many browsers fetching individually. Its size is unmeasured
and carried as an open question below.

---

## Contract 2 — Design upload and removal (U10 → U7)

The one affordance that moves a design off the device (US8.3, a product Stage 1
Must Have). Nothing crosses this boundary unless the user explicitly asks
(AC8.3.2), and removal requires no account because the upload required none
(AC8.3.4).

```yaml
# Generated from the shared types crate. Source of truth: crate `cityloom-api-types`.
openapi: 3.1.0
info:
  title: Design storage
  version: "0"
paths:
  /api/designs:
    post:
      operationId: uploadDesign
      description: >
        Upload a design. The anonymous identifier is minted by the client from
        a cryptographic source (decisions.md ADR-006) and supplied here; the
        server does not derive it from the design or the device.
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: "#/components/schemas/UploadRequest" }
      responses:
        "201":
          description: Stored.
          content:
            application/json:
              schema: { $ref: "#/components/schemas/UploadReceipt" }
        "400": { $ref: "#/components/responses/TypedError" }
        "409": { $ref: "#/components/responses/TypedError" }
        "413": { $ref: "#/components/responses/TypedError" }
        "429": { $ref: "#/components/responses/TypedError" }
  /api/designs/{anonymousDesignId}:
    get:
      operationId: fetchDesign
      description: >
        Returns the design only to a requester granted access. An
        ungranted request returns 404 or 403 and never the design
        (AC10.1.2, AC10.1.3) — asserted as a response, not a UI state.
      parameters:
        - name: anonymousDesignId
          in: path
          required: true
          schema: { type: string, description: "128-bit value, hex-encoded" }
      responses:
        "200":
          description: The stored design.
          content:
            application/json:
              schema: { $ref: "./contract-3-design-payload.yaml#/DesignPayload" }
        "403": { $ref: "#/components/responses/TypedError" }
        "404": { $ref: "#/components/responses/TypedError" }
        "429": { $ref: "#/components/responses/TypedError" }
    delete:
      operationId: removeDesign
      description: Removal without an account, because the upload required none.
      parameters:
        - name: anonymousDesignId
          in: path
          required: true
          schema: { type: string }
      responses:
        "204": { description: Removed, or already absent. }
        "429": { $ref: "#/components/responses/TypedError" }
components:
  responses:
    TypedError:
      description: A project-owned failure reason.
      content:
        application/json:
          schema: { $ref: "#/components/schemas/ApiError" }
  schemas:
    UploadRequest:
      type: object
      required: [anonymousDesignId, payload]
      properties:
        anonymousDesignId: { type: string }
        payload: { $ref: "./contract-3-design-payload.yaml#/DesignPayload" }
    UploadReceipt:
      type: object
      required: [anonymousDesignId, uploadedAt, expiresAt]
      properties:
        anonymousDesignId: { type: string }
        uploadedAt: { type: string, format: date-time }
        expiresAt:
          type: string
          format: date-time
          description: 30 days after creation for an anonymous design (AC11.3.1).
    ApiError:
      type: object
      required: [reason]
      properties:
        reason:
          type: string
          enum:
            - design_not_found
            - not_granted
            - payload_too_large
            - unsupported_payload_version
            - rate_limited
            - internal
        detail: { type: string }
```

**Rate limiting by identifier is part of this contract, not an operational
add-on.** An anonymous identifier is the only protection an anonymous design
has, so the provider limits access attempts by identifier to make guessing
impractical (`components.md`, `DesignRepository`).

**A failed upload leaves the design on the device and says nothing was lost**
(AC8.3.3) — the consumer's obligation, and the difference between a retry and
a panic.

---

## Contract 3 — The stored design payload (U3 → U7, U10, U12)

The one serialised shape that crosses the device/server line, the
Rust/database line, and the export boundary. Its consumers are
`LocalDesignStore` and `UploadClient` (U7), `DesignRepository` (U10), and
`DataRightsService`'s export (U12).

This is a **shared schema** contract: the Rust types are the definition, and
this block is their shape written out.

```yaml
# Shared schema. Source of truth: crate `cityloom-design-payload`.
DesignPayload:
  type: object
  required: [payloadVersion, design, corrections]
  properties:
    payloadVersion:
      type: integer
      const: 1
      description: >
        Written from the first release so the option to migrate exists later
        (Q4). A reader that does not recognise a version fails in a stated,
        handled way — unsupported_payload_version — rather than crashing or
        silently mis-reading. No migration machinery is built yet.
    design:
      type: object
      required: [designId, name, streets, createdAt, updatedAt]
      properties:
        designId: { type: string }
        name: { type: string }
        createdAt: { type: string, format: date-time }
        updatedAt: { type: string, format: date-time }
        streets:
          type: array
          items: { $ref: "#/StreetKey" }
    edits:
      type: array
      items: { $ref: "#/LaneEdit" }
    corrections:
      type: array
      items: { $ref: "#/Correction" }

StreetKey:
  type: object
  required: [osmWayId, boundingNodeIds]
  description: >
    The overlay key, per team.md's Corrections: the OSM way id plus the
    direction-normalised pair of bounding OSM node ids. Never osm2streets'
    positional indices, which are not stable across a version bump or a
    re-fetch.
  properties:
    osmWayId: { type: integer, format: int64 }
    boundingNodeIds:
      type: array
      minItems: 2
      maxItems: 2
      items: { type: integer, format: int64 }

Dimension:
  type: object
  required: [metres, provenance]
  description: >
    No physical quantity is a bare number once it has crossed the adapter
    boundary (team.md Code Style). Metres are the only unit; conversion happens
    at the presentation boundary. This is what makes AC11.2.3's "every inferred
    value identifiable as inferred" true of the wire shape rather than of a
    rendering decision.
  properties:
    metres: { type: number }
    provenance: { type: string, enum: [Mapped, Inferred, UserSet] }

LaneEdit:
  type: object
  required: [editId, street, kind]
  properties:
    editId: { type: string }
    street: { $ref: "#/StreetKey" }
    laneKey:
      type: string
      description: >
        Derived from lane type, direction and ordinal from the kerb — absent on
        an addition, which carries an anchor instead (AC5.2.3, AC5.2.4).
    anchor:
      type: object
      description: Position relative to a keyed baseline lane, for a lane that exists in no import.
    kind: { type: string, enum: [change, add, remove] }
    attribute: { type: string, enum: [laneType, width, direction] }
    width: { $ref: "#/Dimension" }
    value: { type: string }

Correction:
  type: object
  required: [correctionId, street, attribute, state, importFingerprint]
  description: >
    A correction states the import itself was wrong, so it is permanent and its
    provenance is UserSet and never returns to Mapped (FR3.4, AC7.3.1). It
    carries a fingerprint a design edit has no use for.
  properties:
    correctionId: { type: string }
    street: { $ref: "#/StreetKey" }
    laneKey: { type: string }
    attribute: { type: string }
    width: { $ref: "#/Dimension" }
    value: { type: string }
    state: { type: string, enum: [applied, reapplied_after_change, unresolved] }
    importFingerprint:
      type: object
      required: [wayTags, mapConfig, osm2streetsRevision]
      description: >
        What was imported at the moment the correction was made (AC7.5.1). The
        complete MapConfig is part of it, including country_code, driving_side,
        inferred_sidewalks and inferred_kerbs.
      properties:
        wayTags: { type: object, additionalProperties: { type: string } }
        mapConfig: { type: object }
        osm2streetsRevision: { type: string }
```

**A reload must come back identical** — lane order, types, widths and every
provenance state (AC8.1.1, AC8.1.2). That is a promise about data written by
an older build, which is why `payloadVersion` exists from release one even
though nothing migrates yet.

---

## Contracts 4, 5 and 6 — product Stage 2, shape only

Recorded per Q1: mechanism, ownership, error shape and versioning are fixed
now; endpoints are filled in when Stage 2 work starts. Nothing here is
reachable before then — `project.md` mandates that the accounts and sharing
surface stays behind a server-side access gate, read from configuration and
defaulting to off, until erasure and export exist.

| Contract | Provider | Consumer | Shape fixed now |
|---|---|---|---|
| 4 | U11 `accounts-sharing` | U6 `client-surfaces` | Synchronous HTTP, JSON, types in the shared crate. Session cookies carry `HttpOnly`, `Secure` and a `SameSite` policy. Named grants and link enablement are **independent in both directions** (AC10.2.1–AC10.2.4): enabling a link leaves named grants untouched and disabling it does not remove them, so the API has no single "visibility" field that could collapse them |
| 5 | U10 / U11 | `External: public web` | Synchronous HTTP, JSON. An unauthenticated reader resolves a share link and fetches the design. A link that is off does nothing. The reader is this project's own WASM client, not a programmatic consumer — see "The condition this rests on" |
| 6 | U12 `data-rights` | U6 `client-surfaces` | Synchronous HTTP, JSON. Erasure states what will be removed **before** it happens (AC11.1.1, AC11.1.3); a deletion that fails partway is retried to completion or leaves the account intact and says so, never a half-deleted account with orphaned designs (AC11.1.4). Export returns Contract 3's payload shape for every design, with every inferred value identifiable as inferred (AC11.2.1–AC11.2.3) |

**Contract 6 does not exist in the component catalogue.** `components.md` gives
`DataRightsService` an empty `dependents` list — nothing calls it — yet US11.1
and US11.2 are user-facing Must Haves and `project.md` makes data subject
rights a gate on public release. The boundary is declared here (Q7), and the
two upstream amendments it implies are recorded under "Amendments required"
below.

## Versioning and breaking-change policy

**On the wire: no API version, a build stamp instead** (Q3). One Railway
service serves both the WASM bundle and the API and they deploy together on
merge to `main`, so a fresh client and the server are never out of step. The
only real skew is a browser holding a cached bundle from before a deploy, or a
tab left open across one — so every request carries `x-client-build`, and a
server that no longer serves that build answers `426` with the current build
identifier. The client surfaces "reload required" through `AppShell`'s live
region rather than failing silently.

**A URL version prefix was rejected**, not forgotten: it would be honest only
if there were two versions to serve, and a single deployable never has them.
The build stamp handles the one case that actually occurs.

**In storage: a version field, no migrations yet** (Q4). See Contract 3.

**Additive changes stay safe by consumers ignoring unknown fields.** A new
optional field may be added at any time. Removing a field, renaming one, or
changing its meaning is breaking.

**Breaking changes are agreed by changing the shared crate**, which makes both
ends fail to compile until both are updated. That is the whole mechanism: with
one owner per contract and one crate, there is no negotiation step to skip and
no drift to detect after the fact.

## Contract ownership

| Contract | Owner | What ownership means |
|---|---|---|
| 1 Extract fetch | U9 `osm-extract-proxy` | Owns the endpoint shape, the failure-reason set, and the no-logging obligation |
| 2 Design upload | U10 `design-storage` | Owns the endpoints, retention, object-level authorisation and identifier rate limiting |
| 3 Design payload | U3 `design-payload-spec` | Owns the shape and `payloadVersion`. Its four consumers do not change it independently — that is why it is a Unit |
| 4 Accounts and sharing | U11 `accounts-sharing` | Owns sessions, grants, links and the access gate |
| 5 Public link read | U11 `accounts-sharing` | Owns link resolution; U10 owns the design it returns |
| 6 Data rights | U12 `data-rights` | Owns erasure and export, including partial-failure behaviour |
| 7 Rights to storage/accounts | U12 | In-process; the compiler is the contract |
| 8 `StreetSource` port | U2 `street-core` | The core declares the shape; the adapter implements it. `CompositionRoot` wires them (ADR-009) |

The shared types crate is owned by whichever Unit owns the contract whose types
it carries; no Unit edits another's types. A contract with two owners has none.

## Amendments required

Three upstream declarations do not match what this stage asserted. Recorded
rather than edited, because all three belong to approved and closed stages —
the same treatment `decisions.md` ADR-001 gives its seven divergences.

| Artifact | What must change | Why |
|---|---|---|
| `components.md` (domain-design) | `AppShell` gains a `depends_on` entry for `DataRightsService`, and `DataRightsService` gains the matching `dependents` entry | `DataRightsService` currently has an empty `dependents` list, so nothing reaches it. US11.1 and US11.2 are user-facing Must Haves and `project.md` makes data subject rights a gate on public release. Contract 6 cannot be implemented against a component nothing calls |
| `unit-of-work-dependency.md` (units-generation) | The edge block gains `data-rights` to `client-surfaces`'s `depends_on` list | Contract 6 is a Unit boundary the DAG does not carry. Checked: adding it creates no cycle, because `data-rights` does not depend on `client-surfaces` — the graph stays acyclic and `client-surfaces` stays the single sink |
| `unit-of-work-dependency.md` (units-generation) | `data-rights` gains a direct `design-payload-spec` entry in its `depends_on` list | Contract 3's consumer list, that file's own "Integration points between Units" table, and `unit-of-work.md`'s statement that export "consumes U3 rather than serialising its own shape" all assert a dependency the edge block does not carry: `data-rights` declares only `[design-storage, accounts-sharing]`. The payload shape does reach it — `design-storage` depends on `design-payload-spec` — but transitively, not by the direct edge three documents claim. Checked: adding it closes no cycle, because `design-payload-spec` depends only on `street-core` |

None of the three changes a Unit boundary, a contract mechanism, or the story
map.

**Why three, when two of them were found before the third.** The first two
gaps were found by checking an assertion against the machine-readable source;
the third was asserted in the contracts table without running that same check,
and the review caught it. The lesson is narrower than "check everything": a
prose table in an upstream artifact — here, "Integration points between
Units" — is a claim of the same standing as this document's own, not evidence.
The edge block is the evidence, and it is the thing that gets parsed.

## Open questions

| Contract | Question | Blocks |
|---|---|---|
| 1 Extract fetch | The timeout value and retry counts. Deferred to `nfr-requirements` with the conditions AC3.1.2 and AC3.1.6 attach: no device, network or reference street is established yet | U4, U9 — implementable against the stated obligations, not closable without the numbers |
| 1 Extract fetch | How much rate limiting the proxy needs toward the public OpenStreetMap API, and what its egress actually costs. `team.md` requires a real figure at B-0; `project.md` forbids letting spend grow past ~$5/month without treating it as a constraint change | U9 — the measurement is a release gate, not a curiosity |
| 1 Extract fetch | Whether `bbox` is the right request shape, or whether the proxy should accept a street identity instead. `MapView`'s basemap must supply stable OpenStreetMap way identities for selection (AC2.2.4); if it does, an identity-based fetch may be narrower than an area-based one | U4, U9 — and it depends on a tile service Infrastructure Design has not chosen |
| 3 Design payload | Whether `payloadVersion` should also be recorded per-correction, since a correction's `importFingerprint` already pins an osm2streets revision and the two could diverge | U3 — additive, so it can be decided at Functional Design without breaking anything written before |
| 4, 5, 6 | Every endpoint-level detail, deferred by Q1 until product Stage 2 work starts | U11, U12 — nothing is reachable before the access gate opens |
| 5 Public link read | Whether an uploaded anonymous design is reachable by link before accounts exist. `stories.md` OQ-US2 raised this and it is still open: if it is, product Stage 1 has a sharing surface ahead of `scope-document.md`'s Stage 2 placement | U7, U10 — it would move a Stage 2 contract into Stage 1 |

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-11T13:04:44Z
**Iteration:** 1
**Request Challenge:** review:cd4580305b1394853c94aea222040527

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | `contract-summary.md` § "Amendments required", third table row | Resolved last pass. The section carries a third row: `data-rights` gains a direct `design-payload-spec` entry in its `depends_on` list. The opening prose reads "Three upstream declarations do not match," matching the three-row table. A topological sort with both proposed edges added (`client-surfaces → data-rights` and `data-rights → design-payload-spec`) completes — the graph stays acyclic. | None. | Resolved |
| R-02 | Minor | `contract-summary.md` § "How these contracts are expressed" | Resolved last pass. The section states the cross-file `$ref` is the anticipated shape of generated per-crate output, that it resolves to Contract 3's inline `DesignPayload` block, and that the named `.yaml` file "does not exist here and is not expected to." | None. | Resolved |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| YAML parse of the three fenced `yaml` blocks (Contracts 1, 2, 3) | PASS — all three parse as valid YAML | Confirms the regression check for this pass: no syntax damage from the prior repair |
| Cross-check `DataRightsService.dependents` in `components.md` | Confirmed still `[]` (line 908) | Amendment row 1's premise still holds |
| Cross-check `AppShell.depends_on` in `components.md` | Confirmed no `DataRightsService` entry (lines 652–671) | Amendment row 1's premise still holds |
| Cross-check `data-rights` edges in `unit-of-work-dependency.md` | Confirmed `depends_on: [design-storage, accounts-sharing]` only, both in the YAML block (lines 51–53) and the Mermaid diagram (lines 93–94) | Amendment rows 2 and 3's premises still hold — no `client-surfaces → data-rights` or `data-rights → design-payload-spec` edge exists |

### Summary

This pass is a narrow re-verification following a formatting-only repair to the prior review's findings table. The contract body — the contracts table, all three spec blocks, versioning policy, ownership table, the three amendment rows, and the open questions — is unchanged from what was verified last iteration. All three YAML blocks still parse, every internal `$ref` still resolves to a block defined in this same document, and both amendment claims (the empty `DataRightsService.dependents` list and the missing `data-rights` edges) still hold against the current upstream files. No new issues found; the document remains READY.
