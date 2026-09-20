# Business Rules — `street-core` (U2)

_Confirmed._

Upstream inputs: `entities.md` (this stage), `components.md` StreetModel
(domain-design), `requirements.md` FR3.1-FR3.4 (requirements-analysis),
`team.md` Code Style.

```yaml
rules:
  # BR1 — Provenance
  - id: BR1.1
    statement: "Every Dimension carries exactly one Provenance state: Mapped, Inferred, or UserSet. There is no unmarked or default state."
    category: constraint
    applies_to: Dimension
    trigger: "Dimension construction"
    logic: "IF a Dimension is constructed THEN it must be given a Provenance; the type system makes the un-annotated form unconstructable."
    violation_behaviour: "Not a runtime case — enforced at compile time by the type itself (no `Dimension::new(metres)` without a Provenance argument)."
    source: FR3.1

  - id: BR1.2
    statement: "A correction to an imported value sets that value's Provenance to UserSet, and a UserSet value never reverts to Mapped."
    category: policy
    applies_to: Dimension, Provenance
    trigger: "A correction is applied to a Dimension"
    logic: "IF a Dimension's value is corrected THEN its Provenance becomes UserSet; IF a Provenance is already UserSet THEN no operation in this Unit may set it back to Mapped."
    violation_behaviour: "This Unit exposes no API that transitions UserSet back to Mapped — the illegal transition is absent, not rejected at runtime."
    source: FR3.4

  # BR2 — Lane identity and access
  - id: BR2.1
    statement: "A Lane's key is derived from its laneType, direction, and ordinalFromKerb — never from its position in the lane list, and never from an osm2streets positional (usize) index."
    category: constraint
    applies_to: Lane
    trigger: "Lane construction during import"
    logic: "IF a Lane is constructed THEN its laneKey is computed from (laneType, direction, ordinalFromKerb); a positional index is never used as or folded into the key."
    violation_behaviour: "A build-time/code-review discipline (team.md Code Style); this Unit's own API never accepts a positional index as a lane identifier."
    source: "team.md Code Style (Corrections); AC7.3.4"

  - id: BR2.2
    statement: "Lane access by index returns an Option, never panics, for any out-of-range index."
    category: constraint
    applies_to: Street, Lane
    trigger: "Any positional lane access (for display ordering)"
    logic: "IF a caller requests lane N of a Street THEN the accessor returns None for N outside the lane list's bounds, rather than panicking."
    violation_behaviour: "Compile-time enforced by using `.get(i)`-style bounds-checked accessors exclusively — never bare `lanes[i]` indexing."
    source: "team.md Code Style"

  # BR3 — Street identity
  - id: BR3.1
    statement: "A Street's identity is its OSM way id plus the direction-normalised pair of bounding OSM node ids for its segment — never the way id alone."
    category: constraint
    applies_to: Street
    trigger: "Street construction during import"
    logic: "IF a Street is constructed THEN its identity key is (osmWayId, boundingNodeIds); two Streets from the same way id but different node-id pairs are distinct Streets."
    violation_behaviour: "A build-time type constraint — the identity key type has no single-field constructor from osmWayId alone."
    source: "team.md Corrections (osm2streets split_ways many-to-many correction)"

  # BR4 — Carriageway width
  - id: BR4.1
    statement: "Carriageway width is the sum of lane widths between the two kerb buffers where both exist; otherwise the sum of lane widths excluding walkable lane types and verge buffers."
    category: calculation
    applies_to: Street
    trigger: "Street construction, or any read of carriagewayWidth"
    logic: "IF both kerb-buffer lanes exist in the lane list THEN sum widths strictly between them; ELSE sum all lane widths except walkable-type lanes and verge buffers."
    violation_behaviour: "This is the single, sole definition of carriageway width in the system — no other component computes or overrides it (components.md StreetModel: 'one definition of the kerb')."
    source: "components.md StreetModel"

  # BR5 — Immutability
  - id: BR5.1
    statement: "The imported baseline (Street, Lane, StreetNetworkGraph) is held behind shared references only, never mutable references, once constructed."
    category: constraint
    applies_to: Street, Lane, StreetNetworkGraph
    trigger: "Any access to the baseline after import completes"
    logic: "IF code outside this Unit needs to change a street THEN it constructs a separate overlay (CorrectionOverlay, DesignOverlay) rather than obtaining a mutable reference into the baseline; no API in this Unit returns `&mut Street` or equivalent."
    violation_behaviour: "A compile error — this Unit's public API has no mutable-access function to violate at runtime."
    source: "team.md Code Style (NEVER edit OSM data corollary)"

  # BR6 — Metric units
  - id: BR6.1
    statement: "Every Dimension's metres field is stored in metres; no other unit is stored anywhere in this Unit's types."
    category: constraint
    applies_to: Dimension
    trigger: "Dimension construction"
    logic: "IF a physical quantity is constructed as a Dimension THEN its stored value is in metres, regardless of the jurisdiction that will eventually display it."
    violation_behaviour: "A type-level convention; unit conversion is a presentation-boundary concern this Unit does not implement."
    source: "requirements.md TC-5; team.md Code Style"

  # BR7 — The StreetSource port
  - id: BR7.1
    statement: "This Unit declares the StreetSource port (the shape of 'import a street'); it does not implement it, and it depends on no concrete import mechanism."
    category: policy
    applies_to: StreetModel
    trigger: "Compile time — crate dependency graph"
    logic: "IF a component needs to trigger an import THEN it calls through the StreetSource port this Unit declares; the concrete implementor (StreetImportAdapter, u4-street-import) is supplied by CompositionRoot, never named here."
    violation_behaviour: "Enforced by Cargo's own dependency graph: this Unit's crate has no dependency on the osm2streets crate or the adapter crate — a violation is a compile error, not a lint (team.md Code Style, three inward-pointing layers)."
    source: "components.md StreetModel; decisions.md ADR-009"
```

## Summary

| Rule | Category | What it protects |
|---|---|---|
| BR1.1, BR1.2 | Provenance | A value's origin is never lost, and a correction is permanent |
| BR2.1, BR2.2 | Lane identity/access | A lane survives re-import; access never panics |
| BR3.1 | Street identity | A `split_ways`-divided street doesn't collide with its neighbour |
| BR4.1 | Carriageway width | One definition of the kerb, computed once |
| BR5.1 | Immutability | No edit can reach the imported baseline directly |
| BR6.1 | Units | Metres only; jurisdiction conversion stays out of the core |
| BR7.1 | The import port | The core never depends on osm2streets directly |

## Traceability

See `traceability.json` in this directory.
