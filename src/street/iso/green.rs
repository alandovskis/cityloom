//! The green elements in the 2.5D: on the selected zone's section, each as the patch of ground it is (`Patch`, in the
//! raised layer so that it lies over the piece it stands on) and, for a tree, the trunk and crown that stand on its pit.
//! A tree's pit is a patch of the tree-pit material flush with its piece's top; a bed is a slab of the planted material
//! standing its planting height above it; a rain garden and a bioswale are a flush patch of the rain-garden material
//! with an outline. An element on a roadway piece is not shown at all (`Editor::green_shown`).

use std::fmt::Write;

use crate::shared::catalogue::KINDS;
use crate::shared::i18n::{Args, I18n};
use crate::shared::iso::format::esc;
use crate::street::model::{GreenKind, SegView, ShownGreen, TreeSize};

use super::order::Bx;
use super::projection::{Projection, Reach, drawn_mm, points};
use super::raised::{Face, face, height_mm, inner_face, top_over};
use super::solids::{Block, Solid, wide_enough};

/// How close to an explicit tree, along the street, an automatic tree of a strip of street trees is left out, in
/// millimetres: the explicit tree stands in its place.
///
/// UNVERIFIED (placeholder): 4 000 mm.
pub(super) const AUTOMATIC_CLEAR_MM: f64 = 4_000.0;

/// How much of the tree's canopy the drawn crown is: the canopy is how far the tree spreads when grown, which
/// the drawing, a stretch of a few tens of metres, shows smaller so that a street's trees do not hide it.
///
/// UNVERIFIED (placeholder): 30%.
const CROWN_OF_CANOPY: f64 = 0.3;

/// How tall the trunk and the crown are drawn, in millimetres, as the street trees' are (`solids::blocks`).
const TRUNK_MM: f64 = 2_000.0;
const CROWN_MM: f64 = 3_000.0;

/// The kind a green element is drawn as: its tint and hatch, which its faces keep.
const PLANTING: &str = "planting";

/// A patch of ground a green element covers.
#[derive(Clone, Copy, Debug)]
pub(super) struct Patch {
    pub kind: GreenKind,
    pub along: (f64, f64),
    /// Across the street, the part of the element's span that lies on its piece.
    pub across: (f64, f64),
    /// How high the piece's top is drawn, where the patch lies.
    pub base: f64,
    /// How far the patch stands above it, drawn true: a bed's planting height, nothing for the rest.
    pub rise: f64,
}

impl Patch {
    /// The box it fills, for ordering.
    pub(super) fn bounds(&self) -> Bx {
        Bx { x: self.across, y: (self.base, self.base + self.rise), z: self.along }
    }
}

/// The part of `g`'s span across that lies on its piece, in the coordinates of the section.
fn on_piece(segs: &[SegView], g: &ShownGreen) -> Option<(f64, f64)> {
    let s = &segs[g.piece];
    let (x0, x1) = (g.across.0.max(f64::from(s.x_mm)), g.across.1.min(f64::from(s.x_mm + s.width_mm)));
    (x1 > x0).then_some((x0, x1))
}

/// The patches of the shown elements, in the order they are kept.
pub(super) fn patches(segs: &[SegView], shown: &[ShownGreen]) -> Vec<Patch> {
    shown
        .iter()
        .filter_map(|g| {
            let across = on_piece(segs, g)?;
            let base = drawn_mm(height_mm(KINDS[segs[g.piece].kind].id));
            let rise = if g.kind == GreenKind::Bed { f64::from(height_mm(PLANTING)) } else { 0.0 };
            Some(Patch { kind: g.kind, along: g.along, across, base, rise })
        })
        .collect()
}

