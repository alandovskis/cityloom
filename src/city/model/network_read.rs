//! What the network-level checks read of the city: plain data about each street and junction, with no wording and
//! no editor. `City::network_read` makes it from the streets and junctions as they stand now; the rules that judge it
//! are the map's (`map::network`).

use super::{City, node_uid};
use crate::shared::catalogue::{KINDS, StreetClass};
use crate::shared::said::Said;
use crate::street::model::{Street, walkway_access};

/// One end of a street: the zone that meets the node, as the junction there reads it.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct EndRead {
    /// The driving and transit lanes of that zone.
    pub lanes: usize,
    /// Whether that zone has a bike lane.
    pub bike: bool,
    /// Whether that zone has a transit lane.
    pub bus: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StreetRead {
    pub uid: u32,
    /// The nodes (their uids) at the first and last ends.
    pub a: u32,
    pub b: u32,
    /// How the notes name the street.
    pub name: Said,
    pub pedestrian: bool,
    pub freeway: bool,
    /// Whether any zone has a bike lane.
    pub bike: bool,
    /// Whether every zone has an unbroken run of sidewalk wide enough for emergency access.
    pub walkway_ok: bool,
    /// The zone at `a`, then the zone at `b`.
    pub ends: [EndRead; 2],
}

/// An arm of a junction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmRead {
    pub street: u32,
    /// The width of the pedestrian crossing across the arm, if it has one.
    pub crossing_mm: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeRead {
    pub uid: u32,
    pub name: Said,
    /// A junction of streets, as opposed to a plain connection or where a street leaves the map.
    pub junction: bool,
    /// The arms, for a junction.
    pub arms: Vec<ArmRead>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetworkRead {
    pub streets: Vec<StreetRead>,
    pub nodes: Vec<NodeRead>,
}

fn lanes_of(street: &Street) -> usize {
    street.segments.iter().filter(|s| matches!(KINDS[s.kind].id, "travel" | "bus")).count()
}

fn end_of(street: &Street, first_end: bool) -> EndRead {
    let zone = street.at_end(first_end);
    EndRead {
        lanes: lanes_of(&zone),
        bike: zone.segments.iter().any(|s| KINDS[s.kind].id == "bike"),
        bus: zone.segments.iter().any(|s| KINDS[s.kind].id == "bus"),
    }
}

impl City {
    /// The streets and junctions as they stand now, for the network-level checks, under the side of the road of `region`.
    pub fn network_read(&self, region: usize) -> NetworkRead {
        let mut streets = Vec::new();
        for (i, e) in self.layout.edges.iter().enumerate() {
            let uid = i as u32 + 1;
            let now = &self.streets[&uid];
            let name = Said::new("map-note-street")
                .with("street", crate::shared::said::Arg::Said(Box::new(self.layout.edge_title(i, now))))
                .with("ends", crate::shared::said::Arg::Said(Box::new(self.layout.edge_ends(i))));
            streets.push(StreetRead {
                uid,
                a: node_uid(e.a),
                b: node_uid(e.b),
                name,
                pedestrian: now.class == StreetClass::Pedestrian,
                freeway: e.class.is_freeway(),
                bike: now.sections().iter().any(|s| s.iter().any(|p| KINDS[p.kind].id == "bike")),
                walkway_ok: walkway_access(now),
                ends: [end_of(now, true), end_of(now, false)],
            });
        }
        let mut nodes = Vec::new();
        for (i, def) in self.layout.nodes.iter().enumerate() {
            let uid = node_uid(i);
            let arms = if def.junction {
                // the junction as its editor reads it, with the crossings the streets now allow
                self.junction_editor(uid, region)
                    .map(|j| j.current().arms.iter().map(|a| ArmRead { street: a.edge, crossing_mm: a.crossing.map(|c| c.width_mm) }).collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            nodes.push(NodeRead { uid, name: self.layout.node_name(i), junction: def.junction, arms });
        }
        NetworkRead { streets, nodes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_read_has_every_street_and_node_with_the_arms_of_its_junctions() {
        let city = City::new();
        let read = city.network_read(0);
        assert_eq!(read.streets.len(), 23);
        assert!(read.streets.iter().any(|s| s.freeway) && read.streets.iter().all(|s| !s.pedestrian));
        let junctions: Vec<_> = read.nodes.iter().filter(|n| n.junction).collect();
        assert!(!junctions.is_empty() && junctions.iter().all(|n| n.arms.len() >= 3));
        assert!(read.nodes.iter().filter(|n| !n.junction).all(|n| n.arms.is_empty()));
        // a street with travel lanes reads them at both ends
        assert!(read.streets.iter().filter(|s| !s.freeway).all(|s| s.ends[0].lanes > 0 && s.ends[1].lanes > 0));
    }

    #[test]
    fn a_street_made_pedestrian_reads_as_one_with_no_lanes_and_its_arm_has_no_crossing() {
        let mut city = City::new();
        let mut e = city.street_editor(10, 0).unwrap();
        assert!(e.make_pedestrian(crate::shared::catalogue::kind_index("sidewalk").unwrap()));
        assert!(city.keep_street(10, e.snapshot()));
        let read = city.network_read(0);
        let s = read.streets.iter().find(|s| s.uid == 10).unwrap();
        assert!(s.pedestrian && s.ends[0].lanes == 0 && s.ends[1].lanes == 0);
        assert!(read.nodes.iter().flat_map(|n| &n.arms).filter(|a| a.street == 10).all(|a| a.crossing_mm.is_none()));
    }
}
