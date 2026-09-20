# Business Rules — `design-payload-spec` (U3)

_Confirmed._

Upstream inputs: `entities.md` (this stage), `contract-summary.md`
Contract 3 (contract-design), `requirements.md` FR3.4.

```yaml
rules:
  # BR1 — Versioning
  - id: BR1.1
    statement: "payloadVersion is a fixed constant (1) for every payload this release writes; a reader encountering an unrecognised version fails with a stated outcome, never a crash or silent misread."
    category: constraint
    applies_to: DesignPayload
    trigger: "Payload construction (write) or deserialization (read)"
    logic: "IF payloadVersion is written THEN it is exactly 1; IF a reader encounters any other value THEN it returns unsupported_payload_version rather than attempting to interpret the payload."
    violation_behaviour: "A reader that does not implement this check risks silently misreading a future-versioned payload — this rule is why every consumer must check the field before touching `design`/`edits`/`corrections`."
    source: "contract-summary.md Contract 3 (Q4)"

  # BR2 — Round-trip fidelity
  - id: BR2.1
    statement: "A reload must come back identical: lane order, types, widths, and every provenance state, for a payload written by any build that shares payloadVersion 1."
    category: constraint
    applies_to: DesignPayload
    trigger: "Write then read of the same payload"
    logic: "IF a payload is serialized and later deserialized THEN every field's value and every Dimension's provenance state is byte-for-byte equal to what was written."
    violation_behaviour: "This is the acceptance criterion this whole Unit exists to satisfy (AC8.1.1, AC8.1.2) — a serialization/deserialization asymmetry here is a defect, not a design choice."
    source: "AC8.1.1, AC8.1.2; contract-summary.md Contract 3"

  # BR3 — Provenance carried on the wire
  - id: BR3.1
    statement: "Every Dimension on the wire carries its Provenance; an inferred value is identifiable as inferred from the payload alone, without any rendering-time decision."
    category: constraint
    applies_to: Dimension
    trigger: "Any Dimension serialized into a payload"
    logic: "IF a Dimension crosses this boundary THEN its provenance field is present and one of Mapped/Inferred/UserSet — never omitted or inferred from context."
    violation_behaviour: "Operationalises AC11.2.3 — export must preserve the inferred/mapped distinction, which is only possible if the wire shape carries it explicitly."
    source: "AC11.2.3; contract-summary.md Contract 3"

  # BR4 — Correction permanence
  - id: BR4.1
    statement: "A correction's provenance is always UserSet and never returns to Mapped, because a correction states the import itself was wrong."
    category: policy
    applies_to: Correction
    trigger: "Correction construction"
    logic: "IF a Correction is created THEN the value it carries has UserSet provenance implicitly (Correction has no provenance field of its own — its existence as a Correction, distinct from a LaneEdit, is what asserts UserSet, per FR3.4)."
    violation_behaviour: "Enforced by type distinction: this shape has no field that could represent a Correction as Mapped."
    source: "FR3.4; AC7.3.1"

  # BR5 — Import fingerprint completeness
  - id: BR5.1
    statement: "Every Correction carries a complete import fingerprint (way tags, the complete MapConfig, and the osm2streets revision) captured at the moment the correction was made."
    category: constraint
    applies_to: Correction, ImportFingerprint
    trigger: "Correction construction"
    logic: "IF a Correction is created THEN its importFingerprint includes wayTags, mapConfig (with country_code, driving_side, inferred_sidewalks, inferred_kerbs), and osm2streetsRevision — all required, none optional."
    violation_behaviour: "A Correction missing any fingerprint field cannot support the re-import reconciliation logic (AC7.5.1-AC7.5.4) that `u4-street-import` implements against this shape."
    source: "AC7.5.1; contract-summary.md Contract 3"

  # BR6 — Additive-only evolution
  - id: BR6.1
    statement: "Only additive, optional-field changes to this shape are backward compatible; removing a field, renaming one, or changing its meaning is a breaking change requiring the shared crate itself to change."
    category: policy
    applies_to: DesignPayload
    trigger: "Any future change to this schema"
    logic: "IF a field is added THEN it must be optional, so old readers ignore it safely; IF a field is removed, renamed, or its meaning changes THEN this is breaking and both ends recompile against the updated shared crate together — there is no independent negotiation."
    violation_behaviour: "Not runtime-enforced — this is the evolution policy the shared-crate ownership model exists to make cheap to follow and hard to violate accidentally (one crate, one owner, both ends fail to compile until updated)."
    source: "contract-summary.md 'Versioning and breaking-change policy'"
```

## Summary

| Rule | Category | What it protects |
|---|---|---|
| BR1.1 | Versioning | A future format change fails loudly, not silently |
| BR2.1 | Round-trip fidelity | A saved design is never silently altered by reload |
| BR3.1 | Provenance on the wire | Export/import never loses the inferred/mapped distinction |
| BR4.1 | Correction permanence | A correction can never be mistaken for a measurement |
| BR5.1 | Import fingerprint | Re-import reconciliation has what it needs |
| BR6.1 | Additive-only evolution | Four independent consumers never drift out of sync |

## Traceability

See `traceability.json` in this directory.
