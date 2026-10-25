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

/// `street` divided into zones starting at `starts_mm` (millimetres from its first end), every zone on the base
/// section, for a street `length_mm` long, settled.
pub fn zoned(street: Street, starts_mm: &[i32], length_mm: i32) -> Street {
    let zones = starts_mm.iter().map(|&from_mm| Zone { from_mm, segments: None, name: None }).collect();
    Street { zones, from_end: ZoneEnd::First, length_mm: Some(length_mm), ..street }.settled()
}

/// A 100 000 mm street of `Street::sample(0, Side::Right)` with zones at 0 and 50 000. Zone 2 has a section of its
/// own, one travel lane fewer, when `second_own`.
pub fn two_zones(second_own: bool) -> Editor {
    let mut s = zoned(Street::sample(0, Side::Right), &[0, 50_000], 100_000);
    if second_own {
        let mut own = s.segments.clone();
        let lane = own.iter().position(|g| KINDS[g.kind].id == "travel").expect("the sample has a travel lane");
        own.remove(lane);
        s.zones[1].segments = Some(own);
    }
    let region = REGIONS.iter().position(|r| r.drive_side == Side::Right).unwrap_or(0);
    Editor::from_street(&s, &s, region)
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
            zone: 0,
            frame: None,
            ends: [EndInfo::default(); 2],
            selected: None,
            gesture: None,
            curb_sel: None,
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
        self.states = vec![State {
            label: Said::new("street-today"),
            content: Content { segments, zones: Vec::new(), from_end: ZoneEnd::First, length_mm: None, speed: Speed::Undecided, curb: Vec::new() },
            class: s.class,
            class_before: None,
            row_mm: s.row_mm,
        }];
        self.cursor = 0;
        self.selected = None;
        self.gesture = None;
    }
}
