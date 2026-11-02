//! The things that stand on the pieces, drawn as plain boxes: a tree, a bus shelter, a post on a curb.

use crate::shared::catalogue::KINDS;
use crate::street::model::SegView;
use crate::street::pieces::{curb_edges, curb_width_mm};

use super::order::Bx;

/// Half the depth, along the street, of a tree's crown, in millimetres.
pub const CROWN_HALF_DEPTH_MM: f64 = 1_200.0;
use super::projection::{EYE_BACK_MM, MIN_STANDING_PX, Projection, drawn_mm};
use super::raised::{curb_top_mm, height_mm};

/// Something standing on a piece, drawn as a plain box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Solid {
    Trunk,
    Crown,
    Shelter,
    Post,
}

impl Solid {
    pub(super) fn name(self) -> &'static str {
        match self {
            Solid::Trunk => "trunk",
            Solid::Crown => "crown",
            Solid::Shelter => "shelter",
            Solid::Post => "post",
        }
    }
}

/// A box standing in the scene, in millimetres of the drawing: across, up, along.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Block {
    pub(super) solid: Solid,
    pub(super) x: (f64, f64),
    pub(super) y: (f64, f64),
    pub(super) z: (f64, f64),
}

impl Block {
    /// The box, for ordering.
    pub(super) fn bounds(&self) -> Bx {
        Bx { x: self.x, y: self.y, z: self.z }
    }

    /// How far the box is from the eye on the ground, in millimetres, from its nearest point: the order the boxes are
    /// offered in, far to near, where none is in front of another (`order::paint_order`).
    pub(super) fn from_eye(&self, p: &Projection) -> f64 {
        let across = (self.x.0 - p.centre_mm).max(p.centre_mm - self.x.1).max(0.0);
        across.hypot(self.z.0 + EYE_BACK_MM)
    }
}

/// Whether a piece `w` mm wide is wide enough on the screen, where it is `along` mm from the near end, to carry
/// something standing on it: a piece shrinks with distance, so each thing is judged where it stands.
pub(super) fn wide_enough(p: &Projection, w: f64, along: f64) -> bool {
    w * p.scale_at(along) >= MIN_STANDING_PX
}

/// How long a shelter box is along the street, in millimetres. Unverified (TAC / STM to check).
const SHELTER_LONG_MM: f64 = 4_000.0;

/// A bus shelter on a piece spanning `x` across, standing on `base_mm`, at the middle of `along`: a box 4 m long, as
/// deep as the piece allows (up to 1.5 m). Whole or not at all: it stands only where the piece is wide enough on the
/// screen to show it at that distance (`wide_enough`) and the box lies wholly in the reach.
pub(super) fn shelter(p: &Projection, x: (f64, f64), along: (f64, f64), base_mm: f64) -> Vec<Block> {
    let w = x.1 - x.0;
    let (cx, middle) = (x.0 + w / 2.0, along.0 + (along.1 - along.0) / 2.0);
    let d = (w * 0.8).min(1_500.0);
    let z = (middle - SHELTER_LONG_MM / 2.0, middle + SHELTER_LONG_MM / 2.0);
    let fits = wide_enough(p, w, middle) && z.0 >= p.reach.near && z.1 <= p.reach.far;
    fits.then(|| Block { solid: Solid::Shelter, x: (cx - d / 2.0, cx + d / 2.0), y: (base_mm, base_mm + 2_500.0), z }).into_iter().collect()
}

