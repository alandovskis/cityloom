//! A layered diagram as flat items (polygons, lines, words), for the design-review PDF, which has no CSS: the walker
//! gives each shape the colour its classes have in the light theme (the table below, pinned to the stylesheets by a test).

use crate::shared::diagram::{Diagram, Face, Layer, LayerKind, Projection, Shape};
use cityloom_report::{Figure, Item, Rgb};

/// What a polygon is painted with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub fill: Rgb,
    pub stroke: Option<(Rgb, f32)>,
}

/// How much of its colour the face turned from the eye keeps (the 2.5D's `SHADES`, `street/iso/raised.rs`; a ramp is
/// between the two).
pub const FRONT_SHADE: f32 = 0.9;
pub const SIDE_SHADE: f32 = 0.78;
pub const RAMP_SHADE: f32 = 0.84;

/// The outline of a top, a front and a side (`--ink-2`) and its width.
const EDGE_WIDTH: f32 = 1.0;
const JOINT_WIDTH: f32 = 0.8;
const PAINT_WIDTH: f32 = 1.5;
const LINE_WIDTH: f32 = 1.0;
/// The outline of a solid's faces (`.iso-solid polygon`), of paint (`.iso-paint`), of a junction's flat piece
/// (`#drawing .iso-flat`), of a sign's panel, and of a stop's pad.
const SOLID_EDGE_WIDTH: f32 = 0.8;
const PAINT_EDGE_WIDTH: f32 = 0.5;
const FLAT_EDGE_WIDTH: f32 = 0.6;
const SIGN_EDGE_WIDTH: f32 = 1.0;
const STOP_EDGE_WIDTH: f32 = 1.5;
/// A word's size where its shape names none (a caption).
const CAPTION_SIZE: f32 = 12.0;
/// A caption's size in a figure fitted to its box (`flatten_fit`), in the box's own units, whatever the drawing's scale.
const FIT_CAPTION_SIZE: f32 = 6.5;
/// The average advance of a glyph, as a fraction of its size (conservative: wider than a proportional face's average),
/// and the height of a word above and below its baseline, for the box a word is judged to take.
const ADVANCE: f32 = 0.6;
const ASCENT: f32 = 0.8;
const DESCENT: f32 = 0.25;

/// How wide a word is estimated to be set at `size`.
fn text_width(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * ADVANCE
}

/// Where a colour comes from in the stylesheets: the file and the token (`--name`).
#[derive(Clone, Copy)]
pub enum Css {
    App,
    Editor,
}

/// The light-theme colours the figures use, each with the token it copies (`every_flat_colour_matches_the_css`).
pub const COLOURS: &[(&str, Css, Rgb)] = &[
    ("ink-2", Css::App, Rgb(0x4b, 0x55, 0x63)),
    ("fill", Css::App, Rgb(0xf1, 0xf3, 0xf5)),
    ("k-sidewalk", Css::App, Rgb(0xcf, 0xd7, 0xde)),
    ("k-planting", Css::App, Rgb(0xa5, 0xd6, 0x96)),
    ("k-bike", Css::App, Rgb(0x7f, 0xd3, 0xc3)),
    ("k-bus", Css::App, Rgb(0xf2, 0xa9, 0x9b)),
    ("k-median", Css::App, Rgb(0x93, 0xc9, 0x8a)),
    ("k-loading", Css::App, Rgb(0xf6, 0xd7, 0x77)),
    ("k-shoulder", Css::App, Rgb(0xcb, 0xd3, 0xdb)),
    ("k-bikerack", Css::App, Rgb(0xcd, 0xd6, 0xe2)),
    ("k-terrace", Css::App, Rgb(0xf1, 0xdc, 0xa0)),
    ("k-buffer", Css::App, Rgb(0xbd, 0xb8, 0xae)),
    ("k-travel", Css::Editor, Rgb(0xc3, 0xcb, 0xd3)),
    ("k-parking", Css::Editor, Rgb(0xb4, 0xbd, 0xc7)),
    ("mat-asphalt", Css::App, Rgb(0x3b, 0x3f, 0x43)),
    ("mat-concrete", Css::App, Rgb(0xbd, 0xc0, 0xc1)),
    ("mat-permeable", Css::App, Rgb(0x9a, 0x9a, 0x95)),
    ("mat-brick", Css::App, Rgb(0xa4, 0x52, 0x3b)),
    ("mat-grass", Css::App, Rgb(0x5f, 0x8f, 0x3e)),
    ("mat-planted", Css::App, Rgb(0x4e, 0x42, 0x30)),
    ("mat-gravel", Css::App, Rgb(0xb3, 0xaa, 0x98)),
    ("mat-trees", Css::App, Rgb(0x6a, 0x63, 0x58)),
    ("mat-concrete-line", Css::App, Rgb(0x7c, 0x80, 0x82)),
    ("mat-permeable-line", Css::App, Rgb(0x66, 0x66, 0x5f)),
    ("mat-brick-line", Css::App, Rgb(0xd6, 0xc4, 0xae)),
    ("mat-trees-alt", Css::App, Rgb(0x38, 0x33, 0x2b)),
    ("iso-curb", Css::Editor, Rgb(0xa3, 0xad, 0xb8)),
    ("iso-paint-white", Css::Editor, Rgb(0xfb, 0xfb, 0xf8)),
    ("iso-paint-yellow", Css::Editor, Rgb(0xe3, 0xa6, 0x00)),
    ("iso-paint-edge", Css::Editor, Rgb(0x10, 0x15, 0x1a)),
    ("ink", Css::App, Rgb(0x11, 0x18, 0x27)),
    ("accent", Css::App, Rgb(0x1f, 0x5e, 0xe0)),
    ("accent-wash", Css::App, Rgb(0xe8, 0xef, 0xfd)),
    ("k-bench", Css::App, Rgb(0xd8, 0xc8, 0xad)),
    ("k-busshelter", Css::App, Rgb(0xf6, 0xc9, 0xbf)),
    ("s-tree", Css::Editor, Rgb(0x6f, 0xb3, 0x5e)),
    ("iso-sign-panel", Css::Editor, Rgb(0xfb, 0xfb, 0xf8)),
    ("iso-sign-ink", Css::Editor, Rgb(0x10, 0x15, 0x1a)),
];

