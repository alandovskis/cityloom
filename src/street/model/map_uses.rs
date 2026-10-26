//! The curbside uses and transit stops as the map draws them: where each stands along the street and across it, in
//! millimetres, with the sides and positions of the section turned into the street's own axes.
//!
//! Along values count from node `a` (the street's first end), ascending, whatever end the programme is measured
//! from (`Street::from_end`). Lateral values are positive to the right of the axis looking from `a` to `b`, so
//! `Side::Left` (the section's first piece) is negative and a point is `offset * (-ty, tx)` from the axis in layout
//! millimetres. A programme crossing a zone boundary is cut there, because each zone has its own section.

use super::curb::{Use, curbside};
use super::stop::{StopKind, bus_lane, stand};
use super::{Segment, Side, Street, ZoneEnd, ZoneFrame};

/// One part of a curbside use: where it stands along the street and across it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapUse {
    pub side: Side,
    pub use_: Use,
    pub from_mm: i32,
    pub to_mm: i32,
    pub lo_mm: i32,
    pub hi_mm: i32,
}

/// One part of a transit stop: where its platform stands along the street and across it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapStop {
    pub side: Side,
    pub kind: StopKind,
    pub name: Option<String>,
    pub from_mm: i32,
    pub to_mm: i32,
    pub lo_mm: i32,
    pub hi_mm: i32,
}

/// Everything a street has to draw on the map.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapUses {
    pub uses: Vec<MapUse>,
    pub stops: Vec<MapStop>,
}

/// The lateral edges of every piece of a section, as `PieceView.offset_mm` places them: laid from the edge of the
/// street's row (`row_mm`, one value for all its zones), so a section narrower than the row sits left-aligned.
fn edges(segs: &[Segment], row_mm: i32) -> Vec<(i32, i32)> {
    let mut at = -row_mm / 2;
    segs.iter()
        .map(|s| {
            let e = (at, at + s.width_mm);
            at += s.width_mm;
            e
        })
        .collect()
}

impl Street {
    /// The parts of the programme (`curb`, `stops`) to draw on the map, laid along `frame.length_mm` (the street as
    /// loaded). A part is one use or stop inside one zone; a part whose zone has no piece to stand on draws nothing.
    pub fn map_uses(&self, frame: ZoneFrame) -> MapUses {
        let length = frame.length_mm;
        // Each zone: its anchored span, its section and the lateral edges of the section's pieces.
        let zones: Vec<((i32, i32), &[Segment], Vec<(i32, i32)>)> = self
            .sections()
            .into_iter()
            .enumerate()
            .map(|(z, segs)| {
                ((self.zones.get(z).map_or(0, |g| g.from_mm), self.zones.get(z + 1).map_or(length, |g| g.from_mm)), segs, edges(segs, self.row_mm))
            })
            .collect();
        // The parts of the anchored span `from..to` inside the street, one per zone, as positions from node a.
        let parts = |from: i32, to: i32| {
            let (from, to) = (from.max(0), to.min(length));
            zones
                .iter()
                .filter_map(|((z_from, z_to), segs, edges)| {
                    let (a, b) = (from.max(*z_from), to.min(*z_to));
                    let along = if frame.from_end == ZoneEnd::Last { (length - b, length - a) } else { (a, b) };
                    (a < b).then_some((along, *segs, edges))
                })
                .collect::<Vec<_>>()
        };

        let mut uses = Vec::new();
        for i in &self.curb {
            for ((from_mm, to_mm), segs, edges) in parts(i.from_mm, i.from_mm + i.length_mm) {
                if let Some(p) = curbside(segs, i.side) {
                    uses.push(MapUse { side: i.side, use_: i.use_, from_mm, to_mm, lo_mm: edges[p].0, hi_mm: edges[p].1 });
                }
            }
        }
        let mut stops = Vec::new();
        for st in &self.stops {
            for ((from_mm, to_mm), segs, edges) in parts(st.from_mm, st.to_mm()) {
                let Some(extent) = stop_extent(segs, edges, st.side, self.side, st.kind) else { continue };
                stops.push(MapStop { side: st.side, kind: st.kind, name: st.name.clone(), from_mm, to_mm, lo_mm: extent.0, hi_mm: extent.1 });
            }
        }
        uses.sort_by_key(|u| (u.from_mm, u.side == Side::Right));
        stops.sort_by_key(|s| (s.from_mm, s.side == Side::Right));
        MapUses { uses, stops }
    }
}

