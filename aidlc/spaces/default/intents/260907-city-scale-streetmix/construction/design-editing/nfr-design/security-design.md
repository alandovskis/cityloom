# Security Design — `design-editing` (U5)

_Confirmed._

Upstream inputs: `security-requirements.md`, `tech-stack-decisions.md`
(nfr-requirements, this Unit); `functional-spec.md` (functional-design,
this Unit).

## Design elements

- **SD-1 — Bounded undo stack and corridor selection.** `EditSession`'s
  `undo_stack` is a fixed-capacity ring buffer (or an explicitly truncated
  `Vec`) with a stated maximum entry count; pushing beyond capacity drops
  the oldest entry rather than growing unbounded. `CorridorSelection`'s
  `target_streets` is bounded by the connected-street set `CorridorPlanner`
  computes, which is itself bounded by the real street network's
  intersection topology (a real corridor has a finite, small number of
  connected streets) — no separate cap is needed here beyond what the
  network graph itself already bounds. Meets NFR6.4.1.
- **SD-2 — No PII-shaped fields.** Confirmed by inspection of
  `entities.md`: every field in `Design`, `LaneEdit`, `EditSession`,
  `UndoEntry`, `CorridorSelection` describes street/lane geometry or
  editing-session bookkeeping, never a person. Meets NFR6.3.1.
- **SD-3 — No secret to manage.** Confirmed — this Unit performs no I/O.
  Meets NFR6.1.1.

## Threats not applicable here

See `security-requirements.md`'s STRIDE table — Spoofing, Tampering,
Repudiation, Information Disclosure, and Elevation of Privilege all
remain N/A for the reasons stated there; this Unit's only real design
concern is bounding in-memory growth (SD-1).

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
