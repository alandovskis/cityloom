//! `CorridorPlanner` — connectivity, fit assessment, correspondence, bulk
//! apply (`components.md`).

use street_core::{Dimension, Provenance, Street, StreetIdentity, StreetNetworkGraph};

use crate::editing_session::EditingSession;
use crate::entities::{
    CorrespondenceResult, CorridorApplyOutcome, FitState, FitStatus, LaneEdit, LaneEditKind,
};

/// Connectivity, fit assessment, correspondence, and bulk apply
/// (`components.md`). Holds no state of its own — every operation is a
/// pure computation over the `StreetNetworkGraph`, `Street`s and
/// `LaneEdit`s supplied to it, or a call that applies edits into a
/// caller-supplied [`EditingSession`].
pub struct CorridorPlanner;

impl CorridorPlanner {
    /// The set of streets connected to `source`, computed from `graph`'s
    /// own adjacency (BR11.1) — never a coarse display pass. Order
    /// follows the graph's own intersection order; duplicates (a street
    /// meeting `source` at more than one intersection) are not repeated.
    pub fn connected_streets(
        graph: &StreetNetworkGraph,
        source: &StreetIdentity,
    ) -> Vec<StreetIdentity> {
        let mut connected = Vec::new();
        for intersection in graph.intersections() {
            let Some(streets_here) = graph.streets_at(intersection) else {
                continue;
            };
            if !streets_here.contains(source) {
                continue;
            }
            for street in streets_here {
                if street != source && !connected.contains(street) {
                    connected.push(street.clone());
                }
            }
        }
        connected
    }

    /// `identity`'s name from `graph`, or a stable, human-readable
    /// fallback identifier when the import gave it none (BR11.1).
    pub fn display_name(graph: &StreetNetworkGraph, identity: &StreetIdentity) -> String {
        graph
            .streets()
            .find(|street| &street.identity() == identity)
            .and_then(|street| street.name().map(str::to_string))
            .unwrap_or_else(|| format!("Unnamed street (way {})", identity.osm_way_id))
    }

    /// Matches `source_edits` onto `target_street`'s lanes by
    /// `(lane_type, ordinal_from_kerb)` alone (BR7.1) — never direction,
    /// so the rule still matches a lane across streets digitised in
    /// different directions. Only `change`/`remove`-kind edits (which
    /// carry a `lane_type`/`ordinal_from_kerb` referencing an existing
    /// lane) participate; an `add`-kind edit introduces a lane that
    /// exists in no import and has nothing in `target_street` to
    /// correspond to, so it is never a candidate for this match.
    pub fn build_correspondence(
        source_edits: &[LaneEdit],
        target_street: &Street,
    ) -> CorrespondenceResult {
        let mut matched = Vec::new();
        let mut unmatched = Vec::new();

        for edit in source_edits
            .iter()
            .filter(|edit| matches!(edit.kind, LaneEditKind::Change | LaneEditKind::Remove))
        {
            let found = match (edit.lane_type.as_deref(), edit.ordinal_from_kerb) {
                (Some(lane_type), Some(ordinal)) => (0..target_street.lane_count())
                    .filter_map(|index| target_street.lane_at(index))
                    .any(|lane| {
                        lane.lane_type() == lane_type && lane.ordinal_from_kerb() == ordinal
                    }),
                _ => false,
            };

            if found {
                matched.push(edit.edit_id.clone());
            } else {
                unmatched.push(edit.edit_id.clone());
            }
        }

        CorrespondenceResult { matched, unmatched }
    }

    /// Extends one matched `source_edit` onto `target_street`: adds a new
    /// design-layer [`LaneEdit`] targeting it, leaving `target_street`'s
    /// own corrections and untouched lanes intact (BR7.1) — this is
    /// never wholesale replacement of the target's lane list, only a new
    /// entry alongside whatever the target already has.
    pub fn extend_onto_target(
        session: &mut EditingSession,
        source_edit: &LaneEdit,
        target_street: &StreetIdentity,
    ) {
        let mut extended = source_edit.clone();
        extended.edit_id =
            session.design().design_id().to_string() + "-corridor-" + &source_edit.edit_id;
        extended.design_id = session.design().design_id().to_string();
        extended.target_street = target_street.clone();

        session.apply_corridor_edits(vec![extended], vec![target_street.clone()]);
    }

