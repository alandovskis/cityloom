# Entities — `design-payload-spec` (U3)

_Confirmed._

Upstream inputs: `contract-summary.md` Contract 3 (contract-design —
the source-of-truth schema for this Unit), `unit-of-work.md` U3
(units-generation), `requirements.md` FR3.1-FR3.4 (requirements-analysis).

This Unit owns **the shape only** — the serialised design payload that
crosses the device/server line, the Rust/database line, and the export
boundary. It reads and writes nothing; it is a contract consumed in
place by `u7-local-persistence`, `u10-design-storage`, and
`u12-data-rights`'s export.

```yaml
entities:
  - name: DesignPayload
    description: >
      The one serialised shape that crosses every boundary this project
      has. Its `payloadVersion` exists from release one even though
      nothing migrates yet, so the option to migrate exists later.
    attributes:
      - name: payloadVersion
        type: integer
        required: true
        constraints: ["const 1 — a reader that does not recognise a version fails as unsupported_payload_version, a stated and handled outcome, never a crash or silent misread"]
      - name: design
        type: object
        required: true
        constraints: ["required sub-fields: designId, name, streets, createdAt, updatedAt"]
      - name: edits
        type: array
        required: false
        references: LaneEdit
      - name: corrections
        type: array
        required: true
        references: Correction

  - name: StreetKey
    description: >
      The overlay key per team.md's Corrections: the OSM way id plus the
      direction-normalised pair of bounding OSM node ids — never
      osm2streets' positional indices, which are not stable across a
      version bump or a re-fetch.
    attributes:
      - name: osmWayId
        type: integer
        required: true
      - name: boundingNodeIds
        type: array
        required: true
        constraints: ["exactly 2 items"]

  - name: Dimension
    description: >
      No physical quantity is a bare number once it has crossed the
      adapter boundary. Metres are the only unit; conversion happens at
      the presentation boundary. This is what makes AC11.2.3's "every
      inferred value identifiable as inferred" true of the wire shape
      itself, not of a rendering decision made later.
    attributes:
      - name: metres
        type: float
        required: true
      - name: provenance
        type: enum
        required: true
        allowed_values: [Mapped, Inferred, UserSet]

  - name: LaneEdit
    description: >
      A proposed change to the baseline: change an existing lane's
      attribute, add a lane that exists in no import, or remove one.
    attributes:
      - name: editId
        type: string
        required: true
      - name: street
        type: reference
        required: true
        references: StreetKey
      - name: laneKey
        type: string
        required: false
        constraints: ["derived from lane type, direction and ordinal from the kerb; absent on an addition, which carries an anchor instead (AC5.2.3, AC5.2.4)"]
      - name: anchor
        type: object
        required: false
        constraints: ["position relative to a keyed baseline lane, for a lane that exists in no import"]
      - name: kind
        type: enum
        required: true
        allowed_values: [change, add, remove]
      - name: attribute
        type: enum
        required: false
        allowed_values: [laneType, width, direction]
      - name: width
        type: reference
        required: false
        references: Dimension
      - name: value
        type: string
        required: false

  - name: Correction
    description: >
      States that the import itself was wrong, so it is permanent and
      its provenance is UserSet and never returns to Mapped (FR3.4,
      AC7.3.1). Carries a fingerprint a design edit has no use for.
    attributes:
      - name: correctionId
        type: string
        required: true
      - name: street
        type: reference
        required: true
        references: StreetKey
      - name: laneKey
        type: string
        required: false
      - name: attribute
        type: string
        required: false
      - name: width
        type: reference
        required: false
        references: Dimension
      - name: value
        type: string
        required: false
      - name: state
        type: enum
        required: true
        allowed_values: [applied, reapplied_after_change, unresolved]
      - name: importFingerprint
        type: object
        required: true
        references: ImportFingerprint

  - name: ImportFingerprint
    description: >
      What was imported at the moment the correction was made (AC7.5.1).
    attributes:
      - name: wayTags
        type: object
        required: true
      - name: mapConfig
        type: object
        required: true
        constraints: ["includes country_code, driving_side, inferred_sidewalks, inferred_kerbs"]
      - name: osm2streetsRevision
        type: string
        required: true
```

## Summary

Six types, all named directly by Contract 3: `DesignPayload` is the root,
`StreetKey`/`Dimension` are shared value shapes also used by `street-core`
(this crate does not depend on `street-core`, however — it is a
standalone serialization spec, per `unit-of-work.md`'s "spec" kind
carrying no components of its own), `LaneEdit` and `Correction` are the
two kinds of change a payload can carry, and `ImportFingerprint` pins
exactly what was imported when a correction was made.

## Assumptions & Open Questions

None — the shape is already fixed by `contract-summary.md` Contract 3.

## Traceability

See `traceability.json` in this directory.
