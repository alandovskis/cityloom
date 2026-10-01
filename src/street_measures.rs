//! The lane arrangements of the Transit Priority Atlas toolbox (families A to
//! F) as the street editor models them: recognising which of them a section
//! forms, what each is missing, and arranging a section as one.
//!
//! The definitions are CityLoom's own simplification of the Atlas's names,
//! worded in the comments below. They are not the Atlas's guidance.

use crate::catalogue::{DirectionRule, KINDS, Side, kind_index};
use crate::model::{Segment, Variant};

/// One measure: its Atlas code and name, and whether it is for a freeway.
pub struct Def {
    pub code: &'static str,
    pub name: &'static str,
    /// `Some(true)` only on a freeway, `Some(false)` only off one.
    pub freeway: Option<bool>,
}

pub const DEFS: [Def; 14] = [
    Def { code: "A1", name: "Transit Streets", freeway: Some(false) },
    Def { code: "A2", name: "Transit Ways", freeway: Some(false) },
    Def { code: "A3", name: "Transit and Direct Access Streets", freeway: Some(false) },
    Def { code: "B1", name: "Center-Running Transit Lanes", freeway: Some(false) },
    Def { code: "B2", name: "Center-Running Transit Lanes on Freeway Medians", freeway: Some(true) },
    Def { code: "B3", name: "Static Alternate-Direction Center-Running Transit Lanes", freeway: Some(false) },
    Def { code: "B4", name: "Dynamic Alternate-Direction Center-Running Transit Lanes", freeway: Some(false) },
    Def { code: "C1", name: "Edge-Running Bidirectional Transit Lanes", freeway: Some(false) },
    Def { code: "D1", name: "Offset Transit Lanes", freeway: Some(false) },
    Def { code: "E1", name: "Curb-Adjacent Transit Lanes", freeway: Some(false) },
    Def { code: "E2", name: "Curb-Adjacent Reversible Parking and Transit Lanes", freeway: Some(false) },
    Def { code: "E3", name: "Transit Lanes on Freeway Shoulders", freeway: Some(true) },
    Def { code: "F1", name: "Contraflow Transit Lanes", freeway: Some(false) },
    Def { code: "F2", name: "Offset Contraflow Transit Lanes", freeway: Some(false) },
];

pub struct Found {
    pub code: &'static str,
    pub present: bool,
    pub problems: Vec<String>,
}

fn id(s: &Segment) -> &'static str {
    KINDS[s.kind].id
}

fn is_road(s: &Segment) -> bool {
    crate::junction::is_roadway(s.kind)
}

/// Which direction (0 away, 1 toward) lanes on the left half of the street run.
fn left_half(side: Side) -> usize {
    usize::from(side == Side::Right)
}

