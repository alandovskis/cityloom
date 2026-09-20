# Code Summary — `street-import` (U4)

## Files created

- `crates/cityloom-street-import/Cargo.toml`
- `crates/cityloom-street-import/src/lib.rs` — module wiring, public re-exports, `OSM2STREETS_REVISION` constant
- `crates/cityloom-street-import/src/failure.rs` — `ImportFailure` (+ `FailureReason` reused from `cityloom-api-types`), 6 tests
- `crates/cityloom-street-import/src/fingerprint.rs` — `ImportFingerprint`, `FingerprintMapConfig`, `DrivingSide`, 3 tests
- `crates/cityloom-street-import/src/correction.rs` — `Correction`, `CorrectionState`, `CorrectionReconciliationOutcome`, `ReconciliationOutcome`, `CorrectionOverlay`, 7 tests
- `crates/cityloom-street-import/src/transport.rs` — `ExtractTransport`/`TimeoutSource` ports, `TransportOutcome`, `GlooExtractTransport`, `GlooTimeoutSource` (no native unit tests — see Gaps)
- `crates/cityloom-street-import/src/fetcher.rs` — `ExtractFetcher<T, S>`, 7 tests
- `crates/cityloom-street-import/src/log.rs` — `log_locally` (SD-1/SD-2 local-only failure log)
- `crates/cityloom-street-import/src/adapter.rs` — `StreetImportAdapter`, `ImportedStreet`, all osm2streets conversion logic, 10 tests

## Files modified

- `docs/dependencies.md` — added rows for `cityloom-street-import`, `street-core`, `osm2streets`, `streets_reader`, `abstutil`, `thiserror`, `gloo-net`, `gloo-timers`.
- `docs/osm2streets-pin.md` — records that `u4-street-import` now consumes the pin, adds `streets_reader`, and documents the `abstutil` pinning-mechanism deviation (see below).

Workspace root `Cargo.toml` ended unchanged (a `[patch]` approach was tried and reverted — see Deviations).

## Key implementation decisions

- **`ImportFailure` wraps `cityloom-api-types::FailureReason`** rather than redefining the enum, so the client and the proxy (U9) cannot drift on failure vocabulary — reused directly per the plan's intent.
- **SD-1's `catch_unwind` boundary** is implemented in `adapter.rs`: every osm2streets call is wrapped so a panic inside the third-party crate degrades to one failed import (`ImportFailure::internal`) rather than aborting the whole WASM module.
- **`Correction` has no `provenance` field** — its UserSet-ness is structural, per BR5.1/BR4.5, matching `entities.md` exactly.
- **`ExtractFetcher<T, S>` is generic over an `ExtractTransport` and `TimeoutSource` port**, so its retry/timeout/classification logic (BR1.1, BR2.1) is fully unit-testable without a live network call or a browser runtime; `GlooExtractTransport`/`GlooTimeoutSource` are the thin production implementations.

## Test coverage summary

30 tests, all green (19 required + 11 additional edge-case tests):

```
$ cargo test -p cityloom-street-import
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --workspace
cityloom-api-types: 4/4, cityloom-design-payload: 10/10,
cityloom-street-import: 30/30, osm-extract-proxy: 111/111 (+1 ignored slow-tier),
street-core: 28/28 — zero failures.

$ cargo build --workspace --locked        # Finished cleanly
$ cargo clippy --workspace --all-targets -- -D warnings   # clean
$ cargo fmt --all -- --check              # clean
$ bash scripts/verify.sh                  # full gate green
$ cargo llvm-cov -p cityloom-street-import --summary-only  # 95.34% line coverage
$ cargo tree -p cityloom-street-import -i osm2streets      # exactly one resolved package
$ cargo tree -p cityloom-street-import -i abstutil         # exactly one resolved package
```

Test breakdown: 7 `ExtractFetcher` tests, 10 `StreetImportAdapter` tests (7 required + 3 extra), 7 `CorrectionOverlay` tests (5 required + 1 extra + isolation), 6 `ImportFailure` tests, 3 `ImportFingerprint` tests — 95.34% line coverage, well above the 80% floor `team.md` names this crate against as "the osm2streets adapter module."

