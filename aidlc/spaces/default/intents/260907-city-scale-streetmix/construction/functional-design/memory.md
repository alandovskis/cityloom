<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-09-12T02:53:15Z — [osm-extract-proxy] Q3's "24 hours" is read as superseded by Q1b's weekly build cadence rather than as a cache TTL; a clip cut from a regional file cannot go stale inside a build and every deploy empties the cache, so expiresAt was dropped from CachedExtract and freshness is stated as the build cadence. The human's answer is kept verbatim in the questions file with a note.
- 2026-09-12T02:53:15Z — [osm-extract-proxy] Contract 1's failure enum is U9-owned, so this design proposes its amendments (add invalid_area, rename upstream_rate_limited to rate_limited, reserve malformed_extract, classify internal retryable) as recorded amendments rather than editing the approved contract; the wire shape is otherwise unchanged.
- 2026-09-12T02:53:15Z — [osm-extract-proxy] Coverage is strict: a box touching any uncovered cell is area_not_found, chosen over serving the covered part, because a silently truncated street is worse than a stated absence at a provincial border.
- 2026-09-20T03:20:00Z — [accounts-sharing] `components.md`'s `AccountService` entity list has no entity to hold a saved design's human-chosen name, and U10's already-built `StoredDesign` has none either. Introduced `SavedDesign` (account_id, stored_design_id, name, saved_at) at this stage to close the gap, keyed to U10's design by opaque ID only (no cross-crate FK), matching the precedent U10 itself set with `owner_account_id`.
- 2026-09-20T03:20:00Z — [accounts-sharing] BR8.1 states the authorization *decision* (grant/link check) as this Unit's job while leaving the not-found/forbidden *response* to U10's already-reviewed BR3.1, rather than re-specifying a second response rule that could drift from it.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-09-12T02:53:15Z — [osm-extract-proxy] CachedExtract loses the catalogue's expiresAt attribute and gains buildId (components.md lists expiresAt); recorded as an amendment in functional-spec.md.
- 2026-09-12T02:53:15Z — [osm-extract-proxy] Two QUESTION_ANSWERED receipts were intended (batch 1, then the Q1 follow-up); the second was refused because both batches were answered across two human turns that the first receipt consumed. The follow-up answers are in the questions file and are covered by the SUMMARY_CONFIRMATION_RECORDED digest; no retry was attempted.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-09-12T02:53:15Z — [osm-extract-proxy] Pre-cut offline cells (Q1a) over start-up slicing: a province PBF is 0.3–1.2 GB and the node lookup needed to clip it does not fit the ~$5/month memory budget; the cost is that U9 gains a build-time pipeline and a scheduled job, and that freshness becomes a deploy cadence.
- 2026-09-12T02:53:15Z — [osm-extract-proxy] Per-requester limiting on a process-keyed address hash (Q4) over no requester state: ADR-004 forbids logs and address/location pairs, not transient counters; the hash key is minted per process start so values cannot be correlated across restarts.
- 2026-09-12T02:53:15Z — [osm-extract-proxy] Serving PBF rather than XML: caching does not reduce egress (a hit still sends the bytes), so the format is the only lever on the $0.05/GB the budget worries about; osm2streets' reader accepts both.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-09-12T02:53:15Z — [osm-extract-proxy] The element-level traceability check fails closed for every Unit in this workflow: the dependency graph names Units without the u<n>- prefix while unit-of-work.md's index and the story map name only the prefixed directory, so no story maps to any Unit. Recorded as an amendment; confirm whether to fix the two inception artifacts before the next per-unit stage runs.
- 2026-09-12T02:53:15Z — [osm-extract-proxy] streets_reader/Cargo.toml in a-b-street/osm2streets carries unpinned git dependencies (abstutil, muv-osm, osm-reader) — the same pattern team.md flags for osm2streets-js. U1's pinning practice must cover all three, not only abstutil.
- 2026-09-12T02:53:15Z — [osm-extract-proxy] Whether osm2streets needs element classes beyond highway ways and their nodes (crossings, barriers, areas) is left to the golden fixtures in U1 to prove; if it does, BR8.2's minimum profile widens.
