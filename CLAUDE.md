# CityLoom

Streetmix at city scale. Rust everywhere: `cityloom-server` (axum + Postgres)
and `cityloom-client` (wasm + WebGL2), sharing `cityloom-core`. Python (uv +
pytest) is the test harness and tooling only — never product code.

## Commands
- Session start: ./scripts/state-summary.sh
- Single gate: just verify (exit 0 = done)
- Gate incl. integration + e2e: just verify-full (needs `just db`)
- Build: just build
- Format: just format
- One-time toolchain: just setup
- Postgres: just db
- Migrations: just migrate
- Ready tasks: ./scripts/next-tasks.sh
- Feature coverage: just features

## Non-obvious conventions
- `just verify` is the only authority on "done". Never claim completion from a
  green sub-command; run the gate.
- Add dependencies with `cargo add` / `uv add`, never by hand-editing a
  manifest. The skeleton is deliberately dependency-free so each task pulls
  only what it needs, at current versions.
- Ids carry provenance (`Osm { id }` vs `Authored`) from the model layer up.
  Anything that creates a node or edge must set it; reconciliation depends on
  it and silently corrupts user work if it is wrong.
- The map is WebGL, so there is no DOM to assert against. E2E tests read
  `window.__cityloom.scene_state()`. That hook is a product surface, ships in
  release builds, and must not be removed to "clean up".
- Every document mutation goes through `apply_op`. Undo and dirty-tracking are
  built on that single choke point; a direct mutation bypasses both.
- Cross-sections are shared by reference. Editing one changes every street that
  points at it — that is intended (XS-016), not a bug. Use "make unique" to
  break the link.
- Geometry is in city-local metres. Only the projection boundary and PostGIS
  (SRID 4326) see degrees.
- Tests never touch the network. OSM input comes from checked-in fixtures.
- A feature may only be marked `[PASSING]` in docs/state/features.md when a
  test claims its id. Task 051 makes the gate enforce this.

## Known traps
- (grows by sedimentation — start empty)

## Project state
- Requirements and status: docs/state/features.md
- Last session's note: docs/state/handoff.md
- Tasks: docs/tasks/ (READY via ./scripts/next-tasks.sh)

## Session start
- ALWAYS run ./scripts/state-summary.sh before anything else.
- When wrapping meaningful work, update docs/state/handoff.md (30 lines max).
