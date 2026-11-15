//! Street trees and green infrastructure: a tree pit, a planting bed, a rain garden or a bioswale, placed along a
//! street and across its section.
//!
//! Like the curbside uses (`curb.rs`) and the transit stops (`stop.rs`) they belong to the street, not to a
//! section: a position along is measured from the anchored end (`Street::from_end`). Across, `offset_mm` is the
//! element's centre from the section's left edge.

use serde::{Deserialize, Serialize};

use super::{Street, total};

/// What an element is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GreenKind {
    /// One tree in its pit.
    Tree,
    /// A planting bed.
    Bed,
    /// A planted basin that takes the street's runoff.
    RainGarden,
    /// A long planted channel that carries and soaks up runoff.
    Bioswale,
}

impl GreenKind {
    /// Every kind, in the order a choice lists them.
    pub const ALL: [GreenKind; 4] = [GreenKind::Tree, GreenKind::Bed, GreenKind::RainGarden, GreenKind::Bioswale];
}

/// The size a tree grows to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TreeSize {
    S,
    M,
    L,
}

/// One element, `length_mm` along the street from `from_mm`, centred `offset_mm` from the section's left edge and
/// `width_mm` across. `size` is set for a tree and only for a tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Green {
    pub kind: GreenKind,
    /// Millimetres from the anchored end (`Street::from_end`) to where it starts, a multiple of the editor's snap (100 mm).
    pub from_mm: i32,
    pub length_mm: i32,
    /// The element's centre across, from the left edge of the section. The anchored end does not
    /// flip it.
    pub offset_mm: i32,
    pub width_mm: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<TreeSize>,
}

/// The elements in their canonical order: by position along, then across.
pub fn sort_green(green: &mut [Green]) {
    green.sort_by_key(|g| (g.from_mm, g.offset_mm));
}

impl Street {
    pub(super) fn sort_green(&mut self) {
        sort_green(&mut self.green);
    }

    /// The total width of the section that lies at `along_mm` from the anchored end: the zone's own section, or
    /// the base. Sections are laid out from x = 0 and may total less than `row_mm`.
    pub fn section_total_at(&self, along_mm: i32) -> i32 {
        let zone = self.zones.iter().rev().find(|z| z.from_mm <= along_mm);
        total(zone.and_then(|z| z.segments.as_deref()).unwrap_or(&self.segments))
    }

    /// Mirrors the elements across for a street seen from its other end. Their positions along stay measured from
    /// the same physical end (`reverse_zones` flips the label of that end); the section is reversed, so each
    /// centre is measured from the other edge, of the section under the element.
    pub(super) fn reverse_green(&mut self) {
        let mirrored: Vec<Green> =
            self.green.iter().map(|g| Green { offset_mm: self.section_total_at(g.from_mm.saturating_add(g.length_mm / 2)) - g.offset_mm, ..*g }).collect();
        self.green = mirrored;
        self.sort_green();
    }

    /// Whether the elements can be placed: they need the length they were drawn against, and each lies in front
    /// of it with a size exactly when it is a tree.
    pub(super) fn green_is_sound(&self) -> bool {
        self.green.is_empty()
            || (self.length_mm.is_some()
                && self.green.iter().all(|g| {
                    g.from_mm >= 0
                        && g.length_mm > 0
                        && g.width_mm > 0
                        && (0..=self.section_total_at(g.from_mm.saturating_add(g.length_mm / 2))).contains(&g.offset_mm)
                        && g.size.is_some() == (g.kind == GreenKind::Tree)
                }))
    }
}

impl Green {
    /// Where it ends, in millimetres from the anchored end.
    pub fn to_mm(&self) -> i32 {
        self.from_mm.saturating_add(self.length_mm)
    }

    /// Its middle along the street, in millimetres from the anchored end.
    pub fn centre_mm(&self) -> i32 {
        self.from_mm.saturating_add(self.length_mm / 2)
    }
}
