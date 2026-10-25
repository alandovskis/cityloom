//! The curbside uses in the 2.5D: on the selected zone's curbside pieces, each use the stretch shows is drawn over its
//! length as the kind and material it stands for (`Use::kind_id`, `Use::material`), the lane showing in the gaps. A
//! parking or loading use lies on the roadway, in the surface layer, in the lane's own group; the rest stand at
//! sidewalk height, a raised strip of their own (`lift`) in the raised layer with its faces; a tree is one tree at
//! the middle of its use and bike parking a rack every metre.

use std::fmt::Write;

use crate::shared::catalogue::{KINDS, MATERIALS, kind_index};
use crate::shared::i18n::I18n;
use crate::shared::iso::format::esc;
use crate::street::model::{SegView, ShownInstance, ShownStop};

use super::order::Bx;
use super::projection::{Projection, Reach};
use super::raised::{slab_of, top_over};
use super::solids::{Block, CROWN_HALF_DEPTH_MM, Solid, wide_enough};
use super::taper::Draw;

/// One stretch of a curbside piece, drawn flat.
pub(super) struct Part {
    pub along: (f64, f64),
    pub kind: usize,
    pub material: &'static str,
}

/// The kind and material something is drawn as on `piece`: its own material, where it is the piece's kind.
fn look_of(piece: &SegView, kind_id: &str, material: Option<&'static str>) -> (usize, &'static str) {
    let kind = kind_index(kind_id).expect("a use is drawn as a catalogue kind");
    let material = material.unwrap_or(if kind == piece.kind { piece.material } else { MATERIALS[KINDS[kind].materials[0]].id });
    (kind, material)
}

/// Something laid over a piece for part of the stretch, drawn as a kind and material of its own: a curbside use
/// or the platform of a transit stop.
#[derive(Clone, Copy, Debug)]
pub(super) struct Over {
    pub piece: usize,
    pub along: (f64, f64),
    /// It stands at sidewalk height (a strip of its own in the raised layer) rather than lying on the roadway.
    pub raised: bool,
    pub kind: usize,
    pub material: &'static str,
    /// The message that names it when hovered.
    pub name_key: &'static str,
}

/// What is laid over the pieces: the shown curbside uses, then the platforms of the shown stops.
pub(super) fn overs(segs: &[SegView], curb: &[ShownInstance], stops: &[ShownStop]) -> Vec<Over> {
    let uses = curb.iter().map(|m| {
        let (kind, material) = look_of(&segs[m.piece], m.use_.kind_id(), m.use_.material());
        Over { piece: m.piece, along: m.along, raised: m.use_.raised(), kind, material, name_key: m.use_.name_key() }
    });
    // A bulb's or an island's platform is the sidewalk reaching over its pieces.
    let platforms = stops.iter().flat_map(|s| {
        s.raised.iter().map(move |&piece| {
            let (kind, material) = look_of(&segs[piece], "sidewalk", None);
            Over { piece, along: s.along, raised: true, kind, material, name_key: s.kind.name_key() }
        })
    });
    uses.chain(platforms).collect()
}

/// The part of `parts` outside `range`, in order.
fn cut(parts: Vec<Part>, range: (f64, f64)) -> Vec<Part> {
    let mut out = Vec::new();
    for p in parts {
        let (before, after) = ((p.along.0, p.along.1.min(range.0)), (p.along.0.max(range.1), p.along.1));
        if before.1 > before.0 {
            out.push(Part { along: before, kind: p.kind, material: p.material });
        }
        if after.1 > after.0 {
            out.push(Part { along: after, kind: p.kind, material: p.material });
        }
    }
    out
}

/// The flat parts of the piece at `i` over `reach`: its gaps in its own look and the roadway uses (parking, loading)
/// in theirs, in order along the stretch; what stands up is left out (`lifted`). A raised platform over a roadway
/// use hides it. None when nothing is laid on the piece there, so the piece is drawn as it always was.
pub(super) fn flat_parts(segs: &[SegView], i: usize, reach: Reach, shown: &[Over]) -> Option<Vec<Part>> {
    let mine: Vec<&Over> = shown.iter().filter(|o| o.piece == i).collect();
    if mine.is_empty() {
        return None;
    }
    let s = &segs[i];
    let mut parts = vec![Part { along: (reach.near, reach.far), kind: s.kind, material: s.material }];
    // The roadway uses lie over the piece's own look, then the raised ones cover them both.
    for m in mine.iter().filter(|m| !m.raised) {
        parts = cut(parts, m.along);
        parts.push(Part { along: m.along, kind: m.kind, material: m.material });
    }
    for m in mine.iter().filter(|m| m.raised) {
        parts = cut(parts, m.along);
    }
    parts.sort_by(|a, b| a.along.0.total_cmp(&b.along.0));
    Some(parts)
}

