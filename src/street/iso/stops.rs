//! The transit stops in the 2.5D: the platform of a bulb or an island is drawn raised over its length like a curbside
//! use (`curbside::overs`); what stands on a stop (the shelter, the sign) is placed from the whole stop, so it stays
//! where it is as the stretch moves, and is drawn whole or not at all within the reach.

use crate::shared::catalogue::KINDS;
use crate::street::model::{SegView, ShownStop, View};

use super::projection::{Projection, Reach, STRETCH_MM, drawn_mm};
use super::raised::height_mm;
use super::sign::Panel;
use super::solids::{Block, shelter};

/// How far past a stop's near end its sign stands, in millimetres.
const SIGN_AT_MM: f64 = 500.0;

/// The drawn height of the top the stop's things stand on: the sidewalk's where the platform is raised, else the
/// piece's own.
fn top_mm(segs: &[SegView], s: &ShownStop, piece: usize) -> f64 {
    if s.raised.is_empty() { drawn_mm(height_mm(KINDS[segs[piece].kind].id)) } else { drawn_mm(height_mm("sidewalk")) }
}

/// The shelters of the stops that have one: a box at the middle of the whole stop, on the piece it stands on.
pub(super) fn blocks(segs: &[SegView], stops: &[ShownStop], p: &Projection) -> Vec<Block> {
    stops
        .iter()
        .filter(|s| s.shelter)
        .filter_map(|s| s.stand.map(|i| (s, i)))
        .flat_map(|(s, i)| {
            let piece = &segs[i];
            shelter(p, (piece.x_mm as f64, (piece.x_mm + piece.width_mm) as f64), s.full, top_mm(segs, s, i))
        })
        .collect()
}

/// The stops that are drawn at all in a drawing `width` wide: a raised platform, or a sign or a shelter that lies in
/// the reach of the selected zone's run. The label lists these, not the stops that merely overlap the stretch. It asks
/// the very functions that draw (`sign_panel`, `blocks`), so a sign scaled up to its minimum size, or a shelter on a
/// piece too narrow on the screen, is judged as it is drawn.
pub(super) fn drawn(v: &View, width: f64) -> Vec<&ShownStop> {
    let half = STRETCH_MM / 2.0;
    let reach = match &v.zones.joint {
        None => Reach::STRETCH,
        Some(j) if j.far => Reach { near: 0.0, far: half },
        Some(_) => Reach { near: half, far: STRETCH_MM },
    };
    let p = Projection::fit(width, super::span_mm(v)).over(reach);
    v.stops_shown
        .iter()
        .filter(|s| {
            let one = std::slice::from_ref(*s);
            !s.raised.is_empty() || sign_panel(&v.segments, s, &p).is_some() || !blocks(&v.segments, one, &p).is_empty()
        })
        .collect()
}

/// A bus on a stop sign's panel, in panel units (across and up, each 0 to 1): the body, its windows and its wheels.
const BODY: ((f64, f64), (f64, f64)) = ((0.16, 0.84), (0.30, 0.80));
const WINDOWS: [(f64, f64); 3] = [(0.22, 0.38), (0.42, 0.58), (0.62, 0.78)];
const WHEELS: [(f64, f64); 2] = [(0.26, 0.38), (0.62, 0.74)];

/// The panel of `s`'s sign, when it has a piece to stand on and the whole post stands in the reach.
fn sign_panel(segs: &[SegView], s: &ShownStop, p: &Projection) -> Option<Panel> {
    let i = s.stand?;
    let piece = &segs[i];
    let cx = piece.x_mm as f64 + piece.width_mm as f64 / 2.0;
    let sign = Panel::at(p, cx, top_mm(segs, s, i), s.full.0 + SIGN_AT_MM);
    sign.in_reach(p).then_some(sign)
}

