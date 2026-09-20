# Code Generation — Questions — `street-import` (U4)

## Plan Approval

Does this Code Generation plan, its embedded Testing Contract, and
`unit-test-instructions.md` all look correct?

Produces (workspace): new crate `crates/cityloom-street-import/`
(`Cargo.toml`, `src/lib.rs`) added to the workspace members; owns
`ExtractFetcher`, `StreetImportAdapter` (the only component permitted to
depend on `osm2streets`), and `CorrectionOverlay`. 19 tests across three
components, run against the five committed osm2streets golden fixtures
for the adapter tests. New dependencies: `gloo-net`, `gloo-timers`,
`thiserror`; reused: `osm2streets` (existing pin), `cityloom-street-core`,
`cityloom-api-types`, `serde`/`serde_json`.

[Approval Fingerprint]: sha256:b76e18ff7d828043a44350523b861ccff23f2aac255a02eaf75887f258a4cd19

- Approve Plan
- Request Changes

[Answer]: Approve Plan