/// The flat parts of a piece, each its own top over its part of the stretch; `edge` gives the piece's edges along the
/// street where a transition moves them.
pub(super) fn flat(out: &mut String, p: &Projection, segs: &[SegView], i: usize, parts: &[Part], edge: Option<&dyn Fn(f64) -> (f64, f64)>) {
    let s = &segs[i];
    let (x0, x1) = (s.x_mm as f64, (s.x_mm + s.width_mm) as f64);
    for part in parts {
        let sub = p.over(Reach { near: part.along.0, far: part.along.1 });
        let id = KINDS[part.kind].id;
        match edge {
            Some(edge) => top_over(out, &sub, id, part.material, 0.0, edge, false),
            None => top_over(out, &sub, id, part.material, 0.0, &|_| (x0, x1), true),
        }
    }
}

/// Something that stands at sidewalk height on the piece at `piece`.
#[derive(Clone, Copy, Debug)]
pub(super) struct Lift {
    pub piece: usize,
    pub along: (f64, f64),
    pub kind: usize,
    pub material: &'static str,
    pub name_key: &'static str,
}

/// The raised things shown, for the raised layer.
pub(super) fn lifted(shown: &[Over]) -> Vec<Lift> {
    shown.iter().filter(|o| o.raised).map(|o| Lift { piece: o.piece, along: o.along, kind: o.kind, material: o.material, name_key: o.name_key }).collect()
}

/// The edges across of a lifted strip's piece along the stretch: the transition's where the piece tapers, else none.
fn edges<'a>(segs: &[SegView], l: &Lift, p: &'a Projection, taper: Option<&'a Draw>) -> Option<impl Fn(f64) -> (f64, f64) + 'a> {
    let d = taper?;
    let shape = d.shape_of(&segs[l.piece])?;
    let reach = p.reach;
    Some(move |z: f64| d.edges(shape, reach, z))
}

/// The box a lifted strip fills, for ordering.
pub(super) fn lift_box(segs: &[SegView], l: &Lift, p: &Projection, taper: Option<&Draw>) -> Bx {
    let s = &segs[l.piece];
    let h = super::projection::drawn_mm(super::raised::height_mm(KINDS[l.kind].id));
    let x = match edges(segs, l, p, taper) {
        Some(e) => {
            let (a, b) = (e(l.along.0), e(l.along.1));
            (a.0.min(b.0), a.1.max(b.1))
        }
        None => (s.x_mm as f64, (s.x_mm + s.width_mm) as f64),
    };
    Bx { x, y: (0.0, h), z: l.along }
}

/// A lifted strip of its own, hovered by its name; on a tapering piece it follows the piece's edges.
pub(super) fn lift(out: &mut String, p: &Projection, segs: &[SegView], i18n: &I18n, l: &Lift, taper: Option<&Draw>) {
    let s = &segs[l.piece];
    let name = i18n.tr(l.name_key, &crate::shared::i18n::Args::new());
    write!(out, "<g class=\"iso-piece\" data-uid=\"{}\"><title>{}</title>", s.uid, esc(&name)).unwrap();
    let edge = edges(segs, l, p, taper);
    slab_of(out, p, segs, l.piece, KINDS[l.kind].id, l.material, l.along, edge.as_ref().map(|e| e as &dyn Fn(f64) -> (f64, f64)));
    out.push_str("</g>");
}

