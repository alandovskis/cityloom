//! Turbo roundabouts: the rules of the five Dutch layouts and four blocks, with no drawing in them.
//!
//! The figures are placeholders taken from secondary sources that reproduce CROW (2008): FHWA-SA-20-019
//! *Turbo Roundabouts* and Džambas et al., CETRA 2016, Table 1. CROW 126 itself could not be read, so these are
//! UNVERIFIED: which turns each entry lane serves (the commonly known rule is used), the lane counts of basic,
//! spiral and rotor, every figure for a third ring lane, the labels Δv / Δu (CETRA's are used) and the skew an arm
//! may have (no figure is published).

use serde::{Deserialize, Serialize};

use crate::junction::geometry::gap;
use crate::junction::model::{Arm, LEFT, Profile, RIGHT, THROUGH};
use crate::shared::catalogue::StreetClass;

/// The most an arm may be off its template direction before the edit is refused, and the least that is reported
/// by the `turbo-square` check. UNVERIFIED placeholders. `jn-refusal-turbo-skewed` says "20°".
pub const MAX_SKEW_DEG: f64 = 20.0;
pub const WARN_SKEW_DEG: f64 = 10.0;
/// Two arms are opposite when their bearings are 180° apart, give or take this.
pub const OPPOSITE_TOLERANCE_DEG: i32 = 20;
/// How far the translation axis is turned from the main road, with four arms and with three.
pub const AXIS_4_ARMS_DEG: f64 = 57.5;
pub const AXIS_3_ARMS_DEG: f64 = 65.0;
/// The painted divider between two ring lanes (R3 - R2).
pub const DIVIDER_MM: i32 = 300;
/// A crossing stands at least this far from the ring on a turbo.
pub const CROSSING_SETBACK_MM: i32 = 6_000;
/// An entry flares out 1 m sideways in this many metres along.
pub const FLARE: i32 = 10;
/// How wide each entry lane of a turbo is where the junction widens its approach. A PLACEHOLDER (UNVERIFIED): the
/// sources give the widths of the ring lanes (R4 - R3 and the like), not of the entry lanes. The ring lanes keep the
/// template's widths.
pub const TURBO_ENTRY_LANE_MM: i32 = 3_500;

/// One of the layouts. `arms` are the arm counts it suits; `key` names it, `label_key` says "… turbo roundabout, { $size } size".
#[derive(Clone, Copy, Debug)]
pub struct TurboLayout {
    pub id: &'static str,
    pub key: &'static str,
    pub label_key: &'static str,
    pub arms: &'static [usize],
    /// Entry lanes on a main arm and on any other arm. A knee has `main_entry` on its first main arm only.
    pub main_entry: usize,
    pub other_entry: usize,
    /// Ring lanes (UNVERIFIED for basic, spiral and rotor).
    pub ring_lanes: usize,
    pub knee: bool,
}

pub const LAYOUTS: [TurboLayout; 5] = [
    TurboLayout {
        id: "basic",
        key: "jn-turbo-basic",
        label_key: "jn-turbo-label-basic",
        arms: &[4],
        main_entry: 2,
        other_entry: 2,
        ring_lanes: 2,
        knee: false,
    },
    TurboLayout { id: "egg", key: "jn-turbo-egg", label_key: "jn-turbo-label-egg", arms: &[3, 4], main_entry: 2, other_entry: 1, ring_lanes: 2, knee: false },
    TurboLayout { id: "knee", key: "jn-turbo-knee", label_key: "jn-turbo-label-knee", arms: &[3, 4], main_entry: 2, other_entry: 1, ring_lanes: 2, knee: true },
    TurboLayout {
        id: "spiral",
        key: "jn-turbo-spiral",
        label_key: "jn-turbo-label-spiral",
        arms: &[4],
        main_entry: 3,
        other_entry: 2,
        ring_lanes: 3,
        knee: false,
    },
    TurboLayout {
        id: "rotor",
        key: "jn-turbo-rotor",
        label_key: "jn-turbo-label-rotor",
        arms: &[4],
        main_entry: 3,
        other_entry: 3,
        ring_lanes: 3,
        knee: false,
    },
];

/// One edge of a ring lane: the radius of its two half circles and how far apart their centres are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    pub radius_mm: i32,
    pub shift_mm: i32,
}

/// A block (a size): CETRA 2016 Table 1, Dutch values, in millimetres.
#[derive(Clone, Copy, Debug)]
pub struct TurboBlock {
    pub id: &'static str,
    pub key: &'static str,
    pub r1_mm: i32,
    pub r2_mm: i32,
    pub r3_mm: i32,
    pub r4_mm: i32,
    /// Δv: how far apart the centres of the island's half circles are.
    pub dv_mm: i32,
    /// Δu: the same for the lane edges.
    pub du_mm: i32,
}

