//! The sample junctions the tests are written against. They are not part of the app: a junction
//! in the app is a place in a city, with the streets and name the network gives it.

use super::*;

pub struct SampleArm {
    pub street: usize,
    pub bearing: i32,
    pub offset_mm: i32,
    pub corner_mm: i32,
    /// Starts with a refuge island in its crossing.
    pub island: bool,
}

const fn sa(street: usize, bearing: i32, offset_mm: i32, corner_mm: i32, island: bool) -> SampleArm {
    SampleArm { street, bearing, offset_mm, corner_mm, island }
}

pub struct JunctionSample {
    pub name: &'static str,
    pub control: usize,
    pub arms: &'static [SampleArm],
}

pub const JUNCTION_SAMPLES: [JunctionSample; 4] = [
    JunctionSample {
        name: "Avenue and street",
        control: SIGNAL,
        arms: &[sa(1, 0, 0, 6_000, true), sa(0, 90, 0, 6_000, false), sa(1, 180, 0, 6_000, true), sa(0, 270, 0, 6_000, false)],
    },
    JunctionSample {
        name: "Street and lane",
        control: PRIORITY,
        arms: &[sa(0, 90, 0, 3_000, false), sa(2, 180, 0, 3_000, false), sa(0, 270, 0, 3_000, false)],
    },
    JunctionSample {
        name: "Offset crossing",
        control: PRIORITY,
        arms: &[sa(2, 0, 2_500, 3_000, false), sa(0, 90, 0, 3_000, false), sa(2, 180, 2_500, 3_000, false), sa(0, 270, 0, 3_000, false)],
    },
    JunctionSample {
        name: "Five ways",
        control: ALL_WAY_STOP,
        arms: &[sa(0, 0, 0, 4_000, false), sa(2, 70, 0, 3_000, false), sa(1, 145, 0, 4_000, true), sa(2, 215, 0, 3_000, false), sa(0, 290, 0, 4_000, false)],
    },
];

impl Junction {
    /// The sandbox junction the tests are written against, on one of the samples. It is not part of the app.
    pub fn new(sample: usize) -> Junction {
        let mut j = Junction {
            linked: false,
            name: crate::street::model::named_said(""),
            region: 0,
            states: Vec::new(),
            cursor: 0,
            selected: Target::None,
            gesture: None,
            pending_label: earlier(),
            refusal: None,
            refusal_arm: None,
        };
        j.load_sample(sample);
        j
    }

    pub fn load_sample(&mut self, sample: usize) {
        let sample = sample.min(JUNCTION_SAMPLES.len() - 1);
        let s = &JUNCTION_SAMPLES[sample];
        self.name = crate::street::model::named_said(s.name);
        let mut arms: Vec<Arm> = s
            .arms
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let mut arm = self.fresh_arm(i as u32 + 1, a.street, a.bearing, a.offset_mm);
                arm.corner_mm = a.corner_mm;
                if let Some(c) = arm.crossing.as_mut() {
                    c.island = a.island;
                }
                arm
            })
            .collect();
        arms.sort_by_key(|a| a.bearing);
        normalize(&mut arms, self.region);
        self.states = vec![State {
            label: today(),
            arms,
            control: s.control,
            ring_extra_mm: 0,
            bus: None,
            cycle: None,
            raised: false,
            turbo: None,
            signal_plan: None,
            source: Vec::new(),
        }];
        self.cursor = 0;
        self.selected = Target::None;
        self.gesture = None;
    }

    /// An arm of a sample junction: it reads the sample street `sample`, laid out for the right-hand side.
    fn fresh_arm(&self, uid: u32, sample: usize, bearing: i32, offset_mm: i32) -> Arm {
        let mut arm = Arm::new(uid, bearing, offset_mm);
        arm.section = Some(Street::sample(sample, Side::Right));
        arm
    }
}

