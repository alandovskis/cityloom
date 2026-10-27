//! One phase of a traffic signal as a small drawing of the junction: the vehicle movements the phase serves as arrows,
//! the crossings that are walked as bars and the bike movements that ride as bike symbols, all lit; what does not run
//! is muted (a dashed outline, a faint bike). Pure: the model's view in, markup out, no frame and no wording (the
//! caller says the title and the summary, in the language it is in).

use std::fmt::Write;

use serde_json::Value;

use crate::junction::model::THROUGH;
use crate::junction::read_model::{ArmView, JView, PhaseView};
use crate::shared::catalogue::KINDS;
use crate::shared::iso::format::{esc, f1};
use crate::shared::symbols;

type P = (f64, f64);

fn num(v: &Value, i: usize) -> f64 {
    v[i].as_f64().unwrap_or(0.0)
}

fn dir_of(bearing: f64) -> P {
    let t = bearing.to_radians();
    (t.sin(), -t.cos())
}

fn pt(p: P) -> String {
    format!("{} {}", f1(p.0), f1(p.1))
}

/// A path from the model's drawing commands, in the junction's own millimetres.
fn path_d(cmds: &[Value]) -> String {
    let mut d = String::new();
    for c in cmds {
        match c[0].as_str().unwrap_or("Z") {
            m @ ("M" | "L") => write!(d, "{m}{}", pt((num(c, 1), num(c, 2)))).unwrap(),
            "A" => write!(d, "A{r} {r} 0 {} {} {}", c[2], c[3], pt((num(c, 4), num(c, 5))), r = f1(num(c, 1))).unwrap(),
            "Q" => write!(d, "Q{} {}", pt((num(c, 1), num(c, 2))), pt((num(c, 3), num(c, 4)))).unwrap(),
            _ => d.push('Z'),
        }
    }
    d
}

/// Where a movement's line starts and ends: in the lane on the driver's side of the arm it enters by, and of the arm it
/// leaves by.
fn lane_point(a: &ArmView, side: f64, unit: f64, leaving: bool) -> P {
    let d = dir_of(a.bearing as f64);
    // The way the traffic heads on this arm: toward the junction when it enters, away from it when it leaves.
    let heading = if leaving { d } else { (-d.0, -d.1) };
    // To the driver's right in a y-down drawing is a quarter turn clockwise.
    let right = (-heading.1, heading.0);
    let lateral = (a.road_mm as f64 * 0.12).clamp(unit * 2.0, unit * 6.0) * side;
    (a.mouth_at.0 + right.0 * lateral, a.mouth_at.1 + right.1 * lateral)
}

/// An arrow from the entering lane of `from` to the leaving lane of `to`: straight on, or a curve round the corner.
fn movement(from: &ArmView, to: &ArmView, class: u8, side: f64, unit: f64) -> String {
    let a = lane_point(from, side, unit, false);
    let b = lane_point(to, side, unit, true);
    let (din, dout) = (dir_of(from.bearing as f64), dir_of(to.bearing as f64));
    // The line enters heading -din and leaves heading +dout.
    let enter = (-din.0, -din.1);
    let det = enter.0 * dout.1 - enter.1 * dout.0;
    let control = (class != THROUGH && det.abs() > 1e-6).then(|| {
        // Where the line through `a` along `enter` meets the line through `b` along `dout`.
        let t = ((b.0 - a.0) * dout.1 - (b.1 - a.1) * dout.0) / det;
        (a.0 + enter.0 * t, a.1 + enter.1 * t)
    });
    let tip = control.map_or_else(|| ((b.0 - a.0), (b.1 - a.1)), |c| (b.0 - c.0, b.1 - c.1));
    let len = tip.0.hypot(tip.1).max(1.0);
    let t = (tip.0 / len, tip.1 / len);
    let head = unit * 5.0;
    // The line stops where the head begins.
    let base = (b.0 - t.0 * head, b.1 - t.1 * head);
    let n = (-t.1, t.0);
    let line = match control {
        Some(c) => format!("M{}Q{} {}", pt(a), pt(c), pt(base)),
        None => format!("M{}L{}", pt(a), pt(base)),
    };
    format!(
        "<g class=\"ph-move lit\"><path class=\"ph-line\" fill=\"none\" stroke-width=\"{}\" d=\"{line}\"/><path class=\"ph-head\" d=\"M{}L{}L{}Z\"/></g>",
        f1(unit * 2.2),
        pt(b),
        pt((base.0 + n.0 * head * 0.6, base.1 + n.1 * head * 0.6)),
        pt((base.0 - n.0 * head * 0.6, base.1 - n.1 * head * 0.6)),
    )
}

