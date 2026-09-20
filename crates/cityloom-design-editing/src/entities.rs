//! The ten types this Unit owns (`entities.md`): [`Design`], [`LaneEdit`],
//! [`EditOutcome`], [`EditFinding`], [`EditSession`], [`UndoEntry`],
//! [`CorridorSelection`], [`FitState`], [`CorrespondenceResult`], and
//! [`CorridorApplyOutcome`].

use std::collections::HashMap;

use street_core::{Dimension, Direction, StreetIdentity};

/// What the user wants a street (or set of streets) to become, held
/// separately from what it is. Carries no import fingerprint.
#[derive(Clone, Debug, PartialEq)]
pub struct Design {
    pub design_id: String,
    pub name: String,
    /// The street keys (OSM way id + bounding node ids) this design
    /// targets.
    pub streets: Vec<StreetIdentity>,
    pub created_at: String,
    pub updated_at: String,
}

/// A position for an added lane, relative to a keyed baseline lane
/// (BR4.1) — never a positional index, so it survives baseline
/// re-imports.
#[derive(Clone, Debug, PartialEq)]
pub enum Anchor {
    /// Immediately to the left of the named neighbour lane.
    LeftOf {
        neighbour_lane_discriminator: String,
    },
    /// Immediately to the right of the named neighbour lane.
    RightOf {
        neighbour_lane_discriminator: String,
    },
    /// A stated offset from a named edge of the street (e.g. the kerb).
    OffsetFromEdge { edge: String, offset_metres: f64 },
}

/// The kind of change a [`LaneEdit`] proposes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaneEditKind {
    /// An existing lane's attribute is changed.
    Change,
    /// A lane is added that exists in no import.
    Add,
    /// An existing lane is removed.
    Remove,
}

/// Which single attribute of an existing lane a `kind: change` [`LaneEdit`]
/// changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaneAttribute {
    LaneType,
    Width,
    Direction,
}

/// A proposed change — an existing lane's attribute changed, a lane added
/// (exists in no import), or a lane removed.
///
/// `target_lane_discriminator`, `lane_type` and `ordinal_from_kerb`
/// together carry this Unit's own correspondence key (BR7.1: matching is
/// by `(lane_type, ordinal_from_kerb)` alone, never direction) — beyond
/// `entities.md`'s field list, `lane_type` and `ordinal_from_kerb` are
/// carried directly on the edit (rather than re-derived by parsing
/// `target_lane_discriminator`) so [`crate::corridor_planner`] can match
/// an edit against a target street's lanes without this crate inventing a
/// second discriminator grammar to parse. `direction` is likewise carried
/// directly, needed only to fully specify an added lane's identity
/// (`kind: add`); it is `None` for `change`/`remove`, whose
/// `target_lane_discriminator` already names an existing baseline lane.
#[derive(Clone, Debug, PartialEq)]
pub struct LaneEdit {
    pub edit_id: String,
    pub design_id: String,
    /// The street key (OSM way id + bounding node ids) this edit targets.
    pub target_street: StreetIdentity,
    /// Absent for an addition (`kind: add`); present for a change or
    /// removal (BR4.1).
    pub target_lane_discriminator: Option<String>,
    /// The lane type this edit concerns — the new lane's type for an
    /// addition, or the targeted lane's type for a change/removal.
    pub lane_type: Option<String>,
    /// The lane's position counting outward from the kerb — present for a
    /// change/removal (copied from the targeted baseline lane) and absent
    /// for an addition, whose position is expressed by `anchor` instead.
    pub ordinal_from_kerb: Option<u32>,
    /// Present only for an addition, to fully specify the new lane's
    /// identity.
    pub direction: Option<Direction>,
    /// Present only for an addition (BR4.1): position relative to a keyed
    /// baseline lane.
    pub anchor: Option<Anchor>,
    pub kind: LaneEditKind,
    pub attribute: Option<LaneAttribute>,
    pub value: Option<String>,
    /// `UserSet` provenance once edited (BR3.1).
    pub width: Option<Dimension>,
}

/// What every editing operation returns, instead of throwing.
#[derive(Clone, Debug, PartialEq)]
pub struct EditOutcome {
    pub applied: bool,
    pub findings: Option<Vec<EditFinding>>,
}

/// A single reason an edit was rejected or a warning attached to one.
#[derive(Clone, Debug, PartialEq)]
pub struct EditFinding {
    pub reason: String,
    pub detail: Option<String>,
}

/// The editing state machine's live state — selection and undo history.
/// Synchronous (BR1.1).
#[derive(Clone, Debug, PartialEq)]
pub struct EditSession {
    pub session_id: String,
    pub selected_street: Option<StreetIdentity>,
    pub selected_lane_discriminator: Option<String>,
    pub undo_stack: Vec<UndoEntry>,
}

