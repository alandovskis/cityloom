# CI/CD Pipeline — `osm2streets-build` (U1)

_Confirmed._

This Unit adds to the existing CI tiers `osm-extract-proxy` (U9) already
established (`scripts/verify.sh` fast tier; a slow tier for release-mode
builds and `cargo audit`), rather than defining a new pipeline. It has no
deployment stage of its own — nothing about this Unit is deployed
independently; it is compiled into whichever Unit consumes it
(`u4-street-import`, ultimately `u6-client-surfaces`'s WASM bundle).

## Build stages this Unit adds to

| Stage | Tier | What this Unit contributes |
|---|---|---|
| Build | Fast | `cargo build --workspace --locked` already covers the pinned dependency once it is declared in a workspace member's `Cargo.toml` — no separate build step |
| Test | Fast | The golden-fixture suite (fixtures authored here, asserted from `u4-street-import`) runs as an ordinary `cargo test` target |
| Dependency scan | Slow | `cargo audit` (already a CI gate) now also covers `osm2streets`/`abstutil` once they resolve into `Cargo.lock` |
| Manifest check | Fast | `scripts/verify.sh`'s existing dependency-manifest check gains one row for this pinned dependency (`docs/dependencies.md`, per `security-design.md` SD-5) |

## Deployment strategy

Not applicable. This Unit produces no deployable artifact; the pinned
source becomes part of whichever downstream crate compiles against it.

## Rollback

Not applicable at this Unit's level. If the pin needs to move backward
(a regression discovered in a newer pinned commit), that is a one-line
`Cargo.toml` edit reverting `rev = "<sha>"` to the previous value —
ordinary version control, not a rollback procedure distinct from any
other source change.

## Environment promotion

Not applicable — no environment-specific behaviour exists for this Unit.

## Secrets management

None required. The fork is public and fetched over HTTPS with no
authentication.

## Traceability

See `traceability.json` in this directory.