/// The patch as its own group, hovered by the element's name: its pit, its slab (top, front, and the side the eye
/// sees) or its flush top and outline.
pub(super) fn draw(out: &mut String, p: &Projection, i18n: &I18n, patch: &Patch) {
    let name = i18n.tr(patch.kind.name_key(), &Args::new());
    write!(out, "<g class=\"iso-green\"><title>{}</title>", esc(&name)).unwrap();
    let (x0, x1) = patch.across;
    let (n, l) = patch.along;
    let sub = p.over(Reach { near: n, far: l });
    let (base, top) = (patch.base, patch.base + patch.rise);
    match patch.kind {
        GreenKind::Tree => top_over(out, &sub, PLANTING, "trees", base, &|_| (x0, x1), true),
        GreenKind::Bed => {
            let hatch = format!("h-{PLANTING}");
            face(out, Face::Front, Some(PLANTING), &hatch, &points(&sub, &[(x0, base, n), (x1, base, n), (x1, top, n), (x0, top, n)]));
            if let Some(right) = inner_face(&sub, x0, x1) {
                let x = if right { x1 } else { x0 };
                face(out, Face::Side, Some(PLANTING), &hatch, &points(&sub, &[(x, base, n), (x, base, l), (x, top, l), (x, top, n)]));
            }
            top_over(out, &sub, PLANTING, "planted", top, &|_| (x0, x1), true);
        }
        GreenKind::RainGarden | GreenKind::Bioswale => {
            top_over(out, &sub, PLANTING, "rain_garden", base, &|_| (x0, x1), true);
            let rim = points(&sub, &[(x0, base, n), (x1, base, n), (x1, base, l), (x0, base, l)]);
            write!(out, "<polygon class=\"iso-green-edge\" points=\"{rim}\"/>").unwrap();
        }
    }
    out.push_str("</g>");
}

/// Where the explicit trees on piece `piece` stand along the stretch, at the middle of their pits: the automatic
/// trees of a strip of street trees keep clear of them (`AUTOMATIC_CLEAR_MM`). Only a tree that is drawn (`blocks`: its
/// crown wholly in the reach and wide enough to show) takes the place of an automatic one.
pub(super) fn tree_stands(segs: &[SegView], shown: &[ShownGreen], piece: usize, p: &Projection) -> Vec<f64> {
    shown
        .iter()
        .filter(|g| g.kind == GreenKind::Tree && g.piece == piece && !blocks(segs, std::slice::from_ref(g), p).is_empty())
        .map(|g| (g.full.0 + g.full.1) / 2.0)
        .collect()
}

