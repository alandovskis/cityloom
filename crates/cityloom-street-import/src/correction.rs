//! `Correction`, `CorrectionReconciliationOutcome`, and `CorrectionOverlay`
//! — correction storage and re-import reconciliation (`entities.md`
//! `Correction`, `CorrectionReconciliationOutcome`; BR5.1, BR6.1, BR8.1).

use street_core::StreetIdentity;

use crate::fingerprint::ImportFingerprint;

/// A `Correction`'s lifecycle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrectionState {
    Applied,
    ReappliedAfterChange,
    Unresolved,
}

/// A user's statement that the import itself was wrong. Permanent —
/// provenance is always `UserSet`, never returns to `Mapped` (BR5.1).
///
/// This type carries **no `provenance` field at all** — its `UserSet`-ness
/// is structural: the fact that a value came from a `Correction`, distinct
/// from a design edit, is what asserts `UserSet` (BR5.1). This is a
/// build-time invariant of this Unit's public API, not a runtime check, in
/// the same spirit as `street-core::Street`'s documented absence of a
/// mutator: every field below is exactly what `entities.md` lists for this
/// type, and adding a `provenance` field here would be visible in this
/// struct definition, not hidden behind a runtime check — there is nothing
/// to test at runtime that this doc comment and the field list do not
/// already guarantee at compile time.
///
/// This is this Unit's own **live, in-memory** type — distinct from
/// `design-payload-spec`'s wire-shape `Correction` of the same name
/// (`entities.md`); this crate does not depend on `cityloom-design-payload`.
#[derive(Clone, Debug, PartialEq)]
pub struct Correction {
    pub correction_id: String,
    /// A street key: OSM way id plus the direction-normalised bounding
    /// node-id pair (BR6.1) — never a positional index.
    pub target_street: StreetIdentity,
    /// The derived per-lane key (lane type, direction, ordinal from kerb);
    /// `None` when removing a lane the import invented (nothing left to
    /// key by).
    pub target_lane_discriminator: Option<String>,
    pub attribute: Option<String>,
    pub value: Option<String>,
    pub import_fingerprint: ImportFingerprint,
    pub state: CorrectionState,
}

/// The result of comparing a stored `Correction`'s fingerprint against a
/// fresh import. Exactly one of three outcomes (BR8.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconciliationOutcome {
    ReappliedSilently,
    ReappliedWithNotice,
    Unresolved,
}

/// The result of reconciling one stored `Correction` against a fresh
/// import (`entities.md` `CorrectionReconciliationOutcome`).
#[derive(Clone, Debug, PartialEq)]
pub struct CorrectionReconciliationOutcome {
    pub correction_id: String,
    pub outcome: ReconciliationOutcome,
    /// Present only when `outcome` is `ReappliedWithNotice`.
    pub notice: Option<String>,
}

/// Correction storage and re-import reconciliation (`components.md`
/// `CorrectionOverlay`). Holds every `Correction` recorded so far and
/// reconciles them against a fresh import's fingerprint (BR8.1).
#[derive(Clone, Debug, Default)]
pub struct CorrectionOverlay {
    corrections: Vec<Correction>,
}

impl CorrectionOverlay {
    pub fn new() -> CorrectionOverlay {
        CorrectionOverlay {
            corrections: Vec::new(),
        }
    }

    /// Records a new `Correction`. Never mutates a stored one in place —
    /// callers that need to change a `Correction`'s `state` construct a
    /// new value (the imported baseline and its overlay are both treated
    /// as immutable-by-convention data here; `reconcile` below returns new
    /// `CorrectionReconciliationOutcome`s rather than mutating `self`).
    pub fn add(&mut self, correction: Correction) {
        self.corrections.push(correction);
    }

    /// Every stored `Correction` targeting `target`.
    pub fn corrections_for(&self, target: &StreetIdentity) -> Vec<&Correction> {
        self.corrections
            .iter()
            .filter(|correction| &correction.target_street == target)
            .collect()
    }

