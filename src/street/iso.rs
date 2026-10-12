//! The street in 2.5D: a straight stretch of it seen from above and to one side, each piece an
//! extruded strip with its height, the few things that stand on the pieces as plain solids, and the
//! one-way arrows on the roadway. It is drawn from the same view the section is and is read-only:
//! the section is where the street is edited.

/// How long a stretch of the street is shown, in millimetres.
pub const STRETCH_MM: f64 = 40_000.0;
/// How far a millimetre along the street moves the drawing right, and up: half scale, at the
/// angle of a 3-4-5 triangle (about 37 degrees), so the points it gives are exact.
pub const RECEDE_X: f64 = 0.4;
pub const RECEDE_Y: f64 = 0.3;
/// The tallest the drawing grows, so that it stays about as tall as the section.
pub const MAX_HEIGHT: f64 = 560.0;
/// Room round the scene, in pixels.
const PAD: f64 = 30.0;
/// The narrowest the drawing is laid out for: the section's group is no narrower.
const MIN_WIDTH: f64 = 600.0;
/// Room above the far end of the ground for the tallest solid, a tree's crown at 5 m.
const HEADROOM_MM: f64 = 5_500.0;

/// How a point of the street, in millimetres across it, up from the ground and along it from the
/// near end, lands in the drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    /// Pixels to the millimetre.
    pub scale: f64,
    /// Where the street's left edge is at its near end, in pixels.
    pub left: f64,
    /// Where the ground is at the near end, in pixels from the top.
    pub base: f64,
    pub width: f64,
    pub height: f64,
}

impl Projection {
    /// The projection for a drawing `width` wide showing a street `span_mm` wide: the scene fills
    /// the width, unless that would make the drawing taller than `MAX_HEIGHT`; then it is centred.
    pub fn fit(width: f64, span_mm: i32) -> Projection {
        let width = width.max(MIN_WIDTH);
        let across = span_mm.max(1) as f64 + STRETCH_MM * RECEDE_X;
        let up = STRETCH_MM * RECEDE_Y + HEADROOM_MM;
        let scale = ((width - 2.0 * PAD) / across).min((MAX_HEIGHT - 2.0 * PAD) / up);
        let left = (width - across * scale) / 2.0;
        let base = PAD + up * scale;
        Projection { scale, left, base, width, height: base + PAD }
    }

    /// Where a point is in the drawing, in pixels.
    pub fn at(&self, x_mm: f64, up_mm: f64, along_mm: f64) -> (f64, f64) {
        (self.left + (x_mm + along_mm * RECEDE_X) * self.scale, self.base - (up_mm + along_mm * RECEDE_Y) * self.scale)
    }
}

/// How high each kind of piece stands above the roadway, in millimetres. They are plausible, not
/// surveyed, like the catalogue's rates. The furniture kinds stand on the sidewalk, so at its height.
pub const HEIGHTS: [(&str, i32); 17] = [
    ("sidewalk", 150),
    ("planting", 150),
    ("bike", 0),
    ("travel", 0),
    ("bus", 0),
    ("parking", 0),
    ("median", 150),
    ("loading", 0),
    ("shoulder", 0),
    ("bikerack", 150),
    ("bikeshare", 150),
    ("pole", 150),
    ("busshelter", 150),
    ("busstation", 150),
    ("bench", 150),
    ("terrace", 150),
    ("streetlamp", 150),
];

/// How high a piece of this kind stands, in millimetres.
pub fn height_mm(kind_id: &str) -> i32 {
    HEIGHTS.iter().find(|(id, _)| *id == kind_id).map_or(0, |(_, h)| *h)
}

/// How high a curb stands above the roadway, in millimetres: a bike-friendly curb is a low ramp, a
/// Kassel kerb and a bus boarding island are raised for boarding.
pub fn curb_height_mm(curb: &str) -> i32 {
    match curb {
        "bikefriendly" => 60,
        "kassel" => 180,
        "island" => 200,
        _ => 150,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::KINDS;

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn the_projection_puts_the_corners_of_a_unit_box_exactly_where_the_oblique_view_has_them() {
        // 31 m of street and 16 m of depth across 940 px: 0.02 px to the millimetre.
        let p = Projection::fit(1000.0, 31_000);
        assert!((p.scale - 0.02).abs() < 1e-12, "{}", p.scale);
        assert_eq!((p.left, p.width), (30.0, 1000.0));
        assert!((p.base - 380.0).abs() < 1e-9 && (p.height - 410.0).abs() < 1e-9, "{p:?}");
        // Width runs right at full scale, height runs up, depth runs up and right at half scale.
        assert!(close(p.at(0.0, 0.0, 0.0), (30.0, 380.0)));
        assert!(close(p.at(1_000.0, 0.0, 0.0), (50.0, 380.0)));
        assert!(close(p.at(0.0, 1_000.0, 0.0), (30.0, 360.0)));
        assert!(close(p.at(0.0, 0.0, 1_000.0), (38.0, 374.0)));
        assert!(close(p.at(1_000.0, 1_000.0, 1_000.0), (58.0, 354.0)));
    }

    #[test]
    fn the_scene_fits_the_width_and_a_narrow_street_is_held_to_the_height_and_centred() {
        // A wide street: the width decides, the scene spans the drawing less its margins.
        let wide = Projection::fit(1000.0, 31_000);
        let across = 31_000.0 + STRETCH_MM * RECEDE_X;
        assert!((wide.left + across * wide.scale - (1000.0 - 30.0)).abs() < 1e-9);
        // A narrow street: the height decides, so the drawing is no taller than MAX_HEIGHT, and the scene is centred.
        let narrow = Projection::fit(1000.0, 4_000);
        assert!((narrow.height - MAX_HEIGHT).abs() < 1e-9, "{narrow:?}");
        let across = 4_000.0 + STRETCH_MM * RECEDE_X;
        assert!((narrow.left - (1000.0 - (narrow.left + across * narrow.scale))).abs() < 1e-9, "centred: {narrow:?}");
        // The drawing is laid out no narrower than the section's group.
        assert_eq!(Projection::fit(100.0, 18_000).width, 600.0);
        // A street of no width still gives a scale.
        assert!(Projection::fit(1000.0, 0).scale.is_finite());
    }

    #[test]
    fn every_kind_has_one_height_and_the_table_names_no_other() {
        for k in &KINDS {
            assert_eq!(HEIGHTS.iter().filter(|(id, _)| *id == k.id).count(), 1, "{}", k.id);
        }
        for (id, _) in &HEIGHTS {
            assert!(KINDS.iter().any(|k| k.id == *id), "{id} is not a kind");
        }
    }

    #[test]
    fn the_roadway_is_at_the_ground_and_the_sidewalk_median_and_planting_stand_150_mm_above_it() {
        for id in ["travel", "bus", "parking", "loading", "shoulder", "bike"] {
            assert_eq!(height_mm(id), 0, "{id}");
        }
        for id in ["sidewalk", "median", "planting"] {
            assert_eq!(height_mm(id), 150, "{id}");
        }
        assert_eq!(height_mm("nonsense"), 0);
    }

    #[test]
    fn a_curb_stands_150_mm_and_a_ramp_an_island_and_a_kassel_kerb_have_their_own_step() {
        assert_eq!(curb_height_mm("concrete"), 150);
        assert_eq!(curb_height_mm("granite"), 150);
        assert_eq!(curb_height_mm("bikefriendly"), 60);
        assert_eq!(curb_height_mm("kassel"), 180);
        assert_eq!(curb_height_mm("island"), 200);
    }
}