/// A bike symbol on the arm, beside the road's end, lit when the phase lets bikes ride from the arm.
fn bike(a: &ArmView, lit: bool, unit: f64) -> String {
    let d = dir_of(a.bearing as f64);
    let size = unit * 14.0;
    let reach = unit * 12.0;
    let at = (a.mouth_at.0 + d.0 * reach - size / 2.0, a.mouth_at.1 + d.1 * reach - size * 0.4);
    let icon = symbols::icon("bike").replacen(
        "<svg class=\"icon\"",
        &format!("<svg class=\"icon\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"", f1(at.0), f1(at.1), f1(size), f1(size * 0.8)),
        1,
    );
    format!("<g class=\"ph-bike {}\">{icon}</g>", if lit { "lit" } else { "muted" })
}

/// The drawing of one phase. `title` names it, `summary` says what it allows; both are read by a screen reader.
pub fn phase_svg(v: &JView, phase: &PhaseView, title: &str, summary: &str) -> String {
    let [x0, y0, x1, y1] = v.bounds;
    // A square, with room for the bike symbols past the arms' ends.
    let size = (x1 - x0).max(y1 - y0) * 1.08;
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let unit = size / 100.0;
    let side = if v.drive_side == "left" { -1.0 } else { 1.0 };
    let core = v.arms.iter().map(|a| a.mouth_at.0.hypot(a.mouth_at.1)).fold(0.0, f64::max);

    let mut out = format!(
        "<svg class=\"phase-svg\" role=\"img\" aria-label=\"{}\" viewBox=\"{} {} {} {}\" preserveAspectRatio=\"xMidYMid meet\"><title>{}</title>",
        esc(summary),
        f1(cx - size / 2.0),
        f1(cy - size / 2.0),
        f1(size),
        f1(size),
        esc(title)
    );
    write!(out, "<circle class=\"ph-road\" cx=\"0\" cy=\"0\" r=\"{}\"/>", f1(core)).unwrap();
    for a in &v.arms {
        write!(out, "<path class=\"ph-road\" d=\"{}\"/>", path_d(&a.outline)).unwrap();
    }
    for a in &v.arms {
        let Some(c) = &a.crossing else { continue };
        let lit = phase.walk_arms.contains(&a.uid);
        write!(
            out,
            "<path class=\"ph-walk {}\" stroke-width=\"{}\" d=\"{}\"/>",
            if lit { "lit" } else { "muted" },
            f1(unit * 0.6),
            path_d(&c.poly)
        )
        .unwrap();
    }
    for a in v.arms.iter().filter(|a| a.pieces.iter().any(|p| KINDS[p.kind].id == "bike")) {
        out += &bike(a, phase.ride_arms.contains(&a.uid), unit);
    }
    for m in &phase.moves {
        let (Some(from), Some(to)) = (v.arms.iter().find(|a| a.uid == m.from), v.arms.iter().find(|a| a.uid == m.to)) else { continue };
        let class = v.movements.iter().find(|x| x.from == m.from && x.to == m.to).map_or(THROUGH, |x| x.class);
        out += &movement(from, to, class, side, unit);
    }
    out + "</svg>"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::{Junction, junction_of};
    use crate::shared::testing::count;

    fn draw(j: &Junction, phase: usize) -> (String, usize) {
        let v = j.view();
        let p = v.signal.as_ref().unwrap().phases[phase].clone();
        let moves = p.moves.len();
        (phase_svg(&v, &p, "Phase 1", "Cars go: north."), moves)
    }

    #[test]
    fn a_drawing_is_an_image_with_the_summary_as_its_label_and_the_name_as_its_title() {
        let (s, _) = draw(&Junction::new(0), 0);
        assert!(s.starts_with("<svg class=\"phase-svg\" role=\"img\" aria-label=\"Cars go: north.\""), "{s}");
        assert!(s.contains("<title>Phase 1</title>") && s.ends_with("</svg>"));
        assert!(!s.contains(" id="), "the same junction is drawn once a phase: no id may repeat");
    }

    #[test]
    fn every_movement_the_phase_serves_is_an_arrow_and_nothing_else_is() {
        let j = Junction::new(0);
        for phase in 0..4 {
            let (s, moves) = draw(&j, phase);
            assert!(moves > 0);
            assert_eq!(count(&s, "class=\"ph-move lit\""), moves);
            assert_eq!(count(&s, "class=\"ph-head\""), moves);
        }
    }

    #[test]
    fn the_crossings_walked_are_lit_and_the_others_muted() {
        let j = Junction::new(0);
        let v = j.view();
        let (s, _) = draw(&j, 0);
        let walked = v.signal.as_ref().unwrap().phases[0].walk_arms.len();
        let crossings = v.arms.iter().filter(|a| a.crossing.is_some()).count();
        assert!(walked > 0 && walked < crossings, "the north phase leaves some crossings red");
        assert_eq!(count(&s, "ph-walk lit"), walked);
        assert_eq!(count(&s, "ph-walk muted"), crossings - walked);
    }

    #[test]
    fn a_bike_lane_is_drawn_lit_when_it_runs_and_muted_when_it_does_not() {
        let j = junction_of(&[("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)]);
        let v = j.view();
        let biked = v.arms.iter().filter(|a| a.pieces.iter().any(|p| KINDS[p.kind].id == "bike")).count();
        let (s, _) = draw(&j, 0);
        let ride = v.signal.as_ref().unwrap().phases[0].ride_arms.len();
        assert!(ride > 0);
        assert_eq!(count(&s, "ph-bike lit"), ride);
        assert_eq!(count(&s, "ph-bike muted"), biked - ride);
        assert!(!draw(&Junction::new(0), 0).0.contains("ph-bike"), "no bike lane, no bike");
    }

    #[test]
    fn the_title_and_summary_are_escaped() {
        let j = Junction::new(0);
        let v = j.view();
        let p = v.signal.as_ref().unwrap().phases[0].clone();
        let s = phase_svg(&v, &p, "A & B <C>", "\"x\"");
        assert!(s.contains("A &amp; B &lt;C&gt;") && s.contains("aria-label=\"&quot;x&quot;\""), "{s}");
    }
}
