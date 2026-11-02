//! The street in 2.5D: a straight stretch of it seen in perspective from over its centre line, looking along it, each
//! piece an extruded strip with its height, the few things that stand on the pieces as plain solids, and the
//! lane markings on the roadway. It is drawn from the same view the section is and is read-only:
//! the section is where the street is edited.
//!
//! The rises are exaggerated: a curb or a sidewalk stands 150 mm, a few pixels at the scale a whole
//! street is drawn at, so the slabs, and the curbs with them, are drawn `RISE` times as high as they stand
//! (the height tables keep the true heights). What a curb stands above its own piece (a bike lane's divider)
//! is drawn true, so that it does not bury a narrow lane (`curb_top_mm`). The things that stand on the
//! pieces keep their own size and sit on the drawn tops.
//!
//! The code is split by what it draws: `projection` (where a point lands), `raised` (the strips standing
//! above the roadway), `order` (what is painted after what), `solids` (boxes on the pieces), `surface` (the roadway), `markings` (which
//! lines and symbols a street carries), `symbols` (their shapes) and `paint` (the rule for a symbol's room). `scene` builds each
//! layer as `shared::diagram` shapes, which `shared::diagram::render` writes through the projection (`iso_svg`).

use crate::shared::diagram::render::render;
use crate::shared::diagram::{Diagram, Layer, LayerKind, Pt, Section, Shape};
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::say;
use crate::shared::units::Units;
use crate::street::model::{SegView, ShownInstance, ShownStop, View};

mod curbside;
mod joints;
mod markings;
mod order;
mod paint;
mod projection;
mod raised;
mod scene;
mod sign;
mod solids;
mod stops;
mod surface;
mod symbols;
mod taper;
#[cfg(test)]
mod testing;

pub use projection::{ARROW_AT_MM, EYE_BACK_MM, EYE_HEIGHT_MM, MIN_SOLID_PX, Projection, RISE, Reach, STRETCH_MM, TALLEST_MM, drawn_mm, max_height};
pub use raised::{HEIGHTS, curb_height_mm, curb_top_mm, height_mm};

use raised::{materials, shades, strips};

/// The 2.5D drawn, and the size it is drawn at.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoSvg {
    pub markup: String,
    pub width: f64,
    pub height: f64,
}

/// How wide the scene is across: the street, or the pieces where they run over it, or the section across the boundary.
fn span_mm(v: &View) -> i32 {
    v.row_mm.max(v.total_mm).max(v.zones.joint.as_ref().map_or(0, |j| j.total_mm))
}

/// How tall the 2.5D of this street is drawn `width` wide, without drawing it.
pub fn iso_height(v: &View, width: f64) -> f64 {
    Projection::fit(width, span_mm(v)).height
}

/// One section drawn over its half of the stretch.
struct Run<'a> {
    segs: &'a [SegView],
    reach: Reach,
    /// It is the neighbour's section, across the boundary.
    across: bool,
    /// The transition the run holds, when it is the host's.
    taper: Option<taper::Draw>,
}

/// The street in 2.5D, drawn `width` wide: a stretch of `STRETCH_MM` seen in perspective from the centre line,
/// from the same view the section is drawn from. Hovering a piece names it.
///
/// It is built in layers, each its own group: the ground; then the section (`section`: its surface, its markings,
/// its raised layer and solids); and last the selected piece's outline. Across a boundary (`View.zones.joint`) there
/// are two sections, each over its half of the stretch (`STRETCH_MM / 2`, a `Reach`), the far one first so that the
/// near one covers it, each in a group of its own (`iso-section`, and `iso-across` for the neighbour's); the outline
/// is drawn over both, in the selected zone's half. The neighbour is drawn over its whole half whatever its own length.
pub fn iso_svg(v: &View, i18n: &I18n, width: f64) -> IsoSvg {
    let p = Projection::fit(width, span_mm(v));
    IsoSvg { markup: render(&diagram(v, i18n, width), &p), width: p.width, height: p.height }
}

/// One section's layers, bottom to top, over `run`'s half of the stretch.
fn section(base: &Projection, v: &View, i18n: &I18n, run: &Run, class: Option<&str>) -> Section {
    let p = base.over(run.reach);
    let segs = run.segs;
    let (flat, standing) = surface::split(strips(segs, &p));
    // The selected zone's run shows the curbside uses and stops; the neighbour's, across the boundary, none.
    let (uses, stops): (&[ShownInstance], &[ShownStop]) = if run.across { (&[], &[]) } else { (&v.curb_shown, &v.stops_shown) };
    let laid = curbside::overs(segs, uses, stops);
    let lifted = curbside::lifted(&laid);
    Section {
        class: class.map(str::to_string),
        layers: vec![
            scene::surface(&p, segs, i18n, &flat, run.across, run.taper.as_ref(), &laid),
            scene::markings(&p, segs, run.taper.as_ref()),
            scene::raised(&p, segs, i18n, &standing, &lifted, run.across, run.taper.as_ref()),
            scene::objects(&p, segs, uses, stops),
            scene::signs(v, segs, &p, i18n, run.across),
        ],
    }
}

/// The street in 2.5D as a diagram: the ground, the section or sections, and the selected piece's outline (see
/// `iso_svg` for how they lie).
fn diagram(v: &View, i18n: &I18n, width: f64) -> Diagram {
    let span = span_mm(v);
    let p = Projection::fit(width, span);
    let (w, l) = (span as f64, STRETCH_MM);
    let ground = Layer {
        kind: LayerKind::Ground,
        shapes: vec![Shape::Polygon {
            pts: [(0.0, 0.0), (w, 0.0), (w, l), (0.0, l)].iter().map(|&(x, z)| Pt::new(x, 0.0, z)).collect(),
            class: "iso-ground".into(),
            face: None,
            texture: None,
        }],
    };
    let mut selected = Reach::STRETCH;
    let sections = match &v.zones.joint {
        None => vec![section(&p, v, i18n, &Run { segs: &v.segments, reach: selected, across: false, taper: None }, None)],
        Some(j) => {
            let (near, far) = (Reach { near: 0.0, far: l / 2.0 }, Reach { near: l / 2.0, far: l });
            // The neighbour lies beyond the selected zone (`far`), or before it.
            let (own, across) = if j.far { (near, far) } else { (far, near) };
            selected = own;
            // The host's run holds the transition, its seam at the end that meets the other run.
            let own_run = Run {
                segs: &v.segments,
                reach: own,
                across: false,
                taper: j.taper.as_ref().filter(|t| t.host_is_selected).map(|t| taper::Draw::new(t, j.far)),
            };
            let across_run = Run {
                segs: &j.segments,
                reach: across,
                across: true,
                taper: j.taper.as_ref().filter(|t| !t.host_is_selected).map(|t| taper::Draw::new(t, !j.far)),
            };
            let runs = if j.far { [&across_run, &own_run] } else { [&own_run, &across_run] };
            runs.map(|run| section(&p, v, i18n, run, Some(if run.across { "iso-section iso-across" } else { "iso-section" }))).into()
        }
    };
    let outline = match v.selected.and_then(|u| v.segments.iter().position(|s| s.uid == u)) {
        Some(i) => scene::outline(&p.over(selected), &v.segments, i),
        None => Layer { kind: LayerKind::Outline, shapes: Vec::new() },
    };
    Diagram { defs: format!("<defs>{}{}</defs>", shades(), materials()), ground, sections, outline }
}

/// What a screen reader is told the 2.5D drawing is: the street, the zone shown when it has several, the stretch
/// shown, and its pieces; across a near boundary, the two zones that meet there.
pub fn iso_label(v: &View, i18n: &I18n, units: Units) -> String {
    iso_label_at(v, i18n, units, LABEL_WIDTH)
}

/// The width the label judges the stops by when the drawing's own is not known.
const LABEL_WIDTH: f64 = 1050.0;

/// `iso_label` for a drawing `width` wide: what the drawing shows of the stops depends on it, as a sign has a least
/// size on the screen.
pub fn iso_label_at(v: &View, i18n: &I18n, units: Units, width: f64) -> String {
    let mut label = iso_label_of(v, i18n, units);
    if !v.curb_shown.is_empty() {
        label = format!("{label} {}", i18n.tr("svg-iso-curb", &Args::new().num("count", v.curb_shown.len() as i64)));
    }
    let shown = stops::drawn(v, width);
    if !shown.is_empty() {
        let mut kinds: Vec<String> = Vec::new();
        for s in &shown {
            let name = i18n.tr(s.kind.name_key(), &Args::new()).to_lowercase();
            if !kinds.contains(&name) {
                kinds.push(name);
            }
        }
        label = format!("{label} {}", i18n.tr("svg-iso-stops", &Args::new().num("count", shown.len() as i64).str("kinds", kinds.join(", "))));
    }
    match v.speed_kmh {
        Some(kmh) => format!("{label} {}", i18n.tr("svg-iso-speed", &Args::new().num("kmh", i64::from(kmh)))),
        None => label,
    }
}

fn iso_label_of(v: &View, i18n: &I18n, units: Units) -> String {
    let args = Args::new()
        .str("name", say(i18n, units, &v.name))
        .str("length", units.length_whole_in(STRETCH_MM as i32, i18n.locale()))
        .num("count", v.segments.len() as i64);
    if v.zones.zones.len() < 2 {
        return i18n.tr("svg-iso-label", &args);
    }
    let zone = &v.zones.zones[v.zones.selected.min(v.zones.zones.len() - 1)];
    if let Some(joint) = &v.zones.joint {
        // The selected zone is the near run when the neighbour lies far; otherwise the neighbour is in front.
        let (near, far) = if joint.far { (&zone.label_mid, &joint.label_mid) } else { (&joint.label_mid, &zone.label_mid) };
        let args = args.str("zone", say(i18n, units, &zone.label)).str("near", say(i18n, units, near)).str("far", say(i18n, units, far));
        let mut label = i18n.tr("svg-iso-label-joint", &args);
        if let Some(t) = &joint.taper {
            // The transition leads from the host into the other zone.
            let other = if t.host_is_selected { &joint.label_mid } else { &zone.label_mid };
            let args = Args::new().str("length", units.length_whole_in(t.length_mm, i18n.locale())).str("zone", say(i18n, units, other));
            label = format!("{label} {}", i18n.tr("svg-iso-transition", &args));
        }
        return label;
    }
    let args = args
        .str("zone", say(i18n, units, &zone.label))
        .str("from", units.length_whole_in(zone.from_mm, i18n.locale()))
        .str("to", units.length_whole_in(zone.to_mm, i18n.locale()));
    i18n.tr("svg-iso-label-zone", &args)
}

