//! The layers the 2.5D is drawn in, one group per kind of thing, stacked in the order they are painted.
//!
//! From the bottom: the ground; the surface (the top of every roadway piece); the markings (paint on the surface);
//! the raised layer (every strip standing above the roadway, with its faces, and the curbside uses lifted into
//! strips); the objects (trees, shelters, posts, racks: everything that stands on a piece); the signs; and the
//! selected piece's outline. A layer is a group of its own with a class that names it, and holds nothing of another
//! kind. Within a layer the order is the one the eye needs (`order`); between layers it is this one.

/// One kind of thing in the drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Layer {
    Ground,
    Surface,
    Markings,
    Raised,
    Objects,
    Signs,
    Outline,
}

impl Layer {
    /// Every layer, painted first to last (the order a test pins).
    #[cfg(test)]
    pub(super) const ALL: [Layer; 7] = [Layer::Ground, Layer::Surface, Layer::Markings, Layer::Raised, Layer::Objects, Layer::Signs, Layer::Outline];

    /// The class of the layer's group.
    pub(super) fn class(self) -> &'static str {
        match self {
            Layer::Ground => "iso-layer-ground",
            Layer::Surface => "iso-surface",
            Layer::Markings => "iso-markings",
            Layer::Raised => "iso-raised",
            Layer::Objects => "iso-objects",
            Layer::Signs => "iso-signs",
            Layer::Outline => "iso-layer-outline",
        }
    }

    /// The start of the layer's group.
    pub(super) fn open(self) -> String {
        format!("<g class=\"{}\">", self.class())
    }

    /// `inner` in the layer's group.
    pub(super) fn wrap(self, inner: &str) -> String {
        format!("{}{inner}</g>", self.open())
    }
}
