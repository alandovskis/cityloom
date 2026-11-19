//! The plan: the whole street seen from above, first end on the left, drawn to scale along its length (`scale`).
//! Each run is a stack of bands, one per piece, in the section's order top to bottom, as wide across as the piece is
//! at one scale across for the whole plan (the widest run fills `BANDS_H`); along the street the scale is the ground's,
//! so a change along the street is drawn where it happens. What stands on a side (a curbside use, a stop) is drawn
//! in a lane beside the bands on that side; a raised element across the roadway's bands of the run where it starts,
//! a green element on the band of the piece it stands on there. A boundary between runs is a line across the bands, with its taper drawn at its true length
//! and cut at the street's ends; the picked stretch is a tint with a handle at each end; an end cut by the edge of the
//! map is a zigzag.
//!
//! Like the 2.5D it is a pure function returning SVG markup (`plan_svg`); what a press on it means is the
//! view-model's.

mod scale;

pub use scale::{MIN_PX_PER_M, mm_per_px, x_px};

use crate::shared::catalogue::{KINDS, MATERIALS, Side, is_roadway};
use crate::shared::i18n::{Args, I18n};
use crate::shared::iso::format::{esc, f1};
use crate::shared::said::say;
use crate::shared::units::Units;
use crate::street::model::{PlanObject, PlanRun, PlanThing, PlanView};
use crate::street::vm::row::{CalmingRow, CurbRow, StopRow, cell_id};

/// The height of the lane beside the bands on each side, where what stands on that side is drawn: a target high.
const LANE_H: f64 = 24.0;
/// The height the widest run's bands fill.
const BANDS_H: f64 = 96.0;
/// The whole plan's height: the bands with a lane above and below.
const HEIGHT: f64 = BANDS_H + 2.0 * LANE_H;
/// How far a handle reaches past the bands, above and below, and how wide it is.
const HANDLE_OVER: f64 = 4.0;
const HANDLE_W: f64 = 6.0;
/// The zigzag of a cut end: how deep it is along the street and how long each of its strokes is across.
const CUT_DEPTH: f64 = 6.0;
const CUT_STEP: f64 = 8.0;
/// The gap between a thing in a lane and the lane's edges.
const LANE_GAP: f64 = 2.0;

/// The plan drawn `width_px` wide, or wider for a street too long to fit at `MIN_PX_PER_M`. A plan with no length
/// (no end to measure from) is schematic: its one run fills the width and both ends are marked cut.
pub fn plan_svg(p: &PlanView, width_px: f64, i18n: &I18n, units: Units) -> String {
    let at = Along::of(p, width_px);
    let width = at.width;
    let mut out = format!(
        "<svg class=\"plan\" xmlns=\"http://www.w3.org/2000/svg\"{} width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        p.length_mm.map_or(String::new(), |l| format!(" data-length-mm=\"{l}\"")),
        w = f1(width),
        h = f1(HEIGHT),
    );
    let across = BANDS_H / p.runs.iter().map(run_total).max().unwrap_or(0).max(1) as f64;
    let stacks: Vec<Vec<(f64, f64)>> = p.runs.iter().map(|r| stack(r, across)).collect();
    for (r, ys) in p.runs.iter().zip(&stacks) {
        out.push_str(&run(r, ys, at.span(r.from_mm, r.to_mm)));
    }
    let pick = p.pick.and_then(|(a, b)| at.cut_span(a, b));
    if let Some((from, to)) = pick {
        let (y, h) = (f1(LANE_H), f1(BANDS_H));
        let (x, w) = (f1(from), f1(to - from));
        out.push_str(&format!("<rect class=\"plan-pick-casing\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\"/>"));
        out.push_str(&format!("<rect class=\"plan-pick\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\"/>"));
    }
    for b in &p.boundaries {
        out.push_str(&format!("<g class=\"plan-boundary\" data-zone=\"{}\">", b.zone));
        if let Some((from, to)) = b.taper.and_then(|(a, z)| at.cut_span(a, z)) {
            for class in ["plan-taper-casing", "plan-taper"] {
                out.push_str(&format!(
                    "<rect class=\"{class}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                    f1(from),
                    f1(LANE_H),
                    f1(to - from),
                    f1(BANDS_H)
                ));
            }
        }
        let x = f1(at.x(b.at_mm));
        let (y1, y2) = (f1(LANE_H), f1(LANE_H + BANDS_H));
        for class in ["plan-boundary-casing", "plan-boundary-line"] {
            out.push_str(&format!("<line class=\"{class}\" x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\"/>"));
        }
        out.push_str("</g>");
    }
    for o in &p.objects {
        if let Some(span) = at.cut_span(o.from_mm, o.to_mm) {
            out.push_str(&thing(o, span, across_of(o, p, &stacks), i18n, units));
        }
    }
    if let Some((from, to)) = pick {
        for (end, x) in [("left", from), ("right", to)] {
            out.push_str(&format!(
                "<rect class=\"plan-handle\" data-end=\"{end}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\"/>",
                f1(x - HANDLE_W / 2.0),
                f1(LANE_H - HANDLE_OVER),
                f1(HANDLE_W),
                f1(BANDS_H + 2.0 * HANDLE_OVER)
            ));
        }
    }
    let cut_title = esc(&i18n.tr("plan-cut", &Args::new()));
    for (end, cut) in ["left", "right"].into_iter().zip(p.cut) {
        if cut {
            let d = zigzag(if end == "left" { 0.0 } else { width - CUT_DEPTH });
            out.push_str(&format!("<g class=\"plan-cut\" data-end=\"{end}\"><title>{cut_title}</title>"));
            for class in ["plan-cut-casing", "plan-cut-line"] {
                out.push_str(&format!("<path class=\"{class}\" d=\"{d}\"/>"));
            }
            out.push_str("</g>");
        }
    }
    out.push_str("</svg>");
    out
}