#[cfg(test)]
mod tests {
    use super::projection::{BANDS, EYE_BACK_MM, Reach, band_edges, points};
    use super::projection::{PAD, r};
    use super::raised::inner_face;
    use super::solids::blocks;
    use super::testing::{count, drawn, en, furnished, joined, joined_from_last, marked, one_way_bikes, top_of, xs, xs_ys};
    use super::*;
    use crate::shared::catalogue::{KINDS, MATERIALS, kind_index};
    use crate::shared::diagram::render::layer_markup;
    use crate::shared::i18n::Locale;
    use crate::street::model::Editor;

    /// How far a face rises on the drawing, in pixels: the first polygon of `class` in the `nth` group drawn for `uid`.
    fn rise_of(markup: &str, uid: u32, nth: usize, class: &str) -> f64 {
        let group = markup.split(&format!("<g class=\"iso-piece\" data-uid=\"{uid}\">")).nth(nth).expect("the group is drawn");
        let face = &group[group.find(&format!("class=\"{class}")).expect("the face is drawn")..];
        let at = face.find("points=\"").unwrap() + 8;
        let ys: Vec<f64> = face[at..at + face[at..].find('"').unwrap()].split(' ').map(|pt| pt.split(',').nth(1).unwrap().parse().unwrap()).collect();
        ys.iter().cloned().fold(f64::MIN, f64::max) - ys.iter().cloned().fold(f64::MAX, f64::min)
    }

    #[test]
    fn a_rise_is_drawn_three_times_as_high_as_it_stands_so_that_it_reads() {
        assert_eq!(drawn_mm(150), 450.0);
        assert_eq!(drawn_mm(0), 0.0);
        // The tables keep the true heights; only the drawing exaggerates them.
        assert_eq!((height_mm("sidewalk"), curb_height_mm("concrete")), (150, 150));
        let mut e = Editor::new(0);
        let bike = e.add(kind_index("bike").unwrap(), 2);
        let v = e.view();
        let scale = Projection::fit(1050.0, v.row_mm.max(v.total_mm)).scale;
        let m = iso_svg(&v, &en(), 1050.0).markup;
        // The sidewalk's front face is exaggerated; the front face of the bike lane's first curb block (its slab is the
        // first group), a divider standing above a lane at the roadway's level, is drawn true (`curb_top_mm`).
        let sidewalk = v.segments[0].uid;
        assert!((rise_of(&m, sidewalk, 1, "iso-front ") - 450.0 * scale).abs() < 0.02, "{}", rise_of(&m, sidewalk, 1, "iso-front "));
        assert!((rise_of(&m, bike, 2, "iso-curb") - 150.0 * scale).abs() < 0.02, "{}", rise_of(&m, bike, 2, "iso-curb"));
    }

    #[test]
    fn a_known_street_has_one_top_face_per_piece_each_in_its_tint_and_hatch() {
        let e = Editor::new(0);
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-top "), 6);
        assert_eq!(count(&m, "class=\"iso-piece\""), 6 + 2, "a group per piece and per curb");
        assert_eq!(count(&m, "class=\"iso-top k-sidewalk mat-"), 2);
        assert_eq!(count(&m, "fill=\"url(#iso-m-asphalt)\""), 4, "a texture group per travel lane and parking lane, in the material's pattern");
        assert_eq!(count(&m, "<polygon transform=\"matrix("), 6 * BANDS, "BANDS bands on each of the six tops");
        assert_eq!(count(&m, "fill=\"url(#h-travel)\""), 0, "the kind's hatch stays on the faces");
        assert_eq!(count(&m, "class=\"iso-ground\""), 1);
        assert!(!m.contains("NaN"));
    }

    /// The class attribute of the top face of the piece `uid`.
    fn top_class(markup: &str, uid: u32) -> String {
        let group = markup.split(&format!("<g class=\"iso-piece\" data-uid=\"{uid}\">")).nth(1).expect("the piece is drawn");
        let at = group.find("class=\"iso-top ").expect("a top face") + 7;
        group[at..at + group[at..].find('"').unwrap()].to_string()
    }

    #[test]
    fn every_top_is_in_its_pieces_material_with_the_material_s_pattern_over_it() {
        let e = Editor::new(0);
        let v = e.view();
        let m = drawn(&e);
        for s in &v.segments {
            let class = top_class(&m, s.uid);
            assert!(class.contains(&format!("k-{} mat-{}", KINDS[s.kind].id, s.material)), "{class}");
        }
        // The pattern is the material's, and the kind's hatch is no longer on a top.
        let sidewalk = v.segments[0].uid;
        let group = m.split(&format!("<g class=\"iso-piece\" data-uid=\"{sidewalk}\">")).nth(1).unwrap();
        let top = &group[group.find("class=\"iso-top ").unwrap()..];
        assert!(top.contains(&format!("fill=\"url(#iso-m-{})\"", v.segments[0].material)));
        assert!(!top.split("</g>").next().unwrap().contains("url(#h-sidewalk)") || top.contains("iso-front"), "only the faces keep the kind's hatch");
    }

    #[test]
    fn a_piece_whose_material_changes_changes_its_top_and_nothing_else_of_its_class() {
        let mut e = Editor::new(0);
        let uid = e.view().segments[0].uid;
        let kind = e.view().segments[0].kind;
        let before = drawn(&e);
        let other =
            KINDS[kind].materials.iter().copied().find(|&i| MATERIALS[i].id != e.view().segments[0].material).expect("a sidewalk has a second material");
        assert!(e.set_material(uid, other));
        let after = drawn(&e);
        assert_ne!(top_class(&before, uid), top_class(&after, uid));
        assert!(top_class(&after, uid).ends_with(&format!("mat-{}", MATERIALS[other].id)));
        assert!(after.contains(&format!("fill=\"url(#iso-m-{})\"", MATERIALS[other].id)));
        // the faces keep the kind's tint
        assert_eq!(count(&before, "class=\"iso-front k-sidewalk\""), count(&after, "class=\"iso-front k-sidewalk\""));
    }

    /// The bands of the top of the piece `uid` (its `iso-m-*` polygons): each as its `matrix(..)` and its points.
    fn bands(markup: &str, uid: u32) -> Vec<([f64; 6], Vec<(f64, f64)>)> {
        let group = markup.split(&format!("<g class=\"iso-piece\" data-uid=\"{uid}\">")).nth(1).expect("the piece is drawn");
        // The first group closed is the top's texture group.
        let group = &group[..group.find("</g>").unwrap()];
        group
            .split("<polygon transform=\"matrix(")
            .skip(1)
            .map(|b| {
                let m: Vec<f64> = b[..b.find(')').unwrap()].split(' ').map(|n| n.parse().unwrap()).collect();
                let pts = &b[b.find("points=\"").unwrap() + 8..];
                let pts = pts[..pts.find('"').unwrap()]
                    .split(' ')
                    .map(|pt| {
                        let mut c = pt.split(',').map(|n| n.parse::<f64>().unwrap());
                        (c.next().unwrap(), c.next().unwrap())
                    })
                    .collect();
                ([m[0], m[1], m[2], m[3], m[4], m[5]], pts)
            })
            .collect()
    }

    #[test]
    fn a_material_with_joints_draws_them_as_geometry_over_its_fine_texture_and_a_fine_one_draws_none() {
        // Sample Street 1: two concrete sidewalks (joints) and four asphalt lanes (grain only).
        let e = Editor::new(0);
        let v = e.view();
        let m = drawn(&e);
        assert_eq!(count(&m, "<path class=\"joints j-concrete\" d=\"M"), 2);
        assert_eq!(count(&m, "class=\"joints"), 2);
        // The joints come after the top's bands, inside the piece's group, and the bands are still there for the grain.
        for s in v.segments.iter().filter(|s| s.material == "concrete") {
            let group = m.split(&format!("<g class=\"iso-piece\" data-uid=\"{}\">", s.uid)).nth(1).unwrap();
            let group = &group[..group.find("<g class=\"iso-piece\"").unwrap_or(group.len())];
            assert!(group.find("class=\"hatch\"").unwrap() < group.find("class=\"joints").unwrap());
        }
        // The material is the piece's own: a brick sidewalk draws brick joints and no concrete ones.
        let mut e = Editor::new(0);
        let uid = e.view().segments[0].uid;
        let brick = KINDS[e.view().segments[0].kind].materials.iter().copied().find(|&i| MATERIALS[i].id == "brick");
        if let Some(i) = brick {
            assert!(e.set_material(uid, i));
            let after = drawn(&e);
            assert_eq!(count(&after, "class=\"joints j-brick\""), 1);
            assert_eq!(count(&after, "class=\"joints j-concrete\""), 1);
        }
    }