fn lookup(name: &str) -> Option<Rgb> {
    COLOURS.iter().find(|c| c.0 == name).map(|c| c.2)
}

fn colour(name: &str) -> Rgb {
    lookup(name).unwrap_or(NEUTRAL)
}

/// Grey, for a class the table does not know: it is drawn, and plainly not a material.
const NEUTRAL: Rgb = Rgb(0x9a, 0xa4, 0xb0);

fn shaded(c: Rgb, f: f32) -> Rgb {
    let s = |v: u8| (v as f32 * f).round() as u8;
    Rgb(s(c.0), s(c.1), s(c.2))
}

fn paint_of(class: &str) -> Option<Rgb> {
    let has = |w: &str| class.split_whitespace().any(|x| x == w);
    has("iso-paint").then(|| colour(if has("yellow") { "iso-paint-yellow" } else { "iso-paint-white" }))
}

/// The paint of a polygon of this class (see `style_in`), with no enclosing group.
pub fn style_of(class: &str, face: Option<Face>, material: Option<&str>) -> Style {
    style_in("", class, face, material)
}

/// The paint of a polygon of `class` inside groups of classes `context` (nearest first): a solid's faces (`o-front`,
/// `o-side`, `o-top`) have no colour of their own and take it from their group (`s-crown`, `s-trunk`, `s-shelter`,
/// `s-post`), as a sign's panel does. The colour is the material's (the argument, else the class's `mat-*`), else
/// the paint's, a sign's, a solid's, the curb's, the ground's, a stop's or its kind's (`k-*`), else grey; the faces
/// turned from the eye are darker; the outline is the one the stylesheet gives that class.
pub fn style_in(context: &str, class: &str, face: Option<Face>, material: Option<&str>) -> Style {
    let words: Vec<&str> = class.split_whitespace().chain(context.split_whitespace()).collect();
    let has = |w: &str| words.contains(&w);
    let material = material.or_else(|| words.iter().find_map(|w| w.strip_prefix("mat-")));
    let paint = has("iso-paint") || has("iso-zebra") || has("iso-stopline");
    let base = material
        .and_then(|m| lookup(&format!("mat-{m}")))
        .or_else(|| paint.then(|| colour(if has("yellow") { "iso-paint-yellow" } else { "iso-paint-white" })))
        .or_else(|| (has("iso-sign-panel") || has("iso-sign-glyph-cut")).then(|| colour("iso-sign-panel")))
        .or_else(|| has("iso-sign-glyph").then(|| colour("iso-sign-ink")))
        .or_else(|| has("s-crown").then(|| colour("s-tree")))
        .or_else(|| has("s-trunk").then(|| colour("k-bench")))
        .or_else(|| has("s-shelter").then(|| colour("k-busshelter")))
        .or_else(|| has("s-post").then(|| colour("ink-2")))
        .or_else(|| has("lane-arrow").then(|| colour("iso-paint-white")))
        .or_else(|| has("iso-curb").then(|| colour("iso-curb")))
        .or_else(|| has("iso-ground").then(|| colour("fill")))
        .or_else(|| (has("stop-pad") || has("stop-dwell")).then(|| colour("accent-wash")))
        .or_else(|| words.iter().find_map(|w| w.strip_prefix("k-").and_then(|k| lookup(&format!("k-{k}")))))
        .unwrap_or(NEUTRAL);
    let fill = match face {
        Some(Face::Front) => shaded(base, FRONT_SHADE),
        Some(Face::Side) => shaded(base, SIDE_SHADE),
        Some(Face::Ramp) => shaded(base, RAMP_SHADE),
        Some(Face::Top) | None => base,
    };
    // Faces of one tint would run together without an outline.
    let edged = ["iso-top", "iso-side", "iso-front", "iso-ramp", "iso-curb"].iter().any(|w| has(w));
    let stroke = if has("iso-solid") {
        Some((colour("ink-2"), SOLID_EDGE_WIDTH))
    } else if has("iso-sign-panel") {
        Some((colour("iso-sign-ink"), SIGN_EDGE_WIDTH))
    } else if paint {
        Some((colour("iso-paint-edge"), PAINT_EDGE_WIDTH))
    } else if has("stop-pad") || has("stop-dwell") {
        Some((colour("accent"), STOP_EDGE_WIDTH))
    } else if has("iso-flat") {
        Some((colour("ink"), FLAT_EDGE_WIDTH))
    } else {
        edged.then(|| (colour("ink-2"), EDGE_WIDTH))
    };
    Style { fill, stroke }
}

