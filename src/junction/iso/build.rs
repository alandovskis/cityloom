//! The junction's 2.5D built from its view: what is drawn of each part, as shapes in the scene (`scene.rs`), in the order they
//! are laid. A street's pieces, the corners, the ring, the crossings, the posts and the marks on the road, the stops
//! and the measures, the turns of what is selected, and the words.

use super::scene::Scene;
use super::{IsoFrame, circle, content_points, flatten, height_of, hit_attrs, shades};
use crate::junction::iso_marks::{
    LEAVE_ARROW_MM, Spot, crossing_dimension, lane_arrow, lane_arrow_size, movement_strokes, plain_arrow, ring_arrows, spread_labels,
};
use crate::junction::read_model::{ArmView, JView};
use crate::shared::catalogue::KINDS;
use crate::shared::diagram::Diagram;
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::say;
use crate::shared::units::Units;
use crate::street::model::StopKind;

type P = (f64, f64);

/// How tall the things that stand on the ground are.
const SIGNAL_POST_MM: f64 = 2500.0;
const SIGN_POST_MM: f64 = 2000.0;
const BOLLARD_MM: f64 = 900.0;

/// The junction in 2.5D, in a drawing `width` wide in a window `window_height` high. Every part the keys select can
/// be pressed (`hit_attrs`), and the one that is selected is outlined.
pub(super) fn build(v: &JView, i18n: &I18n, width: f64, window_height: f64, units: Units) -> (IsoFrame, Diagram) {
    let f = IsoFrame::fit(&content_points(v), width, window_height);
    let mut d = Scene::new(f);
    let wall = height_of("sidewalk");
    let selected = |kind: &str, uid: u32| v.selected.kind == Some(kind) && v.selected.uid == uid;

    // the carriageway where the streets meet, and a roundabout's ring and island
    if let Some(ring) = &v.ring {
        let track = ring.cycle_mm.is_some_and(|c| c != 0);
        if ring.lanes.is_empty() {
            if track {
                // The cycle track is the band outside the carriageway: the whole ring in its colour, the carriageway over it.
                d.pressing(hit_attrs("cycle", None, None), |d| d.lay("bike", &circle(ring.radius_mm as f64)));
                d.lay("travel", &circle(ring.road_mm as f64));
            } else {
                d.lay("travel", &circle(ring.radius_mm as f64));
            }
            if let Some(bus) = &v.bus {
                let poly = flatten(&bus.poly);
                d.pressing(hit_attrs("bus", None, None), |d| d.lay("bus", &poly));
                if v.selected.kind == Some("bus") {
                    d.outline(&poly, 0.0);
                }
            }
            d.stand("median", circle(ring.island_mm as f64), height_of("median"), true);
            if track && v.selected.kind == Some("cycle") {
                d.outline(&circle(ring.radius_mm as f64), 0.0);
                d.outline(&circle(ring.road_mm as f64), 0.0);
            }
        } else {
            // A turbo: the carriageway (the lanes and the 300 mm between them) is one travel surface, the dividers are paint on it.
            if track {
                d.pressing(hit_attrs("cycle", None, None), |d| d.lay("bike", &ring.outer_edge));
            }
            d.lay("travel", &ring.road_edge);
            for divider in &ring.dividers {
                d.divider(divider);
            }
            d.stand("median", ring.island_edge.clone(), height_of("median"), true);
            if track && v.selected.kind == Some("cycle") {
                d.outline(&ring.outer_edge, 0.0);
                d.outline(&ring.road_edge, 0.0);
            }
        }
    } else {
        d.lay("travel", &flatten(&v.core));
    }
    // Which way the ring runs, and what the bus lane across the middle of a plain one is.
    for a in ring_arrows(v) {
        d.glyph("", &plain_arrow(a.at, a.heading, a.size, RING_HEAD), a.size, 0.0);
    }
    d.ground_laid();
    if v.ring.as_ref().is_some_and(|r| r.lanes.is_empty()) && v.bus.is_some() {
        d.label("t-mark t-halo", (0.0, 0.0), 0.0, 5.0, &i18n.tr("jn-svg-bus-only", &Args::new()));
    }

    for c in &v.corners {
        // A corner that fails its check has its curb drawn in the warning's colour.
        if !c.ok || c.fast {
            d.mark_lines(true, "curb-line bad", true, &[flatten(&c.curb)], if c.walk { wall } else { 0.0 });
        }
        // A straight curb is no corner: nothing of it is pressed.
        let hit = if c.straight { String::new() } else { hit_attrs("corner", Some(c.uid), None) };
        d.pressing(hit.clone(), |d| {
            if c.walk {
                d.stand("sidewalk", flatten(&c.wedge), wall, true);
            }
            if let Some(p) = &c.protected {
                d.lay("bike", &flatten(&p.track));
                d.stand("sidewalk", flatten(&p.island), wall, true);
            }
        });
        if !c.straight {
            if selected("corner", c.uid) {
                d.outline(&flatten(&c.wedge), if c.walk { wall } else { 0.0 });
            }
        }
    }

    for a in &v.arms {
        d.pressing(hit_attrs("arm", Some(a.uid), None), |d| {
            // The arm's end zone, then the taper and the zone across: a taper's quad stands or lies like any piece.
            for piece in a.pieces.iter().chain(&a.beyond) {
                let id = KINDS[piece.kind].id;
                let foot = flatten(&piece.poly);
                // A walkway is walking surface wall to wall, whatever stands on it.
                let h = if a.pedestrian { wall } else { height_of(id) };
                match h {
                    h if h > 0.0 => d.stand_in(id, piece.material, foot, h, true),
                    _ => d.lay_in(id, piece.material, &foot),
                }
            }
            // The street's curbside uses stand or lie on their piece, over it.
            for u in &a.curb_uses {
                let Some(poly) = &u.poly else { continue };
                let id = KINDS[u.kind].id;
                let foot = flatten(poly);
                let h = if a.pedestrian { wall } else { height_of(id) };
                // One that fails its check is outlined over what it stands on.
                if u.bad {
                    d.mark_ring(true, "curb-use curb-use-bad", false, &foot, h);
                }
                match h {
                    h if h > 0.0 => d.stand_in(id, u.material, foot, h, true),
                    _ => d.lay_in(id, u.material, &foot),
                }
            }
            // A turbo's entry is wider than its street, where the layout wants more lanes than the street has.
            if let Some(entry) = &a.turbo_entry {
                d.lay("travel", &flatten(entry));
            }
            // Where there is no parking beside the curb the road runs on, over the parking piece.
            for gap in &a.gaps {
                d.lay("travel", &flatten(gap));
            }
            // The bus lane and the queue jump run over the arm's pieces; a stop's bulb and an island stand like a sidewalk.
            let t = &a.transit;
            for lane in [&t.bus, &t.queue].into_iter().flatten() {
                d.lay("bus", &flatten(lane));
            }
            // The queue jump is a dashed lane.
            if let Some(queue) = &t.queue {
                d.mark_ring(false, "iso-queue", false, &flatten(queue), 0.0);
            }
            for poly in [&t.stop_poly, &t.island].into_iter().flatten() {
                d.stand("sidewalk", flatten(poly), wall, true);
            }
            for lane in &a.core_bike {
                d.lay("bike", &flatten(lane));
            }
            if let Some(b) = &a.bike_box {
                d.lay("bike", &flatten(b));
            }
            for bulb in a.bulbs.iter().flatten() {
                d.stand("sidewalk", flatten(bulb), wall, true);
            }
            if let Some(i) = a.crossing.as_ref().and_then(|c| c.island_poly.as_ref()) {
                // The island of a crossing is a part of the crossing.
                d.hit = hit_attrs("crossing", Some(a.uid), None);
                d.stand("sidewalk", flatten(i), wall, true);
            }
        });
        // An entering lane is pressed over the arm's pieces, as it is on the plan.
        let name = say(i18n, units, &a.label);
        for (i, lane) in a.lanes.iter().enumerate() {
            let on = selected("lane", a.uid) && v.selected.lane == i;
            let title = i18n.tr("jn-sel-lane", &Args::new().num("n", i as i64 + 1).num("count", a.lanes.len() as i64).str("arm", name.clone()));
            let class = if on { "lane-hit on" } else { "lane-hit" };
            d.press_area(class, &hit_attrs("lane", Some(a.uid), Some(i)), &flatten(&lane.poly), 0.0, Some(&title));
            if on {
                d.outline(&flatten(&lane.poly), 0.0);
            }
        }
    }

    // A corner is a place to press, drawn after the streets so that it is the one pressed where it is made wider than
    // it is, to be as big as a finger (bare ground or a sidewalk's corner alike).
    for c in v.corners.iter().filter(|c| !c.straight) {
        d.press_area("iso-hit", &hit_attrs("corner", Some(c.uid), None), &flatten(&c.wedge), if c.walk { wall } else { 0.0 }, None);
    }

    // The lengths of the crossings, written over the drawing once all are known: two that would overlap on the
    // screen are moved apart, the longer crossing keeping its place, and one with no room is left out.
    let mut dims: Vec<Dim> = Vec::new();

    // Surfaces raised to the sidewalk's level go over the road they are on, so they come after everything flat.
    if v.raised && v.ring.is_none() {
        d.table("sidewalk", &flatten(&v.core), wall);
    }
    for a in &v.arms {
        if let Some(r) = &a.raised {
            let top = flatten(&r.top);
            d.table("sidewalk", &top, wall);
            d.ramp("travel", &flatten(&r.ramp), &top, wall);
        }
        // The markings lie on the road, or on the table where it is raised.
        let level = if a.raised.is_some() { wall } else { 0.0 };
        // Only where traffic is controlled: the street that has the right of way has no line.
        if let Some([from, to]) = a.stop_line.filter(|_| a.role != "free") {
            d.stop_line(from, to, level, a.role == "yield");
        }
        if let Some(c) = a.crossing.as_ref().filter(|c| !c.continuous) {
            d.zebra(&flatten(&c.poly), level);
        }
        if let Some([end, _]) = a.stop_line {
            match a.role {
                "signal" => d.post(end, SIGNAL_POST_MM, "signal"),
                "stop" => d.post(end, SIGN_POST_MM, "stop"),
                _ => {}
            }
        }
        for b in &a.transit.bollards {
            d.post(*b, BOLLARD_MM, "bollard");
        }
        // The turn arrows on the lanes, and the small ones where the traffic leaves.
        let size = lane_arrow_size(&a.lanes);
        for lane in &a.lanes {
            d.glyph(if lane.bad { " bad" } else { "" }, &lane_arrow(lane, size), size, level);
        }
        for out in &a.leave_arrows {
            d.glyph(" out", &plain_arrow(out.at, out.heading as f64, LEAVE_ARROW_MM, LEAVE_HEAD), LEAVE_ARROW_MM, level);
        }
        draw_stops(&mut d, a, wall, level);
        draw_measures(&mut d, a, level);
        let on = selected("arm", a.uid);
        d.name(a.end, &say(i18n, units, &a.street), on);
        if on {
            d.outline(&flatten(&a.outline), 0.0);
        }
        let Some(c) = a.crossing.as_ref() else { continue };
        let top = flatten(&c.poly);
        let h = if c.continuous { wall } else { level };
        // How far across, on the junction side of the crossing.
        if let Some((strokes, at)) = crossing_dimension(&top, a.bearing as f64) {
            d.mark_lines(false, if c.too_far { "dim dim-warn" } else { "dim" }, c.too_far, &strokes, h);
            let number = |mm: i32| units.number_in(mm, i18n.locale());
            let text = if c.stages > 1 { format!("{} × {}", c.stages, number(c.stage_mm)) } else { number(c.distance_mm) };
            let (x, y) = d.f.at(at, h);
            dims.push(Dim { at: (x, y + 5.0), warn: c.too_far, mm: c.distance_mm, text });
        }
        if c.continuous {
            d.table("sidewalk", &top, wall);
            for ramp in &c.ramps {
                d.ramp("sidewalk", &flatten(ramp), &top, wall);
            }
        }
        d.press_area("iso-hit", &hit_attrs("crossing", Some(a.uid), None), &top, h, None);
        if selected("crossing", a.uid) {
            d.outline(&top, h);
        }
    }

    dims.sort_by(|a, b| b.warn.cmp(&a.warn).then(b.mm.cmp(&a.mm)));
    let spots: Vec<Spot> = dims.iter().map(|m| Spot { at: m.at, chars: m.text.chars().count() }).collect();
    for (m, spot) in dims.iter().zip(spread_labels(&spots)) {
        if let Some((x, y)) = spot {
            d.label_px(&format!("t-dim t-halo{}", if m.warn { " t-warn" } else { "" }), (x, y), &m.text);
        }
    }

    // The turns of the street that is selected (or of a part of it), where they may go and where they may not.
    if v.selected.kind.is_some()
        && let Some(sa) = v.arms.iter().find(|a| a.uid == v.selected.uid)
    {
        let level = if v.raised && v.ring.is_none() { wall } else { 0.0 };
        for m in v.movements.iter().filter(|m| m.from == sa.uid) {
            let class = format!(
                "mv{}{}{}",
                if m.allowed { "" } else { " no" },
                if m.allowed && !m.lane { " bad" } else { "" },
                if m.indirect { " indirect" } else { "" }
            );
            let strokes = movement_strokes(&flatten(&m.path), m.allowed);
            d.mark_lines(true, &class, m.allowed && !m.lane, &strokes, level);
        }
    }

    (f, d.into_diagram(shades(&f)))
}