pub const BLOCKS: [TurboBlock; 4] = [
    TurboBlock { id: "mini", key: "jn-turbo-size-mini", r1_mm: 10_500, r2_mm: 15_850, r3_mm: 16_150, r4_mm: 21_150, dv_mm: 5_750, du_mm: 5_050 },
    TurboBlock { id: "regular", key: "jn-turbo-size-regular", r1_mm: 12_000, r2_mm: 17_150, r3_mm: 17_450, r4_mm: 22_450, dv_mm: 5_350, du_mm: 5_050 },
    TurboBlock { id: "medium", key: "jn-turbo-size-medium", r1_mm: 15_000, r2_mm: 20_000, r3_mm: 20_300, r4_mm: 25_200, dv_mm: 5_150, du_mm: 4_950 },
    TurboBlock { id: "large", key: "jn-turbo-size-large", r1_mm: 20_000, r2_mm: 24_900, r3_mm: 25_200, r4_mm: 29_900, dv_mm: 5_150, du_mm: 4_750 },
];

impl TurboBlock {
    /// The width of the outer ring lane (R4 - R3), which a third ring lane is also made.
    pub fn lane_mm(&self) -> i32 {
        self.r4_mm - self.r3_mm
    }

    /// The outer radius of the ring's carriageway: R4, or with a third ring lane (UNVERIFIED) a further divider and lane out.
    pub fn road_outer_mm(&self, ring_lanes: usize) -> i32 {
        if ring_lanes >= 3 { self.r4_mm + DIVIDER_MM + self.lane_mm() } else { self.r4_mm }
    }

    /// The inner and outer edge of each ring lane, innermost lane first.
    pub fn lanes(&self, ring_lanes: usize) -> Vec<[Edge; 2]> {
        let e = |radius_mm, shift_mm| Edge { radius_mm, shift_mm };
        let mut v = vec![[e(self.r1_mm, self.dv_mm), e(self.r2_mm, self.du_mm)], [e(self.r3_mm, self.du_mm), e(self.r4_mm, self.du_mm)]];
        if ring_lanes >= 3 {
            v.push([e(self.r4_mm + DIVIDER_MM, self.du_mm), e(self.road_outer_mm(3), self.du_mm)]);
        }
        v
    }

    /// The centre line of each divider, between one lane's outer edge and the next lane's inner edge.
    pub fn dividers(&self, ring_lanes: usize) -> Vec<Edge> {
        self.lanes(ring_lanes).windows(2).map(|w| Edge { radius_mm: (w[0][1].radius_mm + w[1][0].radius_mm) / 2, shift_mm: self.du_mm }).collect()
    }
}

/// The turbo a roundabout has: an index into `LAYOUTS`, one into `BLOCKS`, and the two arms (by uid) of the main road.
/// For an egg or knee on three arms the main road is the through road; for a knee the first uid is the arm with two entry lanes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Turbo {
    pub layout: usize,
    pub size: usize,
    pub main: Vec<u32>,
}

impl Turbo {
    pub fn spec(&self) -> Option<&'static TurboLayout> {
        LAYOUTS.get(self.layout)
    }

    pub fn block(&self) -> Option<&'static TurboBlock> {
        BLOCKS.get(self.size)
    }

    /// Whether this turbo can stand on these arms: a layout and a block that exist, a layout made for this many
    /// arms, and two different main arms that are among them.
    pub fn fits_arms(&self, arms: &[Arm]) -> bool {
        let (Some(spec), Some(_)) = (self.spec(), self.block()) else { return false };
        let &[a, b] = self.main.as_slice() else { return false };
        a != b && spec.arms.contains(&arms.len()) && [a, b].iter().all(|u| arms.iter().any(|x| x.uid == *u))
    }

    /// How many lanes the layout brings in at arm `uid` (before a street that cars cannot enter is taken into account).
    pub fn entry_lanes(&self, uid: u32) -> usize {
        let Some(spec) = self.spec() else { return 0 };
        let main = if spec.knee { self.main.first() == Some(&uid) } else { self.main.contains(&uid) };
        if main { spec.main_entry } else { spec.other_entry }
    }

    /// The translation axis, as a bearing: the main road's template direction turned 57.5° (65° with three arms),
    /// clockwise for right-hand traffic and anticlockwise for left-hand.
    pub fn axis_deg(&self, arms: &[Arm], right_hand: bool) -> Option<f64> {
        let theta = main_axis_deg(arms, &self.main)?;
        let turn = if arms.len() == 3 { AXIS_3_ARMS_DEG } else { AXIS_4_ARMS_DEG };
        Some((theta + if right_hand { turn } else { -turn }).rem_euclid(360.0))
    }

    /// The arm furthest from its template direction and by how much; the lowest uid of equals. None when the turbo does not fit.
    pub fn worst_skew(&self, arms: &[Arm]) -> Option<(u32, f64)> {
        if !self.fits_arms(arms) {
            return None;
        }
        skews(arms, &self.main)?.into_iter().max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
    }

    /// The first arm (in the order of `arms`) whose entry this turbo would widen past the street's property line, the ends
    /// of its section (0 and `row_mm`). An arm cars do not enter is not widened and never qualifies. None when no arm
    /// does, and when the turbo does not fit the arms.
    pub fn entry_too_narrow(&self, arms: &[Arm], region: usize) -> Option<u32> {
        if !self.fits_arms(arms) {
            return None;
        }
        arms.iter()
            .find(|a| {
                let prof = a.profile(region);
                if prof.enter_x.is_empty() {
                    return false;
                }
                let (_, span) = widen_entry(&prof, self.entry_lanes(a.uid), TURBO_ENTRY_LANE_MM);
                span.0 < 0 || span.1 > prof.row_mm
            })
            .map(|a| a.uid)
    }

    /// Whether this turbo can stand on these arms: it fits them, no arm is more than `MAX_SKEW_DEG` off its template
    /// and no entry would be widened past its street's property line. What `settle_turbo` keeps, what `set_turbo`
    /// chooses a main road by and what the inspector offers.
    pub fn standable(&self, arms: &[Arm], region: usize) -> bool {
        self.fits_arms(arms) && self.worst_skew(arms).is_some_and(|(_, deg)| deg <= MAX_SKEW_DEG) && self.entry_too_narrow(arms, region).is_none()
    }
}