/// Where along the plan a position lies.
struct Along {
    /// The plan's length, `None` when it is schematic.
    length_mm: Option<i32>,
    mm_per_px: f64,
    width: f64,
}

impl Along {
    fn of(p: &PlanView, width_px: f64) -> Along {
        match p.length_mm {
            Some(length) => {
                let mm_per_px = mm_per_px(length, width_px);
                Along { length_mm: Some(length), mm_per_px, width: x_px(length, mm_per_px) }
            }
            None => Along { length_mm: None, mm_per_px: 1.0, width: width_px },
        }
    }

    fn x(&self, at_mm: i32) -> f64 {
        x_px(at_mm, self.mm_per_px)
    }

    /// A run's span in pixels: where it lies, or the whole width on a schematic plan, whatever its positions.
    fn span(&self, from_mm: i32, to_mm: i32) -> (f64, f64) {
        match self.length_mm {
            Some(_) => (self.x(from_mm), self.x(to_mm)),
            None => (0.0, self.width),
        }
    }

    /// The span from `from_mm` to `to_mm` in pixels, cut to the street's ends; `None` when nothing of it is on the
    /// street, or the plan is schematic (it has no positions).
    fn cut_span(&self, from_mm: i32, to_mm: i32) -> Option<(f64, f64)> {
        let length = self.length_mm?;
        let (from, to) = (from_mm.max(0), to_mm.min(length));
        (from < to).then(|| (self.x(from), self.x(to)))
    }
}

/// How wide a run's section is across.
fn run_total(r: &PlanRun) -> i32 {
    r.bands.iter().map(|b| b.width_mm).sum()
}

/// Each of a run's bands as `(y, height)`, `across` px to the millimetre across, the stack centred on the plan's centre
/// line.
fn stack(r: &PlanRun, across: f64) -> Vec<(f64, f64)> {
    let mut y = LANE_H + (BANDS_H - run_total(r) as f64 * across) / 2.0;
    r.bands
        .iter()
        .map(|b| {
            let h = b.width_mm as f64 * across;
            y += h;
            (y - h, h)
        })
        .collect()
}