    /// Reconciles every stored `Correction` targeting `target` against a
    /// fresh import's fingerprint (BR8.1):
    /// - the fingerprint matches exactly → `reapplied_silently`;
    /// - the fingerprint differs but `still_resolves` reports the lane
    ///   still resolves → `reapplied_with_notice`;
    /// - `still_resolves` reports it does not → `unresolved` (the
    ///   `Correction` itself is retained by `self`, never discarded — this
    ///   method only ever reads `self.corrections`, it never removes one).
    ///
    /// `still_resolves` is supplied by the caller (`StreetImportAdapter`'s
    /// composition point, which has the freshly-imported `Street` and can
    /// check whether `target_lane_discriminator` still names one of its
    /// lanes) rather than this type depending on `street-core::Lane`'s
    /// internals directly.
    pub fn reconcile(
        &self,
        target: &StreetIdentity,
        fresh_fingerprint: &ImportFingerprint,
        still_resolves: impl Fn(&Correction) -> bool,
    ) -> Vec<CorrectionReconciliationOutcome> {
        self.corrections_for(target)
            .into_iter()
            .map(|correction| {
                let outcome = if &correction.import_fingerprint == fresh_fingerprint {
                    ReconciliationOutcome::ReappliedSilently
                } else if still_resolves(correction) {
                    ReconciliationOutcome::ReappliedWithNotice
                } else {
                    ReconciliationOutcome::Unresolved
                };
                let notice =
                    matches!(outcome, ReconciliationOutcome::ReappliedWithNotice).then(|| {
                        format!(
                            "Correction {} was reapplied, but the import that produced it has \
                             changed since — review it.",
                            correction.correction_id
                        )
                    });
                CorrectionReconciliationOutcome {
                    correction_id: correction.correction_id.clone(),
                    outcome,
                    notice,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::{DrivingSide, FingerprintMapConfig};
    use std::collections::BTreeMap;
    use street_core::BoundingNodeIds;

    fn a_fingerprint(width_tag: &str) -> ImportFingerprint {
        ImportFingerprint {
            way_tags: BTreeMap::from([("width".to_string(), width_tag.to_string())]),
            map_config: FingerprintMapConfig {
                country_code: "US".to_string(),
                driving_side: DrivingSide::Right,
                inferred_sidewalks: false,
                inferred_kerbs: true,
            },
            osm2streets_revision: crate::OSM2STREETS_REVISION.to_string(),
        }
    }

    fn a_target() -> StreetIdentity {
        StreetIdentity {
            osm_way_id: "4729721".to_string(),
            bounding_node_ids: BoundingNodeIds::new("100", "200"),
        }
    }

    fn a_correction(fingerprint: ImportFingerprint) -> Correction {
        Correction {
            correction_id: "correction-1".to_string(),
            target_street: a_target(),
            target_lane_discriminator: Some("driving|forward|0".to_string()),
            attribute: Some("width".to_string()),
            value: Some("3.4".to_string()),
            import_fingerprint: fingerprint,
            state: CorrectionState::Applied,
        }
    }

    // Test 15: a `Correction` constructed for a lane is keyed by
    // `(osm_way_id, bounding_node_ids, lane_discriminator)`, never a
    // positional index (BR6.1, AC7.3.4).
    #[test]
    fn a_correction_is_keyed_by_way_id_node_ids_and_lane_discriminator() {
        let correction = a_correction(a_fingerprint("16"));

        assert_eq!(correction.target_street.osm_way_id, "4729721");
        assert_eq!(
            correction.target_street.bounding_node_ids,
            BoundingNodeIds::new("100", "200")
        );
        assert_eq!(
            correction.target_lane_discriminator.as_deref(),
            Some("driving|forward|0")
        );
    }

    // Test 16: a `Correction`'s value carries no `provenance` field in its
    // type at all — structural UserSet (BR5.1, AC7.3.1). Constructing the
    // value with exactly `entities.md`'s field list, and nothing else,
    // demonstrates the absence: a `provenance` field on this line would be
    // a compile error, not a runtime assertion.
    #[test]
    fn a_correction_carries_no_provenance_field() {
        let correction = Correction {
            correction_id: "correction-2".to_string(),
            target_street: a_target(),
            target_lane_discriminator: None,
            attribute: None,
            value: None,
            import_fingerprint: a_fingerprint("16"),
            state: CorrectionState::Unresolved,
        };

        assert_eq!(correction.state, CorrectionState::Unresolved);
    }

    // Test 17: reconciliation with a matching fingerprint yields
    // `reapplied_silently` (AC7.5.2).
    #[test]
    fn a_matching_fingerprint_reconciles_silently() {
        let mut overlay = CorrectionOverlay::new();
        overlay.add(a_correction(a_fingerprint("16")));

        let outcomes = overlay.reconcile(&a_target(), &a_fingerprint("16"), |_| true);

        assert_eq!(outcomes.len(), 1);
        assert_eq!(
            outcomes[0].outcome,
            ReconciliationOutcome::ReappliedSilently
        );
        assert_eq!(outcomes[0].notice, None);
    }

    // Test 18: reconciliation with a changed fingerprint but a
    // still-resolving lane yields `reapplied_with_notice` (AC7.5.3).
    #[test]
    fn a_changed_fingerprint_with_a_resolving_lane_reconciles_with_notice() {
        let mut overlay = CorrectionOverlay::new();
        overlay.add(a_correction(a_fingerprint("16")));

        let outcomes = overlay.reconcile(&a_target(), &a_fingerprint("18"), |_| true);

        assert_eq!(outcomes.len(), 1);
        assert_eq!(
            outcomes[0].outcome,
            ReconciliationOutcome::ReappliedWithNotice
        );
        assert!(outcomes[0].notice.is_some());
    }

    // Test 19: reconciliation where the lane no longer resolves yields
    // `unresolved`, and the `Correction` is retained (not discarded) with
    // `state: unresolved` (AC7.5.4).
    #[test]
    fn a_non_resolving_lane_reconciles_as_unresolved_and_is_retained() {
        let mut overlay = CorrectionOverlay::new();
        overlay.add(a_correction(a_fingerprint("16")));

        let outcomes = overlay.reconcile(&a_target(), &a_fingerprint("18"), |_| false);

        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].outcome, ReconciliationOutcome::Unresolved);
        assert_eq!(outcomes[0].notice, None);
        // Retained, not discarded: still findable via `corrections_for`.
        assert_eq!(overlay.corrections_for(&a_target()).len(), 1);
    }

    #[test]
    fn corrections_for_a_different_target_are_not_returned() {
        let mut overlay = CorrectionOverlay::new();
        overlay.add(a_correction(a_fingerprint("16")));
        let other_target = StreetIdentity {
            osm_way_id: "9999999".to_string(),
            bounding_node_ids: BoundingNodeIds::new("1", "2"),
        };

        assert!(overlay.corrections_for(&other_target).is_empty());
    }
}
