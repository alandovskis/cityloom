//! `DesignOverlay` — proposed lane edits, additions, removals, and revert
//! semantics (`components.md`).

use cityloom_street_import::CorrectionOverlay;
use street_core::{Dimension, Direction, Lane, Provenance, Street, StreetIdentity};

use crate::entities::{Anchor, EditFinding, EditOutcome, LaneAttribute, LaneEdit, LaneEditKind};

/// A single lane as this Unit's overlay resolves it: the baseline value,
/// or the latest design edit overriding it (BR5.1). Positionally ordered
/// the same way the underlying baseline is (AC5.2.2) — removal filters,
/// it never reorders.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectiveLane {
    pub lane_type: String,
    pub direction: Direction,
    pub ordinal_from_kerb: Option<u32>,
    pub width: Dimension,
    pub target_lane_discriminator: Option<String>,
}

/// The outcome of a revert operation (BR3.1) — not one of `entities.md`'s
/// ten persisted types, since it is a call return describing what a
/// revert resolved to, not domain state this Unit stores.
#[derive(Clone, Debug, PartialEq)]
pub struct RevertOutcome {
    pub applied: bool,
    pub findings: Option<Vec<EditFinding>>,
    pub resulting_width: Option<Dimension>,
}

const MIN_LANE_WIDTH_METRES: f64 = 0.0;
const MAX_LANE_WIDTH_METRES: f64 = 20.0;

fn validate_width(metres: f64) -> Result<(), EditFinding> {
    if metres > MIN_LANE_WIDTH_METRES && metres <= MAX_LANE_WIDTH_METRES {
        Ok(())
    } else {
        Err(EditFinding {
            reason: "invalid_width".to_string(),
            detail: Some(format!(
                "width must be greater than {MIN_LANE_WIDTH_METRES} and at most \
                 {MAX_LANE_WIDTH_METRES} metres, got {metres}"
            )),
        })
    }
}

/// This Unit's own correspondence/discriminator key for a lane —
/// `(lane_type, ordinal_from_kerb)` (BR7.1), deliberately excluding
/// direction so the correspondence rule can match a lane across streets
/// digitised in different directions.
fn lane_discriminator(lane_type: &str, ordinal_from_kerb: u32) -> String {
    format!("{lane_type}|{ordinal_from_kerb}")
}

/// Proposed lane edits, additions and removals held as an overlay over an
/// immutable baseline `Street` (never a mutation of it — `team.md` Code
/// Style). Purely additive: the "current" value for a target is always
/// the latest matching [`LaneEdit`], so undo (BR6.1) is exactly removing
/// the edits an action added.
#[derive(Clone, Debug, Default)]
pub struct DesignOverlay {
    design_id: String,
    edits: Vec<LaneEdit>,
    next_edit_seq: u64,
}

impl DesignOverlay {
    pub fn new(design_id: impl Into<String>) -> DesignOverlay {
        DesignOverlay {
            design_id: design_id.into(),
            edits: Vec::new(),
            next_edit_seq: 0,
        }
    }

    pub fn design_id(&self) -> &str {
        &self.design_id
    }

    /// Every edit proposed so far, in the order they were made.
    pub fn edits(&self) -> &[LaneEdit] {
        &self.edits
    }

    /// Appends an already-constructed [`LaneEdit`] directly — used by
    /// [`crate::editing_session::EditingSession::apply_corridor_edits`]
    /// to apply a corridor apply's per-target-street edits, which are
    /// constructed by [`crate::corridor_planner::CorridorPlanner`] rather
    /// than by one of this type's own `add_lane`/`change_width`/
    /// `remove_lane` methods.
    pub(crate) fn push_edit(&mut self, edit: LaneEdit) {
        self.edits.push(edit);
    }

    /// Allocates a fresh, unique edit id from this overlay's own counter —
    /// used by callers (e.g. `CorridorPlanner`) that construct a
    /// [`LaneEdit`] directly rather than through one of this type's own
    /// mutating methods.
    pub(crate) fn next_edit_id(&mut self) -> String {
        self.next_edit_seq += 1;
        format!("{}-edit-{}", self.design_id, self.next_edit_seq)
    }

    /// Removes the edits named by `edit_ids` — the mechanism undo (BR6.1)
    /// uses to reverse an action exactly.
    pub fn remove_edits(&mut self, edit_ids: &[String]) {
        self.edits.retain(|edit| !edit_ids.contains(&edit.edit_id));
    }

