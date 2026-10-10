//! The street's section as SVG markup, from the model's view: the street as it
//! is today above the design, each piece with its symbol, slab and dimension, the
//! handles to drag, and what hangs over when the pieces do not fit.

use std::fmt::Write;

use crate::shared::catalogue::{KINDS, kind_key, mark_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::say;
use crate::shared::symbols::{SymbolOpts, symbol};
use crate::shared::units::Units;
use crate::street::model::{SegView, View};
use crate::street::text::fit_phrase;

/// The heights of the drawing's bands, scaled with its width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rows {
    pub existing_label: f64,
    pub existing: f64,
    pub existing_height: f64,
    pub proposal_label: f64,
    pub dim: f64,
    pub top: f64,
    pub ground: f64,
    pub slab: f64,
    pub mark: f64,
    pub total: f64,
    pub total2: f64,
    pub scale: f64,
    pub height: f64,
}

impl Rows {
    fn at(f: f64) -> Rows {
        let r = |n: f64| (n * f).round();
        Rows {
            existing_label: r(16.0),
            existing: r(26.0),
            existing_height: r(36.0),
            proposal_label: r(92.0),
            dim: r(156.0),
            top: r(172.0),
            ground: r(312.0),
            slab: r(34.0),
            mark: r(368.0),
            total: r(408.0),
            total2: r(434.0),
            scale: r(466.0),
            height: r(484.0),
        }
    }
}

/// How the street sits in the drawing: millimetres to pixels, and the bands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    /// The drawing's size factor: bigger on a wide screen, to a point.
    pub factor: f64,
    pub rows: Rows,
    pub pad_left: f64,
    pub pad_right: f64,
    /// Pixels to the millimetre.
    pub scale: f64,
    pub width: f64,
}

impl Geometry {
    /// The geometry for a drawing `width` wide holding a street `row_mm` wide.
    pub fn fit(width: f64, row_mm: i32) -> Geometry {
        let width = width.max(680.0);
        let factor = (width / 1050.0).clamp(0.9, 1.1);
        let pad_left = 30.0;
        // Room right of the right-of-way line for the orange cloud when a street runs over.
        let pad_right = 60.0_f64.max((width * 0.17).round());
        Geometry { factor, rows: Rows::at(factor), pad_left, pad_right, scale: (width - pad_left - pad_right) / row_mm as f64, width }
    }

    /// Where a width in millimetres from the street's left edge is, in pixels.
    pub fn x(&self, mm: f64) -> f64 {
        self.pad_left + mm * self.scale
    }

    /// The street's width in millimetres to the left of a pixel position.
    pub fn mm_at(&self, px: f64) -> f64 {
        (px - self.pad_left) / self.scale
    }
}

/// A piece being dragged to a new place, and where the drop would put it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moving {
    pub uid: u32,
    /// The pointer, in pixels across the drawing.
    pub px: f64,
    /// The place among the other pieces it would take.
    pub index: Option<usize>,
}

/// What the pointer is doing to the drawing, which shows as it happens.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Interaction {
    /// The boundary being dragged, by the index of the piece to its left.
    pub resizing: Option<usize>,
    /// The right-hand edge of the last piece is being dragged.
    pub edge: bool,
    pub moving: Option<Moving>,
    /// The pointer is a finger, which needs more to land on.
    pub coarse: bool,
    /// The street has just gone over its width.
    pub fresh: bool,
}