/// The lateral extent of the platform a stop of `kind` stands on in a section, or `None` when the section has no bus
/// lane on `side` or lacks what the kind needs (the stop's check fails, so there is nothing to draw it on).
fn stop_extent(segs: &[Segment], edges: &[(i32, i32)], side: Side, driving: Side, kind: StopKind) -> Option<(i32, i32)> {
    let lane = bus_lane(segs, side, driving)?;
    let found = stand(segs, lane, side, kind)?;
    let pieces: Vec<usize> = match kind {
        StopKind::Island => found.raised,
        StopKind::Curb | StopKind::Bulb => found.stand.into_iter().collect(),
    };
    let lo = pieces.iter().map(|&p| edges[p].0).min()?;
    let hi = pieces.iter().map(|&p| edges[p].1).max()?;
    Some((lo, hi))
}

#[cfg(test)]
mod tests {
    use super::super::curb::Instance;
    use super::super::stop::Stop;
    use super::super::zones::Zone;
    use super::*;
    use crate::shared::catalogue::kind_index;

    const LEN: i32 = 60_000;

    fn first() -> ZoneFrame {
        ZoneFrame { length_mm: LEN, from_end: ZoneEnd::First }
    }

    fn section(pieces: &[(&str, i32)]) -> Vec<Segment> {
        pieces.iter().enumerate().map(|(i, (k, w))| Segment::new(i as u32 + 1, kind_index(k).unwrap(), *w)).collect()
    }

    /// Sidewalk 3 000, parking 2 500, two travel lanes (one each way), parking 2 500, sidewalk 3 000: 18 000.
    fn base() -> Vec<(&'static str, i32)> {
        vec![("sidewalk", 3_000), ("parking", 2_500), ("travel", 3_500), ("travel", 3_500), ("parking", 2_500), ("sidewalk", 3_000)]
    }

    fn street(pieces: &[(&str, i32)]) -> Street {
        let mut s = Street::sample(0, Side::Right);
        s.segments = section(pieces);
        // Right-hand traffic: the right half runs away from you (direction 0), the left half toward you.
        let lanes: Vec<usize> = (0..s.segments.len()).filter(|&i| s.segments[i].kind == kind_index("travel").unwrap()).collect();
        s.segments[lanes[0]].direction = Some(1);
        s.segments[lanes[1]].direction = Some(0);
        s.row_mm = pieces.iter().map(|p| p.1).sum();
        s
    }

    fn use_of(side: Side, from_mm: i32, length_mm: i32) -> Instance {
        Instance { side, from_mm, length_mm, use_: Use::Parking }
    }

    fn stop(side: Side, from_mm: i32, length_mm: i32, kind: StopKind) -> Stop {
        Stop { side, from_mm, length_mm, kind, shelter: false, name: None }
    }

    /// The point `offset` to the right of `at` for a street running along `(tx, ty)`, in layout mm (x east, y south).
    fn point(at: (f64, f64), t: (f64, f64), offset: f64) -> (f64, f64) {
        (at.0 - t.1 * offset, at.1 + t.0 * offset)
    }

    #[test]
    fn left_is_the_first_piece_on_a_street_running_east_and_north() {
        let mut s = street(&base());
        s.curb = vec![use_of(Side::Left, 10_000, 6_000), use_of(Side::Right, 10_000, 6_000)];
        s.length_mm = Some(LEN);
        let m = s.map_uses(first());
        let (l, r) = (m.uses.iter().find(|u| u.side == Side::Left).unwrap(), m.uses.iter().find(|u| u.side == Side::Right).unwrap());
        assert_eq!((l.lo_mm, l.hi_mm), (-6_000, -3_500));
        assert_eq!((r.lo_mm, r.hi_mm), (3_500, 6_000));
        // Heading east (+x), right is south (+y): Left is north, so its y is smaller.
        let (e, n) = ((1.0, 0.0), (0.0, -1.0));
        assert!(point((0.0, 0.0), e, f64::from(l.lo_mm)).1 < 0.0);
        assert!(point((0.0, 0.0), e, f64::from(r.lo_mm)).1 > 0.0);
        // Heading north (-y), right is east: Left is west, so its x is smaller.
        assert!(point((0.0, 0.0), n, f64::from(l.lo_mm)).0 < 0.0);
        assert!(point((0.0, 0.0), n, f64::from(r.lo_mm)).0 > 0.0);
    }

