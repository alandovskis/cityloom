//! The source-data quality inspector: where the imported OpenStreetMap data may be missing or wrong, read from
//! `SourceRead` (the streets and nodes as the importer laid them out) and judged here with no DOM, signals or I/O.
//!
//! A finding is not a design-rule failure. It says what looks missing or doubtful in the source (`source`), which
//! design reading it could mislead (`impact`) and which places it concerns; it has no severity, and it is counted
//! apart from the checks on each place and from the network checks. What the importer cannot tell (a tagged width
//! from osm2streets' default, a buffer's type) is not looked at.
//!
//! Every threshold is a placeholder, UNVERIFIED against a design manual or the OpenStreetMap data model.

use std::collections::{BTreeMap, BTreeSet};

use crate::city::model::SourceRead;
use crate::map::network::{Place, PlaceKind};
use crate::shared::catalogue::{KINDS, is_roadway};
use crate::shared::said::{Arg, Said};

/// A street narrower than this between its property lines is implausible. UNVERIFIED: a placeholder.
pub const MIN_ROW_MM: i32 = 3_000;
/// A street wider than this is implausible. UNVERIFIED: a placeholder (a freeway with its ramps can be wide).
pub const MAX_ROW_MM: i32 = 60_000;

/// What a finding is about; the groups of the panel, in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A street with no name in the source.
    Name,
    /// Streets joined to each other and to nothing else.
    Disconnected,
    /// A street that ends at a node of its own.
    DeadEnd,
    /// A width outside what is plausible.
    Width,
    /// A tag a design reads that the source lacks.
    Speed,
}

impl Kind {
    pub const ALL: [Kind; 5] = [Kind::Name, Kind::Disconnected, Kind::DeadEnd, Kind::Width, Kind::Speed];