/// What stands on the piece at `i`: a tree every 8 m on street trees, a shelter on a bus shelter's
/// piece, a post every 4 m along a bike lane's curb (not an island's). Each keeps its own size and
/// stands on the top as drawn, `RISE` times as high as the piece stands. A thing stands only where its piece is wide
/// enough on the screen, at its own distance, to show it (`wide_enough`).
pub(super) fn blocks(segs: &[SegView], i: usize, p: &Projection) -> Vec<Block> {
    let s = &segs[i];
    let (x0, w) = (s.x_mm as f64, s.width_mm as f64);
    let id = KINDS[s.kind].id;
    let (cx, h) = (x0 + w / 2.0, drawn_mm(height_mm(id)));
    let (near, far) = (p.reach.near, p.reach.far);
    let along = |first: f64, every: f64| (0..).map(move |n| near + first + n as f64 * every).take_while(|z| *z < far);
    let mut out = Vec::new();
    if s.material == "trees" {
        let crown = (w * 0.9).min(2_400.0);
        for z in along(4_000.0, 8_000.0).filter(|&z| wide_enough(p, w, z)) {
            out.push(Block { solid: Solid::Trunk, x: (cx - 125.0, cx + 125.0), y: (h, h + 2_000.0), z: (z - 125.0, z + 125.0) });
            out.push(Block {
                solid: Solid::Crown,
                x: (cx - crown / 2.0, cx + crown / 2.0),
                y: (h + 2_000.0, h + 5_000.0),
                z: (z - CROWN_HALF_DEPTH_MM, z + CROWN_HALF_DEPTH_MM),
            });
        }
    }
    if id == "busshelter" {
        out.extend(shelter(p, (x0, x0 + w), (near, near + p.reach.len()), h));
    }
    if let Some(c) = s.curb.filter(|c| id == "bike" && *c != "island") {
        let (top, cw) = (curb_top_mm(c, id), curb_width_mm(c).min(w / 2.0));
        for right in curb_edges(segs, i) {
            let px = if right { x0 + w - cw / 2.0 } else { x0 + cw / 2.0 };
            for z in along(2_000.0, 4_000.0).filter(|&z| wide_enough(p, w, z)) {
                out.push(Block { solid: Solid::Post, x: (px - 60.0, px + 60.0), y: (top, top + 900.0), z: (z - 60.0, z + 60.0) });
            }
        }
    }
    // A box stands only where it lies wholly in the reach.
    out.retain(|b| b.z.0 >= near && b.z.1 <= far);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::kind_index;
    use crate::shared::diagram::render::layer_markup;
    use crate::shared::diagram::{Layer, LayerKind};
    use crate::street::iso::iso_svg;
    use crate::street::iso::projection::points;
    use crate::street::iso::testing::{count, drawn, en, furnished};
    use crate::street::model::Editor;

    /// A box as markup, in a layer of its own.
    fn block(out: &mut String, p: &Projection, b: &Block) {
        let layer = Layer { kind: LayerKind::Objects, shapes: vec![crate::street::iso::scene::block(p, b)] };
        out.push_str(&layer_markup(&layer, p));
    }

    #[test]
    fn trees_posts_and_the_shelter_stay_wholly_inside_the_reach() {
        use crate::street::iso::projection::Reach;
        let mut e = furnished();
        e.add(kind_index("bike").unwrap(), 2);
        let v = e.view();
        let base = Projection::fit(1050.0, v.row_mm.max(v.total_mm));
        for (near, far) in [(0.0, 20_000.0), (20_000.0, 40_000.0), (0.0, 40_000.0)] {
            let p = base.over(Reach { near, far });
            let all: Vec<Block> = (0..v.segments.len()).flat_map(|i| blocks(&v.segments, i, &p)).collect();
            assert!(all.iter().any(|b| b.solid == Solid::Crown) && all.iter().any(|b| b.solid == Solid::Post) && all.iter().any(|b| b.solid == Solid::Shelter));
            assert!(all.iter().all(|b| b.z.0 >= near && b.z.1 <= far), "{near}..{far}");
        }
        let p = base.over(Reach { near: 0.0, far: 20_000.0 });
        let shelter = (0..v.segments.len()).flat_map(|i| blocks(&v.segments, i, &p)).find(|b| b.solid == Solid::Shelter).unwrap();
        assert_eq!(shelter.z, (8_000.0, 12_000.0));
        // Trees count from the near end of the reach: the first is 4 m in.
        let p = base.over(Reach { near: 20_000.0, far: 40_000.0 });
        let first = (0..v.segments.len()).flat_map(|i| blocks(&v.segments, i, &p)).filter(|b| b.solid == Solid::Trunk).map(|b| b.z.0).fold(f64::MAX, f64::min);
        assert_eq!(first, 24_000.0 - 125.0);
    }

    #[test]
    fn a_planting_strip_of_trees_has_a_tree_every_8_m_each_a_trunk_and_a_crown() {
        // Sample Avenue 2 has two 1.5 m planting strips of street trees: five trees each along 40 m.
        let m = drawn(&Editor::new(1));
        assert_eq!(count(&m, "class=\"iso-solid s-crown\""), 10);
        assert_eq!(count(&m, "class=\"iso-solid s-trunk\""), 10);
        // A tree is a trunk with its crown over it: a crown follows its trunk.
        assert!(m.find("s-trunk").unwrap() < m.find("s-crown").unwrap());
    }

    #[test]
    fn a_tree_stands_on_its_strip_s_drawn_top_at_its_own_true_height() {
        // The nearest tree of Sample Avenue 2's first (left) planting strip, 4 m along: a 250 mm trunk 2 m tall whose
        // foot is on the strip's top, drawn RISE times as high as the strip stands, and a crown 3 m tall over it. The
        // faces are matched whole, across as well as up, so a tree on another piece would not pass.
        let v = Editor::new(1).view();
        let strip = v.segments.iter().find(|s| s.material == "trees").unwrap();
        let p = Projection::fit(1050.0, v.row_mm.max(v.total_mm));
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let (cx, top) = (strip.x_mm as f64 + strip.width_mm as f64 / 2.0, drawn_mm(150));
        let trunk_front =
            points(&p, &[(cx - 125.0, top, 3_875.0), (cx + 125.0, top, 3_875.0), (cx + 125.0, top + 2_000.0, 3_875.0), (cx - 125.0, top + 2_000.0, 3_875.0)]);
        assert!(
            m.contains(&format!("<g class=\"iso-solid s-trunk\"><polygon class=\"o-front\" points=\"{trunk_front}\" filter=\"url(#iso-shade-front)\"/>")),
            "the drawing has not got it"
        );
        let half = strip.width_mm as f64 * 0.9 / 2.0;
        let crown_top = points(
            &p,
            &[
                (cx - half, top + 5_000.0, 2_800.0),
                (cx + half, top + 5_000.0, 2_800.0),
                (cx + half, top + 5_000.0, 5_200.0),
                (cx - half, top + 5_000.0, 5_200.0),
            ],
        );
        assert!(m.contains(&format!("<polygon class=\"o-top\" points=\"{crown_top}\"/>")), "the drawing has not got it");
    }

    #[test]
    fn what_stands_on_the_pieces_is_drawn_from_the_far_end_to_the_near_so_that_a_nearer_tree_covers_a_farther() {
        // Sample Avenue 2's trees: each crown's front face is lower on the screen than the one drawn before it.
        let m = drawn(&Editor::new(1));
        let bottoms: Vec<f64> = m
            .split("<g class=\"iso-solid s-crown\"><polygon class=\"o-front\" points=\"")
            .skip(1)
            .map(|c| {
                let pts = &c[..c.find('"').unwrap()];
                pts.split(' ').map(|pt| pt.split(',').nth(1).unwrap().parse::<f64>().unwrap()).fold(f64::MIN, f64::max)
            })
            .collect();
        assert_eq!(bottoms.len(), 10);
        // The two strips' trees at one depth are drawn together, and each pair is lower on the screen than the one before.
        assert!(bottoms.windows(2).all(|w| w[1] >= w[0]) && bottoms.first() < bottoms.last(), "{bottoms:?}");
    }

    #[test]
    fn a_nearer_box_is_later_and_a_tree_s_crown_comes_after_its_trunk() {
        let p = Projection::fit(1050.0, 30_000);
        let b = |z: f64, y: f64| Block { solid: Solid::Crown, x: (2_000.0, 3_000.0), y: (y, y + 1.0), z: (z, z + 1_000.0) };
        assert!(b(30_000.0, 0.0).from_eye(&p) > b(10_000.0, 0.0).from_eye(&p));
        // A box over the eye is as near across as it can be; one beside it is farther at the same depth.
        let over = Block { x: (14_000.0, 16_000.0), ..b(10_000.0, 0.0) };
        assert!(b(10_000.0, 0.0).from_eye(&p) > over.from_eye(&p));
        assert_eq!(over.from_eye(&p), 10_000.0 + EYE_BACK_MM);
    }

    /// The index in the markup of the first solid of `class` whose front face is `front`.
    fn at_front(m: &str, class: &str, front: &str) -> usize {
        m.find(&format!("<g class=\"iso-solid s-{class}\"><polygon class=\"o-front\" points=\"{front}\""))
            .unwrap_or_else(|| panic!("no {class} has that front"))
    }

    /// The front face of a block as `block` writes it.
    fn front_of(p: &Projection, b: &Block) -> String {
        let ((x0, x1), (y0, y1), z0) = (b.x, b.y, b.z.0);
        points(p, &[(x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0)])
    }

    #[test]
    fn a_crown_in_front_of_a_bus_shelter_is_drawn_after_it_and_covers_it() {
        // Sample Avenue 2 with a bus shelter (18 to 22 m along) beside its left planting strip, whose tree at 20 m has a
        // crown from 18.8 to 21.2 m: the crown is nearer the middle of the street than the shelter and overlaps it
        // in depth, so it stands between the eye and the shelter's top and inner face, though the shelter's nearest
        // point is nearer the eye on the ground.
        let v = furnished().view();
        let p = Projection::fit(1050.0, v.row_mm.max(v.total_mm));
        let all: Vec<Block> = (0..v.segments.len()).flat_map(|i| blocks(&v.segments, i, &p)).collect();
        let shelter = all.iter().find(|b| b.solid == Solid::Shelter).unwrap();
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let covers: Vec<&Block> = all
            .iter()
            .filter(|b| {
                b.solid == Solid::Crown && b.z.0 < shelter.z.1 && shelter.z.0 < b.z.1 && b.x.0 >= shelter.x.1 && b.x.1 <= p.centre_mm && b.y.1 > shelter.y.0
            })
            .collect();
        assert!(!covers.is_empty(), "the case is not in the street");
        for c in covers {
            assert!(
                at_front(&m, "shelter", &front_of(&p, shelter)) < at_front(&m, "crown", &front_of(&p, c)),
                "the crown {c:?} stands between the eye and the shelter"
            );
        }
    }

    #[test]
    fn objects_are_a_layer_over_every_raised_strip_so_a_post_beyond_a_nearer_taller_strip_shows_over_it() {
        // A bike lane (curbed, posts on its curbs) on the left and a median farther in, nearer the middle of the street,
        // 450 mm as drawn. With objects and strips in layers of their own the posts are painted over the median that
        // stands in front of them (the one deliberate difference from painting both in one depth order).
        let mut e = Editor::new(0);
        let bike = e.add(kind_index("bike").unwrap(), 1);
        let median = e.add(kind_index("median").unwrap(), 3);
        let v = e.view();
        let (bike_x, median_x) = {
            let at = |uid| v.segments.iter().find(|s| s.uid == uid).unwrap().x_mm;
            (at(bike), at(median))
        };
        assert!(bike_x < median_x && (median_x as f64) < Projection::fit(1050.0, v.row_mm.max(v.total_mm)).centre_mm, "the median is left of the eye");
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let median_at = m.rfind(&format!("<g class=\"iso-piece\" data-uid=\"{median}\">")).unwrap();
        let first_post = m.find("class=\"iso-solid s-post\"").unwrap();
        assert!(first_post > median_at, "every post is painted after every raised strip");
        // And a tree stands over its own strip, on top of it.
        let m = drawn(&Editor::new(1));
        assert!(m.find("<g class=\"iso-piece\"").unwrap() < m.find("s-trunk").unwrap());
    }

    #[test]
    fn a_box_shows_the_side_of_it_that_faces_the_middle_of_the_street() {
        let p = Projection::fit(1050.0, 30_000);
        let sides = |x: (f64, f64)| {
            let mut out = String::new();
            block(&mut out, &p, &Block { solid: Solid::Trunk, x, y: (0.0, 2_000.0), z: (4_000.0, 4_250.0) });
            (out.contains("o-front"), out.contains("o-side"), out.contains("o-top"), out)
        };
        let (front, side, top, left) = sides((3_000.0, 3_250.0));
        assert!(front && side && top);
        // Left of the eye: the side is its right face, at x1.
        let want = points(&p, &[(3_250.0, 0.0, 4_000.0), (3_250.0, 0.0, 4_250.0), (3_250.0, 2_000.0, 4_250.0), (3_250.0, 2_000.0, 4_000.0)]);
        assert!(left.contains(&format!("<polygon class=\"o-side\" points=\"{want}\"")), "{left}");
        let (_, side, _, right) = sides((26_000.0, 26_250.0));
        let want = points(&p, &[(26_000.0, 0.0, 4_000.0), (26_000.0, 0.0, 4_250.0), (26_000.0, 2_000.0, 4_250.0), (26_000.0, 2_000.0, 4_000.0)]);
        assert!(side && right.contains(&format!("<polygon class=\"o-side\" points=\"{want}\"")), "{right}");
        // Over the eye: none.
        assert!(!sides((14_900.0, 15_100.0)).1);
    }

    #[test]
    fn a_bus_shelter_stands_on_its_piece_and_a_bike_lane_s_curb_carries_posts() {
        let mut e = Editor::new(0);
        e.add(kind_index("busshelter").unwrap(), 1);
        assert_eq!(count(&drawn(&e), "class=\"iso-solid s-shelter\""), 1);
        // A bike lane with a curb on both edges: a post every 4 m on each, ten each along 40 m.
        let mut e = Editor::new(0);
        let u = e.add(kind_index("bike").unwrap(), 2);
        assert_eq!(count(&drawn(&e), "class=\"iso-solid s-post\""), 20);
        // A bus boarding island is a platform: no posts.
        assert!(e.set_curb(u, Some(6)));
        assert_eq!(count(&drawn(&e), "class=\"iso-solid s-post\""), 0);
    }

    #[test]
    fn each_thing_is_judged_where_it_stands_and_drawn_while_it_is_a_few_pixels_wide() {
        // Sample Avenue 2's 1.5 m planting strips at 600 px are about 24 px at the near end and 7 px at the far: every tree
        // stands, to the end of the stretch.
        let v = Editor::new(1).view();
        let strip = v.segments.iter().position(|s| s.material == "trees").unwrap();
        let zs = |width: f64, span: i32| -> Vec<f64> {
            blocks(&v.segments, strip, &Projection::fit(width, span)).iter().filter(|b| b.solid == Solid::Trunk).map(|b| (b.z.0 + b.z.1) / 2.0).collect()
        };
        assert_eq!(zs(600.0, v.row_mm.max(v.total_mm)), vec![4_000.0, 12_000.0, 20_000.0, 28_000.0, 36_000.0]);
        // On a wider scene the strip is narrower: it stops drawing where it is under MIN_STANDING_PX wide, from the far end.
        // At 600 px and 120 m across the 1.5 m strip is 6.75 px at the near end, 5.2 px at 4 m, 3.6 px at 12 m and 2.8 at 20 m.
        assert_eq!(zs(600.0, 120_000), vec![4_000.0, 12_000.0]);
        // A piece too narrow even at the near end draws without any, and the street is still drawn.
        assert!(zs(600.0, 300_000).is_empty());
        assert_eq!(count(&iso_svg(&v, &en(), 600.0).markup, "class=\"iso-top "), 11);
        // The freeway at 600 px still draws every piece, with no missing number.
        let freeway = iso_svg(&Editor::new(3).view(), &en(), 600.0);
        assert_eq!(count(&freeway.markup, "class=\"iso-top "), 9);
        assert!(!freeway.markup.contains("NaN"));
    }

    #[test]
    fn at_600_and_760_px_the_planting_strips_keep_every_tree_the_shelter_and_the_bike_lane_s_posts() {
        for width in [600.0, 760.0] {
            let avenue = iso_svg(&Editor::new(1).view(), &en(), width).markup;
            assert_eq!(count(&avenue, "class=\"iso-solid s-crown\""), 10, "{width}");
            assert_eq!(count(&avenue, "class=\"iso-solid s-trunk\""), 10, "{width}");
            let m = iso_svg(&furnished().view(), &en(), width).markup;
            assert_eq!(count(&m, "class=\"iso-solid s-shelter\""), 1, "{width}");
            assert_eq!(count(&m, "class=\"iso-solid s-crown\""), 10, "{width}");
            // A post every 4 m along 40 m on each of the bike lane's two curbed edges.
            assert_eq!(count(&m, "class=\"iso-solid s-post\""), 20, "{width}");
        }
    }
}
