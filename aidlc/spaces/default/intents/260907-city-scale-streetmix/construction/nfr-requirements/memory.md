<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-09-20T09:00:00Z — [accounts-sharing] Caught mid-draft, not after: NFR1.2 and NFR2.2 looked like free slots but are already `requirements.md`'s own top-level items (client editing-feedback latency, browser memory) — used NFR1.5/NFR2.4 instead (next unused second-level slot after `design-storage`'s NFR1.4/NFR2.3). Re-affirms the 2026-09-12 osm-extract-proxy note: always grep every sibling Unit's already-used NFRx.y numbers before assuming a slot is free, not just requirements.md's own list.
- 2026-09-20T09:00:00Z — [accounts-sharing] Session-restart survival (NFR3.2.2) is a third-level refinement of the EXISTING NFR3.2 ("deployments shall not count against availability"), not a new NFR3.x group — a session surviving redeploy is a direct consequence of NFR3.2, not a separate concern. Only the erasure-cascade requirement needed a genuinely new group (NFR3.5).
- 2026-09-12T21:12:32Z — read BR7.4's 'no per-request row exists anywhere' literally: a successful request emits no log row at all, so p95 latency is measured from aggregate buckets carried in ServiceCounters; recorded as an entities.md amendment (latency histograms, uptime) rather than a per-request span log that would have been the easy default.
- 2026-09-12T20:55:11Z — derived requirement ids use a third level (NFR1.1.1 refines NFR1.1) because requirements.md already uses NFRx.y for its own inception sub-requirements, so the stage template's NFRx.y form would collide with existing ids.
- 2026-09-12T20:44:48Z — resumed osm-extract-proxy with three questions written but unanswered from the previous session; kept them and added Q4 (server HTTP stack). No stage had chosen the Rust HTTP framework, and U9 is the first of the four server Units sharing one Railway service to reach this stage, so the choice lands here and is inherited rather than re-asked per Unit.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-09-12T21:12:32Z — four security requirements (input validation, output encoding, headers, transport) refine no inception sub-requirement; numbered under a proposed NFR6.4 with a requirements.md amendment recorded, instead of inventing ids that imply an inception requirement which does not exist. The traceability sensor accepts either; the reviewer and a reader should not have to guess.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-09-12T20:55:11Z — project-owned PBF encoder over a crate: osmpbf is read-only, osm-io's writer takes only a file path and osmio has no PBF writer; one encoder used at build and request time keeps served bytes and fixture bytes from one code path, which BR5.4's byte-identity needs. Cost is a few hundred lines of protobuf plus the vendored MIT .proto files in the asset manifest.
- 2026-09-12T20:55:11Z — cell size 0.01° with a 12-cell span bound over 0.02° with a smaller bound; the area cap is the product of bound and cell area either way (about 10 km²), so smaller cells buy finer read sets and fewer straddle rejections at the price of 4x the cell count — flagged to infrastructure-design as a packing question rather than solved here.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-09-12T21:55:51Z — the osm-extract-proxy review came back READY with three findings left for the stage gate (R-01 Major: NFR6.3.1–NFR6.3.4 cite inception NFR6.3, whose text is about payment and special-category data, as their parent; R-02: T14 names an osm2streets pin this Unit does not have; R-03: the RequesterWindow amendment leaves an 'or' the stage already decided). All three are table-cell corrections, applied on Request Changes at the gate; under unit-major that gate comes after every Unit is built.
- 2026-09-12T21:12:32Z — whether team.md's cargo-llvm-cov measured set should list the proxy's request-path and build modules explicitly (NFR7.1.1 adds them for this Unit); a candidate for the Testing Posture practice at the learnings step.
