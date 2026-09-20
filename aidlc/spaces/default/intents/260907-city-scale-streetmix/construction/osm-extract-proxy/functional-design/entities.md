# Entities — `osm-extract-proxy` (U9)

Upstream inputs: `unit-of-work.md` and `unit-of-work-story-map.md`
(units-generation), `requirements.md` (requirements-analysis), `components.md`
(domain-design), `contract-summary.md` (contract-design),
`functional-design-questions.md` (this stage).

The proxy serves OpenStreetMap extract bytes for a bounding box. After this
stage's Q1 it does so by cutting clips from **regional data built offline**
(Geofabrik sub-region extracts, filtered and sliced into a grid of cells,
shipped with the deploy) rather than by querying a live service. The entity
set below is the source of truth for the data this Unit holds; `rules.md`
holds the decision logic and `functional-spec.md` the workflows and
lifecycles.

## Entity model

```yaml
entities:
  - name: RegionalDataBuild
    description: >
      One product of the offline build step: every configured region,
      filtered to what osm2streets reads and sliced into grid cells, plus the
      manifest the service loads at start. A running service holds exactly
      one build; a new weekly build supersedes the previous one entirely.
    attributes:
      - name: buildId
        type: identifier
        required: true
        unique: true
        constraints: >
          Content-derived from the regions' source digests and the grid
          specification, so two builds from identical inputs are the same
          build and a build from changed inputs is a different one.
      - name: builtAt
        type: timestamp
        required: true
      - name: gridSpec
        type: GridSpec
        required: true
      - name: regions
        type: list of Region
        required: true
        min: 1
      - name: coverage
        type: CoverageIndex
        required: true
      - name: cellCount
        type: integer
        required: true
        min: 0
      - name: totalBytes
        type: integer
        required: true
        min: 0
      - name: filterProfile
        type: text
        required: true
        constraints: >
          Names the tag classes kept by the filter step (at minimum every way
          carrying a highway tag and every node those ways reference), so a
          build records what it contains rather than implying it.
    entity_constraints:
      - Immutable once published; a correction is a new build.
      - The service loads a build only if every cell listed in its manifest is present and matches its recorded digest.
    relationships:
      - target: Region
        cardinality: one build contains one or more regions
        direction: build -> region
      - target: Cell
        cardinality: one build contains zero or more cells
        direction: build -> cell

  - name: Region
    description: >
      One Geofabrik sub-region extract as consumed by a build, identified by
      its source path (for example a province).
    attributes:
      - name: sourcePath
        type: text
        required: true
        unique: true
        constraints: The publisher's path for the sub-region, as configured.
      - name: sourcePublishedAt
        type: timestamp
        required: true
        constraints: The publisher's timestamp for the file that was downloaded.
      - name: sourceDigest
        type: digest
        required: true
        constraints: Verified against the publisher's checksum before use.
      - name: sourceBytes
        type: integer
        required: true
        min: 0
      - name: bounds
        type: BoundingBox
        required: true
        constraints: The extent the region contributes to the coverage index.
    entity_constraints:
      - A region contributes only cells inside its bounds.
    relationships:
      - target: Cell
        cardinality: one region contributes to zero or more cells; a cell on a border may be fed by more than one region
        direction: region -> cell

  - name: GridSpec
    description: >
      Value object fixing how the plane is cut into cells: a regular grid in
      WGS84 degrees anchored at longitude 0, latitude 0.
    attributes:
      - name: cellSizeDegrees
        type: decimal
        required: true
        min: 0
        constraints: >
          The value is owed to nfr-requirements (this stage sets the rule,
          not the number); it must be a divisor of 1 so cell edges are exact.
      - name: coordinateSystem
        type: enumeration
        required: true
        allowed: [WGS84]
        default: WGS84
    entity_constraints:
      - Equality is by value; a build and every clip cut from it share one GridSpec.

  - name: CoverageIndex
    description: >
      Value object: the set of cell identifiers that lie inside at least one
      configured region's bounds. It answers "is this box somewhere this
      deployment covers", independently of whether the cell has any content.
    attributes:
      - name: coveredCellIds
        type: set of CellId
        required: true
    entity_constraints:
      - Derived at build time from the regions' bounds and the GridSpec; never edited at run time.

  - name: Cell
    description: >
      The filtered ways and nodes falling in one grid cell. A way appears in
      every cell its geometry touches, so a clip can be assembled from cells
      alone.
    attributes:
      - name: cellId
        type: CellId
        required: true
        unique: true
        constraints: Derived from the cell's column and row in the GridSpec.
      - name: bounds
        type: BoundingBox
        required: true
      - name: wayCount
        type: integer
        required: true
        min: 0
      - name: nodeCount
        type: integer
        required: true
        min: 0
      - name: sizeBytes
        type: integer
        required: true
        min: 0
      - name: digest
        type: digest
        required: true
    entity_constraints:
      - A cell file exists only for cells with at least one way; a covered cell with no file is "covered but empty".
      - Every node referenced by a way in the cell is present in that cell, even when the node lies outside the cell's bounds.

  - name: BoundingBox
    description: >
      Value object for a requested or stored area in WGS84 decimal degrees.
    attributes:
      - name: minLongitude
        type: decimal
        required: true
        min: -180
        max: 180
      - name: minLatitude
        type: decimal
        required: true
        min: -90
        max: 90
      - name: maxLongitude
        type: decimal
        required: true
        min: -180
        max: 180
      - name: maxLatitude
        type: decimal
        required: true
        min: -90
        max: 90
    entity_constraints:
      - minLongitude is strictly less than maxLongitude and minLatitude strictly less than maxLatitude.
      - The canonical form rounds every coordinate to a fixed number of decimal places; only the canonical form keys a cache entry or an extract key.

  - name: ExtractKey
    description: >
      Value object identifying one extract: derived from the canonical
      BoundingBox and the buildId, and nothing else. Returned to the client
      in the x-extract-key header.
    attributes:
      - name: value
        type: digest
        required: true
        unique: true
    entity_constraints:
      - Contains no requester-derived input; the same box on the same build yields the same key on every instance.

  - name: CachedExtract
    description: >
      One clip held in memory after it was cut, keyed on the extract itself
      (components.md). Replaces expiresAt with buildId: a clip cannot go stale
      inside a build, and a new build starts an empty cache.
    attributes:
      - name: extractKey
        type: ExtractKey
        required: true
        unique: true
      - name: boundingArea
        type: BoundingBox
        required: true
        constraints: Canonical form.
      - name: buildId
        type: identifier
        required: true
        references: RegionalDataBuild.buildId
      - name: fetchedAt
        type: timestamp
        required: true
        constraints: The moment the clip was cut, kept under the catalogue's attribute name.
      - name: sizeBytes
        type: integer
        required: true
        min: 0
      - name: cellIds
        type: list of CellId
        required: true
        min: 1
      - name: bytes
        type: binary
        required: true
        constraints: The encoded clip exactly as served.
    entity_constraints:
      - Held in memory only; never persisted, never written to any log.
      - Holds nothing about any requester.
      - Evicted least-recently-used when the cache's total sizeBytes would exceed its ceiling (value owed to nfr-requirements).

  - name: RequesterWindow
    description: >
      The only per-requester state the service holds: a request count for one
      requester over the current limiting window, keyed on a keyed hash of the
      requester's network address.
    attributes:
      - name: requesterHash
        type: digest
        required: true
        unique: true
        constraints: >
          Keyed hash of the address with a key minted at process start and
          never written anywhere, so values cannot be matched across
          restarts or against any other record.
      - name: windowStartedAt
        type: timestamp
        required: true
      - name: requestCount
        type: integer
        required: true
        min: 0
    entity_constraints:
      - Memory-only; never persisted, never logged, never exported.
      - Never associated with a BoundingBox, an ExtractKey or a CachedExtract.
      - Discarded when the window ends and when the process ends.

  - name: ServiceCounters
    description: >
      Aggregate counters for the running service: the only measurement it
      keeps, and what B-3's egress figure is read from.
    attributes:
      - name: buildId
        type: identifier
        required: true
        references: RegionalDataBuild.buildId
      - name: extractsServed
        type: integer
        required: true
        min: 0
      - name: bytesServed
        type: integer
        required: true
        min: 0
      - name: cacheHits
        type: integer
        required: true
        min: 0
      - name: cacheMisses
        type: integer
        required: true
        min: 0
      - name: requestsLimited
        type: integer
        required: true
        min: 0
      - name: failuresByReason
        type: map of FailureReason to integer
        required: true
      - name: lastExtractBytes
        type: integer
        required: false
        min: 0
        constraints: Size of the most recently served extract; the single-street figure for B-3 is read here.
    entity_constraints:
      - Aggregate only; no per-request row exists anywhere.

  - name: FailureRecord
    description: >
      One entry in the service's failure log: what went wrong and when, never
      for whom or where.
    attributes:
      - name: occurredAt
        type: timestamp
        required: true
      - name: reason
        type: FailureReason
        required: true
      - name: status
        type: integer
        required: true
      - name: phase
        type: enumeration
        required: true
        allowed: [build-stamp, limiting, validation, coverage, cutting, encoding, unexpected]
      - name: detail
        type: text
        required: false
        constraints: Project-authored; never a dependency's native error text.
    entity_constraints:
      - Contains no network address, no BoundingBox, no ExtractKey and no value derived from any of them.

  - name: FailureReason
    description: >
      The closed failure set on the wire (Contract 1, with the amendments
      recorded in functional-spec.md).
    attributes:
      - name: value
        type: enumeration
        required: true
        allowed: [invalid_area, area_too_large, area_not_found, upstream_unavailable, rate_limited, malformed_extract, timeout, internal]
    entity_constraints:
      - malformed_extract is reserved; nothing in this Unit produces it.
```

## Summary

Three groups of things, held in three different places:

- **Built offline, shipped with the deploy, read-only at run time** —
  `RegionalDataBuild` with its `Region`s, `GridSpec`, `CoverageIndex` and
  `Cell`s. This is the regional data the proxy cuts from. It changes only
  when the weekly build runs and the service redeploys.
- **Held in memory while the service runs, gone when it stops** —
  `CachedExtract` (keyed on `ExtractKey`, which derives from the canonical
  `BoundingBox` and the `buildId`), `RequesterWindow` and `ServiceCounters`.
  Nothing in this group is persisted, and only `RequesterWindow` knows
  anything about a requester — a keyed hash, a window and a count, never a
  place.
- **Written to the failure log** — `FailureRecord`: reason, status, phase and
  time. Never an address, never a box.

The privacy obligation from `decisions.md` ADR-004 is carried by the shape
rather than by a rule alone: the entity that knows a requester
(`RequesterWindow`) has no attribute that could hold a location, and the
entities that know a location (`CachedExtract`, `Cell`) have no attribute
that could hold a requester.
