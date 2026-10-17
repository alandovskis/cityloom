//! The surface materials' patterns laid on the ground plane of a 2.5D drawing.
//!
//! The section's `m-*` patterns are drawn flat on the screen. In an oblique view that looks pasted on, so the
//! drawing carries its own copy of each, with a `patternTransform` that maps the pattern's plane onto the ground:
//! the same pattern, foreshortened as the pavement is. The caller gives the matrix of its own projection.

use crate::shared::symbols::MATERIAL_HATCH;

/// How many millimetres of ground one unit of a pattern covers: a brick paver's 12 by 8 units are then 480 by 320 mm,
/// a paving stone, not a street. An estimate, like the symbols' sizes.
pub const MM_PER_UNIT: f64 = 40.0;

/// Where a ground point lands on the screen, as the six numbers of an SVG matrix: `x_px = a*x + c*y + e` and
/// `y_px = b*x + d*y + f`, with `x` and `y` in millimetres on the ground.
pub type Ground = [f64; 6];

/// The materials' patterns for a drawing's `<defs>`, each with the id `{prefix}-{material}` (the section's is
/// `m-{material}`), laid on the ground by `ground`.
pub fn patterns(prefix: &str, ground: Ground) -> String {
    let [a, b, c, d, e, f] = ground;
    let k = MM_PER_UNIT;
    let matrix = format!("matrix({:.4} {:.4} {:.4} {:.4} {:.2} {:.2})", a * k, b * k, c * k, d * k, e, f);
    MATERIAL_HATCH
        .iter()
        .map(|(id, pattern)| pattern.replacen(&format!("<pattern id=\"m-{id}\""), &format!("<pattern patternTransform=\"{matrix}\" id=\"{prefix}-{id}\""), 1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_material_gets_its_pattern_under_the_prefix_with_the_ground_transform() {
        let out = patterns("g", [0.5, 0.0, 0.2, -0.15, 10.0, 300.0]);
        assert_eq!(out.matches("<pattern ").count(), MATERIAL_HATCH.len());
        for (id, _) in MATERIAL_HATCH {
            assert!(out.contains(&format!("id=\"g-{id}\"")), "{id}");
            assert!(!out.contains(&format!("id=\"m-{id}\"")), "{id}: the section's id must not be reused");
        }
        assert!(out.contains("patternTransform=\"matrix(20.0000 0.0000 8.0000 -6.0000 10.00 300.00)\""));
        assert!(out.contains("patternUnits=\"userSpaceOnUse\""));
    }
}
