//! The street in 2.5D: a straight stretch of it seen from above and to one side, each piece an
//! extruded strip with its height, the few things that stand on the pieces as plain solids, and the
//! one-way arrows on the roadway. It is drawn from the same view the section is and is read-only:
//! the section is where the street is edited.
//!
//! The rises are exaggerated: a curb or a sidewalk stands 150 mm, a few pixels at the scale a whole
//! street is drawn at, so the slabs, and the curbs with them, are drawn `RISE` times as high as they stand
//! (the height tables keep the true heights). What a curb stands above its own piece (a bike lane's divider)
//! is drawn true, so that it does not bury a narrow lane (`curb_top_mm`). The things that stand on the
//! pieces keep their own size and sit on the drawn tops.
//!
//! The code is split by what it draws: `projection` (where a point lands), `raised` (the strips standing
//! above the roadway and their order), `solids` (boxes on the pieces) and `surface` (the roadway).

use std::fmt::Write;

use crate::shared::catalogue::{KINDS, kind_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::iso::format::esc;
use crate::shared::said::say;
use crate::shared::units::Units;
use crate::street::model::View;
use crate::street::svg::curb_edges;

#[allow(dead_code)]
mod markings;
mod projection;
mod raised;
mod solids;
mod surface;
#[cfg(test)]
mod testing;

pub use projection::{ARROW_AT_MM, MAX_HEIGHT, MIN_SOLID_PX, Projection, RECEDE_X, RECEDE_Y, RISE, STRETCH_MM, drawn_mm};
pub use raised::{HEIGHTS, curb_height_mm, curb_top_mm, height_mm};

use projection::points;
use raised::{beside, curb, shades, slab, strips};
use solids::{Block, block, blocks};
use surface::arrow;

/// The 2.5D drawn, and the size it is drawn at.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoSvg {
    pub markup: String,
    pub width: f64,
    pub height: f64,
}

/// The outline of the selected piece at `i`, as one shape round everything drawn for it (its strip
/// and its curbs): its near face, its right side where that is seen, and its top, at the height of
/// its highest part.
fn outline(out: &mut String, p: &Projection, v: &View, i: usize) {
    let s = &v.segments[i];
    let id = KINDS[s.kind].id;
    let top = s.curb.filter(|_| !curb_edges(v, i).is_empty()).map_or(drawn_mm(height_mm(id)), |c| curb_top_mm(c, id));
    let (x0, x1, l) = (s.x_mm as f64, (s.x_mm + s.width_mm) as f64, STRETCH_MM);
    let low = beside(v, i).min(top);
    write!(
        out,
        "<polygon class=\"iso-sel\" points=\"{}\"/>",
        points(p, &[(x0, 0.0, 0.0), (x1, 0.0, 0.0), (x1, low, 0.0), (x1, low, l), (x1, top, l), (x0, top, l), (x0, top, 0.0)])
    )
    .unwrap();
}

/// How wide the scene is across: the street, or the pieces where they run over it.
fn span_mm(v: &View) -> i32 {
    v.row_mm.max(v.total_mm)
}

/// How tall the 2.5D of this street is drawn `width` wide, without drawing it.
pub fn iso_height(v: &View, width: f64) -> f64 {
    Projection::fit(width, span_mm(v)).height
}

