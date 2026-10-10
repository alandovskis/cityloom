//! The junction's plan as SVG markup, from the model's view. Millimetres on the
//! plan become pixels through one frame; everything else is already in the view.

use std::fmt::Write;

use serde_json::Value;

use crate::junction::frame::Frame;
use crate::junction::model::{LEFT, RIGHT, THROUGH};
use crate::junction::read_model::{ArmView, JView, LaneView};
use crate::junction::text::arms_text;
use crate::junction::turns::compass_key;
use crate::shared::catalogue::KINDS;
use crate::shared::i18n::{Args, I18n, Locale};
use crate::shared::said::{Said, say};
use crate::shared::units::Units;

type P = (f64, f64);

/// What the page needs to put the plan in its `<svg>`.
pub struct PlanSvg {
    pub frame: Frame,
    pub label: String,
    pub markup: String,
}

/// A number to a tenth, with no trailing zero.
fn f1(n: f64) -> String {
    let r = (n * 10.0).round() / 10.0;
    if r == 0.0 { "0".to_string() } else { format!("{r}") }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn dir_of(bearing: f64) -> P {
    let t = bearing.to_radians();
    (t.sin(), -t.cos())
}

fn hatch(kind_id: &str) -> String {
    format!("url(#h-{kind_id})")
}

fn num(v: &Value, i: usize) -> f64 {
    v[i].as_f64().unwrap_or(0.0)
}

struct Plan<'a> {
    v: &'a JView,
    f: Frame,
    units: Units,
    i18n: &'a I18n,
    /// The language the numbers are written in, read once so the drawing is made again on a switch.
    locale: Locale,
}

