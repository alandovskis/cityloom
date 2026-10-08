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
            name: String::new(),
            region: 0,
            states: Vec::new(),
            cursor: 0,
            selected: Target::None,
            gesture: None,
            pending_label: String::new(),
            refusal: None,
        };
        j.load_sample(sample);
        j
    }

    pub fn load_sample(&mut self, sample: usize) {
        let sample = sample.min(JUNCTION_SAMPLES.len() - 1);
        let s = &JUNCTION_SAMPLES[sample];
        self.name = s.name.to_string();
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
        self.states = vec![State { label: "Junction today".into(), arms, control: s.control, ring_extra_mm: 0, bus: None, cycle: None, source: Vec::new() }];
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
