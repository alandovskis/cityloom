# Entities — `street-core` (U2)

_Confirmed._

Upstream inputs: `components.md` `StreetModel` (domain-design), `requirements.md`
FR3.1-FR3.4 (requirements-analysis), `unit-of-work.md` U2 (units-generation),
`team.md` Code Style (metres-only, bounds-checked access, overlay-keying
correction).

This Unit owns the **imported baseline**: the street and lane types exactly
as osm2streets returned them, converted into this project's own
provenance-carrying types. It owns no editing state (that is `u5-design-editing`)
and no rendering (that is `u6-client-surfaces`).

```yaml
entities:
  - name: Provenance
    description: >
      The state of a single dimension: whether it was read from OpenStreetMap,
      derived by osm2streets from other tags or defaults, or corrected/entered
      by a user. Every Dimension carries exactly one. This is a closed enum,
      not a nullable field — the "no provenance" state does not exist
      (FR3.1; the un-annotated form is unconstructable per `team.md`).
    attributes:
      - name: state
        type: enum
        required: true
        allowed_values: [Mapped, Inferred, UserSet]
    constraints:
      - "A Mapped value becomes UserSet on correction (FR3.4) and never reverts to Mapped."
      - "Inferred is never presented as a measurement in any view or export (FR3.3)."

  - name: Dimension
    description: >
      A physical quantity (a width, a count, an offset) paired with its
      Provenance. The wrapper is the point: a bare number for a quantity
      that has crossed the osm2streets adapter boundary cannot be
      constructed without also stating where it came from.
    attributes:
      - name: metres
        type: float
        required: true
        constraints: ["Metres is the only unit stored; conversion happens only at the presentation boundary (TC-5)."]
      - name: provenance
        type: reference
        required: true
        references: Provenance

  - name: Lane
    description: >
      One lane of a Street, in the order it is drawn. Its key is derived,
      not generated, from properties intrinsic to the lane itself, so the
      same lane in two independent imports of the same street resolves to
      the same key (AC7.3.4) and a correction survives a re-import.
    identifier: "laneKey, derived from laneType, direction and ordinalFromKerb — never a positional index"
    attributes:
      - name: laneKey
        type: string
        required: true
        unique: true
        constraints: ["Derived (laneType, direction, ordinalFromKerb), never a position in a list, and never an osm2streets positional (usize) index (team.md Code Style)."]
      - name: laneType
        type: string
        required: true
        constraints: ["Naming authority is osm2streets' own schema (team.md Code Style) — this project does not invent lane-type vocabulary."]
      - name: direction
        type: enum
        required: true
      - name: width
        type: reference
        required: true
        references: Dimension
      - name: ordinalFromKerb
        type: integer
        required: true
        min: 0
    relationships:
      - entity: Street
        owned_by: StreetModel
        cardinality: "N:1"
        relationship: "each Lane belongs to exactly one Street"

  - name: Street
    description: >
      One imported street: its identity, name, and ordered lane list. The
      baseline is immutable once constructed — held behind shared
      references only, never mutable ones (team.md Code Style) — so no
      editing operation anywhere in the system can reach into it directly.
    identifier: "osmWayId plus the direction-normalised bounding node-id pair of the segment (the corrected overlay key from team.md's Corrections — never osmWayId alone, since split_ways can produce multiple Streets sharing one way id)"
    attributes:
      - name: osmWayId
        type: string
        required: true
      - name: boundingNodeIds
        type: reference
        required: true
        constraints: ["A direction-normalised pair of OSM node ids bounding this segment — what split_ways actually splits on, and stable OSM identities."]
      - name: name
        type: string
        required: false
      - name: lanes
        type: array
        required: true
        references: Lane
        constraints: ["Ordered, positionally addressed for display purposes only — identity and access use laneKey via bounds-checked accessors, per team.md's `.get(i)`-style rule, never `lanes[i]`."]
      - name: carriagewayWidth
        type: reference
        required: true
        references: Dimension
        constraints: ["The project-owned definition: sum of lane widths between the two kerb buffers where both exist, otherwise the sum excluding walkable lane types and verge buffers. osm2streets returns no kerb-to-kerb width directly; this is computed here, once, so there is exactly one definition of the kerb (components.md StreetModel)."]
    relationships:
      - entity: StreetNetworkGraph
        owned_by: StreetModel
        cardinality: "N:1"
        relationship: "a Street is one node of the network graph"

  - name: StreetNetworkGraph
    description: >
      The network of imported streets and how they connect — a core type,
      not an adapter type, so the outer ring never depends on osm2streets'
      own graph representation.
    identifier: "graphId"
    attributes:
      - name: graphId
        type: string
        required: true
        unique: true
      - name: streets
        type: array
        required: true
        references: Street
      - name: intersections
        type: array
        required: true
      - name: adjacency
        type: reference
        required: true
        constraints: ["Which streets meet at which intersections; read-only from this Unit's perspective."]
    relationships:
      - entity: Street
        owned_by: StreetModel
        cardinality: "1:N"
        relationship: "the graph's nodes are Streets from this same component"
```

## Summary

Four entities, all owned by `StreetModel`: `Provenance` and `Dimension` are
the value-object pair that makes "this number's origin" a compile-time
fact rather than a convention; `Lane` and `Street` are the imported
baseline itself, keyed so a correction or a design edit made against one
import survives a later re-import of the same real-world street. The
`StreetNetworkGraph` composes `Street`s into the connected network the
corridor and connectivity features (`u5-design-editing`) read.

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
