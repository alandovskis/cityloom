//! The surface materials' textures laid on the ground plane of a 2.5D drawing.
//!
//! The section's `m-*` patterns are flat hatches in screen units. The 2.5D carries its own: each material's texture
//! drawn in millimetres of ground (a brick is 200 by 100 mm) with a `patternTransform` that maps the ground onto the
//! drawing in its oblique projection, so the texture lies on the pavement and recedes with it. Colours are the
//! material's own, from the `--mat-*` tokens, and do not change with the theme.
//!
//! At the 2.5D's scale (about 0.03 px to the millimetre for a street) the finest grain of a material cannot be
//! seen, so each texture draws the coarsest detail that still reads: an asphalt's speckle is of 20 to 30 mm
//! stones, not 2 mm grains, a paver's joint is 25 mm wide, a concrete's broom finish is a faint line every 90 mm.

use std::fmt::Write;

use crate::shared::catalogue::MATERIALS;

/// Where a ground point lands on the screen, as the six numbers of an SVG matrix: `x_px = a*x + c*y + e` and
/// `y_px = b*x + d*y + f`, with `x` and `y` in millimetres on the ground.
pub type Ground = [f64; 6];

/// The size in millimetres of one tile of a material's texture, which repeats across the ground.
pub fn tile_mm(material: &str) -> (f64, f64) {
    match material {
        "concrete" => (1800.0, 1800.0),
        "permeable" => (500.0, 250.0),
        "brick" => (800.0, 400.0),
        "grass" | "trees" => (400.0, 400.0),
        "gravel" => (500.0, 500.0),
        _ => (600.0, 600.0),
    }
}