/// The signs of the stops: a post at the stop's near end on the piece the stop stands on, and on it a white panel
/// with a bus. Drawn only where the whole post stands in the reach.
pub(super) fn signs(segs: &[SegView], stops: &[ShownStop], p: &Projection) -> String {
    let mut out = String::new();
    for s in stops {
        let Some(sign) = sign_panel(segs, s, p) else { continue };
        out.push_str("<g class=\"iso-stop-sign\" aria-hidden=\"true\">");
        sign.post(&mut out, p);
        sign.panel(&mut out, p);
        sign.rect(&mut out, p, "iso-sign-glyph", BODY.0, BODY.1);
        for u in WINDOWS {
            sign.rect(&mut out, p, "iso-sign-glyph-cut", u, (0.52, 0.72));
        }
        for u in WHEELS {
            sign.rect(&mut out, p, "iso-sign-glyph", u, (0.20, 0.32));
        }
        out.push_str("</g>");
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::shared::catalogue::{REGIONS, Side, kind_index};
    use crate::shared::units::Units;
    use crate::street::iso::projection::points;
    use crate::street::iso::testing::{count, drawn, en};
    use crate::street::iso::{Projection, drawn_mm, iso_label, iso_svg};
    use crate::street::model::{Editor, Piece, StopKind, Street, ZoneEnd, ZoneFrame};

    type Layout = [(&'static str, i32, Option<usize>)];

    /// Right-hand traffic: the bus runs away (direction 0) in the last lane; the curb is the right one.
    const CURB: [(&str, i32, Option<usize>); 4] = [("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("sidewalk", 3000, None)];
    /// A parking lane between the bus's lane and the sidewalk: a curb extension stands on it.
    const BULB: [(&str, i32, Option<usize>); 5] =
        [("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("parking", 2400, None), ("sidewalk", 3000, None)];
    /// A buffer and a bike lane behind it: a boarding island stands on the buffer.
    const ISLAND: [(&str, i32, Option<usize>); 6] = [
        ("sidewalk", 3000, None),
        ("travel", 3300, Some(1)),
        ("bus", 3300, Some(0)),
        ("buffer", 1800, None),
        ("bike", 1800, Some(0)),
        ("sidewalk", 3000, None),
    ];

    fn editor(pieces: &Layout, from_end: ZoneEnd) -> Editor {
        let pieces: Vec<Piece> =
            pieces.iter().map(|&(id, width_mm, direction)| Piece { kind: kind_index(id).unwrap(), width_mm, direction, variants: Vec::new() }).collect();
        let s = Street::imported(crate::shared::catalogue::StreetClass::Local, Side::Right, &pieces);
        let region = REGIONS.iter().position(|r| r.drive_side == Side::Right).unwrap();
        Editor::from_street(&s, &s, region).with_frame(Some(ZoneFrame { length_mm: 60_000, from_end }))
    }

    /// `pieces` with a stop on the right, 10 m to 22 m from the first end.
    fn with_stop(pieces: &Layout, kind: StopKind, shelter: bool) -> Editor {
        let mut e = editor(pieces, ZoneEnd::First);
        e.add_stop(Side::Right, 10_000, 12_000, kind).unwrap();
        if shelter {
            e.set_stop_shelter(0, true).unwrap();
        }
        e
    }

    fn projection(e: &Editor) -> Projection {
        let v = e.view();
        Projection::fit(1050.0, v.row_mm.max(v.total_mm))
    }

    fn fnv(s: &str) -> u64 {
        s.bytes().fold(0xcbf29ce484222325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3))
    }

    /// The stop sign's group.
    fn stop_sign(m: &str) -> &str {
        let at = m.find("<g class=\"iso-stop-sign\"").expect("a stop sign");
        &m[at..at + m[at..].find("</g></g>").map_or(m.len() - at, |i| i + 8)]
    }

    #[test]
    fn no_stops_draws_as_today() {
        // Hashes of what these fixtures drew before the stops existed.
        for (name, pieces, len, hash) in [
            ("curb", &CURB[..], 34324usize, 0xfe408c31ce4128abu64),
            ("bulb", &BULB[..], 35630, 0x79bae35920d099cf),
            ("island", &ISLAND[..], 39738, 0xdad452c8e5174c10),
        ] {
            let m = drawn(&editor(pieces, ZoneEnd::First));
            assert_eq!((m.len(), fnv(&m)), (len, hash), "{name}");
        }
    }

    #[test]
    fn a_bulb_platform_is_raised_over_its_length() {
        let e = with_stop(&BULB, StopKind::Bulb, false);
        let (v, p) = (e.view(), projection(&e));
        let piece = &v.segments[3];
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let (x0, x1, h) = (piece.x_mm as f64, (piece.x_mm + piece.width_mm) as f64, drawn_mm(150));
        // The platform's top stands at sidewalk height over 10 m to 22 m, in the sidewalk's tint.
        let top = points(&p, &[(x0, h, 10_000.0), (x1, h, 10_000.0), (x1, h, 22_000.0), (x0, h, 22_000.0)]);
        assert!(m.contains(&format!("class=\"iso-top k-sidewalk mat-concrete\" points=\"{top}\"")), "its raised top");
        let front = points(&p, &[(x0, 0.0, 10_000.0), (x1, 0.0, 10_000.0), (x1, h, 10_000.0), (x0, h, 10_000.0)]);
        assert!(m.contains(&format!("class=\"iso-front k-sidewalk\" points=\"{front}\"")), "its front");
        // The lane keeps the rest: before it and after it, nothing over the platform.
        let lane = m.split(&format!("<g class=\"iso-piece\" data-uid=\"{}\">", piece.uid)).nth(1).unwrap();
        let lane = &lane[..lane.find("</g></g>").or_else(|| lane.find("</g>")).unwrap()];
        assert_eq!(count(lane, "class=\"iso-top k-parking "), 2);
        // It hovers as the kind of stop.
        assert!(m.contains("<title>Curb extension</title>"));
        // An island raises its buffer, the same way.
        let e = with_stop(&ISLAND, StopKind::Island, false);
        assert!(drawn(&e).contains("<title>Boarding island</title>"));
    }

    #[test]
    fn a_curb_stop_draws_no_platform() {
        let none = drawn(&editor(&CURB, ZoneEnd::First));
        let m = drawn(&with_stop(&CURB, StopKind::Curb, false));
        assert!(!m.contains("<title>Curb stop</title>"));
        // The sidewalk is the platform: only the sign is added.
        assert_eq!(m.replace(stop_sign(&m), ""), none);
    }

    #[test]
    fn a_shelter_is_drawn_when_set() {
        let none = count(&drawn(&with_stop(&CURB, StopKind::Curb, false)), "class=\"iso-solid s-shelter\"");
        let e = with_stop(&CURB, StopKind::Curb, true);
        let (v, p) = (e.view(), projection(&e));
        let m = iso_svg(&v, &en(), 1050.0).markup;
        assert_eq!(count(&m, "class=\"iso-solid s-shelter\""), none + 1);
        // 4 m long at the middle of the stop (16 m), 1.5 m deep, on the sidewalk beside the lane.
        let piece = &v.segments[3];
        let (cx, h) = (piece.x_mm as f64 + piece.width_mm as f64 / 2.0, drawn_mm(150));
        let front =
            points(&p, &[(cx - 750.0, h, 14_000.0), (cx + 750.0, h, 14_000.0), (cx + 750.0, h + 2_500.0, 14_000.0), (cx - 750.0, h + 2_500.0, 14_000.0)]);
        assert!(m.contains(&format!("<g class=\"iso-solid s-shelter\"><polygon class=\"o-front\" points=\"{front}\"")), "the shelter's front");
        // A bulb's shelter stands on its platform.
        let bulb = drawn(&with_stop(&BULB, StopKind::Bulb, true));
        assert_eq!(count(&bulb, "class=\"iso-solid s-shelter\""), 1);
    }

    #[test]
    fn a_shelter_and_a_sign_are_whole_or_not_at_all_at_the_reach_edge() {
        // The stretch ends at 40 m. A stop from 36 m: its middle (the shelter's 4 m) would cross the edge.
        let mut e = editor(&CURB, ZoneEnd::First);
        e.add_stop(Side::Right, 36_000, 12_000, StopKind::Curb).unwrap();
        e.set_stop_shelter(0, true).unwrap();
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-solid s-shelter\""), 0, "the shelter's middle is at 42 m");
        assert_eq!(count(&m, "iso-stop-sign"), 1, "its near end is in the reach");
        // A stop from 30 m has its middle at 36 m: 34 m to 38 m, whole.
        let mut e = editor(&CURB, ZoneEnd::First);
        e.add_stop(Side::Right, 30_000, 12_000, StopKind::Curb).unwrap();
        e.set_stop_shelter(0, true).unwrap();
        assert_eq!(count(&drawn(&e), "class=\"iso-solid s-shelter\""), 1);
        // From the last end, 34 m to 46 m: its middle (40 m from the last end) is at the stretch's 0 m, so the
        // shelter would start before the reach, and the platform's near end (stretch coordinates) is at 0 m too.
        let mut e = editor(&CURB, ZoneEnd::Last);
        e.add_stop(Side::Right, 34_000, 12_000, StopKind::Curb).unwrap();
        e.set_stop_shelter(0, true).unwrap();
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-solid s-shelter\""), 0);
    }

    #[test]
    fn a_stop_has_a_sign_post() {
        let e = with_stop(&CURB, StopKind::Curb, false);
        let m = iso_svg(&e.view(), &en(), 1050.0).markup;
        assert_eq!(count(&m, "iso-stop-sign"), 1);
        let sign = stop_sign(&m);
        assert!(sign.contains("aria-hidden=\"true\"") && sign.contains("class=\"iso-solid s-post\"") && sign.contains("class=\"iso-sign-panel\""));
        assert!(sign.contains("class=\"iso-sign-glyph\""), "the bus pictogram");
        assert!(!sign.contains("<text") && !sign.contains("data-uid"));
        // Never under 32 px wide, even in a small drawing.
        let small = iso_svg(&e.view(), &en(), 280.0).markup;
        let sign = stop_sign(&small);
        let at = sign.find("class=\"iso-sign-panel\" points=\"").unwrap() + 31;
        let xs: Vec<f64> = sign[at..at + sign[at..].find('"').unwrap()].split(' ').map(|p| p.split(',').next().unwrap().parse().unwrap()).collect();
        let w = xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min);
        assert!(w >= 32.0 - 0.05, "{w}");
        assert!(!m.contains("NaN"));
    }

    #[test]
    fn a_stop_is_drawn_from_the_last_end_at_the_mirrored_place() {
        // 10 m to 22 m from the last end is 30 m to 18 m from the stretch's near end.
        let mut e = editor(&BULB, ZoneEnd::Last);
        e.add_stop(Side::Right, 10_000, 12_000, StopKind::Bulb).unwrap();
        let (v, p) = (e.view(), projection(&e));
        let piece = &v.segments[3];
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let (x0, x1, h) = (piece.x_mm as f64, (piece.x_mm + piece.width_mm) as f64, drawn_mm(150));
        let top = points(&p, &[(x0, h, 18_000.0), (x1, h, 18_000.0), (x1, h, 30_000.0), (x0, h, 30_000.0)]);
        assert!(m.contains(&format!("class=\"iso-top k-sidewalk mat-concrete\" points=\"{top}\"")), "{top}");
        assert_eq!(count(&m, "iso-stop-sign"), 1);
    }

    #[test]
    fn a_platform_covers_a_roadway_use_it_overlaps() {
        // A parking use from 5 m to 15 m, a bulb from 10 m: the lane shows its gap, the use up to 10 m, then nothing.
        let mut e = with_stop(&BULB, StopKind::Bulb, false);
        e.add_curb(Side::Right, 5_000, 10_000, crate::street::model::Use::Parking).unwrap();
        let (v, p) = (e.view(), projection(&e));
        let piece = &v.segments[3];
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let lane = m.split(&format!("<g class=\"iso-piece\" data-uid=\"{}\">", piece.uid)).nth(1).unwrap();
        let lane = &lane[..lane.find("</g></g>").or_else(|| lane.find("</g>")).unwrap()];
        assert_eq!(count(lane, "class=\"iso-top k-parking "), 3);
        let (x0, x1) = (piece.x_mm as f64, (piece.x_mm + piece.width_mm) as f64);
        let covered = points(&p, &[(x0, 0.0, 10_000.0), (x1, 0.0, 10_000.0), (x1, 0.0, 15_000.0), (x0, 0.0, 15_000.0)]);
        assert!(!lane.contains(&format!("points=\"{covered}\"")));
    }

    #[test]
    fn a_stop_with_nothing_drawn_is_not_counted() {
        // A curb stop from 39.7 m: shown by the model (0.3 m of it is in the stretch), but its sign would stand at 40.2 m.
        let mut e = editor(&CURB, ZoneEnd::First);
        e.add_stop(Side::Right, 39_700, 12_000, StopKind::Curb).unwrap();
        assert_eq!(count(&drawn(&e), "iso-stop-sign"), 0, "its near end is 40.2 m: no sign");
        assert!(!iso_label(&e.view(), &en(), Units::Metres).contains("transit"), "shown by the model, drawn by nothing");
        assert_eq!(e.view().stops_shown.len(), 1);
        // A platform counts though its sign is out.
        let mut e = editor(&BULB, ZoneEnd::First);
        e.add_stop(Side::Right, 39_700, 12_000, StopKind::Bulb).unwrap();
        assert!(iso_label(&e.view(), &en(), Units::Metres).ends_with("One transit stop is shown (curb extension)."));
    }

    #[test]
    fn a_stop_across_a_zone_boundary_is_drawn_in_the_zone_that_holds_it() {
        let mut e = editor(&BULB, ZoneEnd::First);
        e.add_stop(Side::Right, 10_000, 20_000, StopKind::Bulb).unwrap();
        e.set_stop_shelter(0, true).unwrap();
        e.split_zone(Some(20_000)).unwrap();
        // Zone 1 (0 to 20 m) holds the first half: the platform to its end; the sign stands, the shelter (middle at
        // 20 m, 18 m to 22 m) crosses the zone and is not drawn.
        assert!(e.select_zone(0));
        let v = e.view();
        assert_eq!(v.stops_shown.len(), 1);
        assert_eq!((v.stops_shown[0].along, v.stops_shown[0].full), ((10_000.0, 20_000.0), (10_000.0, 30_000.0)));
        let m = iso_svg(&v, &en(), 1050.0).markup;
        assert_eq!(count(&m, "iso-stop-sign"), 1);
        assert_eq!(count(&m, "class=\"iso-solid s-shelter\""), 0);
        assert!(!m.contains("NaN"));
        // Zone 2 holds the rest: the platform from 20 m, no sign (its near end is in the other zone).
        assert!(e.select_zone(1));
        let v = e.view();
        assert_eq!(v.stops_shown[0].along.0, 20_000.0);
        let m = iso_svg(&v, &en(), 1050.0).markup;
        assert_eq!(count(&m, "iso-stop-sign"), 0);
        assert!(m.contains("<title>Curb extension</title>"));
    }

    #[test]
    fn the_label_counts_what_is_drawn_at_the_reach_edge_at_any_width() {
        use crate::street::iso::iso_label_at;
        // A sign scaled up to its least size on a narrow drawing has a thicker post, which leaves the reach sooner.
        // The label counts the stops with a sign (a curb stop draws nothing else) exactly as the drawing shows them.
        let mut disagreements = 0;
        for width in [320.0, 480.0, 700.0, 1050.0, 1500.0] {
            for from_mm in (37_000..=39_900).step_by(100) {
                let mut e = editor(&CURB, ZoneEnd::First);
                e.add_stop(Side::Right, from_mm, 12_000, StopKind::Curb).unwrap();
                let v = e.view();
                let signs = count(&iso_svg(&v, &en(), width).markup, "iso-stop-sign");
                let label = iso_label_at(&v, &en(), Units::Metres, width);
                assert_eq!(label.contains("transit stop"), signs == 1, "{width} {from_mm}: {label}");
                disagreements += usize::from(signs == 0);
            }
        }
        assert!(disagreements > 0, "the sweep reaches the edge");
    }

    #[test]
    fn the_label_counts_the_stops() {
        let mut e = editor(&BULB, ZoneEnd::First);
        let none = iso_label(&e.view(), &en(), Units::Metres);
        assert!(!none.contains("transit"));
        e.add_stop(Side::Right, 10_000, 12_000, StopKind::Bulb).unwrap();
        assert!(iso_label(&e.view(), &en(), Units::Metres).ends_with(" One transit stop is shown (curb extension)."));
        e.add_stop(Side::Right, 24_000, 12_000, StopKind::Bulb).unwrap();
        assert!(iso_label(&e.view(), &en(), Units::Metres).ends_with(" 2 transit stops are shown (curb extension)."));
        let fr = crate::i18n_for(crate::shared::i18n::Locale::FrCa);
        assert!(iso_label(&e.view(), &fr, Units::Metres).ends_with(" 2 arrêts d’autobus sont montrés (avancée de trottoir)."));
        e.remove_stop(1).unwrap();
        assert!(iso_label(&e.view(), &fr, Units::Metres).ends_with(" Un arrêt d’autobus est montré (avancée de trottoir)."));
        // After the curbside sentence.
        e.add_curb(Side::Right, 2_000, 2_000, crate::street::model::Use::Patio).unwrap();
        let label = iso_label(&e.view(), &en(), Units::Metres);
        assert!(label.find("curbside use").unwrap() < label.find("transit stop").unwrap(), "{label}");
    }
}
