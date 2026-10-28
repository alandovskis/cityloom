//! Street trees and green infrastructure: a tree pit, a planting bed, a rain garden or a bioswale, placed along a
//! street and across its section.
//!
//! Like the curbside uses (`curb.rs`) and the transit stops (`stop.rs`) they belong to the street, not to a
//! section: a position along is measured from the anchored end (`Street::from_end`), so splitting, merging or moving
//! zones leaves them where they are. Across, `offset_mm` is the element's centre from the section's left edge.
//! A street with none is written, drawn and checked exactly as before.

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

    /// The message that names it.
    pub fn name_key(self) -> &'static str {
        match self {
            GreenKind::Tree => "green-kind-tree",
            GreenKind::Bed => "green-kind-bed",
            GreenKind::RainGarden => "green-kind-rain-garden",
            GreenKind::Bioswale => "green-kind-bioswale",
        }
    }

    /// How long a new element is along the street.
    ///
    /// UNVERIFIED (placeholder): a tree pit 1 500 mm square, a bed 4 000 mm, a rain garden 6 000 mm, a bioswale
    /// 20 000 mm.
    pub fn default_length_mm(self) -> i32 {
        match self {
            GreenKind::Tree => 1_500,
            GreenKind::Bed => 4_000,
            GreenKind::RainGarden => 6_000,
            GreenKind::Bioswale => 20_000,
        }
    }
}

/// The size a tree grows to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TreeSize {
    S,
    M,
    L,
}

impl TreeSize {
    /// The width of the crown, which is also the spacing between trees.
    ///
    /// UNVERIFIED (placeholder): 6 000, 8 000 and 10 000 mm.
    pub fn canopy_mm(self) -> i32 {
        match self {
            TreeSize::S => 6_000,
            TreeSize::M => 8_000,
            TreeSize::L => 10_000,
        }
    }

    /// The least area of soil a pit gives this tree, in square millimetres.
    ///
    /// UNVERIFIED (placeholder): 2.25, 3.5 and 5 square metres.
    pub fn min_pit_mm2(self) -> i64 {
        match self {
            TreeSize::S => 2_250_000,
            TreeSize::M => 3_500_000,
            TreeSize::L => 5_000_000,
        }
    }
}

/// One element, `length_mm` along the street from `from_mm`, centred `offset_mm` from the section's left edge and
/// `width_mm` across. `size` is set for a tree and only for a tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Green {
    pub kind: GreenKind,
    /// Millimetres from the anchored end (`Street::from_end`) to where it starts, a multiple of `SNAP_MM`.
    pub from_mm: i32,
    pub length_mm: i32,
    /// The element's centre across, from the left edge of the section (`SegView.x_mm`). The anchored end does not
    /// flip it.
    pub offset_mm: i32,
    pub width_mm: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<TreeSize>,
}

/// Why a change to the elements was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GreenRefusal {
    /// It would reach past the street's length.
    TooLong,
    /// It would overlap another element of the same kind.
    Overlap,
}

pub(super) fn sort_green(green: &mut [Green]) {
    green.sort_by_key(|g| (g.from_mm, g.offset_mm));
}

impl Street {
    pub(super) fn sort_green(&mut self) {
        sort_green(&mut self.green);
    }

    /// The total width of the section that lies at `along_mm` from the anchored end: the zone's own section, or
    /// the base. Sections are laid out from x = 0 and may total less than `row_mm`.
    fn section_total_at(&self, along_mm: i32) -> i32 {
        let zone = self.zones.iter().rev().find(|z| z.from_mm <= along_mm);
        total(zone.and_then(|z| z.segments.as_deref()).unwrap_or(&self.segments))
    }

