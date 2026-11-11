//! Cutting a network into tiles, merging tiles back, and cutting a place out of the result.
//!
//! The cuts are made in longitude and latitude: a frame maps them linearly onto its metres, so a straight stretch of
//! a centre line is straight in either, and a box's edges are where its degrees say.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{Control, Coordinate, Frame, Junction, Network, NodeId, Point, Provenance, Road, RoadId, Zone};

/// A piece of a whole area's network, with the frame its positions are in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkTile {
    pub frame: Frame,
    pub network: Network,
}

/// A cut piece shorter than this, in metres, is where a road only touches the box, and is left out.
const MIN_PIECE_M: f64 = 0.001;

impl Network {
    /// Roads whose centre line reaches into `rect` (a Frame, as lon/lat), whole, with the junctions they use;
    /// priority controls keep only roads present, sorted by id; corridors cleared. `from` is this network's frame.
    pub fn reaching(&self, from: &Frame, rect: &Frame) -> Network {
        let degrees = Degrees::of(from);
        let roads: Vec<Road> = self.roads.iter().filter(|r| reaches(&degrees.line(&r.points), rect)).cloned().collect();
        let used: BTreeSet<NodeId> = roads.iter().flat_map(|r| [r.edge.from, r.edge.to]).collect();
        let present: BTreeSet<RoadId> = roads.iter().map(|r| r.edge.id).collect();
        let junctions = self
            .junctions
            .iter()
            .filter(|j| used.contains(&j.id))
            .map(|j| {
                let mut j = j.clone();
                if let Control::Priority(ids) = &mut j.control {
                    ids.retain(|id| present.contains(id));
                    ids.sort_unstable();
                }
                j
            })
            .collect();
        Network { left_hand: self.left_hand, junctions, roads, corridors: Vec::new() }
    }

    /// Union by id: roads and junctions of `other` this network lacks are added (same frame assumed). Where both hold
    /// the same id with different content, this network's is kept, except that a junction both have gains the
    /// priority roads only `other`'s names (a tile leaves out those it lacks), its list then sorted by id, the order
    /// `reaching` and `clip_to` give, so that merged tiles cut as the whole network does. The corridors are left as
    /// they are: they are derived once the network is cut (`clip_to`).
    pub fn absorb(&mut self, other: &Network) {
        let roads: BTreeSet<RoadId> = self.roads.iter().map(|r| r.edge.id).collect();
        self.roads.extend(other.roads.iter().filter(|r| !roads.contains(&r.edge.id)).cloned());
        for theirs in &other.junctions {
            match self.junctions.iter_mut().find(|j| j.id == theirs.id) {
                None => self.junctions.push(theirs.clone()),
                Some(ours) => {
                    if let (Control::Priority(ours), Control::Priority(more)) = (&mut ours.control, &theirs.control) {
                        for id in more {
                            if !ours.contains(id) {
                                ours.push(*id);
                            }
                        }
                        ours.sort_unstable();
                    }
                }
            }
        }
    }