    /// Proposes a new lane that exists in no import (BR4.1): carries an
    /// `anchor` and no `target_lane_discriminator`, with `UserSet`
    /// provenance on its width. Synchronous — no `async`, no `Future`
    /// (BR1.1).
    pub fn add_lane(
        &mut self,
        target_street: StreetIdentity,
        lane_type: &str,
        direction: Direction,
        width_metres: f64,
        anchor: Anchor,
    ) -> EditOutcome {
        if let Err(finding) = validate_width(width_metres) {
            return EditOutcome {
                applied: false,
                findings: Some(vec![finding]),
            };
        }

        let edit_id = self.next_edit_id();
        self.edits.push(LaneEdit {
            edit_id,
            design_id: self.design_id.clone(),
            target_street,
            target_lane_discriminator: None,
            lane_type: Some(lane_type.to_string()),
            ordinal_from_kerb: None,
            direction: Some(direction),
            anchor: Some(anchor),
            kind: LaneEditKind::Add,
            attribute: None,
            value: None,
            width: Some(Dimension {
                metres: width_metres,
                provenance: Provenance::UserSet,
            }),
        });

        EditOutcome {
            applied: true,
            findings: None,
        }
    }

    /// Proposes removing an existing lane, keyed by
    /// `(lane_type, ordinal_from_kerb)` — never a positional index.
    /// Synchronous (BR1.1).
    pub fn remove_lane(
        &mut self,
        target_street: StreetIdentity,
        lane_type: &str,
        ordinal_from_kerb: u32,
    ) -> EditOutcome {
        let edit_id = self.next_edit_id();
        self.edits.push(LaneEdit {
            edit_id,
            design_id: self.design_id.clone(),
            target_street,
            target_lane_discriminator: Some(lane_discriminator(lane_type, ordinal_from_kerb)),
            lane_type: Some(lane_type.to_string()),
            ordinal_from_kerb: Some(ordinal_from_kerb),
            direction: None,
            anchor: None,
            kind: LaneEditKind::Remove,
            attribute: None,
            value: None,
            width: None,
        });

        EditOutcome {
            applied: true,
            findings: None,
        }
    }

    /// Proposes changing an existing lane's width. Rejects a value
    /// outside `(0, 20]` metres with a stated reason, retaining the
    /// previous value (BR2.1); a valid change sets `UserSet` provenance
    /// (BR3.1). Synchronous (BR1.1).
    pub fn change_width(
        &mut self,
        target_street: StreetIdentity,
        lane_type: &str,
        ordinal_from_kerb: u32,
        new_metres: f64,
    ) -> EditOutcome {
        if let Err(finding) = validate_width(new_metres) {
            return EditOutcome {
                applied: false,
                findings: Some(vec![finding]),
            };
        }

        let edit_id = self.next_edit_id();
        self.edits.push(LaneEdit {
            edit_id,
            design_id: self.design_id.clone(),
            target_street,
            target_lane_discriminator: Some(lane_discriminator(lane_type, ordinal_from_kerb)),
            lane_type: Some(lane_type.to_string()),
            ordinal_from_kerb: Some(ordinal_from_kerb),
            direction: None,
            anchor: None,
            kind: LaneEditKind::Change,
            attribute: Some(LaneAttribute::Width),
            value: None,
            width: Some(Dimension {
                metres: new_metres,
                provenance: Provenance::UserSet,
            }),
        });

        EditOutcome {
            applied: true,
            findings: None,
        }
    }