    /// Mirrors the elements across for a street seen from its other end. Their positions along stay measured from
    /// the same physical end (`reverse_zones` flips the label of that end); the section is reversed, so each
    /// centre is measured from the other edge, of the section under the element.
    pub(super) fn reverse_green(&mut self) {
        let mirrored: Vec<Green> =
            self.green.iter().map(|g| Green { offset_mm: self.section_total_at(g.from_mm + g.length_mm / 2) - g.offset_mm, ..*g }).collect();
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
                        && (0..=self.row_mm).contains(&g.offset_mm)
                        && g.size.is_some() == (g.kind == GreenKind::Tree)
                }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::Side;
    use crate::street::model::fixtures::zoned;
    use crate::street::model::{Segment, ZoneEnd};

    fn tree(from_mm: i32, offset_mm: i32) -> Green {
        Green { kind: GreenKind::Tree, from_mm, length_mm: 1_500, offset_mm, width_mm: 1_500, size: Some(TreeSize::M) }
    }

    fn bed(from_mm: i32, offset_mm: i32) -> Green {
        Green { kind: GreenKind::Bed, from_mm, length_mm: 4_000, offset_mm, width_mm: 2_000, size: None }
    }

    fn with_green(green: Vec<Green>) -> Street {
        Street { green, length_mm: Some(100_000), ..Street::sample(0, Side::Right) }
    }

    #[test]
    fn a_street_without_green_serialises_as_before() {
        let json = serde_json::to_string(&Street::sample(0, Side::Right)).unwrap();
        assert!(!json.contains("green"), "{json}");
    }

    #[test]
    fn green_round_trips() {
        let s = with_green(vec![tree(3_000, 1_000), bed(10_000, 5_000)]);
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"green\""));
        assert!(json.contains("\"kind\":\"tree\"") && json.contains("\"size\":\"M\""));
        let back: Street = serde_json::from_str(&json).unwrap();
        assert_eq!(back.green, s.green);
        assert_eq!(back.green[1].size, None);
        assert!(back.is_sound());
    }

    #[test]
    fn a_tree_without_a_size_is_not_sound() {
        let mut t = tree(3_000, 1_000);
        t.size = None;
        assert!(!with_green(vec![t]).is_sound());
        let mut b = bed(3_000, 1_000);
        b.size = Some(TreeSize::S);
        assert!(!with_green(vec![b]).is_sound());
        assert!(with_green(vec![tree(3_000, 1_000)]).is_sound());
    }

    #[test]
    fn an_element_without_a_recorded_length_is_not_sound() {
        let mut s = with_green(vec![tree(3_000, 1_000)]);
        s.length_mm = None;
        assert!(!s.is_sound());
    }

    #[test]
    fn reversing_keeps_each_element_where_it_stands() {
        let s = with_green(vec![tree(3_000, 1_000), bed(10_000, 5_000)]);
        let once = s.reversed();
        let total_mm = total(&s.segments);
        assert_eq!(once.from_end, ZoneEnd::Last);
        assert_eq!(once.green.iter().map(|g| (g.from_mm, g.length_mm)).collect::<Vec<_>>(), vec![(3_000, 1_500), (10_000, 4_000)]);
        assert_eq!(once.green.iter().map(|g| g.offset_mm).collect::<Vec<_>>(), vec![total_mm - 1_000, total_mm - 5_000]);
        assert_eq!(once.reversed(), s);
    }

    #[test]
    fn reversing_mirrors_by_the_section_under_the_element() {
        // The second zone has a section of its own that totals less than the street's row.
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 50_000], 100_000);
        let mut own: Vec<Segment> = s.segments.clone();
        let removed = own.remove(1).width_mm;
        s.zones[1].segments = Some(own);
        s.row_mm = total(&s.segments);
        assert!(removed > 0);
        s.green = vec![tree(10_000, 1_000), tree(60_000, 1_000)];
        let once = s.reversed();
        assert_eq!(once.green[0].offset_mm, total(&s.segments) - 1_000);
        assert_eq!(once.green[1].offset_mm, total(&s.segments) - removed - 1_000);
        assert_eq!(once.reversed(), s);
    }

    #[test]
    fn at_end_drops_the_elements() {
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 50_000], 100_000);
        s.green = vec![tree(3_000, 1_000)];
        assert!(s.at_end(true).green.is_empty());
    }
}