    /// For an element id; and the message `prov-kind-<id>` that words it.
    pub fn id(self) -> &'static str {
        match self {
            Kind::Name => "name",
            Kind::Disconnected => "disconnected",
            Kind::DeadEnd => "ends",
            Kind::Width => "width",
            Kind::Speed => "speed",
        }
    }

    /// The messages that say where the doubt comes from and which design reading it could mislead.
    fn keys(self) -> (&'static str, &'static str) {
        match self {
            Kind::Name => ("prov-source-name", "prov-impact-name"),
            Kind::Disconnected => ("prov-source-disconnected", "prov-impact-disconnected"),
            Kind::DeadEnd => ("prov-source-ends", "prov-impact-ends"),
            Kind::Width => ("prov-source-width", "prov-impact-width"),
            Kind::Speed => ("prov-source-speed", "prov-impact-speed"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceFinding {
    pub kind: Kind,
    /// What looks missing or doubtful.
    pub said: Said,
    /// Where the doubt comes from: what the source holds, or the importer cannot tell.
    pub source: Said,
    /// Which design reading it could mislead.
    pub impact: Said,
    pub places: Vec<Place>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceReport {
    /// Every finding, by kind and then by place.
    pub findings: Vec<SourceFinding>,
}

pub fn inspect(read: &SourceRead) -> SourceReport {
    let ctx = Ctx::new(read);
    let mut findings = names(&ctx);
    findings.extend(disconnected(&ctx));
    findings.extend(dead_ends(&ctx));
    findings.extend(widths(&ctx));
    findings.extend(speeds(&ctx));
    SourceReport { findings }
}

struct Ctx<'a> {
    read: &'a SourceRead,
    /// Per node uid: the street ends that meet there, as indices into `read.streets` (a loop counts twice).
    at: BTreeMap<u32, Vec<usize>>,
}

impl<'a> Ctx<'a> {
    fn new(read: &'a SourceRead) -> Ctx<'a> {
        let mut at: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for (i, s) in read.streets.iter().enumerate() {
            at.entry(s.a).or_default().push(i);
            at.entry(s.b).or_default().push(i);
        }
        Ctx { read, at }
    }

    fn place(&self, i: usize) -> Place {
        let s = &self.read.streets[i];
        Place { kind: PlaceKind::Street, uid: s.uid, name: s.name.clone() }
    }

    fn called(&self, i: usize) -> Arg {
        Arg::Said(Box::new(self.read.streets[i].called.clone()))
    }

    fn open(&self, node: u32) -> bool {
        self.read.nodes.iter().any(|n| n.uid == node && n.open)
    }

    fn found(&self, kind: Kind, said: Said, places: Vec<Place>) -> SourceFinding {
        let (source, impact) = kind.keys();
        SourceFinding { kind, said, source: Said::new(source), impact: Said::new(impact), places }
    }
}

/// Streets OpenStreetMap does not name: the name shown is an invented one.
fn names(ctx: &Ctx) -> Vec<SourceFinding> {
    (0..ctx.read.streets.len())
        .filter(|&i| !ctx.read.streets[i].named)
        .map(|i| ctx.found(Kind::Name, Said::new("prov-name").with("street", ctx.called(i)), vec![ctx.place(i)]))
        .collect()
}

fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// Groups of streets joined to each other and to no others, other than the biggest. A group that reaches a node
/// where the road is cut at the area's box is not reported: it may join the rest outside the area.
fn disconnected(ctx: &Ctx) -> Vec<SourceFinding> {
    let streets = &ctx.read.streets;
    let mut parent: Vec<usize> = (0..streets.len()).collect();
    for ends in ctx.at.values() {
        for &i in &ends[1..] {
            let (a, b) = (root(&mut parent, ends[0]), root(&mut parent, i));
            parent[b] = a;
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..streets.len() {
        let r = root(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    let mut groups: Vec<Vec<usize>> = groups.into_values().collect();
    // the biggest first; a tie goes to the group with the lowest street
    groups.sort_by_key(|g| (std::cmp::Reverse(g.len()), g[0]));
    groups
        .into_iter()
        .skip(1)
        .filter(|g| !g.iter().any(|&i| ctx.open(streets[i].a) || ctx.open(streets[i].b)))
        .map(|g| ctx.found(Kind::Disconnected, Said::new("prov-disconnected").with("n", Arg::Num(g.len() as i64)), g.iter().map(|&i| ctx.place(i)).collect()))
        .collect()
}

/// A street that ends at an OpenStreetMap node no other street reaches. It may be a real dead end.
fn dead_ends(ctx: &Ctx) -> Vec<SourceFinding> {
    let junctions: BTreeSet<u32> = ctx.read.nodes.iter().filter(|n| n.junction || n.open).map(|n| n.uid).collect();
    ctx.at
        .iter()
        .filter(|(node, ends)| ends.len() == 1 && !junctions.contains(node))
        .map(|(_, ends)| ctx.found(Kind::DeadEnd, Said::new("prov-ends").with("street", ctx.called(ends[0])), vec![ctx.place(ends[0])]))
        .collect()
}

/// A street whose width is implausible, and lanes at the least or most the catalogue allows for their kind: the
/// importer keeps a lane within those limits, so a lane at one may have been cut to it. The importer cannot tell a
/// width the road is tagged with from the one osm2streets assumes.
fn widths(ctx: &Ctx) -> Vec<SourceFinding> {
    let mut found = Vec::new();
    for (i, s) in ctx.read.streets.iter().enumerate() {
        if s.row_mm < MIN_ROW_MM || s.row_mm > MAX_ROW_MM {
            let (key, limit) = if s.row_mm < MIN_ROW_MM { ("prov-width-row-narrow", MIN_ROW_MM) } else { ("prov-width-row-wide", MAX_ROW_MM) };
            found.push(ctx.found(
                Kind::Width,
                Said::new(key).with("street", ctx.called(i)).with("width", Arg::Length(s.row_mm)).with("limit", Arg::Length(limit)),
                vec![ctx.place(i)],
            ));
        }
        for lane in s.lanes.iter().filter(|l| is_roadway(l.kind)) {
            let k = &KINDS[lane.kind];
            let (key, limit) = if lane.width_mm <= k.min_mm {
                ("prov-width-lane-narrow", k.min_mm)
            } else if lane.width_mm >= k.max_mm {
                ("prov-width-lane-wide", k.max_mm)
            } else {
                continue;
            };
            found.push(ctx.found(
                Kind::Width,
                Said::new(key).with("street", ctx.called(i)).with("kind", Arg::KindLower(k.id)).with("limit", Arg::Length(limit)),
                vec![ctx.place(i)],
            ));
        }
    }
    found
}

/// Streets with no speed limit in the source, other than pedestrian streets, which have no motor traffic.
fn speeds(ctx: &Ctx) -> Vec<SourceFinding> {
    (0..ctx.read.streets.len())
        .filter(|&i| ctx.read.streets[i].no_speed && !ctx.read.streets[i].pedestrian)
        .map(|i| ctx.found(Kind::Speed, Said::new("prov-speed").with("street", ctx.called(i)), vec![ctx.place(i)]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::city::model::{LaneRead, SourceNode, SourceStreet};
    use crate::shared::catalogue::kind_index;

    fn street(uid: u32, a: u32, b: u32) -> SourceStreet {
        SourceStreet {
            uid,
            a,
            b,
            name: Said::new("n").with("uid", Arg::Num(uid as i64)),
            called: Said::new("c"),
            named: true,
            row_mm: 12_000,
            lanes: vec![LaneRead { kind: kind_index("travel").unwrap(), width_mm: 3_300 }],
            no_speed: false,
            pedestrian: false,
        }
    }

    fn node(uid: u32, junction: bool, open: bool) -> SourceNode {
        SourceNode { uid, name: Said::new("node"), junction, open }
    }

    /// Streets 1..=4 in a line over real nodes 1..=5, the ends cut at the box.
    fn line() -> SourceRead {
        SourceRead { streets: (1..=4).map(|i| street(i, i, i + 1)).collect(), nodes: (1..=5).map(|i| node(i, i > 1 && i < 5, i == 1 || i == 5)).collect() }
    }

    fn of(read: &SourceRead, kind: Kind) -> Vec<SourceFinding> {
        inspect(read).findings.into_iter().filter(|f| f.kind == kind).collect()
    }

    fn uids(f: &SourceFinding) -> Vec<u32> {
        f.places.iter().map(|p| p.uid).collect()
    }

    #[test]
    fn a_clean_network_has_no_findings() {
        assert_eq!(inspect(&line()), SourceReport::default());
    }

    #[test]
    fn a_street_with_no_name_is_found_and_says_its_source_and_impact() {
        let mut r = line();
        r.streets[1].named = false;
        let f = of(&r, Kind::Name);
        assert_eq!(f.len(), 1);
        assert_eq!(uids(&f[0]), vec![2]);
        assert_eq!((f[0].source.key, f[0].impact.key), ("prov-source-name", "prov-impact-name"));
    }

    #[test]
    fn a_group_of_streets_cut_off_from_the_rest_is_found_but_not_the_biggest() {
        let mut r = line();
        // streets 5 and 6 join each other, on real nodes of their own
        r.streets.push(street(5, 10, 11));
        r.streets.push(street(6, 11, 12));
        r.nodes.extend([node(10, false, false), node(11, true, false), node(12, false, false)]);
        let f = of(&r, Kind::Disconnected);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(uids(&f[0]), vec![5, 6]);
    }

    #[test]
    fn a_fragment_cut_at_the_area_s_box_is_not_reported() {
        let mut r = line();
        r.streets.push(street(5, 10, 11));
        r.nodes.extend([node(10, false, true), node(11, false, false)]);
        assert!(of(&r, Kind::Disconnected).is_empty());
        // and a street ending at the box is no dead end
        assert!(of(&r, Kind::DeadEnd).iter().all(|f| uids(f) != vec![1] && uids(f) != vec![4]));
    }

    #[test]
    fn a_street_ending_at_a_real_node_of_its_own_is_a_dead_end_notice() {
        let mut r = line();
        r.nodes[4] = node(5, false, false);
        let f = of(&r, Kind::DeadEnd);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(uids(&f[0]), vec![4]);
    }

    #[test]
    fn an_implausible_street_width_is_found_and_a_lane_at_its_limit() {
        let mut r = line();
        r.streets[0].row_mm = MIN_ROW_MM - 1;
        r.streets[1].row_mm = MAX_ROW_MM + 1;
        let travel = kind_index("travel").unwrap();
        r.streets[2].lanes = vec![LaneRead { kind: travel, width_mm: KINDS[travel].min_mm }];
        r.streets[3].lanes = vec![LaneRead { kind: travel, width_mm: KINDS[travel].max_mm }];
        let f = of(&r, Kind::Width);
        let keys: Vec<&str> = f.iter().map(|f| f.said.key).collect();
        assert_eq!(keys, ["prov-width-row-narrow", "prov-width-row-wide", "prov-width-lane-narrow", "prov-width-lane-wide"]);
        assert_eq!(f.iter().map(|f| uids(f)[0]).collect::<Vec<_>>(), [1, 2, 3, 4]);
    }

    #[test]
    fn a_sidewalk_at_its_limit_is_not_a_roadway_width_finding() {
        let mut r = line();
        let sidewalk = kind_index("sidewalk").unwrap();
        r.streets[0].lanes = vec![LaneRead { kind: sidewalk, width_mm: KINDS[sidewalk].max_mm }];
        assert!(of(&r, Kind::Width).is_empty());
    }

    #[test]
    fn a_street_without_a_speed_limit_is_found_unless_it_is_pedestrian() {
        let mut r = line();
        r.streets[0].no_speed = true;
        r.streets[1].no_speed = true;
        r.streets[1].pedestrian = true;
        let f = of(&r, Kind::Speed);
        assert_eq!(f.len(), 1);
        assert_eq!(uids(&f[0]), vec![1]);
    }

    #[test]
    fn the_findings_come_in_the_order_of_the_kinds() {
        let mut r = line();
        r.streets[0].no_speed = true;
        r.streets[3].named = false;
        r.streets[2].row_mm = 100;
        let order: Vec<Kind> = inspect(&r).findings.iter().map(|f| f.kind).collect();
        let mut sorted = order.clone();
        sorted.sort_by_key(|k| Kind::ALL.iter().position(|a| a == k));
        assert_eq!(order, sorted);
    }
}