    /// What lies in `to`, re-expressed in `to`'s frame: roads wholly inside kept, crossing roads cut at the
    /// border (one new road id per inside piece, a new empty-provenance end junction), lanes laid out,
    /// corridors derived, priorities pruned. `from` is this network's frame.
    ///
    /// A road that leaves the box and comes back is two pieces, never joined across the gap; a road that only
    /// touches the box, or whose piece has fewer than two points, is left out. A priority naming a cut road names
    /// the piece that reaches the junction. A junction no road left uses is left out; one kept is as it was, but
    /// for its position and the pieces its priority names. The roads are taken in order of id, and the result's roads,
    /// junctions and priority lists are sorted by id, so the order the network holds them in makes no difference.
    pub fn clip_to(&self, from: &Frame, to: &Frame) -> Network {
        let degrees = Degrees::of(from);
        let metres = Metres::of(to);
        let mut next_road = self.roads.iter().map(|r| r.edge.id.0).max().map_or(0, |id| id + 1);
        let mut next_node = self.junctions.iter().map(|j| j.id.0).max().map_or(0, |id| id + 1);
        let mut roads = Vec::new();
        let mut ends = Vec::new();
        let mut whole: BTreeSet<RoadId> = BTreeSet::new();
        // (the road cut, the junction of its own a piece reaches) -> the piece
        let mut pieces_at: BTreeMap<(RoadId, NodeId), RoadId> = BTreeMap::new();
        let mut in_order: Vec<&Road> = self.roads.iter().collect();
        in_order.sort_by_key(|r| r.edge.id);
        for road in in_order {
            let line = degrees.line(&road.points);
            let pieces = clip_line(&line, to);
            if let [piece] = pieces.as_slice()
                && piece.from_start
                && piece.to_end
            {
                let mut kept = road.clone();
                kept.points = line.iter().map(|&p| metres.point(p)).collect();
                whole.insert(kept.edge.id);
                roads.push(kept);
                continue;
            }
            let along = along(&road.points);
            for piece in pieces {
                let points: Vec<Point> = piece.points.iter().map(|&p| metres.point(p)).collect();
                if points.len() < 2 || points.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>() < MIN_PIECE_M {
                    continue;
                }
                let id = RoadId(next_road);
                next_road += 1;
                let mut end = |at: Point| {
                    let node = NodeId(next_node);
                    next_node += 1;
                    ends.push(Junction { id: node, provenance: Provenance::default(), position: at, junction: false, control: Control::None });
                    node
                };
                let start = if piece.from_start {
                    pieces_at.insert((road.edge.id, road.edge.from), id);
                    road.edge.from
                } else {
                    end(points[0])
                };
                let finish = if piece.to_end {
                    pieces_at.insert((road.edge.id, road.edge.to), id);
                    road.edge.to
                } else {
                    end(*points.last().expect("two points"))
                };
                let mut cut = road.clone();
                cut.edge.id = id;
                cut.edge.from = start;
                cut.edge.to = finish;
                // each zone keeps its lanes over the stretch of the piece it covered, measured from the piece's start
                cut.zones = zones_between(&road.zones, along.at(piece.start), along.at(piece.end));
                cut.points = points;
                cut.lay_out_lanes();
                roads.push(cut);
            }
        }
        let used: BTreeSet<NodeId> = roads.iter().flat_map(|r| [r.edge.from, r.edge.to]).collect();
        let mut junctions: Vec<Junction> = self
            .junctions
            .iter()
            .filter(|j| used.contains(&j.id))
            .map(|j| {
                let mut j = j.clone();
                j.position = metres.point(degrees.of_point(j.position));
                if let Control::Priority(ids) = &mut j.control {
                    let mut named = Vec::new();
                    for id in ids.iter() {
                        let now = if whole.contains(id) { Some(*id) } else { pieces_at.get(&(*id, j.id)).copied() };
                        if let Some(now) = now.filter(|now| !named.contains(now)) {
                            named.push(now);
                        }
                    }
                    named.sort_unstable();
                    *ids = named;
                }
                j
            })
            .collect();
        junctions.extend(ends);
        junctions.sort_by_key(|j| j.id);
        roads.sort_by_key(|r| r.edge.id);
        let mut place = Network { left_hand: self.left_hand, junctions, roads, corridors: Vec::new() };
        place.derive_corridors();
        place
    }
}

/// From a frame's metres to degrees, with its size worked out once.
struct Degrees<'a> {
    frame: &'a Frame,
    width_m: f64,
    height_m: f64,
}

impl<'a> Degrees<'a> {
    fn of(frame: &'a Frame) -> Degrees<'a> {
        Degrees { frame, width_m: frame.width_m(), height_m: frame.height_m() }
    }

    fn of_point(&self, p: Point) -> [f64; 2] {
        self.frame.lon_lat_sized(self.width_m, self.height_m, p.x_m(), p.y_m())
    }

    fn line(&self, points: &[Point]) -> Vec<[f64; 2]> {
        points.iter().map(|&p| self.of_point(p)).collect()
    }
}

/// From degrees to a frame's metres, with its size worked out once; `Frame::local` otherwise.
struct Metres<'a> {
    frame: &'a Frame,
    width_m: f64,
    height_m: f64,
}