    /// Compares `design_total_width_metres` against `target`'s
    /// carriageway width (BR8.1): `could_not_be_checked` when the target
    /// carries no lanes at all (an absent carriageway) or its
    /// carriageway width carries `Inferred` provenance; `does_not_fit`
    /// (with the numeric shortfall) when the design's total exceeds it;
    /// `fits` otherwise. Total width only — no lane-type compatibility
    /// rule.
    ///
    /// **Known limitation**: `street-core::Street` computes and exposes
    /// only the single carriageway-width `Dimension` (its own sole
    /// definition, per `components.md`'s `StreetModel` — this crate must
    /// not duplicate that computation), whose provenance is `Inferred` if
    /// **any** contributing lane is `Inferred`. BR8.1 asks for
    /// `could_not_be_checked` when **every** contributing lane is
    /// `Inferred`, a strictly narrower condition this crate cannot
    /// distinguish from "at least one contributor is `Inferred`" without
    /// access to `Street`'s private per-lane contributing set. This
    /// method therefore treats any `Inferred` carriageway width as
    /// `could_not_be_checked`, which is a conservative superset of what
    /// BR8.1 states (it never reports `could_not_be_checked` when the
    /// carriageway is fully `Mapped`, and it correctly reports it when
    /// the carriageway is wholly `Inferred`, but it may also report it
    /// for a street whose carriageway is only partially `Inferred`,
    /// which the rule would have wanted assessed as `fits`/`does_not_fit`
    /// instead).
    pub fn assess_fit(design_total_width_metres: f64, target: &Street) -> FitState {
        if target.lane_count() == 0 {
            return FitState {
                status: FitStatus::CouldNotBeChecked,
                shortfall: None,
            };
        }

        let carriageway = target.carriageway_width();
        if carriageway.provenance == Provenance::Inferred {
            return FitState {
                status: FitStatus::CouldNotBeChecked,
                shortfall: None,
            };
        }

        if design_total_width_metres > carriageway.metres {
            return FitState {
                status: FitStatus::DoesNotFit,
                shortfall: Some(Dimension {
                    metres: design_total_width_metres - carriageway.metres,
                    // The shortfall is this Unit's own derived comparison,
                    // never read directly from an OSM tag or corrected by
                    // a user — `Inferred` states that plainly rather than
                    // borrowing the carriageway's own provenance.
                    provenance: Provenance::Inferred,
                }),
            };
        }

        FitState {
            status: FitStatus::Fits,
            shortfall: None,
        }
    }