/// The indexes into `LAYOUTS` of the layouts made for `arm_count` arms.
pub fn layouts_for(arm_count: usize) -> Vec<usize> {
    (0..LAYOUTS.len()).filter(|&i| LAYOUTS[i].arms.contains(&arm_count)).collect()
}

/// The classes of turn lane `k` of `n` entry lanes serves, given the classes the arm has (`avail`). From the driver's
/// left: the first lane left and through, the last right and through, a middle one through only; one lane serves
/// everything. UNVERIFIED (the figures that say it could not be read). The U-turn the first lane also serves has no
/// movement in this model, so nothing stands for it. A lane that would serve nothing the arm has serves what it has.
///
/// On a side arm (one that is not on the main road; `side_arm`) of two or three lanes the outermost lane (the last in
/// right-hand traffic, the first in left-hand) turns toward the outside only, right or left, when the arm has that
/// turn: under the spiral a lane only steps outward, so a street just before a tip reaches nothing but the next exit from
/// its outer lane (spec amendment 2026-10-13, "Lane table"; UNVERIFIED, it depends on which end of the axis leads, J1).
pub fn uses(k: usize, n: usize, avail: u8, side_arm: bool, right_hand: bool) -> u8 {
    let mut raw = match (n, k) {
        (1, _) => LEFT | THROUGH | RIGHT,
        (_, 0) => LEFT | THROUGH,
        (_, k) if k + 1 == n => THROUGH | RIGHT,
        _ => THROUGH,
    };
    let (outermost, outward) = if right_hand { (n.saturating_sub(1), RIGHT) } else { (0, LEFT) };
    if side_arm && n >= 2 && k == outermost && avail & outward != 0 {
        raw = outward;
    }
    match raw & avail {
        0 => avail,
        m => m,
    }
}

/// How main a class of street is, for choosing the main road: Arterial above Collector above Local (above Pedestrian
/// where that class exists); a motorway is above them all.
pub fn class_rank(c: StreetClass) -> u8 {
    match c {
        StreetClass::Motorway => 4,
        StreetClass::Arterial => 3,
        StreetClass::Collector => 2,
        StreetClass::Local => 1,
        StreetClass::Pedestrian => 0,
    }
}

/// Pairs of arms (by uid, the lower first) whose bearings are 180° apart, give or take `OPPOSITE_TOLERANCE_DEG`.
pub fn opposite_pairs(arms: &[Arm]) -> Vec<(u32, u32)> {
    let mut pairs = Vec::new();
    for (i, a) in arms.iter().enumerate() {
        for b in &arms[i + 1..] {
            if (gap(a.bearing, b.bearing) - 180).abs() <= OPPOSITE_TOLERANCE_DEG {
                pairs.push((a.uid.min(b.uid), a.uid.max(b.uid)));
            }
        }
    }
    pairs
}

/// The main roads `layout` can have on these arms, best first: the opposite pairs by the sum of their arms' class
/// ranks, then of their travel lanes, then the lowest uid. A knee has each pair in both orders (the first uid is the
/// arm with two entry lanes). Empty for a layout that does not suit the arm count, or when no two arms are opposite.
pub fn main_candidates(arms: &[Arm], region: usize, layout: usize) -> Vec<Vec<u32>> {
    let Some(spec) = LAYOUTS.get(layout) else { return Vec::new() };
    if !spec.arms.contains(&arms.len()) {
        return Vec::new();
    }
    let score = |uid: u32| {
        arms.iter().find(|a| a.uid == uid).map_or((0u8, 0usize), |a| {
            let p = a.profile(region);
            (a.class().map_or(0, class_rank), p.enter_x.len() + p.leave_x.len())
        })
    };
    let mut pairs = opposite_pairs(arms);
    pairs.sort_by_key(|&(a, b)| {
        let (sa, sb) = (score(a), score(b));
        (std::cmp::Reverse(sa.0 + sb.0), std::cmp::Reverse(sa.1 + sb.1), a, b)
    });
    pairs.into_iter().flat_map(|(a, b)| if spec.knee { vec![vec![a, b], vec![b, a]] } else { vec![vec![a, b]] }).collect()
}