impl Plan<'_> {
    fn pt(&self, p: P) -> String {
        let (x, y) = self.f.at(p);
        format!("{} {}", f1(x), f1(y))
    }

    /// A path from the model's drawing commands.
    fn path_d(&self, cmds: &[Value]) -> String {
        let mut d = String::new();
        for c in cmds {
            match c[0].as_str().unwrap_or("Z") {
                m @ ("M" | "L") => write!(d, "{m}{}", self.pt((num(c, 1), num(c, 2)))).unwrap(),
                "A" => {
                    let r = f1(num(c, 1) * self.f.scale);
                    write!(d, "A{r} {r} 0 {} {} {}", c[2], c[3], self.pt((num(c, 4), num(c, 5)))).unwrap()
                }
                "Q" => write!(d, "Q{} {}", self.pt((num(c, 1), num(c, 2))), self.pt((num(c, 3), num(c, 4)))).unwrap(),
                _ => d.push('Z'),
            }
        }
        d
    }

    fn is_sel(&self, kind: &str, uid: u32) -> bool {
        self.v.selected.kind == Some(kind) && self.v.selected.uid == uid
    }

    fn is_lane(&self, uid: u32, i: usize) -> bool {
        self.v.selected.kind == Some("lane") && self.v.selected.uid == uid && self.v.selected.lane == i
    }

    /// A shape filled with the hatch of a kind of piece, over its tint.
    fn piece(&self, class: &str, kind_id: &str, d: &str) -> String {
        format!("<path class=\"{class}\" d=\"{d}\"/><path class=\"hatch\" fill=\"{}\" d=\"{d}\"/>", hatch(kind_id))
    }

    fn line(&self, class: &str, a: P, b: P) -> String {
        format!("<path class=\"{class}\" d=\"M{} {}L{} {}\"/>", f1(a.0), f1(a.1), f1(b.0), f1(b.1))
    }

    /// A turn arrow lying on a lane, pointing along the lane's travel.
    fn lane_arrow(&self, l: &LaneView, size: f64) -> String {
        let h = size;
        let through = l.uses & THROUGH != 0;
        let stem = if through { -h / 2.0 } else { 0.0 };
        let bw = h * 0.42;
        let mut d = format!("M0 {}V{}", h / 2.0, stem);
        if through {
            write!(d, "M{} {}L0 {}L{} {}", -h * 0.18, -h / 2.0 + h * 0.2, -h / 2.0, h * 0.18, -h / 2.0 + h * 0.2).unwrap();
        }
        if l.uses & LEFT != 0 {
            write!(
                d,
                "M0 {}Q0 {} {} {}M{} {}L{} {}L{} {}",
                -h * 0.05,
                -h * 0.3,
                -bw,
                -h * 0.3,
                -bw + h * 0.16,
                -h * 0.3 - h * 0.16,
                -bw,
                -h * 0.3,
                -bw + h * 0.16,
                -h * 0.3 + h * 0.16
            )
            .unwrap();
        }
        if l.uses & RIGHT != 0 {
            write!(
                d,
                "M0 {}Q0 {} {} {}M{} {}L{} {}L{} {}",
                -h * 0.05,
                -h * 0.3,
                bw,
                -h * 0.3,
                bw - h * 0.16,
                -h * 0.3 - h * 0.16,
                bw,
                -h * 0.3,
                bw - h * 0.16,
                -h * 0.3 + h * 0.16
            )
            .unwrap();
        }
        let (x, y) = self.f.at(l.at);
        format!(
            "<g class=\"lane-arrow{}\" transform=\"translate({} {}) rotate({})\"><path class=\"halo\" d=\"{d}\"/><path d=\"{d}\"/></g>",
            if l.bad { " bad" } else { "" },
            f1(x),
            f1(y),
            l.heading
        )
    }

    fn grip(&self, x: f64, y: f64, angle: f64, role: &str, uid: u32, label: &str) -> String {
        format!(
            "<g class=\"handle\" data-role=\"{role}\" data-uid=\"{uid}\" transform=\"translate({} {}) rotate({})\">\
             <title>{}</title><rect class=\"hit\" x=\"-16\" y=\"-15\" width=\"32\" height=\"30\"/>\
             <rect class=\"grip\" x=\"-10\" y=\"-8\" width=\"20\" height=\"16\" rx=\"2\"/>\
             <path class=\"grip-arrow\" d=\"M-6 0H6M-3 -3 -6 0l3 3M3 -3 6 0 3 3\"/></g>",
            f1(x),
            f1(y),
            f1(angle),
            esc(label)
        )
    }

    /// The stop line of an arm and the sign that says how traffic is controlled.
    fn control_marker(&self, a: &ArmView) -> String {
        let (Some(line), true) = (a.stop_line, a.role != "free") else { return String::new() };
        let (p0, p1) = (self.f.at(line[0]), self.f.at(line[1]));
        let mid = ((p0.0 + p1.0) / 2.0, (p0.1 + p1.1) / 2.0);
        let ax = self.f.at(a.mouth_at);
        // The end of the stop line nearest the kerb is the one farthest from the axis.
        let far = if (p0.0 - ax.0).hypot(p0.1 - ax.1) > (p1.0 - ax.0).hypot(p1.1 - ax.1) { p0 } else { p1 };
        let d = (far.0 - mid.0).hypot(far.1 - mid.1);
        let d = if d == 0.0 { 1.0 } else { d };
        let out = ((far.0 - mid.0) / d, (far.1 - mid.1) / d);
        let c = (far.0 + out.0 * 15.0, far.1 + out.1 * 15.0);
        let class = if a.role == "yield" { "stop-line yield" } else { "stop-line" };
        let sign = match a.role {
            "signal" => format!(
                "<g class=\"sign\" transform=\"translate({} {}) rotate({})\"><rect x=\"-4.5\" y=\"-10\" width=\"9\" height=\"20\" rx=\"2\"/><circle cx=\"0\" cy=\"-5\" r=\"1.7\"/><circle cx=\"0\" cy=\"0\" r=\"1.7\"/><circle cx=\"0\" cy=\"5\" r=\"1.7\"/></g>",
                f1(c.0),
                f1(c.1),
                a.bearing
            ),
            "stop" => format!(
                "<g class=\"sign\" transform=\"translate({} {})\"><path d=\"M-3.5 -8.5h7l5 5v7l-5 5h-7l-5 -5v-7z\"/><path d=\"M-4 0h8\"/></g>",
                f1(c.0),
                f1(c.1)
            ),
            "yield" => format!("<g class=\"sign\" transform=\"translate({} {}) rotate({})\"><path d=\"M-8 -7H8L0 8z\"/></g>", f1(c.0), f1(c.1), a.bearing),
            _ => String::new(),
        };
        self.line(class, p0, p1) + &sign
    }

    /// The distance across as a dimension string with ticks, on the junction side
    /// of the crossing where nothing else is drawn, its figure over a paper halo.
    fn crossing_dim(&self, a: &ArmView) -> String {
        let Some(c) = &a.crossing else { return String::new() };
        let n = dir_of(a.bearing as f64);
        let off = -800.0;
        let q = |i: usize| self.f.at((num(&c.poly[i], 1) + n.0 * off, num(&c.poly[i], 2) + n.1 * off));
        let (q0, q1) = (q(0), q(1));
        let ang = (q1.1 - q0.1).atan2(q1.0 - q0.0);
        let tk = ((ang + 0.785).cos() * 5.0, (ang + 0.785).sin() * 5.0);
        let tick = |p: P| format!("M{} {}L{} {}", f1(p.0 - tk.0), f1(p.1 - tk.1), f1(p.0 + tk.0), f1(p.1 + tk.1));
        let number = |mm: i32| self.units.number_in(mm, self.locale);
        let text = if c.stages > 1 { format!("{} × {}", c.stages, number(c.stage_mm)) } else { number(c.distance_mm) };
        let warn = c.too_far;
        format!(
            "<path class=\"dim{}\" d=\"M{} {}L{} {}{}{}\"/><text class=\"t-dim t-halo{}\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{text}</text>",
            if warn { " dim-warn" } else { "" },
            f1(q0.0),
            f1(q0.1),
            f1(q1.0),
            f1(q1.1),
            tick(q0),
            tick(q1),
            if warn { " t-warn" } else { "" },
            f1((q0.0 + q1.0) / 2.0),
            f1((q0.1 + q1.1) / 2.0 + 5.0)
        )
    }

    /// A code tag, as the Atlas names the measure, on a paper halo.
    fn code_tag(&self, p: P, code: &str) -> String {
        let (x, y) = self.f.at(p);
        format!("<text class=\"t-note t-halo measure-tag\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{code}</text>", f1(x), f1(y + 4.0))
    }

    /// The transit priority measures on one arm: gates, stops, filters, caps and
    /// their Atlas codes. The bus lane and queue jumps are drawn with the arm itself.
    fn measure_marks(&self, a: &ArmView) -> String {
        let t = &a.transit;
        let mut out = String::new();
        if let Some(l) = &t.virtual_loop {
            write!(out, "<path class=\"measure-loop\" d=\"{}\"/>", self.path_d(l)).unwrap();
        }
        if let Some(g) = &t.gate {
            out += &self.line(if t.gate_yields { "stop-line yield" } else { "stop-line" }, self.f.at(g[0]), self.f.at(g[1]));
        }
        if let Some(s) = &t.stop_poly {
            out += &self.piece("bulb measure-stop k-sidewalk", "sidewalk", &self.path_d(s));
        }
        for b in &t.bollards {
            let (x, y) = self.f.at(*b);
            write!(out, "<circle class=\"bollard\" cx=\"{}\" cy=\"{}\" r=\"3.2\"/>", f1(x), f1(y)).unwrap();
        }
        if let Some(c) = &t.cap {
            out += &self.line("dead-cap", self.f.at(c[0]), self.f.at(c[1]));
        }
        if let Some(i) = &t.island {
            out += &self.piece("island k-sidewalk", "sidewalk", &self.path_d(i));
        }
        for g in &t.tags {
            out += &self.code_tag(g.at, g.code);
        }
        out
    }

    fn scale_bar(&self, x: f64, y: f64) -> String {
        let block = 4000.0 * self.f.scale;
        let blocks: String = (0..5)
            .map(|i| {
                format!(
                    "<rect class=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"6\"/>",
                    if i % 2 == 1 { "bar-w" } else { "bar-b" },
                    f1(x + 36.0 + i as f64 * block),
                    y - 6.0,
                    f1(block)
                )
            })
            .collect();
        // Twenty metres, or the whole feet nearest them.
        let end = self.units.length_whole_in(20_000, self.locale);
        let scale = esc(&self.i18n.tr("jn-svg-scale", &Args::new()));
        format!(
            "<g class=\"scale\"><text class=\"t-label\" x=\"{x}\" y=\"{y}\">{scale}</text>{blocks}<text class=\"t-dim\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">0</text><text class=\"t-dim\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{end}</text></g>",
            f1(x + 36.0),
            y + 20.0,
            f1(x + 36.0 + 5.0 * block),
            y + 20.0
        )
    }
}