    #[test]
    fn anchored_at_the_last_end_the_range_is_turned_round() {
        let mut s = street(&base());
        s.curb = vec![use_of(Side::Left, 10_000, 6_000)];
        s.length_mm = Some(LEN);
        s.from_end = ZoneEnd::Last;
        let m = s.map_uses(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::Last });
        assert_eq!(m.uses.len(), 1);
        assert_eq!((m.uses[0].from_mm, m.uses[0].to_mm), (44_000, 50_000));
        assert_eq!(m.uses[0].side, Side::Left);
    }

    fn zoned_by_30(second: &[(&str, i32)]) -> Street {
        let mut s = street(&base());
        s.curb = vec![use_of(Side::Left, 24_000, 12_000)];
        s.length_mm = Some(LEN);
        s.zones = vec![Zone { from_mm: 0, segments: None, name: None }, Zone { from_mm: 30_000, segments: Some(section(second)), name: None }];
        s
    }

    #[test]
    fn a_use_is_cut_at_a_zone_boundary() {
        let wider = [("sidewalk", 3_000), ("parking", 3_500), ("travel", 3_500), ("travel", 3_500), ("parking", 2_500), ("sidewalk", 3_000)];
        let m = zoned_by_30(&wider).map_uses(first());
        assert_eq!(m.uses.len(), 2);
        assert_eq!((m.uses[0].from_mm, m.uses[0].to_mm, m.uses[0].lo_mm, m.uses[0].hi_mm), (24_000, 30_000, -6_000, -3_500));
        // Every section is laid from the row's edge (the street's 18 000): the parking piece lies -9 000 + 3 000 .. + 3 500.
        assert_eq!((m.uses[1].from_mm, m.uses[1].to_mm, m.uses[1].lo_mm, m.uses[1].hi_mm), (30_000, 36_000, -6_000, -2_500));
    }

    #[test]
    fn a_part_without_a_curbside_piece_draws_nothing() {
        let none = [("sidewalk", 3_000), ("travel", 3_500), ("travel", 3_500), ("sidewalk", 3_000)];
        let m = zoned_by_30(&none).map_uses(first());
        assert_eq!(m.uses.len(), 1);
        assert_eq!((m.uses[0].from_mm, m.uses[0].to_mm), (24_000, 30_000));
    }

    #[test]
    fn a_curb_stop_stands_on_the_sidewalk_and_a_bulb_on_the_parking_piece() {
        // 15 500 wide, no parking on the left: a curb stop fits there, a bulb on the right.
        let mut s = street(&[("sidewalk", 3_000), ("travel", 3_500), ("travel", 3_500), ("parking", 2_500), ("sidewalk", 3_000)]);
        s.length_mm = Some(LEN);
        s.stops = vec![stop(Side::Left, 10_000, 12_000, StopKind::Curb), stop(Side::Right, 30_000, 12_000, StopKind::Bulb)];
        let m = s.map_uses(first());
        assert_eq!(m.stops.len(), 2);
        // Curb: the left sidewalk, -7 750 .. -4 750; bulb: the right parking piece, 2 250 .. 4 750.
        assert_eq!((m.stops[0].kind, m.stops[0].lo_mm, m.stops[0].hi_mm), (StopKind::Curb, -7_750, -4_750));
        assert_eq!((m.stops[1].kind, m.stops[1].lo_mm, m.stops[1].hi_mm), (StopKind::Bulb, 2_250, 4_750));
        assert_eq!((m.stops[0].from_mm, m.stops[0].to_mm, m.stops[0].side), (10_000, 22_000, Side::Left));
    }

    #[test]
    fn an_island_spans_its_raised_pieces() {
        let mut s =
            street(&[("sidewalk", 3_000), ("bike", 1_800), ("buffer", 700), ("parking", 2_500), ("travel", 3_500), ("travel", 3_500), ("sidewalk", 3_000)]);
        s.length_mm = Some(LEN);
        s.stops = vec![stop(Side::Left, 5_000, 10_000, StopKind::Island)];
        let m = s.map_uses(first());
        // The left half of the section runs toward you: the nearest bus lane is the first travel lane. The island is
        // the buffer and the parking piece between it and the bike lane (outward).
        assert_eq!(m.stops.len(), 1);
        assert_eq!((m.stops[0].lo_mm, m.stops[0].hi_mm), (-9_000 + 3_000 + 1_800, -9_000 + 3_000 + 1_800 + 700 + 2_500));
    }

    #[test]
    fn a_stop_whose_kind_does_not_fit_is_not_drawn() {
        let mut s = street(&[("sidewalk", 3_000), ("travel", 3_500), ("travel", 3_500), ("sidewalk", 3_000)]);
        s.length_mm = Some(LEN);
        s.stops = vec![stop(Side::Right, 10_000, 12_000, StopKind::Bulb)];
        assert!(s.map_uses(first()).stops.is_empty());
    }

    #[test]
    fn a_street_without_a_programme_has_none() {
        let s = street(&base());
        assert_eq!(s.map_uses(first()), MapUses::default());
    }

    #[test]
    fn a_narrower_section_stays_where_its_pieces_are_drawn() {
        // 16 000 wide on the street's 18 000 row: laid from the row's edge, so left-aligned, not centred.
        let narrower = [("sidewalk", 2_000), ("parking", 2_500), ("travel", 3_500), ("travel", 3_500), ("parking", 2_500), ("sidewalk", 2_000)];
        let mut s = zoned_by_30(&narrower);
        s.curb = vec![use_of(Side::Left, 24_000, 12_000), use_of(Side::Right, 24_000, 12_000)];
        let m = s.map_uses(first());
        let at = |side, from| m.uses.iter().find(|u| u.side == side && u.from_mm == from).map(|u| (u.lo_mm, u.hi_mm)).unwrap();
        assert_eq!(at(Side::Left, 24_000), (-6_000, -3_500));
        assert_eq!(at(Side::Left, 30_000), (-7_000, -4_500));
        assert_eq!(at(Side::Right, 24_000), (3_500, 6_000));
        assert_eq!(at(Side::Right, 30_000), (2_500, 5_000));
    }

    #[test]
    fn a_single_zone_under_filled_street_is_left_aligned() {
        let mut s = street(&[("sidewalk", 2_000), ("parking", 2_500), ("travel", 3_500), ("travel", 3_500), ("parking", 2_500), ("sidewalk", 2_000)]);
        s.row_mm = 18_000;
        s.curb = vec![use_of(Side::Left, 0, 6_000)];
        s.length_mm = Some(LEN);
        let m = s.map_uses(first());
        assert_eq!((m.uses[0].lo_mm, m.uses[0].hi_mm), (-7_000, -4_500));
    }

    #[test]
    fn anchored_at_the_last_end_a_zone_cut_is_turned_round_too() {
        let wider = [("sidewalk", 3_000), ("parking", 3_500), ("travel", 3_500), ("travel", 3_500), ("parking", 2_500), ("sidewalk", 3_000)];
        let mut s = zoned_by_30(&wider);
        s.from_end = ZoneEnd::Last;
        let m = s.map_uses(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::Last });
        // Anchored 24 000..30 000 (zone 1) is 30 000..36 000 from node a; 30 000..36 000 (zone 2) is 24 000..30 000.
        let parts: Vec<_> = m.uses.iter().map(|u| (u.from_mm, u.to_mm, u.lo_mm, u.hi_mm)).collect();
        assert_eq!(parts, vec![(24_000, 30_000, -6_000, -2_500), (30_000, 36_000, -6_000, -3_500)]);
    }
}
