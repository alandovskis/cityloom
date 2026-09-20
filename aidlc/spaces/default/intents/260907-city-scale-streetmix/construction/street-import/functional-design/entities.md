# Entities — `street-import` (U4)

_Confirmed._

Upstream inputs: `unit-of-work.md` U4, `components.md` (`ExtractFetcher`,
`StreetImportAdapter`, `CorrectionOverlay`), `contract-summary.md` Contract 1
(extract fetch) and Contract 8 (`StreetSource` port), `stories.md` US3.1,
US7.1-US7.5.

This Unit owns the request-to-corrected-street pipeline: fetching extract
bytes, converting them into the core's provenance-carrying types, and
correcting what the import got wrong. It does **not** own `Street`/`Lane`
(those belong to `street-core`, U2) — it constructs them. It does not own
the wire shape a correction is serialized into (that is `design-payload-spec`,
U3's `Correction`/`ImportFingerprint` types) — it owns the live, in-memory
correction model `CorrectionOverlay` operates on, which the outer ring
(`LocalDesignStore`, U7) serializes through U3's shape at the persistence
boundary.

```yaml
entities:
  - name: ImportFailure
    description: >
      The typed, closed failure surface an import can produce. Never a
      dependency's native error text (team.md Code Style; AC7.1.2). The
      adapter and the fetcher both produce values of this type — the
      fetcher for transport-level failures, the adapter for conversion
      failures — sharing one enum so US7.1 has one finite surface to
      write copy for.
    attributes:
      - name: reason
        type: enum
        required: true
        allowed_values: [area_not_found, area_too_large, upstream_unavailable, upstream_rate_limited, malformed_extract, timeout, internal]
        constraints: ["mirrors Contract 1's FailureReason enum verbatim — the fetcher's failures ARE this enum; the adapter adds no reasons of its own beyond malformed_extract/internal for conversion-time failures osm2streets itself cannot avoid triggering"]
      - name: street_name
        type: string
        required: true
        constraints: ["the same name shown at selection (AC7.1.1) — never a bare identifier"]
      - name: retryable
        type: boolean
        required: true
        constraints: ["true for upstream_unavailable, upstream_rate_limited, timeout; false for area_not_found, area_too_large, malformed_extract, internal — per Contract 1's retryable-classes note"]

  - name: ImportedStreet
    description: >
      The successful outcome of an import: the constructed `StreetModel`
      types (owned by U2, not this Unit) plus this Unit's own bookkeeping
      about which corrections apply to it. Not persisted by this Unit —
      it is handed to the outer ring through the `StreetSource` port
      (Contract 8) and the caller decides what happens next.
    attributes:
      - name: street
        type: reference
        required: true
        references: "Street (street-core, external)"
      - name: network_graph
        type: reference
        required: false
        references: "StreetNetworkGraph (street-core, external)"
        constraints: ["present when the import is part of a corridor fetch; a single-street import may omit it"]
      - name: correction_reconciliation
        type: array
        required: false
        references: CorrectionReconciliationOutcome
        constraints: ["one entry per stored Correction whose targetStreet matches this import — empty on a street's first-ever import"]

  - name: Correction
    description: >
      A user's statement that the import itself was wrong. Permanent —
      its provenance is UserSet and never returns to Mapped (FR3.4,
      AC7.3.1) — distinct from a design edit (owned by `design-editing`,
      U5), which proposes a change to a street the import described
      correctly. This Unit's live, in-memory model of the type; U3's
      `Correction` (in `entities.md` there) is the wire shape the same
      conceptual entity takes when `local-persistence` (U7) serializes
      it — the two are intentionally not the same Rust type, per U3's
      own stated boundary ("the shape only").
    attributes:
      - name: correction_id
        type: string
        required: true
      - name: target_street
        type: reference
        required: true
        references: "Street key (osmWayId + direction-normalised bounding node-id pair)"
      - name: target_lane_discriminator
        type: string
        required: false
        constraints: ["the AC7.3.4 derived per-lane key (lane type, direction, ordinal from the kerb) — absent for a correction that removes a lane invented by the import, since there is no lane left to key by once removed"]
      - name: attribute
        type: string
        required: false
      - name: value
        type: string
        required: false
      - name: import_fingerprint
        type: reference
        required: true
        references: ImportFingerprint
      - name: state
        type: enum
        required: true
        allowed_values: [applied, reapplied_after_change, unresolved]
        constraints: ["set by CorrectionReconciliationOutcome on every re-import (AC7.5.2-AC7.5.4); starts applied at creation"]

  - name: ImportFingerprint
    description: >
      What was imported at the moment a Correction was made (AC7.5.1) —
      the basis for the three re-import outcomes (BR6.1). Structurally
      identical to U3's `ImportFingerprint` (they describe the same
      fact); this Unit's copy is the live value `CorrectionOverlay`
      compares against a fresh import, never the wire shape itself.
    attributes:
      - name: way_tags
        type: object
        required: true
      - name: map_config
        type: object
        required: true
        constraints: ["includes country_code, driving_side, inferred_sidewalks, inferred_kerbs"]
      - name: osm2streets_revision
        type: string
        required: true

  - name: CorrectionReconciliationOutcome
    description: >
      The result of comparing a stored Correction's ImportFingerprint
      against a fresh import (BR6.1, AC7.5.2-AC7.5.4). Exactly one of
      three outcomes — never a fourth, and never silent.
    attributes:
      - name: correction_id
        type: reference
        required: true
        references: Correction
      - name: outcome
        type: enum
        required: true
        allowed_values: [reapplied_silently, reapplied_with_notice, unresolved]
      - name: notice
        type: string
        required: false
        constraints: ["present only when outcome is reapplied_with_notice — tells the user the underlying import changed (AC7.5.3)"]
```

## Summary

Five types own this Unit's behaviour: `ImportFailure` is the closed, typed
failure surface US7.1/US7.2 render from; `ImportedStreet` is the
`StreetSource` port's success value; `Correction` and `ImportFingerprint` are
this Unit's live in-memory models of the same facts U3 defines as a wire
shape; `CorrectionReconciliationOutcome` is the per-correction verdict a
re-import produces. `Street`/`Lane`/`StreetNetworkGraph` are referenced but
owned by `street-core` (U2) — this Unit constructs values of those types, it
does not define them.

## Assumptions & Open Questions

None — every type here is named directly by `components.md`'s behaviour
prose for `ExtractFetcher`, `StreetImportAdapter`, and `CorrectionOverlay`,
or by Contract 1's `FailureReason` enum.

## Traceability

See `traceability.json` in this directory.
