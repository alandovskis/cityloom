//! The things that stand on the pieces, drawn as plain boxes: a tree, a bus shelter, a post on a curb.

use std::fmt::Write;

use crate::shared::catalogue::KINDS;
use crate::street::model::View;
use crate::street::svg::{curb_edges, curb_width_mm};

use super::projection::{MIN_SOLID_PX, Projection, STRETCH_MM, drawn_mm, points};
use super::raised::{Face, curb_top_mm, height_mm, shade};

/// Something standing on a piece, drawn as a plain box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Solid {
    Trunk,
    Crown,
    Shelter,
    Post,
}

impl Solid {
    fn name(self) -> &'static str {
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

/// What stands on the piece at `i`: a tree every 8 m on street trees, a shelter on a bus shelter's
/// piece, a post every 4 m along a bike lane's curb (not an island's). Each keeps its own size and
/// stands on the top as drawn, `RISE` times as high as the piece stands. Nothing on a piece too
/// narrow on screen to show it.
pub(super) fn blocks(v: &View, i: usize, p: &Projection) -> Vec<Block> {
    let s = &v.segments[i];
    let (x0, w) = (s.x_mm as f64, s.width_mm as f64);
    if w * p.scale < MIN_SOLID_PX {
        return Vec::new();
    }
    let id = KINDS[s.kind].id;
    let (cx, h) = (x0 + w / 2.0, drawn_mm(height_mm(id)));
    let along = |first: f64, every: f64| (0..).map(move |n| first + n as f64 * every).take_while(|z| *z < STRETCH_MM);
    let mut out = Vec::new();
    if s.material == "trees" {
        let crown = (w * 0.9).min(2_400.0);
        for z in along(4_000.0, 8_000.0) {
            out.push(Block { solid: Solid::Trunk, x: (cx - 125.0, cx + 125.0), y: (h, h + 2_000.0), z: (z - 125.0, z + 125.0) });
            out.push(Block { solid: Solid::Crown, x: (cx - crown / 2.0, cx + crown / 2.0), y: (h + 2_000.0, h + 5_000.0), z: (z - 1_200.0, z + 1_200.0) });
        }
    }
    if id == "busshelter" {
        let d = (w * 0.8).min(1_500.0);
        out.push(Block { solid: Solid::Shelter, x: (cx - d / 2.0, cx + d / 2.0), y: (h, h + 2_500.0), z: (18_000.0, 22_000.0) });
    }
    if let Some(c) = s.curb.filter(|c| id == "bike" && *c != "island") {
        let (top, cw) = (curb_top_mm(c, id), curb_width_mm(c).min(w / 2.0));
        for right in curb_edges(v, i) {
            let px = if right { x0 + w - cw / 2.0 } else { x0 + cw / 2.0 };
            for z in along(2_000.0, 4_000.0) {
                out.push(Block { solid: Solid::Post, x: (px - 60.0, px + 60.0), y: (top, top + 900.0), z: (z - 60.0, z + 60.0) });
            }
        }
    }
    out
}

/// A box's three faces the eye sees: its front, its right side and its top.
pub(super) fn block(out: &mut String, p: &Projection, b: &Block) {
    let ((x0, x1), (y0, y1), (z0, z1)) = (b.x, b.y, b.z);
    write!(
        out,
        "<g class=\"iso-solid s-{}\"><polygon class=\"o-front\" points=\"{}\"{}/><polygon class=\"o-side\" points=\"{}\"{}/><polygon class=\"o-top\" points=\"{}\"/></g>",
        b.solid.name(),
        points(p, &[(x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0)]),
        shade(Face::Front),
        points(p, &[(x1, y0, z0), (x1, y0, z1), (x1, y1, z1), (x1, y1, z0)]),
        shade(Face::Side),
        points(p, &[(x0, y1, z0), (x1, y1, z0), (x1, y1, z1), (x0, y1, z1)])
    )
    .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::kind_index;
    use crate::street::iso::iso_svg;
    use crate::street::iso::testing::{count, drawn, en, furnished};
    use crate::street::model::Editor;

    #[test]
    fn a_planting_strip_of_trees_has_a_tree_every_8_m_each_a_trunk_and_a_crown() {
        // Sample Avenue 2 has two 1.5 m planting strips of street trees: five trees each along 40 m.
        let m = drawn(&Editor::new(1));
        assert_eq!(count(&m, "class=\"iso-solid s-crown\""), 10);
        assert_eq!(count(&m, "class=\"iso-solid s-trunk\""), 10);
        // A tree is a trunk with its crown over it: a crown follows its trunk.
        assert!(m.find("s-trunk").unwrap() < m.find("s-crown").unwrap());
        // Every solid comes after every strip.
        assert!(m.rfind("class=\"iso-piece\"").unwrap() < m.find("class=\"iso-solid").unwrap());
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
    fn what_stands_on_a_piece_is_drawn_after_what_stands_on_the_pieces_to_its_left() {
        // The eye sees the right sides, so a solid on a piece further right is nearer and covers the ones to its left
        // even when it starts farther along: the shelter (18 m to 22 m) comes before the crown of the tree at 20 m
        // on the planting strip to its right.
        let mut e = Editor::new(1);
        e.add(kind_index("busshelter").unwrap(), 1);
        let m = drawn(&e);
        assert!(m.find("s-shelter").unwrap() < m.find("s-crown").unwrap());
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
    fn a_piece_too_narrow_on_screen_draws_without_its_solids_but_keeps_its_faces() {
        // At 600 px Sample Avenue 2's 1.5 m planting strips are under 20 px (17.6): no trees, all eleven tops.
        let narrow = iso_svg(&Editor::new(1).view(), &en(), 600.0).markup;
        assert_eq!(count(&narrow, "class=\"iso-solid"), 0);
        assert_eq!(count(&narrow, "class=\"iso-top "), 11);
        // The freeway at 600 px still draws every piece, with no missing number.
        let freeway = iso_svg(&Editor::new(3).view(), &en(), 600.0);
        assert_eq!(count(&freeway.markup, "class=\"iso-top "), 9);
        assert!(!freeway.markup.contains("NaN"));
    }

    #[test]
    fn at_760_px_the_planting_strips_keep_their_trees_the_shelter_and_the_bike_lane_s_posts() {
        // A 1.5 m strip is about 21 px there: narrow, but a tree is a box standing up and reads.
        let avenue = iso_svg(&Editor::new(1).view(), &en(), 760.0).markup;
        assert_eq!(count(&avenue, "class=\"iso-solid s-crown\""), 10);
        let m = iso_svg(&furnished().view(), &en(), 760.0).markup;
        assert_eq!(count(&m, "class=\"iso-solid s-crown\""), 10);
        assert_eq!(count(&m, "class=\"iso-solid s-shelter\""), 1);
        assert_eq!(count(&m, "class=\"iso-solid s-post\""), 20);
    }
}
