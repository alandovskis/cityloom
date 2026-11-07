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
/// A word's size where its shape names none (a caption).
const CAPTION_SIZE: f32 = 12.0;

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

/// The paint of a polygon of this class: its material's colour (the argument, else the class's `mat-*`), else the
/// paint's, the curb's, the ground's or its kind's (`k-*`), else grey; the faces turned from the eye are darker.
pub fn style_of(class: &str, face: Option<Face>, material: Option<&str>) -> Style {
    let words: Vec<&str> = class.split_whitespace().collect();
    let has = |w: &str| words.contains(&w);
    let material = material.or_else(|| words.iter().find_map(|w| w.strip_prefix("mat-")));
    let base = material
        .and_then(|m| lookup(&format!("mat-{m}")))
        .or_else(|| paint_of(class))
        .or_else(|| has("lane-arrow").then(|| colour("iso-paint-white")))
        .or_else(|| has("iso-curb").then(|| colour("iso-curb")))
        .or_else(|| has("iso-ground").then(|| colour("fill")))
        .or_else(|| has("stop-pad").then(|| colour("mat-concrete")))
        .or_else(|| words.iter().find_map(|w| w.strip_prefix("k-").and_then(|k| lookup(&format!("k-{k}")))))
        .unwrap_or(NEUTRAL);
    let fill = match face {
        Some(Face::Front) => shaded(base, FRONT_SHADE),
        Some(Face::Side) => shaded(base, SIDE_SHADE),
        Some(Face::Ramp) => shaded(base, RAMP_SHADE),
        Some(Face::Top) | None => base,
    };
    // Faces of one tint would run together without an outline.
    let edged = ["iso-top", "iso-side", "iso-front", "iso-ramp", "iso-flat", "iso-curb"].iter().any(|w| has(w));
    Style { fill, stroke: edged.then(|| (colour("ink-2"), EDGE_WIDTH)) }
}

struct Walk<'a> {
    p: &'a dyn Projection,
    scale: f32,
    items: Vec<Item>,
    omitted: usize,
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
                let st = style_of(class, *face, texture.as_ref().map(|t| t.material.as_str()));
                self.items.push(Item::Poly { pts: self.pts(pts), fill: st.fill, stroke: st.stroke });
            }
            Shape::Plane { pts, class, face, .. } => {
                let st = style_of(class, Some(*face), None);
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
                let (x, y) = self.p.at(*at);
                self.items.push(Item::Text {
                    at: (x as f32 * self.scale, y as f32 * self.scale),
                    size: *size as f32 * self.scale,
                    color: colour("ink-2"),
                    text: text.clone(),
                });
            }
            Shape::Caption { px, text, .. } => {
                self.items.push(Item::Text {
                    at: (px.0 as f32 * self.scale, px.1 as f32 * self.scale),
                    size: CAPTION_SIZE * self.scale,
                    color: colour("ink-2"),
                    text: text.clone(),
                });
            }
            Shape::Group { hidden, shapes, .. } => {
                if !*hidden {
                    for s in shapes {
                        self.shape(s);
                    }
                }
            }
            Shape::Path { .. } | Shape::Elem { .. } => self.omitted += 1,
        }
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
    let mut walk = Walk { p, scale, items: Vec::new(), omitted: 0 };
    walk.layer(&d.ground);
    for section in &d.sections {
        for l in &section.layers {
            walk.layer(l);
        }
    }
    Figure { width: w * scale, height: h * scale, items: walk.items, omitted: walk.omitted }
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
    fn path_and_elem_are_counted_in_omitted_and_not_drawn() {
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
        assert!(f.items.is_empty(), "{:?}", f.items);
        assert_eq!(f.omitted, 3);
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
        assert_eq!(f.items.len(), 3, "{:?}", f.items);
        assert!(matches!(&f.items[0], Item::Line { color, .. } if hex(*color) == "#d6c4ae"));
        assert!(matches!(&f.items[1], Item::Text { at: (1.0, 2.0), size, text, .. } if *size == 6.0 && text == "3 m"));
        assert!(matches!(&f.items[2], Item::Text { at: (10.0, 20.0), text, .. } if text == "Rue"));
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
        assert!(COLOURS.len() == 30, "{}", COLOURS.len());
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