    /// Reverts a width change on the targeted lane, restoring it to what
    /// it was before the edit (BR3.1): the corrected baseline value where
    /// a `Correction` exists for that attribute (via
    /// `cityloom-street-import`'s [`CorrectionOverlay`]) — carrying its
    /// own structural `UserSet` provenance — otherwise the imported
    /// baseline value carrying that value's own provenance. Never forces
    /// `UserSet` on a reverted value that was never corrected. This
    /// removes the design-layer width edit(s) on the targeted lane so
    /// [`Self::effective_lanes`] reflects the reverted value too.
    pub fn revert_width(
        &mut self,
        target_street: &StreetIdentity,
        lane_type: &str,
        ordinal_from_kerb: u32,
        baseline_lane: &Lane,
        corrections: &CorrectionOverlay,
    ) -> RevertOutcome {
        let discriminator = lane_discriminator(lane_type, ordinal_from_kerb);

        let removed_ids: Vec<String> = self
            .edits
            .iter()
            .filter(|edit| {
                &edit.target_street == target_street
                    && edit.target_lane_discriminator.as_deref() == Some(discriminator.as_str())
                    && edit.attribute == Some(LaneAttribute::Width)
            })
            .map(|edit| edit.edit_id.clone())
            .collect();
        self.remove_edits(&removed_ids);

        let correction = corrections
            .corrections_for(target_street)
            .into_iter()
            .rfind(|correction| {
                correction.target_lane_discriminator.as_deref() == Some(discriminator.as_str())
                    && correction.attribute.as_deref() == Some("width")
            });

        let resulting_width = match correction.and_then(|correction| correction.value.as_deref()) {
            Some(raw_metres) => raw_metres.parse::<f64>().ok().map(|metres| Dimension {
                metres,
                // A `Correction`'s value is structurally `UserSet`
                // (`cityloom-street-import::Correction` carries no
                // `provenance` field at all — see that type's doc
                // comment) — never the raw imported baseline's own
                // provenance.
                provenance: Provenance::UserSet,
            }),
            None => Some(baseline_lane.width()),
        };

        RevertOutcome {
            applied: true,
            findings: None,
            resulting_width,
        }
    }

    /// Resolves this overlay's edits against `baseline` into an ordered
    /// list of lanes as the design currently proposes them: baseline
    /// lanes in their original relative order, with a `remove`-kind edit
    /// filtering one out and a `change`-kind edit overriding its width,
    /// followed by any `add`-kind edits targeting this street. Never
    /// mutates `baseline` (BR5.1).
    pub fn effective_lanes(&self, baseline: &Street) -> Vec<EffectiveLane> {
        let mut lanes = Vec::new();

        for index in 0..baseline.lane_count() {
            let Some(lane) = baseline.lane_at(index) else {
                continue;
            };
            let discriminator = lane_discriminator(lane.lane_type(), lane.ordinal_from_kerb());

            let removed = self.edits.iter().any(|edit| {
                edit.kind == LaneEditKind::Remove
                    && edit.target_lane_discriminator.as_deref() == Some(discriminator.as_str())
            });
            if removed {
                continue;
            }

            let width = self
                .edits
                .iter()
                .rev()
                .find(|edit| {
                    edit.kind == LaneEditKind::Change
                        && edit.attribute == Some(LaneAttribute::Width)
                        && edit.target_lane_discriminator.as_deref() == Some(discriminator.as_str())
                })
                .and_then(|edit| edit.width)
                .unwrap_or_else(|| lane.width());

            lanes.push(EffectiveLane {
                lane_type: lane.lane_type().to_string(),
                direction: lane.direction(),
                ordinal_from_kerb: Some(lane.ordinal_from_kerb()),
                width,
                target_lane_discriminator: Some(discriminator),
            });
        }

        for edit in self
            .edits
            .iter()
            .filter(|edit| edit.kind == LaneEditKind::Add)
        {
            lanes.push(EffectiveLane {
                lane_type: edit.lane_type.clone().unwrap_or_default(),
                direction: edit.direction.unwrap_or(Direction::Forward),
                ordinal_from_kerb: None,
                width: edit.width.unwrap_or(Dimension {
                    metres: 0.0,
                    provenance: Provenance::UserSet,
                }),
                target_lane_discriminator: None,
            });
        }

        lanes
    }
}

#[cfg(test)]
mod tests {
    use cityloom_street_import::{
        Correction, CorrectionOverlay, CorrectionState, DrivingSide, FingerprintMapConfig,
        ImportFingerprint,
    };
    use std::collections::BTreeMap;
    use street_core::{BoundingNodeIds, Dimension, Direction, Lane, Provenance, StreetIdentity};

    use crate::design_overlay::DesignOverlay;
    use crate::entities::{Anchor, LaneEditKind};

    fn a_street_identity() -> StreetIdentity {
        StreetIdentity {
            osm_way_id: "4729721".to_string(),
            bounding_node_ids: BoundingNodeIds::new("100", "200"),
        }
    }

