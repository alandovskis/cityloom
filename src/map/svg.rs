//! The city as SVG markup, from the city's view: every street and junction as a
//! place on the map, drawn in metres, and whether it works. Widths that stay a
//! few pixels wide are set in metres from the zoom, so a new zoom draws again.

use std::collections::HashMap;
use std::fmt::Write;

use crate::city::model::{CityView, EdgeView, NodeView};
use crate::map::vm::{junction_label, street_label};
use crate::shared::symbols::HATCH;
use crate::shared::units::Units;

/// Pieces beside the roadway, not part of it.
const OFF_ROAD: [&str; 7] = ["sidewalk", "planting", "bikerack", "bikeshare", "pole", "busshelter", "busstation"];

/// A number to two places, with no trailing zeros.
fn r2(n: f64) -> String {
    let r = (n * 100.0).round() / 100.0;
    if r == 0.0 { "0".to_string() } else { format!("{r}") }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Hatch for the map: the same textures as the section, kept to the same size on
/// screen at every zoom, under their own names.
pub fn map_hatch(px: f64) -> String {
    let t = format!("scale({})", r2(px));
    HATCH
        .iter()
        .map(|(kind, svg)| {
            let renamed = svg.replace(&format!("id=\"h-{kind}\""), &format!("id=\"mh-{kind}\""));
            if renamed.contains("patternTransform=\"") {
                renamed.replace("patternTransform=\"", &format!("patternTransform=\"{t} "))
            } else {
                renamed.replacen("<pattern ", &format!("<pattern patternTransform=\"{t}\" "), 1)
            }
        })
        .collect()
}

/// A street laid on the map: where it starts and which way it runs.
struct Geometry {
    ax: f64,
    ay: f64,
    ux: f64,
    uy: f64,
    nx: f64,
    ny: f64,
    len: f64,
}

impl Geometry {
    fn of(e: &EdgeView, nodes: &HashMap<u32, &NodeView>) -> Option<Geometry> {
        let (a, b) = (nodes.get(&e.a)?, nodes.get(&e.b)?);
        let (ax, ay) = (a.x_mm as f64 / 1000.0, a.y_mm as f64 / 1000.0);
        let (dx, dy) = (b.x_mm as f64 / 1000.0 - ax, b.y_mm as f64 / 1000.0 - ay);
        let len = dx.hypot(dy);
        let (ux, uy) = (dx / len, dy / len);
        Some(Geometry { ax, ay, ux, uy, nx: -uy, ny: ux, len })
    }

    /// A point `t` along the street and `off` metres to its right.
    fn at(&self, off: f64, t: f64) -> (f64, f64) {
        (self.ax + self.ux * t + self.nx * off, self.ay + self.uy * t + self.ny * off)
    }

    /// Attributes of a line along the street, shifted `off` metres to its right, from `t0` to `t1`.
    fn line(&self, off: f64, t0: f64, t1: f64) -> String {
        let (a, b) = (self.at(off, t0), self.at(off, t1));
        format!("x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"", r2(a.0), r2(a.1), r2(b.0), r2(b.1))
    }
}

/// The narrowest a street is drawn, in pixels.
const MIN_STREET_PX: f64 = 16.0;

/// A street ready to draw: its geometry, width, where its road lies and where its details run.
struct Laid<'a> {
    e: &'a EdgeView,
    g: Geometry,
    row: f64,
    road_off: f64,
    road_w: f64,
    /// How much wider than true the street is drawn, so that it can be seen from far off.
    boost: f64,
    t0: f64,
    t1: f64,
}

fn badge(x: f64, y: f64, px: &dyn Fn(f64) -> f64) -> String {
    format!(
        "<g class=\"m-badge\" transform=\"translate({} {})\"><circle r=\"{}\" stroke-width=\"{}\"/><path d=\"M{} {}l{} {}m0 {}l{} {}\" stroke-width=\"{}\"/></g>",
        r2(x),
        r2(y),
        r2(px(8.0)),
        r2(px(2.0)),
        r2(-px(3.0)),
        r2(-px(3.0)),
        r2(px(6.0)),
        r2(px(6.0)),
        r2(-px(6.0)),
        r2(-px(6.0)),
        r2(px(6.0)),
        r2(px(2.0))
    )
}

/// The whole map at zoom `k` (pixels to the metre).
pub fn map_svg(v: &CityView, k: f64, units: Units) -> String {
    let px = |n: f64| n / k;
    let nodes: HashMap<u32, &NodeView> = v.nodes.iter().map(|n| (n.uid, n)).collect();
    let laid: Vec<Laid> = v
        .edges
        .iter()
        .filter_map(|e| {
            let g = Geometry::of(e, &nodes)?;
            let road: Vec<&crate::city::model::PieceView> = e.pieces.iter().filter(|p| !OFF_ROAD.contains(&p.kind)).collect();
            let lo = road.iter().map(|p| (p.offset_mm - p.width_mm / 2) as f64).fold(f64::MAX, f64::min) / 1000.0;
            let hi = road.iter().map(|p| (p.offset_mm + p.width_mm / 2) as f64).fold(f64::MIN, f64::max) / 1000.0;
            let t0 = e.trim_a_mm as f64 / 1000.0;
            let t1 = g.len - e.trim_b_mm as f64 / 1000.0;
            // A street is never drawn narrower than this on screen, whatever the zoom.
            let row = e.row_mm as f64 / 1000.0;
            let boost = (px(MIN_STREET_PX) / row).max(1.0);
            Some(Laid { e, g, row: row * boost, road_off: (lo + hi) / 2.0 * boost, road_w: (hi - lo) * boost, boost, t0, t1 })
        })
        .collect();
    let mut places: Vec<&NodeView> = v.nodes.iter().filter(|n| n.junction).collect();
    places.sort_by_key(|n| n.number);
    let rad = |n: &NodeView| n.radius_mm as f64 / 1000.0;
    let mut s = String::new();

    write!(s, "<defs>{}</defs>", map_hatch(1.0 / k)).unwrap();
    for l in laid.iter().filter(|l| !l.e.ok) {
        write!(s, "<line class=\"m-bad\" {} stroke-width=\"{}\"/>", l.g.line(0.0, 0.0, l.g.len), r2(l.row + px(7.0))).unwrap();
    }
    for n in places.iter().filter(|n| !n.ok) {
        write!(s, "<circle class=\"m-bad-disc\" cx=\"{}\" cy=\"{}\" r=\"{}\"/>", n.x_mm as f64 / 1000.0, n.y_mm as f64 / 1000.0, r2(rad(n) + px(4.0))).unwrap();
    }
    for l in &laid {
        write!(s, "<line class=\"m-out\" {} stroke-width=\"{}\"/>", l.g.line(0.0, 0.0, l.g.len), r2(l.row)).unwrap();
    }
    for l in &laid {
        write!(
            s,
            "<line class=\"{}\" {} stroke-width=\"{}\"/>",
            if l.e.freeway { "m-shoulder" } else { "m-walk" },
            l.g.line(0.0, 0.0, l.g.len),
            r2(l.row - px(3.0))
        )
        .unwrap();
    }
    for l in &laid {
        write!(s, "<line class=\"m-road\" {} stroke-width=\"{}\"/>", l.g.line(l.road_off, 0.0, l.g.len), r2(l.road_w)).unwrap();
    }
    // the pieces' tints, then their hatch, along the part of each street not under a junction
    for hatch in [false, true] {
        for l in laid.iter().filter(|l| l.t1 > l.t0) {
            for p in &l.e.pieces {
                let line = l.g.line(p.offset_mm as f64 / 1000.0 * l.boost, l.t0, l.t1);
                if hatch {
                    write!(s, "<line class=\"m-h\" stroke=\"url(#mh-{})\" {line} stroke-width=\"{}\"/>", p.kind, r2(p.width_mm as f64 / 1000.0 * l.boost))
                        .unwrap();
                } else {
                    write!(s, "<line class=\"m-t m-k-{}\" {line} stroke-width=\"{}\"/>", p.kind, r2(p.width_mm as f64 / 1000.0 * l.boost)).unwrap();
                }
            }
        }
    }

    // Names sit beside a street, on the upper side, where it is long enough to hold one.
    for l in laid.iter().filter(|l| (l.t1 - l.t0) * k >= 110.0) {
        let tm = (l.t0 + l.t1) / 2.0;
        let mut ang = l.g.uy.atan2(l.g.ux).to_degrees();
        if !(-90.0..=90.0).contains(&ang) {
            ang += 180.0;
        }
        let (x, y) = l.g.at(0.0, tm);
        write!(
            s,
            "<text class=\"m-name\" transform=\"translate({} {}) rotate({})\" dy=\"{}\" text-anchor=\"middle\" font-size=\"{}\" stroke-width=\"{}\">{}{}</text>",
            r2(x),
            r2(y),
            r2(ang),
            r2(-(l.row / 2.0 + px(6.0))),
            r2(px(13.0)),
            r2(px(4.0)),
            esc(&l.e.kind),
            if l.e.edited { " \u{b7} changed" } else { "" }
        )
        .unwrap();
    }
    for l in laid.iter().filter(|l| !l.e.ok) {
        let (x, y) = l.g.at(0.0, (l.t0 + l.t1) / 2.0);
        s += &badge(x, y, &px);
    }
    for n in &places {
        let (x, y) = (n.x_mm as f64 / 1000.0, n.y_mm as f64 / 1000.0);
        write!(
            s,
            "<g transform=\"translate({x} {y})\"><circle class=\"m-jc\" r=\"{}\" stroke-width=\"{}\"/><text class=\"m-jn\" text-anchor=\"middle\" dy=\"0.35em\" font-size=\"{}\">{}</text>",
            r2(px(11.0)),
            r2(px(1.5)),
            r2(px(13.0)),
            n.number
        )
        .unwrap();
        if n.edited {
            write!(
                s,
                "<text class=\"m-tag\" text-anchor=\"middle\" y=\"{}\" font-size=\"{}\" stroke-width=\"{}\">changed</text>",
                r2(px(26.0)),
                r2(px(12.0)),
                r2(px(4.0))
            )
            .unwrap();
        }
        s += "</g>";
        if !n.ok {
            s += &badge(x + px(12.0), y - px(12.0), &px);
        }
    }

    // Pressing a street or a junction opens it: plain links over the drawing.
    for l in &laid {
        write!(
            s,
            "<a class=\"place\" href=\"{}\" data-hl=\"s-{}\" aria-label=\"{}\"><title>{}</title><line class=\"m-hit\" {} stroke-width=\"{}\"/></a>",
            crate::map::vm::street_href(l.e.uid),
            l.e.uid,
            esc(&street_label(l.e, units)),
            esc(&l.e.name),
            l.g.line(0.0, l.t0, l.t1.max(l.t0 + 0.01)),
            r2(l.row.max(px(18.0)))
        )
        .unwrap();
    }
    for n in &places {
        write!(
            s,
            "<a class=\"place\" href=\"{}\" data-hl=\"j-{}\" aria-label=\"{}\"><title>{}</title><circle class=\"m-hit\" cx=\"{}\" cy=\"{}\" r=\"{}\"/></a>",
            crate::map::vm::junction_href(n.uid),
            n.uid,
            esc(&junction_label(n)),
            esc(&n.name),
            n.x_mm as f64 / 1000.0,
            n.y_mm as f64 / 1000.0,
            r2(rad(n).max(px(16.0)))
        )
        .unwrap();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::city::model::City;

    fn view() -> CityView {
        City::new().view(0)
    }

    /// The stroke widths, in metres, of the lines of one class.
    fn widths(s: &str, class: &str) -> Vec<f64> {
        s.split(&format!("class=\"{class}\""))
            .skip(1)
            .filter_map(|rest| rest.split("stroke-width=\"").nth(1))
            .filter_map(|w| w.split('"').next()?.parse().ok())
            .collect()
    }

    #[test]
    fn a_street_is_never_drawn_narrower_than_the_minimum_on_screen_and_keeps_its_true_width_when_zoomed_in() {
        let v = view();
        let far = 0.3; // pixels to the metre, with the whole city in view
        let narrowest = widths(&map_svg(&v, far, Units::Metres), "m-out").into_iter().fold(f64::MAX, f64::min);
        assert!(narrowest * far >= MIN_STREET_PX - 0.05, "{narrowest}");
        let near = 6.0;
        let true_widths: Vec<f64> = v.edges.iter().map(|e| e.row_mm as f64 / 1000.0).collect();
        let drawn = widths(&map_svg(&v, near, Units::Metres), "m-out");
        assert!(drawn.iter().all(|w| true_widths.iter().any(|t| (t - w).abs() < 0.01)), "unchanged when the street is wide enough to see");
    }

    #[test]
    fn a_boosted_street_keeps_its_lanes_in_proportion() {
        let v = view();
        let far = map_svg(&v, 0.3, Units::Metres);
        let (row, road) = (widths(&far, "m-out")[0], widths(&far, "m-road")[0]);
        let e = &v.edges[0];
        let (lo, hi) = e
            .pieces
            .iter()
            .filter(|p| !OFF_ROAD.contains(&p.kind))
            .fold((i32::MAX, i32::MIN), |(lo, hi), p| (lo.min(p.offset_mm - p.width_mm / 2), hi.max(p.offset_mm + p.width_mm / 2)));
        let true_ratio = (hi - lo) as f64 / e.row_mm as f64;
        assert!((road / row - true_ratio).abs() < 0.02, "{} vs {true_ratio}", road / row);
    }

    #[test]
    fn the_streets_are_named_at_the_zoom_that_fits_the_city_in_a_laptop_window() {
        let v = view();
        let world = crate::map::camera::World::round(v.bounds_mm);
        let camera = crate::map::camera::Camera::new(world, 650.0, 800.0);
        let at_fit = map_svg(&v, camera.k, Units::Metres);
        assert!(count(&at_fit, "class=\"m-name\"") >= 6, "{}", count(&at_fit, "class=\"m-name\""));
    }

    fn count(s: &str, needle: &str) -> usize {
        s.matches(needle).count()
    }

    #[test]
    fn numbers_are_written_to_two_places_with_no_trailing_zeros() {
        assert_eq!(r2(12.0), "12");
        assert_eq!(r2(12.345), "12.35");
        assert_eq!(r2(-0.004), "0");
        assert_eq!(r2(0.5), "0.5");
    }

    #[test]
    fn the_hatches_are_renamed_for_the_map_and_scaled_to_stay_the_same_size_on_screen() {
        let h = map_hatch(0.5);
        assert_eq!(count(&h, "<pattern "), HATCH.len());
        assert!(h.contains("id=\"mh-sidewalk\"") && !h.contains("id=\"h-sidewalk\""));
        // A pattern that already turns is scaled first; one that does not is given the scale.
        assert!(h.contains("patternTransform=\"scale(0.5) rotate(45)\""));
        assert!(h.contains("<pattern patternTransform=\"scale(0.5)\" id=\"mh-sidewalk\""));
    }

    #[test]
    fn every_street_is_drawn_as_an_outline_a_walk_a_road_and_a_link_that_opens_it() {
        let v = view();
        let s = map_svg(&v, 1.0, Units::Metres);
        let n = v.edges.len();
        assert_eq!(count(&s, "class=\"m-out\""), n);
        assert_eq!(count(&s, "class=\"m-walk\"") + count(&s, "class=\"m-shoulder\""), n);
        assert_eq!(count(&s, "class=\"m-road\""), n);
        assert_eq!(count(&s, "data-hl=\"s-"), n);
        assert!(s.contains("href=\"street.html?street="));
        assert!(!s.contains("NaN") && !s.contains("inf"));
    }

    #[test]
    fn a_freeway_has_a_shoulder_where_a_street_has_a_walk() {
        let v = view();
        let s = map_svg(&v, 1.0, Units::Metres);
        assert_eq!(count(&s, "class=\"m-shoulder\""), v.edges.iter().filter(|e| e.freeway).count());
    }

    #[test]
    fn every_junction_is_a_numbered_disc_and_a_link_that_opens_its_plan() {
        let v = view();
        let s = map_svg(&v, 1.0, Units::Metres);
        let j = v.nodes.iter().filter(|n| n.junction).count();
        assert_eq!(count(&s, "class=\"m-jc\""), j);
        assert_eq!(count(&s, "data-hl=\"j-"), j);
        assert!(s.contains("href=\"intersection.html?junction=") && s.contains("class=\"m-jn\""));
    }

    #[test]
    fn what_is_drawn_to_a_few_pixels_is_set_in_metres_from_the_zoom() {
        let v = view();
        let near = map_svg(&v, 10.0, Units::Metres);
        let far = map_svg(&v, 1.0, Units::Metres);
        assert_ne!(near, far);
        // A junction's number is 13 pixels tall at any zoom.
        assert!(near.contains("font-size=\"1.3\"") && far.contains("font-size=\"13\""));
    }

    #[test]
    fn a_street_is_named_where_it_is_long_enough_to_hold_the_name() {
        let v = view();
        let far = map_svg(&v, 0.05, Units::Metres);
        let near = map_svg(&v, 1.0, Units::Metres);
        assert!(count(&near, "class=\"m-name\"") > count(&far, "class=\"m-name\""));
        assert_eq!(count(&far, "class=\"m-name\""), 0);
        assert!(near.contains("rotate("));
    }

    #[test]
    fn a_name_is_never_upside_down() {
        let s = map_svg(&view(), 1.0, Units::Metres);
        for part in s.split("class=\"m-name\" transform=\"").skip(1) {
            let ang: f64 = part.split("rotate(").nth(1).unwrap().split(')').next().unwrap().parse().unwrap();
            assert!((-90.0..=90.0).contains(&ang), "{ang}");
        }
    }

    fn broken_city() -> CityView {
        let mut c = City::new();
        let v = c.view(0);
        let e = v.edges.iter().find(|e| !e.freeway).unwrap().uid;
        let mut ed = c.street_editor(e, 0).unwrap();
        let u = ed.view().segments[0].uid;
        ed.set_width(u, ed.view().segments[0].width_mm + 1_000);
        assert!(c.keep_street(e, ed.snapshot()));
        c.view(0)
    }

    #[test]
    fn a_place_that_no_longer_works_is_ringed_and_badged_and_a_changed_one_says_so() {
        let v = broken_city();
        let s = map_svg(&v, 1.0, Units::Metres);
        let bad = v.edges.iter().filter(|e| !e.ok).count();
        assert!(bad >= 1);
        assert_eq!(count(&s, "class=\"m-bad\""), bad);
        assert_eq!(count(&s, "class=\"m-bad-disc\""), v.nodes.iter().filter(|n| n.junction && !n.ok).count());
        assert_eq!(count(&s, "class=\"m-badge\""), bad + v.nodes.iter().filter(|n| n.junction && !n.ok).count());
        assert!(s.contains("\u{b7} changed") || s.contains(">changed<"));
    }

    #[test]
    fn a_working_city_has_no_rings_and_no_badges() {
        let s = map_svg(&view(), 1.0, Units::Metres);
        for none in ["m-bad", "m-badge", "changed"] {
            assert!(!s.contains(none), "{none}");
        }
    }

    #[test]
    fn the_links_tell_a_screen_reader_what_each_place_is() {
        let v = view();
        let s = map_svg(&v, 1.0, Units::Metres);
        assert!(s.contains("Opens the junction plan.") && s.contains("Opens the street cross-section."));
        let feet = map_svg(&v, 1.0, Units::Feet);
        assert!(feet.contains(" ft wide."));
    }
}
