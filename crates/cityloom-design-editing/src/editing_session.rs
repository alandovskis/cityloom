//! `EditingSession` — selection and undo state machine (`components.md`).

use cityloom_street_import::CorrectionOverlay;
use street_core::{Direction, StreetIdentity};

use crate::design_overlay::DesignOverlay;
use crate::entities::{Anchor, EditFinding, EditOutcome, LaneEdit, UndoEntry, UndoRecord};

/// A fixed capacity for `EditingSession`'s undo stack (`security-design.md`
/// SD-1): pushing beyond this drops the oldest entry rather than growing
/// unbounded. `CorridorSelection.target_streets` needs no analogous cap —
/// it is naturally bounded by the real street network's connected-street
/// set size.
pub const MAX_UNDO_ENTRIES: usize = 200;

/// The three imported-baseline / corrections / design layers, distinguished
/// (BR5.1) — not one of `entities.md`'s ten persisted types, since it is a
/// call return summarising a comparison, not domain state this Unit
/// stores.
#[derive(Clone, Debug, PartialEq)]
pub struct DeltaReport {
    pub target_street: StreetIdentity,
    pub design_edit_count: usize,
    pub correction_count: usize,
    /// `true` when the design layer carries zero edits for this street —
    /// reported explicitly rather than left to be inferred from an empty
    /// comparison (BR5.1).
    pub nothing_has_changed: bool,
}

/// The editing state machine's live state — selection and undo history
/// (`entities.md` `EditSession`), plus the `DesignOverlay` it drives.
/// Synchronous (BR1.1): no method here is `async` or returns a `Future`.
#[derive(Clone, Debug)]
pub struct EditingSession {
    session_id: String,
    design: DesignOverlay,
    selected_street: Option<StreetIdentity>,
    selected_lane_discriminator: Option<String>,
    undo_stack: Vec<UndoEntry>,
    next_undo_seq: u64,
}