/// Recognises the measures a section forms. `bus_min_mm` is the narrowest bus
/// lane the street's standard allows.
///
/// Also:  `raw` is the section with its
/// other-times windows; `now` is the same section as it is at the shown time.
pub fn detect(raw: &[Segment], now: &[Segment], side: Side, freeway: bool, bus_min_mm: i32) -> Vec<Found> {
    let road: Vec<&Segment> = now.iter().filter(|s| is_road(s)).collect();
    let raw_road: Vec<&Segment> = raw.iter().filter(|s| is_road(s)).collect();
    let n = road.len();
    let at = |i: usize| id(road[i]);
    let bus: Vec<usize> = (0..n).filter(|&i| at(i) == "bus").collect();
    let general = (0..n).filter(|&i| at(i) == "travel").count();
    let has = |k: &str| (0..n).any(|i| at(i) == k);
    let dir_of = |i: usize| road[i].direction;
    let away = bus.iter().filter(|&&i| dir_of(i) == Some(0)).count();
    let toward = bus.iter().filter(|&&i| dir_of(i) == Some(1)).count();
    let both_ways = away >= 1 && toward >= 1;
    let contiguous = !bus.is_empty() && bus[bus.len() - 1] - bus[0] + 1 == bus.len();
    let at_edge = |i: usize| i == 0 || i + 1 == n;
    let offset = |i: usize| (i == 1 && matches!(at(0), "parking" | "loading")) || (i + 2 == n && matches!(at(n - 1), "parking" | "loading"));
    // A lane runs with the traffic of its half of the street unless it is two-way.
    let with_flow = |i: usize| dir_of(i).is_none_or(|d| d == if i < n / 2 { left_half(side) } else { 1 - left_half(side) });
    let dirs: Vec<usize> = (0..n).filter(|&i| at(i) == "travel").filter_map(dir_of).collect();
    let one_way = general > 0 && !dirs.is_empty() && dirs.iter().all(|&d| d == dirs[0]);
    let contra = |i: usize| one_way && dir_of(i).is_some_and(|d| d != dirs[0]);
    let centre_pair = both_ways && contiguous && bus.len() >= 2 && bus[0] > 0 && bus[bus.len() - 1] + 1 < n;
    let centre_one = bus.len() == 1 && bus[0] > 0 && bus[0] + 1 < n;
    let no_other = general == 0 && !has("parking");
    // The first or last roadway piece, as it is at any time of day.
    let raw_edge = |first: bool| if first { raw_road.first() } else { raw_road.last() };
    let reversible = |s: Option<&&Segment>, a: &str, b: &str| {
        s.is_some_and(|s| (id(s) == a && s.variants.iter().any(|v| KINDS[v.kind].id == b)) || (id(s) == b && s.variants.iter().any(|v| KINDS[v.kind].id == a)))
    };
    let alternating = centre_one && {
        let s = raw_road.iter().find(|s| id(s) == "bus");
        s.is_some_and(|s| s.direction.is_some() && s.variants.iter().any(|v| KINDS[v.kind].id == "bus" && v.direction.is_some() && v.direction != s.direction))
    };
    let edge_bus = (0..bus.len()).map(|k| bus[k]).filter(|&i| at_edge(i)).collect::<Vec<_>>();

    let a1 = no_other && !has("loading") && both_ways && !freeway;
    let present = |code: &str| match code {
        "A1" => a1,
        "A2" => a1 && !has("bike"),
        "A3" => no_other && has("loading") && both_ways && !freeway,
        "B1" => centre_pair && !freeway,
        "B2" => centre_pair && freeway,
        "B3" => alternating && !freeway,
        "B4" => centre_one && dir_of(bus[0]).is_none() && !freeway,
        "C1" => general > 0 && bus.len() == 2 && both_ways && contiguous && (bus[0] == 0 || bus[1] + 1 == n) && !freeway,
        "D1" => general > 0 && bus.iter().any(|&i| offset(i) && !contra(i)) && !freeway,
        "E1" => general > 0 && edge_bus.iter().any(|&i| !contra(i) && with_flow(i)) && !(bus.len() == 2 && both_ways && contiguous) && !freeway,
        "E2" => reversible(raw_edge(true), "parking", "bus") || reversible(raw_edge(false), "parking", "bus"),
        "E3" => freeway && (!edge_bus.is_empty() || reversible(raw_edge(true), "shoulder", "bus") || reversible(raw_edge(false), "shoulder", "bus")),
        "F1" => bus.iter().any(|&i| at_edge(i) && contra(i)) && !freeway,
        "F2" => bus.iter().any(|&i| offset(i) && contra(i)) && !freeway,
        _ => false,
    };

    // What any transit lane needs, and what centre-running lanes need besides.
    let mut base = Vec::new();
    for &i in &bus {
        if road[i].width_mm < bus_min_mm {
            base.push(format!("A bus lane is {} mm wide, and a transit lane wants {} mm", road[i].width_mm, bus_min_mm));
        }
    }
    let median_beside = now.iter().enumerate().any(|(k, s)| {
        id(s) == "median" && (k > 0 && id(&now[k - 1]) == "bus" || k + 1 < now.len() && id(&now[k + 1]) == "bus")
    });
    DEFS.iter()
        .map(|d| {
            let p = present(d.code);
            let mut problems = Vec::new();
            if p {
                for b in &base {
                    if !problems.contains(b) {
                        problems.push(b.clone());
                    }
                }
                if matches!(d.code, "B1" | "B2") && !median_beside {
                    problems.push("Center-running lanes need a median or platform beside them for stops".into());
                }
            }
            Found { code: d.code, present: p, problems }
        })
        .collect()
}

