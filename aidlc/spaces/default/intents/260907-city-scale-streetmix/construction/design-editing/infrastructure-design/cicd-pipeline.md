# CI/CD Pipeline — `design-editing` (U5)

_Confirmed._

Upstream inputs: `security-design.md`, `logical-components.md`
(nfr-design, this Unit); `components.md`; `contract-summary.md`.

This Unit is `library`-kind, embedded — a crate in the client Cargo
workspace, like `street-core` (U2) and `street-import` (U4) before it.
It adds no new pipeline stage, deployment target, or infrastructure
resource.

## What this Unit adds to the existing pipeline

| Stage | What changes for this Unit |
|---|---|
| `cargo build --workspace` (fast tier) | Now also builds `cityloom-design-editing`. No new step. |
| `cargo clippy --workspace --all-targets -- -D warnings` (fast tier) | Now also lints this crate. No new step. |
| `cargo fmt --all -- --check` (fast tier) | Now also checks this crate's formatting. No new step. |
| `cargo test --workspace` (fast tier) | Now also runs this crate's tests (fixed at `code-generation`). No new step. |
| `cargo-llvm-cov` coverage floor (fast tier) | This crate IS `team.md`'s "the editing state machine," one of the five explicitly named measured crates/modules — its tests are measured against the same 80% floor as the others, no new named target needed. |
| Keyboard-path and accessibility tests (slow tier) | This Unit's editing operations (select, change type, change width, extend, undo — `team.md`'s named five editing actions) are what the keyboard/accessibility test suite in `client-surfaces` (U6) exercises the model layer of; this Unit does not itself run browser tests, since it renders nothing, but its API is what makes those tests possible without a drag interaction. No new CI step here — U6 owns that step. |

## Deployment

No new deployment step. This crate compiles into the same WASM artifact
`client-surfaces` produces.

## Rollback

Not applicable independently — a regression here is rolled back with the
whole client deployment, same as any other workspace crate.

## Secrets management

None — this Unit performs no I/O.

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