    #[test]
    fn a_joint_is_drawn_in_its_material_s_own_tone_and_the_width_the_material_says() {
        let css = include_str!("../../../web/editor.css");
        for m in MATERIALS.iter().filter(|m| crate::shared::iso::ground::joints(m.id).is_some()) {
            let j = crate::shared::iso::ground::joints(m.id).unwrap();
            let rule = &css[css.find(&format!(".joints.j-{} {{", m.id)).unwrap_or_else(|| panic!("no rule for {}", m.id))..];
            let rule = &rule[..rule.find('}').unwrap()];
            let tone = if j.alt { "alt" } else { "line" };
            assert!(rule.contains(&format!("stroke: var(--mat-{}-{tone});", m.id)), "{}: {rule}", m.id);
            assert!(rule.contains(&format!("stroke-width: {};", j.width_px)), "{}: {rule}", m.id);
        }
    }

    #[test]
    fn the_patterns_are_drawn_in_millimetres_of_ground_and_each_band_of_a_top_lays_one_by_its_own_matrix() {
        let e = Editor::new(0);
        let v = e.view();
        let p = Projection::fit(1050.0, span_mm(&v));
        let m = drawn(&e);
        // The patterns carry no projection: they are in ground millimetres, the identity.
        for mat in MATERIALS {
            let at = m.find(&format!("id=\"iso-m-{}\"", mat.id)).expect("every material has a pattern");
            let start = m[..at].rfind("<pattern").unwrap();
            let tag = &m[start..start + m[start..].find('>').unwrap()];
            assert!(tag.contains("patternTransform=\"matrix(1.000000 0.000000 0.000000 1.000000 0.00 0.00)\""), "{tag}");
        }
        // Each top is cut into BANDS bands, whose polygons, laid by their matrices, are exactly the perspective's
        // quadrilaterals of the band (so they fit the top and each other), and whose matrices are the slope at the band's depth.
        let edges = band_edges(Reach::STRETCH);
        for s in &v.segments {
            let h = drawn_mm(height_mm(KINDS[s.kind].id));
            let bands = bands(&m, s.uid);
            assert_eq!(bands.len(), BANDS, "{}", KINDS[s.kind].id);
            let (x0, x1) = (s.x_mm as f64, (s.x_mm + s.width_mm) as f64);
            for (k, (t, local)) in bands.iter().enumerate() {
                let want = [(x0, edges[k]), (x1, edges[k]), (x1, edges[k + 1]), (x0, edges[k + 1])];
                for ((u, vv), (x, z)) in local.iter().zip(want) {
                    let got = (t[0] * u + t[2] * vv + t[4], t[1] * u + t[3] * vv + t[5]);
                    let (sx, sy) = p.at(x, h, z);
                    assert!((got.0 - sx).abs() < 0.1 && (got.1 - sy).abs() < 0.1, "{k}: {got:?} {sx} {sy}");
                }
                // The size across is the projection's scale at the band's middle in depth, which lies in the band.
                let depth = ((t[0] / p.scale).recip() * EYE_BACK_MM) - EYE_BACK_MM;
                assert!(depth > edges[k] && depth < edges[k + 1], "{k}: {depth}");
                assert_eq!((t[1], t[3] < 0.0), (0.0, true), "across stays horizontal, along goes up the screen");
            }
        }
    }

    #[test]
    fn a_raised_piece_has_a_front_face_and_a_side_face_where_its_neighbour_is_lower_and_the_roadway_has_neither() {
        let m = drawn(&Editor::new(0));
        assert_eq!(count(&m, "class=\"iso-front "), 2);
        assert_eq!(count(&m, "class=\"iso-front k-sidewalk\""), 2);
        assert_eq!(count(&m, "class=\"iso-side "), 2);
        assert_eq!(count(&m, "class=\"iso-side k-sidewalk\""), 2);
        for road in ["travel", "parking"] {
            assert_eq!(count(&m, &format!("iso-front k-{road}")) + count(&m, &format!("iso-side k-{road}")), 0, "{road}");
        }
        // Sample Avenue 2: a sidewalk beside a planting strip of the same height has no side face; each planting strip,
        // beside parking, shows its inner one; the median is over the eye, so it shows none.
        let avenue = drawn(&Editor::new(1));
        assert_eq!(count(&avenue, "class=\"iso-front "), 5);
        assert_eq!(count(&avenue, "class=\"iso-side "), 2);
    }

    /// The `points` of the first face of `class` in the first group drawn for `uid` that has one.
    fn face_points(markup: &str, uid: u32, class: &str) -> Option<String> {
        markup.split(&format!("<g class=\"iso-piece\" data-uid=\"{uid}\">")).skip(1).find_map(|g| {
            let g = &g[..g.find("</g>")?];
            let face = &g[g.find(&format!("class=\"{class}"))?..];
            let at = face.find("points=\"")? + 8;
            Some(face[at..at + face[at..].find('"')?].to_string())
        })
    }

    #[test]
    fn from_the_middle_the_left_sidewalk_shows_its_right_face_and_the_right_one_its_left_and_both_their_front() {
        let e = Editor::new(0);
        let v = e.view();
        let p = Projection::fit(1050.0, span_mm(&v));
        let m = drawn(&e);
        let (left, right) = (&v.segments[0], &v.segments[5]);
        let h = drawn_mm(150);
        let (lx, rx) = ((left.x_mm + left.width_mm) as f64, right.x_mm as f64);
        assert_eq!(face_points(&m, left.uid, "iso-side").unwrap(), points(&p, &[(lx, 0.0, 0.0), (lx, 0.0, STRETCH_MM), (lx, h, STRETCH_MM), (lx, h, 0.0)]));
        assert_eq!(face_points(&m, right.uid, "iso-side").unwrap(), points(&p, &[(rx, 0.0, 0.0), (rx, 0.0, STRETCH_MM), (rx, h, STRETCH_MM), (rx, h, 0.0)]));
        // The near face shows for both.
        for s in [left, right] {
            let (x0, x1) = (s.x_mm as f64, (s.x_mm + s.width_mm) as f64);
            assert_eq!(face_points(&m, s.uid, "iso-front").unwrap(), points(&p, &[(x0, 0.0, 0.0), (x1, 0.0, 0.0), (x1, h, 0.0), (x0, h, 0.0)]));
        }
        // The sidewalks' inner faces are seen from the middle of the street, so the left one is a mirror of the right.
        let (l, r) = (xs_ys(&face_points(&m, left.uid, "iso-side").unwrap()), xs_ys(&face_points(&m, right.uid, "iso-side").unwrap()));
        for ((lx, ly), (rx, ry)) in l.iter().zip(&r) {
            assert!((lx + rx - p.width).abs() < 0.02 && (ly - ry).abs() < 0.02, "{l:?} {r:?}");
        }
    }

    #[test]
    fn a_piece_right_of_the_middle_is_outlined_along_its_left_face() {
        let mut e = Editor::new(0);
        let (uid, x0) = (e.view().segments[5].uid, e.view().segments[5].x_mm);
        e.select(Some(uid));
        let m = drawn(&e);
        let v = e.view();
        let p = Projection::fit(1050.0, span_mm(&v));
        let (x1, h) = ((x0 + v.segments[5].width_mm) as f64, drawn_mm(150));
        let (sx, sy) = p.at(x1, 0.0, 0.0);
        assert!(m.contains(&format!("<polygon class=\"iso-sel\" points=\"{},{} ", r(sx), r(sy))), "starts at the near right corner");
        let (fx, fy) = p.at(x0 as f64, h, STRETCH_MM);
        assert!(m.contains(&format!("{},{} ", r(fx), r(fy))), "goes up the left face's far edge");
    }

    #[test]
    fn a_buffer_is_drawn_at_the_roadway_s_level_with_no_face_no_curb_and_no_symbol() {
        let mut e = Editor::new(0);
        e.add(kind_index("buffer").unwrap(), 3);
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-top k-buffer mat-"), 1);
        assert_eq!(count(&m, "iso-front k-buffer") + count(&m, "iso-side k-buffer"), 0, "it stands no higher than the roadway");
        assert_eq!(count(&m, "class=\"iso-curb\""), 2, "only the two sidewalks' curbs");
        assert!(!m.contains("NaN"));
    }

    #[test]
    fn the_roadway_is_drawn_before_the_raised_pieces_so_they_stand_in_front_of_it() {
        let m = drawn(&Editor::new(0));
        assert!(m.rfind("iso-top k-travel").unwrap() < m.find("iso-top k-sidewalk").unwrap());
        assert!(m.find("class=\"iso-ground\"").unwrap() < m.find("class=\"iso-top ").unwrap());
    }

