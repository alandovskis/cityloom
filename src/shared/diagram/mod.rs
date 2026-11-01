//! The street's drawing as data: layers of shapes in ground millimetres.

pub mod render;

/// How a point of the ground lands in the drawing, in pixels.
pub trait Projection {
    /// Where the point is, in pixels.
    fn at(&self, p: Pt) -> (f64, f64);
    /// The drawing's width and height, in pixels.
    fn size(&self) -> (f64, f64);
}

/// A point on the ground and above it, in millimetres: across the street,
/// up, and along it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Pt {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

/// Which face of a solid a polygon is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Top,
    Front,
    Side,
    Ramp,
}

/// A material laid on a polygon, its tile in millimetres of ground.
#[derive(Clone, Debug, PartialEq)]
pub struct Texture {
    pub material: String,
    pub tile_mm: (f64, f64),
}

/// One thing drawn: a closed polygon, an open polyline, or a group (a piece
/// with its faces, a uid and a title).
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Polygon { pts: Vec<Pt>, class: String, face: Option<Face>, texture: Option<Texture> },
    Polyline { pts: Vec<Pt>, class: String },
    Group { class: String, uid: Option<u32>, title: Option<String>, shapes: Vec<Shape> },
}

/// The kinds of layer, bottom to top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerKind {
    Ground,
    Surface,
    Markings,
    Raised,
    Objects,
    Signs,
    Outline,
}

impl LayerKind {
    pub const ALL: [LayerKind; 7] =
        [LayerKind::Ground, LayerKind::Surface, LayerKind::Markings, LayerKind::Raised, LayerKind::Objects, LayerKind::Signs, LayerKind::Outline];

    /// The CSS class of the layer's group.
    pub fn class(self) -> &'static str {
        match self {
            LayerKind::Ground => "iso-layer-ground",
            LayerKind::Surface => "iso-surface",
            LayerKind::Markings => "iso-markings",
            LayerKind::Raised => "iso-raised",
            LayerKind::Objects => "iso-objects",
            LayerKind::Signs => "iso-signs",
            LayerKind::Outline => "iso-layer-outline",
        }
    }
}

/// A layer holds shapes of one kind and nothing else.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub kind: LayerKind,
    pub shapes: Vec<Shape>,
}

/// A run of the street with its own layers. `class` is `iso-section` or
/// `iso-section iso-across` (the neighbour's); `None` for the only section
/// when there is no joint.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub class: Option<String>,
    pub layers: Vec<Layer>,
}

/// The whole drawing: the ground, the sections, then the outline.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagram {
    pub defs: String,
    pub ground: Layer,
    pub sections: Vec<Section>,
    pub outline: Layer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_are_listed_bottom_to_top_with_the_classes_the_markup_uses() {
        assert_eq!(
            LayerKind::ALL.map(LayerKind::class),
            ["iso-layer-ground", "iso-surface", "iso-markings", "iso-raised", "iso-objects", "iso-signs", "iso-layer-outline",]
        );
    }

    #[test]
    fn a_diagram_holds_ground_then_sections_then_outline() {
        let poly = Shape::Polygon {
            pts: vec![Pt::new(0.0, 0.0, 0.0), Pt::new(1.0, 0.0, 0.0), Pt::new(1.0, 0.0, 2.0)],
            class: "iso-top".into(),
            face: Some(Face::Top),
            texture: Some(Texture { material: "asphalt".into(), tile_mm: (100.0, 200.0) }),
        };
        let group = Shape::Group { class: "iso-piece".into(), uid: Some(3), title: Some("Lane".into()), shapes: vec![poly.clone()] };
        let d = Diagram {
            defs: "<defs/>".into(),
            ground: Layer { kind: LayerKind::Ground, shapes: vec![poly] },
            sections: vec![Section { class: None, layers: vec![Layer { kind: LayerKind::Surface, shapes: vec![group.clone()] }] }],
            outline: Layer {
                kind: LayerKind::Outline,
                shapes: vec![Shape::Polyline { pts: vec![Pt::new(0.0, 0.0, 0.0), Pt::new(0.0, 1.0, 0.0)], class: "iso-sel".into() }],
            },
        };
        assert_eq!(d.ground.kind, LayerKind::Ground);
        assert_eq!(d.sections.len(), 1);
        assert_eq!(d.sections[0].class, None);
        assert_eq!(d.sections[0].layers[0].shapes, vec![group]);
        assert_eq!(d.outline.kind, LayerKind::Outline);
        assert_eq!(d.clone(), d);
    }
}
