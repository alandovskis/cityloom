# Restriction slice (`src/restriction/`)

Loaded when working in this folder; the root `CLAUDE.md` has the project-wide rules.

- `model.rs`: outline in lon/lat, exact on micro-degrees (`contains`, `valid`, `midpoint`, `inside`); four `Rule`s and `conflicts` as `Said`; `Restriction`, `Finding`. `draw.rs`: pure draw state machine.
- `store.rs`: `RestrictionStore` over `cityloom-restrictions` (version 1, `next_id`). Future version left alone; damaged copied to `cityloom-restrictions-damaged`; unreadable records, unknown fields and other keys kept as-is; non-integer `version` is a newer build's; each write re-reads and changes one record.
- A street is inside when its midpoint is; only loaded streets count. No dependence on tiles or place keys.
- Car-free forbids exactly `catalogue::CAR_KINDS` (travel, parking, loading; bus allowed), a subset of `MOTOR_KINDS`, so a pedestrian street never conflicts.
- Speed limit: street `Street::speed_kmh()` above the limit breaks it (counted, map-tinted, Checks row `restriction-check-speed-over`). No speed: listed as `restriction-check-speed-unknown` (`Finding.speed_unknown`), never breaks. Rule `SPEEDS` (10-70 km/h, `u8`) differ from `SPEED_CHOICES`.
- Street page Checks: a row per area holding the street (`StreetVm::area_checks`).
- `AreasVm` refuses an invalid ring (`model::valid`) before add/update; the store does not.
- Known limits: street page reads areas once on open (stale until reload); `Storage` cannot signal a failed read, so it looks like an empty document and a later write would replace it.
