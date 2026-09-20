# Unit Test Instructions — `street-import` (U4)

## Framework and setup

Standard `cargo test`, reusing `rust-toolchain.toml` (rustc/cargo 1.97.1).
No new test framework crate. Fixture files at
`fixtures/osm2streets/*.osm.xml` (committed by `u1-osm2streets-build`) are
read from this crate's tests via a workspace-relative path built with
`concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/osm2streets/<file>")`.

## Exact unit-scoped run command

```bash
cargo test -p cityloom-street-import
```

Scoped to only this crate's tests.

## Test scope (Standard strategy: 5-8 tests per component, 3 components)

**`ExtractFetcher` / data model (combined — the failure types are the
data model, exercised primarily through the fetcher):**
1. A successful fetch response classifies as `Ok`, not a failure.
2. A timeout (exceeding the budget) classifies as
   `ImportFailure { reason: timeout, retryable: true }` (BR1.1).
3. A `429`/`503`-shaped response classifies as
   `upstream_rate_limited`/`upstream_unavailable`, both `retryable: true`.
4. A `400`/`404`-shaped response classifies as
   `area_too_large`/`area_not_found`, both `retryable: false` (BR2.1).
5. A retry after a transient failure that succeeds returns the imported
   result, not a synthetic blank one (AC7.2.2).
6. `ImportFailure`'s `street_name` matches the name passed in at
   construction, never a bare identifier (AC7.1.1).
7. No variant of `ImportFailure` can be constructed carrying raw
   dependency error text — only the closed `FailureReason` plus a
   project-authored `detail` (AC7.1.2, BR3.1).

**`StreetImportAdapter` (business logic, fixture-driven):**
8. `well-tagged-street.osm.xml` converts to a `Street` whose lanes are
   entirely `Mapped` provenance, in left-to-right order with type,
   direction, and width populated (BR4.1, AC3.1.1).
9. `thinly-tagged-street.osm.xml` converts with at least one lane carrying
   `Inferred` provenance (BR4.1, `team.md`'s provenance-test floor).
10. `one-way-street.osm.xml` converts with every lane attributed to a
    single travel direction.
11. `street-with-cycleway.osm.xml` converts without conflating the road
    way and the separately-mapped cycleway way.
12. `one-intersection.osm.xml` converts to a `StreetNetworkGraph`
    connecting the split segments at the shared node.
13. A synthetic malformed byte sequence (not a fixture — built inline)
    classifies as `ImportFailure::malformed_extract` without an uncaught
    panic escaping the adapter (SD-1's `catch_unwind` boundary).
14. `well-tagged-street.osm.xml`'s lane count and order match the fixture
    exactly (AC3.1.4) — asserted against literal expected values recorded
    from the fixture's own tags, not re-derived from the fixture at test
    time.

**`CorrectionOverlay` (business logic):**
15. A `Correction` constructed for a lane is keyed by
    `(osm_way_id, bounding_node_ids, lane_discriminator)`, never a
    positional index (BR6.1, AC7.3.4).
16. A `Correction`'s value carries no `provenance` field in its type at
    all — structural UserSet (BR5.1, AC7.3.1).
17. Reconciliation with a matching fingerprint yields
    `reapplied_silently` (AC7.5.2).
18. Reconciliation with a changed fingerprint but a still-resolving lane
    yields `reapplied_with_notice` (AC7.5.3).
19. Reconciliation where the lane no longer resolves yields `unresolved`,
    and the `Correction` is retained (not discarded) with
    `state: unresolved` (AC7.5.4).

19 tests total (7 for ExtractFetcher/data model, 7 for StreetImportAdapter,
5 for CorrectionOverlay), within the Standard strategy's 5-8-per-component
band across the three components this Unit owns.

## Coverage target

80% line coverage floor — this crate is `team.md`'s explicitly named "the
osm2streets adapter module," one of the five crates/modules the workspace
`cargo-llvm-cov` invocation measures directly. Branch coverage on this
crate is **reported, not gated**, per that same testing posture, until a
real number exists at a later stage to set a floor against.

## Mocking/stubbing guidance

`ExtractFetcher`'s transport is injectable (a trait or a constructor
parameter) so tests 1-7 run without a live network call — never mock
`gloo-net` internals directly; wrap the boundary in a small trait this
crate owns and mock that. `StreetImportAdapter`'s tests run osm2streets
for real against the committed fixture bytes — never mocked, per
`team.md`'s "characterised, not trusted" testing posture; the whole point
of these tests is exercising the real pinned dependency.

## Test data management

Fixture bytes come from the five committed files at
`fixtures/osm2streets/`; expected lane counts/provenance values for tests
8-14 are recorded as literal constants in the test module (read from the
fixture's own OSM tags by the test author, not computed from the fixture
at test time — computing them from the same input the test asserts
against would make the test tautological). Tests 1-7 and 15-19 construct
synthetic values inline; no external test-data files needed for them.