impl<'a> Metres<'a> {
    fn of(frame: &'a Frame) -> Metres<'a> {
        Metres { frame, width_m: frame.width_m(), height_m: frame.height_m() }
    }

    fn point(&self, [lon, lat]: [f64; 2]) -> Point {
        let f = self.frame;
        Point::new((lon - f.west) / (f.east - f.west) * self.width_m, (lat - f.south) / (f.north - f.south) * self.height_m)
    }
}

/// The part of the stretch from `a` to `b` that lies in the box, as the fractions of the way from `a` where it starts
/// and ends (Liang–Barsky); none when the stretch misses the box. A stretch that only touches it starts and ends at
/// the same fraction. The box's edges are in it.
fn clip_segment(a: [f64; 2], b: [f64; 2], r: &Frame) -> Option<(f64, f64)> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [(-dx, a[0] - r.west), (dx, r.east - a[0]), (-dy, a[1] - r.south), (dy, r.north - a[1])] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                if t > t1 {
                    return None;
                }
                t0 = t0.max(t);
            } else {
                if t < t0 {
                    return None;
                }
                t1 = t1.min(t);
            }
        }
    }
    Some((t0, t1))
}

/// Whether a line in degrees reaches into the box: a point in it, or a stretch across it.
fn reaches(line: &[[f64; 2]], r: &Frame) -> bool {
    match line {
        [p] => clip_segment(*p, *p, r).is_some(),
        _ => line.windows(2).any(|w| clip_segment(w[0], w[1], r).is_some()),
    }
}

/// Where along a line: the index of a stretch and the fraction of the way along it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct At(usize, f64);

/// A run of a line inside the box.
struct Piece {
    points: Vec<[f64; 2]>,
    start: At,
    end: At,
    /// Whether it starts at the line's first point, and so at the road's own end.
    from_start: bool,
    /// Whether it ends at the line's last point.
    to_end: bool,
}

/// The runs of a line, in degrees, that lie in the box, in order. A run ends where the line leaves the box (or
/// touches its edge from inside and turns out), and the next starts where it comes back. Points cut at an edge are
/// put on it exactly.
fn clip_line(line: &[[f64; 2]], r: &Frame) -> Vec<Piece> {
    let on_edge = |p: [f64; 2]| [p[0].clamp(r.west, r.east), p[1].clamp(r.south, r.north)];
    let lerp = |a: [f64; 2], b: [f64; 2], t: f64| {
        if t == 0.0 {
            a
        } else if t == 1.0 {
            b
        } else {
            on_edge([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t])
        }
    };
    let mut pieces = Vec::new();
    // the run being followed; it always ends at the point the next stretch starts from
    let mut open: Option<Piece> = None;
    // whether every stretch before this one has no length, so that this one starts at the line's first point
    let mut at_first = true;
    for (i, w) in line.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        if a == b {
            // a stretch of no length: the run, if any, goes on through it
            if let Some(piece) = &mut open {
                piece.end = At(i, 1.0);
            }
            continue;
        }
        let first = std::mem::replace(&mut at_first, false);
        match clip_segment(a, b, r) {
            Some((t0, t1)) if t1 > t0 => {
                if t0 > 0.0 {
                    pieces.extend(open.take());
                }
                let piece = open.get_or_insert_with(|| Piece {
                    points: vec![lerp(a, b, t0)],
                    start: At(i, t0),
                    end: At(i, t0),
                    from_start: first && t0 == 0.0,
                    to_end: false,
                });
                piece.points.push(lerp(a, b, t1));
                piece.end = At(i, t1);
                if t1 < 1.0 {
                    pieces.extend(open.take());
                }
            }
            _ => pieces.extend(open.take()),
        }
    }
    if let Some(mut piece) = open {
        piece.to_end = true;
        pieces.push(piece);
    }
    pieces
}

/// How far along a road's centre line each of its points lies, in metres.
struct Along(Vec<f64>);