/// What stands on the shown uses: a tree at the middle of a tree's, a rack every metre of a bike corral's. Each stands
/// on its use's drawn top and where its piece is wide enough on the screen to show it, as the other solids do.
pub(super) fn blocks(segs: &[SegView], shown: &[ShownInstance], p: &Projection) -> Vec<Block> {
    use crate::street::model::Use;
    let mut out = Vec::new();
    for m in shown {
        let s = &segs[m.piece];
        let (cx, w) = (s.x_mm as f64 + s.width_mm as f64 / 2.0, s.width_mm as f64);
        let h = super::projection::drawn_mm(super::raised::height_mm(m.use_.kind_id()));
        match m.use_ {
            Use::Tree => {
                // At the middle of the whole use, drawn only if that lies in the reach (the retain below).
                let z = (m.full.0 + m.full.1) / 2.0;
                // Whole or not at all: the crown is the widest part along the street, so it decides.
                if wide_enough(p, w, z) && z - CROWN_HALF_DEPTH_MM >= p.reach.near && z + CROWN_HALF_DEPTH_MM <= p.reach.far {
                    let crown = (w * 0.9).min(2_400.0);
                    out.push(Block { solid: Solid::Trunk, x: (cx - 125.0, cx + 125.0), y: (h, h + 2_000.0), z: (z - 125.0, z + 125.0) });
                    out.push(Block {
                        solid: Solid::Crown,
                        x: (cx - crown / 2.0, cx + crown / 2.0),
                        y: (h + 2_000.0, h + 5_000.0),
                        z: (z - CROWN_HALF_DEPTH_MM, z + CROWN_HALF_DEPTH_MM),
                    });
                }
            }
            Use::BikeParking => {
                // No bikerack solid exists, so a rack is a post. Phased from the whole use's start, a metre apart.
                let racks = (0..).map(|n| m.full.0 + 500.0 + f64::from(n) * 1_000.0).take_while(|z| *z < m.full.1);
                for z in racks.filter(|&z| z >= m.along.0 && z <= m.along.1 && wide_enough(p, w, z)) {
                    out.push(Block { solid: Solid::Post, x: (cx - 60.0, cx + 60.0), y: (h, h + 900.0), z: (z - 60.0, z + 60.0) });
                }
            }
            _ => {}
        }
    }
    // A box stands only where it lies wholly in the reach.
    out.retain(|b| b.z.0 >= p.reach.near && b.z.1 <= p.reach.far);
    out
}

#[cfg(test)]
mod tests {
    use crate::shared::catalogue::{KINDS, MATERIALS, Side, kind_index};
    use crate::shared::units::Units;
    use crate::street::iso::projection::points;
    use crate::street::iso::testing::{count, drawn, en, furnished, joined, marked};
    use crate::street::iso::{Projection, drawn_mm, iso_label, iso_svg};
    use crate::street::model::{Editor, Street, Use, ZoneEnd, ZoneFrame};

