# Functional Specification — `design-payload-spec` (U3)

_Confirmed._

Upstream inputs: `entities.md`, `rules.md` (this stage), `contract-summary.md`
Contract 3 (contract-design).

This Unit has no workflow of its own — it is a shape, consumed in place
by four other Units. What follows is the round-trip contract those
consumers must all satisfy, since `entities.md`/`rules.md` describe data
shape and rules but not the ordered read/write behaviour the round-trip
promise depends on.

## Workflow: write then read (the round-trip promise BR2.1 exists to name)

1. A consumer (`LocalDesignStore`, `UploadClient`, or `DesignRepository`)
   constructs a `DesignPayload` with `payloadVersion: 1`, the current
   `design` fields, the `edits` list, and the `corrections` list.
2. The payload is serialized and stored (browser storage, or the server
   database via `u10-design-storage`).
3. On a later read, the consumer deserializes the payload and checks
   `payloadVersion` first (BR1.1) before touching any other field.
4. If the version is recognised (1, today's only value), every field —
   lane order, types, widths, every `Dimension`'s `provenance` — must
   equal what was written (BR2.1). If unrecognised, the read fails with
   `unsupported_payload_version` and nothing further is attempted.

## Workflow: export (the AC11.2.3 promise)

`u12-data-rights`'s export reuses this exact shape rather than
serialising its own — `unit-of-work.md`'s stated reason is exactly
BR3.1: every inferred value is already identifiable as inferred in this
payload, so export has nothing extra to compute.

## State machine: Correction provenance (restated from `street-core`'s Provenance model, at the wire level)

A `Correction`'s existence as a distinct type — rather than a `LaneEdit`
with a `provenance` field set to `UserSet` — is itself the enforcement
mechanism for BR4.1: there is no field in this shape that could
represent "a correction with Mapped provenance." The distinction is
structural, not a runtime check.

## Assumptions & Open Questions

None — Contract 3 already resolved every open point this stage would
otherwise raise (payloadVersion behaviour at Q4, the additive-evolution
policy).

## Traceability

See `traceability.json` in this directory.