### Real osm2streets output per fixture (verified via a throwaway spike, since deleted)

- `well-tagged-street.osm.xml` → 1 street, 6 lanes: bike lane (Backward, 1.5m, **Inferred** — no width tag), kerb buffer (Backward, 0.1m, **Inferred**), 4 driving lanes (3 Forward + 1 Backward, 4.0m each, **Mapped** — the explicit `width=16` divides across them).
- `thinly-tagged-street.osm.xml` → 1 street, 2 driving lanes, both 3.0m, both **Inferred**.
- `one-way-street.osm.xml` → 1 street, 2 driving lanes, both Forward — no split.
- `street-with-cycleway.osm.xml` → 2 separate streets (the road, 4 lanes; the trail, 2 SharedUse lanes) — never conflated.
- `one-intersection.osm.xml` → 5 streets, 6 intersections, one connecting exactly 4 streets (the real junction).

## Deviations from the plan

1. **Crate name**: the plan called the dependency `cityloom-street-core`; the actual published crate is `street-core`. Used the real name.
2. **`streets_reader` added** as a second git dependency at the same pinned commit/repo as `osm2streets` — `osm2streets` itself has no OSM XML parser; `streets_reader::osm_to_street_network` is the actual entry point. Not named in the original plan or `docs/osm2streets-pin.md`; now documented in both.
3. **`abstutil` cannot carry an explicit `rev =` in this crate's own `Cargo.toml`**, contrary to `project.md`'s literal "pinned the same way" wording. Verified empirically: an explicit rev creates a second, independent `abstutil` package that fails to type-unify with `streets_reader`'s own `abstutil::Timer`. The working form declares `abstutil` bare/unpinned (matching upstream's own form), letting Cargo unify to one package; the pin is enforced one layer down via the committed `Cargo.lock` plus `cargo build --locked`. Documented in `docs/osm2streets-pin.md`.
4. **BR2.1 vs. `cityloom-api-types::FailureReason::is_retryable()` disagree on `internal`** — the reused method implements a different rule for the proxy side. Implemented `ImportFailure::retryable` per this Unit's own BR2.1.
5. **AC3.1.1's "entirely Mapped provenance" does not hold against the real pinned dependency** for `well-tagged-street.osm.xml` — the cycleway lane and its synthesised kerb buffer are legitimately `Inferred` (no width tag exists for either in the fixture); only the 4 driving lanes (with the explicit `width=16` tag) are `Mapped`. The test asserts the true, verified per-lane provenance rather than a false "all Mapped" claim. Flagged for reconciliation between the AC wording and the real fixture/dependency behavior.
6. **AC3.1.5's release-mode-only assertion**: no genuinely release-mode-only behavior was needed; the lane-count/order test runs identically in debug and release.
7. **Correction ordinal-from-kerb** is computed as each lane's position within its own `(lane_type, direction)` group in osm2streets' own encounter order — deterministic and stable, but not a driving-side-aware physical-kerb-distance calculation (out of this Unit's design scope).

## Gaps / honest limitations

- `transport.rs`'s `GlooExtractTransport`/`GlooTimeoutSource` have 0% native test coverage — awaiting a `gloo-net`/`gloo-timers` future on a native (non-`wasm32`) target aborts the process (verified). `ExtractFetcher`'s actual logic is fully tested through the port/fake seam; these two thin Gloo wrappers have no branching logic of their own.
- `scripts/verify.sh`'s coverage gate does not yet measure `cityloom-street-import` (it currently runs `-p osm-extract-proxy -p cityloom-api-types` only) — wiring it in was outside this Unit's Step 12 scope. Flagged for a later stage.
- The `osm2streets`/`abstutil` fork (per `security-design.md` SD-3 / `u1-osm2streets-build`'s own stated status) still does not exist; both are consumed directly from upstream as an interim measure.