/// A small deterministic generator, so that a texture is the same in every drawing.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as f64 / (1u64 << 31) as f64
    }
    fn between(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

/// The marks of a material's texture within its tile, as SVG; colours are the material's tokens.
fn marks(material: &str) -> String {
    let (w, h) = tile_mm(material);
    let line = format!("var(--mat-{material}-line)");
    let alt = format!("var(--mat-{material}-alt)");
    let mut out = String::new();
    let mut r = Lcg(material.bytes().fold(7u64, |a, b| a * 31 + b as u64));
    match material {
        // Concrete: a panel's joints on two edges of the tile (a 1.8 m panel), and a broom finish.
        "concrete" => {
            let mut y = 45.0;
            while y < h {
                write!(out, "<path d=\"M0,{y:.0}H{w}\" style=\"stroke:{alt};stroke-width:9;stroke-opacity:0.35\"/>").unwrap();
                y += 90.0;
            }
            write!(out, "<path d=\"M0,0H{w}M0,0V{h}\" style=\"stroke:{line};stroke-width:30;fill:none\"/>").unwrap();
        }
        // Permeable pavers: 225 by 100 mm stones in two courses, laid with a 25 mm gap.
        "permeable" => {
            write!(out, "<path d=\"M0,0H{w}M0,125H{w}M0,0V125M250,125V250\" style=\"stroke:{line};stroke-width:25;fill:none\"/>").unwrap();
            write!(out, "<rect x=\"262\" y=\"12\" width=\"225\" height=\"100\" style=\"fill:{alt};fill-opacity:0.5\"/>").unwrap();
        }
        // Brick in running bond: 200 by 100 mm bricks, the courses a half brick out, a few bricks of another tone.
        "brick" => {
            for (x, y) in [(200.0, 100.0), (600.0, 300.0), (0.0, 200.0)] {
                write!(out, "<rect x=\"{x}\" y=\"{y}\" width=\"200\" height=\"100\" style=\"fill:{alt};fill-opacity:0.6\"/>").unwrap();
            }
            let mut d = String::new();
            for course in 0..4 {
                let y = course as f64 * 100.0;
                write!(d, "M0,{y}H{w}").unwrap();
                let shift = if course % 2 == 0 { 0.0 } else { 100.0 };
                for k in 0..5 {
                    write!(d, "M{},{y}V{}", shift + k as f64 * 200.0, y + 100.0).unwrap();
                }
            }
            write!(out, "<path d=\"{d}\" style=\"stroke:{line};stroke-width:16;fill:none\"/>").unwrap();
        }
        // Grass: mottled patches and short blades.
        "grass" => {
            for _ in 0..5 {
                write!(
                    out,
                    "<ellipse cx=\"{:.0}\" cy=\"{:.0}\" rx=\"{:.0}\" ry=\"{:.0}\" style=\"fill:{alt};fill-opacity:0.35\"/>",
                    r.between(0.0, w),
                    r.between(0.0, h),
                    r.between(50.0, 110.0),
                    r.between(35.0, 80.0)
                )
                .unwrap();
            }
            for k in 0..26 {
                let (x, y, len, lean) = (r.between(0.0, w), r.between(0.0, h), r.between(60.0, 120.0), r.between(-30.0, 30.0));
                let tone = if k % 3 == 0 { &alt } else { &line };
                write!(out, "<path d=\"M{x:.0},{y:.0}l{lean:.0},-{len:.0}\" style=\"stroke:{tone};stroke-width:14;fill:none;stroke-linecap:round\"/>").unwrap();
            }
        }
        // A planted bed: mulch and soil speckle, with a few clumps of plants.
        "planted" => {
            for _ in 0..40 {
                write!(
                    out,
                    "<circle cx=\"{:.0}\" cy=\"{:.0}\" r=\"{:.0}\" style=\"fill:{alt};fill-opacity:0.8\"/>",
                    r.between(0.0, w),
                    r.between(0.0, h),
                    r.between(14.0, 26.0)
                )
                .unwrap();
            }
            for _ in 0..3 {
                let (cx, cy) = (r.between(60.0, w - 60.0), r.between(60.0, h - 60.0));
                for _ in 0..4 {
                    write!(
                        out,
                        "<circle cx=\"{:.0}\" cy=\"{:.0}\" r=\"{:.0}\" style=\"fill:{line};fill-opacity:0.85\"/>",
                        cx + r.between(-45.0, 45.0),
                        cy + r.between(-45.0, 45.0),
                        r.between(28.0, 48.0)
                    )
                    .unwrap();
                }
            }
        }
        // Gravel: dense small stones of varied tone.
        "gravel" => {
            for k in 0..70 {
                let tone = if k % 3 == 1 { &alt } else { &line };
                write!(
                    out,
                    "<ellipse cx=\"{:.0}\" cy=\"{:.0}\" rx=\"{:.0}\" ry=\"{:.0}\" style=\"fill:{tone};fill-opacity:{:.2}\"/>",
                    r.between(0.0, w),
                    r.between(0.0, h),
                    r.between(18.0, 34.0),
                    r.between(14.0, 26.0),
                    r.between(0.45, 0.9)
                )
                .unwrap();
            }
        }
        // A tree pit: a grate of bars over mulch.
        "trees" => {
            for _ in 0..24 {
                write!(
                    out,
                    "<circle cx=\"{:.0}\" cy=\"{:.0}\" r=\"{:.0}\" style=\"fill:{line};fill-opacity:0.5\"/>",
                    r.between(0.0, w),
                    r.between(0.0, h),
                    r.between(14.0, 24.0)
                )
                .unwrap();
            }
            write!(out, "<path d=\"M0,0H{w}M0,200H{w}M0,0V{h}M200,0V{h}\" style=\"stroke:{alt};stroke-width:30;fill:none\"/>").unwrap();
        }
        // Asphalt: a speckle of stones, light and dark.
        _ => {
            for k in 0..48 {
                let tone = if k % 2 == 0 { &line } else { &alt };
                write!(
                    out,
                    "<circle cx=\"{:.0}\" cy=\"{:.0}\" r=\"{:.0}\" style=\"fill:{tone};fill-opacity:0.7\"/>",
                    r.between(0.0, w),
                    r.between(0.0, h),
                    r.between(16.0, 30.0)
                )
                .unwrap();
            }
        }
    }
    out
}

/// The materials' textures for a drawing's `<defs>`, each with the id `{prefix}-{material}`, laid on the ground by
/// `ground` (pixels to the millimetre, as `Ground`).
pub fn patterns(prefix: &str, ground: Ground) -> String {
    let [a, b, c, d, e, f] = ground;
    let matrix = format!("matrix({a:.6} {b:.6} {c:.6} {d:.6} {e:.2} {f:.2})");
    MATERIALS
        .iter()
        .map(|m| {
            let (w, h) = tile_mm(m.id);
            format!(
                "<pattern id=\"{prefix}-{}\" width=\"{w}\" height=\"{h}\" patternUnits=\"userSpaceOnUse\" patternTransform=\"{matrix}\">{}</pattern>",
                m.id,
                marks(m.id)
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_material_has_its_own_texture_in_millimetres_under_the_prefix_with_the_ground_transform() {
        let out = patterns("g", [0.03, 0.0, 0.012, -0.009, 10.0, 300.0]);
        assert_eq!(out.matches("<pattern ").count(), MATERIALS.len());
        let mut seen = Vec::new();
        for m in MATERIALS {
            let at = out.find(&format!("<pattern id=\"g-{}\"", m.id)).unwrap_or_else(|| panic!("{}", m.id));
            let tag = &out[at..out[at..].find("</pattern>").unwrap() + at];
            let (w, h) = tile_mm(m.id);
            assert!(tag.contains(&format!("width=\"{w}\" height=\"{h}\" patternUnits=\"userSpaceOnUse\"")), "{}", m.id);
            assert!(tag.contains("patternTransform=\"matrix(0.030000 0.000000 0.012000 -0.009000 10.00 300.00)\""), "{}", m.id);
            assert!(tag.contains(&format!("var(--mat-{}-line)", m.id)), "{} is drawn in its own colours", m.id);
            assert!(!tag.contains("var(--ink") && !tag.contains("var(--panel"), "{}: no theme colour", m.id);
            seen.push(tag[tag.find('>').unwrap()..].to_string());
        }
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), MATERIALS.len(), "every texture is its own");
    }

    #[test]
    fn a_brick_is_200_by_100_mm_and_a_concrete_panel_1_8_m() {
        let brick = marks("brick");
        assert!(brick.contains("width=\"200\" height=\"100\""));
        assert!(brick.contains("M0,0H800M0,100H800") || brick.contains("M0,0H800"));
        assert_eq!(tile_mm("brick"), (800.0, 400.0));
        assert_eq!(tile_mm("concrete").0, 1800.0);
    }

    #[test]
    fn the_texture_is_the_same_each_time() {
        assert_eq!(marks("gravel"), marks("gravel"));
    }
}