fn along(points: &[Point]) -> Along {
    let mut sums = vec![0.0];
    for w in points.windows(2) {
        sums.push(sums.last().copied().unwrap_or(0.0) + w[0].distance(w[1]));
    }
    Along(sums)
}

impl Along {
    fn at(&self, At(i, t): At) -> f64 {
        let start = self.0[i];
        start + (self.0.get(i + 1).copied().unwrap_or(start) - start) * t
    }
}

/// The zones of the stretch of a road from `start` to `end` metres along it, measured from `start`: the zone `start`
/// lies in begins at 0, and each that begins before `end` follows.
fn zones_between(zones: &[Zone], start: f64, end: f64) -> Vec<Zone> {
    let mut found: Vec<Zone> = Vec::new();
    for (k, zone) in zones.iter().enumerate() {
        let next = zones.get(k + 1).map_or(f64::INFINITY, |z| z.from.metres());
        if next <= start {
            continue;
        }
        if zone.from.metres() >= end && !found.is_empty() {
            break;
        }
        let mut zone = zone.clone();
        zone.from = Coordinate::new((zone.from.metres() - start).max(0.0));
        found.push(zone);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Control, Corridor, Direction, Edge, Junction, Lane, LaneKind, NodeId, NodeRef, Point, Provenance, Road, RoadId, RoadType, UserClass, WayRef, Zone,
    };

    /// About a kilometre on each side, in Montreal.
    fn frame() -> Frame {
        Frame { south: 45.5, west: -73.6, north: 45.509, east: -73.5872 }
    }

    /// The box of `f` from (`x0`, `y0`) to (`x1`, `y1`), in metres of `f`.
    fn sub(f: &Frame, x0: f64, y0: f64, x1: f64, y1: f64) -> Frame {
        let ([west, south], [east, north]) = (f.lon_lat(x0, y0), f.lon_lat(x1, y1));
        Frame { south, west, north, east }
    }

    /// The middle of the kilometre: 400 m to 600 m both ways.
    fn middle() -> Frame {
        sub(&frame(), 400.0, 400.0, 600.0, 600.0)
    }

    /// Where a position of `from` lies in `to`'s metres.
    fn moved(from: &Frame, to: &Frame, x: f64, y: f64) -> [f64; 2] {
        let [lon, lat] = from.lon_lat(x, y);
        to.local(lon, lat)
    }

    fn junction(id: u32, x: f64, y: f64) -> Junction {
        Junction {
            id: NodeId(id),
            provenance: Provenance::of_nodes([NodeRef::with_version(1000 + id as i64, 3)]),
            position: Point::new(x, y),
            junction: true,
            control: Control::Signs,
        }
    }

    fn lane(kind: LaneKind) -> Lane {
        Lane { kind, direction: Direction::Forward, width_m: 3.0, buffer_type: None, windows: vec![], shape: vec![] }
    }

    fn road(id: u32, from: u32, to: u32, points: &[(f64, f64)]) -> Road {
        Road {
            edge: Edge { id: RoadId(id), from: NodeId(from), to: NodeId(to) },
            provenance: Provenance::of_ways([WayRef::with_version(500 + id as i64, 2)]),
            name: format!("Road {id}"),
            road_type: RoadType::Local,
            zones: vec![Zone::whole(vec![lane(LaneKind::Driving)])],
            points: points.iter().map(|&(x, y)| Point::new(x, y)).collect(),
        }
    }

    /// The network of these roads, with a junction at each end of each, where it lies.
    fn network(roads: Vec<Road>) -> Network {
        let mut junctions: Vec<Junction> = Vec::new();
        for r in &roads {
            for (id, p) in [(r.edge.from, r.points[0]), (r.edge.to, *r.points.last().unwrap())] {
                if !junctions.iter().any(|j| j.id == id) {
                    junctions.push(junction(id.0, p.x_m(), p.y_m()));
                }
            }
        }
        Network { left_hand: false, junctions, roads, corridors: vec![] }
    }

    fn road_ids(net: &Network) -> Vec<u32> {
        net.roads.iter().map(|r| r.edge.id.0).collect()
    }

    fn junction_ids(net: &Network) -> Vec<u32> {
        net.junctions.iter().map(|j| j.id.0).collect()
    }

    fn junction_of(net: &Network, id: NodeId) -> &Junction {
        net.junctions.iter().find(|j| j.id == id).unwrap_or_else(|| panic!("no junction {id:?} in {net:?}"))
    }

    fn near(a: [f64; 2], b: Point, tolerance_m: f64) -> bool {
        (a[0] - b.x_m()).abs() <= tolerance_m && (a[1] - b.y_m()).abs() <= tolerance_m
    }

    #[test]
    fn reaching_keeps_whole_the_roads_with_a_point_in_the_box_or_a_stretch_across_it() {
        let net = network(vec![
            road(1, 1, 2, &[(300.0, 500.0), (500.0, 500.0)]),
            road(2, 3, 4, &[(300.0, 450.0), (700.0, 450.0)]),
            road(3, 5, 6, &[(100.0, 100.0), (200.0, 100.0)]),
        ]);
        let tile = net.reaching(&frame(), &middle());
        assert_eq!(road_ids(&tile), [1, 2], "the road wholly outside is left out");
        assert_eq!(tile.roads, net.roads[..2], "the roads are whole, as they were");
        assert_eq!(junction_ids(&tile), [1, 2, 3, 4], "the junctions the roads use, with their ids");
        assert_eq!(tile.junctions, net.junctions[..4]);
    }

    #[test]
    fn reaching_names_only_present_roads_in_a_priority_and_has_no_corridors() {
        let mut net = network(vec![road(1, 1, 2, &[(300.0, 500.0), (500.0, 500.0)]), road(2, 1, 3, &[(300.0, 500.0), (100.0, 500.0)])]);
        net.junctions[0].control = Control::Priority(vec![RoadId(2), RoadId(1)]);
        net.derive_corridors();
        assert!(!net.corridors.is_empty());
        let tile = net.reaching(&frame(), &middle());
        assert_eq!(road_ids(&tile), [1]);
        assert_eq!(junction_of(&tile, NodeId(1)).control, Control::Priority(vec![RoadId(1)]));
        assert_eq!(tile.corridors, Vec::<Corridor>::new());
    }

    #[test]
    fn absorbing_a_network_that_shares_roads_adds_each_road_and_junction_once_with_its_id() {
        let whole = network(vec![
            road(1, 1, 2, &[(0.0, 0.0), (100.0, 0.0)]),
            road(2, 2, 3, &[(100.0, 0.0), (200.0, 0.0)]),
            road(3, 3, 4, &[(200.0, 0.0), (300.0, 0.0)]),
            road(4, 4, 5, &[(300.0, 0.0), (400.0, 0.0)]),
        ]);
        let mut a = network(whole.roads[..3].to_vec());
        let b = network(whole.roads[1..].to_vec());
        a.absorb(&b);
        assert_eq!(road_ids(&a), [1, 2, 3, 4]);
        assert_eq!(junction_ids(&a), [1, 2, 3, 4, 5]);
        assert_eq!(a.roads, whole.roads);
        assert_eq!(a.junctions, whole.junctions);
    }

    #[test]
    fn absorbing_gives_back_the_priority_roads_a_tile_left_out() {
        let mut whole = network(vec![road(1, 1, 2, &[(0.0, 0.0), (100.0, 0.0)]), road(2, 2, 3, &[(100.0, 0.0), (200.0, 0.0)])]);
        whole.junctions[1].control = Control::Priority(vec![RoadId(1), RoadId(2)]);
        let mut a = whole.clone();
        a.roads.truncate(1);
        a.junctions.truncate(2);
        a.junctions[1].control = Control::Priority(vec![RoadId(1)]);
        let mut b = whole.clone();
        b.roads.remove(0);
        b.junctions.remove(0);
        b.junctions[0].control = Control::Priority(vec![RoadId(2)]);
        a.absorb(&b);
        assert_eq!(a, whole);
    }

    #[test]
    fn clip_to_keeps_a_road_inside_with_its_id_and_ends() {
        let net = network(vec![road(7, 1, 2, &[(450.0, 450.0), (500.0, 520.0), (550.0, 550.0)])]);
        let (from, to) = (frame(), middle());
        let place = net.clip_to(&from, &to);
        assert_eq!(road_ids(&place), [7]);
        assert_eq!(place.roads[0].edge, net.roads[0].edge);
        assert_eq!(place.roads[0].provenance, net.roads[0].provenance);
        for (p, q) in place.roads[0].points.iter().zip(&net.roads[0].points) {
            assert!(near(moved(&from, &to, q.x_m(), q.y_m()), *p, 1e-6), "{p:?} for {q:?}");
        }
        assert_eq!(place.roads[0].points.len(), 3);
        let kept = junction_of(&place, NodeId(1));
        assert_eq!((&kept.provenance, kept.junction, &kept.control), (&net.junctions[0].provenance, true, &Control::Signs), "a kept junction is as it was");
    }

    #[test]
    fn clip_to_cuts_a_crossing_road_at_the_border_and_ends_it_in_a_street_running_off_the_map() {
        let net = network(vec![road(7, 1, 2, &[(500.0, 500.0), (800.0, 500.0)]), road(3, 3, 1, &[(450.0, 500.0), (500.0, 500.0)])]);
        let (from, to) = (frame(), middle());
        let place = net.clip_to(&from, &to);
        let cut = place.roads.iter().find(|r| r.edge.id != RoadId(3)).expect("the inside piece");
        assert!(cut.edge.id.0 > 7, "a cut piece has a new id above the network's: {:?}", cut.edge);
        assert_eq!(cut.provenance, net.roads[0].provenance);
        assert_eq!(cut.edge.from, NodeId(1), "the end inside is the road's own");
        assert!(cut.edge.to.0 > 3, "the end at the border is new: {:?}", cut.edge);
        let end = junction_of(&place, cut.edge.to);
        assert!(end.provenance.is_empty());
        assert!(!end.junction);
        assert_eq!(end.control, Control::None);
        assert!((end.position.x_m() - to.width_m()).abs() <= 0.001, "on the east edge: {:?} of {}", end.position, to.width_m());
        assert!((end.position.y_m() - moved(&from, &to, 600.0, 500.0)[1]).abs() <= 0.001, "{:?}", end.position);
        assert_eq!(*cut.points.last().unwrap(), end.position, "the centre line ends at its end");
        assert!((cut.length_m() - 100.0).abs() < 0.1, "{}", cut.length_m());
        assert_eq!(cut.base_lanes()[0].shape.last().unwrap().x_m(), cut.length_m(), "the lanes are laid out along the piece");
        assert_eq!(junction_ids(&place).len(), 3, "the junction outside is gone");
        assert_eq!(place.corridors, [Corridor { user: UserClass::Driver, roads: vec![RoadId(3), cut.edge.id] }]);
    }

    #[test]
    fn clip_to_makes_two_roads_of_one_that_leaves_and_comes_back() {
        let net = network(vec![road(4, 1, 2, &[(450.0, 500.0), (700.0, 500.0), (700.0, 550.0), (450.0, 550.0)])]);
        let (from, to) = (frame(), middle());
        let place = net.clip_to(&from, &to);
        assert_eq!(place.roads.len(), 2, "{place:?}");
        let (a, b) = (&place.roads[0], &place.roads[1]);
        assert_ne!(a.edge.id, b.edge.id);
        assert!(a.edge.id.0 > 4 && b.edge.id.0 > 4);
        assert_eq!((a.edge.from, b.edge.to), (NodeId(1), NodeId(2)), "each keeps the road's own end inside");
        assert!(![b.edge.from, b.edge.to].contains(&a.edge.to), "they never meet across the gap");
        for r in [a, b] {
            assert!((r.length_m() - 150.0).abs() < 0.1, "{}", r.length_m());
        }
        assert_eq!(place.junctions.len(), 4);
    }

    #[test]
    fn clip_to_cuts_a_road_at_its_point_on_the_border_where_it_turns_out() {
        let net = network(vec![road(1, 1, 2, &[(500.0, 500.0), (600.0, 500.0), (700.0, 500.0)])]);
        let (from, to) = (frame(), middle());
        let place = net.clip_to(&from, &to);
        assert_eq!(place.roads.len(), 1, "{place:?}");
        let cut = &place.roads[0];
        assert_eq!(cut.points.len(), 2, "{cut:?}");
        assert_eq!(cut.edge.from, NodeId(1));
        let end = junction_of(&place, cut.edge.to);
        assert!(end.provenance.is_empty() && !end.junction, "{end:?}");
        assert!((end.position.x_m() - to.width_m()).abs() <= 0.001, "{:?}", end.position);
    }

    #[test]
    fn clip_to_leaves_out_a_road_that_touches_the_box_at_a_point() {
        let net = network(vec![road(1, 1, 2, &[(300.0, 500.0), (400.0, 500.0), (300.0, 450.0)]), road(2, 3, 4, &[(300.0, 300.0), (400.0, 400.0)])]);
        let place = net.clip_to(&frame(), &middle());
        assert!(place.roads.is_empty(), "{place:?}");
        assert!(place.junctions.is_empty(), "{place:?}");
    }

    #[test]
    fn clip_to_puts_the_south_west_corner_of_the_box_at_the_origin() {
        let net = network(vec![road(1, 1, 2, &[(400.0, 400.0), (500.0, 500.0)])]);
        let place = net.clip_to(&frame(), &middle());
        let corner = junction_of(&place, NodeId(1)).position;
        assert!(corner.x_m().abs() < 1e-9 && corner.y_m().abs() < 1e-9, "{corner:?}");
        assert_eq!(place.roads[0].points[0], corner);
    }

    #[test]
    fn clip_to_gives_a_priority_on_a_cut_road_to_its_piece() {
        let mut net = network(vec![road(1, 1, 2, &[(500.0, 500.0), (800.0, 500.0)]), road(2, 3, 1, &[(450.0, 500.0), (500.0, 500.0)])]);
        net.junctions[0].control = Control::Priority(vec![RoadId(1), RoadId(2), RoadId(9)]);
        let place = net.clip_to(&frame(), &middle());
        let piece = place.roads.iter().find(|r| r.edge.id != RoadId(2)).unwrap().edge.id;
        assert_eq!(junction_of(&place, NodeId(1)).control, Control::Priority(vec![RoadId(2), piece]), "sorted by id");
    }

    #[test]
    fn clip_to_carries_each_zone_to_the_stretch_of_the_piece_it_covered() {
        let mut r = road(1, 1, 2, &[(300.0, 500.0), (500.0, 500.0)]);
        r.zones.push(Zone { from: crate::Coordinate::new(50.0), speed_kmh: Some(30), lanes: vec![lane(LaneKind::Bike)] });
        r.zones.push(Zone { from: crate::Coordinate::new(150.0), speed_kmh: Some(40), lanes: vec![lane(LaneKind::Bus)] });
        let place = network(vec![r]).clip_to(&frame(), &middle());
        let zones = &place.roads[0].zones;
        let starts: Vec<(f64, Option<u16>)> = zones.iter().map(|z| (z.from.metres(), z.speed_kmh)).collect();
        assert_eq!(starts.len(), 2, "{starts:?}");
        assert_eq!(starts[0], (0.0, Some(30)), "the zone the box starts in starts the piece");
        assert!((starts[1].0 - 50.0).abs() < 0.01 && starts[1].1 == Some(40), "{starts:?}");
        assert!(Road::is_zoned(zones));
    }

    #[test]
    fn clip_to_keeps_whole_a_road_inside_that_starts_or_ends_with_a_repeated_point() {
        let net =
            network(vec![road(1, 1, 2, &[(450.0, 450.0), (450.0, 450.0), (550.0, 550.0)]), road(2, 3, 4, &[(450.0, 500.0), (550.0, 500.0), (550.0, 500.0)])]);
        let place = net.clip_to(&frame(), &middle());
        let edges: Vec<Edge> = place.roads.iter().map(|r| r.edge).collect();
        assert_eq!(edges, [net.roads[0].edge, net.roads[1].edge], "kept whole, with their ids and ends");
        assert_eq!(junction_ids(&place), [1, 2, 3, 4]);
    }

    /// The network with the ids it made up for its cut pieces and ends (those not in `whole`) numbered in order,
    /// from 1000, so two cuts that differ only in where their new ids start compare equal.
    fn renumbered(mut net: Network, whole: &Network) -> Network {
        let new_roads: Vec<RoadId> = net.roads.iter().map(|r| r.edge.id).filter(|id| !whole.roads.iter().any(|r| r.edge.id == *id)).collect();
        let new_nodes: Vec<NodeId> = net.junctions.iter().map(|j| j.id).filter(|id| !whole.junctions.iter().any(|j| j.id == *id)).collect();
        let road = |id: RoadId| new_roads.iter().position(|n| *n == id).map_or(id, |k| RoadId(1000 + k as u32));
        let node = |id: NodeId| new_nodes.iter().position(|n| *n == id).map_or(id, |k| NodeId(1000 + k as u32));
        for r in &mut net.roads {
            r.edge = Edge { id: road(r.edge.id), from: node(r.edge.from), to: node(r.edge.to) };
        }
        for j in &mut net.junctions {
            j.id = node(j.id);
            if let Control::Priority(ids) = &mut j.control {
                ids.iter_mut().for_each(|id| *id = road(*id));
            }
        }
        for c in &mut net.corridors {
            c.roads.iter_mut().for_each(|id| *id = road(*id));
        }
        net
    }

    #[test]
    fn a_place_across_the_line_between_two_tiles_is_cut_from_them_as_from_the_whole() {
        let mut whole = network(vec![
            road(1, 1, 2, &[(420.0, 520.0), (480.0, 560.0)]),
            road(2, 3, 4, &[(300.0, 500.0), (580.0, 500.0)]),
            road(3, 4, 5, &[(580.0, 500.0), (700.0, 500.0)]),
            road(6, 4, 6, &[(580.0, 500.0), (580.0, 800.0)]),
            road(8, 7, 8, &[(100.0, 100.0), (900.0, 900.0)]),
            road(9, 9, 10, &[(450.0, 580.0), (650.0, 580.0), (650.0, 590.0), (450.0, 590.0)]),
            // far from both tiles, with the largest ids, so the whole makes up its new ids from higher than the tiles do
            road(99, 98, 99, &[(100.0, 950.0), (200.0, 950.0)]),
        ]);
        whole.junctions.iter_mut().find(|j| j.id == NodeId(4)).unwrap().control = Control::Priority(vec![RoadId(6), RoadId(2), RoadId(3)]);
        let from = frame();
        // two tiles split at x = 500 m, each widened by a 50 m margin; the place straddles the line
        let (tile_a, tile_b) = (sub(&from, 0.0, 300.0, 550.0, 700.0), sub(&from, 450.0, 300.0, 1000.0, 700.0));
        let place = middle();
        let (a, b) = (whole.reaching(&from, &tile_a), whole.reaching(&from, &tile_b));
        assert!(!road_ids(&a).contains(&3) && road_ids(&b).contains(&3), "the tiles differ: {:?} {:?}", road_ids(&a), road_ids(&b));
        let expected = renumbered(whole.clip_to(&from, &place), &whole);
        for (first, second) in [(&a, &b), (&b, &a)] {
            let mut merged = first.clone();
            merged.absorb(second);
            assert_eq!(renumbered(merged.clip_to(&from, &place), &whole), expected);
        }
        assert_eq!(junction_of(&expected, NodeId(4)).control, Control::Priority(vec![RoadId(1000), RoadId(1001), RoadId(1002)]), "{expected:?}");
    }

    #[test]
    fn a_tile_is_kept_as_json() {
        let tile = NetworkTile { frame: frame(), network: network(vec![road(1, 1, 2, &[(0.0, 0.0), (10.0, 0.0)])]) };
        let json = serde_json::to_string(&tile).unwrap();
        assert_eq!(serde_json::from_str::<NetworkTile>(&json).unwrap(), tile);
    }
}