/// A run's bands, laid across as `ys`, over `(from, to)` px.
fn run(r: &PlanRun, ys: &[(f64, f64)], (from, to): (f64, f64)) -> String {
    let mut out = format!("<g class=\"plan-run\" data-zone=\"{}\">", r.zone);
    for (b, &(y, h)) in r.bands.iter().zip(ys) {
        out.push_str(&format!(
            "<rect class=\"plan-band k-{} mat-{}\" data-uid=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
            KINDS[b.kind].id,
            MATERIALS[b.material].id,
            b.uid,
            f1(from),
            f1(y),
            f1(to - from),
            f1(h)
        ));
    }
    out.push_str("</g>");
    out
}

/// The id of the cell of its row a thing is, so the view can find the cell a press on it means.
fn thing_id(t: PlanThing) -> String {
    match t {
        PlanThing::Curb { side, cell } => cell_id::<CurbRow>(side, cell),
        PlanThing::Stop { side, cell } => cell_id::<StopRow>(side, cell),
        PlanThing::Calming { cell } => CalmingRow::cell_id(cell),
        PlanThing::Green { i } => format!("green-{i}"),
    }
}

/// Where a thing lies across, as `(y, height)`: in the lane on its side; on its host's band, or across the roadway's
/// bands for what lies across the street, in the run where it starts; else over all the bands.
fn across_of(o: &PlanObject, p: &PlanView, stacks: &[Vec<(f64, f64)>]) -> (f64, f64) {
    match o.side {
        Some(Side::Left) => return (LANE_GAP, LANE_H - 2.0 * LANE_GAP),
        Some(Side::Right) => return (LANE_H + BANDS_H + LANE_GAP, LANE_H - 2.0 * LANE_GAP),
        None => {}
    }
    let whole = (LANE_H, BANDS_H);
    let Some(k) = p.runs.iter().position(|r| r.from_mm <= o.from_mm && o.from_mm < r.to_mm) else { return whole };
    let laid = || p.runs[k].bands.iter().zip(&stacks[k]);
    match (o.thing, o.host) {
        (PlanThing::Green { .. }, Some(uid)) => laid().find(|(b, _)| b.uid == uid).map_or(whole, |(_, &ys)| ys),
        (PlanThing::Calming { .. }, _) => {
            let road: Vec<(f64, f64)> = laid().filter(|(b, _)| is_roadway(b.kind)).map(|(_, &ys)| ys).collect();
            match (road.first(), road.last()) {
                (Some(&(top, _)), Some(&(y, h))) => (top, y + h - top),
                _ => whole,
            }
        }
        _ => whole,
    }
}

/// A thing over `(from, to)` px and `(y, h)` across, named by its label.
fn thing(o: &PlanObject, (from, to): (f64, f64), (y, h): (f64, f64), i18n: &I18n, units: Units) -> String {
    let rect = |class: &str| format!("<rect class=\"{class}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>", f1(from), f1(y), f1(to - from), f1(h));
    format!(
        "<g class=\"{}\" data-thing=\"{}\"><title>{}</title>{}{}</g>",
        if o.chosen { "plan-thing plan-chosen" } else { "plan-thing" },
        thing_id(o.thing),
        esc(&say(i18n, units, &o.label)),
        rect("plan-thing-casing"),
        rect("plan-thing-mark"),
    )
}

