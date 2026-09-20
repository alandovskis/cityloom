# CI/CD Pipeline — `street-core` (U2)

_Confirmed._

This Unit adds to `osm-extract-proxy`'s existing CI tiers, same as
`osm2streets-build` did — no new pipeline. It produces no deployable
artifact of its own; it is compiled into whichever crate depends on it
(ultimately the client WASM bundle).

## Build stages this Unit adds to

| Stage | Tier | What this Unit contributes |
|---|---|---|
| Build | Fast | `cargo build --workspace --locked` covers this crate once it exists as a workspace member |
| Lint | Fast | `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check` cover this crate; `#![forbid(unsafe_code)]` (`nfr-design` SD-3) is a compile-time check, not a separate CI step |
| Test | Fast | `cargo test --workspace` covers this crate's unit tests |
| Coverage | Fast | This crate is explicitly named in `team.md`'s coverage-floor set ("the street/lane domain model crate"), so its lines count toward the workspace's 80% floor |

## Deployment strategy

Not applicable — no deployable artifact.

## Rollback

Not applicable at this Unit's level — ordinary version control.

## Secrets management

None required.

## Traceability

See `traceability.json` in this directory.