/// The street in 2.5D, drawn `width` wide: a stretch of `STRETCH_MM` seen obliquely, from the same
/// view the section is drawn from. Hovering a piece names it.
///
/// What is drawn is ordered so that nothing needs sorting by depth: the ground, then the strips from
/// the lowest to the highest (a raised piece stands in front of the roadway beside it on both sides),
/// each piece's slab, then its one-way arrow, before its curbs. Then what stands on the pieces from far to
/// near, and last the selected piece's outline.
pub fn iso_svg(v: &View, i18n: &I18n, width: f64) -> IsoSvg {
    let span = span_mm(v);
    let p = Projection::fit(width, span);
    let (w, l) = (span as f64, STRETCH_MM);
    let mut out = format!("<defs>{}</defs>", shades());
    write!(out, "<polygon class=\"iso-ground\" points=\"{}\"/>", points(&p, &[(0.0, 0.0, 0.0), (w, 0.0, 0.0), (w, 0.0, l), (0.0, 0.0, l)])).unwrap();

    for (_, i, edge) in strips(v) {
        let s = &v.segments[i];
        let name = i18n.tr(&kind_key(KINDS[s.kind].id), &Args::new());
        write!(out, "<g class=\"iso-piece\" data-uid=\"{}\"><title>{}</title>", s.uid, esc(&name)).unwrap();
        match edge {
            None => slab(&mut out, &p, v, i),
            Some(right) => curb(&mut out, &p, v, i, right),
        }
        out.push_str("</g>");
        // A lane's arrow lies on its own strip, so its curbs and any higher piece, drawn later, stand over it.
        if edge.is_none() {
            arrow(&mut out, &p, s);
        }
    }

    // What stands on the pieces, so that a nearer box covers the ones behind it: piece by piece from left
    // to right (each box stays inside its piece, and the eye sees the right sides, so a box on a piece
    // further right is the nearer wherever two overlap), and on each piece from the far end to the near and
    // from left to right. A tree's crown starts nearer than its trunk, so it covers it.
    let mut solids: Vec<(usize, Block)> = (0..v.segments.len()).flat_map(|i| blocks(v, i, &p).into_iter().map(move |b| (i, b))).collect();
    solids.sort_by(|(i, a), (j, b)| i.cmp(j).then(b.z.0.total_cmp(&a.z.0)).then(a.x.0.total_cmp(&b.x.0)));
    for (_, b) in &solids {
        block(&mut out, &p, b);
    }
    if let Some(i) = v.selected.and_then(|u| v.segments.iter().position(|s| s.uid == u)) {
        outline(&mut out, &p, v, i);
    }

    IsoSvg { markup: out, width: p.width, height: p.height }
}

/// What a screen reader is told the 2.5D drawing is: the street, the stretch shown, and its pieces.
pub fn iso_label(v: &View, i18n: &I18n, units: Units) -> String {
    let args = Args::new()
        .str("name", say(i18n, units, &v.name))
        .str("length", units.length_whole_in(STRETCH_MM as i32, i18n.locale()))
        .num("count", v.segments.len() as i64);
    i18n.tr("svg-iso-label", &args)
}

#[cfg(test)]
mod tests {
    use super::projection::{PAD, r};
    use super::testing::{count, drawn, en, furnished, one_way_bikes, top_of, xs, xs_ys};
    use super::*;
    use crate::shared::catalogue::kind_index;
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
        assert_eq!(count(&m, "class=\"iso-top k-sidewalk\""), 2);
        assert_eq!(count(&m, "fill=\"url(#h-travel)\""), 2);
        assert_eq!(count(&m, "class=\"iso-ground\""), 1);
        assert!(!m.contains("NaN"));
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
        // Sample Avenue 2: a sidewalk beside a planting strip of the same height has no side face; the planting strip,
        // beside parking, has one; the median has one; the right sidewalk, at the street's edge, has one.
        let avenue = drawn(&Editor::new(1));
        assert_eq!(count(&avenue, "class=\"iso-front "), 5);
        assert_eq!(count(&avenue, "class=\"iso-side "), 3);
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
            let mut kinds: Vec<String> = m.split("class=\"iso-top k-").skip(1).map(|rest| rest.split('"').next().unwrap().to_string()).collect();
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
            for width in [600.0, 760.0, 1050.0, 1800.0] {
                let s = iso_svg(&Editor::new(sample).view(), &en(), width);
                assert!(!s.markup.contains("NaN") && !s.markup.contains("inf"), "sample {sample} at {width}");
                assert!(s.height <= MAX_HEIGHT + 1e-9);
            }
        }
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
            .chain([("furnished".to_string(), furnished()), ("one-way bikes".to_string(), one_way_bikes().0)])
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
}
