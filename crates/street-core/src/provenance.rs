//! `Provenance` and `Dimension` — BR1.1, BR1.2, BR6.1.

/// The origin of a single [`Dimension`]'s value. A closed enum, not a
/// nullable field — the "no provenance" state does not exist (BR1.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provenance {
    /// Read directly from an OpenStreetMap tag.
    Mapped,
    /// Derived by osm2streets from other tags or defaults.
    Inferred,
    /// Corrected or entered by a user. Once set, never reverts to
    /// `Mapped` or `Inferred` (BR1.2).
    UserSet,
}

/// A physical quantity — a width, a count, an offset — paired with its
/// [`Provenance`]. Metres is the only unit stored; conversion happens
/// only at the presentation boundary (BR6.1). Both fields are required,
/// so a `Dimension` cannot be constructed without stating where its
/// value came from (BR1.1) — there is no `Default` impl and no
/// constructor that omits `provenance`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dimension {
    /// The quantity, in metres.
    pub metres: f64,
    /// Where this value came from.
    pub provenance: Provenance,
}

impl Dimension {
    /// Applies a user correction: the returned `Dimension` always carries
    /// [`Provenance::UserSet`], regardless of this `Dimension`'s prior
    /// provenance (BR1.2) — there is no operation on this type that sets
    /// `UserSet` back to `Mapped` or `Inferred`. This never mutates
    /// `self`: the imported baseline is immutable (BR5.1), so callers
    /// hold the returned value in their own overlay rather than writing
    /// back into the baseline.
    #[must_use]
    pub fn corrected(&self, new_metres: f64) -> Dimension {
        Dimension {
            metres: new_metres,
            provenance: Provenance::UserSet,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_stores_metres_and_provenance() {
        let dimension = Dimension {
            metres: 3.5,
            provenance: Provenance::Mapped,
        };

        assert_eq!(dimension.metres, 3.5);
        assert_eq!(dimension.provenance, Provenance::Mapped);
    }

    #[test]
    fn provenance_variants_are_distinct() {
        assert_ne!(Provenance::Mapped, Provenance::Inferred);
        assert_ne!(Provenance::Mapped, Provenance::UserSet);
        assert_ne!(Provenance::Inferred, Provenance::UserSet);
    }

    #[test]
    fn correcting_a_mapped_dimension_sets_user_set() {
        let mapped = Dimension {
            metres: 3.0,
            provenance: Provenance::Mapped,
        };

        let corrected = mapped.corrected(3.2);

        assert_eq!(corrected.provenance, Provenance::UserSet);
    }

    #[test]
    fn correcting_an_inferred_dimension_sets_user_set() {
        let inferred = Dimension {
            metres: 2.7,
            provenance: Provenance::Inferred,
        };

        let corrected = inferred.corrected(2.9);

        assert_eq!(corrected.provenance, Provenance::UserSet);
    }

    #[test]
    fn correcting_a_user_set_dimension_stays_user_set() {
        let already_corrected = Dimension {
            metres: 3.2,
            provenance: Provenance::UserSet,
        };

        let corrected_again = already_corrected.corrected(3.4);

        assert_eq!(corrected_again.provenance, Provenance::UserSet);
    }

    #[test]
    fn correcting_a_dimension_updates_its_metres_value() {
        let mapped = Dimension {
            metres: 3.0,
            provenance: Provenance::Mapped,
        };

        let corrected = mapped.corrected(3.75);

        assert_eq!(corrected.metres, 3.75);
    }
}
