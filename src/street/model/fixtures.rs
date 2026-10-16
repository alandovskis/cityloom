//! The sample streets the tests are written against. They are not part of the app: a street in the
//! app is a road of the network, with the lanes and name OpenStreetMap gives it.

use super::*;
use crate::shared::catalogue::kind_index;

pub struct Sample {
    pub name: &'static str,
    pub row_mm: i32,
    pub class: StreetClass,
    pub segments: &'static [(&'static str, i32)],
}

pub const SAMPLES: [Sample; 4] = [
    Sample {
        name: "Sample Street 1",
        row_mm: 18000,
        class: StreetClass::Local,
        segments: &[("sidewalk", 3300), ("parking", 2400), ("travel", 3300), ("travel", 3300), ("parking", 2400), ("sidewalk", 3300)],
    },
    Sample {
        name: "Sample Avenue 2",
        row_mm: 30000,
        class: StreetClass::Arterial,
        segments: &[
            ("sidewalk", 3500),
            ("planting", 1500),
            ("parking", 2400),
            ("travel", 3300),
            ("travel", 3300),
            ("median", 2000),
            ("travel", 3300),
            ("travel", 3300),
            ("parking", 2400),
            ("planting", 1500),
            ("sidewalk", 3500),
        ],
    },
    Sample {
        name: "Sample Lane 3",
        row_mm: 12000,
        class: StreetClass::Local,
        segments: &[("sidewalk", 2100), ("travel", 3000), ("travel", 3000), ("parking", 2400), ("sidewalk", 1500)],
    },
    Sample {
        name: "Sample Freeway 4",
        row_mm: 30600,
        class: StreetClass::Motorway,
        segments: &[
            ("shoulder", 3000),
            ("travel", 3600),
            ("travel", 3600),
            ("travel", 3600),
            ("median", 3000),
            ("travel", 3600),
            ("travel", 3600),
            ("travel", 3600),
            ("shoulder", 3000),
        ],
    },
];

/// A copy of `street` whose every motor piece (as its kind or in any time window) is a sidewalk of the same
/// width, settled: the street pedestrianised.
pub fn pedestrianised(street: &Street) -> Street {
    let sidewalk = kind_index("sidewalk").expect("the catalogue has sidewalks");
    let mut s = street.clone();
    for g in s.segments.iter_mut().filter(|g| g.has_motor()) {
        *g = Segment { variants: Vec::new(), direction: None, ..Segment::new(g.uid, sidewalk, g.width_mm) };
    }
    s.settled()
}

impl Street {
    /// The sample street laid out for a side of the road.
    pub fn sample(sample: usize, side: Side) -> Street {
        let sample = sample.min(SAMPLES.len() - 1);
        let mut e = Editor::new(sample);
        let region = REGIONS.iter().position(|r| r.drive_side == side).unwrap_or(0);
        e.set_region(region);
        e.snapshot()
    }
}

impl Editor {
    /// A street laid out on one of the sample profiles.
    pub fn new(sample: usize) -> Editor {
        let mut e = Editor {
            street_name: None,
            source: Vec::new(),
            region: 0,
            time_min: 12 * 60,
            states: Vec::new(),
            cursor: 0,
            next_uid: 1,
            selected: None,
            gesture: None,
            pending_label: None,
        };
        e.load_sample(sample);
        e
    }

    pub fn load_sample(&mut self, sample: usize) {
        let sample = sample.min(SAMPLES.len() - 1);
        let s = &SAMPLES[sample];
        self.street_name = Some(s.name.to_string());
        self.next_uid = 1;
        let segments = s
            .segments
            .iter()
            .map(|(id, w)| {
                let uid = self.next_uid;
                self.next_uid += 1;
                Segment::new(uid, kind_index(id).expect("sample uses catalogue kinds"), *w)
            })
            .collect::<Vec<Segment>>();
        let mut segments = segments;
        // Driving lanes on the left half of the street run away, the rest toward.
        default_directions(&mut segments, REGIONS[self.region].drive_side);
        self.states = vec![State { label: Said::new("street-today"), segments, class: s.class, class_before: None, row_mm: s.row_mm }];
        self.cursor = 0;
        self.selected = None;
        self.gesture = None;
    }
}