    fn a_fingerprint() -> ImportFingerprint {
        ImportFingerprint {
            way_tags: BTreeMap::new(),
            map_config: FingerprintMapConfig {
                country_code: "US".to_string(),
                driving_side: DrivingSide::Right,
                inferred_sidewalks: false,
                inferred_kerbs: true,
            },
            osm2streets_revision: "test-revision".to_string(),
        }
    }

    // Test 1: adding a lane constructs a `LaneEdit{kind: add}` with an
    // anchor and no `target_lane_discriminator`, UserSet on every
    // attribute (BR4.1, AC5.2.1).
    #[test]
    fn adding_a_lane_carries_an_anchor_no_discriminator_and_user_set_width() {
        let mut overlay = DesignOverlay::new("design-1");

        let outcome = overlay.add_lane(
            a_street_identity(),
            "cycleway",
            Direction::Forward,
            2.0,
            Anchor::RightOf {
                neighbour_lane_discriminator: "kerb_buffer|0".to_string(),
            },
        );

        assert!(outcome.applied);
        let edit = overlay.edits().last().expect("an edit was recorded");
        assert_eq!(edit.kind, LaneEditKind::Add);
        assert!(edit.anchor.is_some());
        assert!(edit.target_lane_discriminator.is_none());
        assert_eq!(edit.width.unwrap().provenance, Provenance::UserSet);
    }

    // Test 2: removing a lane constructs `LaneEdit{kind: remove}`;
    // remaining lanes keep relative order (AC5.2.2).
    #[test]
    fn removing_a_lane_keeps_the_remaining_lanes_in_order() {
        let lanes = vec![
            Lane::new(
                "Parking",
                Direction::Forward,
                Dimension {
                    metres: 2.0,
                    provenance: Provenance::Mapped,
                },
                0,
            ),
            Lane::new(
                "Travel",
                Direction::Forward,
                Dimension {
                    metres: 3.0,
                    provenance: Provenance::Mapped,
                },
                1,
            ),
            Lane::new(
                "Travel",
                Direction::Backward,
                Dimension {
                    metres: 3.0,
                    provenance: Provenance::Mapped,
                },
                2,
            ),
        ];
        let street =
            street_core::Street::new("way-1", BoundingNodeIds::new("100", "200"), None, lanes);
        let mut overlay = DesignOverlay::new("design-1");

        let outcome = overlay.remove_lane(a_street_identity(), "Parking", 0);
        assert!(outcome.applied);

        let effective = overlay.effective_lanes(&street);
        let types: Vec<&str> = effective
            .iter()
            .map(|lane| lane.lane_type.as_str())
            .collect();
        assert_eq!(types, vec!["Travel", "Travel"]);
        assert_eq!(effective[0].direction, Direction::Forward);
        assert_eq!(effective[1].direction, Direction::Backward);
    }

    // Test 3: a width change outside `(0, 20]` metres is rejected with a
    // stated reason; the previous value is retained (BR2.1, AC5.1.5).
    #[test]
    fn a_width_outside_the_valid_range_is_rejected_and_previous_value_retained() {
        let lanes = vec![Lane::new(
            "Travel",
            Direction::Forward,
            Dimension {
                metres: 3.0,
                provenance: Provenance::Mapped,
            },
            0,
        )];
        let street =
            street_core::Street::new("way-1", BoundingNodeIds::new("100", "200"), None, lanes);
        let mut overlay = DesignOverlay::new("design-1");

        for invalid in [0.0, -1.0, 20.1] {
            let outcome = overlay.change_width(a_street_identity(), "Travel", 0, invalid);
            assert!(!outcome.applied);
            assert!(!outcome.findings.unwrap().is_empty());
        }

        // The previous (baseline) value is unaffected.
        let effective = overlay.effective_lanes(&street);
        assert_eq!(effective[0].width.metres, 3.0);
        assert_eq!(effective[0].width.provenance, Provenance::Mapped);
    }

