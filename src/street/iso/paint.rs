//! The markings layer's one rule that others read: whether a piece is wide enough on the screen to carry its symbol.
//! The layer itself is `scene::markings`.

use super::projection::{ARROW_AT_MM, MIN_SOLID_PX, Projection};

/// Whether a piece `w` mm wide is wide enough on the screen, where its symbol lies, to carry it.
pub(super) fn symbol_fits(p: &Projection, w: f64) -> bool {
    w * p.scale_at(p.reach.near + ARROW_AT_MM) >= MIN_SOLID_PX
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::diagram::render::layer_markup;
    use crate::street::iso::markings::markings;
    use crate::street::iso::projection::{STRETCH_MM, points};
    use crate::street::iso::symbols::shapes;
    use crate::street::iso::taper::Draw;
    use crate::street::model::SegView;

    /// The markings layer as markup.
    fn layer(p: &Projection, segs: &[SegView]) -> String {
        layer_markup(&crate::street::iso::scene::markings(p, segs, None::<&Draw>), p)
    }

    use crate::shared::catalogue::{KINDS, kind_index};
    use crate::street::iso::markings::Symbol;
    use crate::street::iso::projection::drawn_mm;
    use crate::street::iso::solids::{Solid, blocks};
    use crate::street::iso::testing::{apart, count, furnished, hull};
    use crate::street::model::Editor;
    use crate::street::model::View;

    fn fit(v: &View, width: f64) -> Projection {
        Projection::fit(width, v.row_mm.max(v.total_mm))
    }

    #[test]
    fn a_known_street_paints_its_lines_and_one_arrow_per_driving_lane() {
        // Sample Street 1: a solid white line each side of the roadway, a double yellow one between the lanes.
        let v = Editor::new(0).view();
        let p = fit(&v, 1050.0);
        let m = layer(&p, &v.segments);
        assert!(m.starts_with("<g class=\"iso-markings\">") && m.ends_with("</g>"));
        assert_eq!(count(&m, "class=\"iso-paint yellow\""), 2);
        assert_eq!(count(&m, "class=\"iso-paint white\""), 2 + 2, "two solid lines and two arrows");
        assert_eq!(count(&m, "class=\"iso-mark\""), 2);
        assert!(!m.contains("NaN") && !m.contains("inf"));
        // The yellow line is drawn flat on the ground, 120 mm each side of the boundary, the whole 40 m.
        let x = (v.segments[2].x_mm + v.segments[2].width_mm) as f64;
        let left = points(&p, &[(x - 180.0, 0.0, 0.0), (x - 180.0, 0.0, STRETCH_MM), (x - 60.0, 0.0, STRETCH_MM), (x - 60.0, 0.0, 0.0)]);
        assert!(m.contains(&format!("<polygon class=\"iso-paint yellow\" points=\"{left}\"/>")), "the drawing has not got it");
    }

    #[test]
    fn a_half_paints_its_lines_and_symbols_inside_its_reach() {
        use crate::street::iso::projection::Reach;
        let v = Editor::new(0).view();
        let base = fit(&v, 1050.0);
        let y = |z: f64| base.at(0.0, 0.0, z).1;
        let ys = |m: &str| crate::street::iso::testing::xs_ys(m).into_iter().map(|(_, y)| y).collect::<Vec<_>>();
        let (near_half, far_half) = (Reach { near: 0.0, far: 20_000.0 }, Reach { near: 20_000.0, far: 40_000.0 });
        let m = layer(&base.over(near_half), &v.segments);
        assert!(ys(&m).iter().all(|&v| v >= y(20_000.0) - 0.01), "no paint beyond the reach");
        assert_eq!(count(&m, "class=\"iso-mark\""), 2);
        let m = layer(&base.over(far_half), &v.segments);
        assert!(ys(&m).iter().all(|&v| v <= y(20_000.0) + 0.01), "no paint nearer than the reach");
        // A symbol is laid ARROW_AT_MM from the reach's near end: 2.5 m nearer to 2 m farther.
        let marks = m.split("<g class=\"iso-mark\"").skip(1).collect::<Vec<_>>();
        assert_eq!(marks.len(), 2);
        for g in marks {
            let g = &g[..g.find("</g>").unwrap()];
            let ys = ys(g);
            assert!(ys.iter().all(|&v| v <= y(23_500.0) + 0.01 && v >= y(28_000.0) - 0.01), "{ys:?}");
        }
    }

    #[test]
    fn a_symbol_is_centred_on_its_lane_at_the_arrow_distance_and_lies_on_the_ground() {
        let v = Editor::new(0).view();
        let p = fit(&v, 1050.0);
        let m = layer(&p, &v.segments);
        let lane = &v.segments[2];
        let cx = lane.x_mm as f64 + lane.width_mm as f64 / 2.0;
        let toward = lane.direction == Some("toward");
        let arrow = &shapes(Symbol::Ahead { toward }, lane.width_mm as f64)[0];
        let pts = points(&p, &arrow.iter().map(|&(u, a)| (cx + u, 0.0, ARROW_AT_MM + a)).collect::<Vec<_>>());
        assert!(
            m.contains(&format!("<g class=\"iso-mark\" data-mark=\"{}\"><polygon class=\"iso-paint white\" points=\"{pts}\"/></g>", lane.uid)),
            "the drawing has not got it"
        );
    }

    #[test]
    fn a_piece_too_narrow_on_screen_keeps_its_lines_but_loses_its_symbol() {
        // A two-way bike lane at its narrowest, 1.2 m, is under 20 px wide where its symbol lies when the drawing is 480 px wide
        // (a phone held sideways), and wide enough at 600 px.
        let mut e = Editor::new(0);
        let bike = e.add(kind_index("bike").unwrap(), 3);
        assert!(e.set_width(bike, 1_200));
        // Uncurbed, so that its edges are painted (a curbed bike lane's are the curb's).
        assert!(e.set_curb(bike, None));
        let v = e.view();
        let p = fit(&v, 600.0);
        assert!(symbol_fits(&p, 1_200.0), "on this street at 600 px it fits");
        let p = fit(&v, 480.0);
        assert!(!symbol_fits(&p, 1_200.0), "at 480 px it does not");
        let m = layer(&p, &v.segments);
        assert!(!m.contains(&format!("data-mark=\"{bike}\"")));
        assert_eq!(count(&m, "class=\"iso-mark\""), 2, "the driving lanes keep theirs");
        // Sample Street 1 with the bike lane between its driving lanes: four solid white lines (parking, driving lane,
        // bike lane, driving lane, parking), the bike lane's broken yellow middle (5 dashes in 40 m) and the two arrows.
        assert_eq!(count(&m, "class=\"iso-paint yellow\""), 5);
        assert_eq!(count(&m, "class=\"iso-paint white\""), 4 + 2);
        assert!(!m.contains("NaN"));
        // At 1050 px on the street itself the same lane is wide enough.
        let p = fit(&v, 1050.0);
        assert!(layer(&p, &v.segments).contains(&format!("data-mark=\"{bike}\"")));
    }

    #[test]
    fn an_empty_street_gives_an_empty_markings_layer() {
        let mut e = Editor::new(0);
        while let Some(uid) = e.view().segments.first().map(|s| s.uid) {
            assert!(e.remove(uid));
        }
        let v = e.view();
        assert_eq!(layer(&fit(&v, 1050.0), &v.segments), "<g class=\"iso-markings\"></g>");
    }

    #[test]
    fn a_bus_lane_paints_the_diamond_and_the_letters() {
        let mut e = Editor::new(0);
        let bus = e.add(kind_index("bus").unwrap(), 3);
        let v = e.view();
        let m = layer(&fit(&v, 1050.0), &v.segments);
        let open = format!("<g class=\"iso-mark\" data-mark=\"{bus}\">");
        let at = m.find(&open).expect("the bus lane has a symbol");
        let group = &m[at..at + m[at..].find("</g>").unwrap()];
        assert_eq!(count(group, "<polygon"), 15);
    }

    #[test]
    fn a_symbol_lies_near_the_near_end_where_no_tree_can_hide_it() {
        // Trees and the shelter stand on the pieces; what they hide of the ground is none of the near end. On every
        // street with trees, at every width that draws them, no trunk or crown covers any part of the ground a
        // symbol is laid on (its extent along by across). If this fails, shrink or move the symbol in `symbols.rs`;
        // do not weaken the test.
        for e in [Editor::new(1), furnished()] {
            let v = e.view();
            for width in [760.0, 1050.0, 1800.0] {
                let p = fit(&v, width);
                let trees: Vec<Vec<(f64, f64)>> = (0..v.segments.len())
                    .flat_map(|i| blocks(&v.segments, i, &p))
                    .filter(|b| matches!(b.solid, Solid::Trunk | Solid::Crown))
                    .map(|b| {
                        let corners = [b.x.0, b.x.1]
                            .into_iter()
                            .flat_map(move |x| [b.y.0, b.y.1].into_iter().flat_map(move |y| [b.z.0, b.z.1].into_iter().map(move |z| (x, y, z))));
                        hull(corners.map(|(x, y, z)| p.at(x, y, z)).collect())
                    })
                    .collect();
                assert!(!trees.is_empty(), "{width}");
                for mark in markings(&v.segments).marks {
                    let s = v.segments.iter().find(|s| s.uid == mark.uid).unwrap();
                    if !symbol_fits(&p, s.width_mm as f64) {
                        continue;
                    }
                    let cx = s.x_mm as f64 + s.width_mm as f64 / 2.0;
                    let shapes = shapes(mark.symbol, s.width_mm as f64);
                    let pts = || shapes.iter().flatten();
                    let (u0, u1) = (pts().map(|q| q.0).fold(f64::MAX, f64::min), pts().map(|q| q.0).fold(f64::MIN, f64::max));
                    let (a0, a1) = (pts().map(|q| q.1).fold(f64::MAX, f64::min), pts().map(|q| q.1).fold(f64::MIN, f64::max));
                    let h = drawn_mm(crate::street::iso::height_mm(KINDS[s.kind].id));
                    let ground =
                        [(cx + u0, h, ARROW_AT_MM + a0), (cx + u1, h, ARROW_AT_MM + a0), (cx + u1, h, ARROW_AT_MM + a1), (cx + u0, h, ARROW_AT_MM + a1)]
                            .map(|(x, y, z)| p.at(x, y, z));
                    assert!(trees.iter().all(|t| apart(t, &ground)), "{width}: the symbol of piece {} is hidden", mark.uid);
                }
            }
        }
    }

    #[test]
    fn a_broken_line_keeps_its_rhythm_across_the_boundary() {
        use crate::street::iso::projection::Reach;
        use crate::street::iso::testing::{marked, xs_ys};
        // Sample Street 1 with a two-way bike lane: a broken yellow middle line.
        let v = marked().view();
        let base = fit(&v, 1050.0);
        let y = |z: f64| base.at(0.0, 0.0, z).1;
        let lines = |m: &str| m.split("<g class=\"iso-mark\"").next().unwrap().to_string();
        let ranges = |m: &str| -> Vec<(f64, f64)> {
            lines(m)
                .split("<polygon")
                .skip(1)
                .map(|poly| {
                    let ys: Vec<f64> = xs_ys(&format!("<polygon{}", &poly[..poly.find("/>").unwrap()])).into_iter().map(|(_, y)| y).collect();
                    (ys.iter().cloned().fold(f64::MAX, f64::min), ys.iter().cloned().fold(f64::MIN, f64::max))
                })
                .collect()
        };
        let whole = ranges(&layer(&base, &v.segments));
        for (near, far) in [(20_000.0, 40_000.0), (0.0, 20_000.0)] {
            let (yn, yf) = (y(near), y(far));
            let want: Vec<(f64, f64)> = whole.iter().map(|&(lo, hi)| (lo.max(yf), hi.min(yn))).filter(|(lo, hi)| hi - lo > 1e-6).collect();
            let got = ranges(&layer(&base.over(Reach { near, far }), &v.segments));
            assert_eq!(got.len(), want.len(), "{near}: {got:?} {want:?}");
            for (g, w) in got.iter().zip(&want) {
                assert!((g.0 - w.0).abs() < 0.02 && (g.1 - w.1).abs() < 0.02, "{near}: {g:?} {w:?}");
            }
        }
    }

    #[test]
    fn a_symbol_follows_its_lane_along_the_taper() {
        use crate::street::iso::projection::Reach;
        use crate::street::model::{Change, Taper};
        let v = Editor::new(0).view();
        let lane = &v.segments[2];
        let (x0, x1) = (lane.x_mm, lane.x_mm + lane.width_mm);
        // the lane lies 3.5 m farther across at the seam (the far end of the run)
        let t = Taper {
            host_is_selected: true,
            length_mm: 50_000,
            changes: vec![Change { kind: lane.base_kind, before: Some((x0, x1)), after: Some((x0 + 3_500, x1 + 3_500)) }],
        };
        let draw = Draw::new(&t, true);
        let p = fit(&v, 1050.0).over(Reach { near: 0.0, far: 20_000.0 });
        let m = layer_markup(&crate::street::iso::scene::markings(&p, &v.segments, Some(&draw)), &p);
        let (e0, e1) = draw.edges(draw.shape_of(lane).unwrap(), p.reach, ARROW_AT_MM);
        assert!(e0 > f64::from(x0) + 1_000.0, "the lane has moved by the arrow's distance");
        let toward = lane.direction == Some("toward");
        let arrow = &shapes(Symbol::Ahead { toward }, e1 - e0)[0];
        let pts = points(&p, &arrow.iter().map(|&(u, a)| ((e0 + e1) / 2.0 + u, 0.0, ARROW_AT_MM + a)).collect::<Vec<_>>());
        assert!(
            m.contains(&format!("<g class=\"iso-mark\" data-mark=\"{}\"><polygon class=\"iso-paint white\" points=\"{pts}\"/></g>", lane.uid)),
            "the arrow lies within the shifted lane"
        );
    }
}
