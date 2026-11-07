//! What the source-data quality inspector reads of the city: plain data about each street and node *as the importer
//! laid them out*, before any edit, with no wording and no editor. `City::source_read` makes it from the layout; the
//! rules that judge it are the map's (`map::source_quality`). It is kept apart from `NetworkRead`, which describes the
//! streets as they stand now for the design checks.

use super::{City, node_uid, street_note};
use crate::shared::catalogue::StreetClass;
use crate::shared::said::Said;
use crate::street::model::Speed;

/// One piece of a street's section, as the importer made it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneRead {
    /// Index into `KINDS`.
    pub kind: usize,
    pub width_mm: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceStreet {
    pub uid: u32,
    /// The nodes (their uids) at the first and last ends.
    pub a: u32,
    pub b: u32,
    /// How the notes name the street.
    pub name: Said,
    /// What it is called, for a sentence.
    pub called: Said,
    /// Whether OpenStreetMap gave the street a name (otherwise `called` is an invented one).
    pub named: bool,
    /// The width of the street as imported.
    pub row_mm: i32,
    /// The pieces of its base section as imported.
    pub lanes: Vec<LaneRead>,
    /// Whether the road has no speed limit in the source.
    pub no_speed: bool,
    /// Whether it is a pedestrian street (it has no motor traffic, so a speed limit means nothing).
    pub pedestrian: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceNode {
    pub uid: u32,
    pub name: Said,
    pub junction: bool,
    /// Where a road is cut at the area's box: the node was made from no OpenStreetMap node, so the network goes on
    /// beyond it.
    pub open: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceRead {
    pub streets: Vec<SourceStreet>,
    pub nodes: Vec<SourceNode>,
}

impl City {
    /// The streets and nodes as the importer laid them out, for the source-data inspector.
    pub fn source_read(&self) -> SourceRead {
        let streets = self
            .layout
            .edges
            .iter()
            .enumerate()
            .map(|(i, e)| SourceStreet {
                uid: i as u32 + 1,
                a: node_uid(e.a),
                b: node_uid(e.b),
                name: street_note(self.layout.edge_title(i, &e.section), self.layout.edge_ends(i)),
                called: self.layout.edge_title(i, &e.section),
                named: e.name.is_some(),
                row_mm: e.section.row_mm,
                lanes: e.section.segments.iter().map(|s| LaneRead { kind: s.kind, width_mm: s.width_mm }).collect(),
                no_speed: e.section.speed == Speed::NotSet,
                pedestrian: e.section.class == StreetClass::Pedestrian,
            })
            .collect();
        let nodes = self
            .layout
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| SourceNode { uid: node_uid(i), name: self.layout.node_name(i), junction: n.junction, open: n.source.is_empty() && !n.junction })
            .collect();
        SourceRead { streets, nodes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_read_has_every_street_and_node_as_laid_out() {
        let city = City::new();
        let read = city.source_read();
        assert_eq!(read.streets.len(), crate::city::model::Layout::sample().edges.len());
        assert!(read.streets.iter().all(|s| s.row_mm > 0 && !s.lanes.is_empty()));
        assert!(read.nodes.iter().any(|n| n.junction) && read.nodes.iter().all(|n| !(n.junction && n.open)));
    }

    #[test]
    fn an_edit_does_not_change_what_the_source_read_says() {
        let mut city = City::new();
        let before = city.source_read();
        let mut e = city.street_editor(10, 0).unwrap();
        assert!(e.make_pedestrian(crate::shared::catalogue::kind_index("sidewalk").unwrap()));
        assert!(city.keep_street(10, e.snapshot()));
        assert_eq!(city.source_read(), before);
    }
}