// ---- arranging a section --------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Dir {
    None,
    /// With the traffic of its half of the street.
    Half,
    Fixed(usize),
}

struct Spec {
    kind: &'static str,
    dir: Dir,
    /// May be left out when the street is too narrow.
    removable: bool,
    /// Other times: (kind, from, to) with the same direction rule as the piece, reversed if `flip`.
    other: Vec<(&'static str, i32, i32, bool)>,
}

fn sp(kind: &'static str, dir: Dir, removable: bool) -> Spec {
    Spec { kind, dir, removable, other: Vec::new() }
}

const PEAKS: [(i32, i32); 2] = [(7 * 60, 10 * 60), (16 * 60, 19 * 60)];

fn recipe(code: &str) -> Option<Vec<Spec>> {
    use Dir::*;
    let (t, tr) = (|| sp("travel", Half, false), || sp("travel", Half, true));
    let bus = || sp("bus", Half, false);
    let median = || sp("median", None, false);
    let shoulder = || sp("shoulder", None, false);
    let v = match code {
        "A1" => vec![sp("bike", None, true), bus(), sp("median", None, true), bus(), sp("bike", None, true)],
        "A2" => vec![sp("planting", None, true), bus(), median(), bus(), sp("planting", None, true)],
        "A3" => vec![sp("loading", None, false), bus(), sp("median", None, true), bus(), sp("loading", None, false)],
        "B1" => vec![tr(), t(), bus(), median(), bus(), t(), tr()],
        "B2" => vec![shoulder(), tr(), t(), bus(), median(), bus(), t(), tr(), shoulder()],
        "B3" => {
            let mut c = sp("bus", Half, false);
            c.other = PEAKS[1..].iter().map(|&(a, b)| ("bus", a, b, true)).collect();
            vec![tr(), t(), c, t(), tr()]
        }
        "B4" => vec![tr(), t(), sp("bus", None, false), t(), tr()],
        "C1" => vec![tr(), t(), sp("median", None, true), t(), tr(), sp("bus", Fixed(1), false), sp("bus", Fixed(0), false)],
        "D1" => vec![sp("parking", None, false), bus(), t(), t(), bus(), sp("parking", None, false)],
        "E1" => vec![bus(), tr(), t(), t(), tr(), bus()],
        "E2" => {
            let mut p = sp("parking", None, false);
            p.other = PEAKS.iter().map(|&(a, b)| ("bus", a, b, false)).collect();
            vec![p, tr(), t(), t(), tr(), {
                let mut q = sp("parking", None, false);
                q.other = PEAKS.iter().map(|&(a, b)| ("bus", a, b, false)).collect();
                q
            }]
        }
        "E3" => vec![bus(), tr(), t(), t(), median(), t(), t(), tr(), bus()],
        "F1" => vec![bus(), sp("travel", Fixed(0), true), sp("travel", Fixed(0), false), sp("travel", Fixed(0), false), sp("travel", Fixed(0), true)],
        "F2" => vec![sp("parking", None, false), bus(), sp("travel", Fixed(0), true), sp("travel", Fixed(0), false), sp("travel", Fixed(0), true), sp("parking", None, true)],
        _ => return Option::None,
    };
    Some(v)
}

/// Arranges a section as measure `code`: the sidewalks and planting at its
/// edges stay, the roadway between them is laid out afresh. None when the
/// measure does not suit this street or will not fit its width.
pub fn arrange(code: &str, existing: &[Segment], row_mm: i32, side: Side, freeway: bool, next_uid: &mut u32) -> Option<Vec<Segment>> {
    let def = DEFS.iter().find(|d| d.code == code)?;
    if def.freeway.is_some_and(|f| f != freeway) {
        return None;
    }
    let mut specs = recipe(code)?;
    if side == Side::Left {
        specs.reverse();
    }
    // Edge zones stay as they are.
    let lead = existing.iter().take_while(|s| !is_road(s)).cloned().collect::<Vec<_>>();
    let trail = existing.iter().rev().take_while(|s| !is_road(s)).cloned().collect::<Vec<_>>();
    let (lead, trail): (Vec<Segment>, Vec<Segment>) = if lead.len() == existing.len() { (lead, Vec::new()) } else { (lead, trail.into_iter().rev().collect()) };
    let edge: i32 = lead.iter().chain(&trail).map(|s| s.width_mm).sum();
    let room = row_mm - edge;

    let kinds = |s: &Spec| kind_index(s.kind).expect("recipes use catalogue kinds");
    // Leave out what can go until the minimums fit, from one end and then the
    // other so the street stays balanced.
    let mut removed = 0;
    while specs.iter().map(|s| KINDS[kinds(s)].min_mm).sum::<i32>() > room {
        let i = if removed % 2 == 0 { specs.iter().rposition(|s| s.removable)? } else { specs.iter().position(|s| s.removable)? };
        specs.remove(i);
        removed += 1;
    }
    let mut w: Vec<i32> = specs.iter().map(|s| KINDS[kinds(s)].default_mm).collect();
    let sum = |w: &[i32]| w.iter().sum::<i32>();
    // Squeeze other traffic before the transit lanes: a transit lane wants its width.
    'squeeze: while sum(&w) > room {
        for pick in ["travel", "parking", "bike", "loading", "shoulder", "planting", "median", "bus"] {
            if let Some(i) = (0..w.len()).filter(|&i| specs[i].kind == pick && w[i] > KINDS[kinds(&specs[i])].min_mm).max_by_key(|&i| w[i]) {
                w[i] -= 100;
                continue 'squeeze;
            }
        }
        return None;
    }
    'grow: while sum(&w) + 100 <= room {
        for pick in ["bus", "travel", "median", "planting", "bike", "shoulder", "loading", "parking"] {
            if let Some(i) = (0..w.len()).filter(|&i| specs[i].kind == pick && w[i] + 100 <= KINDS[kinds(&specs[i])].max_mm).min_by_key(|&i| w[i]) {
                w[i] += 100;
                continue 'grow;
            }
        }
        break;
    }

    // Which half a lane is in is counted among roadway pieces only.
    let on_road = |s: &Spec| !matches!(s.kind, "median" | "planting");
    let count = specs.iter().filter(|s| on_road(s)).count();
    let mut out: Vec<Segment> = lead;
    let mut idx = 0;
    for (s, width) in specs.iter().zip(w) {
        let k = kinds(s);
        let half = if idx < count / 2 { left_half(side) } else { 1 - left_half(side) };
        let dir = |d: Dir| match d {
            Dir::None => None,
            Dir::Half => Some(half),
            Dir::Fixed(x) => Some(x),
        };
        let mut seg = Segment::new(*next_uid, k, width);
        *next_uid += 1;
        match KINDS[k].direction {
            DirectionRule::None => {}
            DirectionRule::Required => seg.direction = Some(dir(s.dir).unwrap_or(0)),
            DirectionRule::Optional => seg.direction = dir(s.dir),
        }
        for &(vk, from, to, flip) in &s.other {
            let vk = kind_index(vk).expect("catalogue kind");
            let vdir = match KINDS[vk].direction {
                DirectionRule::None => None,
                _ => Some(if flip { 1 - seg.direction.unwrap_or(half) } else { half }),
            };
            seg.variants.push(Variant { kind: vk, material: KINDS[vk].materials[0], direction: vdir, from_min: from, to_min: to });
        }
        if on_road(s) {
            idx += 1;
        }
        out.push(seg);
    }
    out.extend(trail);
    Some(out)
}
