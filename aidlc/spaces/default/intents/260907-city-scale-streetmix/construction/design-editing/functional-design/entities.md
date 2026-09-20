# Entities — `design-editing` (U5)

_Confirmed._

Upstream inputs: `unit-of-work.md` U5, `components.md` (`DesignOverlay`,
`EditingSession`, `CorridorPlanner`), `stories.md` US5.1, US5.2, US5.4,
US5.5, US6.1-US6.4.

This Unit owns the proposed change and the machine that applies it: lane
edits, additions and removals held as a proposal over the baseline
(`street-core`, U2) and its corrections (`street-import`, U4); selection
and undo state; and corridor work (connectivity, fit assessment,
correspondence, bulk apply). It does not own the controls that issue
these actions (`client-surfaces`, U6).

```yaml
entities:
  - name: Design
    description: >
      What the user wants a street (or set of streets) to become, held
      separately from what it is. Makes no claim about reality, so it
      carries no import fingerprint — a proposal cannot go stale against
      OpenStreetMap the way a Correction can.
    attributes:
      - name: design_id
        type: string
        required: true
      - name: name
        type: string
        required: true
      - name: streets
        type: array
        required: true
        constraints: ["the street keys (osm_way_id + bounding_node_ids) this design targets"]
      - name: created_at
        type: string
        required: true
      - name: updated_at
        type: string
        required: true

  - name: LaneEdit
    description: >
      A proposed change: an existing lane's attribute changed, a lane
      added that exists in no import, or a lane removed. Kind
      distinguishes the three; an addition carries an anchor instead of
      a target lane discriminator, since it has no baseline lane to key
      by (AC5.2.3).
    attributes:
      - name: edit_id
        type: string
        required: true
      - name: design_id
        type: reference
        required: true
        references: Design
      - name: target_street
        type: reference
        required: true
        constraints: ["street key: osm_way_id + bounding_node_ids"]
      - name: target_lane_discriminator
        type: string
        required: false
        constraints: ["absent for an addition (AC5.2.1); present for a change or removal"]
      - name: anchor
        type: object
        required: false
        constraints: ["present only for an addition — position relative to a keyed baseline lane: left-of/right-of a named neighbour, or an offset from a named edge (AC5.2.3)"]
      - name: kind
        type: enum
        required: true
        allowed_values: [change, add, remove]
      - name: attribute
        type: enum
        required: false
        allowed_values: [laneType, width, direction]
      - name: value
        type: string
        required: false
      - name: width
        type: reference
        required: false
        references: "Dimension (street-core, external) — UserSet provenance once edited (AC5.1.3)"

  - name: EditOutcome
    description: >
      What every editing operation returns, instead of throwing
      (`team.md` Code Style; `components.md` EditingSession). Carries
      both the result and any findings the caller must render before or
      instead of applying.
    attributes:
      - name: applied
        type: boolean
        required: true
      - name: findings
        type: array
        required: false
        references: EditFinding

  - name: EditFinding
    description: >
      A single reason an edit was rejected or a warning attached to one
      (AC5.1.5's width-range rejection is the canonical example).
    attributes:
      - name: reason
        type: string
        required: true
      - name: detail
        type: string
        required: false

  - name: EditSession
    description: >
      The editing state machine's live state: what is selected and the
      undo history. Synchronous — every action returns the updated model
      within the same call, issuing no network request (AC5.1.2).
    attributes:
      - name: session_id
        type: string
        required: true
      - name: selected_street
        type: reference
        required: false
        constraints: ["street key; absent when nothing is selected"]
      - name: selected_lane_discriminator
        type: string
        required: false
      - name: undo_stack
        type: array
        required: true
        references: UndoEntry

  - name: UndoEntry
    description: >
      One reversible action. `affected_streets` has more than one entry
      only for a corridor apply, so undo reverses the whole apply as a
      single unit (AC5.5.2).
    attributes:
      - name: undo_entry_id
        type: string
        required: true
      - name: action
        type: string
        required: true
      - name: affected_streets
        type: array
        required: true
      - name: prior_values
        type: object
        required: true
        constraints: ["enough state to reverse the action exactly — the value(s) each affected attribute held immediately before this action"]

  - name: CorridorSelection
    description: >
      A source street, the target streets chosen from its connected set,
      and the per-target fit state, computed from the network graph's
      own intersection topology (`street-core`'s `StreetNetworkGraph`),
      never from a coarse display pass.
    attributes:
      - name: selection_id
        type: string
        required: true
      - name: source_street
        type: reference
        required: true
      - name: target_streets
        type: array
        required: true
      - name: fit_states
        type: object
        required: true
        constraints: ["map of target street key -> FitState"]
      - name: applied_at
        type: string
        required: false

  - name: FitState
    description: >
      Four states, never two (AC6.2.1-AC6.2.5) — a target's carriageway
      width is compared to the design's total lane width using the
      carriageway definition `street-core` owns, using total width alone
      with no lane-type compatibility rule (AC6.2.6).
    attributes:
      - name: status
        type: enum
        required: true
        allowed_values: [fits, does_not_fit, could_not_be_checked, not_yet_checked]
      - name: shortfall
        type: reference
        required: false
        constraints: ["present only when status is does_not_fit — the width by which the design exceeds the target's carriageway (Dimension, street-core, external)"]

  - name: CorrespondenceResult
    description: >
      The outcome of matching a design's lane changes onto a target
      street's lanes by type and ordinal-from-kerb (AC6.1.2, AC6.1.3).
    attributes:
      - name: matched
        type: array
        required: true
        constraints: ["LaneEdit ids successfully matched onto a target lane"]
      - name: unmatched
        type: array
        required: true
        constraints: ["LaneEdit ids the correspondence rule could not match — the target keeps what the design does not speak to"]

  - name: CorridorApplyOutcome
    description: >
      The per-street result of a bulk apply (AC6.4.2, AC6.4.3) — never
      atomic: a street that cannot be prepared does not roll back the
      others.
    attributes:
      - name: succeeded
        type: array
        required: true
      - name: warned
        type: array
        required: true
        constraints: ["target streets whose FitState was does_not_fit but the design was applied anyway (AC6.2.2)"]
      - name: could_not_be_checked
        type: array
        required: true
      - name: failed
        type: array
        required: true
        constraints: ["target streets that could not be prepared at all, with a reason each (AC6.4.3)"]
```

## Summary

Ten types. `Design`/`LaneEdit` are the proposal (`DesignOverlay`);
`EditOutcome`/`EditFinding` are the outcome-plus-findings shape every
editing operation returns; `EditSession`/`UndoEntry` are the state
machine (`EditingSession`); `CorridorSelection`/`FitState`/
`CorrespondenceResult`/`CorridorApplyOutcome` are `CorridorPlanner`'s
domain. `Street`/`Lane`/`Dimension`/`StreetNetworkGraph` are referenced
but owned by `street-core` (U2); `Correction` is referenced but owned by
`street-import` (U4).

## Assumptions & Open Questions

None — every type is named directly by `components.md`'s behaviour prose
for `DesignOverlay`, `EditingSession`, and `CorridorPlanner`, or by the
assigned stories' acceptance criteria.

## Traceability

See `traceability.json` in this directory.
