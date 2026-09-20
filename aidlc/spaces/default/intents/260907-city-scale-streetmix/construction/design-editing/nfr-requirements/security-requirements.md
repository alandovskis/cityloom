# Security Requirements — `design-editing` (U5)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md` (functional-design, this
Unit); `requirements.md` NFR6.

## STRIDE pass

| Threat | Applicable? | Reasoning |
|---|---|---|
| Spoofing | N/A | No identity concept in this Unit. |
| Tampering | N/A | This Unit holds no persisted state and performs no I/O — `LocalDesignStore` (U7) is where a design is actually written to storage; a device-local tampering concern belongs there, not here. |
| Repudiation | N/A | No user-attributable action requiring non-repudiation; Stage 1 has no accounts. |
| Information Disclosure | N/A | This Unit's in-memory state (a `Design`, an `EditSession`) never leaves the process; nothing here transmits or logs it. |
| Denial of Service | **Yes** | An unbounded undo stack or an unbounded corridor selection could exhaust browser tab memory. See NFR6.4.1. |
| Elevation of Privilege | N/A | No privilege boundary in this Unit. |

## Detailed requirements

- **NFR6.4.1 — Bounded undo history and corridor selection size.** The
  undo stack and a single corridor's target-street set are bounded to a
  sane maximum (a stated cap, e.g. a few hundred entries/streets — the
  exact figure is a code-generation implementation detail, not a
  contract), so a pathological sequence of edits or an unreasonably
  large corridor selection cannot exhaust the browser tab's memory.
  Meets the DoS row above.
- **NFR6.3.1 — No PII or account data.** This Unit's entities (`Design`,
  `LaneEdit`, `EditSession`, `CorridorSelection`) carry no field shaped
  to hold personal or payment data — `Design.name` is free text the user
  chooses for their own street proposal, not an identity field. N/A
  beyond confirming the schema stays that way.
- **NFR6.1.1 — No secret, credential, or connection string.** This Unit
  performs no I/O of any kind; there is nothing to authenticate to and
  nothing to secure. N/A beyond restating org policy.

## Data protection

No payment data, no special-category personal data, no PII flows through
this Unit (NFR6.3) — a `Design` describes a proposed street cross-section,
never a person.

## Assumptions & Open Questions

None.

## Traceability

See `traceability.json` in this directory.