/// A crossing's length, to be written where there is room for it: where it wants to stand on the screen, and whether
/// the crossing is too long, and how long it is, which of two that would overlap is kept.
struct Dim {
    at: P,
    warn: bool,
    mm: i32,
    text: String,
}

/// An arrow's head, as a part of its length: a ring's, and where the traffic leaves.
const RING_HEAD: f64 = 0.2;
const LEAVE_HEAD: f64 = 0.19;

/// The street's transit stops on an arm: the platform over what it stands on, the bus's dwell and its run to the stop
/// line on the road, and the post.
fn draw_stops(d: &mut Scene, a: &ArmView, wall: f64, level: f64) {
    for st in &a.stops {
        let bad = if st.bad { " stop-bad" } else { "" };
        if let Some(pad) = &st.pad {
            let kind = match st.kind {
                StopKind::Curb => "",
                StopKind::Bulb => " stop-bulb",
                StopKind::Island => " stop-island",
            };
            d.mark_ring(true, &format!("stop-pad{kind}{bad}"), false, &flatten(pad), wall);
        }
        if let Some(dwell) = &st.dwell {
            d.mark_ring(false, &format!("stop-dwell{bad}"), false, &flatten(dwell), level);
        }
        if let Some(run) = &st.run {
            d.mark_lines(false, "stop-run", false, &[flatten(run)], level);
        }
        if let Some(post) = st.post {
            d.stop_post(post, wall, st.shelter);
        }
    }
}

/// The measures of an arm that are marks on the road: where a boundary's taper begins and ends, a virtual queue
/// jump's loop, a bus gate's line, a dead end's cap, and each measure's Atlas code.
fn draw_measures(d: &mut Scene, a: &ArmView, level: f64) {
    for [from, to] in &a.beyond_marks {
        d.mark_lines(false, "approach-mark", false, &[vec![*from, *to]], 0.0);
    }
    let t = &a.transit;
    if let Some(l) = &t.virtual_loop {
        d.mark_ring(false, "measure-loop", false, &flatten(l), level);
    }
    if let Some([from, to]) = t.gate {
        d.stop_line(from, to, level, t.gate_yields);
    }
    if let Some(c) = t.cap {
        d.mark_lines(false, "dead-cap", false, &[vec![c[0], c[1]]], level);
    }
    for tag in &t.tags {
        d.label("t-note t-halo measure-tag", tag.at, level, 4.0, tag.code);
    }
}