/// An angle in -180..180.
fn signed(deg: f64) -> f64 {
    (deg + 540.0).rem_euclid(360.0) - 180.0
}

/// The direction the main road's first arm would have if the two main arms were exactly opposite: the mean of the
/// first arm's bearing and the second's turned half a circle. None when `main` does not name two arms.
pub fn main_axis_deg(arms: &[Arm], main: &[u32]) -> Option<f64> {
    let &[a, b] = main else { return None };
    if a == b {
        return None;
    }
    let (a, b) = (arms.iter().find(|x| x.uid == a)?, arms.iter().find(|x| x.uid == b)?);
    let diff = signed(b.bearing as f64 - 180.0 - a.bearing as f64);
    Some((a.bearing as f64 + diff / 2.0).rem_euclid(360.0))
}

/// How far each arm is from its template direction, in degrees: the first main arm from θ, the second from θ + 180°,
/// every other arm from θ + 90° or θ + 270°, whichever is nearer. None unless there are three or four arms and `main`
/// names two of them.
pub fn skews(arms: &[Arm], main: &[u32]) -> Option<Vec<(u32, f64)>> {
    if !(3..=4).contains(&arms.len()) {
        return None;
    }
    let theta = main_axis_deg(arms, main)?;
    Some(
        arms.iter()
            .map(|a| {
                let b = a.bearing as f64;
                let dev = if main.contains(&a.uid) {
                    let target = if main.first() == Some(&a.uid) { theta } else { theta + 180.0 };
                    signed(b - target).abs()
                } else {
                    let rel = (b - theta).rem_euclid(360.0);
                    let target = if rel < 180.0 { 90.0 } else { 270.0 };
                    signed(rel - target).abs()
                };
                (a.uid, dev)
            })
            .collect(),
    )
}

/// The entering lanes of a street widened to the layout's `n` lanes of `lane_mm` each: the new extent across the
/// section and the centres of the lanes (driver's left first, like `Profile::enter_x`). The extent keeps the edge
/// nearest the middle of the road and grows away from it, to `n` lanes or the street's own entering width if that is
/// more; the lanes share it equally. The street itself is not changed, and the extent may pass its property line
/// (`Turbo::entry_too_narrow` says so).
pub fn widen_entry(prof: &Profile, n: usize, lane_mm: i32) -> (Vec<i32>, (i32, i32)) {
    let (e0, e1) = prof.enter_span;
    let width = (n as i32 * lane_mm).max(e1 - e0);
    let middle = (prof.road_l + prof.road_r) / 2;
    let span = if (e0 + e1) / 2 > middle { (e0, e0 + width) } else { (e1 - width, e1) };
    let each = width / n.max(1) as i32;
    let xs = (0..n as i32).map(|i| span.1 - each / 2 - i * each).collect();
    (xs, span)
}

