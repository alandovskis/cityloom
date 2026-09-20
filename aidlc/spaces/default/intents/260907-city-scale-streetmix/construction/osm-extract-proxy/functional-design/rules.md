# Business Rules — `osm-extract-proxy` (U9)

Upstream inputs: `unit-of-work.md` and `unit-of-work-story-map.md`
(units-generation), `requirements.md` (requirements-analysis), `components.md`
(domain-design), `contract-summary.md` (contract-design),
`functional-design-questions.md` (this stage).

Every rule here is one decision the proxy makes. The `source` field names the
requirement, story criterion, decision record or this stage's question the
rule follows from. Numbers marked *owed* are values `nfr-requirements` sets;
this stage fixes the rule and its shape, not the figure — the same treatment
contract-design Q6 gave the timeout.

## Rules

```yaml
rules:
  # BR1 — Build stamp
  - id: BR1.1
    statement: "A request whose x-client-build header does not name the build the server is running is answered 426 with the server's current build, before anything else is examined."
    category: policy
    applies_to: "every request to the extract endpoint"
    trigger: "a request arrives"
    logic: "IF the request carries no x-client-build, or its value differs from the running server build, THEN answer 426 ReloadRequired carrying the current build and stop."
    violation: "A stale client would receive bytes cut for a contract it may not understand."
    source: "contract-summary.md Contract 1 (426), Versioning policy"

  # BR2 — Requester limiting
  - id: BR2.1
    statement: "Each requester is allowed at most a fixed number of requests per window, counted on a keyed hash of the requester's network address; the limit and the window length are owed to nfr-requirements."
    category: constraint
    applies_to: "every request that passed BR1.1"
    trigger: "a request arrives from a requester"
    logic: "IF the requester's RequesterWindow count for the current window has reached the limit THEN answer 429 rate_limited and stop; ELSE increment the count and continue."
    violation: "One looping or abusive client can run the egress bill and starve every other user (NFR5.1)."
    source: "NFR5.1; functional-design-questions.md Q4"
  - id: BR2.2
    statement: "Cache hits count against the requester's limit exactly as misses do."
    category: policy
    applies_to: "BR2.1"
    trigger: "a request is served from the cache"
    logic: "IF the request is answered from CachedExtract THEN it still consumed one unit of the requester's window."
    violation: "Egress scales with bytes served, cached or not, so exempting hits would leave the cost unbounded."
    source: "NFR5.1; functional-design-questions.md Q2, Q4"
  - id: BR2.3
    statement: "The hash key that turns an address into a requesterHash is minted at process start and exists nowhere but in that process."
    category: constraint
    applies_to: "RequesterWindow"
    trigger: "the service starts"
    logic: "IF the process starts THEN a fresh random key is created in memory; it is never written to configuration, storage or a log."
    violation: "A stable key would let requester hashes be matched across restarts or against other records, which is what makes them identifying."
    source: "decisions.md ADR-004; functional-design-questions.md Q4"

  # BR3 — Request validation
  - id: BR3.1
    statement: "The bbox parameter must read as four decimal numbers in WGS84 order (min longitude, min latitude, max longitude, max latitude), each within range, with each minimum strictly below its maximum; otherwise the request is invalid."
    category: validation
    applies_to: "ExtractRequest bbox"
    trigger: "a request passed BR2.1"
    logic: "IF bbox is absent, has other than four numbers, a longitude outside -180..180, a latitude outside -90..90, or a minimum not strictly below its maximum THEN answer 400 invalid_area and stop."
    violation: "An unreadable box cannot be keyed, covered or cut; guessing would serve the wrong place."
    source: "contract-summary.md Contract 1 (bbox parameter); FR2.1"
  - id: BR3.2
    statement: "A valid bbox is canonicalised — every coordinate rounded to a fixed number of decimal places — before it is used for anything, so equal areas produce equal keys."
    category: calculation
    applies_to: "BoundingBox"
    trigger: "BR3.1 passed"
    logic: "IF the box is valid THEN replace it with its canonical form and use only that form for the cell set, the extract key and the cache."
    violation: "Two requests for the same street would miss each other's cache entries and carry different keys."
    source: "components.md OsmExtractProxy (cache keyed on the extract); functional-design-questions.md Q2"
  - id: BR3.3
    statement: "A bbox may touch at most a fixed number of grid cells; the number is owed to nfr-requirements."
    category: constraint
    applies_to: "canonical BoundingBox against GridSpec"
    trigger: "BR3.2 produced a canonical box"
    logic: "IF the count of cells the box intersects exceeds the bound THEN answer 400 area_too_large and stop."
    violation: "A city-sized box would cut and serve far more than one street's data, against the egress budget (NFR5.1) and the single-street import (FR2.2)."
    source: "contract-summary.md Contract 1 (area_too_large); NFR5.1; FR2.2"

  # BR4 — Coverage
  - id: BR4.1
    statement: "A box is covered only when every cell it touches is in the build's CoverageIndex; a box touching any uncovered cell is not covered."
    category: constraint
    applies_to: "canonical BoundingBox against CoverageIndex"
    trigger: "BR3.3 passed"
    logic: "IF any touched cell is absent from coveredCellIds THEN answer 404 area_not_found with the detail \"not an area this deployment covers\" and stop."
    violation: "Serving a box that straddles the edge of the loaded regions would silently omit the far side of the street."
    source: "functional-design-questions.md Q1, Q1a; FR6.1"
  - id: BR4.2
    statement: "A covered box whose clip contains no way is answered as an area with nothing mapped in it."
    category: policy
    applies_to: "the assembled clip"
    trigger: "BR5.1 to BR5.3 produced a clip"
    logic: "IF the clip contains zero ways THEN answer 404 area_not_found with the detail \"nothing mapped here\" and stop; the empty clip is not cached."
    violation: "An empty extract would reach the adapter as a street with no lanes rather than as a stated absence (FR6.1)."
    source: "FR6.1; contract-summary.md Contract 1 (area_not_found)"

  # BR5 — Clipping
  - id: BR5.1
    statement: "The clip for a box is assembled only from the cells the box touches; no other data is read at request time."
    category: constraint
    applies_to: "clip assembly"
    trigger: "BR4.1 passed and the cache holds no entry for the key"
    logic: "IF the key is not cached THEN read exactly the touched cells that exist and assemble from them."
    violation: "Reading beyond the touched cells makes request cost depend on region size rather than box size."
    source: "functional-design-questions.md Q1a; NFR2.2"
  - id: BR5.2
    statement: "A way is included when its own bounding extent intersects the requested box; every node an included way references is included, whether or not that node lies inside the box."
    category: calculation
    applies_to: "clip assembly"
    trigger: "cells are read"
    logic: "IF a way's extent intersects the canonical box THEN include the way and all its referenced nodes; ELSE exclude it."
    violation: "A way with a missing node cannot be turned into geometry by the adapter, and the street would import as broken rather than complete (FR2.1)."
    source: "FR2.1; AC3.1.1"
  - id: BR5.3
    statement: "Each way and each node appears at most once in a clip, however many cells contributed it, and elements are written in a fixed order."
    category: constraint
    applies_to: "clip assembly"
    trigger: "cells contributed overlapping content"
    logic: "IF the same element identifier arrives from more than one cell THEN keep one copy; order the output by element type and identifier."
    violation: "Duplicates corrupt the adapter's view of the network; unstable order breaks BR5.4."
    source: "AC3.1.4; AC3.1.5"
  - id: BR5.4
    statement: "The same canonical box on the same build yields byte-identical output on every instance and every run."
    category: constraint
    applies_to: "served extract bytes"
    trigger: "a clip is encoded"
    logic: "IF two requests carry the same canonical box and buildId THEN their served bytes are identical."
    violation: "Fixtures could not be committed against the proxy's output, and the extract key would not identify one extract."
    source: "AC3.1.4; AC3.1.5; team.md Testing Posture (golden fixtures)"
  - id: BR5.5
    statement: "The served extract is encoded in the compact binary OpenStreetMap format (PBF) that osm2streets reads, not XML."
    category: policy
    applies_to: "served extract bytes"
    trigger: "a clip is encoded"
    logic: "IF a clip is served THEN it is PBF-encoded; the content type stays the contract's application/octet-stream."
    violation: "XML is several times larger for the same content, paid for on every response (NFR5.1)."
    source: "NFR5.1; functional-design-questions.md Q1 follow-up"

  # BR6 — Caching
  - id: BR6.1
    statement: "The extract key is derived from the canonical box and the buildId and from nothing else."
    category: calculation
    applies_to: "ExtractKey"
    trigger: "a canonical box is known"
    logic: "IF a key is computed THEN its inputs are exactly the canonical box and the buildId."
    violation: "A key that varied with the requester would key the cache on who asked (ADR-004) and break US7.5's fingerprint."
    source: "decisions.md ADR-004; components.md OsmExtractProxy; AC7.5.1"
  - id: BR6.2
    statement: "The cache holds successful clips only, in memory only, up to a total-bytes ceiling owed to nfr-requirements, evicting the least recently used entry first."
    category: policy
    applies_to: "CachedExtract"
    trigger: "a clip is cut, or the ceiling would be exceeded"
    logic: "IF a clip was served successfully THEN store it; IF storing would exceed the ceiling THEN evict least-recently-used entries until it fits; failures are never stored."
    violation: "Persisting the cache would need the storage NFR3.3 forbids; caching failures would repeat them."
    source: "NFR3.3; functional-design-questions.md Q2"
  - id: BR6.3
    statement: "Concurrent requests for the same extract key share one cut; the second waits for the first rather than cutting again."
    category: policy
    applies_to: "clip assembly"
    trigger: "a request arrives for a key already being cut"
    logic: "IF a cut for the key is in flight THEN attach to its result; ELSE start the cut."
    violation: "A burst for one popular street would multiply the work for no additional output."
    source: "NFR1.1; functional-design-questions.md Q2"
  - id: BR6.4
    statement: "The cache starts empty at every process start and is never carried across builds."
    category: constraint
    applies_to: "CachedExtract"
    trigger: "the service starts"
    logic: "IF the process starts THEN the cache is empty; every entry it later holds carries the running buildId."
    violation: "A clip from a superseded build would be served under a key that no longer identifies it."
    source: "functional-design-questions.md Q1b, Q2, Q3"

  # BR7 — Privacy and logging
  - id: BR7.1
    statement: "No log entry, metric label, trace or stored record may contain a requester's network address, a requested bounding box, an extract key, or any value derived from one of them."
    category: constraint
    applies_to: "every write the service makes outside its own memory"
    trigger: "anything is logged, recorded or exported"
    logic: "IF a value is an address, a box, a key, or derived from one THEN it is not written."
    violation: "A record pairing an address with a place is location data about an identifiable device (ADR-004)."
    source: "decisions.md ADR-004; NFR6.3"
  - id: BR7.2
    statement: "The failure log records reason, status, phase, time and a project-authored detail, and nothing else."
    category: constraint
    applies_to: "FailureRecord"
    trigger: "a request fails"
    logic: "IF a failure is recorded THEN exactly one FailureRecord is written with those fields."
    violation: "See BR7.1; and a dependency's native text in the record would leak past the boundary team.md seals."
    source: "decisions.md ADR-004; team.md Code Style (typed results); FR6.1"
  - id: BR7.3
    statement: "The only per-requester state is RequesterWindow, held in memory, holding no location, and discarded at window end and process end."
    category: constraint
    applies_to: "RequesterWindow"
    trigger: "always"
    logic: "IF the service holds anything keyed on a requester THEN it is a RequesterWindow and nothing else."
    violation: "Any richer requester record reopens the Stage 1 privacy question ADR-004 closed."
    source: "decisions.md ADR-004; functional-design-questions.md Q4"
  - id: BR7.4
    statement: "Measurement is aggregate only: ServiceCounters are updated per request, and no per-request row exists anywhere."
    category: constraint
    applies_to: "ServiceCounters"
    trigger: "a request completes or fails"
    logic: "IF a request completes THEN increment the relevant counters and set lastExtractBytes; no other record of the request is made."
    violation: "Per-request rows are a request log by another name."
    source: "decisions.md ADR-004; bolt-plan.md B-3 (egress figure)"

  # BR8 — Regional data build (offline)
  - id: BR8.1
    statement: "The build reads exactly the configured list of publisher sub-regions, and each downloaded file is verified against the publisher's checksum before it is used."
    category: validation
    applies_to: "Region"
    trigger: "the build runs"
    logic: "IF a downloaded file's digest does not match the published checksum THEN the build fails for that region and publishes nothing."
    violation: "A truncated or tampered download would be sliced into cells and served as the map."
    source: "functional-design-questions.md Q1, Q1a; NFR6"
  - id: BR8.2
    statement: "The filter keeps the element classes osm2streets reads — at minimum every way carrying a highway tag and every node those ways reference — and records the profile it applied in the build manifest."
    category: policy
    applies_to: "RegionalDataBuild.filterProfile"
    trigger: "a verified region file is processed"
    logic: "IF an element is in the profile THEN keep it; ELSE drop it; write the profile name into the manifest."
    violation: "Cells would either omit data the adapter needs (streets import broken) or carry data no one reads (cost and size)."
    source: "FR2.1; team.md Testing Posture (golden fixtures prove sufficiency)"
  - id: BR8.3
    statement: "Kept ways are sliced into the grid so that a way is written into every cell its geometry touches, together with every node it references."
    category: calculation
    applies_to: "Cell"
    trigger: "the filtered elements are sliced"
    logic: "IF a way's geometry intersects a cell THEN the way and all its nodes are written to that cell."
    violation: "A clip assembled from touched cells alone (BR5.1) would miss ways that cross into the box from a cell their first node is not in."
    source: "functional-design-questions.md Q1a"
  - id: BR8.4
    statement: "The build publishes a manifest naming the buildId, the build time, the GridSpec, every region with its source timestamp and digest, the CoverageIndex, and every cell with its digest; the buildId is derived from the regions' digests and the GridSpec."
    category: constraint
    applies_to: "RegionalDataBuild"
    trigger: "slicing completes"
    logic: "IF slicing succeeded for every region THEN write the manifest; a build with a missing region publishes nothing."
    violation: "Without the manifest the service cannot know what it holds, what is covered, or whether a cell is intact."
    source: "functional-design-questions.md Q1a; NFR7.2 (reproducibility)"
  - id: BR8.5
    statement: "The build runs weekly on a schedule and on demand; each run that produces a new buildId is deployed."
    category: policy
    applies_to: "RegionalDataBuild"
    trigger: "the schedule fires or the maintainer runs it"
    logic: "IF the new buildId differs from the deployed one THEN deploy; ELSE nothing changes."
    violation: "Data older than the cadence the human chose, or a redeploy that changes nothing."
    source: "functional-design-questions.md Q1b"

  # BR9 — Service readiness
  - id: BR9.1
    statement: "The service serves extracts only after it has loaded a manifest and confirmed every listed cell is present and matches its digest; until then every extract request is answered 503 upstream_unavailable."
    category: constraint
    applies_to: "service start"
    trigger: "the service starts, or a request arrives before readiness"
    logic: "IF the manifest is missing, unreadable, or any cell fails its digest THEN the service is Unready and answers 503 upstream_unavailable; ELSE it is Ready."
    violation: "Serving from a damaged or partial build would return wrong or missing streets with no failure reason."
    source: "NFR3.2; contract-summary.md Contract 1 (upstream_unavailable)"

  # BR10 — Failure mapping and timing
  - id: BR10.1
    statement: "Every failure is answered with exactly one reason from the closed set, a project-authored detail, and the status fixed for that reason; a dependency's native error text never reaches the response."
    category: constraint
    applies_to: "every failed request"
    trigger: "a request fails at any phase"
    logic: "IF a request fails THEN map the cause to one FailureReason, answer with its status (400 invalid_area, area_too_large; 404 area_not_found; 429 rate_limited; 503 upstream_unavailable, timeout, internal) and record one FailureRecord."
    violation: "US7.1's message names one member of the closed set; an unmapped or double-mapped failure has no name to show (AC7.1.1, AC7.1.2)."
    source: "FR6.1; AC7.1.1; AC7.1.2; team.md Code Style (typed results); contract-summary.md Contract 1"
  - id: BR10.2
    statement: "upstream_unavailable, rate_limited, timeout and internal are retryable; invalid_area, area_too_large and area_not_found are not."
    category: policy
    applies_to: "FailureReason"
    trigger: "a failure is classified"
    logic: "IF the reason is one of the first four THEN the client may retry; ELSE retrying returns the same answer."
    violation: "The client's retry offer (AC7.2.1) would either hide a transient failure or loop on a permanent one."
    source: "AC7.2.1; AC7.2.2; contract-summary.md Contract 1 (retryable classes)"
  - id: BR10.3
    statement: "A request that has not completed within the request budget is abandoned and answered 503 timeout; the budget is owed to nfr-requirements and must fit inside the import budget NFR1.1 sets."
    category: constraint
    applies_to: "every request"
    trigger: "the budget elapses"
    logic: "IF the budget elapses before the response starts THEN stop the work, answer 503 timeout, record the failure."
    violation: "An indefinite wait on the server would make the client's own timeout (AC3.1.6) the only guard."
    source: "AC3.1.6; NFR1.1"
  - id: BR10.4
    statement: "An unexpected fault in any phase is answered internal and recorded once; it is never allowed to end the request without a response."
    category: policy
    applies_to: "every request"
    trigger: "a fault the design did not anticipate"
    logic: "IF a phase fails for an unanticipated reason THEN answer 503 internal, record one FailureRecord with phase unexpected."
    violation: "A silent failure is not acceptable at an integration boundary."
    source: "team.md Code Style (typed results); construction phase guardrails (error handling)"
```