struct Walk<'a> {
    p: &'a dyn Projection,
    scale: f32,
    items: Vec<Item>,
    omitted: usize,
    /// A caption's size when it is not the drawing's scale's (`flatten_fit`).
    caption: Option<f32>,
    /// The words, with how much each matters when two would overprint (0 first): the item's index and its rank.
    ranks: Vec<(usize, u8)>,
    /// The classes of the groups the walk is inside, nearest first.
    context: String,
}

impl Walk<'_> {
    fn pts(&self, pts: &[crate::shared::diagram::Pt]) -> Vec<(f32, f32)> {
        pts.iter()
            .map(|&pt| {
                let (x, y) = self.p.at(pt);
                (x as f32 * self.scale, y as f32 * self.scale)
            })
            .collect()
    }

    fn shape(&mut self, s: &Shape) {
        match s {
            Shape::Polygon { pts, class, face, texture } => {
                let st = style_in(&self.context, class, *face, texture.as_ref().map(|t| t.material.as_str()));
                self.items.push(Item::Poly { pts: self.pts(pts), fill: st.fill, stroke: st.stroke });
            }
            Shape::Plane { pts, class, face, .. } => {
                let st = style_in(&self.context, class, Some(*face), None);
                self.items.push(Item::Poly { pts: self.pts(pts), fill: st.fill, stroke: st.stroke });
            }
            Shape::Polyline { pts, class } => {
                let (color, width) = match paint_of(class) {
                    Some(c) => (c, PAINT_WIDTH),
                    None => (colour("ink-2"), LINE_WIDTH),
                };
                self.items.push(Item::Line { pts: self.pts(pts), color, width });
            }
            Shape::Joints { runs, class } => {
                let color = class
                    .split_whitespace()
                    .find_map(|w| w.strip_prefix("j-"))
                    .and_then(|m| lookup(&if m == "trees" { "mat-trees-alt".to_string() } else { format!("mat-{m}-line") }))
                    .unwrap_or_else(|| colour("ink-2"));
                for run in runs {
                    self.items.push(Item::Line { pts: self.pts(&[run.from, run.to]), color, width: JOINT_WIDTH });
                }
            }
            Shape::Text { at, size, text, .. } => {
                // The stylesheet centres a shape's words on their point (`text-anchor: middle`); an item starts at its left.
                let (x, y) = self.p.at(*at);
                let size = *size as f32 * self.scale;
                let x = x as f32 * self.scale - text_width(text, size) / 2.0;
                self.word((x, y as f32 * self.scale), size, text, 1);
            }
            Shape::Caption { px, text, anchor, class } => {
                let size = self.caption.unwrap_or(CAPTION_SIZE * self.scale);
                let w = text_width(text, size);
                let x = px.0 as f32 * self.scale;
                let x = match *anchor {
                    "middle" => x - w / 2.0,
                    "end" => x - w,
                    _ => x,
                };
                // A street's name outranks the numbers and notes set beside it.
                let rank = if class.split_whitespace().any(|c| c == "iso-name") { 0 } else { 1 };
                self.word((x, px.1 as f32 * self.scale), size, text, rank);
            }
            // `hidden` is aria-hidden (a sign is part of the picture, not of what it says): it is drawn.
            Shape::Group { class, shapes, .. } => {
                let outer = self.context.clone();
                self.context = format!("{class} {outer}");
                for s in shapes {
                    self.shape(s);
                }
                self.context = outer;
            }
            Shape::Path { runs, closed, class, .. } => self.path(runs, *closed, class),
            Shape::Elem { .. } => self.omitted += 1,
        }
    }

    /// A path is runs of points. A press target (`*-hit`, invisible in the page) and a casing (a halo) are not
    /// drawn; a ring the stylesheet draws as an outline only (`iso-queue`, `measure-loop`, `approach-mark`,
    /// `curb-use-bad`) is a closed line; any other closed run is a polygon, when its class is one `style_in` knows
    /// (otherwise it would be grey over the arms); an open run is a line.
    fn path(&mut self, runs: &[Vec<crate::shared::diagram::Pt>], closed: bool, class: &str) {
        let has = |f: &dyn Fn(&str) -> bool| class.split_whitespace().any(f);
        if has(&|w| w == "hit" || w.ends_with("-hit") || w == "iso-casing") {
            return;
        }
        let outline = has(&|w| matches!(w, "iso-queue" | "measure-loop" | "approach-mark" | "curb-use-bad"));
        for run in runs {
            let mut pts = self.pts(run);
            if (closed || outline) && run.len() >= 3 && !outline {
                let st = style_in(&self.context, class, None, None);
                if st.fill != NEUTRAL {
                    self.items.push(Item::Poly { pts, fill: st.fill, stroke: st.stroke });
                }
            } else if run.len() >= 2 {
                if closed {
                    if let Some(first) = pts.first().copied() {
                        pts.push(first);
                    }
                }
                let (color, width) = match paint_of(class) {
                    Some(c) => (c, PAINT_WIDTH),
                    None => (colour("ink-2"), LINE_WIDTH),
                };
                self.items.push(Item::Line { pts, color, width });
            }
        }
    }

    fn word(&mut self, at: (f32, f32), size: f32, text: &str, rank: u8) {
        self.ranks.push((self.items.len(), rank));
        self.items.push(Item::Text { at, size, color: colour("ink-2"), text: text.to_string() });
    }

    fn layer(&mut self, l: &Layer) {
        if matches!(l.kind, LayerKind::Outline | LayerKind::Hit) {
            return;
        }
        for s in &l.shapes {
            self.shape(s);
        }
    }
}