/// The ring lane entry lane `k` of `n` (driver's left first) feeds, among `ring_lanes` (0 is the innermost): the left
/// lane the inner ring lane and the right lane the outer one, a middle lane the middle; a lone lane the outer. With
/// left-hand traffic the driver's left is the outside, so the order is reversed.
pub fn ring_lane(k: usize, n: usize, ring_lanes: usize, right_hand: bool) -> usize {
    let kk = if right_hand { k } else { n.saturating_sub(1 + k) };
    let last = ring_lanes.saturating_sub(1);
    if n >= ring_lanes {
        kk.min(last)
    } else if n == 1 || kk > 0 {
        last
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::junction::model::Junction;
    use crate::shared::catalogue::{Side, kind_index};
    use crate::street::model::{Piece, Street};

    #[test]
    fn a_ring_lane_is_always_one_the_ring_has() {
        for ring_lanes in 1..=3 {
            for n in 1..=3 {
                for k in 0..n {
                    for right_hand in [true, false] {
                        assert!(ring_lane(k, n, ring_lanes, right_hand) < ring_lanes, "lane {k} of {n} onto {ring_lanes}, right-hand {right_hand}");
                    }
                }
            }
        }
    }

    fn arm(uid: u32, bearing: i32, street: Street) -> Arm {
        let mut a = Arm::new(uid, bearing, 0);
        a.section = Some(street);
        a
    }

    fn sample(i: usize) -> Street {
        Street::sample(i, Side::Right)
    }

    /// A street of `lanes` travel pieces and nothing else.
    fn travel_street(lanes: usize) -> Street {
        let pieces: Vec<Piece> =
            (0..lanes).map(|_| Piece { kind: kind_index("travel").unwrap(), width_mm: 3_200, direction: Some(1), variants: Vec::new() }).collect();
        Street::imported(StreetClass::Local, Side::Right, &pieces)
    }

    /// A street cars do not enter: sidewalk, bike lane, sidewalk.
    fn walking_street() -> Street {
        let pieces: Vec<Piece> = [("sidewalk", 3_000), ("bike", 2_000), ("sidewalk", 3_000)]
            .iter()
            .map(|&(id, width_mm)| Piece { kind: kind_index(id).unwrap(), width_mm, direction: None, variants: Vec::new() })
            .collect();
        Street::imported(StreetClass::Local, Side::Right, &pieces)
    }

    /// Four arms at the compass points, on the given samples (north, east, south, west).
    fn cross(streets: [usize; 4]) -> Vec<Arm> {
        [0, 90, 180, 270].iter().zip(streets).enumerate().map(|(i, (&b, s))| arm(i as u32 + 1, b, sample(s))).collect()
    }

    #[test]
    fn the_four_blocks_are_the_dutch_ones_of_cetra_table_1() {
        let got: Vec<[i32; 6]> = BLOCKS.iter().map(|b| [b.r1_mm, b.r2_mm, b.r3_mm, b.r4_mm, b.dv_mm, b.du_mm]).collect();
        assert_eq!(
            got,
            [
                [10_500, 15_850, 16_150, 21_150, 5_750, 5_050],
                [12_000, 17_150, 17_450, 22_450, 5_350, 5_050],
                [15_000, 20_000, 20_300, 25_200, 5_150, 4_950],
                [20_000, 24_900, 25_200, 29_900, 5_150, 4_750],
            ]
        );
        assert_eq!(BLOCKS.iter().map(|b| b.r3_mm - b.r2_mm).collect::<Vec<_>>(), [300; 4], "the divider is 300 mm in every block");
        assert_eq!(BLOCKS.iter().map(TurboBlock::lane_mm).collect::<Vec<_>>(), [5_000, 5_000, 4_900, 4_700], "the outer lane is R4 - R3");
        assert_eq!(DIVIDER_MM, 300);
    }

    #[test]
    fn a_block_lays_out_two_ring_lanes_and_a_third_outside_a_second_divider() {
        let mini = &BLOCKS[0];
        let e = |radius_mm, shift_mm| Edge { radius_mm, shift_mm };
        assert_eq!(mini.lanes(2), [[e(10_500, 5_750), e(15_850, 5_050)], [e(16_150, 5_050), e(21_150, 5_050)]]);
        assert_eq!(mini.dividers(2), [e(16_000, 5_050)], "centred in the 300 mm between R2 and R3");
        assert_eq!(mini.road_outer_mm(2), 21_150);
        assert_eq!(mini.lanes(3)[2], [e(21_450, 5_050), e(26_450, 5_050)], "a divider of 300 mm, then a lane of R4 - R3");
        assert_eq!(mini.dividers(3), [e(16_000, 5_050), e(21_300, 5_050)]);
        assert_eq!(mini.road_outer_mm(3), 26_450);
        assert_eq!([&BLOCKS[1], &BLOCKS[2], &BLOCKS[3]].map(|b| b.road_outer_mm(3)), [27_750, 30_400, 34_900]);
        assert_eq!(BLOCKS[1].dividers(2), [e(17_300, 5_050)]);
    }

    #[test]
    fn the_layouts_follow_the_spec_s_table() {
        let row = |l: &TurboLayout| (l.id, l.arms.to_vec(), l.main_entry, l.other_entry, l.ring_lanes, l.knee);
        assert_eq!(
            LAYOUTS.iter().map(row).collect::<Vec<_>>(),
            [
                ("basic", vec![4], 2, 2, 2, false),
                ("egg", vec![3, 4], 2, 1, 2, false),
                ("knee", vec![3, 4], 2, 1, 2, true),
                ("spiral", vec![4], 3, 2, 3, false),
                ("rotor", vec![4], 3, 3, 3, false),
            ]
        );
    }

    #[test]
    fn layouts_are_offered_by_arm_count() {
        assert_eq!(layouts_for(3), [1, 2], "egg and knee");
        assert_eq!(layouts_for(4), [0, 1, 2, 3, 4]);
        assert!(layouts_for(5).is_empty());
        assert!(layouts_for(2).is_empty());
    }

    #[test]
    fn entry_lanes_come_from_the_layout_and_the_main_road() {
        let turbo = |layout| Turbo { layout, size: 0, main: vec![1, 3] };
        let counts = |t: &Turbo| [1, 2, 3, 4].map(|uid| t.entry_lanes(uid));
        assert_eq!(counts(&turbo(0)), [2, 2, 2, 2], "basic");
        assert_eq!(counts(&turbo(1)), [2, 1, 2, 1], "egg: 2 on the main arms, 1 on the others");
        assert_eq!(counts(&turbo(2)), [2, 1, 1, 1], "knee: 2 on the first main arm only");
        assert_eq!(counts(&Turbo { main: vec![3, 1], ..turbo(2) }), [1, 1, 2, 1], "knee: the first uid is the arm with two");
        assert_eq!(counts(&turbo(3)), [3, 2, 3, 2], "spiral");
        assert_eq!(counts(&turbo(4)), [3, 3, 3, 3], "rotor");
    }

    #[test]
    fn a_lane_serves_by_its_place_among_the_arm_s_lanes() {
        let all = LEFT | THROUGH | RIGHT;
        // A main arm (and any lone lane): the first lane left and through, the last through and right, a middle one through only.
        assert_eq!([uses(0, 1, all, false, true)], [all], "one lane serves everything");
        assert_eq!([uses(0, 2, all, false, true), uses(1, 2, all, false, true)], [LEFT | THROUGH, THROUGH | RIGHT]);
        assert_eq!(
            [uses(0, 3, all, false, true), uses(1, 3, all, false, true), uses(2, 3, all, false, true)],
            [LEFT | THROUGH, THROUGH, THROUGH | RIGHT],
            "the middle lane is through only"
        );
        assert_eq!(uses(0, 2, RIGHT, false, true), RIGHT, "a lane that would serve nothing the arm has serves what it has");
        assert_eq!(uses(1, 3, LEFT | RIGHT, false, true), LEFT | RIGHT);
        // A side arm of two or three lanes: its outermost lane turns toward the outside only.
        assert_eq!([uses(0, 2, all, true, true), uses(1, 2, all, true, true)], [LEFT | THROUGH, RIGHT]);
        assert_eq!([uses(0, 3, all, true, true), uses(1, 3, all, true, true), uses(2, 3, all, true, true)], [LEFT | THROUGH, THROUGH, RIGHT]);
        assert_eq!(
            [uses(0, 2, all, true, false), uses(1, 2, all, true, false)],
            [LEFT, THROUGH | RIGHT],
            "left-hand traffic: the outside is the driver's left"
        );
        assert_eq!([uses(0, 3, all, true, false), uses(1, 3, all, true, false), uses(2, 3, all, true, false)], [LEFT, THROUGH, THROUGH | RIGHT]);
        assert_eq!(uses(0, 1, all, true, true), all, "a lone lane keeps every turn, side arm or not (UNVERIFIED)");
        assert_eq!(uses(1, 2, LEFT | THROUGH, true, true), THROUGH, "an arm with no right turn leaves its outer lane as it is");
    }

    #[test]
    fn a_street_class_ranks_by_how_main_it_is() {
        assert!(class_rank(StreetClass::Motorway) > class_rank(StreetClass::Arterial));
        assert!(class_rank(StreetClass::Arterial) > class_rank(StreetClass::Collector));
        assert!(class_rank(StreetClass::Collector) > class_rank(StreetClass::Local));
    }

    #[test]
    fn the_main_road_is_the_opposite_pair_of_the_highest_class() {
        // north and south are the Avenue (arterial), east and west the Street (local)
        let arms = cross([1, 0, 1, 0]);
        assert_eq!(opposite_pairs(&arms), [(1, 3), (2, 4)]);
        assert_eq!(main_candidates(&arms, 0, 0), [vec![1, 3], vec![2, 4]]);
        let arms = cross([0, 1, 0, 1]);
        assert_eq!(main_candidates(&arms, 0, 0), [vec![2, 4], vec![1, 3]], "the arterial pair first, whatever the uids");
    }

    #[test]
    fn of_two_pairs_of_one_class_the_one_with_more_travel_lanes_leads_and_then_the_lowest_uid() {
        let arms = vec![arm(1, 0, sample(0)), arm(2, 90, travel_street(3)), arm(3, 180, sample(0)), arm(4, 270, travel_street(3))];
        assert_eq!(main_candidates(&arms, 0, 0), [vec![2, 4], vec![1, 3]], "three lanes a side beat two");
        let tie = cross([0, 0, 0, 0]);
        assert_eq!(main_candidates(&tie, 0, 0), [vec![1, 3], vec![2, 4]], "a tie goes to the lowest uid");
    }

    #[test]
    fn a_knee_offers_each_pair_in_both_orders_and_a_layout_that_does_not_match_offers_none() {
        let arms = cross([1, 0, 1, 0]);
        assert_eq!(main_candidates(&arms, 0, 2), [vec![1, 3], vec![3, 1], vec![2, 4], vec![4, 2]]);
        assert!(main_candidates(&arms, 0, 99).is_empty());
        let three = vec![arm(1, 90, sample(0)), arm(2, 180, sample(2)), arm(3, 270, sample(0))];
        assert!(main_candidates(&three, 0, 0).is_empty(), "basic needs four arms");
        assert_eq!(main_candidates(&three, 0, 1), [vec![1, 3]], "egg on three: the through road");
    }

    #[test]
    fn a_junction_with_no_opposite_pair_has_no_main_road() {
        let y = vec![arm(1, 0, sample(0)), arm(2, 120, sample(0)), arm(3, 240, sample(0))];
        assert!(opposite_pairs(&y).is_empty());
        assert!(main_candidates(&y, 0, 1).is_empty(), "no layout can be offered");
        let near = vec![arm(1, 0, sample(0)), arm(2, 160, sample(0)), arm(3, 270, sample(0))];
        assert_eq!(opposite_pairs(&near), [(1, 2)], "160 degrees apart is still opposite, within 20");
        let far = vec![arm(1, 0, sample(0)), arm(2, 155, sample(0)), arm(3, 270, sample(0))];
        assert!(opposite_pairs(&far).is_empty());
    }

    #[test]
    fn arms_at_the_template_directions_are_not_skewed_and_others_are_measured_from_them() {
        let arms = cross([1, 0, 1, 0]);
        let main = [1, 3];
        assert_eq!(skews(&arms, &main).unwrap(), [(1, 0.0), (2, 0.0), (3, 0.0), (4, 0.0)]);
        let mut off = arms.clone();
        off[1].bearing = 100;
        let s = skews(&off, &main).unwrap();
        assert!((s[1].1 - 10.0).abs() < 1e-9, "100 degrees is 10 off the east template: {s:?}");
        off[1].bearing = 115;
        assert!((skews(&off, &main).unwrap()[1].1 - 25.0).abs() < 1e-9);
        let t = Turbo { layout: 0, size: 0, main: main.to_vec() };
        let (uid, deg) = t.worst_skew(&off).unwrap();
        assert_eq!(uid, 2);
        assert!((deg - 25.0).abs() < 1e-9);
    }

    #[test]
    fn a_main_pair_that_is_not_quite_opposite_shares_the_error_and_the_template_is_their_mean() {
        let mut arms = cross([1, 0, 1, 0]);
        arms[2].bearing = 170; // the south arm is 10 degrees short of straight on from the north
        let main = [1, 3];
        let theta = main_axis_deg(&arms, &main).unwrap();
        assert!((theta - 355.0).abs() < 1e-9, "the mean of 0 and 170 - 180: {theta}");
        let s = skews(&arms, &main).unwrap();
        assert!((s[0].1 - 5.0).abs() < 1e-9 && (s[2].1 - 5.0).abs() < 1e-9, "5 degrees each: {s:?}");
    }

    #[test]
    fn the_translation_axis_is_57_5_degrees_from_the_main_road_with_four_arms_and_65_with_three() {
        let four = cross([1, 0, 1, 0]);
        let t = Turbo { layout: 0, size: 0, main: vec![1, 3] };
        assert_eq!(t.axis_deg(&four, true), Some(57.5));
        assert_eq!(t.axis_deg(&four, false), Some(302.5), "left-hand traffic mirrors it");
        let three = vec![arm(1, 90, sample(0)), arm(2, 180, sample(2)), arm(3, 270, sample(0))];
        let egg = Turbo { layout: 1, size: 0, main: vec![1, 3] };
        assert_eq!(egg.axis_deg(&three, true), Some(155.0), "the through road runs east-west (theta 90), plus 65");
        assert_eq!(egg.axis_deg(&three, false), Some(25.0), "and minus 65 for left-hand traffic");
    }

    #[test]
    fn a_turbo_fits_only_the_arms_it_names() {
        let four = cross([1, 0, 1, 0]);
        let ok = Turbo { layout: 0, size: 0, main: vec![1, 3] };
        assert!(ok.fits_arms(&four));
        assert!(!Turbo { layout: 9, ..ok.clone() }.fits_arms(&four), "no such layout");
        assert!(!Turbo { size: 4, ..ok.clone() }.fits_arms(&four), "no such block");
        assert!(!Turbo { main: vec![1], ..ok.clone() }.fits_arms(&four), "two main arms");
        assert!(!Turbo { main: vec![1, 1], ..ok.clone() }.fits_arms(&four), "two different ones");
        assert!(!Turbo { main: vec![1, 9], ..ok.clone() }.fits_arms(&four), "arms of the junction");
        assert!(!ok.fits_arms(&four[..3]), "basic is for four arms");
        let three = vec![arm(1, 90, sample(0)), arm(2, 180, sample(2)), arm(3, 270, sample(0))];
        assert!(Turbo { layout: 1, size: 0, main: vec![1, 3] }.fits_arms(&three), "an egg suits three");
        assert_eq!(ok.worst_skew(&four[..3]), None, "a turbo that does not fit has no skew to speak of");
    }

    #[test]
    fn a_street_that_is_entered_is_widened_to_the_layout_s_lanes_away_from_the_middle_of_the_road() {
        // Sample Street 1 (east): the road spans 3300..14700 of 18000 (middle 9000); the lane cars enter by is 5700..9000, left of the middle.
        let street = Junction::new(0).current().arms[1].profile(0);
        assert_eq!(
            (street.enter_x.clone(), street.enter_span, street.road_l, street.road_r, street.row_mm),
            (vec![7_350], (5_700, 9_000), 3_300, 14_700, 18_000)
        );
        // Two lanes of 3.5 m: width = max(2 * 3500, 9000 - 5700) = 7000. The entry is left of the middle (7350 < 9000), so it keeps its right
        // edge, 9000, and grows left: (9000 - 7000, 9000) = (2000, 9000). Each lane is 7000 / 2 = 3500 wide, centred at 9000 - 3500 / 2 = 7250 and 7250 - 3500 = 3750.
        assert_eq!(widen_entry(&street, 2, TURBO_ENTRY_LANE_MM), (vec![7_250, 3_750], (2_000, 9_000)), "two lanes of 3.5 m, growing away from the middle");
        // One lane: width = max(3500, 3300) = 3500, so (9000 - 3500, 9000) = (5500, 9000), centred at 9000 - 1750 = 7250.
        assert_eq!(widen_entry(&street, 1, TURBO_ENTRY_LANE_MM), (vec![7_250], (5_500, 9_000)));
        // Three lanes: width = 10500, so (9000 - 10500, 9000) = (-1500, 9000), 1.5 m past the property line at 0; centres 7250, 3750 and 250.
        assert_eq!(widen_entry(&street, 3, TURBO_ENTRY_LANE_MM), (vec![7_250, 3_750, 250], (-1_500, 9_000)));
        // Lanes of 2 m: width = max(2000, 3300) = 3300, the street's own entry, centred at 9000 - 1650 = 7350.
        assert_eq!(widen_entry(&street, 1, 2_000), (vec![7_350], (5_700, 9_000)), "never narrower than the street's own lanes");
    }

    #[test]
    fn a_turbo_stands_when_it_fits_its_arms_is_square_enough_and_has_room_for_its_entries() {
        let arms = cross([1, 0, 1, 0]);
        let turbo = |layout, main: Vec<u32>| Turbo { layout, size: 0, main };
        assert!(turbo(0, vec![1, 3]).standable(&arms, 0));
        assert!(!turbo(4, vec![1, 3]).standable(&arms, 0), "three lanes on the Street pass its property line");
        assert!(!turbo(0, vec![1, 9]).standable(&arms, 0), "an arm the junction has not");
        let mut skewed = arms.clone();
        skewed[1].bearing = 125; // the east street, 35 degrees off
        assert!(!turbo(0, vec![1, 3]).standable(&skewed, 0));
    }

    #[test]
    fn an_entry_lane_is_3_5_m_wide_as_a_placeholder() {
        assert_eq!(TURBO_ENTRY_LANE_MM, 3_500, "UNVERIFIED: the sources give the widths of the ring lanes, not of the entry lanes");
    }

    #[test]
    fn an_entry_that_the_layout_would_widen_past_the_property_line_is_found_and_named() {
        // Sample Street 1 is 18 m across; its entry lane lies at 5.7..9 m. Three lanes of 3.5 m make it (-1.5 m, 9 m): 1.5 m past the line at 0.
        // Two make it (2 m, 9 m) and one (5.5 m, 9 m): inside. The Avenue (30 m, entry 7.4..14 m) has the room for three: (3.5 m, 14 m).
        let arms = cross([1, 0, 1, 0]); // the Avenue north and south (uids 1 and 3), the Street east and west (uids 2 and 4)
        let rotor = Turbo { layout: 4, size: 0, main: vec![1, 3] };
        assert_eq!(rotor.entry_too_narrow(&arms, 0), Some(2), "three lanes on the Street: the first arm in the order of the arms that does not fit");
        for layout in [0, 1, 2, 3] {
            assert_eq!(Turbo { layout, ..rotor.clone() }.entry_too_narrow(&arms, 0), None, "layout {layout}: two lanes at most on the Street");
        }
        assert_eq!(
            Turbo { layout: 3, size: 0, main: vec![2, 4] }.entry_too_narrow(&arms, 0),
            Some(2),
            "with the Street as the main road a spiral gives it three"
        );
        assert_eq!(rotor.entry_too_narrow(&arms, 3), Some(2), "left-hand traffic widens toward the other line: (9 m, 19.5 m) of 18 m");
        let no_fit = Turbo { layout: 4, size: 0, main: vec![1, 9] };
        assert_eq!(no_fit.entry_too_narrow(&arms, 0), None, "a turbo that does not fit the arms is not this check's business");
        let walkers: Vec<Arm> = [0, 90, 180, 270].iter().enumerate().map(|(i, &b)| arm(i as u32 + 1, b, walking_street())).collect();
        assert_eq!(rotor.entry_too_narrow(&walkers, 0), None, "an arm cars do not enter is not widened");
    }

    #[test]
    fn a_lane_feeds_the_ring_lane_on_its_own_side_and_left_hand_traffic_mirrors_it() {
        let feeds = |n, r, rh| (0..n).map(|k| ring_lane(k, n, r, rh)).collect::<Vec<_>>();
        assert_eq!(feeds(2, 2, true), [0, 1], "the left lane feeds the inner ring lane, the right lane the outer");
        assert_eq!(feeds(3, 3, true), [0, 1, 2], "a middle lane the middle ring lane");
        assert_eq!(feeds(1, 2, true), [1], "a single lane feeds the outer ring lane");
        assert_eq!(feeds(2, 3, true), [0, 2]);
        assert_eq!(feeds(2, 2, false), [1, 0], "left-hand traffic: the driver's left is the outside");
        assert_eq!(feeds(3, 3, false), [2, 1, 0]);
    }

    #[test]
    fn the_skew_limits_are_the_placeholders_the_messages_name() {
        assert_eq!((MAX_SKEW_DEG, WARN_SKEW_DEG), (20.0, 10.0), "jn-refusal-turbo-skewed says 20 degrees");
    }
}
