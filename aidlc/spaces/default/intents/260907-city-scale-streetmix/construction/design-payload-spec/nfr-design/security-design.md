# Security Design — `design-payload-spec` (U3)

_Confirmed._

Upstream inputs: `security-requirements.md`, `tech-stack-decisions.md`
(nfr-requirements, this Unit). Design elements are numbered `SD-n`.

## Design elements

- **SD-1 — Fail-closed version check.** Deserialization derives
  `#[derive(Deserialize)]` for `DesignPayload` with `payloadVersion`
  checked explicitly in a `TryFrom`/constructor wrapper before any other
  field is trusted — a raw `serde` derive alone would happily deserialize
  an unrecognised version's bytes into the current struct shape if the
  fields happen to match, which is exactly the silent-misread failure
  BR1.1 forbids. The wrapper returns `Err(UnsupportedPayloadVersion)`
  for anything other than `1`. Meets NFR6.4.4.
- **SD-2 — Correction has no provenance field.** `Correction`'s struct
  definition (per `entities.md`) carries no `provenance` field at all —
  its UserSet-ness is structural, not a value that could be set
  incorrectly. `LaneEdit` is the only type with attribute values that
  carry a `Dimension` (and therefore a settable `Provenance`); a
  `Correction`'s corrected value uses the same `width`/`value` fields
  without a provenance field, because the type itself (being a
  Correction rather than a LaneEdit) is what asserts UserSet. Meets
  NFR6.4.5.
- **SD-3 — No PII-shaped fields.** The schema (per `entities.md`) has
  exactly the fields Contract 3 specifies — `design.name`, `LaneEdit.value`,
  `Correction.value` are free-text but scoped to street-design content
  (lane type names, corridor names) by their position in the schema and
  their consumers' documented purpose; no field is named or typed in a
  way that invites storing account/personal data (that lives in
  `u11-accounts-sharing`'s own schema, not here). Meets NFR6.3.1.

## Threats not applicable here

See `security-requirements.md`'s STRIDE table — Spoofing, Repudiation,
and Elevation of Privilege remain N/A for the reasons stated there.

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