/// The diagram as flat items: the ground, then each section's layers in order (the outline and the press targets
/// are not drawn), the projection's pixels scaled by one factor to fit `box_w` by `box_h`. The figure is the size
/// the drawing has once fitted. Shapes it cannot draw (`Path`, `Elem`) are counted in `omitted`.
pub fn flatten(d: &Diagram, p: &dyn Projection, box_w: f32, box_h: f32) -> Figure {
    let (w, h) = p.size();
    let (w, h) = (w as f32, h as f32);
    let scale = if w > 0.0 && h > 0.0 { (box_w / w).min(box_h / h) } else { 1.0 };
    let walk = walked(d, p, scale, None);
    Figure { width: w * scale, height: h * scale, items: walk.items, omitted: walk.omitted }
}

fn walked<'a>(d: &Diagram, p: &'a dyn Projection, scale: f32, caption: Option<f32>) -> Walk<'a> {
    let mut walk = Walk { p, scale, items: Vec::new(), omitted: 0, caption, ranks: Vec::new(), context: String::new() };
    walk.layer(&d.ground);
    for section in &d.sections {
        for l in &section.layers {
            walk.layer(l);
        }
    }
    walk
}

/// The box an item takes: its points, or for a word its estimated extent.
fn extent(item: &Item) -> (f32, f32, f32, f32) {
    match item {
        Item::Poly { pts, .. } | Item::Line { pts, .. } => {
            pts.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, p| (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1)))
        }
        Item::Text { at, size, text, .. } => (at.0, at.1 - size * ASCENT, at.0 + text_width(text, *size), at.1 + size * DESCENT),
    }
}

fn bounds(items: &[Item]) -> (f32, f32, f32, f32) {
    items.iter().map(extent).fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, e| (b.0.min(e.0), b.1.min(e.1), b.2.max(e.2), b.3.max(e.3)))
}