## Rules summary

| ID | Category | Rule in one line | Source |
|---|---|---|---|
| BR1.1 | policy | Wrong or missing client build → 426, checked first | Contract 1 |
| BR2.1 | constraint | Per-requester request limit per window, on a keyed address hash (numbers owed) | NFR5.1, Q4 |
| BR2.2 | policy | Cache hits count against the limit | NFR5.1 |
| BR2.3 | constraint | The address hash key is minted per process start, never written | ADR-004 |
| BR3.1 | validation | bbox must be four in-range numbers with min below max → else 400 invalid_area | Contract 1 |
| BR3.2 | calculation | bbox canonicalised to fixed precision before use | components.md |
| BR3.3 | constraint | bbox may touch at most N cells (owed) → else 400 area_too_large | Contract 1, NFR5.1 |
| BR4.1 | constraint | Every touched cell must be covered → else 404 area_not_found "not covered" | Q1a |
| BR4.2 | policy | A covered box with no ways → 404 area_not_found "nothing mapped" | FR6.1 |
| BR5.1 | constraint | Clip assembled from touched cells only | Q1a |
| BR5.2 | calculation | Include intersecting ways and every node they reference | FR2.1 |
| BR5.3 | constraint | Each element once, fixed order | AC3.1.4 |
| BR5.4 | constraint | Same box + same build → identical bytes | AC3.1.5 |
| BR5.5 | policy | Served bytes are PBF | NFR5.1 |
| BR6.1 | calculation | Extract key = canonical box + buildId, nothing else | ADR-004 |
| BR6.2 | policy | Memory-only cache of successes, byte ceiling (owed), LRU | NFR3.3, Q2 |
| BR6.3 | policy | One in-flight cut per key | NFR1.1 |
| BR6.4 | constraint | Cache empty at start; never crosses builds | Q1b |
| BR7.1 | constraint | Never log or store an address, a box, a key, or a derivative | ADR-004 |
| BR7.2 | constraint | Failure log fields: reason, status, phase, time, detail | ADR-004 |
| BR7.3 | constraint | RequesterWindow is the only per-requester state | ADR-004 |
| BR7.4 | constraint | Aggregate counters only, no per-request rows | ADR-004, B-3 |
| BR8.1 | validation | Configured regions only; checksum-verified downloads | Q1a |
| BR8.2 | policy | Keep what osm2streets reads; record the profile | FR2.1 |
| BR8.3 | calculation | A way goes into every cell it touches, with its nodes | Q1a |
| BR8.4 | constraint | Manifest with content-derived buildId | Q1a, NFR7.2 |
| BR8.5 | policy | Weekly scheduled build, deploy on change | Q1b |
| BR9.1 | constraint | Serve only after the manifest and cells verify; else 503 upstream_unavailable | NFR3.2 |
| BR10.1 | constraint | One reason, one status, project-authored detail | FR6.1, AC7.1.x |
| BR10.2 | policy | Retryable: upstream_unavailable, rate_limited, timeout, internal | AC7.2.x |
| BR10.3 | constraint | Request budget (owed) → 503 timeout | AC3.1.6, NFR1.1 |
| BR10.4 | policy | Unexpected fault → 503 internal, recorded once | team.md |
