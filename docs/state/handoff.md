# Handoff

## 2026-08-29 — initializer session

Environment prepared. No product code written, by design.

**Decisions made** (they shaped the whole decomposition):
- Rust owns all product code; Python is test harness + tooling only.
- v1 is network-first: map, select a street, edit its cross-section, persist
  the city as one document.
- Hand-drawn authoring and OSM import are *peer* entry paths in v1. This is the
  expensive choice: it forces provenance-tagged ids in the core model and a
  real re-import reconciliation engine (task 025).
- Rendering is WASM to WebGL2, so `scene_state()` is the E2E assertion seam.
- No accounts in v1. Cities are addressed by slug; anyone with the URL can edit.
  Revisit before any public deployment.

**Built:** cargo workspace (3 crates, dependency-free stubs), uv/pytest harness,
`just verify` (8 stages, fail-fast, summarized output), docker-compose PostGIS,
state scripts, 270 features, 51 tasks.

**Verified:** `just verify` passes all 8 stages. Dependency graph checked — no
cycles, no dangling deps, all 270 feature ids claimed by exactly one task.

**Known gaps, deliberately left:**
- `wasm-pack` is not installed; the gate's `wasm` stage currently type-checks
  against the wasm target instead of producing a bundle. Task 002 fixes this.
- `just features` references `scripts/feature_coverage.py`, which task 051
  creates. The recipe will fail until then.
- No CI. Deliberate; nothing to run it on yet.
- ruff 0.16.5 prints a wrong file count ("58 files") on `format --check`. It
  formats the right 2 files; ignore the number.

## Next up

8 tasks are startable in parallel (no dependencies):
001, 002, 003, 004, 005, 009, 010, 021.

Suggested order if working alone: **001** (prove the gate fails when it should
— everything downstream trusts it), then **004** + **005** (the core model that
the widest part of the graph depends on).