/// The trunk and crown of each shown tree, standing on its pit at the middle of it. The crown is `CROWN_OF_CANOPY` of
/// the tree's canopy across and along, and no wider than the piece and its neighbours, where it may spread. A tree
/// stands only where that width is wide enough on the screen to show it, at its own distance, and wholly in the reach.
pub(super) fn blocks(segs: &[SegView], shown: &[ShownGreen], p: &Projection) -> Vec<Block> {
    let mut out = Vec::new();
    for g in shown.iter().filter(|g| g.kind == GreenKind::Tree) {
        let Some((x0, x1)) = on_piece(segs, g) else { continue };
        let (lo, hi) = (g.piece.saturating_sub(1), (g.piece + 1).min(segs.len() - 1));
        let room: f64 = segs[lo..=hi].iter().map(|s| f64::from(s.width_mm)).sum();
        let canopy = f64::from(g.size.unwrap_or(TreeSize::M).canopy_mm());
        let crown = (canopy * CROWN_OF_CANOPY).min(room);
        let (cx, z) = ((x0 + x1) / 2.0, (g.full.0 + g.full.1) / 2.0);
        let base = drawn_mm(height_mm(KINDS[segs[g.piece].kind].id));
        if !wide_enough(p, room, z) || z - crown / 2.0 < p.reach.near || z + crown / 2.0 > p.reach.far {
            continue;
        }
        out.push(Block { solid: Solid::Trunk, x: (cx - 125.0, cx + 125.0), y: (base, base + TRUNK_MM), z: (z - 125.0, z + 125.0) });
        out.push(Block {
            solid: Solid::Crown,
            x: (cx - crown / 2.0, cx + crown / 2.0),
            y: (base + TRUNK_MM, base + TRUNK_MM + CROWN_MM),
            z: (z - crown / 2.0, z + crown / 2.0),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::shared::catalogue::Side;
    use crate::shared::units::Units;
    use crate::street::iso::projection::points;
    use crate::street::iso::testing::{count, drawn, en, joined, joined_from_last};
    use crate::street::iso::{Projection, drawn_mm, iso_label, iso_svg};
    use crate::street::model::{Editor, Green, GreenKind, SAMPLES, Street, TreeSize, Use, ZoneEnd, ZoneFrame};

    fn fnv(s: &str) -> u64 {
        s.bytes().fold(0xcbf29ce484222325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3))
    }

    /// Sample Street 1, 60 m long, anchored at its first end: sidewalks 0 to 3 300 and 14 700 to 18 000 mm across.
    fn street() -> Editor {
        framed(Editor::new(0), ZoneEnd::First)
    }

    fn framed(e: Editor, from_end: ZoneEnd) -> Editor {
        e.with_frame(Some(ZoneFrame { length_mm: 60_000, from_end }))
    }

    fn projection(e: &Editor) -> Projection {
        let v = e.view();
        Projection::fit(1050.0, v.row_mm.max(v.total_mm))
    }

    /// A tree of size `size` in the pit it is given, centred `offset_mm` across.
    fn tree(from_mm: i32, offset_mm: i32, size: TreeSize) -> Green {
        Green { kind: GreenKind::Tree, from_mm, length_mm: 0, offset_mm, width_mm: 0, size: Some(size) }
    }

    /// An element across the left sidewalk (3 300 mm wide), 100 mm short of its far edge: positions snap to 100 mm.
    fn on_the_left(kind: GreenKind, from_mm: i32, length_mm: i32) -> Green {
        Green { kind, from_mm, length_mm, offset_mm: 1_600, width_mm: 3_200, size: None }
    }

    /// The top of a stretch of ground `x0` to `x1` across and `z0` to `z1` along, at height `y`, as the drawing writes it.
    fn flat(p: &Projection, (x0, x1): (f64, f64), y: f64, (z0, z1): (f64, f64)) -> String {
        points(p, &[(x0, y, z0), (x1, y, z0), (x1, y, z1), (x0, y, z1)])
    }

    /// The front face, `(x0, x1)` across, of a trunk `half` wide, from `y0` to `y1` at `z`.
    fn face(p: &Projection, (x0, x1): (f64, f64), (y0, y1): (f64, f64), z: f64) -> String {
        points(p, &[(x0, y0, z), (x1, y0, z), (x1, y1, z), (x0, y1, z)])
    }

    fn trunk_front(p: &Projection, cx: f64, h: f64, z: f64) -> String {
        format!("<g class=\"iso-solid s-trunk\"><polygon class=\"o-front\" points=\"{}\"", face(p, (cx - 125.0, cx + 125.0), (h, h + 2_000.0), z - 125.0))
    }

    #[test]
    fn a_street_without_green_draws_as_before() {
        // Hashes of what these drew at 60bfe19, before the 2.5D knew green: every sample, and a street joined to its
        // neighbour in either direction.
        let h = |m: &str| (m.len(), fnv(m));
        for (i, pin) in
            [(35652, 0x18fbf355ef73b8cc), (53017, 0x36401776cea26501), (34349, 0xa8c4c46d4918868), (39725, 0x7f5edc34ccaa6191)].into_iter().enumerate()
        {
            assert_eq!(h(&drawn(&Editor::new(i))), pin, "{}", SAMPLES[i].name);
        }
        assert_eq!(h(&iso_svg(&joined_from_last(true, 0), &en(), 1050.0).markup), (45274, 0x4a03b0e761ad60c2));
        assert_eq!(h(&iso_svg(&joined(false, 1), &en(), 1050.0).markup), (45648, 0x3253ebfea4e98c7f));
        // A street that holds a length and an end for its elements, and none, draws the same.
        assert_eq!(drawn(&street()), drawn(&Editor::new(0)));
    }

    #[test]
    fn a_tree_draws_its_pit_trunk_and_crown() {
        let mut e = street();
        let none = drawn(&e);
        e.add_green(tree(12_000, 1_750, TreeSize::M)).unwrap();
        let p = projection(&e);
        let m = drawn(&e);
        let h = drawn_mm(150);
        // The medium tree's 1 900 mm pit lies flush with the sidewalk's top, in the tree-pit material.
        let pit = flat(&p, (800.0, 2_700.0), h, (12_000.0, 13_900.0));
        assert!(m.contains(&format!("<polygon class=\"iso-top k-planting mat-trees\" points=\"{pit}\"/>")), "its pit");
        // Its trunk stands at the pit's centre, 250 mm across and 2 m tall.
        assert!(m.contains(&trunk_front(&p, 1_750.0, h, 12_950.0)), "its trunk");
        // The crown is 30% of the canopy (8 m): 2.4 m across and along, over the trunk.
        let top = flat(&p, (550.0, 2_950.0), h + 5_000.0, (11_750.0, 14_150.0));
        assert!(m.contains(&format!("<polygon class=\"o-top\" points=\"{top}\"/>")), "its crown");
        assert_eq!((count(&m, "s-trunk") - count(&none, "s-trunk"), count(&m, "s-crown") - count(&none, "s-crown")), (1, 1));
        // It hovers as a tree.
        assert!(m.contains("<title>Tree</title>"));
    }

    #[test]
    fn a_tree_is_drawn_whole_or_not_at_all_but_its_pit_stays() {
        let mut e = street();
        let none = drawn(&e);
        // A pit at the very start: its crown would reach before the stretch.
        e.add_green(tree(0, 1_700, TreeSize::M)).unwrap();
        let m = drawn(&e);
        assert_eq!((count(&m, "s-trunk"), count(&m, "s-crown")), (count(&none, "s-trunk"), count(&none, "s-crown")));
        assert_eq!(count(&m, "class=\"iso-top k-planting mat-trees\""), 1, "the pit is drawn");
    }

    #[test]
    fn the_crown_is_as_wide_as_the_size_says() {
        for (size, crown) in [(TreeSize::S, 1_800.0), (TreeSize::M, 2_400.0), (TreeSize::L, 3_000.0)] {
            let mut e = street();
            e.add_green(tree(20_000, 1_750, size)).unwrap();
            let (m, p) = (drawn(&e), projection(&e));
            let pit = f64::from(size.default_pit_mm());
            let z = 20_000.0 + pit / 2.0;
            let top = flat(&p, (1_750.0 - crown / 2.0, 1_750.0 + crown / 2.0), drawn_mm(150) + 5_000.0, (z - crown / 2.0, z + crown / 2.0));
            assert!(m.contains(&format!("<polygon class=\"o-top\" points=\"{top}\"/>")), "{size:?}");
        }
    }

    #[test]
    fn a_bed_is_raised() {
        let mut e = street();
        e.add_green(on_the_left(GreenKind::Bed, 10_000, 4_000)).unwrap();
        let (m, p) = (drawn(&e), projection(&e));
        let (h, rise) = (drawn_mm(150), 150.0);
        // It lies across the sidewalk and stands its planting height, 150 mm, above it: its top, in the planted
        // material, and its front face at its near end.
        let top = flat(&p, (0.0, 3_200.0), h + rise, (10_000.0, 14_000.0));
        assert!(m.contains(&format!("<polygon class=\"iso-top k-planting mat-planted\" points=\"{top}\"/>")), "its top");
        let front = face(&p, (0.0, 3_200.0), (h, h + rise), 10_000.0);
        assert!(m.contains(&format!("<polygon class=\"iso-front k-planting\" points=\"{front}\"")), "its front");
        assert!(m.contains("<title>Planting bed</title>"));
        // No solid stands on it.
        assert_eq!(count(&m, "s-trunk"), count(&drawn(&street()), "s-trunk"));
    }

    #[test]
    fn a_rain_garden_is_flush_with_an_outline() {
        for (kind, length) in [(GreenKind::RainGarden, 6_000), (GreenKind::Bioswale, 20_000)] {
            let mut e = street();
            let none = drawn(&e);
            assert!(!none.contains("rain_garden"), "a street with none carries no rain garden pattern");
            e.add_green(on_the_left(kind, 10_000, length)).unwrap();
            let (m, p) = (drawn(&e), projection(&e));
            let top = flat(&p, (0.0, 3_200.0), drawn_mm(150), (10_000.0, f64::from(10_000 + length)));
            assert!(m.contains(&format!("<polygon class=\"iso-top k-planting mat-rain_garden\" points=\"{top}\"/>")), "{kind:?} top, flush with the sidewalk");
            assert!(m.contains(&format!("<polygon class=\"iso-green-edge\" points=\"{top}\"/>")), "{kind:?} outline");
            assert_eq!(count(&m, "class=\"iso-front k-planting\""), 0, "{kind:?} stands no higher than the sidewalk");
            assert!(m.contains("<pattern id=\"iso-m-rain_garden\""), "{kind:?}: the pattern its top is filled with is defined");
        }
    }

    #[test]
    fn an_element_on_a_roadway_piece_is_not_drawn() {
        let mut s = Street::sample(0, Side::Right);
        s.length_mm = Some(60_000);
        // 8 000 mm across is the first driving lane.
        s.green.push(Green { kind: GreenKind::Tree, from_mm: 12_000, length_mm: 1_900, offset_mm: 8_000, width_mm: 1_900, size: Some(TreeSize::M) });
        s.green.push(Green { kind: GreenKind::RainGarden, from_mm: 20_000, length_mm: 6_000, offset_mm: 8_000, width_mm: 3_300, size: None });
        let e = framed(Editor::from_street(&s, &s, 0), ZoneEnd::First);
        assert!(e.view().green_shown.is_empty());
        assert_eq!(drawn(&e), drawn(&street()));
    }

    #[test]
    fn anchored_at_the_last_end_the_element_draws_where_it_stands() {
        let mut e = framed(Editor::new(0), ZoneEnd::Last);
        e.add_green(tree(12_000, 1_750, TreeSize::M)).unwrap();
        let (m, p) = (drawn(&e), projection(&e));
        // 12 m to 13.9 m from the last end is 28 m to 26.1 m from the stretch's near end, across unchanged.
        let pit = flat(&p, (800.0, 2_700.0), drawn_mm(150), (26_100.0, 28_000.0));
        assert!(m.contains(&format!("<polygon class=\"iso-top k-planting mat-trees\" points=\"{pit}\"/>")), "its pit");
        assert!(m.contains(&trunk_front(&p, 1_750.0, drawn_mm(150), 27_050.0)), "its trunk at the middle");
    }

    #[test]
    fn an_explicit_tree_hides_the_automatic_trees_near_it_only() {
        // Sample Avenue 2: a strip of street trees (piece 1, 3 500 to 5 000 mm across) with a tree every 8 m from 4 m.
        let before = framed(Editor::new(1), ZoneEnd::First);
        let h = drawn_mm(150);
        let p = projection(&before);
        let auto = |cx: f64, z: f64| trunk_front(&p, cx, h, z);
        let m0 = drawn(&before);
        for z in [4_000.0, 12_000.0, 20_000.0, 28_000.0, 36_000.0] {
            assert!(m0.contains(&auto(4_250.0, z)), "the strip has its tree at {z}");
        }
        let mut e = before;
        // Explicit trees centred at 10.75 m and 16.75 m along, each in the strip.
        // The strip is 1 500 wide, which a pit on the 100 mm grid cannot fill: 1 400 wide, 3 500 to 5 000 across.
        e.add_green(Green { width_mm: 1_400, ..tree(10_000, 4_200, TreeSize::S) }).unwrap();
        e.add_green(Green { width_mm: 1_400, ..tree(16_000, 4_200, TreeSize::S) }).unwrap();
        let m = drawn(&e);
        // Within 4 m of an explicit tree (12 m is 1.25 m from the first, 20 m is 3.25 m from the second) the automatic
        // tree is gone; at 4 m (6.75 m from the first), 28 m and 36 m, and 12 m's mirror on the other strip, it stays.
        for (cx, z, kept) in
            [(4_250.0, 4_000.0, true), (4_250.0, 12_000.0, false), (4_250.0, 20_000.0, false), (4_250.0, 28_000.0, true), (4_250.0, 36_000.0, true)]
        {
            assert_eq!(m.contains(&auto(cx, z)), kept, "the automatic tree at {z}");
        }
        assert!(m.contains(&auto(25_750.0, 12_000.0)), "the other strip is another piece, whose trees stay");
        // The explicit trees stand in their place: the same number of trunks as before less two plus two.
        assert_eq!(count(&m, "s-trunk"), count(&m0, "s-trunk"));
    }

    #[test]
    fn an_explicit_tree_whose_crown_is_not_drawn_leaves_the_automatic_trees_alone() {
        // Avenue 2's strip has an automatic tree at 4 m. An explicit tree at the very start has no crown in the
        // stretch (it would reach before it), so nothing stands in the automatic tree's place and it stays.
        let before = framed(Editor::new(1), ZoneEnd::First);
        let h = drawn_mm(150);
        let p = projection(&before);
        let auto = trunk_front(&p, 4_250.0, h, 4_000.0);
        assert!(drawn(&before).contains(&auto));
        let mut e = before;
        e.add_green(Green { width_mm: 1_400, ..tree(0, 4_200, TreeSize::S) }).unwrap();
        let m = drawn(&e);
        assert!(m.contains(&auto), "within 4 m of the pit, but the pit draws no tree");
        // One whose crown is drawn still takes the automatic tree's place.
        let mut e = framed(Editor::new(1), ZoneEnd::First);
        e.add_green(Green { width_mm: 1_400, ..tree(3_000, 4_200, TreeSize::S) }).unwrap();
        assert!(!drawn(&e).contains(&auto));
    }

    #[test]
    fn the_curb_rain_garden_uses_the_rain_garden_material() {
        let mut e = street();
        e.add_curb(Side::Left, 10_000, 6_000, Use::RainGarden).unwrap();
        e.add_curb(Side::Right, 10_000, 6_000, Use::RainGarden).unwrap();
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-top k-planting mat-rain_garden\""), 2);
        assert!(!m.contains("class=\"iso-top k-planting mat-planted\""), "no longer a bed's soil");
        assert!(m.contains("<pattern id=\"iso-m-rain_garden\""));
        // Pinned after looking at the picture: the two raised beds on the parking lanes are dark wet soil with cobbles
        // and reeds, where they were a bed's brown mulch; nothing else of the drawing changed.
        assert_eq!((m.len(), fnv(&m)), (45148, 0x73d487904f4efe65));
    }

    #[test]
    fn the_label_counts_the_green_elements_shown() {
        let mut e = street();
        let none = iso_label(&e.view(), &en(), Units::Metres);
        assert!(!none.contains("Trees and plantings"));
        e.add_green(tree(12_000, 1_750, TreeSize::M)).unwrap();
        e.add_green(on_the_left(GreenKind::Bed, 20_000, 4_000)).unwrap();
        e.add_green(tree(50_000, 1_700, TreeSize::M)).unwrap();
        assert_eq!(iso_label(&e.view(), &en(), Units::Metres), format!("{none} Trees and plantings shown: 2."));
        let fr = crate::i18n_for(crate::shared::i18n::Locale::FrCa);
        assert!(iso_label(&e.view(), &fr, Units::Metres).ends_with("Arbres et plantations montrés\u{a0}: 2."));
    }
}