/// `flatten`, but the figure is fitted to what is drawn: the largest scale at which every shape and every word (its
/// extent estimated from its size and length, `text_width`) lies inside `box_w` by `box_h`, moved to the box's origin,
/// the figure being the size of what is drawn. A caption is `FIT_CAPTION_SIZE` whatever the scale. Words that would
/// overprint a word already kept are dropped: the street names (class `iso-name`) first, then the others in order.
pub fn flatten_fit(d: &Diagram, p: &dyn Projection, box_w: f32, box_h: f32) -> Figure {
    let (w, h) = p.size();
    let hi = if w > 0.0 && h > 0.0 { ((box_w / w as f32).min(box_h / h as f32)) * 8.0 } else { 1.0 };
    let fits = |s: f32| {
        let b = bounds(&walked(d, p, s, Some(FIT_CAPTION_SIZE)).items);
        b.2 - b.0 <= box_w && b.3 - b.1 <= box_h
    };
    // The bounds only grow with the scale: find the last that fits.
    let (mut lo, mut hi) = (0.0_f32, hi);
    for _ in 0..30 {
        let mid = (lo + hi) / 2.0;
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mut walk = walked(d, p, lo, Some(FIT_CAPTION_SIZE));
    let mut order = walk.ranks.clone();
    order.sort_by_key(|&(i, rank)| (rank, i));
    let mut kept: Vec<(f32, f32, f32, f32)> = Vec::new();
    let mut drop = vec![false; walk.items.len()];
    for (i, _) in order {
        let b = extent(&walk.items[i]);
        if kept.iter().any(|k| b.0 < k.2 && k.0 < b.2 && b.1 < k.3 && k.1 < b.3) {
            drop[i] = true;
        } else {
            kept.push(b);
        }
    }
    let mut at = 0;
    walk.items.retain(|_| {
        at += 1;
        !drop[at - 1]
    });
    let b = bounds(&walk.items);
    let (dx, dy) = (-b.0, -b.1);
    for item in &mut walk.items {
        match item {
            Item::Poly { pts, .. } | Item::Line { pts, .. } => pts.iter_mut().for_each(|q| *q = (q.0 + dx, q.1 + dy)),
            Item::Text { at, .. } => *at = (at.0 + dx, at.1 + dy),
        }
    }
    Figure { width: (b.2 - b.0).max(0.0), height: (b.3 - b.1).max(0.0), items: walk.items, omitted: walk.omitted }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::diagram::{Pt, Section, Texture};

    /// A projection that drops `up`: x across, z along.
    struct Flat(f64, f64);
    impl Projection for Flat {
        fn at(&self, p: Pt) -> (f64, f64) {
            (p.x, p.z)
        }
        fn size(&self) -> (f64, f64) {
            (self.0, self.1)
        }
    }

    fn tri(class: &str, face: Option<Face>, texture: Option<Texture>) -> Shape {
        Shape::Polygon { pts: vec![Pt::new(0.0, 0.0, 0.0), Pt::new(10.0, 0.0, 0.0), Pt::new(10.0, 0.0, 10.0)], class: class.into(), face, texture }
    }
    fn layer(kind: LayerKind, shapes: Vec<Shape>) -> Layer {
        Layer { kind, shapes }
    }
    fn diagram(ground: Vec<Shape>, layers: Vec<Layer>) -> Diagram {
        Diagram {
            defs: String::new(),
            ground: layer(LayerKind::Ground, ground),
            sections: vec![Section { class: None, layers }],
            outline: layer(LayerKind::Outline, vec![tri("iso-sel", None, None)]),
            hits: layer(LayerKind::Hit, vec![tri("iso-hit-area", None, None)]),
        }
    }
    fn fig(d: &Diagram) -> Figure {
        flatten(d, &Flat(100.0, 50.0), 100.0, 50.0)
    }
    fn hex(c: Rgb) -> String {
        format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
    }
    fn dark(c: Rgb, f: f32) -> Rgb {
        Rgb((c.0 as f32 * f).round() as u8, (c.1 as f32 * f).round() as u8, (c.2 as f32 * f).round() as u8)
    }

    #[test]
    fn a_polygon_becomes_a_flat_poly_with_the_colour_of_its_material() {
        let tex = Texture { material: "asphalt".into(), tile_mm: (1.0, 1.0) };
        let d = diagram(vec![], vec![layer(LayerKind::Surface, vec![tri("iso-top k-travel mat-asphalt", Some(Face::Top), Some(tex))])]);
        let f = fig(&d);
        assert_eq!(f.items.len(), 1);
        let Item::Poly { pts, fill, .. } = &f.items[0] else { panic!("{:?}", f.items) };
        assert_eq!(hex(*fill), "#3b3f43");
        assert_eq!(pts.len(), 3);
        assert_eq!(style_of("iso-top k-travel mat-asphalt", Some(Face::Top), None).fill, Rgb(0x3b, 0x3f, 0x43));
        assert_eq!(style_of("iso-top k-travel", Some(Face::Top), Some("brick")).fill, Rgb(0xa4, 0x52, 0x3b));
    }

    #[test]
    fn a_kind_without_a_material_uses_its_k_colour() {
        assert_eq!(hex(style_of("iso-top k-planting", Some(Face::Top), None).fill), "#a5d696");
        // The street page's own travel and parking tones (editor.css) win over the map's.
        assert_eq!(hex(style_of("iso-top k-travel", Some(Face::Top), None).fill), "#c3cbd3");
        assert_eq!(style_of("iso-top k-nonsense", Some(Face::Top), None).fill, NEUTRAL);
        assert_eq!(style_of("mystery", None, None).fill, NEUTRAL);
    }

    #[test]
    fn front_side_and_ramp_faces_are_darker_than_the_top() {
        assert_eq!((FRONT_SHADE, SIDE_SHADE, RAMP_SHADE), (0.9, 0.78, 0.84));
        let top = style_of("iso-top k-bike", Some(Face::Top), None).fill;
        for (face, f) in [(Face::Front, 0.9), (Face::Side, 0.78), (Face::Ramp, 0.84)] {
            assert_eq!(style_of("iso-side k-bike", Some(face), None).fill, dark(top, f), "{face:?}");
        }
    }

    #[test]
    fn paint_classes_use_the_paint_colours() {
        assert_eq!(hex(style_of("iso-paint white", None, None).fill), "#fbfbf8");
        assert_eq!(hex(style_of("iso-paint yellow", None, None).fill), "#e3a600");
        let d = diagram(
            vec![],
            vec![layer(
                LayerKind::Markings,
                vec![Shape::Polyline { pts: vec![Pt::new(0.0, 0.0, 0.0), Pt::new(0.0, 0.0, 9.0)], class: "iso-paint yellow".into() }],
            )],
        );
        let Item::Line { color, .. } = &fig(&d).items[0] else { panic!() };
        assert_eq!(hex(*color), "#e3a600");
    }

    #[test]
    fn hit_and_outline_layers_are_skipped() {
        let d =
            diagram(vec![], vec![layer(LayerKind::Hit, vec![tri("iso-hit-area", None, None)]), layer(LayerKind::Outline, vec![tri("iso-sel", None, None)])]);
        let f = fig(&d);
        assert!(f.items.is_empty(), "{:?}", f.items);
        assert_eq!(f.omitted, 0);
    }

    #[test]
    fn press_targets_and_casings_are_not_drawn_and_unknown_closed_paths_are_not_grey() {
        let run = vec![Pt::new(0.0, 0.0, 0.0), Pt::new(1.0, 0.0, 0.0), Pt::new(1.0, 0.0, 1.0)];
        let path = |class: &str| Shape::Path { runs: vec![run.clone()], closed: true, class: class.into(), attrs: String::new(), title: None };
        let shapes = ["lane-hit", "lane-hit on", "iso-hit", "iso-casing", "iso-casing warn", "mystery", "iso-queue", "measure-loop"].map(path).to_vec();
        let f = fig(&diagram(vec![], vec![layer(LayerKind::Surface, shapes)]));
        assert!(f.items.iter().all(|i| !matches!(i, Item::Poly { .. })), "{:?}", f.items);
        // The two rings are closed outlines: the first point is repeated.
        let lines: Vec<_> = f.items.iter().filter_map(|i| if let Item::Line { pts, .. } = i { Some(pts) } else { None }).collect();
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|p| p.len() == 4 && p[0] == p[3]));
    }

    #[test]
    fn a_closed_path_is_a_polygon_per_run() {
        let run = vec![Pt::new(0.0, 0.0, 0.0), Pt::new(1.0, 0.0, 0.0), Pt::new(1.0, 0.0, 1.0)];
        let path = Shape::Path { runs: vec![run.clone(), run], closed: true, class: "iso-flat k-travel".into(), attrs: String::new(), title: None };
        let f = fig(&diagram(vec![], vec![layer(LayerKind::Surface, vec![path])]));
        assert_eq!(f.items.iter().filter(|i| matches!(i, Item::Poly { .. })).count(), 2);
    }

    #[test]
    fn an_open_path_is_a_line_and_an_elem_is_counted_in_omitted() {
        let path = Shape::Path {
            runs: vec![vec![Pt::new(0.0, 0.0, 0.0), Pt::new(1.0, 0.0, 1.0)]],
            closed: false,
            class: String::new(),
            attrs: String::new(),
            title: None,
        };
        let elem = Shape::Elem { tag: "circle", class: String::new(), attrs: String::new(), shapes: vec![tri("iso-top", Some(Face::Top), None)] };
        let group = Shape::Group { class: "iso-solid".into(), uid: None, data: None, hidden: false, title: None, shapes: vec![path.clone()] };
        let d = diagram(vec![], vec![layer(LayerKind::Objects, vec![path, elem, group])]);
        let f = fig(&d);
        // The path alone and the path in the group are lines; the elem is not drawn.
        assert_eq!(f.items.iter().filter(|i| matches!(i, Item::Line { .. })).count(), 2, "{:?}", f.items);
        assert_eq!(f.omitted, 1);
    }

    #[test]
    fn layers_draw_in_the_order_of_the_diagram() {
        let mut d = diagram(
            vec![tri("iso-ground", None, None)],
            vec![
                layer(LayerKind::Surface, vec![tri("iso-top k-bike", Some(Face::Top), None)]),
                layer(LayerKind::Markings, vec![tri("iso-paint white", None, None)]),
            ],
        );
        d.sections
            .push(Section { class: Some("iso-section".into()), layers: vec![layer(LayerKind::Surface, vec![tri("iso-top k-bus", Some(Face::Top), None)])] });
        let fills: Vec<String> = fig(&d).items.iter().map(|i| if let Item::Poly { fill, .. } = i { hex(*fill) } else { panic!() }).collect();
        assert_eq!(fills, ["#f1f3f5", "#7fd3c3", "#fbfbf8", "#f2a99b"]);
    }

    #[test]
    fn the_figure_is_fitted_to_the_box_keeping_its_proportions() {
        let d = diagram(vec![], vec![layer(LayerKind::Surface, vec![tri("iso-top", Some(Face::Top), None)])]);
        // 100 x 50 into 200 x 200: scale 2, a 200 x 100 figure.
        let f = flatten(&d, &Flat(100.0, 50.0), 200.0, 200.0);
        assert_eq!((f.width, f.height), (200.0, 100.0));
        let Item::Poly { pts, .. } = &f.items[0] else { panic!() };
        assert_eq!(pts, &vec![(0.0, 0.0), (20.0, 0.0), (20.0, 20.0)]);
        // The box is the limit the other way too.
        let g = flatten(&d, &Flat(100.0, 50.0), 400.0, 25.0);
        assert_eq!((g.width, g.height), (50.0, 25.0));
    }

    #[test]
    fn edges_of_equal_colours_stay_visible() {
        for class in ["iso-top k-travel", "iso-side k-travel", "iso-front k-travel", "iso-curb", "iso-top mat-concrete"] {
            let face = if class.starts_with("iso-top") { Face::Top } else { Face::Side };
            assert!(style_of(class, Some(face), None).stroke.is_some(), "{class}");
        }
        let d = diagram(vec![], vec![layer(LayerKind::Surface, vec![tri("iso-top k-travel", Some(Face::Top), None)])]);
        let Item::Poly { stroke, .. } = &fig(&d).items[0] else { panic!() };
        assert_eq!(*stroke, Some((colour("ink-2"), 1.0)));
    }

    #[test]
    fn text_joints_groups_and_captions_are_drawn() {
        let joints = Shape::Joints {
            runs: vec![crate::shared::diagram::JointRun { from: Pt::new(0.0, 0.0, 0.0), to: Pt::new(5.0, 0.0, 0.0), flat: true }],
            class: "joints j-brick".into(),
        };
        let text = Shape::Text { at: Pt::new(1.0, 0.0, 2.0), class: "t-dim".into(), size: 6.0, text: "3 m".into() };
        let cap = Shape::Caption { px: (10.0, 20.0), class: "iso-name".into(), anchor: "start", text: "Rue".into() };
        let hidden =
            Shape::Group { class: "iso-sign".into(), uid: None, data: None, hidden: true, title: None, shapes: vec![tri("iso-top", Some(Face::Top), None)] };
        let shown = Shape::Group { class: "iso-piece".into(), uid: Some(1), data: None, hidden: false, title: None, shapes: vec![text.clone()] };
        let d = diagram(vec![], vec![layer(LayerKind::Surface, vec![joints, shown, cap, hidden])]);
        let f = fig(&d);
        assert_eq!(f.items.len(), 4, "{:?}", f.items);
        assert!(matches!(&f.items[0], Item::Line { color, .. } if hex(*color) == "#d6c4ae"));
        // A shape's words are centred on their point (the stylesheet's `text-anchor: middle`): 3 glyphs of 6 at 0.6.
        assert!(matches!(&f.items[1], Item::Text { at, size, text, .. } if (at.0 - (1.0 - 5.4)).abs() < 1e-4 && at.1 == 2.0 && *size == 6.0 && text == "3 m"));
        assert!(matches!(&f.items[2], Item::Text { at: (10.0, 20.0), text, .. } if text == "Rue"));
    }

    #[test]
    fn a_hidden_group_is_drawn() {
        let hidden = Shape::Group {
            class: "iso-sign".into(),
            uid: None,
            data: None,
            hidden: true,
            title: None,
            shapes: vec![tri("iso-top k-bus", Some(Face::Top), None)],
        };
        let d = diagram(vec![], vec![layer(LayerKind::Signs, vec![hidden])]);
        let f = fig(&d);
        assert!(matches!(&f.items[..], [Item::Poly { fill, .. }] if hex(*fill) == "#f2a99b"), "{:?}", f.items);
    }

    #[test]
    fn a_trees_three_faces_take_the_colour_of_s_tree_in_three_shades() {
        let face = |f: Face| style_in("iso-solid s-crown", "o-top", Some(f), None);
        let (top, front, side) = (face(Face::Top).fill, face(Face::Front).fill, face(Face::Side).fill);
        assert_eq!(top, colour("s-tree"));
        assert_eq!(front, dark(top, 0.9));
        assert_eq!(side, dark(top, 0.78));
        assert!(top != front && front != side && top != side);
        // Through the walker, from the group.
        let group = |c: &str| Shape::Group {
            class: c.into(),
            uid: None,
            data: None,
            hidden: false,
            title: None,
            shapes: vec![tri("o-front", Some(Face::Front), None)],
        };
        let d = diagram(
            vec![],
            vec![layer(
                LayerKind::Objects,
                vec![group("iso-solid s-crown"), group("iso-solid s-trunk"), group("iso-solid s-shelter"), group("iso-solid s-post")],
            )],
        );
        let fills: Vec<Rgb> = fig(&d).items.iter().map(|i| if let Item::Poly { fill, .. } = i { *fill } else { panic!() }).collect();
        let want: Vec<Rgb> = ["s-tree", "k-bench", "k-busshelter", "ink-2"].iter().map(|n| dark(colour(n), 0.9)).collect();
        assert_eq!(fills, want);
    }

    #[test]
    fn a_sign_panel_is_the_panel_colour_with_the_ink_edge() {
        let st = style_in("iso-sign", "iso-sign-panel", None, None);
        assert_eq!(hex(st.fill), "#fbfbf8");
        assert_eq!(st.stroke, Some((colour("iso-sign-ink"), 1.0)));
        assert_eq!(style_of("iso-sign-glyph-cut", None, None).fill, colour("iso-sign-panel"));
        assert_eq!(style_of("iso-sign-glyph", None, None).fill, colour("iso-sign-ink"));
    }

    #[test]
    fn a_polygon_inside_a_solid_has_the_thin_ink_2_edge() {
        let st = style_in("iso-solid s-crown", "o-top", Some(Face::Top), None);
        assert_eq!(st.stroke, Some((colour("ink-2"), 0.8)));
        assert_eq!(style_of("o-top", Some(Face::Top), None).stroke, None);
    }

    #[test]
    fn the_junctions_flat_pieces_take_their_kind_and_the_plans_edge() {
        let st = style_of("iso-flat k-parking", None, None);
        assert_eq!(hex(st.fill), "#b4bdc7");
        assert_eq!(st.stroke, Some((colour("ink"), 0.6)));
        assert_eq!(style_of("iso-flat k-travel mat-asphalt", None, None).fill, colour("mat-asphalt"));
        for c in ["iso-zebra", "iso-stopline", "iso-paint white"] {
            let st = style_of(c, None, None);
            assert_eq!(st.fill, colour("iso-paint-white"), "{c}");
            assert_eq!(st.stroke, Some((colour("iso-paint-edge"), 0.5)), "{c}");
        }
        let pad = style_of("stop-pad stop-bulb", None, None);
        assert_eq!((pad.fill, pad.stroke), (colour("accent-wash"), Some((colour("accent"), 1.5))));
    }

    #[test]
    fn the_classes_left_unstyled_are_listed_and_grey() {
        // Lines and marks the figure does not paint as polygons (they are paths or lines, or have no fill in the CSS),
        // so a polygon of one of these classes is deliberately grey.
        for c in [
            "mv",
            "mv bad",
            "approach-mark",
            "lane-hit",
            "iso-hit",
            "iso-head",
            "iso-queue",
            "iso-casing",
            "iso-taper",
            "iso-mark",
            "iso-name",
            "stop-run",
            "stop-post",
            "stop-shelter",
            "iso-post",
            "t-dim",
            "iso-piece",
        ] {
            assert_eq!(style_of(c, None, None), Style { fill: NEUTRAL, stroke: None }, "{c}");
        }
    }

    /// The light value of a token, as the table writes it.
    fn css_value(name: &str, css: &Css) -> Rgb {
        let (app, editor) = (include_str!("../../web/app.css"), include_str!("../../web/editor.css"));
        let sheet = match css {
            Css::App => app,
            Css::Editor => editor,
        };
        let h = crate::shared::contrast::token(sheet, name).0.unwrap_or_else(|| panic!("--{name} is no #rrggbb"));
        let n = |i: usize| u8::from_str_radix(&h[1 + 2 * i..3 + 2 * i], 16).unwrap();
        Rgb(n(0), n(1), n(2))
    }

    #[test]
    fn every_flat_colour_matches_the_css() {
        assert!(COLOURS.len() == 38, "{}", COLOURS.len());
        for (name, css, rgb) in COLOURS {
            // The names are the tokens' without the leading dashes, except `ink-2`-style ones that are the same.
            assert_eq!(css_value(name, css), *rgb, "--{name}");
        }
        // Every material and every kind the catalogue has is in the table (or is deliberately neutral).
        for m in crate::shared::catalogue::MATERIALS {
            assert!(COLOURS.iter().any(|c| c.0 == format!("mat-{}", m.id)), "mat-{}", m.id);
        }
    }
}