    #[test]
    fn a_sidewalk_s_curb_is_a_stripe_on_its_edge_and_a_bike_lane_s_curb_is_a_block_standing_above_it() {
        let mut e = Editor::new(0);
        let m = drawn(&e);
        // The two sidewalks' concrete curbs are as high as the sidewalk: a top stripe each.
        assert_eq!(count(&m, "class=\"iso-curb\""), 2);
        assert_eq!(count(&m, "fill=\"url(#c-concrete)\""), 2);
        // A bike lane (its curb is concrete by default) between parking and a driving lane: a block on each edge,
        // each with a front, a side and a top.
        e.add(kind_index("bike").unwrap(), 2);
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-curb\""), 2 + 2 * 3);
        assert!(!m.contains("NaN"));
    }

    #[test]
    fn the_height_of_the_2_5d_is_known_without_drawing_it() {
        let mut e = Editor::new(0);
        e.add(kind_index("travel").unwrap(), 2);
        for e in [Editor::new(0), Editor::new(1), e] {
            for width in [400.0, 600.0, 760.0, 1050.0, 1440.0] {
                assert_eq!(iso_height(&e.view(), width), iso_svg(&e.view(), &en(), width).height, "{width}");
            }
        }
    }

    #[test]
    fn a_curb_s_faces_say_which_face_each_is_so_that_the_turned_ones_are_shaded() {
        let mut e = Editor::new(0);
        e.add(kind_index("bike").unwrap(), 2);
        let m = drawn(&e);
        // The sidewalks' two stripes are tops; the bike lane's two blocks have a top, a front and a side each.
        assert_eq!(count(&m, "class=\"iso-curb\" data-face=\"top\""), 2 + 2);
        assert_eq!(count(&m, "class=\"iso-curb\" data-face=\"front\""), 2);
        assert_eq!(count(&m, "class=\"iso-curb\" data-face=\"side\""), 2);
    }

    #[test]
    fn every_curb_a_piece_can_have_draws_in_2_5d() {
        let mut e = Editor::new(0);
        let u = e.add(kind_index("bike").unwrap(), 2);
        for c in 0..crate::shared::catalogue::CURBS.len() {
            if e.set_curb(u, Some(c)) {
                assert!(!drawn(&e).contains("NaN"), "curb {c}");
            }
        }
    }

    #[test]
    fn the_2_5d_follows_an_edit_a_wider_piece_moves_the_next_one() {
        let mut e = Editor::new(0);
        let (first, second) = (e.view().segments[0].uid, e.view().segments[1].uid);
        let before = top_of(&drawn(&e), second);
        assert!(e.nudge_width(first, -500));
        assert_ne!(top_of(&drawn(&e), second), before);
    }

    #[test]
    fn two_different_streets_give_different_drawings() {
        // Each drawing has a top face for each of its own pieces, in its kind's tint, so two streets made of
        // other pieces draw differently, and not only in some number.
        let tops = |e: &Editor| {
            let m = drawn(e);
            let mut kinds: Vec<String> =
                m.split("class=\"iso-top k-").skip(1).map(|rest| rest.split('"').next().unwrap().split(' ').next().unwrap().to_string()).collect();
            kinds.sort();
            kinds
        };
        let pieces = |e: &Editor| {
            let mut kinds: Vec<String> = e.view().segments.iter().map(|s| KINDS[s.kind].id.to_string()).collect();
            kinds.sort();
            kinds
        };
        let (a, b) = (Editor::new(0), Editor::new(1));
        assert_eq!(tops(&a), pieces(&a));
        assert_eq!(tops(&b), pieces(&b));
        assert_ne!(tops(&a), tops(&b));
        assert_ne!(drawn(&a), drawn(&b));
    }

    #[test]
    fn a_design_wider_than_the_street_is_drawn_whole_inside_the_drawing() {
        let mut e = Editor::new(0);
        let u = e.view().segments[0].uid;
        assert!(e.set_width(u, 8_000));
        assert!(e.view().delta_mm > 0, "the design runs over the street");
        let s = iso_svg(&e.view(), &en(), 1050.0);
        let all = xs(&s.markup);
        assert!(all.iter().all(|x| *x >= 0.0 && *x <= s.width), "{:?}", all.iter().cloned().fold(f64::MIN, f64::max));
        assert_eq!(count(&s.markup, "class=\"iso-top "), 6);
    }

    #[test]
    fn a_street_with_no_pieces_is_the_ground_alone() {
        let mut e = Editor::new(0);
        while let Some(u) = e.view().segments.first().map(|s| s.uid) {
            assert!(e.remove(u));
        }
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-ground\""), 1);
        assert_eq!(count(&m, "class=\"iso-top "), 0);
        assert!(!m.contains("NaN"));
    }

    #[test]
    fn the_names_a_piece_shows_on_hover_are_in_the_language_of_the_page() {
        let e = Editor::new(0);
        let i18n = en();
        assert!(iso_svg(&e.view(), &i18n, 1050.0).markup.contains("<title>Sidewalk</title>"));
        i18n.set(Locale::FrCa);
        let fr = iso_svg(&e.view(), &i18n, 1050.0).markup;
        assert!(fr.contains("<title>Trottoir</title>") && !fr.contains(">Sidewalk<"), "the French drawing names the sidewalk in French");
    }

    #[test]
    fn every_sample_street_draws_in_2_5d_without_a_missing_number_at_every_width() {
        for sample in 0..4 {
            for width in [300.0, 358.0, 600.0, 760.0, 1050.0, 1800.0] {
                let s = iso_svg(&Editor::new(sample).view(), &en(), width);
                assert!(!s.markup.contains("NaN") && !s.markup.contains("inf"), "sample {sample} at {width}");
            }
        }
    }

    #[test]
    fn the_2_5d_is_never_taller_than_the_height_the_group_keeps_at_any_width_so_it_never_makes_the_card_taller() {
        // The group keeps `max_height` of its width, and the status line under it has to stay in view at 1440 by 900,
        // where the group is about 680 px wide: the 2.5D is held to that height at every width.
        let mut e = Editor::new(0);
        e.add(kind_index("travel").unwrap(), 2);
        for e in [Editor::new(0), Editor::new(1), Editor::new(2), Editor::new(3), e, furnished()] {
            let v = e.view();
            for width in [280.0, 358.0, 600.0, 684.0, 760.0, 945.0, 1050.0, 1155.0, 1440.0, 1800.0] {
                let kept = projection::max_height(width);
                let iso = iso_height(&v, width);
                assert!(iso <= kept + 1e-9, "{width}: the 2.5D is {iso} px tall, the group keeps {kept}");
            }
        }
        // The height the section had at the widths the card is laid out for.
        assert_eq!(projection::max_height(1050.0), 484.0);
        assert_eq!(projection::max_height(358.0), (484.0_f64 * 0.9).round());
        assert_eq!(projection::max_height(3000.0), (484.0_f64 * 1.1).round());
    }

    #[test]
    fn on_a_narrow_screen_the_2_5d_is_laid_out_as_wide_as_it_has_room_for() {
        // The 2.5D is read-only and has no handles: it needs no sideways scroll, and a 390 px phone's group (358 px)
        // gets a drawing 358 px wide, the whole street in view, scaled down.
        for sample in 0..4 {
            let s = iso_svg(&Editor::new(sample).view(), &en(), 358.0);
            assert_eq!(s.width, 358.0, "sample {sample}");
        }
        let v = Editor::new(0).view();
        let (narrow, wide) = (Projection::fit(358.0, v.row_mm), Projection::fit(1050.0, v.row_mm));
        assert!(narrow.scale < wide.scale);
        // The street's right edge is inside the drawing.
        assert!(narrow.at(v.row_mm as f64, 0.0, 0.0).0 <= 358.0 - 30.0 + 1e-9);
    }

    #[test]
    fn the_farthest_crown_stays_inside_the_drawing_at_every_width() {
        for width in [600.0, 760.0, 1050.0, 1800.0] {
            let s = iso_svg(&Editor::new(1).view(), &en(), width);
            let top = xs_ys(&s.markup).into_iter().map(|(_, y)| y).fold(f64::MAX, f64::min);
            assert!(top >= PAD - 1e-9, "{width}: {top}");
        }
    }

    #[test]
    fn the_selected_piece_is_outlined_and_no_other() {
        let mut e = Editor::new(0);
        assert_eq!(count(&drawn(&e), "class=\"iso-sel\""), 0);
        let (uid, x_mm) = (e.view().segments[2].uid, e.view().segments[2].x_mm);
        e.select(Some(uid));
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-sel\""), 1);
        // The outline starts at the selected piece's near left corner on the ground.
        let p = Projection::fit(1050.0, 18_000);
        let (x, y) = p.at(x_mm as f64, 0.0, 0.0);
        assert!(m.contains(&format!("<polygon class=\"iso-sel\" points=\"{},{} ", r(x), r(y))), "the drawing has not got it");
        // Drawn last, over everything.
        assert!(m.rfind("class=\"iso-sel\"").unwrap() > m.rfind("class=\"iso-piece\"").unwrap());
    }

    #[test]
    fn every_outline_has_a_casing_under_it_with_the_same_points() {
        // Left of the eye, right of it, and the one piece across it: the three shapes of the outline.
        let check = |e: &Editor| {
            let m = drawn(e);
            let at = m.find("<polygon class=\"iso-sel-casing\" points=\"").expect("a casing");
            let pts = &m[at + 40..];
            let pts = &pts[..pts.find('"').unwrap()];
            assert!(m.contains(&format!("<polygon class=\"iso-sel-casing\" points=\"{pts}\"/><polygon class=\"iso-sel\" points=\"{pts}\"/>")));
            assert_eq!(count(&m, "iso-sel-casing"), 1);
        };
        let n = Editor::new(0).view().segments.len();
        for i in 0..n {
            let mut e = Editor::new(0);
            e.select(Some(e.view().segments[i].uid));
            check(&e);
        }
        // A piece across the middle: the one a street of the default's pieces has when the ones left of it are gone.
        let mut e = Editor::new(0);
        let p = Projection::fit(1050.0, span_mm(&e.view()));
        let across = |e: &Editor| e.view().segments.iter().find(|s| inner_face(&p, s.x_mm as f64, (s.x_mm + s.width_mm) as f64).is_none()).map(|s| s.uid);
        while across(&e).is_none() && e.view().segments.len() > 1 {
            assert!(e.remove(e.view().segments[0].uid));
        }
        e.select(Some(across(&e).expect("some piece lies across the middle")));
        check(&e);
    }

    #[test]
    fn a_selection_of_no_piece_draws_no_outline_and_a_one_piece_street_draws_whole() {
        let mut v = Editor::new(0).view();
        v.selected = Some(9_999);
        assert_eq!(count(&iso_svg(&v, &en(), 1050.0).markup, "class=\"iso-sel\""), 0);
        let mut e = Editor::new(0);
        while e.view().segments.len() > 1 {
            assert!(e.remove(e.view().segments[0].uid));
        }
        e.select(Some(e.view().segments[0].uid));
        let m = drawn(&e);
        assert!(!m.contains("NaN") && !m.contains("inf") && count(&m, "class=\"iso-sel\"") == 1, "the drawing has not got it");
    }

    #[test]
    fn the_outline_of_a_selected_bike_lane_takes_in_its_curb_blocks() {
        // A bike lane's slab and its two curb blocks are three groups with one uid: the outline goes round all of them.
        let mut e = Editor::new(0);
        let u = e.add(kind_index("bike").unwrap(), 2);
        e.select(Some(u));
        let m = drawn(&e);
        let groups: Vec<&str> = m.split(&format!("<g class=\"iso-piece\" data-uid=\"{u}\">")).skip(1).map(|g| &g[..g.find("</g>").unwrap()]).collect();
        assert_eq!(groups.len(), 3);
        let sel = &m[m.find("class=\"iso-sel\"").unwrap()..];
        let sel = xs_ys(&sel[..sel.find("/>").unwrap()]);
        let (lo_x, hi_x) = sel.iter().fold((f64::MAX, f64::MIN), |(a, b), (x, _)| (a.min(*x), b.max(*x)));
        let (lo_y, hi_y) = sel.iter().fold((f64::MAX, f64::MIN), |(a, b), (_, y)| (a.min(*y), b.max(*y)));
        for g in groups {
            for (x, y) in xs_ys(g) {
                assert!(x >= lo_x - 1e-9 && x <= hi_x + 1e-9 && y >= lo_y - 1e-9 && y <= hi_y + 1e-9, "({x},{y}) outside {sel:?}");
            }
        }
    }

    #[test]
    fn the_faces_turned_from_the_eye_are_shaded_by_the_drawing_s_own_filters_not_by_css() {
        // CSS `filter: brightness()` on SVG shapes may not be applied in Safari; an SVG filter is SVG 1.1.
        let m = drawn(&furnished());
        for (id, slope) in [("iso-shade-front", "0.9"), ("iso-shade-side", "0.78")] {
            let at = m.find(&format!("<filter id=\"{id}\"")).unwrap_or_else(|| panic!("the drawing has no {id} filter"));
            let f = &m[at..at + m[at..].find("</filter>").unwrap()];
            assert!(f.contains("color-interpolation-filters=\"sRGB\""), "the {id} filter");
            assert_eq!(count(f, &format!("type=\"linear\" slope=\"{slope}\"")), 3, "the {id} filter");
        }
        assert!(m.find("<defs>").unwrap() < m.find("class=\"iso-ground\"").unwrap());
        let front = count(&m, "class=\"iso-front ") + count(&m, "data-face=\"front\"") + count(&m, "class=\"o-front\"");
        let side = count(&m, "class=\"iso-side ") + count(&m, "data-face=\"side\"") + count(&m, "class=\"o-side\"");
        assert!(front > 0 && side > 0);
        assert_eq!(count(&m, "filter=\"url(#iso-shade-front)\""), front);
        assert_eq!(count(&m, "filter=\"url(#iso-shade-side)\""), side);
        let css = include_str!("../../../web/editor.css");
        assert!(!css.contains("brightness(0.9)") && !css.contains("brightness(0.78)"));
    }

    #[test]
    fn the_2_5d_is_labelled_with_the_street_the_stretch_and_its_pieces_in_each_language() {
        let v = Editor::new(0).view();
        assert_eq!(iso_label(&v, &en(), Units::Metres), "Sample Street 1 in 2.5D, along a 40 m stretch. 6 segments.");
        assert_eq!(iso_label(&v, &en(), Units::Feet), "Sample Street 1 in 2.5D, along a 131 ft stretch. 6 segments.");
        let fr = crate::i18n_for(Locale::FrCa);
        assert_eq!(iso_label(&v, &fr, Units::Metres), "Sample Street 1 en 2,5D, sur un tronçon de 40\u{a0}m. 6 éléments.");
    }

    /// The sample street divided at 20 m and 40 m of 60 m, with the frame the zone operations need.
    fn three_zones() -> Editor {
        use crate::shared::catalogue::Side;
        use crate::street::model::{Street, ZoneEnd, ZoneFrame, zoned};
        let today = Street::sample(0, Side::Right);
        let now = zoned(today.clone(), &[0, 20_000, 40_000], 60_000);
        Editor::from_street(&today, &now, 0).with_frame(Some(ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::First }))
    }

    /// The sample street divided at 100 m of 160 m: no boundary lies near the 40 m shown of zone 1.
    fn zones_far_apart() -> Editor {
        use crate::shared::catalogue::Side;
        use crate::street::model::{Street, ZoneEnd, ZoneFrame, zoned};
        let today = Street::sample(0, Side::Right);
        let now = zoned(today.clone(), &[0, 100_000], 160_000);
        Editor::from_street(&today, &now, 0).with_frame(Some(ZoneFrame { length_mm: 160_000, from_end: ZoneEnd::First }))
    }

    #[test]
    fn a_street_of_one_zone_has_the_same_2_5d_label_as_before() {
        let v = Editor::new(0).view();
        assert_eq!(iso_label(&v, &en(), Units::Metres), "Sample Street 1 in 2.5D, along a 40 m stretch. 6 segments.");
        let fr = crate::i18n_for(Locale::FrCa);
        assert_eq!(iso_label(&v, &fr, Units::Metres), "Sample Street 1 en 2,5D, sur un tronçon de 40\u{a0}m. 6 éléments.");
    }

    #[test]
    fn the_2_5d_label_names_the_selected_zone_and_its_range_in_both_languages() {
        let mut e = zones_far_apart();
        assert!(e.select_zone(0));
        let v = e.view();
        assert!(v.zones.joint.is_none());
        assert_eq!(iso_label(&v, &en(), Units::Metres), "Sample Street 1 in 2.5D, Zone 1, 0 m to 100 m, along a 40 m stretch. 6 segments.");
        let fr = crate::i18n_for(Locale::FrCa);
        assert_eq!(iso_label(&v, &fr, Units::Metres), "Sample Street 1 en 2,5D, Zone 1, de 0\u{a0}m à 100\u{a0}m, sur un tronçon de 40\u{a0}m. 6 éléments.");
    }

    #[test]
    fn the_label_names_the_two_zones_where_they_meet_in_both_languages() {
        let v = joined(true, 1);
        assert_eq!(
            iso_label(&v, &en(), Units::Metres),
            "Sample Street 1 in 2.5D, along a 40 m stretch centred on the boundary between Zone 1 in front and Zone 2 beyond. The section changes abruptly there. Zone 2 has 5 segments. A transition of 53 m leads into Zone 1."
        );
        let fr = crate::i18n_for(Locale::FrCa);
        assert_eq!(
            iso_label(&v, &fr, Units::Metres),
            "Sample Street 1 en 2,5D, sur un tronçon de 40\u{a0}m centré sur la limite entre zone 1 au premier plan et zone 2 au-delà. La coupe change brusquement à cet endroit. Zone 2 compte 5 éléments. Une transition de 53\u{a0}m mène à zone 1."
        );
    }

    #[test]
    fn the_zone_in_front_is_the_one_nearer_the_eye() {
        let v = joined_from_last(true, 1);
        assert!(v.zones.joint.as_ref().expect("a joint").far, "the neighbour of the last zone lies beyond");
        let label = iso_label(&v, &en(), Units::Metres);
        assert!(label.contains("between Zone 2 in front and Zone 1 beyond"), "{label}");
    }

    #[test]
    fn a_street_without_a_near_boundary_keeps_its_label() {
        let v = Editor::new(0).view();
        assert!(v.zones.joint.is_none());
        assert_eq!(iso_label(&v, &en(), Units::Metres), "Sample Street 1 in 2.5D, along a 40 m stretch. 6 segments.");
        let mut e = zones_far_apart();
        assert!(e.select_zone(0));
        let v = e.view();
        assert!(v.zones.joint.is_none());
        assert_eq!(iso_label(&v, &en(), Units::Metres), "Sample Street 1 in 2.5D, Zone 1, 0 m to 100 m, along a 40 m stretch. 6 segments.");
    }

    #[test]
    fn the_2_5d_draws_the_selected_zones_pieces() {
        let mut e = three_zones();
        let zero = drawn(&e);
        assert!(e.select_zone(1));
        let uid = e.view().segments[0].uid;
        let w = e.view().segments[0].width_mm;
        assert!(e.set_width(uid, w + 300));
        let one = drawn(&e);
        assert_ne!(one, zero);
        let top = top_of(&one, uid);
        assert_ne!(top, top_of(&zero, uid), "the widened piece's top face");
        // The widened top is in the selected zone's own section, and in no other.
        let own = |m: &str| {
            let at = m.find("<g class=\"iso-section\">").expect("the selected zone's section");
            let rest = &m[at..];
            rest[..rest[1..].find("<g class=\"iso-section").map_or(rest.len(), |i| i + 1)].to_string()
        };
        assert!(own(&one).contains(&top), "the widened top lies in the selected zone's section");
        assert!(!own(&zero).contains(&top));
        assert!(one[one.find("iso-across").unwrap()..].split("<g class=\"iso-section\">").next().is_some_and(|across| !across.contains(&top)));
        // Putting the width back puts the drawing back.
        assert!(e.set_width(uid, w));
        assert_eq!(drawn(&e), zero);
        assert!(e.set_width(uid, w + 300));
        assert!(e.select_zone(0));
        // Zone 1 is zone 0's neighbour across its end boundary, so the edit shows beyond it (in the half across, with no
        // uid), and the scene is as wide as the widest section; the selected zone's own pieces are the same.
        let back = drawn(&e);
        assert_ne!(back, zero);
        assert_eq!(count(&back, "data-uid"), count(&zero, "data-uid"));
    }

    #[test]
    fn the_layers_are_drawn_in_order_ground_surface_markings_raised_solids_and_selection() {
        let mut e = furnished();
        e.select(Some(e.view().segments[0].uid));
        let m = drawn(&e);
        let at = |needle: &str| m.find(needle).unwrap_or_else(|| panic!("no {needle}"));
        let order = [
            at("class=\"iso-ground\""),
            at("<g class=\"iso-surface\">"),
            at("<g class=\"iso-markings\">"),
            at("<g class=\"iso-raised\">"),
            at("class=\"iso-solid"),
            at("class=\"iso-sel\""),
        ];
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
        assert!(m.find("<defs>").unwrap() < order[0]);
    }

    /// What is inside the first group of `class`, nested groups and all.
    fn layer_of<'a>(markup: &'a str, class: &str) -> &'a str {
        let open = format!("<g class=\"{class}\">");
        let start = markup.find(&open).unwrap_or_else(|| panic!("no {class} layer"));
        let (mut depth, mut at) = (0_i32, start);
        loop {
            let next_open = markup[at..].find("<g ").map(|i| at + i);
            let next_close = markup[at..].find("</g>").map(|i| at + i).expect("the layer is closed");
            match next_open {
                Some(o) if o < next_close => {
                    depth += 1;
                    at = o + 3;
                }
                _ => {
                    depth -= 1;
                    at = next_close + 4;
                    if depth == 0 {
                        return &markup[start + open.len()..next_close];
                    }
                }
            }
        }
    }

    #[test]
    fn the_signs_layer_is_empty_across_the_boundary() {
        let mut v = joined(true, 0);
        v.speed_kmh = Some(30);
        let p = Projection::fit(1050.0, span_mm(&v)).over(Reach::STRETCH);
        let j = v.zones.joint.as_ref().unwrap();
        assert!(scene::signs(&v, &j.segments, &p, &en(), true).shapes.is_empty());
        assert!(!scene::signs(&v, &v.segments, &p, &en(), false).shapes.is_empty());
        // In the whole drawing the one sign group is in the selected section.
        let m = iso_svg(&v, &en(), 1050.0).markup;
        assert_eq!(count(&m, "<g class=\"iso-sign\""), 1, "{m}");
    }

    #[test]
    fn the_outline_is_one_group_over_both_halves() {
        let mut v = joined(true, 0);
        let first = v.segments[0].uid;
        v.selected = Some(first);
        let d = diagram(&v, &en(), 1050.0);
        assert_eq!(d.sections.len(), 2);
        assert_eq!(d.outline.kind, LayerKind::Outline);
        assert_eq!(d.outline.shapes.len(), 2, "a casing and a line");
        assert!(d.sections.iter().all(|s| s.layers.iter().all(|l| l.kind != LayerKind::Outline)), "the outline is outside the sections");
        let m = iso_svg(&v, &en(), 1050.0).markup;
        assert_eq!(count(&m, "class=\"iso-layer-outline\""), 1);
        assert_eq!(count(&m, "iso-sel-casing"), 1);
        v.selected = None;
        assert!(diagram(&v, &en(), 1050.0).outline.shapes.is_empty());
    }

    #[test]
    fn each_kind_of_thing_is_a_layer_of_its_own_stacked_ground_surface_markings_raised_objects_signs_outline() {
        let mut e = furnished();
        e.select(Some(e.view().segments[0].uid));
        let mut v = e.view();
        v.speed_kmh = Some(30);
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let classes = LayerKind::ALL.map(LayerKind::class);
        assert_eq!(classes, ["iso-layer-ground", "iso-surface", "iso-markings", "iso-raised", "iso-objects", "iso-signs", "iso-layer-outline"]);
        let at: Vec<usize> = classes.iter().map(|c| m.find(&format!("<g class=\"{c}\">")).unwrap_or_else(|| panic!("no {c}"))).collect();
        assert!(at.windows(2).all(|w| w[0] < w[1]), "{at:?}");
        // Each holds its own kind of thing and nothing of another's.
        assert!(layer_of(&m, "iso-layer-ground").contains("class=\"iso-ground\"") && !layer_of(&m, "iso-layer-ground").contains("<g "));
        let objects = layer_of(&m, "iso-objects");
        assert!(objects.contains("class=\"iso-solid") && !objects.contains("iso-piece") && !objects.contains("iso-top"), "{objects}");
        let raised = layer_of(&m, "iso-raised");
        assert!(raised.contains("iso-piece") && !raised.contains("iso-solid"), "no object among the strips");
        assert!(layer_of(&m, "iso-signs").contains("iso-sign"));
        let outline = layer_of(&m, "iso-layer-outline");
        assert!(outline.contains("class=\"iso-sel\"") && !outline.contains("<g "));
        // Nothing is left outside a layer: a section holds layers only, and the outline is the last of them.
        assert!(m.find("<defs>").unwrap() < at[0]);
        assert_eq!(count(&m, "class=\"iso-raised\""), 1, "one raised layer, not runs between the objects");
    }

    #[test]
    fn the_surface_layer_holds_the_top_of_every_roadway_piece_and_the_raised_layer_the_rest() {
        let e = furnished();
        let v = e.view();
        let m = drawn(&e);
        let (surface, markings, raised) =
            (m.find("<g class=\"iso-surface\">").unwrap(), m.find("<g class=\"iso-markings\">").unwrap(), m.find("<g class=\"iso-raised\">").unwrap());
        let roadway = v.segments.iter().filter(|s| height_mm(KINDS[s.kind].id) == 0).count();
        assert_eq!(count(&m[surface..markings], "class=\"iso-top "), roadway);
        assert_eq!(count(&m[raised..], "class=\"iso-top "), v.segments.len() - roadway);
        assert!(!m[surface..markings].contains("k-sidewalk") && !m[surface..markings].contains("iso-curb"));
        // Each keeps its group, so a selection and the e2e selectors still find it.
        for s in &v.segments {
            assert!(m.contains(&format!("<g class=\"iso-piece\" data-uid=\"{}\">", s.uid)));
        }
    }

    #[test]
    fn the_paint_is_between_the_surface_and_the_raised_pieces_so_a_higher_neighbour_covers_it() {
        let (e, _, kerbside) = one_way_bikes();
        let v = e.view();
        let m = drawn(&e);
        let (markings, raised) = (m.find("<g class=\"iso-markings\">").unwrap(), m.find("<g class=\"iso-raised\">").unwrap());
        assert_eq!(count(&m[markings..raised], "iso-paint"), count(&m, "iso-paint"), "every bit of paint is in the markings layer");
        // The kerbside lane's right neighbour, the raised sidewalk, and its curb blocks are drawn after the paint.
        let pos = v.segments.iter().position(|s| s.uid == kerbside).unwrap();
        let sidewalk = v.segments[pos + 1].uid;
        assert_eq!(KINDS[v.segments[pos + 1].kind].id, "sidewalk");
        assert!(m.find(&format!("<g class=\"iso-piece\" data-uid=\"{sidewalk}\">")).unwrap() > raised);
        let curbs: Vec<usize> = m.match_indices(&format!("<g class=\"iso-piece\" data-uid=\"{kerbside}\">")).skip(1).map(|(at, _)| at).collect();
        assert!(!curbs.is_empty() && curbs.iter().all(|c| *c > raised));
    }

    #[test]
    fn a_known_street_has_a_known_number_of_paint_quads() {
        // Sample Street 1: a solid white line beside each parking lane (1 quad each), a double yellow one between the
        // lanes (2 quads), and an arrow on each driving lane.
        let m = drawn(&Editor::new(0));
        assert_eq!(count(&m, "class=\"iso-paint yellow\""), 2);
        assert_eq!(count(&m, "class=\"iso-paint white\""), 4);
        assert_eq!(count(&m, "class=\"iso-mark\""), 2);
        // The paint never carries the classes the shading and the counting of faces go by.
        for tag in m.split("<polygon ").filter(|t| t.starts_with("class=\"iso-paint")) {
            let head = &tag[..tag.find('>').unwrap()];
            assert!(
                !head.contains("iso-top")
                    && !head.contains("iso-front")
                    && !head.contains("iso-side")
                    && !head.contains("data-face")
                    && !head.contains("filter="),
                "{head}"
            );
        }
    }

    #[test]
    fn symbols_replace_the_arrows_on_driving_bus_and_bike_lanes() {
        for e in [Editor::new(0), Editor::new(1), Editor::new(3), furnished(), one_way_bikes().0] {
            let m = drawn(&e);
            assert_eq!(count(&m, "class=\"iso-dir\""), 0);
            assert_eq!(count(&m, "class=\"dir\""), 0);
            assert_eq!(count(&m, "class=\"dir-halo\""), 0);
        }
        // One symbol per lane that has one: furnished() has four driving lanes and a two-way bike lane.
        assert_eq!(count(&drawn(&furnished()), "class=\"iso-mark\""), 5);
        assert_eq!(count(&drawn(&one_way_bikes().0), "class=\"iso-mark\""), 4);
        let mut e = Editor::new(0);
        let bus = e.add(kind_index("bus").unwrap(), 3);
        assert!(drawn(&e).contains(&format!("data-mark=\"{bus}\"")));
    }

    #[test]
    fn paint_stays_inside_the_drawing_when_the_design_is_wider_than_the_street() {
        let mut e = Editor::new(0);
        let u = e.view().segments[0].uid;
        assert!(e.set_width(u, 8_000));
        assert!(e.view().delta_mm > 0);
        for width in [600.0, 1050.0, 1800.0] {
            let s = iso_svg(&e.view(), &en(), width);
            assert!(!s.markup.contains("NaN") && !s.markup.contains("inf"), "{width}");
            let m = &s.markup[s.markup.find("<g class=\"iso-markings\">").unwrap()..s.markup.find("<g class=\"iso-raised\">").unwrap()];
            assert!(count(m, "iso-paint") > 0);
            for (x, y) in xs_ys(m) {
                assert!(x >= 0.0 && x <= s.width && y >= 0.0 && y <= s.height, "{width}: ({x}, {y})");
            }
        }
    }

    #[test]
    fn the_marked_street_draws_every_kind_of_paint() {
        let m = drawn(&marked());
        // `marked()` is: sidewalk, parking, driving lane, bus lane (with the traffic), driving lane, parking, two-way bike
        // lane, sidewalk. By the markings rules the polygons are, yellow: the double yellow between the bus lane and
        // the lane running the other way (2) and the bike lane's broken middle line (5 dashes in 40 m) = 7; white: a
        // solid line between each parking lane and its driving lane (2), the double white between the first lane and
        // the bus lane (2), the two lane arrows (2), the bus diamond and "BUS" (15) and the bike symbol (28: two
        // 12-piece wheels, frame, handlebar, two small arrows) = 49. The bike lane's own curb leaves its edges unpainted.
        assert_eq!(count(&m, "class=\"iso-paint yellow\""), 7);
        assert_eq!(count(&m, "class=\"iso-paint white\""), 49);
        assert!(!m.contains("NaN") && !m.contains("inf"));
    }

    /// Not a check: prints how long the markup of some streets is, at 1050 and 600 px.
    /// `cargo test street::iso::tests::markup_size -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn markup_size() {
        for (name, e) in
            [("sample 0", Editor::new(0)), ("sample 1", Editor::new(1)), ("sample 3", Editor::new(3)), ("furnished", furnished()), ("marked", marked())]
        {
            for width in [1050.0, 600.0] {
                println!("{name} at {width}: {} bytes", iso_svg(&e.view(), &en(), width).markup.len());
            }
        }
    }

    /// Not a check: writes `target/iso-preview-light.html` and `target/iso-preview-dark.html`, the 2.5D of
    /// every sample street and of `furnished()` at three widths with the page's styles and hatches, to look at
    /// in a browser. `cargo test street::iso::tests::preview -- --ignored`.
    #[test]
    #[ignore]
    fn preview() {
        use crate::shared::symbols::{CURB_HATCH, HATCH, MATERIAL_HATCH};
        let root = env!("CARGO_MANIFEST_DIR");
        let defs: String = HATCH.iter().chain(MATERIAL_HATCH.iter()).chain(CURB_HATCH.iter()).map(|(_, s)| *s).collect();
        let streets: Vec<(String, Editor)> = (0..4)
            .map(|i| (format!("sample {i}"), Editor::new(i)))
            .chain([("furnished".to_string(), furnished()), ("one-way bikes".to_string(), one_way_bikes().0), ("marked".to_string(), marked())])
            .collect();
        let mut body = String::new();
        for (name, e) in &streets {
            for width in [1050.0, 760.0, 600.0] {
                let s = iso_svg(&e.view(), &en(), width);
                body += &format!(
                    "<figure><figcaption>{name}, {width} px</figcaption><svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">{m}</svg></figure>",
                    w = s.width,
                    h = s.height,
                    m = s.markup
                );
            }
        }
        for theme in ["light", "dark"] {
            let html = format!(
                "<!doctype html><html data-theme=\"{theme}\"><head><meta charset=\"utf-8\"><link rel=\"stylesheet\" href=\"file://{root}/web/app.css\"><link rel=\"stylesheet\" href=\"file://{root}/web/editor.css\"><style>body{{background:var(--panel);color:var(--ink);font:14px sans-serif}}figure{{margin:16px}}</style></head><body><svg width=\"0\" height=\"0\" style=\"position:absolute\"><defs>{defs}</defs></svg>{body}</body></html>"
            );
            std::fs::write(format!("{root}/target/iso-preview-{theme}.html"), html).unwrap();
        }
    }

    // --- both sections across a boundary ---

    /// A view of a street of Sample Avenue 2 (trees, a median) in two zones, zone `selected` shown.
    fn avenue_joined(selected: usize) -> View {
        use crate::shared::catalogue::Side;
        use crate::street::model::zoned;
        let s = zoned(crate::street::model::Street::sample(1, Side::Right), &[0, 50_000], 100_000);
        let mut e = Editor::from_street(&s, &s, 0);
        assert!(e.select_zone(selected));
        e.view()
    }

    fn halves() -> (Reach, Reach) {
        (Reach { near: 0.0, far: 20_000.0 }, Reach { near: 20_000.0, far: 40_000.0 })
    }

    #[test]
    fn a_street_without_a_near_boundary_draws_as_before() {
        for n in 0..4 {
            let m = drawn(&Editor::new(n));
            assert_eq!(count(&m, "iso-section"), 0);
            assert_eq!(count(&m, "iso-across"), 0);
        }
    }

    #[test]
    fn a_boundary_draws_both_sections_each_in_its_half() {
        let v = joined(true, 1);
        let j = v.zones.joint.as_ref().unwrap();
        let m = iso_svg(&v, &en(), 1050.0).markup;
        assert_eq!(count(&m, "<g class=\"iso-section"), 2);
        assert_eq!(count(&m, "<g class=\"iso-section iso-across"), 1);
        assert_eq!(count(&m, "class=\"iso-ground\""), 1);
        assert_eq!(count(&m, "class=\"iso-top "), v.segments.len() + j.segments.len() + 1, "and the lane the host lacks opening in its run");
        let p = Projection::fit(1050.0, span_mm(&v));
        assert_eq!(count(&m, "class=\"iso-piece iso-across\""), raised::strips(&j.segments, &p).len());
        assert!(!m.contains("NaN"));
        // Each section stays in its own half: the near one (here the neighbour) is nearer than the seam, the far one beyond it.
        let seam = p.at(0.0, 0.0, 20_000.0).1;
        let sections: Vec<&str> =
            m.split("<g class=\"iso-section").skip(1).map(|t| &t[..t.find("<polygon class=\"iso-sel-casing\"").unwrap_or(t.len())]).collect();
        assert_eq!(sections.len(), 2);
        let (own, across) = (sections[0], sections[1]);
        // (a thing that stands up rises above its foot on the drawing, so the bound is the tallest thing's top)
        let tallest = p.at(0.0, TALLEST_MM, 20_000.0).1;
        assert!(xs_ys(across).iter().all(|&(_, y)| y >= tallest - 0.05), "the near half is nearer than the seam");
        assert!(xs_ys(own).iter().all(|&(_, y)| y <= seam + 0.05), "the far half is beyond the seam");
    }

    #[test]
    fn the_far_section_is_painted_first() {
        let m = iso_svg(&joined(true, 0), &en(), 1050.0).markup;
        assert!(m.find("iso-across").unwrap() < m.rfind("<g class=\"iso-section\"").unwrap(), "zone 2 lies beyond zone 1");
        let m = iso_svg(&joined(true, 1), &en(), 1050.0).markup;
        assert!(m.find("<g class=\"iso-section\"").unwrap() < m.find("iso-across").unwrap(), "zone 1 lies before zone 2");
    }

    #[test]
    fn a_street_anchored_at_the_last_end_swaps_the_halves() {
        let m = iso_svg(&joined_from_last(true, 0), &en(), 1050.0).markup;
        assert!(m.find("<g class=\"iso-section\"").unwrap() < m.find("iso-across").unwrap(), "the across section is now the near run");
        let m = iso_svg(&joined_from_last(true, 1), &en(), 1050.0).markup;
        assert!(m.find("iso-across").unwrap() < m.rfind("<g class=\"iso-section\"").unwrap());
    }

    #[test]
    fn the_neighbours_pieces_carry_no_uid_and_the_selected_ones_do() {
        let v = joined(true, 1);
        let m = iso_svg(&v, &en(), 1050.0).markup;
        assert_eq!(count(&m, "iso-piece iso-across\" data-uid"), 0);
        // A uid of the selected section is also one of the neighbour's (uids repeat across sections): only the selected one carries it.
        let uid = v.segments[0].uid;
        assert!(v.zones.joint.as_ref().unwrap().segments.iter().any(|s| s.uid == uid));
        let p = Projection::fit(1050.0, span_mm(&v));
        let want = raised::strips(&v.segments, &p).iter().filter(|s| s.1 == 0).count();
        assert_eq!(count(&m, &format!("data-uid=\"{uid}\"")), want);
        assert!(count(&m, "<g class=\"iso-piece iso-across\"><title>") > 0, "the neighbour's pieces still name themselves");
    }

    #[test]
    fn the_selected_pieces_outline_is_drawn_once_over_both_halves() {
        for (selected, far) in [(1, true), (0, false)] {
            let mut e = crate::street::model::two_zones(true);
            assert!(e.select_zone(selected));
            let uid = e.view().segments[1].uid;
            e.select(Some(uid));
            let v = e.view();
            let m = iso_svg(&v, &en(), 1050.0).markup;
            assert_eq!(count(&m, "class=\"iso-sel\""), 1);
            let at = m.find("<polygon class=\"iso-sel-casing\"").unwrap();
            assert!(!m[at..].contains("<g"), "after the last section");
            // The outline stands over the selected zone's half.
            let (near, farr) = halves();
            let p = Projection::fit(1050.0, span_mm(&v)).over(if far { farr } else { near });
            let want = layer_markup(&scene::outline(&p, &v.segments, 1), &p);
            assert_eq!(want, format!("<g class=\"iso-layer-outline\">{}</g>", layer_of(&m, "iso-layer-outline")), "zone {selected}");
        }
    }

    #[test]
    fn a_raised_piece_that_starts_at_the_boundary_has_its_end_face_there() {
        // Zone 1 selected, zone 2 beyond: its sidewalks stand from 20 m on, with a front face cut there.
        let v = joined(true, 0);
        let j = v.zones.joint.as_ref().unwrap();
        let p = Projection::fit(1050.0, span_mm(&v)).over(halves().1);
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let across = &m[m.find("iso-across").unwrap()..];
        let s = j.segments.iter().find(|s| KINDS[s.kind].id == "sidewalk").unwrap();
        let (x0, x1, h) = (s.x_mm as f64, (s.x_mm + s.width_mm) as f64, drawn_mm(height_mm("sidewalk")));
        let front = points(&p, &[(x0, 0.0, 20_000.0), (x1, 0.0, 20_000.0), (x1, h, 20_000.0), (x0, h, 20_000.0)]);
        assert!(across.contains(&format!("class=\"iso-front k-sidewalk\" points=\"{front}\"")), "{front}");
    }

    #[test]
    fn a_neighbour_wider_than_the_selected_section_widens_the_drawing_and_nothing_panics() {
        let mut v = joined(true, 0);
        let j = v.zones.joint.as_mut().unwrap();
        let more = v.row_mm.max(v.total_mm);
        j.segments.last_mut().unwrap().width_mm += more;
        j.total_mm += more;
        let total = j.total_mm;
        assert_eq!(span_mm(&v), total);
        assert!(total > v.row_mm.max(v.total_mm));
        assert_eq!(iso_height(&v, 1050.0), Projection::fit(1050.0, total).height);
        let m = iso_svg(&v, &en(), 1050.0).markup;
        assert!(!m.contains("NaN") && !m.contains("inf"));
    }

    #[test]
    fn each_half_has_its_markings_symbols_and_solids() {
        use super::solids::Solid;
        let v = avenue_joined(0);
        let j = v.zones.joint.as_ref().unwrap();
        let base = Projection::fit(1050.0, span_mm(&v));
        let (near, far) = halves();
        let m = iso_svg(&v, &en(), 1050.0).markup;
        // The selected zone is the near half, the neighbour the far one.
        let crowns = |segs: &[SegView], p: &Projection| (0..segs.len()).flat_map(|i| blocks(segs, i, p)).filter(|b| b.solid == Solid::Crown).count();
        let (a, b) = (crowns(&v.segments, &base.over(near)), crowns(&j.segments, &base.over(far)));
        assert!(a > 0 && b > 0);
        assert_eq!(count(&m, "s-crown"), a + b);
        let marks = |segs: &[SegView], p: &Projection| count(&layer_markup(&scene::markings(p, segs, None), p), "class=\"iso-mark\"");
        let (a, b) = (marks(&v.segments, &base.over(near)), marks(&j.segments, &base.over(far)));
        assert!(a > 0 && b > 0);
        assert_eq!(count(&m, "class=\"iso-mark\""), a + b);
    }

    /// The polygons of a group of the markup, each as its points.
    fn polygons(markup: &str) -> Vec<Vec<(f64, f64)>> {
        markup
            .split("<polygon ")
            .skip(1)
            .filter(|tag| !tag[..tag.find('>').unwrap()].contains("transform="))
            .map(|tag| {
                let at = tag.find("points=\"").unwrap() + 8;
                tag[at..at + tag[at..].find('"').unwrap()]
                    .split(' ')
                    .map(|pt| {
                        let mut c = pt.split(',').map(|n| n.parse::<f64>().unwrap());
                        (c.next().unwrap(), c.next().unwrap())
                    })
                    .collect()
            })
            .collect()
    }

    /// The part of the markup between `open` and the end of that group (groups here do not nest the same tag).
    fn group_of<'a>(markup: &'a str, open: &str) -> &'a str {
        let at = markup.find(open).unwrap_or_else(|| panic!("{open} is drawn"));
        &markup[at..at + markup[at..].find("</g>").unwrap()]
    }

    /// The same drawing with the transition taken off the joint: the abrupt seam.
    fn untapered(mut v: View) -> String {
        v.zones.joint.as_mut().unwrap().taper = None;
        iso_svg(&v, &en(), 1050.0).markup
    }

    #[test]
    fn a_joint_with_an_added_lane_draws_a_tapered_top() {
        for selected in [0, 1] {
            let v = joined(true, selected);
            assert!(v.zones.joint.as_ref().unwrap().taper.is_some());
            let m = iso_svg(&v, &en(), 1050.0).markup;
            assert_eq!(count(&m, "<g class=\"iso-piece iso-taper\">"), 1, "selected {selected}: the lane the host lacks grows in its run");
            let top = &polygons(group_of(&m, "<g class=\"iso-piece iso-taper\">"))[0];
            // Zone 2 hosts: its run is the far half and the seam is at its near end (the corners are near-left, near-right,
            // far-right, far-left).
            assert_eq!(top.len(), 4);
            let (seam, away) = ((top[1].0 - top[0].0).abs(), (top[2].0 - top[3].0).abs());
            assert!(away < 0.01, "selected {selected}: no width at the end away from the seam ({away})");
            assert!(seam > 5.0, "selected {selected}: the full lane at the seam ({seam})");
        }
    }

    #[test]
    fn a_joint_anchored_at_the_last_end_tapers_toward_the_other_run() {
        for selected in [0, 1] {
            let v = joined_from_last(true, selected);
            assert!(v.zones.joint.as_ref().unwrap().taper.is_some(), "selected {selected}");
            let m = iso_svg(&v, &en(), 1050.0).markup;
            assert_eq!(count(&m, "<g class=\"iso-piece iso-taper\">"), 1, "selected {selected}");
            let top = &polygons(group_of(&m, "<g class=\"iso-piece iso-taper\">"))[0];
            // the seam is where the two runs meet, 20 m along the stretch; the lane opens from a point at the other end
            let seam_y = Projection::fit(1050.0, span_mm(&v)).at(0.0, 0.0, 20_000.0).1;
            let (near_seam, away): (Vec<_>, Vec<_>) = [(top[0], top[1]), (top[3], top[2])].into_iter().partition(|(a, _)| (a.1 - seam_y).abs() < 1.0);
            assert_eq!((near_seam.len(), away.len()), (1, 1), "selected {selected}");
            let width = |(a, b): &((f64, f64), (f64, f64))| (b.0 - a.0).abs();
            assert!(width(&near_seam[0]) > 5.0, "selected {selected}: the full lane at the seam");
            assert!(width(&away[0]) < 0.01, "selected {selected}: no width at the end away from the seam");
        }
    }

    #[test]
    fn the_lines_follow_the_taper() {
        let v = joined(true, 1);
        let tapered = iso_svg(&v, &en(), 1050.0).markup;
        let plain = untapered(v);
        let (a, b) = (polygons(group_of(&plain, "<g class=\"iso-markings\">")), polygons(group_of(&tapered, "<g class=\"iso-markings\">")));
        assert_eq!(a.len(), b.len());
        let span = |p: &Vec<(f64, f64)>| p.iter().map(|c| c.1).fold(f64::MIN, f64::max) - p.iter().map(|c| c.1).fold(f64::MAX, f64::min);
        // The longest line runs the whole run. The seam is at the near end (the larger y) of this run.
        let k = (0..a.len()).max_by(|&i, &j| span(&a[i]).total_cmp(&span(&a[j]))).unwrap();
        let x_at = |p: &Vec<(f64, f64)>, near: bool| {
            let y = if near { p.iter().map(|c| c.1).fold(f64::MIN, f64::max) } else { p.iter().map(|c| c.1).fold(f64::MAX, f64::min) };
            p.iter().filter(|c| (c.1 - y).abs() < 1e-6).map(|c| c.0).sum::<f64>() / p.iter().filter(|c| (c.1 - y).abs() < 1e-6).count() as f64
        };
        assert!((x_at(&a[k], false) - x_at(&b[k], false)).abs() < 0.01, "a line starts where the host's section has it");
        assert!((x_at(&a[k], true) - x_at(&b[k], true)).abs() > 0.5, "and ends where the other section has it");
    }

    #[test]
    fn raised_pieces_stay_abrupt() {
        let v = joined(true, 1);
        let sidewalk = v.segments.iter().find(|s| KINDS[s.kind].id == "sidewalk").unwrap().uid;
        let tapered = iso_svg(&v, &en(), 1050.0).markup;
        assert_eq!(top_of(&tapered, sidewalk), top_of(&untapered(v), sidewalk));
    }

    #[test]
    fn no_taper_without_a_transition() {
        // The markup of joints between identical sections, as it was before transitions (FNV-1a of the drawing).
        let h = |s: &str| s.bytes().fold(0xcbf29ce484222325u64, |a, b| (a ^ u64::from(b)).wrapping_mul(0x100000001b3));
        for (name, v, hash) in [
            ("first, zone 1", joined(false, 0), 0xb0a26882a7ac3ec6u64),
            ("first, zone 2", joined(false, 1), 0x512f95d709f00ffe),
            ("last, zone 1", joined_from_last(false, 0), 0x512f95d709f00ffe),
            ("last, zone 2", joined_from_last(false, 1), 0xb0a26882a7ac3ec6),
        ] {
            assert!(v.zones.joint.as_ref().unwrap().taper.is_none(), "{name}");
            let m = iso_svg(&v, &en(), 1050.0).markup;
            assert_eq!(count(&m, "iso-taper"), 0, "{name}");
            assert_eq!(h(&m), hash, "{name}");
        }
    }

    #[test]
    fn the_label_says_the_transition() {
        let v = joined(true, 1);
        let en_label = iso_label(&v, &en(), Units::Metres);
        assert!(en_label.contains("The section changes abruptly there. Zone 2 has 5 segments. A transition of 53 m leads into Zone 1."), "{en_label}");
        let fr = crate::i18n_for(Locale::FrCa);
        let fr_label = iso_label(&v, &fr, Units::Metres);
        assert!(fr_label.contains("Zone 2 compte 5 éléments. Une transition de 53\u{a0}m mène à zone 1."), "{fr_label}");
        // Seen from zone 1, zone 2 (across the boundary) hosts it and it leads into zone 1.
        let v = joined(true, 0);
        assert!(iso_label(&v, &en(), Units::Metres).ends_with("A transition of 53 m leads into Zone 1."), "{}", iso_label(&v, &en(), Units::Metres));
    }
}
