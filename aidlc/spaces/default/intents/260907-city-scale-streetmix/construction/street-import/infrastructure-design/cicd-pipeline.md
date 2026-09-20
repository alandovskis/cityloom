# CI/CD Pipeline — `street-import` (U4)

_Confirmed._

Upstream inputs: `security-design.md`, `logical-components.md` (nfr-design,
this Unit); `components.md`; `contract-summary.md`.

This Unit is `library`-kind, embedded — a crate in the client Cargo
workspace, not a deployed artifact of its own. It adds no new pipeline
stage, no new deployment target, and no new infrastructure resource: it is
built, tested, linted, and shipped as part of the single existing
workspace CI pipeline `osm-extract-proxy` (U9) and `street-core` (U2)
already established.

## What this Unit adds to the existing pipeline

| Stage | What changes for this Unit |
|---|---|
| `cargo build --workspace` (fast tier) | Now also builds `cityloom-street-import`. No new step. |
| `cargo clippy --workspace --all-targets -- -D warnings` (fast tier) | Now also lints this crate. No new step. |
| `cargo fmt --all -- --check` (fast tier) | Now also checks this crate's formatting. No new step. |
| `cargo test --workspace` (fast tier) | Now also runs this crate's 10 tests (round-trip + fail-closed version tests land in `code-generation`; the exact count is fixed there, not here). No new step. |
| `cargo-llvm-cov` coverage floor (fast tier) | This crate is not one of `team.md`'s five explicitly named measured crates (street/lane domain model, provenance model, **the osm2streets adapter module** — which this Unit's `StreetImportAdapter` component IS). This Unit's adapter code is therefore measured against the 80% floor as part of the existing coverage invocation; `docs/dependencies.md`/`scripts/verify.sh` do not need a new named target, since "the osm2streets adapter module" already names this Unit's component. |
| golden-fixture suite (fast/slow tier, per `team.md` Testing Posture) | This is the first Unit to actually exercise the fixture suite `u1-osm2streets-build` established: `StreetImportAdapter`'s tests (written at `code-generation`) run against the five committed OSM extract fixtures. No new CI step — the fixture-suite step `team.md` already describes now has real tests to run. |
| Release-mode WASM build (slow tier) | Already compiles the whole workspace including the pinned osm2streets crate; this Unit's code is included automatically as a new workspace member. No new step. |

## Deployment

No new deployment step. This crate compiles into the same WASM artifact
`u6-client-surfaces` (the entry point) produces and Railway serves; there
is no independent release process for a library crate.

## Rollback

Not applicable independently — a regression in this crate is rolled back
by rolling back the whole client deployment (Railway's redeploy feature,
per `team.md` Deployment), the same as any other workspace crate.

## Secrets management

None — this Unit makes an unauthenticated request (Contract 1) and holds
no secret (`security-design.md` SD-5).

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
