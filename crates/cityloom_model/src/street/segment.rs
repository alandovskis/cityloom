//! A piece of a street's cross-section, and the other types it takes at certain times of day.

use serde::{Deserialize, Serialize};

use crate::catalogue::{DEFAULT_CURB, DirectionRule, KINDS, is_motor};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Segment {
    pub uid: u32,
    pub kind: usize,
    pub width_mm: i32,
    /// Index into `MATERIALS`; always one the kind allows.
    pub material: usize,
    /// Index into `CURBS`. `None` is a flush edge, and is always the value for
    /// a kind without a curb.
    pub curb: Option<usize>,
    /// Index into `DIRECTIONS`. Always `Some` where the kind requires a
    /// direction (a driving lane), and `None` for a kind without one. Where it
    /// is optional (a bike lane) `None` means two-way.
    pub direction: Option<usize>,
    /// Whether a transit lane carries trams rather than buses. Matters only
    /// while the piece is a transit lane, at its base or in a window.
    #[serde(default)]
    pub tram: bool,
    /// Other types this piece takes at certain times of day. Outside every
    /// window it is the type above. Windows never overlap.
    pub variants: Vec<Variant>,
}

/// A different type a piece takes between two times of day.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Variant {
    pub kind: usize,
    /// Index into `MATERIALS`; always one the kind allows.
    pub material: usize,
    /// Which way the piece runs in this window, by the same rule as the base type.
    pub direction: Option<usize>,
    /// Start of the window, in minutes after midnight, a multiple of 15.
    pub from_min: i32,
    /// End of the window (not included). Earlier than `from_min` means the
    /// window runs past midnight.
    pub to_min: i32,
    /// The days of the week it applies on, as OSM writes them (`Mo-Fr`), where the window is not every day.
    /// Only said: the sheet's clock has no day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days: Option<String>,
}

/// Time of day is kept in quarter hours; a window is a set of those.
pub const SLOT_MIN: i32 = 15;
pub const SLOTS: i32 = 24 * 60 / SLOT_MIN;

/// The quarter hours of a window, as bits.
pub fn window_mask(from_min: i32, to_min: i32) -> u128 {
    let (from, to) = (from_min / SLOT_MIN, to_min / SLOT_MIN);
    let range = |a: i32, b: i32| (a..b).fold(0u128, |m, i| m | (1 << i));
    if from < to { range(from, to) } else { range(from, SLOTS) | range(0, to) }
}

/// Whether a window starts and ends on quarter hours of the day, and is not empty.
pub fn valid_window(from_min: i32, to_min: i32) -> bool {
    let ok = |m: i32| (0..24 * 60).contains(&m) && m % SLOT_MIN == 0;
    ok(from_min) && ok(to_min) && from_min != to_min
}

impl Segment {
    /// The variant in force at `time_min`, if any.
    pub fn variant_at(&self, time_min: i32) -> Option<usize> {
        let slot = 1u128 << (time_min.rem_euclid(24 * 60) / SLOT_MIN);
        self.variants.iter().position(|v| window_mask(v.from_min, v.to_min) & slot != 0)
    }

    pub fn kind_at(&self, time_min: i32) -> usize {
        self.variant_at(time_min).map_or(self.kind, |i| self.variants[i].kind)
    }

    /// The piece as it is at `time_min`: its type, surface and direction then,
    /// with no variants of its own.
    pub fn at(&self, time_min: i32) -> Segment {
        let vi = self.variant_at(time_min);
        let kind = vi.map_or(self.kind, |i| self.variants[i].kind);
        let k = &KINDS[kind];
        let dir = vi.map_or(self.direction, |i| self.variants[i].direction);
        Segment {
            uid: self.uid,
            kind,
            width_mm: self.width_mm,
            material: vi.map_or(self.material, |i| self.variants[i].material),
            curb: if k.has_curb { self.curb } else { None },
            direction: match k.direction {
                DirectionRule::None => None,
                DirectionRule::Required => Some(dir.unwrap_or(0)),
                DirectionRule::Optional => dir,
            },
            tram: self.tram && k.id == "bus",
            variants: Vec::new(),
        }
    }

    /// Whether the piece is a motor piece (`is_motor`), as its own kind or in any of its time windows.
    pub fn has_motor(&self) -> bool {
        std::iter::once(self.kind).chain(self.variants.iter().map(|v| v.kind)).any(is_motor)
    }

    /// A piece with its kind's default surface and, where the kind has one,
    /// the default curb.
    pub fn new(uid: u32, kind: usize, width_mm: i32) -> Segment {
        let k = &KINDS[kind];
        Segment {
            uid,
            kind,
            width_mm,
            material: k.materials[0],
            curb: k.has_curb.then_some(DEFAULT_CURB),
            direction: (k.direction == DirectionRule::Required).then_some(0),
            tram: false,
            variants: Vec::new(),
        }
    }
}