pub struct StreetSvg {
    pub geometry: Geometry,
    pub label: String,
    pub markup: String,
    /// Where the right-of-way line is in pixels, for bringing a cloud into view.
    pub row_right: f64,
    pub over: bool,
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn dim_line(x0: f64, x1: f64, y: f64, class: &str) -> String {
    format!(
        "<path class=\"dim {class}\" d=\"M{x0} {y}H{x1}M{} {}L{} {}M{} {}L{} {}\"/>",
        x0 - 4.0,
        y + 4.0,
        x0 + 4.0,
        y - 4.0,
        x1 - 4.0,
        y + 4.0,
        x1 + 4.0,
        y - 4.0
    )
}

/// A revision cloud: a rectangle outlined with outward arcs.
fn cloud(x: f64, y: f64, w: f64, h: f64, r: f64, fresh: bool) -> String {
    let edge = |len: f64| ((len / (r * 2.2)).round() as usize).max(1);
    let (nx, ny) = (edge(w), edge(h));
    let (sx, sy) = (w / nx as f64, h / ny as f64);
    let arc = |dx: f64, dy: f64| format!("a{r} {r} 0 0 1 {dx} {dy}");
    let mut d = format!("M{x} {y}");
    for _ in 0..nx {
        d += &arc(sx, 0.0);
    }
    for _ in 0..ny {
        d += &arc(0.0, sy);
    }
    for _ in 0..nx {
        d += &arc(-sx, 0.0);
    }
    for _ in 0..ny {
        d += &arc(0.0, -sy);
    }
    format!("<path class=\"cloud{}\" pathLength=\"1\" d=\"{d}Z\"/>", if fresh { " fresh" } else { "" })
}

/// The surface finish, drawn as a paving course along the top of the slab.
fn surface_course(x: f64, y: f64, w: f64, material: &str) -> String {
    format!(
        "<rect class=\"surface\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"10\"/><rect class=\"hatch\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"10\" fill=\"url(#m-{material})\"/>"
    )
}

/// Traffic direction as an arrow: up runs away from you, down comes toward you.
/// The paler outline under the arrow keeps it readable over hatching.
pub fn dir_glyph(direction: &str, cx: f64, cy: f64) -> String {
    let d = if direction == "toward" { 1.0 } else { -1.0 };
    let path = format!("M{cx},{} V{} M{},{} L{cx},{} L{},{}", cy - 7.0 * d, cy + 7.0 * d, cx - 4.5, cy + 2.5 * d, cy + 7.0 * d, cx + 4.5, cy + 2.5 * d);
    format!("<path class=\"dir-halo\" d=\"{path}\"/><path class=\"dir\" d=\"{path}\"/>")
}

/// Marks a piece that is a different type at other times.
fn clock_badge(cx: f64, cy: f64) -> String {
    format!("<circle class=\"badge\" cx=\"{cx}\" cy=\"{cy}\" r=\"7\"/><path class=\"badge-hands\" d=\"M{cx},{} V{cy} L{},{}\"/>", cy - 4.0, cx + 3.0, cy + 2.0)
}

/// The kinds a curb is drawn beside: road pieces.
const ROAD: [&str; 4] = ["travel", "bus", "parking", "loading"];

/// A block on each side of a curbed piece that faces a road piece.
fn curbs(v: &View, g: &Geometry) -> String {
    let (gy, mut out) = (g.rows.ground, String::new());
    for (i, s) in v.segments.iter().enumerate() {
        let Some(curb) = s.curb else { continue };
        let x = g.x(s.x_mm as f64);
        let w = s.width_mm as f64 * g.scale;
        // A planted curb is a strip of planting; a Kassel kerb is wider with a
        // sloped road face; a boarding island is a wide raised platform; a
        // bike-friendly curb is a low ramp a wheel can ride up.
        let (planted, kassel, island, ramp) = (curb == "planted", curb == "kassel", curb == "island", curb == "bikefriendly");
        let cw = (if planted {
            (600.0 * g.scale).max(14.0)
        } else if island {
            (900.0 * g.scale).max(16.0)
        } else if kassel {
            (250.0 * g.scale).max(9.0)
        } else if ramp {
            (300.0 * g.scale).max(9.0)
        } else {
            (150.0 * g.scale).max(5.0)
        })
        .min(w / 2.0);
        let ch = if island { 16.0 } else { 12.0 };
        let fill = if planted { "m-planted".to_string() } else { format!("c-{curb}") };
        let kind_of = |n: Option<&SegView>| n.map(|n| KINDS[n.kind].id);
        let left = if i > 0 { v.segments.get(i - 1) } else { None };
        let mut sides = vec![(left, x, false), (v.segments.get(i + 1), x + w - cw, true)];
        // An island stands on one side only, the one facing the buses.
        if island {
            let rank = |id: &str| ["bus", "travel"].iter().position(|k| *k == id).map_or(-1, |p| p as i32);
            let mut facing: Vec<_> = sides.into_iter().filter(|(nb, _, _)| kind_of(*nb).is_some_and(|id| ROAD.contains(&id))).collect();
            facing.sort_by_key(|(nb, _, _)| std::cmp::Reverse(rank(kind_of(*nb).unwrap_or(""))));
            facing.truncate(1);
            sides = facing;
        }
        for (nb, bx, road_right) in sides {
            if !kind_of(nb).is_some_and(|id| ROAD.contains(&id)) {
                continue;
            }
            if kassel || ramp {
                // a: the road-side foot, b: the back
                let (a, b) = if road_right { (bx + cw, bx) } else { (bx, bx + cw) };
                let top = if ramp {
                    b
                } else if road_right {
                    bx + cw * 0.35
                } else {
                    bx + cw * 0.65
                };
                let h = if ramp { 7.0 } else { 12.0 };
                let pts = format!("{a},{gy} {b},{gy} {b},{} {top},{}", gy - h, gy - h);
                write!(out, "<polygon class=\"curb\" points=\"{pts}\"/><polygon class=\"hatch\" points=\"{pts}\" fill=\"url(#{fill})\"/>").unwrap();
                continue;
            }
            write!(
                out,
                "<rect class=\"curb\" x=\"{bx}\" y=\"{}\" width=\"{cw}\" height=\"{ch}\"/><rect class=\"hatch\" x=\"{bx}\" y=\"{}\" width=\"{cw}\" height=\"{ch}\" fill=\"url(#{fill})\"/>",
                gy - ch,
                gy - ch
            )
            .unwrap();
        }
    }
    out
}

/// What a screen reader is told the drawing is.
pub fn street_label(v: &View, i18n: &I18n, units: Units) -> String {
    let locale = i18n.locale();
    let args = Args::new()
        .str("name", say(i18n, units, &v.name))
        .num("count", v.segments.len() as i64)
        .str("total", units.length_fine_in(v.total_mm, locale))
        .str("row", units.length_fine_in(v.row_mm, locale))
        .str("fit", fit_phrase(v, i18n, units));
    i18n.tr("svg-label", &args)
}

/// The street drawn `width` wide: `ui` is what the pointer is doing to it.
pub fn street_svg(v: &View, i18n: &I18n, width: f64, units: Units, ui: &Interaction) -> StreetSvg {
    let g = Geometry::fit(width, v.row_mm);
    let (rows, scale) = (g.rows, g.scale);
    let w_px = g.width;
    let ground = rows.ground;
    let body_bottom = ground + rows.slab;
    let over = v.delta_mm > 0;
    let locale = i18n.locale();
    let fine = |mm: i32| units.fine_in(mm, locale);
    let length = |mm: i32| units.length_fine_in(mm, locale);
    // A word of the drawing, with the length it carries when there is one.
    let say = |key: &str| esc(&i18n.tr(key, &Args::new()));
    let say_length = |key: &str, mm: i32| esc(&i18n.tr(key, &Args::new().str("length", length(mm))));
    let mut p = String::new();

    // the street as it is today, at the same scale and origin as the design
    write!(p, "<text class=\"t-label t-soft\" x=\"{}\" y=\"{}\">{}</text>", g.pad_left, rows.existing_label, say("svg-today")).unwrap();
    for s in &v.existing {
        let id = KINDS[s.kind].id;
        let (x, w) = (g.x(s.x_mm as f64), s.width_mm as f64 * scale);
        write!(
            p,
            "<rect class=\"obj k-{id}\" x=\"{x}\" y=\"{}\" width=\"{w}\" height=\"{}\"/><rect class=\"hatch\" x=\"{x}\" y=\"{}\" width=\"{w}\" height=\"{}\" fill=\"url(#h-{id})\"/>",
            rows.existing, rows.existing_height, rows.existing, rows.existing_height
        )
        .unwrap();
        let label = fine(s.width_mm);
        if w > label.len() as f64 * 8.5 + 10.0 {
            write!(
                p,
                "<text class=\"t-dim t-halo\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{label}</text>",
                x + w / 2.0,
                rows.existing + rows.existing_height / 2.0 + 4.5
            )
            .unwrap();
        }
    }

    // the design's heading
    write!(p, "<text class=\"t-label\" x=\"{}\" y=\"{}\">{}</text>", g.pad_left + 30.0, rows.proposal_label, say("svg-your-design")).unwrap();
    if let Some(rev) = v.revisions.last() {
        let change = esc(&i18n.tr("svg-change", &Args::new().num("n", rev.step as i64)));
        write!(p, "<text class=\"t-label t-blue\" x=\"{}\" y=\"{}\">{change}</text>", g.pad_left + 130.0, rows.proposal_label).unwrap();
    }

    // right-of-way lines
    let (xl, xr) = (g.x(0.0), g.x(v.row_mm as f64));
    for x in [xl, xr] {
        write!(p, "<line class=\"rw\" x1=\"{x}\" x2=\"{x}\" y1=\"{}\" y2=\"{}\"/>", rows.dim - 34.0, rows.total2 + 6.0).unwrap();
    }
    let edge = say("svg-street-edge");
    write!(
        p,
        "<text class=\"t-label t-faint\" x=\"{xl}\" y=\"{}\" text-anchor=\"start\">{edge}</text><text class=\"t-label t-faint\" x=\"{xr}\" y=\"{}\" text-anchor=\"end\">{edge}</text>",
        rows.dim - 42.0,
        rows.dim - 42.0
    )
    .unwrap();

    // sky behind the section, within the right-of-way, and the ground
    write!(p, "<rect class=\"sky\" x=\"{xl}\" y=\"{}\" width=\"{}\" height=\"{}\"/>", rows.top, xr - xl, ground - rows.top).unwrap();
    let total_x = g.x(v.total_mm as f64);
    write!(p, "<line class=\"ground\" x1=\"{}\" x2=\"{}\" y1=\"{ground}\" y2=\"{ground}\"/>", xl - 14.0, (total_x.max(xr) + 14.0).min(w_px - 4.0)).unwrap();

    // unassigned space
    if v.delta_mm < 0 {
        let (x, w) = (total_x, xr - total_x);
        write!(p, "<rect class=\"free\" x=\"{x}\" y=\"{}\" width=\"{w}\" height=\"{}\"/>", rows.top, body_bottom - rows.top).unwrap();
        let t = say_length("svg-unused-length", -v.delta_mm);
        if w >= 88.0 {
            write!(
                p,
                "<text class=\"t-label t-faint\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text><text class=\"t-dim t-soft\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
                x + w / 2.0,
                ground - 8.0,
                say("svg-unused"),
                x + w / 2.0,
                ground + 10.0,
                length(-v.delta_mm)
            )
            .unwrap();
        } else if w >= 22.0 {
            write!(p, "<text class=\"t-label t-faint\" transform=\"translate({} {}) rotate(-90)\">{t}</text>", x + w / 2.0 + 4.0, ground - 8.0).unwrap();
        }
    }

    // pieces, dimension strings, marks
    for s in &v.segments {
        let k = &KINDS[s.kind];
        let (x, w) = (g.x(s.x_mm as f64), s.width_mm as f64 * scale);
        let cx = x + w / 2.0;
        let sel = v.selected == Some(s.uid);
        let lifted = ui.moving.is_some_and(|m| m.uid == s.uid);
        let opts = SymbolOpts { material: s.material, tram: s.tram };
        let full = i18n.tr(&kind_key(k.id), &Args::new());
        let name = if w > full.chars().count() as f64 * 8.6 + 10.0 {
            esc(&full)
        } else if w > 30.0 {
            esc(&i18n.tr(&mark_key(k.id), &Args::new()))
        } else {
            String::new()
        };
        write!(
            p,
            "<g class=\"seg{}\" data-role=\"seg\" data-uid=\"{}\"><rect class=\"hit\" x=\"{x}\" y=\"{}\" width=\"{w}\" height=\"{}\"/>{}<rect class=\"obj k-{}\" x=\"{x}\" y=\"{ground}\" width=\"{w}\" height=\"{}\"/><rect class=\"hatch\" x=\"{x}\" y=\"{ground}\" width=\"{w}\" height=\"{}\" fill=\"url(#h-{})\"/>{}",
            if lifted { " lifted" } else { "" },
            s.uid,
            rows.top,
            body_bottom - rows.top,
            symbol(k.id, cx, ground, scale * 1000.0, w, s.width_mm as f64 / 1000.0, g.factor, opts),
            k.id,
            rows.slab,
            rows.slab,
            k.id,
            surface_course(x, ground, w, s.material)
        )
        .unwrap();
        if let (Some(d), true) = (s.direction, w >= 26.0) {
            p += &dir_glyph(d, cx, ground + rows.slab / 2.0);
        }
        if k.id == "parking" && w >= 22.0 {
            write!(p, "<text class=\"slab-p\" x=\"{cx}\" y=\"{}\" text-anchor=\"middle\" font-size=\"20\">P</text>", ground + rows.slab / 2.0 + 7.0).unwrap();
        }
        if !s.variants.is_empty() && w >= 48.0 {
            p += &clock_badge(x + 24.0, ground + rows.slab - 10.0);
        }
        write!(p, "<text class=\"t-mark t-halo{}\" x=\"{cx}\" y=\"{}\" text-anchor=\"middle\">{name}</text></g>", if sel { " t-blue" } else { "" }, rows.mark)
            .unwrap();
        p += &dim_line(x, x + w, rows.dim, "");
        let label = fine(s.width_mm);
        if w > label.len() as f64 * 9.0 + 8.0 {
            write!(
                p,
                "<text class=\"t-dim t-halo{}\" x=\"{cx}\" y=\"{}\" text-anchor=\"middle\">{label}</text>",
                if sel { " t-blue" } else { "" },
                rows.dim - 7.0
            )
            .unwrap();
        }
        if sel {
            write!(p, "<rect class=\"sel-box\" x=\"{x}\" y=\"{}\" width=\"{w}\" height=\"{}\"/>", rows.dim - 28.0, rows.mark + 10.0 - (rows.dim - 28.0))
                .unwrap();
        }
    }

    // boundaries and their drag handles
    let n = v.segments.len();
    for i in 0..=n {
        let mm = if i == 0 { 0 } else { v.segments[i - 1].x_mm + v.segments[i - 1].width_mm };
        let x = g.x(mm as f64);
        let (interior, edge_of_last) = (i > 0 && i < n, i == n && n > 0);
        let line = format!("<line class=\"bound bound-line\" x1=\"{x}\" x2=\"{x}\" y1=\"{}\" y2=\"{body_bottom}\"/>", rows.dim - 8.0);
        if !interior && !edge_of_last {
            p += &line;
            continue;
        }
        let attrs =
            if interior { format!("data-role=\"handle\" data-i=\"{}\"", i - 1) } else { format!("data-role=\"edge\" data-uid=\"{}\"", v.segments[n - 1].uid) };
        let active = (interior && ui.resizing == Some(i - 1)) || (edge_of_last && ui.edge);
        let gy = ground + rows.slab / 2.0;
        // A finger needs more to land on than a pointer does (WCAG 2.5.8: 24px).
        let hit_half = if ui.coarse { 15.0 } else { 9.0 };
        write!(
            p,
            "<g class=\"handle{}\" {attrs}>{line}<rect class=\"hit\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/><rect class=\"grip\" x=\"{}\" y=\"{}\" width=\"18\" height=\"16\"/><path class=\"grip-arrow\" d=\"M{} {gy}H{}M{} {gy}l3 -3M{} {gy}l3 3M{} {gy}l-3 -3M{} {gy}l-3 3\"/></g>",
            if active { " active" } else { "" },
            x - hit_half,
            rows.dim - 8.0,
            hit_half * 2.0,
            body_bottom - (rows.dim - 8.0),
            x - 9.0,
            gy - 8.0,
            x - 5.0,
            x + 5.0,
            x - 5.0,
            x - 5.0,
            x + 5.0,
            x + 5.0
        )
        .unwrap();
    }

    // overflow: orange wash and revision cloud
    if over {
        let clipped = total_x > w_px - 10.0;
        let w = total_x.min(w_px - 10.0) - xr;
        let (top, inset) = (rows.dim - 38.0, 14.0_f64.min(w / 4.0));
        let h = rows.mark + 18.0 - top;
        write!(p, "<rect class=\"over-wash\" x=\"{xr}\" y=\"{top}\" width=\"{w}\" height=\"{h}\"/>").unwrap();
        p += &cloud(xr + inset, top, (w - inset * 2.0).max(14.0), h, 6.0, ui.fresh);
        let key = if clipped { "svg-too-wide-clipped" } else { "svg-too-wide" };
        write!(p, "<text class=\"t-over\" x=\"{}\" y=\"{}\">{}</text>", xr + 10.0, rows.dim - 62.0, say_length(key, v.delta_mm)).unwrap();
    }

    p += &curbs(v, &g);

    // overall dimension strings
    p += &dim_line(xl, xr, rows.total, "");
    write!(
        p,
        "<text class=\"t-dim t-halo\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
        (xl + xr) / 2.0,
        rows.total - 7.0,
        say_length("svg-street-width", v.row_mm)
    )
    .unwrap();
    if v.delta_mm != 0 {
        p += &dim_line(xl, total_x, rows.total2, if over { "dim-warn" } else { "" });
        write!(
            p,
            "<text class=\"t-dim t-halo {}\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            if over { "t-warn" } else { "t-soft" },
            (xl + total_x) / 2.0,
            rows.total2 - 7.0,
            say_length("svg-design-width", v.total_mm)
        )
        .unwrap();
    }

    // graphic scale bar: five equal parts, so the scale claim can be checked by eye
    {
        let unit_mm = match units {
            Units::Metres => 1000.0,
            Units::Feet => 304.8 * 5.0,
        };
        let (parts, y) = (5, rows.scale);
        let bar_w = unit_mm * parts as f64 * scale;
        write!(p, "<text class=\"t-label t-soft\" x=\"{}\" y=\"{}\">{}</text>", g.pad_left, y - 8.0, say("svg-scale")).unwrap();
        for i in 0..parts {
            write!(
                p,
                "<rect class=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"8\"/>",
                if i % 2 == 1 { "bar-w" } else { "bar-b" },
                g.pad_left + 60.0 + i as f64 * unit_mm * scale,
                y - 14.0,
                unit_mm * scale
            )
            .unwrap();
        }
        let step = if units == Units::Metres { 1 } else { 5 };
        for i in 0..=parts {
            let every = if bar_w > 260.0 { 1 } else { 5 };
            if i % every != 0 && i != parts {
                continue;
            }
            write!(
                p,
                "<text class=\"t-dim t-soft\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
                g.pad_left + 60.0 + i as f64 * unit_mm * scale,
                y + 12.0,
                i * step
            )
            .unwrap();
        }
        write!(p, "<text class=\"t-dim t-soft\" x=\"{}\" y=\"{}\">{}</text>", g.pad_left + 60.0 + bar_w + 8.0, y - 6.0, units.word()).unwrap();
    }

    // drag feedback
    if let Some(m) = ui.moving {
        if let Some(s) = v.segments.iter().find(|s| s.uid == m.uid) {
            let w = s.width_mm as f64 * scale;
            write!(p, "<rect class=\"ghost\" x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{}\"/>", m.px - w / 2.0, ground - 4.0, rows.slab + 8.0).unwrap();
            write!(
                p,
                "<text class=\"t-mark t-blue\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
                m.px,
                ground + rows.slab / 2.0 + 5.0,
                esc(&i18n.tr(&mark_key(KINDS[s.kind].id), &Args::new()))
            )
            .unwrap();
        }
        if let Some(index) = m.index {
            let mm: i32 = v.segments.iter().filter(|s| s.uid != m.uid).take(index).map(|s| s.width_mm).sum();
            let x = g.x(mm as f64);
            write!(
                p,
                "<line class=\"caret\" x1=\"{x}\" x2=\"{x}\" y1=\"{}\" y2=\"{}\"/><path class=\"caret-head\" d=\"M{} {}L{} {}L{x} {}Z\"/>",
                rows.dim - 14.0,
                body_bottom + 6.0,
                x - 6.0,
                rows.dim - 24.0,
                x + 6.0,
                rows.dim - 24.0,
                rows.dim - 13.0
            )
            .unwrap();
        }
    }

    StreetSvg { geometry: g, label: street_label(v, i18n, units), markup: p, row_right: xr, over }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;
    use crate::street::model::Editor;

    fn en() -> std::rc::Rc<I18n> {
        crate::i18n_for(Locale::En)
    }

    fn svg(e: &Editor) -> StreetSvg {
        street_svg(&e.view(), &en(), 1050.0, Units::Metres, &Interaction::default())
    }

    fn count(s: &str, needle: &str) -> usize {
        s.matches(needle).count()
    }

    fn uid(e: &Editor, i: usize) -> u32 {
        e.view().segments[i].uid
    }

    #[test]
    fn the_drawing_scales_with_its_width_within_limits_and_maps_millimetres_both_ways() {
        let g = Geometry::fit(1050.0, 18_000);
        assert_eq!(g.factor, 1.0);
        assert_eq!(g.rows, Rows::at(1.0));
        assert_eq!(g.rows.ground, 312.0);
        let wide = Geometry::fit(3000.0, 18_000);
        assert_eq!(wide.factor, 1.1);
        let narrow = Geometry::fit(100.0, 18_000);
        assert_eq!((narrow.width, narrow.factor), (680.0, 0.9));
        assert!(narrow.rows.ground < g.rows.ground);
        // Room is kept to the right of the street for what hangs over.
        assert_eq!(Geometry::fit(1000.0, 18_000).pad_right, 170.0);
        assert_eq!(Geometry::fit(680.0, 18_000).pad_right, 116.0_f64.max(60.0));
        assert!((g.mm_at(g.x(4321.0)) - 4321.0).abs() < 1e-9);
        assert_eq!(g.x(0.0), 30.0);
        assert!((g.x(18_000.0) - (1050.0 - g.pad_right)).abs() < 1e-9);
    }

    #[test]
    fn every_piece_is_drawn_with_its_symbol_slab_and_dimension_and_each_boundary_has_a_handle() {
        let s = svg(&Editor::new(0));
        assert_eq!(count(&s.markup, "class=\"seg\""), 6);
        assert_eq!(count(&s.markup, "class=\"sym\""), 6);
        assert_eq!(count(&s.markup, "data-role=\"handle\""), 5);
        assert_eq!(count(&s.markup, "data-role=\"edge\""), 1);
        assert_eq!(count(&s.markup, "class=\"bound bound-line\""), 7);
        assert_eq!(count(&s.markup, "class=\"surface\""), 6);
        assert!(s.markup.contains(">Street width 18.0 m<"));
        assert!(!s.markup.contains("NaN"));
    }

    #[test]
    fn the_street_as_it_is_today_is_drawn_above_the_design() {
        let s = svg(&Editor::new(0));
        assert!(s.markup.contains(">Today<") && s.markup.contains(">Your design<"));
        assert_eq!(count(&s.markup, "class=\"obj k-"), 6 + 6);
        assert!(!s.markup.contains(">Change "));
    }

    #[test]
    fn the_latest_change_is_numbered_in_the_heading() {
        let mut e = Editor::new(0);
        let u = uid(&e, 0);
        e.nudge_width(u, 100);
        assert!(svg(&e).markup.contains(">Change 1<"));
    }

    #[test]
    fn a_full_street_has_no_unused_space_and_no_cloud() {
        let s = svg(&Editor::new(0));
        for none in ["class=\"free\"", "over-wash", "class=\"cloud", "dim-warn", ">Your design 18.0 m<"] {
            assert!(!s.markup.contains(none), "{none}");
        }
        assert!(!s.over);
    }

    #[test]
    fn room_left_is_shown_as_unused_space_and_a_second_dimension() {
        let mut e = Editor::new(0);
        let u = uid(&e, 0);
        e.remove(u);
        let s = svg(&e);
        assert_eq!(count(&s.markup, "class=\"free\""), 1);
        assert!(s.markup.contains(">Unused<") && s.markup.contains(">3.3 m<"));
        assert!(s.markup.contains(">Your design 14.7 m<") && s.markup.contains("t-dim t-halo t-soft"));
        assert!(!s.markup.contains("dim-warn"));
    }

    #[test]
    fn a_street_that_is_too_full_is_washed_in_orange_inside_a_cloud_that_says_by_how_much() {
        let mut e = Editor::new(0);
        let u = uid(&e, 0);
        e.set_width(u, e.view().segments[0].width_mm + 1_000);
        let s = svg(&e);
        assert!(s.over);
        assert_eq!(count(&s.markup, "class=\"over-wash\""), 1);
        assert_eq!(count(&s.markup, "class=\"cloud\""), 1);
        assert!(s.markup.contains(">1.0 m too wide<"));
        assert!(s.markup.contains("dim-warn") && s.markup.contains("t-warn"));
        assert!(!s.markup.contains("free"));
    }

    #[test]
    fn a_cloud_that_has_just_appeared_says_so_so_it_can_animate() {
        let mut e = Editor::new(0);
        let u = uid(&e, 0);
        e.set_width(u, e.view().segments[0].width_mm + 1_000);
        let fresh = street_svg(&e.view(), &en(), 1050.0, Units::Metres, &Interaction { fresh: true, ..Interaction::default() });
        assert!(fresh.markup.contains("class=\"cloud fresh\""));
    }

    #[test]
    fn a_street_wider_than_the_drawing_says_some_of_it_is_off_screen() {
        let mut e = Editor::new(0);
        let u = uid(&e, 0);
        e.set_width(u, e.view().segments[0].width_mm + 6_000);
        let s = svg(&e);
        // The over-wide street reaches beyond the drawing.
        assert_eq!(s.markup.contains("(more off-screen)"), s.geometry.x(e.view().total_mm as f64) > 1040.0);
    }

    #[test]
    fn the_selected_piece_is_outlined_and_its_name_and_width_are_blue() {
        let mut e = Editor::new(0);
        let u = uid(&e, 2);
        e.select(Some(u));
        let s = svg(&e);
        assert_eq!(count(&s.markup, "class=\"sel-box\""), 1);
        assert_eq!(count(&s.markup, "t-mark t-halo t-blue"), 1);
        assert_eq!(count(&s.markup, "t-dim t-halo t-blue"), 1);
    }

    #[test]
    fn a_handle_being_dragged_is_marked_active() {
        let e = Editor::new(0);
        let drawn = |ui: Interaction| street_svg(&e.view(), &en(), 1050.0, Units::Metres, &ui).markup;
        let h = drawn(Interaction { resizing: Some(2), ..Interaction::default() });
        assert_eq!(count(&h, "class=\"handle active\""), 1);
        assert!(h.contains("class=\"handle active\" data-role=\"handle\" data-i=\"2\""));
        let edge = drawn(Interaction { edge: true, ..Interaction::default() });
        assert!(edge.contains("class=\"handle active\" data-role=\"edge\""));
        assert_eq!(count(&drawn(Interaction::default()), "handle active"), 0);
    }

    #[test]
    fn a_finger_gets_a_bigger_target_on_each_handle() {
        let e = Editor::new(0);
        let hit = |coarse: bool| street_svg(&e.view(), &en(), 1050.0, Units::Metres, &Interaction { coarse, ..Interaction::default() }).markup;
        assert!(hit(false).contains("width=\"18\" height=\"") && !hit(false).contains("width=\"30\""));
        assert!(hit(true).contains("width=\"30\" height=\""));
    }

    #[test]
    fn a_piece_being_moved_is_lifted_with_a_ghost_and_a_caret_where_it_would_land() {
        let mut e = Editor::new(0);
        let u = uid(&e, 2);
        e.select(Some(u));
        let ui = Interaction { moving: Some(Moving { uid: u, px: 400.0, index: Some(1) }), ..Interaction::default() };
        let s = street_svg(&e.view(), &en(), 1050.0, Units::Metres, &ui);
        assert_eq!(count(&s.markup, "class=\"seg lifted\""), 1);
        assert_eq!(count(&s.markup, "class=\"ghost\""), 1);
        assert_eq!(count(&s.markup, "class=\"caret\""), 1);
        assert_eq!(count(&s.markup, "class=\"caret-head\""), 1);
        // The caret is after the first of the other pieces.
        let g = s.geometry;
        let x = g.x(3_300.0);
        assert!(s.markup.contains(&format!("<line class=\"caret\" x1=\"{x}\"")));
        let none = street_svg(
            &e.view(),
            &en(),
            1050.0,
            Units::Metres,
            &Interaction { moving: Some(Moving { uid: u, px: 400.0, index: None }), ..Interaction::default() },
        );
        assert_eq!(count(&none.markup, "class=\"caret\""), 0);
        assert_eq!(count(&none.markup, "class=\"ghost\""), 1);
    }

    #[test]
    fn parking_is_marked_p_and_a_piece_that_changes_through_the_day_has_a_clock() {
        let mut e = Editor::new(0);
        assert_eq!(count(&svg(&e).markup, ">P</text>"), 2);
        assert_eq!(count(&svg(&e).markup, "class=\"badge\""), 0);
        let u = uid(&e, 1);
        assert!(e.add_variant(u));
        assert_eq!(count(&svg(&e).markup, "class=\"badge\""), 1);
    }

    #[test]
    fn a_one_way_piece_shows_its_direction() {
        let e = Editor::new(0);
        let one_way = e.view().segments.iter().filter(|s| s.direction.is_some()).count();
        assert_eq!(one_way, 2);
        assert_eq!(count(&svg(&e).markup, "class=\"dir\""), one_way);
    }

    #[test]
    fn a_direction_arrow_points_up_away_from_you_and_down_toward_you() {
        assert_eq!(
            dir_glyph("away", 10.0, 10.0),
            "<path class=\"dir-halo\" d=\"M10,17 V3 M5.5,7.5 L10,3 L14.5,7.5\"/><path class=\"dir\" d=\"M10,17 V3 M5.5,7.5 L10,3 L14.5,7.5\"/>"
        );
        assert!(dir_glyph("toward", 10.0, 10.0).contains("d=\"M10,3 V17 M5.5,12.5 L10,17 L14.5,12.5\""));
    }

    #[test]
    fn a_piece_too_narrow_for_its_name_is_marked_with_its_letters_and_a_tiny_one_with_nothing() {
        let narrow = |mm: i32| {
            let mut e = Editor::new(0);
            let u = uid(&e, 0);
            if mm != e.view().segments[0].width_mm {
                assert!(e.set_width(u, mm), "{mm}");
            }
            let s = street_svg(&e.view(), &en(), 680.0, Units::Metres, &Interaction::default());
            let at = s.markup.find("t-mark t-halo\" x=").unwrap();
            let tail = &s.markup[at..];
            tail[tail.find('>').unwrap() + 1..tail.find("</text>").unwrap()].to_string()
        };
        let sidewalk = &KINDS[0];
        assert_eq!(sidewalk.id, "sidewalk");
        assert_eq!(narrow(sidewalk.min_mm), sidewalk.mark);
        assert_eq!(narrow(3_300), "Sidewalk");
    }

    #[test]
    fn a_curb_is_drawn_beside_a_road_piece_and_only_there() {
        let e = Editor::new(0);
        // The sidewalks have a curb, and each faces a parking lane.
        let s = svg(&e);
        assert_eq!(count(&s.markup, "class=\"curb\""), 2);
        assert!(s.markup.contains("url(#c-concrete)"));
    }

    #[test]
    fn every_curb_a_piece_can_have_draws() {
        let mut e = Editor::new(1);
        assert!(e.apply_measure("E1"));
        let segs = e.view().segments.len();
        for i in 0..segs {
            let u = uid(&e, i);
            for c in 0..crate::shared::catalogue::CURBS.len() {
                if e.set_curb(u, Some(c)) {
                    let s = svg(&e);
                    assert!(!s.markup.contains("NaN"), "curb {c} on {i}");
                }
            }
        }
    }

    #[test]
    fn the_scale_bar_has_five_parts_in_the_units_shown() {
        let s = svg(&Editor::new(0));
        assert_eq!(count(&s.markup, "class=\"bar-b\"") + count(&s.markup, "class=\"bar-w\""), 5);
        assert!(s.markup.contains(">5</text>") && s.markup.contains(">m</text>"));
        let ft = street_svg(&Editor::new(0).view(), &en(), 1050.0, Units::Feet, &Interaction::default());
        assert!(ft.markup.contains(">25</text>") && ft.markup.contains(">ft</text>"));
        assert!(ft.markup.contains(">Street width 59.1 ft<"));
    }

    #[test]
    fn the_scale_bar_labels_every_part_when_there_is_room_and_only_the_ends_when_there_is_not() {
        let roomy = street_svg(&Editor::new(0).view(), &en(), 1500.0, Units::Metres, &Interaction::default());
        assert_eq!(count(&roomy.markup, "t-dim t-soft\" x="), 6 + 1);
        let tight = street_svg(&Editor::new(0).view(), &en(), 680.0, Units::Metres, &Interaction::default());
        assert_eq!(count(&tight.markup, "t-dim t-soft\" x="), 2 + 1);
    }

    #[test]
    fn the_label_describes_the_street_and_how_the_pieces_fit() {
        let s = svg(&Editor::new(0));
        assert_eq!(s.label, "Cross-section of Sample Street 1. 6 segments, 18.0 m of 18.0 m. Every metre of the street is used.");
    }

    #[test]
    fn a_narrow_piece_is_named_by_its_mark_and_a_very_narrow_one_is_not_named() {
        let mut e = Editor::new(0);
        let u = uid(&e, 0);
        e.set_width(u, 1_800);
        let s = street_svg(&e.view(), &en(), 680.0, Units::Metres, &Interaction::default());
        assert!(
            s.markup.contains(">Sw</text>")
                || s.markup.contains(">SW</text>")
                || s.markup.contains("text-anchor=\"middle\"></text>")
                || s.markup.contains(">Sidewalk<")
        );
    }

    #[test]
    fn every_sample_street_draws_without_a_missing_number() {
        for i in 0..4 {
            let mut e = Editor::new(i);
            for k in 0..e.view().segments.len() {
                let u = uid(&e, k);
                e.select(Some(u));
                assert!(!svg(&e).markup.contains("NaN"), "sample {i} piece {k}");
            }
        }
    }

    fn fr() -> std::rc::Rc<I18n> {
        crate::i18n_for(Locale::FrCa)
    }

    #[test]
    fn the_drawing_says_its_words_in_french_and_not_in_english() {
        let mut e = Editor::new(0);
        let u = uid(&e, 0);
        e.nudge_width(u, 100);
        e.nudge_width(u, -100);
        let v = e.view();
        let s = street_svg(&v, &fr(), 1050.0, Units::Metres, &Interaction::default());
        for want in
            [">Aujourd’hui<", ">Votre aménagement<", ">Variation 2<", ">Limite de la rue<", ">Échelle<", ">Trottoir<", ">Largeur de la rue\u{a0}: 18,0\u{a0}m<"]
        {
            assert!(s.markup.contains(want), "{want}");
        }
        for not in ["Today", "Your design", "Change ", "Street edge", ">Scale<", ">Sidewalk<", "Street width"] {
            assert!(!s.markup.contains(not), "{not}");
        }
        assert_eq!(s.label, "Coupe transversale de Sample Street 1. 6 éléments, 18,0\u{a0}m sur 18,0\u{a0}m. Chaque mètre de la rue est utilisé.");
    }

    #[test]
    fn the_drawing_prints_its_lengths_with_the_decimal_comma_of_french() {
        let mut e = Editor::new(0);
        let u = uid(&e, 0);
        e.remove(u);
        let v = e.view();
        let fr_svg = street_svg(&v, &fr(), 1050.0, Units::Metres, &Interaction::default());
        let en_svg = street_svg(&v, &en(), 1050.0, Units::Metres, &Interaction::default());
        // The piece widths, the unused space and the two totals.
        assert!(
            fr_svg.markup.contains(">3,3\u{a0}m<") && fr_svg.markup.contains(">Inutilisé<"),
            "French SVG did not contain expected localized markers"
        );
        assert!(fr_svg.markup.contains("Votre aménagement\u{a0}: 14,7\u{a0}m"));
        assert!(en_svg.markup.contains(">3.3 m<") && en_svg.markup.contains(">Your design 14.7 m<"));
        for dotted in [">3.3", "14.7 m", "14.7\u{a0}m", "18.0 m", "18.0\u{a0}m"] {
            assert!(!fr_svg.markup.contains(dotted), "{dotted}");
        }
        let ft = street_svg(&v, &fr(), 1050.0, Units::Feet, &Interaction::default());
        assert!(ft.markup.contains("48,2\u{a0}ft"), "{}", ft.markup);
    }

    #[test]
    fn a_street_too_wide_says_by_how_much_in_each_language() {
        let mut e = Editor::new(0);
        let u = uid(&e, 0);
        e.set_width(u, e.view().segments[0].width_mm + 1_000);
        let v = e.view();
        let s = street_svg(&v, &fr(), 1050.0, Units::Metres, &Interaction::default());
        assert!(
            s.markup.contains(">1,0\u{a0}m de trop<") && !s.markup.contains("too wide"),
            "Expected French overflow wording and no English fallback"
        );
    }

    #[test]
    fn a_narrow_piece_shows_its_french_mark_and_a_one_piece_street_is_counted_in_the_singular() {
        let mut e = Editor::new(0);
        let u = uid(&e, 0);
        e.set_width(u, KINDS[0].min_mm);
        let s = street_svg(&e.view(), &fr(), 680.0, Units::Metres, &Interaction::default());
        assert!(
            s.markup.contains(">TO</text>"),
            "Expected French mark TO in SVG markup"
        );
        let mut one = Editor::new(0);
        while one.view().segments.len() > 1 {
            let u = uid(&one, 0);
            one.remove(u);
        }
        assert!(street_label(&one.view(), &en(), Units::Metres).contains(". 1 segment, "));
        assert!(street_label(&one.view(), &fr(), Units::Metres).contains(". 1 élément, "));
    }

    #[test]
    fn the_drawing_is_made_again_in_the_language_the_shell_switches() {
        let owner = leptos::prelude::Owner::new();
        owner.set();
        let i18n = en();
        let v = Editor::new(0).view();
        // What the drawing's closure does: it makes the drawing from the shared words, so it tracks them.
        use leptos::prelude::{Get, StoredValue, WithValue};
        let shared = StoredValue::new_local((i18n.clone(), v));
        let markup =
            leptos::prelude::Memo::new(move |_| shared.with_value(|(i18n, v)| street_svg(v, i18n, 1050.0, Units::Metres, &Interaction::default()).markup));
        assert!(markup.get().contains(">Today<") && !markup.get().contains("Aujourd’hui"));
        i18n.set(Locale::FrCa);
        let m = markup.get();
        assert!(m.contains(">Aujourd’hui<") && !m.contains(">Today<"), "{m}");
    }
}