/// The state an [`UndoEntry`] needs to reverse its action exactly. This
/// crate's [`crate::design_overlay::DesignOverlay`] is purely additive
/// (a "current" value is always the latest matching edit), so reversing
/// an action is exactly removing the [`LaneEdit`]s it added.
#[derive(Clone, Debug, PartialEq)]
pub enum UndoRecord {
    EditsAdded(Vec<String>),
}

/// One reversible action. `affected_streets` has more than one entry only
/// for a corridor apply.
#[derive(Clone, Debug, PartialEq)]
pub struct UndoEntry {
    pub undo_entry_id: String,
    pub action: String,
    pub affected_streets: Vec<StreetIdentity>,
    pub prior_values: UndoRecord,
}

/// Four states, never two (BR8.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitStatus {
    Fits,
    DoesNotFit,
    CouldNotBeChecked,
    NotYetChecked,
}

/// A target street's fit assessment against a design's total lane width.
#[derive(Clone, Debug, PartialEq)]
pub struct FitState {
    pub status: FitStatus,
    /// Present only when `status` is `does_not_fit`.
    pub shortfall: Option<Dimension>,
}

/// A source street, target streets chosen from its connected set, and
/// per-target fit state.
#[derive(Clone, Debug, PartialEq)]
pub struct CorridorSelection {
    pub selection_id: String,
    pub source_street: StreetIdentity,
    pub target_streets: Vec<StreetIdentity>,
    pub fit_states: HashMap<StreetIdentity, FitState>,
    pub applied_at: Option<String>,
}

/// Outcome of matching a design's lane changes onto a target street's
/// lanes by type and ordinal-from-kerb (BR7.1).
#[derive(Clone, Debug, PartialEq)]
pub struct CorrespondenceResult {
    /// `LaneEdit` ids successfully matched.
    pub matched: Vec<String>,
    /// `LaneEdit` ids the correspondence rule could not match — the
    /// target keeps what the design doesn't speak to.
    pub unmatched: Vec<String>,
}