/// What a screen reader is told the drawing is, drawn again when the language changes.
pub fn plan_label(v: &JView, i18n: &I18n) -> String {
    let key = if v.control == "roundabout" { "jn-plan-label-roundabout" } else { "jn-plan-label" };
    i18n.tr(key, &Args::new().str("arms", arms_text(v, i18n)))
}

/// The plan for a drawing `width` wide in a window `window_height` high. Its words are asked for
/// with the tracked `tr` and `say`, so a view that draws it draws it again when the language changes.
pub fn plan_svg(v: &JView, i18n: &I18n, width: f64, window_height: f64, units: Units) -> PlanSvg {
    let said = |s: &Said| say(i18n, units, s);
    let tr = |key: &str, args: Args| i18n.tr(key, &args);
    let locale = i18n.locale();
    let f = Frame::fit(v.bounds, width, window_height);
    let p = Plan { v, f, units, i18n, locale };
    let zebra = (600.0 * f.scale).max(4.0);
    let zebra = (zebra * 10.0).round() / 10.0;

    let defs: String = v
        .arms
        .iter()
        .filter(|a| a.crossing.is_some())
        .map(|a| {
            format!(
                "<pattern id=\"zb-{}\" width=\"{}\" height=\"{}\" patternUnits=\"userSpaceOnUse\" patternTransform=\"rotate({})\"><rect class=\"zebra\" width=\"{zebra}\" height=\"{}\"/></pattern>",
                a.uid, zebra * 2.0, zebra * 2.0, a.bearing, zebra * 2.0
            )
        })
        .collect();

    #[derive(Default)]
    struct Layers {
        wedge: String,
        arm: String,
        lane: String,
        measure: String,
        road: String,
        bulb: String,
        curb: String,
        cross: String,
        mark: String,
        sel: String,
        grip: String,
        mv: String,
        label: String,
    }
    let mut l = Layers::default();

    // pavement wedges between arms; pressing one selects the corner
    for c in &v.corners {
        let d = p.path_d(&c.wedge);
        write!(
            l.wedge,
            "<g class=\"wedge-g\" data-role=\"{}\" data-uid=\"{}\">{}</g>",
            if c.straight { "none" } else { "corner" },
            c.uid,
            if c.walk { p.piece("wedge k-sidewalk", "sidewalk", &d) } else { format!("<path class=\"wedge bare\" d=\"{d}\"/>") }
        )
        .unwrap();
    }

    for a in &v.arms {
        let mut g = String::new();
        for piece in &a.pieces {
            let id = KINDS[piece.kind].id;
            g += &p.piece(&format!("piece k-{id}"), id, &p.path_d(&piece.poly));
        }
        let t = &a.transit;
        if let Some(bus) = &t.bus {
            g += &p.piece("piece k-bus", "bus", &p.path_d(bus));
        }
        if let Some(q) = &t.queue {
            g += &p.piece("piece k-bus queue", "bus", &p.path_d(q));
        }
        write!(
            l.arm,
            "<g class=\"arm{}{}\" data-role=\"arm\" data-uid=\"{}\">{g}</g>",
            if p.is_sel("arm", a.uid) { " on" } else { "" },
            if t.dead { " dead" } else { "" },
            a.uid
        )
        .unwrap();
        l.measure += &p.measure_marks(a);
        let arm = said(&a.label);
        for (i, lane) in a.lanes.iter().enumerate() {
            write!(
                l.lane,
                "<path class=\"lane-hit{}\" data-role=\"lane\" data-uid=\"{}\" data-lane=\"{i}\" d=\"{}\"><title>{}</title></path>",
                if p.is_lane(a.uid, i) { " on" } else { "" },
                a.uid,
                p.path_d(&lane.poly),
                esc(&tr("jn-sel-lane", Args::new().num("n", i as i64 + 1).num("count", a.lanes.len() as i64).str("arm", arm.clone())))
            )
            .unwrap();
        }
        for gap in &a.gaps {
            write!(l.road, "<path class=\"road\" d=\"{}\"/>", p.path_d(gap)).unwrap();
        }
        for b in a.bulbs.iter().flatten() {
            write!(l.bulb, "<g data-role=\"arm\" data-uid=\"{}\">{}</g>", a.uid, p.piece("bulb k-sidewalk", "sidewalk", &p.path_d(b))).unwrap();
        }
    }

    // the carriageway where the streets meet
    if let Some(ring) = &v.ring {
        let r = ring.road_mm as f64 * f.scale;
        let ro = ring.radius_mm as f64 * f.scale;
        let (cx, cy) = f.at((0.0, 0.0));
        let ri = ring.island_mm as f64 * f.scale;
        write!(l.road, "<circle class=\"road ring\" cx=\"{}\" cy=\"{}\" r=\"{}\"/>", f1(cx), f1(cy), f1(r)).unwrap();
        if ring.cycle_mm.is_some_and(|c| c != 0) {
            let circle = |rad: f64| {
                format!("M{} {}a{} {} 0 1 0 {} 0a{} {} 0 1 0 {} 0Z", f1(cx - rad), f1(cy), f1(rad), f1(rad), f1(2.0 * rad), f1(rad), f1(rad), f1(-2.0 * rad))
            };
            let band = circle(ro) + &circle(r);
            write!(
                l.road,
                "<g class=\"cycle-g\" data-role=\"cycle\"><path class=\"piece k-bike cycle-ring\" fill-rule=\"evenodd\" d=\"{band}\"/><path class=\"hatch\" fill-rule=\"evenodd\" fill=\"{}\" d=\"{band}\"/></g>",
                hatch("bike")
            )
            .unwrap();
            if v.selected.kind == Some("cycle") {
                write!(
                    l.sel,
                    "<circle class=\"sel-line\" cx=\"{}\" cy=\"{}\" r=\"{}\"/><circle class=\"sel-line\" cx=\"{}\" cy=\"{}\" r=\"{}\"/>",
                    f1(cx),
                    f1(cy),
                    f1(ro),
                    f1(cx),
                    f1(cy),
                    f1(r)
                )
                .unwrap();
            }
        }
        write!(
            l.road,
            "<circle class=\"island k-median\" cx=\"{}\" cy=\"{}\" r=\"{}\"/><circle class=\"hatch\" fill=\"{}\" cx=\"{}\" cy=\"{}\" r=\"{}\"/>",
            f1(cx),
            f1(cy),
            f1(ri),
            hatch("median"),
            f1(cx),
            f1(cy),
            f1(ri)
        )
        .unwrap();
        if let Some(bus) = &v.bus {
            let d = p.path_d(&bus.poly);
            write!(l.road, "<g class=\"bus-g\" data-role=\"bus\">{}</g>", p.piece("piece k-bus bus-lane", "bus", &d)).unwrap();
            if v.selected.kind == Some("bus") {
                write!(l.sel, "<path class=\"sel-box\" d=\"{d}\"/>").unwrap();
            }
            write!(
                l.label,
                "<text class=\"t-mark t-halo\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
                f1(cx),
                f1(cy + 5.0),
                esc(&tr("jn-svg-bus-only", Args::new()))
            )
            .unwrap();
        }
        // circulation arrows on the ring, between the streets
        let mid = (ring.road_mm as f64 - 3000.0) * f.scale;
        let ccw = ring.circulation == "anticlockwise";
        let bearings: Vec<i32> = v.arms.iter().map(|a| a.bearing).collect();
        for (i, b) in bearings.iter().enumerate() {
            let nb = bearings[(i + 1) % bearings.len()];
            let gap = (nb - b + 360) % 360;
            let gap = if gap == 0 { 360 } else { gap };
            let ang = *b as f64 + gap as f64 / 2.0;
            let (dx, dy) = dir_of(ang);
            let heading = ang + if ccw { -90.0 } else { 90.0 };
            let h = 20.0_f64.min(mid * 0.5);
            let d = format!("M0 {}V{}M-4 {}L0 {}L4 {}", h / 2.0, -h / 2.0, -h / 2.0 + 4.0, -h / 2.0, -h / 2.0 + 4.0);
            write!(
                l.mark,
                "<g class=\"lane-arrow\" transform=\"translate({} {}) rotate({})\"><path class=\"halo\" d=\"{d}\"/><path d=\"{d}\"/></g>",
                f1(cx + dx * mid),
                f1(cy + dy * mid),
                f1(heading)
            )
            .unwrap();
        }
    } else {
        write!(l.road, "<path class=\"road\" d=\"{}\"/>", p.path_d(&v.core)).unwrap();
    }

    for c in &v.corners {
        let bad = !c.ok || c.fast;
        write!(l.curb, "<path class=\"curb-line{}\" d=\"{}\"/>", if bad { " bad" } else { "" }, p.path_d(&c.curb)).unwrap();
    }

    for a in &v.arms {
        if let Some(c) = &a.crossing {
            let d = p.path_d(&c.poly);
            write!(
                l.cross,
                "<g data-role=\"crossing\" data-uid=\"{}\"><path class=\"crossing\" d=\"{d}\"/><path fill=\"url(#zb-{})\" class=\"zebra-fill\" d=\"{d}\"/></g>",
                a.uid, a.uid
            )
            .unwrap();
            if let Some(i) = &c.island_poly {
                write!(l.cross, "<g data-role=\"crossing\" data-uid=\"{}\">{}</g>", a.uid, p.piece("island k-sidewalk", "sidewalk", &p.path_d(i))).unwrap();
            }
            if p.is_sel("crossing", a.uid) {
                write!(l.sel, "<path class=\"sel-box\" d=\"{d}\"/>").unwrap();
            }
            l.label += &p.crossing_dim(a);
        }
        l.mark += &p.control_marker(a);
        let arrow = (2600.0 * f.scale).clamp(16.0, 34.0);
        for lane in &a.lanes {
            l.mark += &p.lane_arrow(lane, arrow);
        }
        for out in &a.leave_arrows {
            let (x, y) = f.at(out.at);
            let h = (1800.0 * f.scale).clamp(12.0, 22.0);
            let d = format!("M0 {}V{}M-3.5 {}L0 {}L3.5 {}", h / 2.0, -h / 2.0, -h / 2.0 + 3.5, -h / 2.0, -h / 2.0 + 3.5);
            write!(
                l.mark,
                "<g class=\"lane-arrow out\" transform=\"translate({} {}) rotate({})\"><path class=\"halo\" d=\"{d}\"/><path d=\"{d}\"/></g>",
                f1(x),
                f1(y),
                out.heading
            )
            .unwrap();
        }

        // the street's name and width, outside the end of the arm
        let (lx, ly) = f.at(a.end);
        let ld = dir_of(a.bearing as f64);
        let side = ld.0.abs() > 0.5;
        // On a narrow screen there is no room beside a sideways arm: set its name above it.
        let beside = side && !f.narrow;
        let anchor = if side {
            if ld.0 > 0.0 {
                if f.narrow { "end" } else { "start" }
            } else if f.narrow {
                "start"
            } else {
                "end"
            }
        } else {
            "middle"
        };
        let x = if beside { lx + ld.0 * 12.0 } else { lx };
        let y1 = if beside {
            ly - 2.0
        } else if side {
            // clear of the road's edge and its crossing: the west name above its arm, the east below its own,
            // so the two never meet in the middle of a narrow drawing
            let half = a.road_mm as f64 * f.scale / 2.0;
            if ld.0 < 0.0 { ly - half - 30.0 } else { ly + half + 26.0 }
        } else if ld.1 < 0.0 {
            ly - 32.0
        } else {
            ly + 26.0
        };
        let road = Args::new().str("compass", tr(compass_key(a.bearing), Args::new())).str("width", units.length_in(a.road_mm, locale));
        let road = match a.offset_mm {
            0 => tr("jn-svg-road", road),
            mm => tr("jn-svg-road-shifted", road.str("offset", units.length_in(mm.abs(), locale))),
        };
        write!(
            l.label,
            "<text class=\"t-mark t-halo{}\" x=\"{}\" y=\"{}\" text-anchor=\"{anchor}\">{}</text><text class=\"t-note t-halo t-soft\" x=\"{}\" y=\"{}\" text-anchor=\"{anchor}\">{}</text>",
            if p.is_sel("arm", a.uid) { " t-blue" } else { "" },
            f1(x),
            f1(y1),
            esc(&said(&a.street)),
            f1(x),
            f1(y1 + 17.0),
            esc(&road)
        )
        .unwrap();

        for (i, lane) in a.lanes.iter().enumerate() {
            if p.is_lane(a.uid, i) {
                write!(l.sel, "<path class=\"sel-box\" d=\"{}\"/>", p.path_d(&lane.poly)).unwrap();
            }
        }
        if p.is_sel("arm", a.uid) {
            write!(l.sel, "<path class=\"sel-box\" d=\"{}\"/>", p.path_d(&a.outline)).unwrap();
        }
        // the end grip: always there, since turning a street is the main move
        let (ex, ey) = f.at(a.end);
        l.grip += &p.grip(ex - ld.0 * 20.0, ey - ld.1 * 20.0, a.bearing as f64, "grip-arm", a.uid, &tr("jn-grip-turn", Args::new().str("arm", said(&a.label))));
    }

    // grips and the outline of a selected corner or crossing
    for c in &v.corners {
        let Some(bis) = c.bisector else { continue };
        if !p.is_sel("corner", c.uid) || c.straight {
            continue;
        }
        write!(l.sel, "<path class=\"sel-line\" d=\"{}\"/>", p.path_d(&c.curb)).unwrap();
        let (gx, gy) = f.at(c.handle);
        l.grip += &p.grip(gx, gy, bis.1.atan2(bis.0).to_degrees(), "grip-corner", c.uid, &tr("jn-grip-corner", Args::new()));
    }
    for a in &v.arms {
        let Some(c) = &a.crossing else { continue };
        if !p.is_sel("crossing", a.uid) {
            continue;
        }
        let (gx, gy) = f.at(c.handle);
        l.grip += &p.grip(gx, gy, a.bearing as f64 - 90.0, "grip-crossing", a.uid, &tr("jn-grip-crossing", Args::new()));
    }

    // the turns of a selected street
    if v.selected.kind.is_some() {
        if let Some(sa) = v.arms.iter().find(|a| a.uid == v.selected.uid) {
            for m in v.movements.iter().filter(|m| m.from == sa.uid) {
                let d = p.path_d(&m.path);
                write!(
                    l.mv,
                    "<path class=\"mv{}{}{}\" d=\"{d}\"{}><title>{}</title></path>",
                    if m.allowed { "" } else { " no" },
                    if m.allowed && !m.lane { " bad" } else { "" },
                    if m.indirect { " indirect" } else { "" },
                    if m.allowed { " marker-end=\"url(#mv-head)\"" } else { "" },
                    esc(&m.blocked.as_ref().map(said).unwrap_or_default())
                )
                .unwrap();
                if !m.allowed {
                    if let (Some(first), Some(last)) = (m.path.first(), m.path.last()) {
                        let n = last.as_array().map_or(0, |a| a.len());
                        let (x, y) = f.at(((num(first, 1) + num(last, n - 2)) / 2.0, (num(first, 2) + num(last, n - 1)) / 2.0));
                        write!(l.mv, "<path class=\"mv-x\" d=\"M{} {}l10 10m0 -10l-10 10\"/>", f1(x - 5.0), f1(y - 5.0)).unwrap();
                    }
                }
            }
        }
    }

    // North mark and scale bar. The `N` stays a literal on purpose: it is the map symbol for north, the same in
    // English and in French (nord), and the plan's label already says "north up" in words.
    let furniture = format!(
        "<g class=\"north\" transform=\"translate(34 44)\"><circle r=\"15\"/><path d=\"M0 11V-9M-4 -4 0 -10l4 6\"/><text class=\"t-label\" y=\"-20\" text-anchor=\"middle\">N</text></g>{}",
        p.scale_bar(20.0, f.height - 30.0)
    );

    let markup = format!(
        "<defs>{defs}</defs><g class=\"plan\">{}{}{}{}{}{}{}{}{}{}{}{}{}</g>{furniture}",
        l.wedge, l.arm, l.lane, l.measure, l.road, l.bulb, l.curb, l.cross, l.mark, l.sel, l.mv, l.label, l.grip
    );
    PlanSvg { frame: f, label: plan_label(v, i18n), markup }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::*;

    fn svg(j: &Junction) -> PlanSvg {
        plan_svg(&j.view(), &crate::i18n_for(crate::shared::i18n::Locale::En), 1000.0, 1000.0, Units::Metres)
    }

    fn count(s: &str, needle: &str) -> usize {
        s.matches(needle).count()
    }

    fn arm_at(j: &Junction, bearing: i32) -> u32 {
        j.current().arms.iter().find(|a| a.bearing == bearing).unwrap().uid
    }

    #[test]
    fn a_corner_without_a_sidewalk_is_drawn_bare() {
        let s = svg(&junction_of(&[("travel", 3200), ("travel", 3200)]));
        assert_eq!(count(&s.markup, "class=\"wedge bare\""), 4);
        assert_eq!(count(&s.markup, "class=\"wedge k-sidewalk\""), 0);
        let walked = svg(&junction_of(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]));
        assert_eq!(count(&walked.markup, "class=\"wedge k-sidewalk\""), 4);
    }

    #[test]
    fn numbers_are_written_to_a_tenth_without_a_trailing_zero() {
        assert_eq!(f1(12.0), "12");
        assert_eq!(f1(12.34), "12.3");
        assert_eq!(f1(-0.04), "0");
        assert_eq!(f1(0.05), "0.1");
        assert_eq!(f1(-3.26), "-3.3");
    }

    #[test]
    fn every_arm_is_drawn_with_a_grip_a_name_and_its_lane_arrows() {
        let j = Junction::new(0);
        let s = svg(&j);
        assert_eq!(count(&s.markup, "class=\"arm\""), 4);
        assert_eq!(count(&s.markup, "data-role=\"grip-arm\""), 4);
        assert_eq!(count(&s.markup, "class=\"lane-hit\""), 6);
        assert_eq!(count(&s.markup, "class=\"lane-arrow\""), 6);
        assert_eq!(count(&s.markup, "class=\"lane-arrow out\""), 6);
        assert!(s.markup.contains(">Sample Avenue 2</text>") && s.markup.contains("N · 20.0 m road"));
        assert!(!s.markup.contains("NaN"));
    }

    #[test]
    fn a_crossing_is_a_zebra_with_its_distance_written_beside_it() {
        let j = Junction::new(0);
        let s = svg(&j);
        assert_eq!(count(&s.markup, "class=\"zebra-fill\""), 4);
        assert_eq!(count(&s.markup, "<pattern id=\"zb-"), 4);
        assert!(s.markup.contains(">2 × 9.0</text>") && s.markup.contains(">11.4</text>"));
        let ft = plan_svg(&j.view(), &crate::i18n_for(crate::shared::i18n::Locale::En), 1000.0, 1000.0, Units::Feet);
        assert!(ft.markup.contains(">2 × 29.5</text>") && ft.markup.contains(">37.4</text>"));
        assert!(ft.markup.contains("20 m") == false && ft.markup.contains("66 ft"));
    }

    #[test]
    fn a_crossing_that_is_too_far_is_drawn_as_a_warning() {
        let mut j = Junction::new(0);
        let e = arm_at(&j, 90);
        j.set_island(e, false);
        // Widen the crossing's stage by taking the island away from the avenue.
        let n = arm_at(&j, 0);
        j.set_island(n, false);
        let s = svg(&j);
        let warn = count(&s.markup, "dim-warn");
        let too_far = j.view().arms.iter().filter(|a| a.crossing.as_ref().is_some_and(|c| c.too_far)).count();
        assert_eq!(warn, too_far);
    }

    #[test]
    fn nothing_selected_draws_no_outline_no_handles_other_than_the_arms_and_no_turns() {
        let s = svg(&Junction::new(0));
        assert_eq!(count(&s.markup, "sel-box") + count(&s.markup, "sel-line"), 0);
        assert_eq!(count(&s.markup, "class=\"mv"), 0);
        assert_eq!(count(&s.markup, "grip-corner") + count(&s.markup, "grip-crossing"), 0);
    }

    #[test]
    fn a_selected_arm_is_outlined_and_shows_where_its_traffic_may_go() {
        let mut j = Junction::new(0);
        let e = arm_at(&j, 90);
        j.select(Target::Arm(e));
        let s = svg(&j);
        assert_eq!(count(&s.markup, "class=\"arm on\""), 1);
        assert_eq!(count(&s.markup, "sel-box"), 1);
        assert_eq!(count(&s.markup, "class=\"mv"), 3);
        assert_eq!(count(&s.markup, "marker-end"), 3);
        assert!(s.markup.contains("t-blue"));
    }

    #[test]
    fn a_banned_turn_is_drawn_crossed_out_with_its_reason() {
        let mut j = Junction::new(0);
        let (e, n) = (arm_at(&j, 90), arm_at(&j, 0));
        j.set_turn(e, n, false);
        j.select(Target::Arm(e));
        let s = svg(&j);
        assert_eq!(count(&s.markup, "class=\"mv no\""), 1);
        assert_eq!(count(&s.markup, "class=\"mv-x\""), 1);
        assert_eq!(count(&s.markup, "marker-end"), 2);
    }

    #[test]
    fn a_selected_corner_shows_its_curb_and_a_handle() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        j.select(Target::Corner(n));
        let s = svg(&j);
        assert_eq!(count(&s.markup, "data-role=\"grip-corner\""), 1);
        assert_eq!(count(&s.markup, "class=\"sel-line\""), 1);
        assert!(s.markup.contains("Change the corner radius"));
    }

    #[test]
    fn a_selected_crossing_shows_a_handle_and_a_selected_lane_its_outline() {
        let mut j = Junction::new(0);
        let n = arm_at(&j, 0);
        j.select(Target::Crossing(n));
        let s = svg(&j);
        assert_eq!(count(&s.markup, "data-role=\"grip-crossing\""), 1);
        assert_eq!(count(&s.markup, "sel-box"), 1);
        j.select(Target::Lane(n, 1));
        let s = svg(&j);
        assert_eq!(count(&s.markup, "class=\"lane-hit on\""), 1);
        assert_eq!(count(&s.markup, "sel-box"), 1);
    }

    #[test]
    fn a_signal_stops_each_arm_with_a_signal_head() {
        let s = svg(&Junction::new(0));
        assert_eq!(count(&s.markup, "class=\"stop-line\""), 4);
        assert_eq!(count(&s.markup, "<circle cx=\"0\" cy=\"-5\""), 4);
    }

    #[test]
    fn an_all_way_stop_draws_stop_signs_and_no_control_draws_nothing() {
        let mut j = Junction::new(0);
        j.set_control(ALL_WAY_STOP);
        assert_eq!(count(&svg(&j).markup, "M-3.5 -8.5h7"), 4);
        j.set_control(0);
        let s = svg(&j);
        assert_eq!(count(&s.markup, "class=\"sign\""), 0);
    }

    #[test]
    fn a_roundabout_draws_its_ring_island_and_circulation_and_no_core() {
        let mut j = Junction::new(0);
        j.set_control(ROUNDABOUT);
        let s = svg(&j);
        assert_eq!(count(&s.markup, "class=\"road ring\""), 1);
        assert_eq!(count(&s.markup, "class=\"island k-median\""), 1);
        // One arrow on the ring between each pair of streets, and a yield line on each arm.
        assert_eq!(count(&s.markup, "class=\"lane-arrow\" transform"), 4 + 6);
        assert_eq!(count(&s.markup, "class=\"stop-line yield\""), 4);
        assert!(s.label.ends_with("Roundabout."));
    }

    #[test]
    fn a_roundabout_s_cycle_track_and_bus_lane_can_be_pressed_and_labelled() {
        let mut j = Junction::new(0);
        j.set_control(ROUNDABOUT);
        let (n, so) = (arm_at(&j, 0), arm_at(&j, 180));
        j.set_bus(Some((n, so)));
        j.set_cycle(Some(2_000));
        let s = svg(&j);
        assert_eq!(count(&s.markup, "data-role=\"cycle\""), 1);
        assert_eq!(count(&s.markup, "data-role=\"bus\""), 1);
        assert!(s.markup.contains(">Bus only</text>"));
        j.select(Target::Cycle);
        assert_eq!(count(&svg(&j).markup, "class=\"sel-line\""), 2);
        j.select(Target::Bus);
        assert_eq!(count(&svg(&j).markup, "sel-box"), 1);
    }

    #[test]
    fn measures_are_drawn_where_they_belong_with_their_codes() {
        let mut j = Junction::new(0);
        let e = arm_at(&j, 90);
        j.set_bus_lane(e, true);
        j.set_approach(e, Q_CURB);
        j.set_stop(e, STOP_BULB);
        j.set_filter(e, true);
        j.set_rule(e, RULE_DEAD_END);
        let s = svg(&j);
        assert_eq!(count(&s.markup, "class=\"piece k-bus queue\""), 1);
        assert_eq!(count(&s.markup, "class=\"bulb measure-stop k-sidewalk\""), 1);
        assert_eq!(count(&s.markup, "class=\"dead-cap\""), 1);
        assert!(count(&s.markup, "class=\"bollard\"") > 0);
        assert_eq!(count(&s.markup, "class=\"arm dead\""), 1);
        for code in ["G2", "M1", "N1", "L4"] {
            assert!(s.markup.contains(&format!(">{code}</text>")), "{code}");
        }
    }

    /// The side each street's name is anchored to, in arm order, for streets called `name`.
    fn anchors(markup: &str, name: &str) -> Vec<String> {
        let tail = format!("\">{name}</text>");
        markup
            .match_indices(&tail)
            .map(|(i, _)| {
                let head = &markup[..i];
                let at = head.rfind("text-anchor=\"").unwrap() + "text-anchor=\"".len();
                head[at..].to_string()
            })
            .collect()
    }

    #[test]
    fn a_street_that_is_shifted_says_so() {
        let mut j = Junction::new(0);
        let e = arm_at(&j, 90);
        j.set_offset(e, 500);
        assert!(svg(&j).markup.contains("· shifted 0.5 m"));
        assert!(!svg(&Junction::new(0)).markup.contains("shifted"));
    }

    #[test]
    fn a_street_s_name_sits_beside_a_sideways_arm_and_above_it_when_the_drawing_is_narrow() {
        let j = Junction::new(0);
        // Sample Street 1 is the east arm, then the west.
        assert_eq!(anchors(&svg(&j).markup, "Sample Street 1"), vec!["start", "end"]);
        let narrow = plan_svg(&j.view(), &crate::i18n_for(crate::shared::i18n::Locale::En), 400.0, 1000.0, Units::Metres);
        assert!(narrow.frame.narrow);
        assert_eq!(anchors(&narrow.markup, "Sample Street 1"), vec!["end", "start"]);
        // Streets that run up and down are centred either way.
        assert_eq!(anchors(&svg(&j).markup, "Sample Avenue 2"), vec!["middle", "middle"]);
    }

    #[test]
    fn on_a_narrow_drawing_a_sideways_arm_s_name_and_width_clear_the_road() {
        let j = Junction::new(0);
        let s = plan_svg(&j.view(), &crate::i18n_for(crate::shared::i18n::Locale::En), 390.0, 1000.0, Units::Metres);
        let centre = s.frame.at((0.0, 0.0)).1;
        let half_road = 11_400.0 * s.frame.scale / 2.0;
        let tail = ">Sample Street 1</text>";
        let found: Vec<_> = s.markup.match_indices(tail).collect();
        assert_eq!(found.len(), 2);
        let mut seen = [0; 2];
        for (i, _) in found {
            let head = &s.markup[..i];
            let at = head.rfind(" y=\"").unwrap() + 4;
            let y: f64 = head[at..].split('"').next().unwrap().parse().unwrap();
            // the width line is written 17 below the name: the one is above the road's edge, the other below it
            let above = y + 17.0 <= centre - half_road;
            let below = y >= centre + half_road + 10.0;
            assert!(above || below, "name at {y}, road from {} to {}", centre - half_road, centre + half_road);
            seen[usize::from(below)] += 1;
        }
        assert_eq!(seen, [1, 1], "one name above, one below");
    }

    #[test]
    fn the_scale_bar_and_north_mark_are_always_drawn() {
        let s = svg(&Junction::new(0));
        assert_eq!(count(&s.markup, "class=\"north\""), 1);
        assert_eq!(count(&s.markup, "class=\"bar-b\"") + count(&s.markup, "class=\"bar-w\""), 5);
        assert!(s.markup.contains(">20 m</text>"));
    }

    #[test]
    fn the_label_says_how_many_streets_there_are() {
        let s = svg(&Junction::new(1));
        assert_eq!(s.label, "Plan of the junction, north up. 3 streets.");
    }

    #[test]
    fn every_sample_junction_draws_without_a_missing_number() {
        for i in 0..4 {
            let mut j = Junction::new(i);
            for t in j.targets() {
                j.select(t);
                let s = svg(&j);
                assert!(!s.markup.contains("NaN") && !s.markup.contains("inf"), "sample {i} {t:?}");
            }
        }
    }
}
