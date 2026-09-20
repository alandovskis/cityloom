# Tech Stack Decisions — `design-editing` (U5)

_Confirmed._

Upstream inputs: `functional-spec.md`, `rules.md` (functional-design, this
Unit); `components.md`; `unit-of-work.md` U5.

## Decisions

- **Language/crate**: Rust, a new workspace crate (`cityloom-design-editing`),
  compiled to `wasm32-unknown-unknown` as part of the client. Reuses the
  workspace's existing `rust-toolchain.toml`.
- **`street-core` (U2)**: a normal workspace dependency — this Unit reads
  the baseline (`Street`, `Lane`, `StreetNetworkGraph`, `Dimension`) and
  the carriageway-width definition it owns, never mutating it (the
  baseline stays behind shared references only, `team.md` Code Style).
- **`street-import` (U4)**: a normal workspace dependency — `DesignOverlay`'s
  revert semantics (BR3.1) need to resolve "the corrected baseline where
  a correction exists," which means reading `street-import`'s
  `CorrectionOverlay`/`Correction` types.
- **No dependency on `design-payload-spec` (U3) or `client-surfaces` (U6).**
  This Unit's `Design`/`LaneEdit`/etc. are its own live, in-memory
  domain types; serialization through U3's wire shape is
  `local-persistence`'s (U7) concern at the storage boundary, not this
  Unit's. U6 depends on this Unit, never the reverse.
- **No new runtime dependency beyond `street-core` and `street-import`.**
  This crate is pure in-memory domain logic and state machine code — no
  I/O, no serialization library, no HTTP client, no timer. `serde`
  derives are deliberately NOT added here; if `local-persistence` (U7)
  needs to serialize this Unit's types directly (rather than mapping
  through U3's wire shape), that dependency is added at U7's own
  `nfr-requirements`, not assumed here.
- **Error handling**: `EditOutcome`/`EditFinding` (per `entities.md`) are
  plain structs — no `thiserror`/`anyhow` needed, since editing
  operations return outcome-plus-findings, not `Result<T, E>` with an
  error trait (`team.md` Code Style: "editing operations return outcome
  plus findings — validation the UI can render before anything is
  applied — using the same shape" as the typed-failure pattern, but
  distinct from it: an edit rejection is expected user-facing feedback,
  not a system failure).

## Rationale

Keeping this crate free of any new third-party dependency matches its
stated boundary: "model and state, not surface" (`unit-of-work.md`). Every
dependency it needs already exists in the workspace from U2 and U4.

## Assumptions & Open Questions

- **[assumption]** The undo-stack and corridor-selection size caps
  (NFR6.4.1 in `security-requirements.md`) are implementation constants
  fixed at code-generation, not a contract this stage sets a specific
  number for — no user-facing behaviour depends on the exact cap value,
  only on a cap existing.

## Traceability

See `traceability.json` in this directory.