/// Per-street result of a bulk apply. Never atomic (BR10.1).
#[derive(Clone, Debug, PartialEq)]
pub struct CorridorApplyOutcome {
    pub succeeded: Vec<StreetIdentity>,
    /// Applied despite `does_not_fit`.
    pub warned: Vec<StreetIdentity>,
    pub could_not_be_checked: Vec<StreetIdentity>,
    /// Could not be prepared at all, with a stated reason.
    pub failed: Vec<(StreetIdentity, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use street_core::{BoundingNodeIds, Direction, StreetIdentity};

    fn a_street_identity() -> StreetIdentity {
        StreetIdentity {
            osm_way_id: "4729721".to_string(),
            bounding_node_ids: BoundingNodeIds::new("100", "200"),
        }
    }

    // Step 3, test: constructing a `Design` carries every stated field.
    #[test]
    fn a_design_carries_its_own_id_name_and_targeted_streets() {
        let design = Design {
            design_id: "design-1".to_string(),
            name: "Wider cycle lanes".to_string(),
            streets: vec![a_street_identity()],
            created_at: "2026-09-18T00:00:00Z".to_string(),
            updated_at: "2026-09-18T00:00:00Z".to_string(),
        };

        assert_eq!(design.design_id, "design-1");
        assert_eq!(design.streets, vec![a_street_identity()]);
    }

    // Step 3, test: an added `LaneEdit` carries an anchor and NO
    // `target_lane_discriminator` (BR4.1); a changed/removed one carries a
    // `target_lane_discriminator` and NO anchor.
    #[test]
    fn an_added_lane_edit_carries_an_anchor_and_no_discriminator() {
        let edit = LaneEdit {
            edit_id: "edit-1".to_string(),
            design_id: "design-1".to_string(),
            target_street: a_street_identity(),
            target_lane_discriminator: None,
            lane_type: Some("cycleway".to_string()),
            ordinal_from_kerb: None,
            direction: Some(Direction::Forward),
            anchor: Some(Anchor::RightOf {
                neighbour_lane_discriminator: "kerb_buffer|0".to_string(),
            }),
            kind: LaneEditKind::Add,
            attribute: None,
            value: None,
            width: None,
        };

        assert_eq!(edit.kind, LaneEditKind::Add);
        assert!(edit.anchor.is_some());
        assert!(edit.target_lane_discriminator.is_none());
    }

    #[test]
    fn a_changed_lane_edit_carries_a_discriminator_and_no_anchor() {
        let edit = LaneEdit {
            edit_id: "edit-2".to_string(),
            design_id: "design-1".to_string(),
            target_street: a_street_identity(),
            target_lane_discriminator: Some("Travel|1".to_string()),
            lane_type: Some("Travel".to_string()),
            ordinal_from_kerb: Some(1),
            direction: None,
            anchor: None,
            kind: LaneEditKind::Change,
            attribute: Some(LaneAttribute::Width),
            value: None,
            width: None,
        };

        assert_eq!(edit.kind, LaneEditKind::Change);
        assert!(edit.anchor.is_none());
        assert!(edit.target_lane_discriminator.is_some());
    }

    #[test]
    fn a_removed_lane_edit_carries_a_discriminator_and_no_anchor() {
        let edit = LaneEdit {
            edit_id: "edit-3".to_string(),
            design_id: "design-1".to_string(),
            target_street: a_street_identity(),
            target_lane_discriminator: Some("Parking|2".to_string()),
            lane_type: Some("Parking".to_string()),
            ordinal_from_kerb: Some(2),
            direction: None,
            anchor: None,
            kind: LaneEditKind::Remove,
            attribute: None,
            value: None,
            width: None,
        };

        assert_eq!(edit.kind, LaneEditKind::Remove);
        assert!(edit.anchor.is_none());
        assert!(edit.target_lane_discriminator.is_some());
    }

    #[test]
    fn edit_outcome_and_finding_carry_a_reason_and_optional_detail() {
        let outcome = EditOutcome {
            applied: false,
            findings: Some(vec![EditFinding {
                reason: "invalid_width".to_string(),
                detail: Some("width must be within (0, 20] metres".to_string()),
            }]),
        };

        assert!(!outcome.applied);
        assert_eq!(outcome.findings.unwrap().len(), 1);
    }

    #[test]
    fn an_edit_session_carries_selection_and_an_undo_stack() {
        let session = EditSession {
            session_id: "session-1".to_string(),
            selected_street: Some(a_street_identity()),
            selected_lane_discriminator: Some("Travel|1".to_string()),
            undo_stack: vec![],
        };

        assert_eq!(session.session_id, "session-1");
        assert!(session.selected_street.is_some());
        assert!(session.undo_stack.is_empty());
    }

    #[test]
    fn an_undo_entry_names_every_street_a_corridor_apply_touched() {
        let entry = UndoEntry {
            undo_entry_id: "undo-1".to_string(),
            action: "corridor_apply".to_string(),
            affected_streets: vec![a_street_identity()],
            prior_values: UndoRecord::EditsAdded(vec!["edit-1".to_string()]),
        };

        assert_eq!(entry.affected_streets.len(), 1);
    }

    #[test]
    fn a_corridor_selection_maps_target_streets_to_fit_states() {
        use std::collections::HashMap;

        let mut fit_states = HashMap::new();
        fit_states.insert(
            a_street_identity(),
            FitState {
                status: FitStatus::NotYetChecked,
                shortfall: None,
            },
        );

        let selection = CorridorSelection {
            selection_id: "selection-1".to_string(),
            source_street: a_street_identity(),
            target_streets: vec![a_street_identity()],
            fit_states,
            applied_at: None,
        };

        assert_eq!(selection.target_streets.len(), 1);
        assert!(selection.applied_at.is_none());
    }

    // A `FitState` reports exactly four states, never two (BR8.1).
    #[test]
    fn fit_state_reports_shortfall_only_when_it_does_not_fit() {
        let does_not_fit = FitState {
            status: FitStatus::DoesNotFit,
            shortfall: Some(street_core::Dimension {
                metres: 1.2,
                provenance: street_core::Provenance::Inferred,
            }),
        };
        let fits = FitState {
            status: FitStatus::Fits,
            shortfall: None,
        };

        assert_eq!(does_not_fit.status, FitStatus::DoesNotFit);
        assert!(does_not_fit.shortfall.is_some());
        assert!(fits.shortfall.is_none());
    }

    #[test]
    fn a_correspondence_result_separates_matched_from_unmatched_edits() {
        let result = CorrespondenceResult {
            matched: vec!["edit-1".to_string()],
            unmatched: vec!["edit-2".to_string()],
        };

        assert_eq!(result.matched, vec!["edit-1".to_string()]);
        assert_eq!(result.unmatched, vec!["edit-2".to_string()]);
    }

    // A `CorridorApplyOutcome` names every target street in exactly one of
    // its four buckets (BR10.1).
    #[test]
    fn a_corridor_apply_outcome_names_every_target_street_once() {
        let outcome = CorridorApplyOutcome {
            succeeded: vec![a_street_identity()],
            warned: vec![],
            could_not_be_checked: vec![],
            failed: vec![],
        };

        assert_eq!(outcome.succeeded.len(), 1);
        assert!(outcome.warned.is_empty());
        assert!(outcome.could_not_be_checked.is_empty());
        assert!(outcome.failed.is_empty());
    }
}
