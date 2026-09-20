//! `ImportFingerprint` — what was imported at the moment a `Correction` was
//! made, the basis for re-import reconciliation (`entities.md`
//! `ImportFingerprint`; BR8.1).

use std::collections::BTreeMap;

/// This project's own driving-side vocabulary — never `osm2streets::
/// DrivingSide` itself, so `ImportFingerprint` (used by `CorrectionOverlay`,
/// which must not depend on `osm2streets`) never re-exports an
/// osm2streets-native type past the adapter boundary (`team.md`, "three
/// inward-pointing layers").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrivingSide {
    Left,
    Right,
}

/// The subset of osm2streets' `MapConfig` that affects lane inference and
/// is therefore part of what a re-import must match to reconcile silently
/// (`entities.md` `ImportFingerprint.map_config`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FingerprintMapConfig {
    pub country_code: String,
    pub driving_side: DrivingSide,
    pub inferred_sidewalks: bool,
    pub inferred_kerbs: bool,
}

/// What was imported at the moment a [`crate::Correction`] was made —
/// basis for re-import reconciliation (BR8.1). Two `ImportFingerprint`s
/// compare equal only when every field matches exactly; `CorrectionOverlay`
/// uses that equality for the `reapplied_silently` outcome.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportFingerprint {
    /// The raw OSM tag key/value pairs on the imported way, at the moment
    /// of the correction.
    pub way_tags: BTreeMap<String, String>,
    pub map_config: FingerprintMapConfig,
    /// The pinned `osm2streets` commit SHA the import ran against
    /// (`docs/osm2streets-pin.md`).
    pub osm2streets_revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_fingerprint() -> ImportFingerprint {
        ImportFingerprint {
            way_tags: BTreeMap::from([("highway".to_string(), "secondary".to_string())]),
            map_config: FingerprintMapConfig {
                country_code: "US".to_string(),
                driving_side: DrivingSide::Right,
                inferred_sidewalks: false,
                inferred_kerbs: true,
            },
            osm2streets_revision: crate::OSM2STREETS_REVISION.to_string(),
        }
    }

    #[test]
    fn identical_fingerprints_are_equal() {
        assert_eq!(a_fingerprint(), a_fingerprint());
    }

    #[test]
    fn a_changed_tag_makes_fingerprints_unequal() {
        let mut changed = a_fingerprint();
        changed
            .way_tags
            .insert("width".to_string(), "16".to_string());

        assert_ne!(a_fingerprint(), changed);
    }

    #[test]
    fn a_changed_osm2streets_revision_makes_fingerprints_unequal() {
        let mut changed = a_fingerprint();
        changed.osm2streets_revision = "different-revision".to_string();

        assert_ne!(a_fingerprint(), changed);
    }
}