    /// Applies `source_edits` to every target in `targets`, never
    /// atomically (BR10.1): a target whose `Street` is unavailable (its
    /// second tuple element is `None`, e.g. a lookup failure upstream)
    /// cannot be prepared at all and is recorded in
    /// `CorridorApplyOutcome.failed` with a reason, while every other
    /// target is still attempted. Applying past a `does_not_fit` warning
    /// applies every matched edit's value verbatim — no width is ever
    /// scaled (BR9.1). Records one undo entry across every street the
    /// apply actually touched (BR6.1).
    pub fn apply_to_corridor(
        session: &mut EditingSession,
        source_edits: &[LaneEdit],
        design_total_width_metres: f64,
        targets: &[(StreetIdentity, Option<&Street>)],
    ) -> CorridorApplyOutcome {
        let mut succeeded = Vec::new();
        let mut warned = Vec::new();
        let mut could_not_be_checked = Vec::new();
        let mut failed = Vec::new();

        let mut applied_edits = Vec::new();
        let mut affected_streets = Vec::new();

        for (target_street, maybe_street) in targets {
            let Some(street) = maybe_street else {
                failed.push((
                    target_street.clone(),
                    "target street could not be resolved for this corridor apply".to_string(),
                ));
                continue;
            };

            let correspondence = Self::build_correspondence(source_edits, street);
            let fit = Self::assess_fit(design_total_width_metres, street);

            for source_edit in source_edits
                .iter()
                .filter(|edit| correspondence.matched.contains(&edit.edit_id))
            {
                let mut extended = source_edit.clone();
                extended.edit_id = format!(
                    "{}-corridor-{}",
                    target_street.osm_way_id, source_edit.edit_id
                );
                extended.design_id = session.design().design_id().to_string();
                extended.target_street = target_street.clone();
                applied_edits.push(extended);
            }
            affected_streets.push(target_street.clone());

            match fit.status {
                FitStatus::Fits => succeeded.push(target_street.clone()),
                FitStatus::DoesNotFit => warned.push(target_street.clone()),
                FitStatus::CouldNotBeChecked => could_not_be_checked.push(target_street.clone()),
                FitStatus::NotYetChecked => {
                    // `assess_fit` always resolves to one of the other
                    // three states; this arm exists only because
                    // `FitStatus` is a shared four-state enum.
                    could_not_be_checked.push(target_street.clone());
                }
            }
        }

        if !applied_edits.is_empty() {
            session.apply_corridor_edits(applied_edits, affected_streets);
        }

        CorridorApplyOutcome {
            succeeded,
            warned,
            could_not_be_checked,
            failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use street_core::{
        BoundingNodeIds, Dimension, Direction, IntersectionId, KERB_BUFFER_LANE_TYPE, Lane,
        Provenance, Street, StreetIdentity, StreetNetworkGraph,
    };

    use crate::corridor_planner::CorridorPlanner;
    use crate::editing_session::EditingSession;
    use crate::entities::{FitStatus, LaneAttribute, LaneEdit, LaneEditKind};

    fn street_identity(way_id: &str, a: &str, b: &str) -> StreetIdentity {
        StreetIdentity {
            osm_way_id: way_id.to_string(),
            bounding_node_ids: BoundingNodeIds::new(a, b),
        }
    }

    fn mapped(metres: f64) -> Dimension {
        Dimension {
            metres,
            provenance: Provenance::Mapped,
        }
    }

    fn inferred(metres: f64) -> Dimension {
        Dimension {
            metres,
            provenance: Provenance::Inferred,
        }
    }

    // Test 13: the connected-street set is computed from a synthetic
    // `StreetNetworkGraph`'s adjacency, and an unnamed connected street
    // gets a stable human-readable fallback identifier (BR11.1, AC6.3.1,
    // AC6.3.3).
    #[test]
    fn connected_streets_come_from_graph_adjacency_with_a_fallback_name() {
        let source = street_identity("1", "100", "200");
        let connected = street_identity("2", "200", "300");
        let unrelated = street_identity("3", "500", "600");

        let source_street = Street::new(
            "1",
            BoundingNodeIds::new("100", "200"),
            Some("Main St".to_string()),
            vec![],
        );
        let connected_street = Street::new("2", BoundingNodeIds::new("200", "300"), None, vec![]);
        let unrelated_street = Street::new("3", BoundingNodeIds::new("500", "600"), None, vec![]);

        let intersection = IntersectionId::new("node-200");
        let mut adjacency = HashMap::new();
        adjacency.insert(intersection, vec![source.clone(), connected.clone()]);

        let graph = StreetNetworkGraph::new(
            "graph-1",
            vec![source_street, connected_street, unrelated_street],
            vec![IntersectionId::new("node-200")],
            adjacency,
        );

        let connected_streets = CorridorPlanner::connected_streets(&graph, &source);

        assert_eq!(connected_streets, vec![connected.clone()]);
        assert!(!connected_streets.contains(&unrelated));

        let display_name = CorridorPlanner::display_name(&graph, &connected);
        assert_eq!(display_name, "Unnamed street (way 2)");
    }

    // Test 14: the correspondence rule matches by
    // `(lane_type, ordinal_from_kerb)`; an edit with no matching target
    // lane is named unmatched and the target lane is untouched (BR7.1,
    // AC6.1.2, AC6.1.3).
    #[test]
    fn correspondence_matches_by_lane_type_and_ordinal_and_names_unmatched() {
        let target_street = Street::new(
            "2",
            BoundingNodeIds::new("200", "300"),
            None,
            vec![Lane::new("Travel", Direction::Forward, mapped(3.0), 0)],
        );

        let matching_edit = LaneEdit {
            edit_id: "edit-1".to_string(),
            design_id: "design-1".to_string(),
            target_street: street_identity("1", "100", "200"),
            target_lane_discriminator: Some("Travel|0".to_string()),
            lane_type: Some("Travel".to_string()),
            ordinal_from_kerb: Some(0),
            direction: None,
            anchor: None,
            kind: LaneEditKind::Change,
            attribute: Some(LaneAttribute::Width),
            value: None,
            width: Some(Dimension {
                metres: 3.5,
                provenance: Provenance::UserSet,
            }),
        };
        let mut unmatched_edit = matching_edit.clone();
        unmatched_edit.edit_id = "edit-2".to_string();
        unmatched_edit.target_lane_discriminator = Some("Parking|1".to_string());
        unmatched_edit.lane_type = Some("Parking".to_string());
        unmatched_edit.ordinal_from_kerb = Some(1);

        let result =
            CorridorPlanner::build_correspondence(&[matching_edit, unmatched_edit], &target_street);

        assert_eq!(result.matched, vec!["edit-1".to_string()]);
        assert_eq!(result.unmatched, vec!["edit-2".to_string()]);
    }

    // Test 15: extending onto a target leaves the target's own
    // corrections and baseline intact — only a new design-layer entry is
    // added (BR7.1, AC6.1.4).
    #[test]
    fn extending_onto_a_target_only_adds_a_design_layer_entry() {
        let target_id = street_identity("2", "200", "300");
        let mut session = EditingSession::new("session-1", "design-1");

        let matching_edit = LaneEdit {
            edit_id: "edit-1".to_string(),
            design_id: "design-1".to_string(),
            target_street: street_identity("1", "100", "200"),
            target_lane_discriminator: Some("Travel|0".to_string()),
            lane_type: Some("Travel".to_string()),
            ordinal_from_kerb: Some(0),
            direction: None,
            anchor: None,
            kind: LaneEditKind::Change,
            attribute: Some(LaneAttribute::Width),
            value: None,
            width: Some(Dimension {
                metres: 3.5,
                provenance: Provenance::UserSet,
            }),
        };

        CorridorPlanner::extend_onto_target(&mut session, &matching_edit, &target_id);

        let target_edits: Vec<&LaneEdit> = session
            .design()
            .edits()
            .iter()
            .filter(|edit| edit.target_street == target_id)
            .collect();
        assert_eq!(target_edits.len(), 1);
        assert_eq!(target_edits[0].width.unwrap().metres, 3.5);
        // The source street's own edit set is untouched.
        assert_eq!(session.design().edits().len(), 1);
    }

    // Test 16: fit assessment returns `fits`/`does_not_fit` (with
    // shortfall) correctly for a target whose carriageway width is at
    // least partially `Mapped` (BR8.1, AC6.2.1).
    #[test]
    fn fit_assessment_reports_fits_and_does_not_fit_with_shortfall() {
        let target = Street::new(
            "2",
            BoundingNodeIds::new("200", "300"),
            None,
            vec![
                Lane::new(KERB_BUFFER_LANE_TYPE, Direction::Forward, mapped(0.3), 0),
                Lane::new("Travel", Direction::Forward, mapped(3.0), 1),
                Lane::new("Travel", Direction::Backward, mapped(3.0), 2),
                Lane::new(KERB_BUFFER_LANE_TYPE, Direction::Backward, mapped(0.3), 3),
            ],
        );

        let fits = CorridorPlanner::assess_fit(5.0, &target);
        assert_eq!(fits.status, FitStatus::Fits);
        assert!(fits.shortfall.is_none());

        let does_not_fit = CorridorPlanner::assess_fit(8.0, &target);
        assert_eq!(does_not_fit.status, FitStatus::DoesNotFit);
        let shortfall = does_not_fit.shortfall.expect("a shortfall");
        assert!((shortfall.metres - 2.0).abs() < f64::EPSILON);
    }

    // Test 17: fit assessment returns `could_not_be_checked` for a target
    // whose carriageway width is wholly `Inferred` or absent (BR8.1,
    // AC6.2.3, AC6.2.4).
    #[test]
    fn fit_assessment_reports_could_not_be_checked_when_wholly_inferred_or_absent() {
        let wholly_inferred = Street::new(
            "2",
            BoundingNodeIds::new("200", "300"),
            None,
            vec![
                Lane::new("Travel", Direction::Forward, inferred(3.0), 0),
                Lane::new("Travel", Direction::Backward, inferred(3.0), 1),
            ],
        );
        let absent = Street::new("3", BoundingNodeIds::new("300", "400"), None, vec![]);

        assert_eq!(
            CorridorPlanner::assess_fit(5.0, &wholly_inferred).status,
            FitStatus::CouldNotBeChecked
        );
        assert_eq!(
            CorridorPlanner::assess_fit(5.0, &absent).status,
            FitStatus::CouldNotBeChecked
        );
    }

    // Test 18: applying past a `does_not_fit` warning applies every
    // value verbatim — no width is scaled (BR9.1, AC6.2.2).
    #[test]
    fn applying_past_a_does_not_fit_warning_applies_verbatim() {
        let target_id = street_identity("2", "200", "300");
        let target = Street::new(
            "2",
            BoundingNodeIds::new("200", "300"),
            None,
            vec![Lane::new("Travel", Direction::Forward, mapped(3.0), 0)],
        );
        let mut session = EditingSession::new("session-1", "design-1");

        let oversized_edit = LaneEdit {
            edit_id: "edit-1".to_string(),
            design_id: "design-1".to_string(),
            target_street: street_identity("1", "100", "200"),
            target_lane_discriminator: Some("Travel|0".to_string()),
            lane_type: Some("Travel".to_string()),
            ordinal_from_kerb: Some(0),
            direction: None,
            anchor: None,
            kind: LaneEditKind::Change,
            attribute: Some(LaneAttribute::Width),
            value: None,
            width: Some(Dimension {
                metres: 12.0,
                provenance: Provenance::UserSet,
            }),
        };

        let outcome = CorridorPlanner::apply_to_corridor(
            &mut session,
            &[oversized_edit],
            12.0,
            &[(target_id.clone(), Some(&target))],
        );

        assert_eq!(outcome.warned, vec![target_id.clone()]);
        let applied_edit = session
            .design()
            .edits()
            .iter()
            .find(|edit| edit.target_street == target_id)
            .expect("the edit was applied to the target");
        assert_eq!(applied_edit.width.unwrap().metres, 12.0);
    }

    // Test 19: a bulk apply where one target street cannot be prepared:
    // the other target streets still succeed, and the failed one is
    // named with a reason in `CorridorApplyOutcome.failed` (BR10.1,
    // AC6.4.2, AC6.4.3).
    #[test]
    fn bulk_apply_continues_past_one_failed_target() {
        let good_target_id = street_identity("2", "200", "300");
        let good_target = Street::new(
            "2",
            BoundingNodeIds::new("200", "300"),
            None,
            vec![Lane::new("Travel", Direction::Forward, mapped(3.0), 0)],
        );
        let failing_target_id = street_identity("3", "300", "400");

        let mut session = EditingSession::new("session-1", "design-1");
        let edit = LaneEdit {
            edit_id: "edit-1".to_string(),
            design_id: "design-1".to_string(),
            target_street: street_identity("1", "100", "200"),
            target_lane_discriminator: Some("Travel|0".to_string()),
            lane_type: Some("Travel".to_string()),
            ordinal_from_kerb: Some(0),
            direction: None,
            anchor: None,
            kind: LaneEditKind::Change,
            attribute: Some(LaneAttribute::Width),
            value: None,
            width: Some(Dimension {
                metres: 2.5,
                provenance: Provenance::UserSet,
            }),
        };

        let outcome = CorridorPlanner::apply_to_corridor(
            &mut session,
            &[edit],
            2.5,
            &[
                (good_target_id.clone(), Some(&good_target)),
                (failing_target_id.clone(), None),
            ],
        );

        assert_eq!(outcome.succeeded, vec![good_target_id]);
        assert_eq!(outcome.failed.len(), 1);
        assert_eq!(outcome.failed[0].0, failing_target_id);
        assert!(!outcome.failed[0].1.is_empty());
    }
}