impl EditingSession {
    pub fn new(session_id: impl Into<String>, design_id: impl Into<String>) -> EditingSession {
        EditingSession {
            session_id: session_id.into(),
            design: DesignOverlay::new(design_id),
            selected_street: None,
            selected_lane_discriminator: None,
            undo_stack: Vec::new(),
            next_undo_seq: 0,
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn design(&self) -> &DesignOverlay {
        &self.design
    }

    pub fn undo_stack(&self) -> &[UndoEntry] {
        &self.undo_stack
    }

    pub fn selected_street(&self) -> Option<&StreetIdentity> {
        self.selected_street.as_ref()
    }

    pub fn selected_lane_discriminator(&self) -> Option<&str> {
        self.selected_lane_discriminator.as_deref()
    }

    /// Records the current selection. Synchronous (BR1.1).
    pub fn select(&mut self, street: StreetIdentity, lane_discriminator: Option<String>) {
        self.selected_street = Some(street);
        self.selected_lane_discriminator = lane_discriminator;
    }

    fn push_undo(
        &mut self,
        action: impl Into<String>,
        affected_streets: Vec<StreetIdentity>,
        edit_ids: Vec<String>,
    ) {
        if edit_ids.is_empty() {
            return;
        }
        self.next_undo_seq += 1;
        let entry = UndoEntry {
            undo_entry_id: format!("{}-undo-{}", self.session_id, self.next_undo_seq),
            action: action.into(),
            affected_streets,
            prior_values: UndoRecord::EditsAdded(edit_ids),
        };
        // SD-1: bounded capacity — drop the oldest entry rather than grow
        // unbounded.
        if self.undo_stack.len() >= MAX_UNDO_ENTRIES {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(entry);
    }

    /// Adds a lane and records one undo entry for it. Synchronous (BR1.1).
    pub fn add_lane(
        &mut self,
        target_street: StreetIdentity,
        lane_type: &str,
        direction: Direction,
        width_metres: f64,
        anchor: Anchor,
    ) -> EditOutcome {
        let before = self.design.edits().len();
        let outcome = self.design.add_lane(
            target_street.clone(),
            lane_type,
            direction,
            width_metres,
            anchor,
        );
        if outcome.applied {
            let new_ids = self.design.edits()[before..]
                .iter()
                .map(|edit| edit.edit_id.clone())
                .collect();
            self.push_undo("add_lane", vec![target_street], new_ids);
        }
        outcome
    }

    /// Removes a lane and records one undo entry for it. Synchronous
    /// (BR1.1).
    pub fn remove_lane(
        &mut self,
        target_street: StreetIdentity,
        lane_type: &str,
        ordinal_from_kerb: u32,
    ) -> EditOutcome {
        let before = self.design.edits().len();
        let outcome = self
            .design
            .remove_lane(target_street.clone(), lane_type, ordinal_from_kerb);
        if outcome.applied {
            let new_ids = self.design.edits()[before..]
                .iter()
                .map(|edit| edit.edit_id.clone())
                .collect();
            self.push_undo("remove_lane", vec![target_street], new_ids);
        }
        outcome
    }

    /// Changes a lane's width and records one undo entry for it.
    /// Synchronous (BR1.1).
    pub fn change_width(
        &mut self,
        target_street: StreetIdentity,
        lane_type: &str,
        ordinal_from_kerb: u32,
        new_metres: f64,
    ) -> EditOutcome {
        let before = self.design.edits().len();
        let outcome = self.design.change_width(
            target_street.clone(),
            lane_type,
            ordinal_from_kerb,
            new_metres,
        );
        if outcome.applied {
            let new_ids = self.design.edits()[before..]
                .iter()
                .map(|edit| edit.edit_id.clone())
                .collect();
            self.push_undo("change_width", vec![target_street], new_ids);
        }
        outcome
    }

    /// Applies a corridor apply's already-constructed per-target-street
    /// edits directly into this session's [`DesignOverlay`] and records
    /// **one** undo entry spanning every affected street — a corridor
    /// apply's `UndoEntry` is reversed as one atomic unit across every
    /// street it touched (BR6.1), never one entry per street. Called by
    /// [`crate::corridor_planner::CorridorPlanner`]. Returns the ids of
    /// the edits that were applied.
    pub fn apply_corridor_edits(
        &mut self,
        edits: Vec<LaneEdit>,
        affected_streets: Vec<StreetIdentity>,
    ) -> Vec<String> {
        let edit_ids: Vec<String> = edits.iter().map(|edit| edit.edit_id.clone()).collect();
        for edit in edits {
            self.design.push_edit(edit);
        }
        self.push_undo("corridor_apply", affected_streets, edit_ids.clone());
        edit_ids
    }

    /// Reports the three layers (baseline, corrections, design)
    /// distinguished for `target_street` (BR5.1). A design with zero
    /// edits for that street reports "nothing has changed," never an
    /// empty comparison.
    pub fn delta_report(
        &self,
        target_street: &StreetIdentity,
        corrections: &CorrectionOverlay,
    ) -> DeltaReport {
        let design_edit_count = self
            .design
            .edits()
            .iter()
            .filter(|edit| &edit.target_street == target_street)
            .count();
        let correction_count = corrections.corrections_for(target_street).len();

        DeltaReport {
            target_street: target_street.clone(),
            design_edit_count,
            correction_count,
            nothing_has_changed: design_edit_count == 0,
        }
    }

    /// Reverses the most recently completed action, restoring the model
    /// to its immediately preceding state (BR6.1). A corridor apply's
    /// entry (multiple `affected_streets`) is reversed as one atomic
    /// unit — removing every edit id it recorded in a single call. With
    /// an empty undo stack, this changes nothing and reports "nothing to
    /// undo" rather than failing.
    pub fn undo(&mut self) -> EditOutcome {
        match self.undo_stack.pop() {
            None => EditOutcome {
                applied: false,
                findings: Some(vec![EditFinding {
                    reason: "nothing to undo".to_string(),
                    detail: None,
                }]),
            },
            Some(entry) => {
                let UndoRecord::EditsAdded(edit_ids) = &entry.prior_values;
                self.design.remove_edits(edit_ids);
                EditOutcome {
                    applied: true,
                    findings: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use cityloom_street_import::CorrectionOverlay;
    use street_core::{BoundingNodeIds, StreetIdentity};

    use crate::editing_session::EditingSession;
    use crate::entities::{LaneEdit, LaneEditKind};

    fn a_street_identity() -> StreetIdentity {
        StreetIdentity {
            osm_way_id: "4729721".to_string(),
            bounding_node_ids: BoundingNodeIds::new("100", "200"),
        }
    }

    // Test 8: zero edits reports "nothing has changed," never an empty
    // comparison (BR5.1, AC5.4.2).
    #[test]
    fn zero_edits_reports_nothing_has_changed() {
        let session = EditingSession::new("session-1", "design-1");
        let corrections = CorrectionOverlay::new();

        let report = session.delta_report(&a_street_identity(), &corrections);

        assert!(report.nothing_has_changed);
        assert_eq!(report.design_edit_count, 0);
    }

    // Test 9: a design with edits reports the three layers (baseline,
    // corrections, design) distinguished (BR5.1, AC5.4.1).
    #[test]
    fn a_design_with_edits_distinguishes_the_three_layers() {
        let mut session = EditingSession::new("session-1", "design-1");
        session.change_width(a_street_identity(), "Travel", 0, 3.5);

        let corrections = CorrectionOverlay::new();
        let report = session.delta_report(&a_street_identity(), &corrections);

        assert!(!report.nothing_has_changed);
        assert_eq!(report.design_edit_count, 1);
        assert_eq!(report.correction_count, 0);
    }

    // Test 10: undo reverses the most recent single-street action and
    // restores the prior state (BR6.1, AC5.5.1).
    #[test]
    fn undo_reverses_the_most_recent_single_street_action() {
        let mut session = EditingSession::new("session-1", "design-1");
        session.change_width(a_street_identity(), "Travel", 0, 3.5);
        assert_eq!(session.design().edits().len(), 1);

        let outcome = session.undo();

        assert!(outcome.applied);
        assert!(session.design().edits().is_empty());
    }

    // Test 11: undo on a corridor-apply `UndoEntry` reverses every
    // affected street as one unit (BR6.1, AC5.5.2).
    #[test]
    fn undo_reverses_a_corridor_apply_as_one_unit() {
        let mut session = EditingSession::new("session-1", "design-1");
        let street_a = a_street_identity();
        let street_b = StreetIdentity {
            osm_way_id: "9999999".to_string(),
            bounding_node_ids: BoundingNodeIds::new("1", "2"),
        };

        let edit_a = LaneEdit {
            edit_id: "corridor-edit-1".to_string(),
            design_id: "design-1".to_string(),
            target_street: street_a.clone(),
            target_lane_discriminator: Some("Travel|0".to_string()),
            lane_type: Some("Travel".to_string()),
            ordinal_from_kerb: Some(0),
            direction: None,
            anchor: None,
            kind: LaneEditKind::Change,
            attribute: Some(crate::entities::LaneAttribute::Width),
            value: None,
            width: Some(street_core::Dimension {
                metres: 3.2,
                provenance: street_core::Provenance::UserSet,
            }),
        };
        let mut edit_b = edit_a.clone();
        edit_b.edit_id = "corridor-edit-2".to_string();
        edit_b.target_street = street_b.clone();

        session.apply_corridor_edits(vec![edit_a, edit_b], vec![street_a, street_b]);

        assert_eq!(session.undo_stack().len(), 1);
        let entry = &session.undo_stack()[0];
        assert_eq!(entry.affected_streets.len(), 2);
        assert_eq!(session.design().edits().len(), 2);

        // A single `undo()` call reverses the whole corridor apply, both
        // affected streets' edits at once.
        let outcome = session.undo();
        assert!(outcome.applied);
        assert!(session.design().edits().is_empty());
    }

    // Test 12: undo with an empty stack reports "nothing to undo" without
    // changing state (BR6.1, AC5.5.3).
    #[test]
    fn undo_with_an_empty_stack_reports_nothing_to_undo() {
        let mut session = EditingSession::new("session-1", "design-1");

        let outcome = session.undo();

        assert!(!outcome.applied);
        assert_eq!(
            outcome.findings.unwrap()[0].reason,
            "nothing to undo".to_string()
        );
    }

    // security-design.md SD-1: the undo stack is bounded; pushing beyond
    // capacity drops the oldest entry rather than growing unbounded.
    #[test]
    fn the_undo_stack_is_bounded_and_drops_the_oldest_entry() {
        let mut session = EditingSession::new("session-1", "design-1");

        for i in 0..(crate::editing_session::MAX_UNDO_ENTRIES + 5) {
            session.change_width(a_street_identity(), "Travel", 0, 3.0 + (i as f64) * 0.01);
        }

        assert_eq!(
            session.undo_stack().len(),
            crate::editing_session::MAX_UNDO_ENTRIES
        );
    }
}
