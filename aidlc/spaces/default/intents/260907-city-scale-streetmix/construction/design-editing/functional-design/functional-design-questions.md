# Functional Design — Questions — `design-editing` (U5)

No open design question remains for this Unit: `unit-of-work.md`,
`components.md`'s `DesignOverlay`/`EditingSession`/`CorridorPlanner`
entries, and the assigned stories (US5.1, US5.2, US5.4, US5.5,
US6.1-US6.4) together fully determine the entity model, the business
rules, and the workflows. US5.3's keyboard/touch-target acceptance
criteria are assigned to `client-surfaces` (U6) per the story map.

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifacts?

Produces: `entities.md` (10 types — `Design`, `LaneEdit`, `EditOutcome`,
`EditFinding`, `EditSession`, `UndoEntry`, `CorridorSelection`,
`FitState`, `CorrespondenceResult`, `CorridorApplyOutcome`, plus external
references to `street-core` and `street-import` types), `rules.md` (11
rules — synchronous editing, width validation, provenance-on-edit,
added-lane anchoring, layer-distinguished delta, undo semantics,
correspondence rule, four-state fit assessment, no-scaling-to-fit,
non-atomic bulk apply, network-graph connectivity), `functional-spec.md`
(6 workflows), and `traceability.json` (34 ACs across US5.1, US5.2,
US5.4, US5.5, US6.1-US6.4; several marked N/A as keyboard/rendering
concerns owned by `client-surfaces`).

- Looks correct
- Request changes

[Answer]: Looks correct