    // Test 4: a valid width change sets UserSet provenance (BR3.1,
    // AC5.1.3).
    #[test]
    fn a_valid_width_change_sets_user_set_provenance() {
        let lanes = vec![Lane::new(
            "Travel",
            Direction::Forward,
            Dimension {
                metres: 3.0,
                provenance: Provenance::Mapped,
            },
            0,
        )];
        let street =
            street_core::Street::new("way-1", BoundingNodeIds::new("100", "200"), None, lanes);
        let mut overlay = DesignOverlay::new("design-1");

        let outcome = overlay.change_width(a_street_identity(), "Travel", 0, 3.5);

        assert!(outcome.applied);
        let effective = overlay.effective_lanes(&street);
        assert_eq!(effective[0].width.metres, 3.5);
        assert_eq!(effective[0].width.provenance, Provenance::UserSet);
    }

    // Test 5: reverting a changed attribute with no correction on it
    // returns the imported baseline value and that value's own
    // provenance (BR3.1, AC5.4.3).
    #[test]
    fn reverting_with_no_correction_returns_the_baseline_value_and_provenance() {
        let lane = Lane::new(
            "Travel",
            Direction::Forward,
            Dimension {
                metres: 3.0,
                provenance: Provenance::Mapped,
            },
            0,
        );
        let mut overlay = DesignOverlay::new("design-1");
        overlay.change_width(a_street_identity(), "Travel", 0, 3.5);

        let corrections = CorrectionOverlay::new();
        let outcome = overlay.revert_width(&a_street_identity(), "Travel", 0, &lane, &corrections);

        assert!(outcome.applied);
        let resulting = outcome.resulting_width.expect("a resulting width");
        assert_eq!(resulting.metres, 3.0);
        assert_eq!(resulting.provenance, Provenance::Mapped);
        // The change edit was actually removed.
        let street = street_core::Street::new(
            "way-1",
            BoundingNodeIds::new("100", "200"),
            None,
            vec![lane],
        );
        let effective = overlay.effective_lanes(&street);
        assert_eq!(effective[0].width.metres, 3.0);
        assert_eq!(effective[0].width.provenance, Provenance::Mapped);
    }

    // Test 6: reverting a changed attribute where a correction exists on
    // it returns the corrected value and its provenance, not the raw
    // imported one (BR3.1, AC5.4.3).
    #[test]
    fn reverting_with_a_correction_returns_the_corrected_value() {
        let lane = Lane::new(
            "Travel",
            Direction::Forward,
            Dimension {
                metres: 3.0,
                provenance: Provenance::Mapped,
            },
            0,
        );
        let mut overlay = DesignOverlay::new("design-1");
        overlay.change_width(a_street_identity(), "Travel", 0, 3.5);

        let mut corrections = CorrectionOverlay::new();
        corrections.add(Correction {
            correction_id: "correction-1".to_string(),
            target_street: a_street_identity(),
            target_lane_discriminator: Some("Travel|0".to_string()),
            attribute: Some("width".to_string()),
            value: Some("3.4".to_string()),
            import_fingerprint: a_fingerprint(),
            state: CorrectionState::Applied,
        });

        let outcome = overlay.revert_width(&a_street_identity(), "Travel", 0, &lane, &corrections);

        assert!(outcome.applied);
        let resulting = outcome.resulting_width.expect("a resulting width");
        assert_eq!(resulting.metres, 3.4);
        assert_eq!(resulting.provenance, Provenance::UserSet);
    }

    // Test 7: every `DesignOverlay` method's signature contains no
    // `async` and no `Future`-returning type (BR1.1, AC5.1.2) — asserted
    // structurally: each method coerces to a plain `fn` pointer type that
    // returns a concrete (non-`Future`) value, which would not compile if
    // the method were `async` or returned `impl Future`.
    #[test]
    fn design_overlay_public_operations_are_synchronous_by_construction() {
        fn assert_is_plain_fn<T>(_: T) {}

        let add_lane: fn(
            &mut DesignOverlay,
            StreetIdentity,
            &str,
            Direction,
            f64,
            Anchor,
        ) -> crate::entities::EditOutcome = DesignOverlay::add_lane;
        let change_width: fn(
            &mut DesignOverlay,
            StreetIdentity,
            &str,
            u32,
            f64,
        ) -> crate::entities::EditOutcome = DesignOverlay::change_width;
        let remove_lane: fn(
            &mut DesignOverlay,
            StreetIdentity,
            &str,
            u32,
        ) -> crate::entities::EditOutcome = DesignOverlay::remove_lane;

        assert_is_plain_fn(add_lane);
        assert_is_plain_fn(change_width);
        assert_is_plain_fn(remove_lane);
    }
}