    /// Sample Street 1 (parking at both edges: pieces 1 and 4) 60 m long, anchored at its first end.
    fn street() -> Editor {
        let s = Street::sample(0, Side::Right);
        Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::First }))
    }

    fn projection(e: &Editor) -> Projection {
        let v = e.view();
        Projection::fit(1050.0, v.row_mm.max(v.total_mm))
    }

    /// The markup of the group drawn for `uid` the `nth` time (0 is the first, in the surface layer).
    fn group(markup: &str, uid: u32, nth: usize) -> String {
        let g = markup.split(&format!("<g class=\"iso-piece\" data-uid=\"{uid}\">")).nth(nth + 1).expect("the group is drawn");
        g[..g.find("</g></g>").or_else(|| g.find("</g>")).unwrap()].to_string()
    }

    fn fnv(s: &str) -> u64 {
        s.bytes().fold(0xcbf29ce484222325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3))
    }

    #[test]
    fn no_instances_draws_as_today() {
        // Hashes of what these fixtures drew before the curbside uses existed.
        for (name, markup, len, hash) in [
            ("sample 0", drawn(&Editor::new(0)), 35652, 0x18fbf355ef73b8cc),
            ("sample 1", drawn(&Editor::new(1)), 53017, 0x36401776cea26501),
            ("furnished", drawn(&furnished()), 69000, 0xf88ed5b72e2384ac),
            ("marked", drawn(&marked()), 47542, 0x75d1da19ec99cfaa),
            ("joined", iso_svg(&joined(true, 1), &en(), 1050.0).markup, 45283, 0xba2cb2c0965d37ce),
        ] {
            assert_eq!((markup.len(), fnv(&markup)), (len, hash), "{name}");
        }
        // A street anchored for a programme that holds none draws the same.
        assert_eq!(drawn(&street()), drawn(&Editor::new(0)));
    }

    #[test]
    fn a_patio_is_drawn_raised_over_its_length_only() {
        let mut e = street();
        e.add_curb(Side::Right, 10_000, 6_000, Use::Patio).unwrap();
        let (v, p) = (e.view(), projection(&e));
        let (piece, uid) = (&v.segments[4], v.segments[4].uid);
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let (x0, x1, h) = (piece.x_mm as f64, (piece.x_mm + piece.width_mm) as f64, drawn_mm(150));
        // The patio's top stands at sidewalk height over 10 m to 16 m, in the terrace's tint and material.
        let top = points(&p, &[(x0, h, 10_000.0), (x1, h, 10_000.0), (x1, h, 16_000.0), (x0, h, 16_000.0)]);
        let material = MATERIALS[KINDS[kind_index("terrace").unwrap()].materials[0]].id;
        assert!(m.contains(&format!("<polygon class=\"iso-top k-terrace mat-{material}\" points=\"{top}\"/>")), "its raised top");
        // Its front face is the cut at its near end.
        let front = points(&p, &[(x0, 0.0, 10_000.0), (x1, 0.0, 10_000.0), (x1, h, 10_000.0), (x0, h, 10_000.0)]);
        assert!(m.contains(&format!("class=\"iso-front k-terrace\" points=\"{front}\"")), "its front");
        // The lane keeps the rest: before it and after it, nothing over the patio.
        let lane = group(&m, uid, 0);
        assert_eq!(count(&lane, "class=\"iso-top k-parking "), 2);
        let before = points(&p, &[(x0, 0.0, 0.0), (x1, 0.0, 0.0), (x1, 0.0, 10_000.0), (x0, 0.0, 10_000.0)]);
        let after = points(&p, &[(x0, 0.0, 16_000.0), (x1, 0.0, 16_000.0), (x1, 0.0, 40_000.0), (x0, 0.0, 40_000.0)]);
        assert!(lane.contains(&format!("points=\"{before}\"")) && lane.contains(&format!("points=\"{after}\"")));
        // The raised strip hovers as the use.
        assert!(m.contains("<title>Patio</title>"));
    }

    #[test]
    fn a_tree_instance_has_one_tree() {
        let mut e = street();
        let none = count(&drawn(&e), "class=\"iso-solid s-trunk\"");
        e.add_curb(Side::Left, 10_000, 6_000, Use::Tree).unwrap();
        let (v, p) = (e.view(), projection(&e));
        let m = iso_svg(&v, &en(), 1050.0).markup;
        assert_eq!(count(&m, "class=\"iso-solid s-trunk\""), none + 1);
        assert_eq!(count(&m, "class=\"iso-solid s-crown\""), none + 1);
        // At the middle of the instance, on the lane, 13 m along: a trunk 250 mm across, as a street tree's is.
        let piece = &v.segments[1];
        let (cx, h) = (piece.x_mm as f64 + piece.width_mm as f64 / 2.0, drawn_mm(150));
        let trunk =
            points(&p, &[(cx - 125.0, h, 12_875.0), (cx + 125.0, h, 12_875.0), (cx + 125.0, h + 2_000.0, 12_875.0), (cx - 125.0, h + 2_000.0, 12_875.0)]);
        assert!(m.contains(&format!("<g class=\"iso-solid s-trunk\"><polygon class=\"o-front\" points=\"{trunk}\"")), "the trunk");
        // Its pit is planting in the trees material.
        assert!(m.contains("class=\"iso-top k-planting mat-trees\""));
    }

    #[test]
    fn bike_parking_has_a_rack_every_metre() {
        let mut e = street();
        e.add_curb(Side::Right, 10_000, 4_000, Use::BikeParking).unwrap();
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-solid s-post\""), 4);
        assert!(m.contains("class=\"iso-top k-bikerack "));
    }

    #[test]
    fn gaps_keep_the_lane() {
        let mut e = street();
        e.add_curb(Side::Right, 10_000, 9_000, Use::Loading).unwrap();
        let (v, p) = (e.view(), projection(&e));
        let (piece, uid) = (&v.segments[4], v.segments[4].uid);
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let lane = group(&m, uid, 0);
        let (x0, x1) = (piece.x_mm as f64, (piece.x_mm + piece.width_mm) as f64);
        // Parking before and after, loading between: one group, three tops that meet.
        assert_eq!((count(&lane, "class=\"iso-top k-parking "), count(&lane, "class=\"iso-top k-loading ")), (2, 1));
        let loading = points(&p, &[(x0, 0.0, 10_000.0), (x1, 0.0, 10_000.0), (x1, 0.0, 19_000.0), (x0, 0.0, 19_000.0)]);
        assert!(lane.contains(&format!("class=\"iso-top k-loading mat-asphalt\" points=\"{loading}\"")), "{lane}");
        // A loading bay is flat: nothing is raised for it.
        assert!(!m.contains("<title>Loading</title>"));
    }

    #[test]
    fn instances_outside_the_stretch_are_not_drawn() {
        let mut e = street();
        let none = drawn(&e);
        e.add_curb(Side::Right, 45_000, 6_000, Use::Patio).unwrap();
        e.add_curb(Side::Left, 41_000, 2_000, Use::Tree).unwrap();
        assert_eq!(drawn(&e), none);
    }

    #[test]
    fn the_label_counts_the_uses() {
        let mut e = street();
        let none = iso_label(&e.view(), &en(), Units::Metres);
        assert!(!none.contains("curbside"));
        e.add_curb(Side::Right, 10_000, 6_000, Use::Patio).unwrap();
        assert_eq!(iso_label(&e.view(), &en(), Units::Metres), format!("{none} One curbside use is shown."));
        e.add_curb(Side::Left, 10_000, 6_000, Use::Tree).unwrap();
        e.add_curb(Side::Left, 50_000, 6_000, Use::Tree).unwrap();
        assert_eq!(iso_label(&e.view(), &en(), Units::Metres), format!("{none} 2 curbside uses are shown."));
        let fr = crate::i18n_for(crate::shared::i18n::Locale::FrCa);
        assert!(iso_label(&e.view(), &fr, Units::Metres).ends_with("2 usages du bord de rue sont montrés."));
    }

    #[test]
    fn anchored_at_the_last_end_a_use_is_drawn_mirrored_along_the_stretch() {
        let s = Street::sample(0, Side::Right);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::Last }));
        e.add_curb(Side::Right, 12_000, 6_000, Use::Patio).unwrap();
        let (v, p) = (e.view(), projection(&e));
        let piece = &v.segments[4];
        let (x0, x1, h) = (piece.x_mm as f64, (piece.x_mm + piece.width_mm) as f64, drawn_mm(150));
        let top = points(&p, &[(x0, h, 22_000.0), (x1, h, 22_000.0), (x1, h, 28_000.0), (x0, h, 28_000.0)]);
        assert!(iso_svg(&v, &en(), 1050.0).markup.contains(&format!("class=\"iso-top k-terrace")), "drawn");
        assert!(iso_svg(&v, &en(), 1050.0).markup.contains(&format!("points=\"{top}\"")), "from the stretch's near end, mirrored");
    }

    #[test]
    fn the_neighbour_across_a_boundary_shows_no_uses() {
        let mut e = street();
        e.add_curb(Side::Right, 12_000, 6_000, Use::Patio).unwrap();
        e.add_curb(Side::Right, 24_000, 6_000, Use::Patio).unwrap();
        e.split_zone(Some(20_000)).unwrap();
        assert!(e.select_zone(0));
        assert!(e.view().zones.joint.is_some());
        // The patio of zone 1 is drawn; the one beyond the boundary, in zone 2's half, is not.
        assert_eq!(count(&drawn(&e), "class=\"iso-top k-terrace"), 1);
    }

    #[test]
    fn a_tree_is_placed_from_the_whole_use_so_a_partly_shown_one_has_none_when_its_middle_is_outside() {
        let mut e = street();
        let none = count(&drawn(&e), "class=\"iso-solid s-trunk\"");
        // 38 m to 46 m: the middle, 42 m, is past the stretch, so the part shown gets no tree of its own.
        e.add_curb(Side::Left, 38_000, 8_000, Use::Tree).unwrap();
        assert_eq!(count(&drawn(&e), "class=\"iso-solid s-trunk\""), none);
        assert_eq!(count(&drawn(&e), "class=\"iso-top k-planting mat-trees\""), 1, "the pit is still shown");
    }

    #[test]
    fn racks_keep_their_places_along_the_street_when_the_use_is_cut_by_the_stretch() {
        // Anchored at the last end the use 38.3 m to 43.7 m lies at -3.7 m to 1.7 m of the stretch: racks are 0.5 m
        // in from its start, so one stands at 0.8 m (from the shown part's start they would stand at 0.5 m and 1.5 m).
        let s = Street::sample(0, Side::Right);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::Last }));
        e.add_curb(Side::Right, 38_300, 5_400, Use::BikeParking).unwrap();
        let (v, p) = (e.view(), projection(&e));
        let m = iso_svg(&v, &en(), 1050.0).markup;
        assert_eq!(count(&m, "class=\"iso-solid s-post\""), 1);
        let piece = &v.segments[4];
        let (cx, h) = (piece.x_mm as f64 + piece.width_mm as f64 / 2.0, drawn_mm(150));
        let front = points(&p, &[(cx - 60.0, h, 740.0), (cx + 60.0, h, 740.0), (cx + 60.0, h + 900.0, 740.0), (cx - 60.0, h + 900.0, 740.0)]);
        assert!(m.contains(&format!("<g class=\"iso-solid s-post\"><polygon class=\"o-front\" points=\"{front}\"")));
    }

    /// The corners of the first polygon of `class` in `markup`, as (x, y) in pixels.
    fn corners(markup: &str, class: &str) -> Vec<(f64, f64)> {
        let at = markup.find(class).unwrap_or_else(|| panic!("{class} is drawn"));
        let pts = &markup[at + markup[at..].find("points=\"").unwrap() + 8..];
        pts[..pts.find('"').unwrap()]
            .split(' ')
            .map(|pt| {
                let mut c = pt.split(',').map(|n| n.parse::<f64>().unwrap());
                (c.next().unwrap(), c.next().unwrap())
            })
            .collect()
    }

    #[test]
    fn on_a_tapering_piece_a_raised_use_follows_the_pieces_edges() {
        let mut e = street();
        e.add_curb(Side::Right, 24_000, 6_000, Use::Patio).unwrap();
        e.split_zone(Some(20_000)).unwrap();
        // The second zone has a wider parking lane than the first, and it hosts the transition into it.
        assert!(e.select_zone(1));
        let uid = e.view().segments[4].uid;
        assert!(e.set_width(uid, 3_000));
        let v = e.view();
        assert!(v.zones.joint.as_ref().and_then(|j| j.taper.as_ref()).is_some_and(|t| t.host_is_selected), "the selected zone hosts a taper");
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let lane = group(&m, v.segments[4].uid, 0);
        let (gap, raised) = (corners(&lane, "class=\"iso-top k-parking "), corners(&m, "class=\"iso-top k-terrace"));
        // The raised top starts where the gap before it ends: across, at 24 m, the corners are the lane's tapered ones.
        assert_eq!((raised[0].0, raised[1].0), (gap[3].0, gap[2].0));
        let (x0, x1) = (v.segments[4].x_mm as f64, (v.segments[4].x_mm + v.segments[4].width_mm) as f64);
        let p = projection(&e);
        let untapered = (p.at(x0, 0.0, 24_000.0).0, p.at(x1, 0.0, 24_000.0).0);
        assert_ne!((raised[0].0, raised[1].0), untapered, "the lane has moved by then");
    }

    #[test]
    fn a_tree_is_drawn_whole_or_not_at_all() {
        // A 2 m tree use at the very start: its crown (2.4 m across the street) would reach before the stretch.
        let mut e = street();
        let (trunks, crowns) = ("class=\"iso-solid s-trunk\"", "class=\"iso-solid s-crown\"");
        let base = (count(&drawn(&e), trunks), count(&drawn(&e), crowns));
        e.add_curb(Side::Left, 0, 2_000, Use::Tree).unwrap();
        let m = drawn(&e);
        assert_eq!((count(&m, trunks) - base.0, count(&m, crowns) - base.1), (0, 0), "neither a trunk nor a crown alone");
        // One whole in the stretch has both.
        e.add_curb(Side::Left, 10_000, 2_000, Use::Tree).unwrap();
        let m = drawn(&e);
        assert_eq!((count(&m, trunks) - base.0, count(&m, crowns) - base.1), (1, 1));
    }
}