/// A four-way, in a city, whose every arm reads a street of these pieces, left to right: (kind id, width in mm).
pub fn junction_of(pieces: &[(&str, i32)]) -> Junction {
    use crate::shared::catalogue::{Side, StreetClass, kind_index};
    use crate::street::model::Piece;
    let pieces: Vec<Piece> = pieces
        .iter()
        .map(|&(id, width_mm)| Piece { kind: kind_index(id).unwrap(), width_mm, direction: (id == "travel").then_some(1), variants: Vec::new() })
        .collect();
    let street = Street::imported(StreetClass::Local, Side::Right, &pieces);
    let mut state = Junction::new(0).current().clone();
    for a in &mut state.arms {
        a.section = Some(street.clone());
        a.edge = a.uid;
    }
    Junction::from_city(named_said("Test"), &state, &state, 0).expect("the street draws")
}

/// A four-way, in a city, whose every arm reads a street of a sidewalk, `leave` travel lanes out, `enter` travel lanes in
/// and a sidewalk, each lane 3.5 m (`junction_of` makes every travel lane come in, so a four-way of it has no movement).
/// `region` is an index into `REGIONS`; a left-hand one gets left-hand pieces, so a turbo can stand on it.
pub fn junction_two_way(enter: usize, leave: usize, region: usize) -> Junction {
    use crate::shared::catalogue::{REGIONS, StreetClass, kind_index};
    use crate::street::model::Piece;
    let piece = |id: &str, width_mm, direction| Piece { kind: kind_index(id).unwrap(), width_mm, direction, variants: Vec::new() };
    let mut pieces = vec![piece("sidewalk", 3_000, None)];
    pieces.extend((0..leave).map(|_| piece("travel", 3_500, Some(0))));
    pieces.extend((0..enter).map(|_| piece("travel", 3_500, Some(1))));
    pieces.push(piece("sidewalk", 3_000, None));
    let street = Street::imported(StreetClass::Local, REGIONS[region].drive_side, &pieces);
    let mut state = Junction::new(0).current().clone();
    for a in &mut state.arms {
        a.section = Some(street.clone());
        a.edge = a.uid;
    }
    Junction::from_city(named_said("Test"), &state, &state, region).expect("the street draws")
}

/// A four-way, in a city, whose arms facing `bearings` (0, 90, 180 or 270) read a pedestrian street of these
/// pieces, left to right: (kind id, width in mm). The other arms read the first sample street.
pub fn junction_with_walkways(bearings: &[i32], pieces: &[(&str, i32)]) -> Junction {
    use crate::shared::catalogue::{Side, StreetClass, kind_index};
    use crate::street::model::Piece;
    let pieces: Vec<Piece> =
        pieces.iter().map(|&(id, width_mm)| Piece { kind: kind_index(id).unwrap(), width_mm, direction: None, variants: Vec::new() }).collect();
    let walkway = Street::imported(StreetClass::Pedestrian, Side::Right, &pieces);
    let street = Street::sample(0, Side::Right);
    let mut state = Junction::new(0).current().clone();
    for a in &mut state.arms {
        a.section = Some(if bearings.contains(&a.bearing) { walkway.clone() } else { street.clone() });
        a.edge = a.uid;
    }
    Junction::from_city(named_said("Test"), &state, &state, 0).expect("the junction draws")
}

/// A basic turbo roundabout, in a city, whose east, south and west streets are walkways: one street carries traffic, so
/// the control is suspended and the turbo with it.
pub fn suspended_turbo() -> Junction {
    use crate::shared::catalogue::{Side, StreetClass};
    let mut j = Junction::new(0);
    assert!(j.set_control(ROUNDABOUT) && j.set_turbo(Some(0)));
    let mut state = j.snapshot();
    let walkway = Street::imported(StreetClass::Pedestrian, Side::Right, &[]);
    for a in &mut state.arms {
        a.section = Some(if a.bearing == 0 { Street::sample(0, Side::Right) } else { walkway.clone() });
        a.edge = a.uid;
    }
    Junction::from_city(named_said("Test"), &state, &state, 0).expect("it draws")
}
