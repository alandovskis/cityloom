# Handoff — 2026-09-20

## Where Construction actually is
Unit-major iteration through the 12 Units in `unit-of-work.md`. Fully done
(functional-design → nfr-requirements → nfr-design → infrastructure-design
→ code-generation, code in `crates/`): `street-core`, `street-import`,
`design-editing`, `design-payload-spec` (as `cityloom-design-payload`),
`osm-extract-proxy`, `osm2streets-build`, and now **`design-storage`**
(`crates/cityloom-design-storage/`, 46 tests, 96% line coverage; review
iteration 2 READY; `aidlc-state.ts unit complete` recorded).
Remaining, not started: `accounts-sharing`, `local-persistence`,
`meeting-output`, `client-surfaces`, `data-rights`. The engine is already
pointed at `accounts-sharing`'s functional-design.

## Known tool bug — do not re-diagnose, just work around it
`.claude/tools/aidlc-sensor-traceability.ts`'s unit-matching (`unitIdMap`/
`tokenPresent`) never matches the `u<N>-<slug>` directory convention used in
`unit-of-work.md` (e.g. `u10-design-storage` vs. bare `design-storage`), so
the `traceability` sensor fails "no story-to-unit mappings" for every unit,
every time. User decision (2026-09-19): override the sensor per-unit rather
than fix the tool now. The real content gaps it also surfaces (e.g.
NFR7.4.2 for design-storage) were still fixed for real — verify each
flagged gap against the reviewer's own findings before assuming it's just
the tool bug.

## Watch out next session
- `next` on session start replays 4 `load-steering` parts before the
  `run-stage` directive — expected, not an error.
- `stage_validity` reports `drifted` (rough-mockups stale; approval-handoff,
  refined-mockups, delivery-planning need revalidation) — advisory only,
  routing continues; a redo has not been requested.
- This harness has no `TaskUpdate`/`TodoWrite` tool, so `aidlc-state.md`'s
  top-level `Current Stage` field does not auto-sync per protocol §4 — it
  reads stale. Don't trust it; trust `next`'s returned directive instead.
- Construction Autonomy Mode is autonomous (confirmed via audit trail);
  each Unit still gets a real reviewer pass, just no human gate between
  stages within a Unit.

## Suggested next task
Continue `next` for `accounts-sharing` functional-design, or ask the user
whether to keep running the remaining 5 Units unattended.