/// The zigzag marking a cut end, `CUT_DEPTH` deep from `x` across the bands.
fn zigzag(x: f64) -> String {
    let steps = (BANDS_H / CUT_STEP) as usize;
    let mut d = format!("M{} {}", f1(x), f1(LANE_H));
    for i in 1..=steps {
        let at = if i % 2 == 1 { x + CUT_DEPTH } else { x };
        d.push_str(&format!(" L{} {}", f1(at), f1(LANE_H + i as f64 * CUT_STEP)));
    }
    d
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::shared::catalogue::kind_index;
    use crate::shared::i18n::{Args, Locale};
    use crate::shared::said::{Arg, Said, say};
    use crate::street::model::{PlanBand, PlanBoundary, PlanObject, PlanRun, PlanThing};

    const LEN: i32 = 60_000;
    /// Drawn 600 px wide, a 60 m street is 100 mm to the pixel.
    const WIDTH: f64 = 600.0;

    fn en() -> Rc<I18n> {
        crate::i18n_for(Locale::En)
    }

    fn band(uid: u32, kind: &str, material: &str, width_mm: i32) -> PlanBand {
        let material = MATERIALS.iter().position(|m| m.id == material).expect("a material");
        PlanBand { uid, kind: kind_index(kind).expect("a kind"), material, width_mm }
    }

    /// A sidewalk, a travel lane and a sidewalk; with `bike`, a bike lane before the last sidewalk.
    fn section(bike: bool) -> Vec<PlanBand> {
        let mut bands = vec![band(1, "sidewalk", "concrete", 2_000), band(2, "travel", "asphalt", 3_300)];
        if bike {
            bands.push(band(4, "bike", "asphalt", 1_800));
        }
        bands.push(band(3, "sidewalk", "concrete", 2_000));
        bands
    }

    fn run(from_mm: i32, to_mm: i32, zone: usize, bike: bool) -> PlanRun {
        PlanRun { from_mm, to_mm, zone, bands: section(bike), ok: true }
    }

    fn boundary(at_mm: i32, zone: usize, taper: Option<(i32, i32)>) -> PlanBoundary {
        PlanBoundary { at_mm, zone, taper, movable: true }
    }

    /// A street `LEN` long, a bike lane added from 20 m on.
    fn two_runs() -> PlanView {
        PlanView {
            length_mm: Some(LEN),
            runs: vec![run(0, 20_000, 0, false), run(20_000, LEN, 1, true)],
            boundaries: vec![boundary(20_000, 1, None)],
            objects: Vec::new(),
            pick: None,
            cut: [false, false],
            elsewhere: false,
        }
    }

    fn drawn(p: &PlanView) -> String {
        plan_svg(p, WIDTH, &en(), Units::Metres)
    }

    /// Every tag whose class list starts with `class`, as written.
    fn tags<'a>(markup: &'a str, class: &str) -> Vec<&'a str> {
        let mut out = Vec::new();
        for (at, _) in markup.match_indices(&format!("class=\"{class}")) {
            let rest = &markup[at + 7 + class.len()..];
            if !rest.starts_with('"') && !rest.starts_with(' ') {
                continue;
            }
            let start = markup[..at].rfind('<').expect("in a tag");
            out.push(&markup[start..at + markup[at..].find('>').expect("a closed tag") + 1]);
        }
        out
    }

    /// The attribute `name` of `tag`.
    fn attr<'a>(tag: &'a str, name: &str) -> &'a str {
        let at = tag.find(&format!(" {name}=\"")).unwrap_or_else(|| panic!("{name} in {tag}")) + name.len() + 3;
        &tag[at..at + tag[at..].find('"').unwrap()]
    }

    fn num(tag: &str, name: &str) -> f64 {
        attr(tag, name).parse().unwrap_or_else(|_| panic!("{name} of {tag} is a number"))
    }

    /// The markup of the run group that starts the zone `zone`.
    fn run_group(markup: &str, zone: usize) -> &str {
        let open = format!("<g class=\"plan-run\" data-zone=\"{zone}\"");
        let at = markup.find(&open).unwrap_or_else(|| panic!("the run of zone {zone}"));
        &markup[at..at + markup[at..].find("</g>").unwrap()]
    }

    #[test]
    fn one_band_per_piece_per_run() {
        let svg = drawn(&two_runs());
        assert!(svg.starts_with("<svg ") && attr(&svg[..svg.find('>').unwrap()], "class") == "plan", "{svg}");
        assert_eq!(attr(&svg[..svg.find('>').unwrap()], "data-length-mm"), "60000");
        assert_eq!(tags(&svg, "plan-run").len(), 2);
        assert_eq!(tags(&svg, "plan-band").len(), 3 + 4);
        let first = tags(run_group(&svg, 0), "plan-band");
        let classes: Vec<&str> = first.iter().map(|t| attr(t, "class")).collect();
        assert_eq!(classes, ["plan-band k-sidewalk mat-concrete", "plan-band k-travel mat-asphalt", "plan-band k-sidewalk mat-concrete"]);
        let second = tags(run_group(&svg, 1), "plan-band");
        assert_eq!(second.iter().map(|t| attr(t, "data-uid")).collect::<Vec<_>>(), ["1", "2", "4", "3"], "in the section's order");
        assert_eq!(attr(second[2], "class"), "plan-band k-bike mat-asphalt");
        // Top to bottom, each band as wide across as its piece is, at one scale for every run.
        let ys = |bands: &[&str]| bands.iter().map(|t| (num(t, "y"), num(t, "height"))).collect::<Vec<_>>();
        for bands in [&first, &second] {
            let ys = ys(bands);
            for pair in ys.windows(2) {
                assert!((pair[0].0 + pair[0].1 - pair[1].0).abs() < 0.11, "{ys:?}");
            }
        }
        let (h_side, h_travel) = (num(first[0], "height"), num(first[1], "height"));
        assert!((h_side / h_travel - 2_000.0 / 3_300.0).abs() < 0.01, "{h_side} {h_travel}");
        assert_eq!(num(second[0], "height"), h_side);
    }

    #[test]
    fn a_run_starts_where_its_left_boundary_is_in_px() {
        let svg = drawn(&two_runs());
        let b = tags(&svg, "plan-boundary");
        assert_eq!(b.len(), 1);
        assert_eq!(attr(b[0], "data-zone"), "1");
        let line = tags(&svg, "plan-boundary-line");
        assert_eq!(num(line[0], "x1"), 200.0);
        assert_eq!(num(line[0], "x2"), 200.0);
        for t in tags(run_group(&svg, 0), "plan-band") {
            assert_eq!((num(t, "x"), num(t, "width")), (0.0, 200.0));
        }
        for t in tags(run_group(&svg, 1), "plan-band") {
            assert_eq!((num(t, "x"), num(t, "width")), (200.0, 400.0));
        }
        let root = &svg[..svg.find('>').unwrap()];
        assert_eq!(attr(root, "width"), "600");
    }

    #[test]
    fn the_pick_is_drawn_with_two_handles() {
        let p = PlanView { pick: Some((10_000, 30_000)), ..two_runs() };
        let svg = drawn(&p);
        let pick = tags(&svg, "plan-pick");
        assert_eq!(pick.len(), 1, "{svg}");
        assert_eq!((num(pick[0], "x"), num(pick[0], "width")), (100.0, 200.0));
        let handles = tags(&svg, "plan-handle");
        assert_eq!(handles.len(), 2);
        let centre = |t: &str| num(t, "x") + num(t, "width") / 2.0;
        assert_eq!(handles.iter().map(|t| (attr(t, "data-end"), centre(t))).collect::<Vec<_>>(), [("left", 100.0), ("right", 300.0)]);
    }

    #[test]
    fn no_pick_draws_no_handles() {
        let svg = drawn(&two_runs());
        assert!(tags(&svg, "plan-pick").is_empty());
        assert!(tags(&svg, "plan-handle").is_empty());
    }

    #[test]
    fn a_taper_is_drawn_at_its_true_length() {
        let p = PlanView { boundaries: vec![boundary(20_000, 1, Some((20_000, 33_300)))], ..two_runs() };
        let svg = drawn(&p);
        let taper = tags(&svg, "plan-taper");
        assert_eq!(taper.len(), 1);
        assert_eq!((num(taper[0], "x"), num(taper[0], "width")), (200.0, 133.0));
        let group = &svg[svg.find("<g class=\"plan-boundary\"").unwrap()..];
        assert!(group[..group.find("</g>").unwrap()].contains("class=\"plan-taper\""), "the taper is drawn with its boundary");
        // With no taper, none is drawn.
        assert!(tags(&drawn(&two_runs()), "plan-taper").is_empty());
    }

    #[test]
    fn a_taper_past_an_end_of_the_street_is_cut_at_that_end() {
        let p = PlanView { boundaries: vec![boundary(20_000, 1, Some((-3_300, 20_000)))], ..two_runs() };
        let taper = tags(&drawn(&p), "plan-taper").into_iter().map(|t| (num(t, "x"), num(t, "width"))).collect::<Vec<_>>();
        assert_eq!(taper, [(0.0, 200.0)]);
        let p = PlanView { boundaries: vec![boundary(50_000, 1, Some((50_000, 103_300)))], ..two_runs() };
        let taper = tags(&drawn(&p), "plan-taper").into_iter().map(|t| (num(t, "x"), num(t, "width"))).collect::<Vec<_>>();
        assert_eq!(taper, [(500.0, 100.0)]);
    }

    fn thing(thing: PlanThing, from_mm: i32, to_mm: i32, side: Option<Side>, label: Said, chosen: bool) -> PlanObject {
        PlanObject { thing, from_mm, to_mm, side, label, chosen, host: None }
    }

    #[test]
    fn things_carry_their_cell_ids() {
        let label = Said::new("plan-cut");
        let named = Said::new("st-path-step").with("name", Arg::Text("Rue <A> & B".to_string()));
        let p = PlanView {
            objects: vec![
                thing(PlanThing::Curb { side: Side::Left, cell: 0 }, 10_000, 16_000, Some(Side::Left), named.clone(), false),
                thing(PlanThing::Calming { cell: 2 }, 20_000, 27_000, None, label.clone(), true),
                thing(PlanThing::Stop { side: Side::Right, cell: 1 }, 30_000, 42_000, Some(Side::Right), label.clone(), false),
                thing(PlanThing::Green { i: 3 }, 45_000, 46_000, None, label.clone(), false),
            ],
            ..two_runs()
        };
        let i18n = en();
        let svg = plan_svg(&p, WIDTH, &i18n, Units::Metres);
        let groups = tags(&svg, "plan-thing");
        assert_eq!(groups.iter().map(|t| attr(t, "data-thing")).collect::<Vec<_>>(), ["curb-left-0", "calming-2", "stop-right-1", "green-3"]);
        assert_eq!(attr(groups[1], "class"), "plan-thing plan-chosen", "the chosen thing is marked");
        let marks = tags(&svg, "plan-thing-mark");
        assert_eq!(marks.iter().map(|t| (num(t, "x"), num(t, "width"))).collect::<Vec<_>>(), [(100.0, 60.0), (200.0, 70.0), (300.0, 120.0), (450.0, 10.0)]);
        // A curbside use on the left lies above the bands, a stop on the right below them, what lies across the
        // street over them.
        let bands = tags(&svg, "plan-band");
        let top = bands.iter().map(|t| num(t, "y")).fold(f64::INFINITY, f64::min);
        let bottom = bands.iter().map(|t| num(t, "y") + num(t, "height")).fold(0.0, f64::max);
        assert!(num(marks[0], "y") + num(marks[0], "height") <= top);
        assert!(num(marks[2], "y") >= bottom);
        for m in [marks[1], marks[3]] {
            assert!(num(m, "y") >= top && num(m, "y") + num(m, "height") <= bottom + 0.11);
        }
        // Each is named by its label, escaped.
        let said = say(&i18n, Units::Metres, &named);
        assert!(said.contains("<A> & B"), "{said}");
        let escaped = said.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        assert!(svg.contains(&format!("<title>{escaped}</title>")), "{svg}");
        assert!(!svg.contains("<A>"));
    }

    /// The `(top, bottom)` of band `uid` in the run of zone `zone`.
    fn band_ys(svg: &str, zone: usize, uid: u32) -> (f64, f64) {
        let band = tags(run_group(svg, zone), "plan-band").into_iter().find(|t| attr(t, "data-uid") == uid.to_string()).expect("the band");
        (num(band, "y"), num(band, "y") + num(band, "height"))
    }

    fn mark_ys(svg: &str, n: usize) -> (f64, f64) {
        let mark = tags(svg, "plan-thing-mark")[n];
        (num(mark, "y"), num(mark, "y") + num(mark, "height"))
    }

    #[test]
    fn a_green_element_lies_on_the_piece_it_stands_on() {
        // A tree on the right sidewalk (uid 3) of the first run.
        let tree = PlanObject { host: Some(3), ..thing(PlanThing::Green { i: 0 }, 5_000, 6_000, None, Said::new("plan-cut"), false) };
        let svg = drawn(&PlanView { objects: vec![tree], ..two_runs() });
        let (top, bottom) = mark_ys(&svg, 0);
        let (side_top, side_bottom) = band_ys(&svg, 0, 3);
        assert!(top >= side_top - 0.051 && bottom <= side_bottom + 0.051, "{top}..{bottom} on {side_top}..{side_bottom}");
        let (_, travel_bottom) = band_ys(&svg, 0, 2);
        assert!(top >= travel_bottom - 0.051, "not over the travel lane");
        // The same tree from 30 m on lies on the second run's right sidewalk, below its bike lane.
        let tree = PlanObject { host: Some(3), ..thing(PlanThing::Green { i: 0 }, 30_000, 31_000, None, Said::new("plan-cut"), false) };
        let svg = drawn(&PlanView { objects: vec![tree], ..two_runs() });
        let (top, _) = mark_ys(&svg, 0);
        assert!((top - band_ys(&svg, 1, 3).0).abs() < 0.11);
    }

    #[test]
    fn a_raised_element_spans_the_roadway_of_its_run_and_not_the_sidewalks() {
        let table = thing(PlanThing::Calming { cell: 0 }, 30_000, 37_000, None, Said::new("plan-cut"), false);
        let svg = drawn(&PlanView { objects: vec![table], ..two_runs() });
        let (top, bottom) = mark_ys(&svg, 0);
        // The travel lane and the bike lane are the roadway; the sidewalks either side are not.
        assert!((top - band_ys(&svg, 1, 2).0).abs() < 0.11, "{top}");
        assert!((bottom - band_ys(&svg, 1, 4).1).abs() < 0.11, "{bottom}");
    }

    #[test]
    fn a_schematic_plan_has_no_length_attribute_and_two_cut_marks() {
        let p = PlanView { length_mm: None, runs: vec![run(0, 0, 2, true)], boundaries: Vec::new(), cut: [true, true], ..two_runs() };
        let i18n = en();
        let svg = plan_svg(&p, 500.0, &i18n, Units::Metres);
        assert!(!svg.contains("data-length-mm"));
        assert_eq!(attr(&svg[..svg.find('>').unwrap()], "width"), "500");
        let bands = tags(&svg, "plan-band");
        assert_eq!(bands.len(), 4);
        assert!(bands.iter().all(|t| (num(t, "x"), num(t, "width")) == (0.0, 500.0)), "the shown run across the whole width");
        let cuts = tags(&svg, "plan-cut");
        assert_eq!(cuts.iter().map(|t| attr(t, "data-end")).collect::<Vec<_>>(), ["left", "right"]);
        assert!(svg.contains(&format!("<title>{}</title>", i18n.tr("plan-cut", &Args::new()))));
    }

    #[test]
    fn only_a_cut_end_is_marked() {
        assert!(tags(&drawn(&two_runs()), "plan-cut").is_empty());
        let svg = drawn(&PlanView { cut: [false, true], ..two_runs() });
        assert_eq!(tags(&svg, "plan-cut").iter().map(|t| attr(t, "data-end")).collect::<Vec<_>>(), ["right"]);
    }

    #[test]
    fn a_long_street_keeps_the_minimum_scale() {
        let long = 2_000_000;
        let p = PlanView { length_mm: Some(long), runs: vec![run(0, long, 0, false)], boundaries: Vec::new(), ..two_runs() };
        assert_eq!(mm_per_px(long, 375.0), 1000.0 / MIN_PX_PER_M);
        let svg = plan_svg(&p, 375.0, &en(), Units::Metres);
        let width: f64 = attr(&svg[..svg.find('>').unwrap()], "width").parse().unwrap();
        assert!((width - 2_000.0 * MIN_PX_PER_M).abs() < 0.1, "drawn wider than the column, to scroll inside its box: {width}");
        assert_eq!(num(tags(&svg, "plan-band")[0], "width"), width);
    }
}
