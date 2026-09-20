# Business Rules — `design-editing` (U5)

_Confirmed._

Upstream inputs: `entities.md` (this stage), `components.md`, `stories.md`
US5.1, US5.2, US5.4, US5.5, US6.1-US6.4.

```yaml
rules:
  # BR1 — Synchronous editing, no network
  - id: BR1.1
    statement: "Every editing operation returns the updated model within the same synchronous call and issues no network request."
    category: constraint
    applies_to: EditingSession
    trigger: "Any editing action (change, add, remove, undo)"
    logic: "IF an editing action is invoked THEN it returns before any await point; no code path in this Unit performs I/O."
    violation_behaviour: "AC5.1.2 states this directly as the property that protects the (deferred) 100ms interaction budget — free to test today, and it fails loudly the moment an await appears in the edit path."
    source: "AC5.1.2; components.md EditingSession behaviour"

  # BR2 — Width validation
  - id: BR2.1
    statement: "A width of zero, negative, or greater than 20 metres for a single lane is rejected with a stated reason, and the previous value is retained."
    category: validation
    applies_to: LaneEdit
    trigger: "A width change"
    logic: "IF new_width <= 0 OR new_width > 20.0 THEN return EditOutcome{applied: false, findings: [reason]} and the lane's width is unchanged."
    violation_behaviour: "AC5.1.5. The 20m ceiling is a sanity bound catching typos/unit confusion, not a jurisdiction-specific design standard — it must never be tightened toward any single jurisdiction's real lane widths (project.md TC-5)."
    source: "AC5.1.5"

  # BR3 — Provenance on edit
  - id: BR3.1
    statement: "Changing a lane's width in the design layer sets that value's provenance to UserSet for as long as the changed value stands; reverting returns the value to what it was before the edit, carrying that value's own provenance (Mapped again if the pre-edit value was Mapped)."
    category: constraint
    applies_to: LaneEdit
    trigger: "A width or attribute change, or a revert"
    logic: "IF an edit changes attribute X THEN X's Dimension.provenance becomes UserSet; IF the edit is reverted THEN X returns to its pre-edit value AND that value's own provenance, never forcing UserSet on a reverted value."
    violation_behaviour: "AC5.1.3, AC5.4.3. Provenance describes the value, not the history of the field — a reverted mapped width must read Mapped again, not UserSet."
    source: "AC5.1.3; AC5.4.3"

  # BR4 — Added-lane anchoring
  - id: BR4.1
    statement: "A lane added by the user is stored against an anchor relative to a keyed baseline lane (left-of/right-of a named neighbour, or an offset from a named edge), never a position index, and returns to that same anchor on reload."
    category: constraint
    applies_to: LaneEdit
    trigger: "Adding a lane"
    logic: "IF a LaneEdit has kind=add THEN it carries an anchor and no target_lane_discriminator; ON reload THEN the lane is re-inserted at the position the anchor resolves to against the current baseline, not a stored index."
    violation_behaviour: "AC5.2.3, AC5.2.4. 'The same position' has no meaning once positional indices are forbidden (team.md Corrections) — an added lane's identity is its anchor, not a slot number."
    source: "AC5.2.3; AC5.2.4"

  # BR5 — Layer-distinguished delta
  - id: BR5.1
    statement: "The three layers — imported baseline, corrections, proposed design — are always distinguished from one another when asked what changed; a design with no edits reports nothing has changed, never an empty comparison."
    category: policy
    applies_to: Design
    trigger: "A 'what changed' query"
    logic: "IF the design has zero LaneEdits THEN report 'nothing has changed'; ELSE report each of the three layers separately, never merged into one undifferentiated diff."
    violation_behaviour: "AC5.4.1, AC5.4.2. Merging corrections and design edits into one delta would hide that a correction states a measurement was wrong while a design edit proposes a change to a correct one."
    source: "AC5.4.1; AC5.4.2"

  # BR6 — Undo semantics
  - id: BR6.1
    statement: "Undo reverses the most recently completed action and returns the model to its immediately preceding state; a corridor apply is reversed as a single unit across every street it touched; with nothing to undo, undo changes nothing and says so."
    category: policy
    applies_to: EditSession, UndoEntry
    trigger: "Undo invoked"
    logic: "IF undo_stack is non-empty THEN pop the most recent UndoEntry and restore prior_values on every street in affected_streets as one atomic operation; IF undo_stack is empty THEN return EditOutcome{applied: false, findings: ['nothing to undo']}."
    violation_behaviour: "AC5.5.1, AC5.5.2, AC5.5.3. A corridor apply reversed street-by-street rather than as one unit would leave a corridor half-undone, which is a worse state than not undoing at all."
    source: "AC5.5.1; AC5.5.2; AC5.5.3"

  # BR7 — Correspondence rule
  - id: BR7.1
    statement: "Extending a design onto a target street matches the design's lane changes onto the target's lanes by lane type and ordinal-from-kerb, counting from the same edge the carriageway definition uses; a lane the design does not speak to is left as the target had it; a lane the rule cannot match is left unchanged and named as unmatched."
    category: policy
    applies_to: CorrespondenceResult
    trigger: "Extending a design onto a target street"
    logic: "IF a design LaneEdit's (lane_type, ordinal_from_kerb) matches a target lane THEN apply the edit to that lane; ELSE add the LaneEdit's id to unmatched and leave the target lane untouched; the target's own corrections and any lane the design says nothing about survive unconditionally."
    violation_behaviour: "AC6.1.2, AC6.1.3. Wholesale replacement of the target's lane list is explicitly not what 'apply' means here — a target's corrected baseline must survive an extension applied to it."
    source: "AC6.1.2; AC6.1.3; AC6.1.4"

  # BR8 — Fit assessment, four states
  - id: BR8.1
    statement: "Fit assessment reports exactly one of four states — fits, does not fit (with shortfall), could not be checked (target width wholly inferred or absent), or not yet checked — using total lane width alone against the carriageway definition, with no lane-type compatibility rule."
    category: validation
    applies_to: FitState
    trigger: "Checking whether a design fits a target street"
    logic: "IF target.carriageway_width is absent OR every contributing lane width has Inferred provenance THEN status = could_not_be_checked; ELSE IF design.total_width > target.carriageway_width THEN status = does_not_fit (shortfall = design.total_width - target.carriageway_width); ELSE status = fits. A target not yet evaluated is not_yet_checked, never silently omitted."
    violation_behaviour: "AC6.2.1-AC6.2.6. Reporting a default-versus-default comparison as a real finding would present a guess as a measurement (raid-log.md R-2's named harm); an unchecked street silently dropped from the report is worse than one explicitly named unchecked (AC6.2.5)."
    source: "AC6.2.1; AC6.2.2; AC6.2.3; AC6.2.4; AC6.2.5; AC6.2.6"

  # BR9 — No scaling to fit
  - id: BR9.1
    statement: "A design applied to a street it does not fit is applied unchanged; lanes are never scaled to fit."
    category: constraint
    applies_to: LaneEdit
    trigger: "Applying a design the user chose to proceed with despite a does_not_fit warning"
    logic: "IF the user proceeds past a does_not_fit warning THEN apply every LaneEdit's stated value verbatim; no width is computed or adjusted to make the total fit."
    violation_behaviour: "AC6.2.2. A scaled width is neither Mapped, Inferred nor UserSet — it would produce a value with no valid provenance state, breaking the model the product's credibility rests on."
    source: "AC6.2.2"

  # BR10 — Bulk apply is not atomic
  - id: BR10.1
    statement: "A corridor apply is not atomic: a street that cannot be prepared does not roll back streets that succeeded, and every street's individual outcome (succeeded, warned, could-not-be-checked, failed-with-reason) is reported."
    category: policy
    applies_to: CorridorApplyOutcome
    trigger: "A corridor (bulk) apply"
    logic: "IF applying to street S fails THEN record S in failed with a reason and continue applying to the remaining target streets; the final CorridorApplyOutcome names every street in exactly one of succeeded/warned/could_not_be_checked/failed."
    violation_behaviour: "AC6.4.2, AC6.4.3. An all-or-nothing bulk apply would let one street's failure discard nine successful ones for no benefit — the failure of one street does not roll back the others."
    source: "AC6.4.2; AC6.4.3; AC6.4.4"

  # BR11 — Connectivity from the network graph
  - id: BR11.1
    statement: "The connected-street set offered for corridor selection comes from the street network's own intersection topology, never from a coarse display pass, and every street in it is identified by something stable and human-readable even when the source data carries no name."
    category: constraint
    applies_to: CorridorSelection
    trigger: "Requesting the connected-street set for a source street"
    logic: "IF the connected set is computed THEN it is derived from street-core's StreetNetworkGraph adjacency, never from a basemap tile layer or any coarse/display-only pass; IF a connected street has no name in the source data THEN it is identified by a stable, human-readable fallback (per AC2.2.2's map-selection identification rule)."
    violation_behaviour: "AC6.3.1, AC6.3.2, AC6.3.3. A coarse display pass yields no real connectivity at all — using one here would silently produce an empty or wrong corridor set."
    source: "AC6.3.1; AC6.3.2; AC6.3.3"
```

## Summary

| Rule | Category | What it protects |
|---|---|---|
| BR1.1 | Synchronous editing | The interaction budget is protected by construction, not by measurement |
| BR2.1 | Width validation | Typos and unit confusion are caught without encoding jurisdiction rules |
| BR3.1 | Provenance on edit | A reverted value's provenance reflects the value, not the edit history |
| BR4.1 | Added-lane anchoring | An added lane survives a re-import or reload at the same conceptual position |
| BR5.1 | Layer-distinguished delta | A correction is never confused with a proposed change |
| BR6.1 | Undo semantics | A corridor apply is reversible as one decision, matching how it was made |
| BR7.1 | Correspondence rule | Extending a design never wholesale-replaces a target's own data |
| BR8.1 | Fit assessment | A guess is never reported as a measured fact |
| BR9.1 | No scaling to fit | Every width in the model stays in a valid provenance state |
| BR10.1 | Bulk apply not atomic | One failing street never costs nine successful ones |
| BR11.1 | Connectivity from network graph | A corridor is a real route, never an artifact of the display pass |

## Traceability

See `traceability.json` in this directory.
