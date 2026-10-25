//! The cross-section editing model. Pure Rust, no browser types, so it is
//! tested natively. All lengths are integer millimetres.

use serde::{Deserialize, Serialize};

use crate::shared::catalogue::{
    CURBS, DEFAULT_CURB, DIRECTIONS, DirectionRule, KINDS, MATERIALS, Mode, PEDESTRIAN_KINDS, REGIONS, Side, StreetClass, class_key, is_motor, kind_index,
};
use crate::shared::provenance::OsmRef;
use crate::street::measures;

pub use crate::shared::said::{Arg, Said};

#[cfg(test)]
mod fixtures;
#[cfg(test)]
pub use fixtures::{SAMPLES, pedestrianised, two_zones, zoned};
pub use joint::{JOINT_REACH_MM, Joint, SHOWN_MM, Taper};
mod joint;
mod speed;
pub use speed::Speed;
mod transition;
pub use transition::{Change, Host, Transition, transition};
mod curb;
mod curb_shown;
mod stop;
use curb::Choice;
pub use curb::{CurbCell, CurbRefusal, CurbSideView, CurbView, EndInfo, Instance, SideView, StripCell, StripView, Subject, Use, curbside};
pub use curb_shown::{ShownInstance, ShownStop};
pub use stop::{MIN_STOP_MM, Stop, StopCell, StopKind, StopSideView, StopsView};
mod zones;
use zones::Failing;
pub use zones::{Elsewhere, Keep, LENGTH_TOLERANCE_MM, MIN_ZONE_MM, Zone, ZoneEnd, ZoneFrame, ZoneRefusal, ZoneView, ZonesView, zone_label, zone_label_mid};

/// Widths snap to this step when dragged.
pub const SNAP_MM: i32 = 100;
/// Sidewalk on each edge is a sheet check, not a hard rule.
const ACCESS_LANE_MM: i32 = 3000;

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
const SLOT_MIN: i32 = 15;
const SLOTS: i32 = 24 * 60 / SLOT_MIN;

fn window_mask(from_min: i32, to_min: i32) -> u128 {
    let (from, to) = (from_min / SLOT_MIN, to_min / SLOT_MIN);
    let range = |a: i32, b: i32| (a..b).fold(0u128, |m, i| m | (1 << i));
    if from < to { range(from, to) } else { range(from, SLOTS) | range(0, to) }
}

fn valid_window(from_min: i32, to_min: i32) -> bool {
    let ok = |m: i32| (0..24 * 60).contains(&m) && m % SLOT_MIN == 0;
    ok(from_min) && ok(to_min) && from_min != to_min
}

fn kind_lower(kind: usize) -> Arg {
    Arg::KindLower(KINDS[kind].id)
}

/// Which side of the road traffic keeps to, as a sentence: the label of a change and of a check.
fn side_said(side: Side) -> Said {
    match side {
        Side::Right => Said::new("side-keeps-right"),
        Side::Left => Said::new("side-keeps-left"),
    }
}

/// A piece of `base` kind taking `kind` between two times of day.
fn window_said(base: usize, kind: usize, from_min: i32, to_min: i32) -> Said {
    Said::new("rev-window")
        .with("base", Arg::Kind(KINDS[base].id))
        .with("kind", kind_lower(kind))
        .with("from", Arg::Clock(from_min))
        .with("to", Arg::Clock(to_min))
}

impl Segment {
    /// The variant in force at `time_min`, if any.
    fn variant_at(&self, time_min: i32) -> Option<usize> {
        let slot = 1u128 << (time_min.rem_euclid(24 * 60) / SLOT_MIN);
        self.variants.iter().position(|v| window_mask(v.from_min, v.to_min) & slot != 0)
    }

    fn kind_at(&self, time_min: i32) -> usize {
        self.variant_at(time_min).map_or(self.kind, |i| self.variants[i].kind)
    }

    /// The range of widths every type of this piece allows.
    fn bounds(&self) -> (i32, i32) {
        self.bounds_except(None)
    }

    /// Like `bounds`, leaving out the variant at `skip`.
    fn bounds_except(&self, skip: Option<usize>) -> (i32, i32) {
        let variants = self.variants.iter().enumerate().filter(|(i, _)| Some(*i) != skip).map(|(_, v)| v.kind);
        std::iter::once(self.kind).chain(variants).fold((0, i32::MAX), |(lo, hi), k| (lo.max(KINDS[k].min_mm), hi.min(KINDS[k].max_mm)))
    }

    /// The piece as it is at `time_min`: its type, surface and direction then,
    /// with no variants of its own.
    fn at(&self, time_min: i32) -> Segment {
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

    /// Kinds this piece may take at other times: other roadway types that
    /// share some width with its own types. Empty for a piece that is not
    /// roadway. Taking one may move the width into the shared range. A bike
    /// lane is not time limited: it takes no other type, and no piece takes
    /// the bike lane for some hours.
    fn alt_kinds(&self) -> Vec<usize> {
        self.alt_kinds_except(None)
    }

    fn alt_kinds_except(&self, skip: Option<usize>) -> Vec<usize> {
        if !KINDS[self.kind].shares_road || KINDS[self.kind].id == "bike" {
            return Vec::new();
        }
        let (lo, hi) = self.bounds_except(skip);
        (0..KINDS.len())
            .filter(|&k| k != self.kind && KINDS[k].shares_road && KINDS[k].id != "bike" && lo.max(KINDS[k].min_mm) <= hi.min(KINDS[k].max_mm))
            .collect()
    }
}

impl Segment {
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

/// One street as the city keeps it: the pieces of its section as the street
/// editor shows them, looking from the street's first end toward its second.
/// Directions are written for the side of the road in `side`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Street {
    /// What sort of street it is.
    pub class: StreetClass,
    /// The class the street had before it became pedestrian: giving it a lane again restores it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_before: Option<StreetClass>,
    /// What the street is called.
    #[serde(default)]
    pub name: Option<String>,
    pub row_mm: i32,
    pub side: Side,
    pub segments: Vec<Segment>,
    pub next_uid: u32,
    /// The OSM ways it was made from. The sample streets are made from none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source: Vec<OsmRef>,
    /// The street's zones along its length, in order of `from_mm`. Empty for a street of one section, the base.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<Zone>,
    /// The end the zones' positions are measured from: the end that is a real OSM node. Written only with zones.
    #[serde(default, skip_serializing_if = "ZoneEnd::is_first")]
    pub from_end: ZoneEnd,
    /// The length along the centreline the zones were drawn against. Written only with zones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_mm: Option<i32>,
    /// The speed limit, kept as `speed_kmh`: absent when not yet decided, `null` when the road has none.
    #[serde(default, rename = "speed_kmh", skip_serializing_if = "Speed::is_undecided")]
    pub speed: Speed,
    /// The uses of the curbside along the street, left side first, then by `from_mm`. Empty for a street whose curbside
    /// is whatever its sections hold; with any, `from_end` and `length_mm` are written as they are with zones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub curb: Vec<Instance>,
    /// The transit stops along the street, left side first, then by `from_mm`. Empty for a street with none; with
    /// any, `from_end` and `length_mm` are written as they are with zones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stops: Vec<Stop>,
}

/// A name that is data, said as it is: a street's, a place's, a junction's.
pub fn named_said(name: &str) -> Said {
    Said::new("city-name").with("name", Arg::Text(name.to_string()))
}

/// What a street is called: its own name (data, said as it is), or else what sort of street it is.
/// The class a street has, and the one it had before it became pedestrian, given whether it has a motor piece.
fn settled_class(motor: bool, class: StreetClass, before: Option<StreetClass>) -> (StreetClass, Option<StreetClass>) {
    match (motor, class) {
        (false, class) if class != StreetClass::Pedestrian => (StreetClass::Pedestrian, Some(class)),
        (true, StreetClass::Pedestrian) => (before.unwrap_or(StreetClass::Local), None),
        _ => (class, before),
    }
}

fn title_of(name: Option<&str>, class: StreetClass) -> Said {
    match (name, class) {
        (Some(name), _) => named_said(name),
        (None, class) => unnamed_said(class, false),
    }
}

/// A road with no name, said as what sort of road it is: as a title ("Unnamed local street") or, with
/// `in_sentence`, as a phrase ("an unnamed local street"). A motorway has a message of its own.
pub fn unnamed_said(class: StreetClass, in_sentence: bool) -> Said {
    match (class, in_sentence) {
        (StreetClass::Motorway, false) => Said::new("city-unnamed-motorway"),
        (StreetClass::Motorway, true) => Said::new("city-unnamed-motorway-in-sentence"),
        (class, false) => Said::new("city-unnamed").with("class", Arg::Msg(class_key(class))),
        (class, true) => Said::new("city-unnamed-in-sentence").with("class", Arg::Msg(class_key(class))),
    }
}

fn flip(d: &mut Option<usize>) {
    if let Some(d) = d {
        *d = 1 - *d;
    }
}

/// One piece of a street, as read from a road of the network.
#[derive(Clone, Debug)]
pub struct Piece {
    pub kind: usize,
    pub width_mm: i32,
    /// Index into `DIRECTIONS`, for a kind that has one.
    pub direction: Option<usize>,
    /// The times of day it is another kind; they must not overlap.
    pub variants: Vec<Window>,
}

/// A time of day, in minutes after midnight and on quarter hours as `Variant`'s are, that a piece is of another kind.
#[derive(Clone, Debug)]
pub struct Window {
    pub kind: usize,
    pub from_min: i32,
    pub to_min: i32,
    /// The days of the week, as `Variant::days`.
    pub days: Option<String>,
}

impl Street {
    /// A street made of the given pieces, left to right as seen from its first
    /// end, with each kind's default surface. `class` says what sort of street
    /// it is (a motorway has its own rules); the row is the pieces' widths,
    /// each brought inside what its kind allows.
    pub fn imported(class: StreetClass, side: Side, pieces: &[Piece]) -> Street {
        let segments: Vec<Segment> = pieces
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let k = &KINDS[p.kind];
                let mut g = Segment::new(i as u32 + 1, p.kind, p.width_mm.clamp(k.min_mm, k.max_mm));
                g.direction = direction_for(p.kind, p.direction);
                g.variants = p
                    .variants
                    .iter()
                    .map(|w| Variant {
                        kind: w.kind,
                        material: KINDS[w.kind].materials[0],
                        direction: direction_for(w.kind, g.direction),
                        from_min: w.from_min,
                        to_min: w.to_min,
                        days: w.days.clone(),
                    })
                    .collect();
                let (lo, hi) = g.bounds();
                g.width_mm = p.width_mm.clamp(lo, hi);
                g
            })
            .collect();
        let mut street = Street {
            class,
            class_before: None,
            name: None,
            row_mm: total(&segments),
            side,
            segments,
            next_uid: pieces.len() as u32 + 1,
            source: Vec::new(),
            zones: Vec::new(),
            from_end: ZoneEnd::First,
            length_mm: None,
            speed: Speed::Undecided,
            curb: Vec::new(),
            stops: Vec::new(),
        };
        street.settle_class();
        street
    }

    /// Whether any piece of any section is a motor piece.
    pub fn has_motor(&self) -> bool {
        self.sections().iter().any(|s| s.iter().any(Segment::has_motor))
    }

    /// Keeps the class true to the pieces. No motor piece: pedestrian, remembering the class it had. A motor
    /// piece (travel, parking, loading, bus, shoulder) on a pedestrian street: the class it had, or else local. Run after every edit and when a street is built.
    pub fn settle_class(&mut self) {
        (self.class, self.class_before) = settled_class(self.has_motor(), self.class, self.class_before);
    }

    /// The same street, settled.
    pub fn settled(mut self) -> Street {
        self.settle_zones();
        self.settle_class();
        self
    }

    /// The same street, called `name`.
    pub fn named(mut self, name: &str) -> Street {
        self.name = Some(name.to_string());
        self
    }

    /// What it is called: its own name, or else what sort of street it is.
    pub fn title(&self) -> Said {
        title_of(self.name.as_deref(), self.class)
    }

    /// The same street with its lanes written for `side`: running the other
    /// way when the side differs.
    pub fn for_side(&self, side: Side) -> Street {
        let mut s = self.clone();
        if s.side != side {
            s.side = side;
            for seg in &mut s.segments {
                flip(&mut seg.direction);
                for v in &mut seg.variants {
                    flip(&mut v.direction);
                }
            }
            s.flip_zone_lanes();
        }
        s
    }

    /// The street as seen from its other end: the pieces in the opposite order
    /// and every lane running the other way.
    pub fn reversed(&self) -> Street {
        let mut s = self.clone();
        s.segments.reverse();
        for seg in &mut s.segments {
            flip(&mut seg.direction);
            for v in &mut seg.variants {
                flip(&mut v.direction);
            }
        }
        s.reverse_zones();
        s.reverse_curb();
        s.reverse_stops();
        s
    }

    /// Whether every index in it points into the catalogue, so that it can be
    /// drawn. A street that came from storage is checked before it is used.
    pub fn is_sound(&self) -> bool {
        self.row_mm > 0 && self.section_is_sound(&self.segments) && self.zones_are_sound() && self.curb_is_sound() && self.stops_are_sound()
    }

    /// Whether every piece of a section points into the catalogue and has a uid the street has issued.
    fn section_is_sound(&self, section: &[Segment]) -> bool {
        let seg_ok = |kind: usize, material: usize, curb: Option<usize>, direction: Option<usize>| {
            kind < KINDS.len()
                && KINDS[kind].materials.contains(&material)
                && material < MATERIALS.len()
                && curb.is_none_or(|c| c < CURBS.len() && KINDS[kind].has_curb)
                && direction.is_none_or(|d| d < DIRECTIONS.len())
        };
        !section.is_empty()
            && section.iter().all(|s| {
                // the variants are checked before the tram rule reads their kinds
                seg_ok(s.kind, s.material, s.curb, s.direction)
                    && s.variants.iter().all(|v| seg_ok(v.kind, v.material, None, v.direction) && valid_window(v.from_min, v.to_min))
                    && (!s.tram || KINDS[s.kind].id == "bus" || s.variants.iter().any(|v| KINDS[v.kind].id == "bus"))
                    && s.width_mm > 0
            })
            && section.iter().map(|s| s.uid).all(|u| u < self.next_uid)
    }

    /// The zones start at 0, rise strictly and end inside the length they were drawn against; each own section is sound.
    fn zones_are_sound(&self) -> bool {
        let Some(first) = self.zones.first() else { return true };
        let Some(length) = self.length_mm else { return false };
        first.from_mm == 0
            && self.zones.windows(2).all(|w| w[0].from_mm < w[1].from_mm)
            && self.zones.last().is_some_and(|z| z.from_mm < length)
            && self.zones.iter().filter_map(|z| z.segments.as_deref()).all(|s| self.section_is_sound(s))
    }
}

/// What the history keeps of the street's layout: the base section and the zoning.
///
/// Uids are unique within a section, not across zones: a zone given a section of its own gets a copy that keeps
/// its pieces' uids, because renumbering on that copy would invalidate the uids a pointer drag holds mid-gesture.
/// `Editor::next_uid` is still the street's one counter, so a new piece's uid is unique everywhere.
#[derive(Clone, Debug, PartialEq)]
struct Content {
    segments: Vec<Segment>,
    zones: Vec<Zone>,
    from_end: ZoneEnd,
    length_mm: Option<i32>,
    /// The speed limit; a street's section edits do not change it, but undo and Start over restore it.
    speed: Speed,
    /// The uses of the curbside, which lie along the street whatever its zones.
    curb: Vec<Instance>,
    /// The transit stops, which lie along the street whatever its zones.
    stops: Vec<Stop>,
}

impl Content {
    fn of(street: &Street) -> Content {
        Content {
            segments: street.segments.clone(),
            zones: street.zones.clone(),
            from_end: street.from_end,
            length_mm: street.length_mm,
            speed: street.speed,
            curb: street.curb.clone(),
            stops: street.stops.clone(),
        }
    }

    /// How many zones there are to choose from: a street with none is one section.
    fn zone_count(&self) -> usize {
        self.zones.len().max(1)
    }

    /// The section of a zone: its own, or else the base.
    fn section(&self, zone: usize) -> &[Segment] {
        match self.zones.get(zone) {
            Some(z) => z.segments.as_deref().unwrap_or(&self.segments),
            None => &self.segments,
        }
    }

    /// The section of a zone, to change. A zone on the base is given a copy of its own, with the same uids.
    fn section_mut(&mut self, zone: usize) -> &mut Vec<Segment> {
        match self.zones.get_mut(zone) {
            Some(z) => z.segments.get_or_insert_with(|| self.segments.clone()),
            None => &mut self.segments,
        }
    }

    /// Whether any section in use (a zone's own, or the base where a zone has none) has a motor piece.
    fn has_motor(&self) -> bool {
        (0..self.zone_count()).any(|z| self.section(z).iter().any(Segment::has_motor))
    }

    fn settle(&mut self) {
        self.sort_curb();
        self.sort_stops();
        let keep_anchor = !self.curb.is_empty() || !self.stops.is_empty();
        zones::settle_zones(&mut self.segments, &mut self.zones, &mut self.from_end, &mut self.length_mm, keep_anchor);
    }
}

#[derive(Clone, Debug)]
struct State {
    label: Said,
    content: Content,
    /// The street's class as of this state, and the one it had before it became pedestrian: the history holds
    /// them so that undo restores both.
    class: StreetClass,
    class_before: Option<StreetClass>,
    /// The room the street has as of this state. An edit never changes it; it differs between states only for a street
    /// taken by the width it had under an earlier importer, which Start over returns to the width it has now.
    row_mm: i32,
    /// The move of a use or of a stop that made this state (its side, where it started and where it ended up), so
    /// that undo and redo carry the chosen one with it.
    curb_move: Option<(Side, i32, i32)>,
    stop_move: Option<(Side, i32, i32)>,
}

/// The speed limits an editor sets, in kilometres an hour.
pub const SPEED_CHOICES: [u16; 12] = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];

pub struct Editor {
    /// What the street is called, where it has a name of its own.
    street_name: Option<String>,
    /// The OSM ways it was made from.
    source: Vec<OsmRef>,
    /// Index into `REGIONS`. A setting of the sheet, not part of the history.
    region: usize,
    /// The time of day the sheet shows, in minutes. Also a setting, not history.
    time_min: i32,
    states: Vec<State>,
    cursor: usize,
    next_uid: u32,
    /// The zone the edits act on. A setting, not part of the history; 0 for a street with no zones.
    zone: usize,
    /// Where the street lies as loaded (its length and its real end), once the page knows it.
    frame: Option<ZoneFrame>,
    /// What the city says of the street's two ends, node `a` then node `b`. A setting, not part of the history.
    ends: [EndInfo; 2],
    selected: Option<u32>,
    gesture: Option<Content>,
    /// The cell of a curbside strip the person chose, and the move of a use that the next revision records (see
    /// `State.curb_move`). The choice is a setting, not history.
    curb_choice: Choice,
    /// The same for the stops' strips (see `State.stop_move`).
    stop_choice: Choice,
    pending_label: Option<Said>,
}

/// The direction a piece takes when it becomes `kind`, keeping the one it has.
fn direction_for(kind: usize, keep: Option<usize>) -> Option<usize> {
    match KINDS[kind].direction {
        DirectionRule::None => None,
        DirectionRule::Required => Some(keep.unwrap_or(0)),
        DirectionRule::Optional => keep,
    }
}

fn required_direction(s: &Segment) -> bool {
    KINDS[s.kind].direction == DirectionRule::Required
}

/// Gives the driving lanes the directions a street in this region starts with:
/// with traffic on the right, lanes on the left half of the street come toward
/// the viewer and lanes on the right half go away; on the left it is the
/// other way round.
fn default_directions(segs: &mut [Segment], side: Side) {
    let lanes = segs.iter().filter(|s| required_direction(s)).count();
    for (i, s) in segs.iter_mut().filter(|s| required_direction(s)).enumerate() {
        let right_half = i * 2 >= lanes;
        s.direction = Some(usize::from(right_half == (side == Side::Left)));
    }
}

pub(crate) fn snap(v: i32) -> i32 {
    ((v as f64 / SNAP_MM as f64).round() as i32) * SNAP_MM
}

fn total(segments: &[Segment]) -> i32 {
    segments.iter().map(|s| s.width_mm).sum()
}

impl Editor {
    fn class(&self) -> StreetClass {
        self.states[self.cursor].class
    }

    fn class_before(&self) -> Option<StreetClass> {
        self.states[self.cursor].class_before
    }

    /// The street as it is now, for the city to keep.
    pub fn snapshot(&self) -> Street {
        let content = &self.states[self.cursor].content;
        Street {
            class: self.class(),
            class_before: self.class_before(),
            name: self.street_name.clone(),
            row_mm: self.row_mm(),
            side: REGIONS[self.region].drive_side,
            segments: content.segments.clone(),
            next_uid: self.next_uid,
            source: self.source.clone(),
            zones: content.zones.clone(),
            from_end: content.from_end,
            length_mm: content.length_mm,
            speed: content.speed,
            curb: content.curb.clone(),
            stops: content.stops.clone(),
        }
    }

    /// An editor on a street the city holds. `today` is the street as it was
    /// first laid out, which Start over returns to; `now` is how it stands.
    /// Edits made earlier appear as one earlier revision.
    pub fn from_street(today: &Street, now: &Street, region: usize) -> Editor {
        let region = region.min(REGIONS.len() - 1);
        let side = REGIONS[region].drive_side;
        let (today, now) = (today.for_side(side).settled(), now.for_side(side).settled());
        let mut e = Editor {
            street_name: now.name.clone(),
            source: now.source.clone(),
            region,
            time_min: 12 * 60,
            states: vec![State {
                label: Said::new("street-today"),
                content: Content::of(&today),
                class: today.class,
                class_before: today.class_before,
                row_mm: today.row_mm,
                curb_move: None,
                stop_move: None,
            }],
            cursor: 0,
            next_uid: now.next_uid.max(1),
            zone: 0,
            frame: None,
            ends: [EndInfo::default(); 2],
            selected: None,
            gesture: None,
            curb_choice: Choice::default(),
            stop_choice: Choice::default(),
            pending_label: None,
        };
        if Content::of(&now) != e.states[0].content || now.row_mm != e.states[0].row_mm {
            e.push(Said::new("rev-earlier"), Content::of(&now));
            e.states[1].row_mm = now.row_mm;
        }
        e
    }

    fn row_mm(&self) -> i32 {
        self.states[self.cursor].row_mm
    }

    /// The selected zone's section.
    fn current(&self) -> &[Segment] {
        self.states[self.cursor].content.section(self.zone)
    }

    /// The zone the edits act on.
    pub fn zone(&self) -> usize {
        self.zone
    }

    /// Chooses the zone the edits act on. Refused when there is no such zone, or it lies beyond the loaded end. The selected piece is cleared when
    /// the zone changes.
    pub fn select_zone(&mut self, zone: usize) -> bool {
        if zone >= self.states[self.cursor].content.zone_count() || self.is_cut_off(zone) {
            return false;
        }
        if zone != self.zone {
            self.zone = zone;
            self.selected = None;
        }
        true
    }

    /// Says where the street lies as loaded. A setting, outside the history.
    pub fn with_frame(mut self, frame: Option<ZoneFrame>) -> Editor {
        self.frame = frame;
        if self.is_cut_off(self.zone) {
            self.zone = 0;
            self.selected = None;
        }
        self
    }

    /// Says what the city knows of the street's two ends, node `a` first. A setting, outside the history.
    pub fn with_ends(mut self, ends: [EndInfo; 2]) -> Editor {
        self.ends = ends;
        self
    }

    pub fn frame(&self) -> Option<ZoneFrame> {
        self.frame
    }

    /// Runs an edit on a copy of the selected zone's section, then either records it as a revision (the zoning
    /// settled), or, inside a gesture, applies it without recording.
    fn edit<F>(&mut self, label: Said, f: F) -> bool
    where
        F: FnOnce(&mut Vec<Segment>) -> bool,
    {
        let mut section = self.current().to_vec();
        if !f(&mut section) {
            return false;
        }
        let zone = self.zone;
        if self.gesture.is_some() {
            *self.states[self.cursor].content.section_mut(zone) = section;
            self.pending_label = Some(label);
            return true;
        }
        self.edit_content(label, |content| {
            *content.section_mut(zone) = section;
            true
        })
    }

    /// Runs an edit on a copy of the whole content and records it as a revision when it changed anything.
    fn edit_content<F>(&mut self, label: Said, f: F) -> bool
    where
        F: FnOnce(&mut Content) -> bool,
    {
        let mut next = self.states[self.cursor].content.clone();
        if !f(&mut next) {
            return false;
        }
        next.settle();
        if next == self.states[self.cursor].content {
            return false;
        }
        self.push(label, next);
        true
    }

    fn push(&mut self, label: Said, mut content: Content) {
        content.settle();
        let (class, class_before) = settled_class(content.has_motor(), self.class(), self.class_before());
        let row_mm = self.row_mm();
        let (curb_move, stop_move) = (self.curb_choice.take_moved(), self.stop_choice.take_moved());
        self.push_state(State { label, content, class, class_before, row_mm, curb_move, stop_move });
        self.clamp_zone();
    }

    /// Settling can leave fewer zones than there were: the selected one falls back to the last.
    fn clamp_zone(&mut self) {
        self.zone = self.zone.min(self.states[self.cursor].content.zone_count() - 1);
    }

    fn push_state(&mut self, state: State) {
        self.states.truncate(self.cursor + 1);
        self.states.push(state);
        self.cursor += 1;
    }

    // ---- gestures ------------------------------------------------------

    /// Starts a drag. Edits until `end_gesture` collapse into one revision.
    pub fn begin_gesture(&mut self) {
        if self.gesture.is_none() {
            self.gesture = Some(self.states[self.cursor].content.clone());
            self.pending_label = None;
            self.curb_choice.forget_moved();
            self.stop_choice.forget_moved();
        }
    }

    pub fn end_gesture(&mut self) -> bool {
        let Some(baseline) = self.gesture.take() else {
            return false;
        };
        let mut now = self.states[self.cursor].content.clone();
        now.settle();
        // The live state was mutated in place; restore it, then record it.
        self.states[self.cursor].content = baseline.clone();
        if now == baseline {
            self.curb_choice.forget_moved();
            self.stop_choice.forget_moved();
            return false;
        }
        // An edit that changed anything left its label; without one there is nothing to record.
        let Some(label) = self.pending_label.take() else {
            self.curb_choice.forget_moved();
            self.stop_choice.forget_moved();
            return false;
        };
        self.push(label, now);
        true
    }

    pub fn cancel_gesture(&mut self) {
        if let Some(baseline) = self.gesture.take() {
            self.states[self.cursor].content = baseline;
            // The use or stop is back where it started: so is the choice.
            self.curb_choice.cancel();
            self.stop_choice.cancel();
        }
    }

    // ---- edits ---------------------------------------------------------

    fn new_uid(&mut self) -> u32 {
        let uid = self.next_uid;
        self.next_uid += 1;
        uid
    }

    /// Inserts a segment of `kind` at `index`. Returns its uid, or 0 when the
    /// kind is unknown.
    pub fn add(&mut self, kind: usize, index: usize) -> u32 {
        if kind >= KINDS.len() {
            return 0;
        }
        let k = &KINDS[kind];
        let remaining = self.row_mm() - total(self.current());
        let width = if remaining >= k.min_mm { (remaining / SNAP_MM * SNAP_MM).clamp(k.min_mm, k.default_mm) } else { k.default_mm };
        let uid = self.new_uid();
        let seg = Segment::new(uid, kind, width);
        self.edit(Said::new("rev-add").with("kind", kind_lower(kind)), |segs| {
            let at = index.min(segs.len());
            segs.insert(at, seg);
            true
        });
        self.selected = Some(uid);
        uid
    }

    pub fn remove(&mut self, uid: u32) -> bool {
        let Some(pos) = self.current().iter().position(|s| s.uid == uid) else {
            return false;
        };
        let name = kind_lower(self.current()[pos].kind_at(self.time_min));
        let neighbour = if pos + 1 < self.current().len() {
            Some(self.current()[pos + 1].uid)
        } else if pos > 0 {
            Some(self.current()[pos - 1].uid)
        } else {
            None
        };
        let changed = self.edit(Said::new("rev-remove").with("kind", name), |segs| {
            segs.remove(pos);
            true
        });
        if changed && self.selected == Some(uid) {
            self.selected = neighbour;
        }
        changed
    }

    /// Moves a segment so it ends up at `index` in the resulting order.
    pub fn move_to(&mut self, uid: u32, index: usize) -> bool {
        let Some(pos) = self.current().iter().position(|s| s.uid == uid) else {
            return false;
        };
        let name = kind_lower(self.current()[pos].kind_at(self.time_min));
        let changed = self.edit(Said::new("rev-move").with("kind", name), |segs| {
            let seg = segs.remove(pos);
            let at = index.min(segs.len());
            segs.insert(at, seg);
            at != pos
        });
        if changed {
            self.selected = Some(uid);
        }
        changed
    }

    /// Sets a width in millimetres, rounded to 10 mm and kept in the kind's
    /// allowed range.
    pub fn set_width(&mut self, uid: u32, width_mm: i32) -> bool {
        let Some(pos) = self.current().iter().position(|s| s.uid == uid) else {
            return false;
        };
        let seg = &self.current()[pos];
        let kind = seg.kind_at(self.time_min);
        let (lo, hi) = seg.bounds();
        let w = (((width_mm as f64 / 10.0).round() as i32) * 10).clamp(lo, hi);
        let label = Said::new("rev-resize").with("kind", kind_lower(kind));
        self.edit(label, |segs| {
            segs[pos].width_mm = w;
            true
        })
    }

    /// Sets a piece's surface. Refused when the kind does not allow it.
    pub fn set_material(&mut self, uid: u32, material: usize) -> bool {
        let Some(pos) = self.current().iter().position(|s| s.uid == uid) else {
            return false;
        };
        let seg = &self.current()[pos];
        let active = seg.variant_at(self.time_min);
        let kind = &KINDS[seg.kind_at(self.time_min)];
        if !kind.materials.contains(&material) {
            return false;
        }
        let label = Said::new("rev-surface").with("kind", Arg::Kind(kind.id)).with("material", Arg::MaterialLower(MATERIALS[material].id));
        self.edit(label, |segs| {
            let target = match active {
                Some(vi) => &mut segs[pos].variants[vi].material,
                None => &mut segs[pos].material,
            };
            let changed = *target != material;
            *target = material;
            changed
        })
    }

    /// Sets a piece's curb; `None` is a flush edge. Refused when the kind has
    /// no curb or the curb is not in the table.
    pub fn set_curb(&mut self, uid: u32, curb: Option<usize>) -> bool {
        let Some(pos) = self.current().iter().position(|s| s.uid == uid) else {
            return false;
        };
        let kind = &KINDS[self.current()[pos].kind_at(self.time_min)];
        if !kind.has_curb || curb.is_some_and(|c| !kind.curbs.contains(&c)) {
            return false;
        }
        let what = curb.map_or(Arg::Msg("rev-word-none"), |c| Arg::CurbLower(CURBS[c].id));
        let label = Said::new("rev-curb").with("kind", Arg::Kind(kind.id)).with("curb", what);
        self.edit(label, |segs| {
            let changed = segs[pos].curb != curb;
            segs[pos].curb = curb;
            changed
        })
    }

    /// Sets whether a transit lane carries trams or buses. Refused unless the
    /// piece is a transit lane at the shown time.
    pub fn set_tram(&mut self, uid: u32, tram: bool) -> bool {
        let Some(pos) = self.current().iter().position(|s| s.uid == uid) else {
            return false;
        };
        let kind = &KINDS[self.current()[pos].kind_at(self.time_min)];
        if kind.id != "bus" {
            return false;
        }
        let label =
            Said::new("rev-vehicle").with("kind", Arg::Kind(kind.id)).with("vehicle", if tram { Arg::Msg("rev-word-tram") } else { Arg::Msg("rev-word-bus") });
        self.edit(label, |segs| {
            let changed = segs[pos].tram != tram;
            segs[pos].tram = tram;
            changed
        })
    }

    /// Sets which way a lane runs; `None` is two-way. Refused for a kind with
    /// no direction, for `None` where the kind requires one, and for an index
    /// outside `DIRECTIONS`.
    pub fn set_direction(&mut self, uid: u32, direction: Option<usize>) -> bool {
        let Some(pos) = self.current().iter().position(|s| s.uid == uid) else {
            return false;
        };
        let active = self.current()[pos].variant_at(self.time_min);
        let kind = &KINDS[self.current()[pos].kind_at(self.time_min)];
        let allowed = match kind.direction {
            DirectionRule::None => false,
            DirectionRule::Required => direction.is_some(),
            DirectionRule::Optional => true,
        };
        if !allowed || direction.is_some_and(|d| d >= DIRECTIONS.len()) {
            return false;
        }
        let what = direction.map_or(Arg::Msg("rev-word-two-way"), |d| Arg::DirectionLower(DIRECTIONS[d].id));
        let label = Said::new("rev-direction").with("kind", Arg::Kind(kind.id)).with("direction", what);
        self.edit(label, |segs| {
            let target = match active {
                Some(vi) => &mut segs[pos].variants[vi].direction,
                None => &mut segs[pos].direction,
            };
            let changed = *target != direction;
            *target = direction;
            changed
        })
    }

    /// Sets the region, by index into `REGIONS`. The region itself is a
    /// setting and stays out of the history. When it changes which side of the
    /// road traffic keeps to, the street's lanes are mirrored to match: silently
    /// while the street is untouched (it is just laid out for that region), and
    /// as one undoable change once it has edits.
    pub fn set_region(&mut self, region: usize) -> bool {
        if region >= REGIONS.len() || region == self.region {
            return false;
        }
        let side = REGIONS[region].drive_side;
        let side_changed = side != REGIONS[self.region].drive_side;
        self.region = region;
        if !side_changed {
            return true;
        }
        if self.states.len() == 1 {
            let content = &mut self.states[0].content;
            default_directions(&mut content.segments, side);
            for own in content.zones.iter_mut().filter_map(|z| z.segments.as_mut()) {
                default_directions(own, side);
            }
        } else {
            // The side of the road is the street's, so every section is mirrored, not just the selected zone's.
            let label = side_said(side);
            self.edit_content(label, |content| {
                let mut changed = false;
                let sections = std::iter::once(&mut content.segments).chain(content.zones.iter_mut().filter_map(|z| z.segments.as_mut()));
                for segs in sections {
                    for s in segs.iter_mut() {
                        if let Some(d) = s.direction {
                            s.direction = Some(1 - d);
                            changed = true;
                        }
                        for v in &mut s.variants {
                            if let Some(d) = v.direction {
                                v.direction = Some(1 - d);
                                changed = true;
                            }
                        }
                    }
                }
                changed
            });
        }
        true
    }

    /// Sets the time of day the sheet shows, in minutes after midnight.
    /// Snapped to a quarter hour. A setting, so it stays out of the history.
    pub fn set_time(&mut self, time_min: i32) -> bool {
        let t = (time_min.clamp(0, 24 * 60 - 1) / SLOT_MIN) * SLOT_MIN;
        let changed = t != self.time_min;
        self.time_min = t;
        changed
    }

    fn seg_pos(&self, uid: u32) -> Option<usize> {
        self.current().iter().position(|s| s.uid == uid)
    }

    /// Gives a roadway piece a different type at certain times. It gets the
    /// first type that fits and the first free window of a few common ones
    /// (morning rush, evening rush, ...). Refused when the piece cannot change
    /// type or every window is taken.
    pub fn add_variant(&mut self, uid: u32) -> bool {
        let Some(pos) = self.seg_pos(uid) else {
            return false;
        };
        let seg = &self.current()[pos];
        let alts = seg.alt_kinds();
        let prefer = ["bus", "parking", "loading", "travel"];
        let Some(kind) = prefer.iter().filter_map(|id| kind_index(id)).find(|k| alts.contains(k)) else {
            return false;
        };
        let used = seg.variants.iter().fold(0u128, |m, v| m | window_mask(v.from_min, v.to_min));
        let windows = [(7 * 60, 10 * 60), (16 * 60, 19 * 60), (10 * 60, 16 * 60), (19 * 60, 7 * 60)];
        let Some(&(from_min, to_min)) = windows.iter().find(|(f, t)| window_mask(*f, *t) & used == 0) else {
            return false;
        };
        let label = window_said(seg.kind, kind, from_min, to_min);
        self.edit(label, |segs| {
            let d = &mut segs[pos];
            d.variants.push(Variant { kind, material: KINDS[kind].materials[0], direction: direction_for(kind, d.direction), from_min, to_min, days: None });
            let (lo, hi) = d.bounds();
            d.width_mm = d.width_mm.clamp(lo, hi);
            true
        })
    }

    /// Changes the type of one of a piece's variants.
    pub fn set_variant_kind(&mut self, uid: u32, index: usize, kind: usize) -> bool {
        let Some(pos) = self.seg_pos(uid) else {
            return false;
        };
        let seg = &self.current()[pos];
        let Some(v) = seg.variants.get(index) else {
            return false;
        };
        if v.kind == kind || !seg.alt_kinds_except(Some(index)).contains(&kind) {
            return false;
        }
        let label = window_said(seg.kind, kind, v.from_min, v.to_min);
        self.edit(label, |segs| {
            let d = &mut segs[pos];
            let keep = d.variants[index].direction.or(d.direction);
            d.variants[index].kind = kind;
            d.variants[index].material = KINDS[kind].materials[0];
            d.variants[index].direction = direction_for(kind, keep);
            let (lo, hi) = d.bounds();
            d.width_mm = d.width_mm.clamp(lo, hi);
            true
        })
    }

    /// Sets which way one of a piece's other-times types runs; `None` is
    /// two-way, allowed only where the type's rule says so.
    pub fn set_variant_direction(&mut self, uid: u32, index: usize, direction: Option<usize>) -> bool {
        let Some(pos) = self.seg_pos(uid) else {
            return false;
        };
        let seg = &self.current()[pos];
        let Some(v) = seg.variants.get(index) else {
            return false;
        };
        let allowed = match KINDS[v.kind].direction {
            DirectionRule::None => false,
            DirectionRule::Required => direction.is_some(),
            DirectionRule::Optional => true,
        };
        if !allowed || direction.is_some_and(|d| d >= DIRECTIONS.len()) || v.direction == direction {
            return false;
        }
        let what = direction.map_or(Arg::Msg("rev-word-two-way"), |d| Arg::DirectionLower(DIRECTIONS[d].id));
        let label = Said::new("rev-variant-direction").with("base", Arg::Kind(KINDS[seg.kind].id)).with("kind", kind_lower(v.kind)).with("direction", what);
        self.edit(label, |segs| {
            segs[pos].variants[index].direction = direction;
            true
        })
    }

    /// Moves the window of one of a piece's variants. Refused when a time is
    /// not on a quarter hour, the two are equal, or it overlaps another window.
    pub fn set_variant_time(&mut self, uid: u32, index: usize, from_min: i32, to_min: i32) -> bool {
        let Some(pos) = self.seg_pos(uid) else {
            return false;
        };
        let seg = &self.current()[pos];
        if index >= seg.variants.len() || !valid_window(from_min, to_min) {
            return false;
        }
        let mask = window_mask(from_min, to_min);
        let others = seg.variants.iter().enumerate().filter(|(i, _)| *i != index).fold(0u128, |m, (_, v)| m | window_mask(v.from_min, v.to_min));
        if mask & others != 0 {
            return false;
        }
        let label = window_said(seg.kind, seg.variants[index].kind, from_min, to_min);
        self.edit(label, |segs| {
            let v = &mut segs[pos].variants[index];
            let changed = (v.from_min, v.to_min) != (from_min, to_min);
            v.from_min = from_min;
            v.to_min = to_min;
            changed
        })
    }

    pub fn remove_variant(&mut self, uid: u32, index: usize) -> bool {
        let Some(pos) = self.seg_pos(uid) else {
            return false;
        };
        let seg = &self.current()[pos];
        if index >= seg.variants.len() {
            return false;
        }
        let label = Said::new("rev-variant-remove").with("kind", kind_lower(seg.kind));
        self.edit(label, |segs| {
            segs[pos].variants.remove(index);
            true
        })
    }

    /// Arranges the roadway as one of the Atlas lane measures (`measures::DEFS`).
    /// Refused when it does not suit this street or will not fit.
    pub fn apply_measure(&mut self, code: &str) -> bool {
        let Some(def) = measures::DEFS.iter().find(|d| d.code == code) else {
            return false;
        };
        let existing = self.current().to_vec();
        let mut next_uid = self.next_uid;
        let Some(arranged) = measures::arrange(code, &existing, self.row_mm(), REGIONS[self.region].drive_side, self.class().is_freeway(), &mut next_uid)
        else {
            return false;
        };
        let ok = self.edit(Said::new("rev-measure").with("code", Arg::Text(def.code.to_string())).with("name", Arg::Measure(def.code)), |segs| {
            *segs = arranged;
            true
        });
        if ok {
            self.next_uid = next_uid;
            self.selected = None;
        }
        ok
    }

    /// Makes the street pedestrian: every motor piece (travel, parking, loading, bus, shoulder, at any hour: a bike
    /// lane with a motor hour is one too) becomes a piece of `kind` (one of `PEDESTRIAN_KINDS`) at its own width and
    /// in its own place, with that kind's surface and curb and without hours or direction; the other pieces, bike
    /// lanes among them, stay. One undoable edit. Refused when no piece is a motor piece, or `kind` is not one of the three.
    pub fn make_pedestrian(&mut self, kind: usize) -> bool {
        if kind >= KINDS.len() || !PEDESTRIAN_KINDS.contains(&KINDS[kind].id) {
            return false;
        }
        self.edit(Said::new("rev-made-pedestrian"), |segs| {
            let mut changed = false;
            for s in segs.iter_mut().filter(|s| s.has_motor()) {
                *s = Segment::new(s.uid, kind, s.width_mm);
                changed = true;
            }
            changed
        })
    }

    pub fn nudge_width(&mut self, uid: u32, delta_mm: i32) -> bool {
        let Some(s) = self.current().iter().find(|s| s.uid == uid) else {
            return false;
        };
        let w = s.width_mm + delta_mm;
        self.set_width(uid, w)
    }

    /// Trades width across the boundary after segment `left_index`, keeping
    /// the sum of the pair fixed. `delta_mm` is measured from the gesture
    /// start, so a drag is idempotent per pointer position.
    pub fn resize_boundary(&mut self, left_index: usize, delta_mm: i32) -> bool {
        let Some(base) = self.gesture.as_ref().map(|g| g.section(self.zone).to_vec()) else {
            return false;
        };
        if left_index + 1 >= base.len() {
            return false;
        }
        let (l, r) = (&base[left_index], &base[left_index + 1]);
        let (lk, rk) = (l.kind_at(self.time_min), r.kind_at(self.time_min));
        let pair = l.width_mm + r.width_mm;
        // Left width limits from both segments' allowed ranges.
        let ((llo, lhi), (rlo, rhi)) = (l.bounds(), r.bounds());
        let lo = llo.max(pair - rhi);
        let hi = lhi.min(pair - rlo);
        if lo > hi {
            return false;
        }
        let left = snap(l.width_mm + delta_mm).clamp(lo, hi);
        let label = Said::new("rev-resize-pair").with("a", kind_lower(lk)).with("b", kind_lower(rk));
        let (lu, ru) = (l.uid, r.uid);
        self.apply_widths(label, &[(lu, left), (ru, pair - left)])
    }

    /// Resizes one segment from its gesture-start width, taking the space
    /// from, or giving it to, the unassigned width of the street.
    pub fn resize_edge(&mut self, uid: u32, delta_mm: i32) -> bool {
        let Some(base) = self.gesture.as_ref().map(|g| g.section(self.zone).to_vec()) else {
            return false;
        };
        let Some(s) = base.iter().find(|s| s.uid == uid) else {
            return false;
        };
        let kind = s.kind_at(self.time_min);
        let (lo, hi) = s.bounds();
        let w = snap(s.width_mm + delta_mm).clamp(lo, hi);
        let label = Said::new("rev-resize").with("kind", kind_lower(kind));
        self.apply_widths(label, &[(uid, w)])
    }

    fn apply_widths(&mut self, label: Said, widths: &[(u32, i32)]) -> bool {
        self.edit(label, |segs| {
            let mut changed = false;
            for (uid, w) in widths {
                if let Some(s) = segs.iter_mut().find(|s| s.uid == *uid)
                    && s.width_mm != *w
                {
                    s.width_mm = *w;
                    changed = true;
                }
            }
            changed
        })
    }

    // ---- history -------------------------------------------------------

    pub fn undo(&mut self) -> bool {
        self.cancel_gesture();
        if self.cursor == 0 {
            return false;
        }
        self.curb_choice.undo(self.states[self.cursor].curb_move);
        self.stop_choice.undo(self.states[self.cursor].stop_move);
        self.cursor -= 1;
        self.reselect();
        true
    }

    pub fn redo(&mut self) -> bool {
        self.cancel_gesture();
        if self.cursor + 1 >= self.states.len() {
            return false;
        }
        self.cursor += 1;
        self.curb_choice.redo(self.states[self.cursor].curb_move);
        self.stop_choice.redo(self.states[self.cursor].stop_move);
        self.reselect();
        true
    }

    /// Sets the speed limit to one of `SPEED_CHOICES`, or to none (`None`). False, with no revision, for any other
    /// value (an imported one outside the choices is kept, never set) or when it is the speed already.
    pub fn set_speed(&mut self, kmh: Option<u16>) -> bool {
        let current = self.states[self.cursor].content.speed;
        if kmh.is_some_and(|n| !SPEED_CHOICES.contains(&n)) || current.kmh() == kmh {
            return false;
        }
        let label = match kmh {
            Some(n) => Said::new("rev-speed").with("kmh", Arg::Num(i64::from(n))),
            None => Said::new("rev-speed-not-set"),
        };
        self.edit_content(label, |c| {
            c.speed = Speed::from_kmh(kmh);
            true
        })
    }

    /// Returns the proposal to the existing street as a new revision.
    pub fn reset(&mut self) -> bool {
        self.cancel_gesture();
        let first = self.states[0].clone();
        if first.content == self.states[self.cursor].content && first.row_mm == self.row_mm() {
            return false;
        }
        self.push_state(State { label: Said::new("rev-reset"), ..first });
        self.clamp_zone();
        self.selected = None;
        true
    }

    /// After the history moved: the selected zone and piece fall back when they no longer exist.
    fn reselect(&mut self) {
        self.clamp_zone();
        if let Some(uid) = self.selected
            && !self.current().iter().any(|s| s.uid == uid)
        {
            self.selected = None;
        }
    }

    // ---- selection and hit testing ------------------------------------

    pub fn select(&mut self, uid: Option<u32>) {
        self.selected = uid.filter(|u| self.current().iter().any(|s| s.uid == *u));
    }

    /// Moves the selection by `delta` segments, starting from the first or
    /// last segment when nothing is selected.
    pub fn select_relative(&mut self, delta: i32) {
        let segs = self.current();
        if segs.is_empty() {
            return;
        }
        let pos = self.selected.and_then(|u| segs.iter().position(|s| s.uid == u));
        let next = match pos {
            Some(p) => (p as i32 + delta).clamp(0, segs.len() as i32 - 1) as usize,
            None if delta >= 0 => 0,
            None => segs.len() - 1,
        };
        self.selected = Some(segs[next].uid);
    }

    /// Index at which a dragged segment would land for a pointer at `x_mm`
    /// from the street's left edge. `exclude` is the dragged uid, or 0.
    pub fn drop_index(&self, x_mm: f64, exclude: u32) -> usize {
        let mut x = 0;
        let mut index = 0;
        for s in self.current().iter().filter(|s| s.uid != exclude) {
            if x_mm < (x + s.width_mm / 2) as f64 {
                break;
            }
            x += s.width_mm;
            index += 1;
        }
        index
    }

    // ---- read side -----------------------------------------------------

    pub fn view(&self) -> View {
        let segs = self.current();
        let total_mm = total(segs);
        let existing = self.states[0].content.section(self.zone.min(self.states[0].content.zone_count() - 1));
        // Everything that measures the street does so as it is at the shown time.
        let at = |v: &[Segment]| v.iter().map(|s| s.at(self.time_min)).collect::<Vec<_>>();
        let (now, existing_now) = (at(segs), at(existing));
        View {
            name: title_of(self.street_name.as_deref(), self.class()),
            class: self.class(),
            has_motor: segs.iter().any(Segment::has_motor),
            region: REGIONS[self.region].id,
            speed_kmh: self.states[self.cursor].content.speed.kmh(),
            side: REGIONS[self.region].drive_side,
            row_mm: self.row_mm(),
            total_mm,
            delta_mm: total_mm - self.row_mm(),
            time_min: self.time_min,
            segments: self.seg_views(segs),
            existing: self.seg_views(existing),
            existing_total_mm: total(existing),
            selected: self.selected,
            outcomes: outcomes(&now, &existing_now),
            checks: self.street_checks(),
            zones: self.zones_view(),
            curb: self.curb_view(),
            stops: self.stops_view(),
            curb_shown: self.curb_shown(),
            stops_shown: self.stops_shown(),
            measures: self.measure_views(segs, &now),
            revisions: self.states[1..=self.cursor].iter().enumerate().map(|(i, s)| Revision { step: i + 1, label: s.label.clone() }).collect(),
            can_undo: self.cursor > 0,
            can_redo: self.cursor + 1 < self.states.len(),
            changed: self.states[self.cursor].content != self.states[0].content,
            source: self.source.clone(),
        }
    }

    /// The checks of one section, as the street is at the shown time.
    fn section_checks(&self, segs: &[Segment]) -> Vec<Check> {
        let now: Vec<Segment> = segs.iter().map(|s| s.at(self.time_min)).collect();
        checks(&now, self.row_mm(), REGIONS[self.region].drive_side, self.class().is_freeway(), self.class() == StreetClass::Pedestrian)
    }

    /// Whether every check passes in a section.
    fn section_passes(&self, segs: &[Segment]) -> bool {
        self.section_checks(segs).iter().all(|c| c.ok)
    }

    /// The street's checks: a check passes only when it passes in every zone. Where it does not, its detail and the
    /// zone it names are the first failing zone's; else they are the selected zone's.
    /// With two zones or more and a transition at some boundary, the boundary checks follow them (`transition_checks`).
    fn street_checks(&self) -> Vec<Check> {
        let mut street = self.zone_checks();
        if !self.states[self.cursor].content.curb.is_empty() {
            street.extend(self.curb_checks());
        }
        if !self.states[self.cursor].content.stops.is_empty() {
            street.extend(self.stop_checks());
        }
        street
    }

    /// The section checks of every zone, and the boundary checks (see `street_checks`).
    fn zone_checks(&self) -> Vec<Check> {
        let content = &self.states[self.cursor].content;
        let mut street = self.section_checks(self.current());
        if content.zones.len() < 2 {
            return street;
        }
        let transitions = !self.transitions().is_empty();
        for (z, zone) in content.zones.iter().enumerate() {
            for (check, in_zone) in street.iter_mut().zip(self.section_checks(content.section(z))) {
                if !in_zone.ok && check.zone.is_none() {
                    check.ok = false;
                    check.zone = Some(zone_label(zone, z + 1));
                    check.detail = in_zone.detail;
                }
            }
        }
        if transitions {
            street.extend(self.transition_checks(content));
        }
        street
    }

    /// The checks of the boundaries: one per failing boundary, else one passing check of each kind.
    fn transition_checks(&self, content: &Content) -> Vec<Check> {
        let label = |z: usize| zone_label_mid(&content.zones[z], z + 1);
        let failing = self.failing_transitions();
        let units = |mm| Arg::Length(mm);
        let make = |id: &'static str, ok: bool, detail: Said, zone: Option<Said>| Check {
            id,
            ok,
            label: if id == "lane-drop" { Said::new("check-label-lane-drop") } else { Said::new("check-label-transition-room") },
            detail,
            zone,
        };
        let mut out = Vec::new();
        for id in ["transition-room", "lane-drop"] {
            let before = out.len();
            for (b, host, why) in &failing {
                let boundary = || Said::new("check-boundary").with("a", Arg::Said(Box::new(label(*b)))).with("b", Arg::Said(Box::new(label(*b + 1))));
                match (id, why) {
                    ("transition-room", Failing::Room { need_mm, room_mm }) => {
                        let detail = Said::new("check-detail-transition-room")
                            .with("need", units(*need_mm))
                            .with("zone", Arg::Said(Box::new(label(*host))))
                            .with("room", units(*room_mm));
                        out.push(make(id, false, detail, Some(boundary())));
                    }
                    ("lane-drop", Failing::LaneDrop) => out.push(make(id, false, Said::new("check-detail-lane-drop"), Some(boundary()))),
                    _ => {}
                }
            }
            if out.len() == before {
                let ok = if id == "lane-drop" { Said::new("check-detail-lane-drop-ok") } else { Said::new("check-detail-transition-room-ok") };
                out.push(make(id, true, ok, None));
            }
        }
        out
    }

    fn measure_views(&self, raw: &[Segment], now: &[Segment]) -> Vec<MeasureView> {
        let side = REGIONS[self.region].drive_side;
        let freeway = self.class().is_freeway();
        measures::detect(raw, now, side, freeway)
            .into_iter()
            .zip(measures::DEFS.iter())
            .map(|(f, d)| {
                let can_apply = measures::arrange(d.code, raw, self.row_mm(), side, freeway, &mut self.next_uid.clone()).is_some();
                MeasureView { code: d.code, present: f.present, problems: f.problems, can_apply }
            })
            .collect()
    }

    fn seg_views(&self, segs: &[Segment]) -> Vec<SegView> {
        let mut x = 0;
        segs.iter()
            .map(|s| {
                let (min_mm, max_mm) = s.bounds();
                let n = s.at(self.time_min);
                let v = SegView {
                    uid: s.uid,
                    kind: n.kind,
                    base_kind: s.kind,
                    width_mm: s.width_mm,
                    x_mm: x,
                    min_mm,
                    max_mm,
                    material: MATERIALS[n.material].id,
                    curb: n.curb.map(|c| CURBS[c].id),
                    direction: n.direction.map(|d| DIRECTIONS[d].id),
                    tram: n.tram,
                    variants: s
                        .variants
                        .iter()
                        .map(|v| VariantView {
                            kind: v.kind,
                            material: MATERIALS[v.material].id,
                            direction: v.direction.map(|d| DIRECTIONS[d].id),
                            from_min: v.from_min,
                            to_min: v.to_min,
                            days: v.days.clone(),
                        })
                        .collect(),
                    active_variant: s.variant_at(self.time_min),
                    alt_kinds: s.alt_kinds(),
                };
                x += s.width_mm;
                v
            })
            .collect()
    }
}

// ---- outcomes and checks ------------------------------------------------

fn capacity(segs: &[Segment]) -> i32 {
    segs.iter().map(|s| KINDS[s.kind].people_per_hour_per_m * s.width_mm / 1000).sum()
}

fn shares(segs: &[Segment]) -> Vec<Share> {
    let t = total(segs).max(1);
    // the modes the width is shared out among, then what no mode owns (the buffers)
    Mode::ALL
        .iter()
        .chain([&Mode::Neutral])
        .map(|m| {
            let mm: i32 = segs.iter().filter(|s| KINDS[s.kind].mode == *m).map(|s| s.width_mm).sum();
            Share { mode: *m, mm, pct: (mm as f64 * 100.0 / t as f64).round() as i32 }
        })
        .collect()
}

fn outcomes(segs: &[Segment], existing: &[Segment]) -> Outcomes {
    Outcomes { share: shares(segs), existing_share: shares(existing), capacity_pph: capacity(segs), existing_capacity_pph: capacity(existing) }
}

/// The widest unbroken run of adjacent sidewalk pieces; any other piece between two sidewalks breaks the run.
fn widest_walkway_mm<'a>(segs: impl IntoIterator<Item = &'a Segment>) -> i32 {
    let (mut best, mut run) = (0, 0);
    for s in segs {
        if KINDS[s.kind].id == "sidewalk" {
            run += s.width_mm;
            best = best.max(run);
        } else {
            run = 0;
        }
    }
    best
}

fn checks(segs: &[Segment], row_mm: i32, side: Side, freeway: bool, pedestrian: bool) -> Vec<Check> {
    let total_mm = total(segs);
    let delta = total_mm - row_mm;
    let fits = delta <= 0;
    let edge_walk = |s: Option<&Segment>| s.is_some_and(|s| KINDS[s.kind].id == "sidewalk");
    // A freeway has no sidewalks to ask for, and a pedestrian street no roadway to edge.
    let both_edges = freeway || pedestrian || (edge_walk(segs.first()) && edge_walk(segs.last()));
    // A pedestrian street keeps room for emergency vehicles in an unbroken run of sidewalk; the others in a lane.
    let access = if pedestrian {
        widest_walkway_mm(segs) >= ACCESS_LANE_MM
    } else {
        segs.iter().any(|s| matches!(KINDS[s.kind].id, "travel" | "bus") && s.width_mm >= ACCESS_LANE_MM)
    };
    // With traffic on the right, lanes coming toward the viewer (1) sit to the
    // left of lanes going away (0); with traffic on the left, the reverse. A
    // one-way street has nothing to conflict.
    let lanes: Vec<usize> = segs.iter().filter(|s| required_direction(s)).filter_map(|s| s.direction).collect();
    let (wrong_first, wrong_then) = if side == Side::Right { (0, 1) } else { (1, 0) };
    let keeps = lanes.iter().position(|&d| d == wrong_first).is_none_or(|i| !lanes[i..].contains(&wrong_then));
    vec![
        Check {
            zone: None,
            id: "fits",
            ok: fits,
            label: Said::new("check-label-fits"),
            detail: if !fits {
                Said::new("check-fits-over").with("amount", Arg::Length(delta))
            } else if delta == 0 {
                Said::new("check-fits-full")
            } else {
                Said::new("fit-left").with("amount", Arg::Length(-delta))
            },
        },
        Check {
            zone: None,
            id: "edges",
            ok: both_edges,
            label: Said::new("check-label-edges"),
            detail: if freeway {
                Said::new("check-detail-no-sidewalks")
            } else if pedestrian {
                Said::new("check-detail-no-edges-pedestrian")
            } else if both_edges {
                Said::new("check-detail-both-sides")
            } else {
                Said::new("check-detail-one-side")
            },
        },
        Check {
            zone: None,
            id: "access",
            ok: access,
            label: Said::new("check-label-access"),
            detail: {
                let key = match (pedestrian, access) {
                    (false, true) => "check-access-ok",
                    (false, false) => "check-access-bad",
                    (true, true) => "check-access-ped-ok",
                    (true, false) => "check-access-ped-bad",
                };
                Said::new(key).with("amount", Arg::Length(ACCESS_LANE_MM))
            },
        },
        Check {
            zone: None,
            id: "side",
            ok: keeps,
            label: side_said(side),
            detail: match (keeps, side) {
                (true, _) => Said::new("check-detail-side-ok"),
                (false, Side::Right) => Said::new("check-detail-side-bad").with("side", Arg::Msg("side-word-right")),
                (false, Side::Left) => Said::new("check-detail-side-bad").with("side", Arg::Msg("side-word-left")),
            },
        },
    ]
}

// ---- serialised view ----------------------------------------------------

#[derive(Serialize)]
pub struct SegView {
    pub uid: u32,
    /// The type at the shown time.
    pub kind: usize,
    /// The type outside every variant's window.
    pub base_kind: usize,
    pub width_mm: i32,
    pub x_mm: i32,
    pub min_mm: i32,
    pub max_mm: i32,
    pub material: &'static str,
    pub curb: Option<&'static str>,
    pub direction: Option<&'static str>,
    /// A transit lane at the shown time carries trams.
    pub tram: bool,
    pub variants: Vec<VariantView>,
    pub active_variant: Option<usize>,
    /// Types this piece may take at other times.
    pub alt_kinds: Vec<usize>,
}

#[derive(Serialize)]
pub struct VariantView {
    pub kind: usize,
    pub direction: Option<&'static str>,
    pub material: &'static str,
    pub from_min: i32,
    pub to_min: i32,
    pub days: Option<String>,
}

#[derive(Serialize)]
pub struct Share {
    pub mode: Mode,
    pub mm: i32,
    pub pct: i32,
}

#[derive(Serialize)]
pub struct Outcomes {
    pub share: Vec<Share>,
    pub existing_share: Vec<Share>,
    pub capacity_pph: i32,
    pub existing_capacity_pph: i32,
}

#[derive(Serialize)]
pub struct Check {
    pub id: &'static str,
    pub ok: bool,
    pub label: Said,
    pub detail: Said,
    /// The first zone that fails this check, when the street has several zones and the check fails; else `None`.
    pub zone: Option<Said>,
}

#[derive(Serialize)]
pub struct MeasureView {
    pub code: &'static str,
    pub present: bool,
    pub problems: Vec<measures::Problem>,
    /// The street can be arranged as this measure.
    pub can_apply: bool,
}

#[derive(Serialize)]
pub struct Revision {
    pub step: usize,
    pub label: Said,
}

#[derive(Serialize)]
pub struct View {
    /// What the street is called: its own name, or else what sort of street it is.
    pub name: Said,
    pub class: StreetClass,
    /// Some piece is a motor piece at some hour: the street can be made pedestrian.
    pub has_motor: bool,
    pub region: &'static str,
    /// The speed limit, when the street has one.
    pub speed_kmh: Option<u16>,
    /// The side of the road traffic keeps, for the 2.5D sign.
    pub side: Side,
    /// The time of day shown, in minutes after midnight.
    pub time_min: i32,
    pub row_mm: i32,
    pub total_mm: i32,
    pub delta_mm: i32,
    pub segments: Vec<SegView>,
    pub existing: Vec<SegView>,
    pub existing_total_mm: i32,
    pub selected: Option<u32>,
    pub outcomes: Outcomes,
    /// The street's checks: each passes only when it passes in every zone.
    pub checks: Vec<Check>,
    /// The street's zones, for the strip.
    pub zones: ZonesView,
    /// The curbside strips.
    pub curb: CurbView,
    /// The strips of the transit stops, a row under each side's curbside strip.
    pub stops: StopsView,
    /// The curbside uses the 2.5D shows, on the selected zone's pieces.
    pub curb_shown: Vec<ShownInstance>,
    /// The transit stops the 2.5D shows, on the selected zone's pieces.
    pub stops_shown: Vec<ShownStop>,
    /// The Atlas lane measures: which this section forms, and what each lacks.
    pub measures: Vec<MeasureView>,
    pub revisions: Vec<Revision>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub changed: bool,
    /// The OSM ways the street was made from.
    pub source: Vec<OsmRef>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a message of the model says in English, as the page shows it.
    fn says(said: &Said) -> String {
        crate::street::text::say_now(&crate::i18n_for(crate::shared::i18n::Locale::En), crate::shared::units::Units::Metres, said)
    }

    #[test]
    fn a_street_made_of_given_pieces_is_sound_and_as_wide_as_they_are() {
        let kind = |id| kind_index(id).unwrap();
        let pieces = [
            Piece { kind: kind("sidewalk"), width_mm: 2000, direction: None, variants: Vec::new() },
            Piece { kind: kind("travel"), width_mm: 3200, direction: Some(1), variants: Vec::new() },
            Piece { kind: kind("travel"), width_mm: 3200, direction: Some(0), variants: Vec::new() },
            Piece { kind: kind("bike"), width_mm: 1500, direction: None, variants: Vec::new() },
            Piece { kind: kind("sidewalk"), width_mm: 2000, direction: Some(0), variants: Vec::new() },
        ];
        let s = Street::imported(StreetClass::Local, Side::Left, &pieces);
        assert!(s.is_sound());
        assert_eq!((s.class, s.row_mm, s.side), (StreetClass::Local, 2000 + 3200 + 3200 + 1500 + 2000, Side::Left));
        assert_eq!(s.segments.iter().map(|g| g.uid).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5]);
        // a driving lane keeps its way; a kind with no direction has none, even if one was given
        assert_eq!(s.segments.iter().map(|g| g.direction).collect::<Vec<_>>(), vec![None, Some(1), Some(0), None, None]);
        assert_eq!(s.next_uid, 6);
        // and the editor opens on it
        assert_eq!(Editor::from_street(&s, &s, 0).view().row_mm, s.row_mm);
    }

    #[test]
    fn a_street_keeps_the_osm_ways_it_came_from_through_the_editor_and_says_when_it_was_changed() {
        use crate::shared::provenance::OsmRef;
        let travel = kind("travel");
        let mut street = Street::imported(StreetClass::Local, Side::Right, &[Piece { kind: travel, width_mm: 3200, direction: Some(0), variants: Vec::new() }]);
        street.source = vec![OsmRef { id: 4687530, version: Some(49) }, OsmRef { id: 7, version: None }];
        let mut e = Editor::from_street(&street, &street, 0);
        assert_eq!(e.view().source, street.source);
        assert!(!e.view().changed);
        let uid = e.view().segments[0].uid;
        assert!(e.nudge_width(uid, 100));
        assert!(e.view().changed);
        assert_eq!(e.snapshot().source, street.source);
        // opened on a street that was changed before, it is changed
        assert!(Editor::from_street(&street, &e.snapshot(), 0).view().changed);
        e.undo();
        assert!(!e.view().changed);
    }

    #[test]
    fn a_piece_wider_or_narrower_than_its_kind_allows_is_brought_inside() {
        let kind = kind_index("travel").unwrap();
        let s = Street::imported(
            StreetClass::Local,
            Side::Right,
            &[
                Piece { kind, width_mm: 100, direction: Some(0), variants: Vec::new() },
                Piece { kind, width_mm: 99_000, direction: Some(1), variants: Vec::new() },
            ],
        );
        assert_eq!(s.segments[0].width_mm, KINDS[kind].min_mm);
        assert_eq!(s.segments[1].width_mm, KINDS[kind].max_mm);
        assert_eq!(s.row_mm, KINDS[kind].min_mm + KINDS[kind].max_mm);
    }

    #[test]
    fn a_piece_with_windows_is_its_kind_outside_them_and_the_window_kind_inside() {
        let (parking, bus) = (kind("parking"), kind("bus"));
        let piece = Piece {
            kind: parking,
            width_mm: 3400,
            direction: Some(0),
            variants: vec![Window { kind: bus, from_min: 6 * 60, to_min: 10 * 60, days: Some("Mo-Fr".into()) }],
        };
        let s = Street::imported(StreetClass::Local, Side::Right, &[piece]);
        assert!(s.is_sound());
        let g = &s.segments[0];
        assert_eq!((g.kind, g.variants.len()), (parking, 1));
        assert_eq!((g.variants[0].kind, g.variants[0].from_min, g.variants[0].to_min), (bus, 360, 600));
        assert_eq!((g.kind_at(8 * 60), g.kind_at(12 * 60)), (bus, parking));
        // the width is one both kinds allow
        assert_eq!(g.width_mm, 3000);
        assert_eq!(g.variants[0].direction, g.direction);
        assert_eq!(g.variants[0].days.as_deref(), Some("Mo-Fr"));
        assert_eq!(s.segments[0].variants[0].days, Editor::from_street(&s, &s, 0).view().segments[0].variants[0].days);
    }

    fn ids(e: &Editor) -> Vec<&'static str> {
        e.current().iter().map(|s| KINDS[s.kind].id).collect()
    }

    fn kind(id: &str) -> usize {
        kind_index(id).unwrap()
    }

    /// One driving lane, so that a street made of it is not pedestrian.
    fn travel_lane() -> Piece {
        Piece { kind: kind("travel"), width_mm: 3300, direction: None, variants: Vec::new() }
    }

    /// A street of these kinds, each at its kind's default width, made as an import makes it.
    fn street_of(class: StreetClass, ids: &[&str]) -> Street {
        let pieces: Vec<Piece> = ids
            .iter()
            .map(|id| {
                let kind = kind_index(id).unwrap();
                Piece { kind, width_mm: KINDS[kind].default_mm, direction: None, variants: Vec::new() }
            })
            .collect();
        Street::imported(class, Side::Right, &pieces)
    }

    #[test]
    fn a_street_with_no_motor_piece_is_pedestrian_and_remembers_the_class_it_had() {
        let walkway = street_of(StreetClass::Local, &["sidewalk", "planting", "sidewalk"]);
        assert_eq!((walkway.class, walkway.class_before), (StreetClass::Pedestrian, Some(StreetClass::Local)));
        let busy = street_of(StreetClass::Arterial, &["sidewalk", "travel", "sidewalk"]);
        assert_eq!((busy.class, busy.class_before), (StreetClass::Arterial, None));
        let given = street_of(StreetClass::Pedestrian, &["sidewalk", "sidewalk"]);
        assert_eq!((given.class, given.class_before), (StreetClass::Pedestrian, None), "an imported pedestrian street has no class before");
        let bikes = street_of(StreetClass::Collector, &["sidewalk", "bike", "sidewalk"]);
        assert_eq!((bikes.class, bikes.class_before), (StreetClass::Pedestrian, Some(StreetClass::Collector)), "a bike lane is not a motor piece");
    }

    #[test]
    fn a_pedestrian_street_given_a_lane_goes_back_to_the_class_it_had_or_else_local() {
        let mut s = street_of(StreetClass::Arterial, &["sidewalk", "travel", "sidewalk"]);
        s.segments.retain(|g| KINDS[g.kind].id != "travel");
        s.settle_class();
        assert_eq!((s.class, s.class_before), (StreetClass::Pedestrian, Some(StreetClass::Arterial)));
        s.segments.push(Segment::new(9, kind("travel"), 3300));
        s.settle_class();
        assert_eq!((s.class, s.class_before), (StreetClass::Arterial, None));
        let mut given = street_of(StreetClass::Pedestrian, &["sidewalk", "sidewalk"]);
        given.segments.push(Segment::new(9, kind("parking"), 2400));
        given.settle_class();
        assert_eq!((given.class, given.class_before), (StreetClass::Local, None));
        let before = s.clone();
        s.settle_class();
        assert_eq!(s, before, "settling a settled street changes nothing");
    }

    #[test]
    fn a_piece_that_is_parking_at_some_hour_is_motor_and_a_bike_lane_or_a_median_is_not() {
        let mut timed = Segment::new(1, kind("bike"), 1800);
        assert!(!timed.has_motor());
        timed.variants.push(Variant { kind: kind("parking"), material: 0, direction: None, from_min: 420, to_min: 540, days: None });
        assert!(timed.has_motor());
        for id in ["travel", "parking", "loading", "bus", "shoulder"] {
            assert!(Segment::new(1, kind(id), 3000).has_motor(), "{id}");
        }
        for id in ["bike", "sidewalk", "median", "buffer"] {
            assert!(!Segment::new(1, kind(id), 2000).has_motor(), "{id}");
        }
    }

    #[test]
    fn the_neutral_buffer_is_the_last_kind_so_saved_indices_hold_and_is_no_motor_roadway_or_green_piece() {
        let buffer = kind("buffer");
        assert_eq!(buffer, KINDS.len() - 1, "appended, so a stored kind index keeps its meaning");
        assert_eq!(buffer, 17);
        let k = &KINDS[buffer];
        assert_eq!((k.mark, k.people_per_hour_per_m, k.direction), ("BF", 0, DirectionRule::None));
        assert_eq!(k.mode, Mode::Neutral);
        assert!(k.min_mm <= k.default_mm && k.default_mm <= k.max_mm);
        assert!(k.materials.iter().all(|&m| ["concrete", "asphalt"].contains(&MATERIALS[m].id)));
        assert!(!crate::shared::catalogue::is_motor(buffer) && !crate::shared::catalogue::is_roadway(buffer));
        assert!(!PEDESTRIAN_KINDS.contains(&"buffer"));
    }

    #[test]
    fn a_buffer_is_the_space_of_no_travel_mode_so_the_shares_of_the_modes_do_not_change_and_the_buffer_has_its_own() {
        let share = |s: &Street| {
            let v = Editor::from_street(s, s, 0).view();
            v.outcomes.share.iter().map(|x| (x.mode, x.mm)).collect::<Vec<_>>()
        };
        let without = street_of(StreetClass::Local, &["sidewalk", "travel", "sidewalk"]);
        let with = street_of(StreetClass::Local, &["sidewalk", "buffer", "travel", "sidewalk"]);
        let (a, b) = (share(&without), share(&with));
        assert_eq!(a[..Mode::ALL.len()], b[..Mode::ALL.len()], "no mode takes the buffer's width");
        assert_eq!(a.last(), Some(&(Mode::Neutral, 0)));
        assert_eq!(b.last(), Some(&(Mode::Neutral, KINDS[kind("buffer")].default_mm)), "and the buffer is counted on its own");
        assert_eq!(KINDS[kind("buffer")].mode, Mode::Neutral);
        assert!(!Mode::ALL.contains(&Mode::Neutral));
        let v = Editor::from_street(&with, &with, 0).view();
        let pct: i32 = v.outcomes.share.iter().map(|s| s.pct).sum();
        assert!((99..=101).contains(&pct), "the shares sum to the street: {pct}");
    }

    #[test]
    fn a_street_of_sidewalks_and_a_buffer_is_pedestrian_and_a_motor_piece_beside_it_is_not() {
        let walk = street_of(StreetClass::Local, &["sidewalk", "buffer", "sidewalk"]);
        assert_eq!(walk.class, StreetClass::Pedestrian);
        let mut e = Editor::from_street(&walk, &walk, 0);
        assert_eq!(e.view().segments[1].curb, None);
        e.add(kind("travel"), 2);
        assert_eq!(e.view().class, StreetClass::Local);
    }

    #[test]
    fn a_bike_only_street_is_pedestrian_and_a_bus_only_or_shoulder_only_street_is_not() {
        // The definition is "no motor piece" (travel, parking, loading, bus, shoulder): this pins it, so that a change is a decision.
        let bikes = street_of(StreetClass::Local, &["sidewalk", "bike", "sidewalk"]);
        assert_eq!(bikes.class, StreetClass::Pedestrian);
        for id in ["bus", "shoulder"] {
            let s = street_of(StreetClass::Collector, &["sidewalk", "bike", id]);
            assert_eq!((s.class, s.class_before), (StreetClass::Collector, None), "{id}");
        }
        // and a pedestrian street given a bus lane or a shoulder gets its class back
        let mut e = Editor::from_street(&bikes, &bikes, 0);
        e.add(kind("bus"), 1);
        assert_eq!((e.view().class, e.snapshot().class_before), (StreetClass::Local, None));
        assert!(e.undo());
        assert_eq!(e.view().class, StreetClass::Pedestrian);
    }

    #[test]
    fn class_before_is_written_only_when_there_is_one_and_comes_back() {
        let plain = street_of(StreetClass::Local, &["sidewalk", "travel", "sidewalk"]);
        assert!(!serde_json::to_string(&plain).unwrap().contains("class_before"));
        let walkway = street_of(StreetClass::Local, &["sidewalk", "sidewalk"]);
        let json = serde_json::to_string(&walkway).unwrap();
        assert!(json.contains("\"class\":\"pedestrian\"") && json.contains("\"class_before\":\"local\""), "{json}");
        assert_eq!(serde_json::from_str::<Street>(&json).unwrap(), walkway);
        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        value.as_object_mut().unwrap().remove("class_before");
        assert_eq!(serde_json::from_value::<Street>(value).unwrap().class_before, None, "a record without the field reads");
    }

    #[test]
    fn an_unnamed_pedestrian_street_is_titled_by_its_class_in_each_language() {
        use crate::shared::i18n::Locale;
        use crate::shared::said::say_now;
        use crate::shared::units::Units;
        let s = street_of(StreetClass::Local, &["sidewalk", "sidewalk"]);
        let say = |locale| say_now(&crate::i18n_for(locale), Units::Metres, &s.title());
        assert_eq!((say(Locale::En), say(Locale::FrCa)), ("Unnamed pedestrian street".to_string(), "Rue piétonne sans nom".to_string()));
    }

    #[test]
    fn the_class_follows_the_pieces_through_edits_undo_redo_and_start_over() {
        let s = street_of(StreetClass::Arterial, &["sidewalk", "travel", "sidewalk"]);
        let mut e = Editor::from_street(&s, &s, 0);
        let lane = e.view().segments[1].uid;
        assert!(e.remove(lane));
        assert_eq!((e.view().class, e.snapshot().class_before), (StreetClass::Pedestrian, Some(StreetClass::Arterial)));
        assert!(e.undo());
        assert_eq!((e.view().class, e.snapshot().class_before), (StreetClass::Arterial, None));
        assert!(e.redo());
        assert_eq!(e.view().class, StreetClass::Pedestrian);
        e.add(kind("travel"), 1);
        assert_eq!((e.view().class, e.snapshot().class_before), (StreetClass::Arterial, None), "a lane given back restores the class");
        assert!(e.undo() && e.undo());
        assert_eq!(e.view().class, StreetClass::Arterial);
        // Start over returns to how the street was first laid out, class included
        let lane = e.view().segments[1].uid;
        assert!(e.remove(lane));
        assert!(e.reset());
        assert_eq!((e.view().class, e.snapshot().class_before), (StreetClass::Arterial, None));
    }

    #[test]
    fn undoing_a_lane_given_to_an_imported_pedestrian_street_restores_it_exactly() {
        let s = street_of(StreetClass::Pedestrian, &["sidewalk", "sidewalk"]);
        let mut e = Editor::from_street(&s, &s, 0);
        e.add(kind("travel"), 1);
        assert_eq!((e.view().class, e.snapshot().class_before), (StreetClass::Local, None));
        assert!(e.undo());
        assert_eq!((e.view().class, e.snapshot().class_before), (StreetClass::Pedestrian, None), "not Some(Local): it never had one");
    }

    #[test]
    fn an_editor_on_a_street_the_city_holds_with_the_motor_pieces_taken_out_is_pedestrian_from_the_start() {
        let today = street_of(StreetClass::Collector, &["sidewalk", "travel", "sidewalk"]);
        let mut now = today.clone();
        now.segments.retain(|g| KINDS[g.kind].id != "travel");
        now.settle_class();
        let e = Editor::from_street(&today, &now, 0);
        assert_eq!(e.view().class, StreetClass::Pedestrian);
        assert_eq!(e.snapshot(), now);
        assert_eq!(e.view().revisions.len(), 1, "the earlier changes show as one revision");
    }

    #[test]
    fn a_resize_drag_does_not_change_the_class() {
        let s = street_of(StreetClass::Local, &["sidewalk", "sidewalk", "planting"]);
        let mut e = Editor::from_street(&s, &s, 0);
        e.begin_gesture();
        assert!(e.resize_boundary(0, 200));
        assert!(e.end_gesture());
        assert_eq!((e.view().class, e.snapshot().class_before), (StreetClass::Pedestrian, Some(StreetClass::Local)));
    }

    #[test]
    fn pedestrianised_turns_every_motor_piece_into_a_sidewalk_of_its_width_and_settles_the_class() {
        let s = Street::sample(1, Side::Right);
        let p = fixtures::pedestrianised(&s);
        assert_eq!(p.segments.len(), s.segments.len());
        for (was, now) in s.segments.iter().zip(&p.segments) {
            assert_eq!(now.width_mm, was.width_mm);
            if was.has_motor() {
                assert_eq!((KINDS[now.kind].id, now.variants.len()), ("sidewalk", 0));
            } else {
                assert_eq!(now.kind, was.kind);
            }
        }
        assert!(!p.has_motor());
        assert_eq!((p.class, p.class_before), (StreetClass::Pedestrian, Some(StreetClass::Arterial)));
    }

    #[test]
    fn every_lane_measure_can_be_arranged_and_is_then_recognised() {
        for (sample, region) in [(1, 0), (1, 3), (0, 0)] {
            for d in measures::DEFS.iter().filter(|d| d.freeway == Some(false)) {
                let mut e = Editor::new(sample);
                e.set_region(region);
                let view = e.view();
                let m = view.measures.iter().find(|m| m.code == d.code).unwrap();
                if !m.can_apply {
                    continue;
                }
                assert!(e.apply_measure(d.code), "{} on {}", d.code, SAMPLES[sample].name);
                let v = e.view();
                assert!(v.delta_mm <= 0, "{} fits: {} over", d.code, v.delta_mm);
                let found = v.measures.iter().find(|m| m.code == d.code).unwrap();
                assert!(
                    found.present,
                    "{} is recognised on {} (region {region}); got {:?}",
                    d.code,
                    SAMPLES[sample].name,
                    v.measures.iter().filter(|m| m.present).map(|m| m.code).collect::<Vec<_>>()
                );
                // A narrow street squeezes the bus lanes; that is the one thing it may be told off for.
                assert!(found.problems.iter().all(|p| matches!(p, measures::Problem::NarrowTransitLane { .. })), "{}: {:?}", d.code, found.problems);
                assert!(says(&v.revisions.last().unwrap().label).starts_with(d.code));
            }
        }
    }

    #[test]
    fn the_avenue_can_take_every_street_measure() {
        let e = Editor::new(1);
        let applicable: Vec<&str> = e.view().measures.iter().filter(|m| m.can_apply).map(|m| m.code).collect();
        for c in ["A1", "A2", "A3", "B1", "B3", "B4", "C1", "D1", "E1", "E2", "F1", "F2"] {
            assert!(applicable.contains(&c), "{c} on the avenue; applicable: {applicable:?}");
        }
        assert!(!applicable.contains(&"B2") && !applicable.contains(&"E3"), "freeway only");
    }

    #[test]
    fn an_other_times_type_has_its_own_direction() {
        let mut e = Editor::new(1);
        assert!(e.apply_measure("B3"));
        let bus = e.view().segments.iter().find(|s| s.kind == kind("bus")).unwrap().uid;
        assert!(e.view().measures.iter().find(|m| m.code == "B3").unwrap().present);
        // Make the afternoon window run the same way as the rest: it is no longer alternating.
        let base = e.view().segments.iter().find(|s| s.uid == bus).unwrap().direction.map(|d| if d == "away" { 0 } else { 1 });
        assert!(e.set_variant_direction(bus, 0, base));
        assert!(!e.view().measures.iter().find(|m| m.code == "B3").unwrap().present);
        assert!(!e.set_variant_direction(bus, 0, base), "unchanged");
        assert!(!e.set_variant_direction(bus, 5, Some(0)));
    }

    #[test]
    fn the_freeway_takes_only_its_own_measures() {
        let mut e = Editor::new(3);
        let applicable: Vec<&str> = e.view().measures.iter().filter(|m| m.can_apply).map(|m| m.code).collect();
        assert_eq!(applicable, vec!["B2", "E3"]);
        assert!(e.apply_measure("B2"));
        let v = e.view();
        assert!(v.measures.iter().find(|m| m.code == "B2").unwrap().present);
        assert!(v.delta_mm <= 0);
        assert!(e.apply_measure("E3"));
        assert!(e.view().measures.iter().find(|m| m.code == "E3").unwrap().present);
        assert!(!e.apply_measure("A1"), "not for a freeway");
    }

    #[test]
    fn a_narrow_bus_lane_is_a_problem() {
        let mut e = Editor::new(1);
        assert!(e.apply_measure("B1"));
        let bus = e.view().segments.iter().find(|s| s.kind == kind("bus")).unwrap().uid;
        let _ = e.set_width(bus, 3000);
        let m = e.view().measures.into_iter().find(|m| m.code == "B1").unwrap();
        assert!(m.present && !m.problems.is_empty());
    }

    #[test]
    fn a_street_is_titled_by_its_name_or_else_by_its_class_in_each_language() {
        use crate::shared::i18n::Locale;
        use crate::shared::said::say_now;
        use crate::shared::units::Units;
        let say = |locale, s: &Street| say_now(&crate::i18n_for(locale), Units::Metres, &s.title());
        let local = Street::imported(StreetClass::Local, Side::Right, &[travel_lane()]);
        assert_eq!((say(Locale::En, &local), say(Locale::FrCa, &local)), ("Unnamed local street".into(), "Rue locale sans nom".into()));
        let motorway = Street::imported(StreetClass::Motorway, Side::Right, &[travel_lane()]);
        assert_eq!(motorway.title(), Said::new("city-unnamed-motorway"));
        assert_eq!((say(Locale::En, &motorway), say(Locale::FrCa, &motorway)), ("Unnamed motorway".into(), "Autoroute sans nom".into()));
        let named = local.named("Rue Rachel Est");
        assert_eq!(named.title(), Said::new("city-name").with("name", Arg::Text("Rue Rachel Est".into())));
        assert_eq!((say(Locale::En, &named), say(Locale::FrCa, &named)), ("Rue Rachel Est".into(), "Rue Rachel Est".into()));
    }

    #[test]
    fn a_street_is_what_it_is_given_and_has_no_template_to_fall_back_on() {
        let s = Street::imported(StreetClass::Local, Side::Right, &[travel_lane()]);
        assert_eq!(s.title(), Said::new("city-unnamed").with("class", Arg::Msg("class-local")));
        assert!(!s.class.is_freeway());
        assert!(Street::imported(StreetClass::Motorway, Side::Right, &[travel_lane()]).class.is_freeway());
    }

    #[test]
    fn samples_fill_their_right_of_way_exactly() {
        for (i, s) in SAMPLES.iter().enumerate() {
            let e = Editor::new(i);
            assert_eq!(total(e.current()), s.row_mm, "{}", s.name);
        }
    }

    #[test]
    fn samples_respect_catalogue_ranges() {
        for s in SAMPLES.iter() {
            for (id, w) in s.segments {
                let k = &KINDS[kind(id)];
                assert!((k.min_mm..=k.max_mm).contains(w), "{} {}", s.name, id);
            }
        }
    }

    #[test]
    fn add_uses_remaining_width_and_is_selected() {
        let mut e = Editor::new(0);
        e.remove(e.current()[1].uid); // drop a parking lane, 2400 free
        let uid = e.add(kind("bike"), 1);
        let s = e.current().iter().find(|s| s.uid == uid).unwrap();
        assert_eq!(s.width_mm, 1800);
        assert_eq!(e.view().selected, Some(uid));
        assert_eq!(e.view().delta_mm, -600);
    }

    #[test]
    fn add_over_width_uses_default_and_reports_overage() {
        let mut e = Editor::new(0);
        e.add(kind("bike"), 0);
        let v = e.view();
        assert_eq!(v.delta_mm, 1800);
        assert!(!v.checks[0].ok);
        assert_eq!(v.checks[0].detail, Said::new("check-fits-over").with("amount", Arg::Length(1800)));
        assert_eq!(says(&v.checks[0].detail), "1.8 m too wide. Narrow or remove a piece.");
    }

    #[test]
    fn add_clamps_to_partial_remaining_width() {
        let mut e = Editor::new(0);
        e.remove(e.current()[1].uid); // 2400 free
        e.add(kind("travel"), 0); // remaining 2400 < min 2700, so default
        assert_eq!(e.current()[0].width_mm, 3300);
        let mut e = Editor::new(0);
        e.remove(e.current()[0].uid); // 3300 free
        e.remove(e.current()[0].uid); // 5700 free
        e.add(kind("planting"), 0);
        assert_eq!(e.current()[0].width_mm, 1800);
    }

    #[test]
    fn move_to_reorders_and_undo_restores() {
        let mut e = Editor::new(0);
        let before = ids(&e);
        let uid = e.current()[0].uid;
        assert!(e.move_to(uid, 3));
        assert_ne!(ids(&e), before);
        assert!(e.undo());
        assert_eq!(ids(&e), before);
        assert!(e.redo());
        assert_eq!(e.current()[3].uid, uid);
    }

    #[test]
    fn move_to_same_place_is_not_a_revision() {
        let mut e = Editor::new(0);
        let uid = e.current()[2].uid;
        assert!(!e.move_to(uid, 2));
        assert!(!e.view().can_undo);
    }

    #[test]
    fn set_width_snaps_to_ten_and_clamps() {
        let mut e = Editor::new(0);
        let uid = e.current()[2].uid; // travel lane
        e.set_width(uid, 3347);
        assert_eq!(e.current()[2].width_mm, 3350);
        e.set_width(uid, 100);
        assert_eq!(e.current()[2].width_mm, 2700);
        e.set_width(uid, 99999);
        assert_eq!(e.current()[2].width_mm, 4000);
    }

    #[test]
    fn boundary_drag_conserves_total_and_respects_limits() {
        let mut e = Editor::new(0);
        let total_before = total(e.current());
        e.begin_gesture();
        assert!(e.resize_boundary(2, 400)); // travel | travel
        assert_eq!(total(e.current()), total_before);
        assert_eq!(e.current()[2].width_mm, 3700);
        assert_eq!(e.current()[3].width_mm, 2900);
        // A huge drag stops at the neighbour's minimum.
        e.resize_boundary(2, 5000);
        assert_eq!(e.current()[3].width_mm, 2700);
        assert_eq!(e.current()[2].width_mm, 3900);
        assert!(e.end_gesture());
        assert_eq!(e.view().revisions.len(), 1);
    }

    #[test]
    fn boundary_drag_is_relative_to_gesture_start() {
        let mut e = Editor::new(0);
        e.begin_gesture();
        e.resize_boundary(2, 300);
        e.resize_boundary(2, 100);
        assert_eq!(e.current()[2].width_mm, 3400);
        e.cancel_gesture();
        assert_eq!(e.current()[2].width_mm, 3300);
        assert!(!e.view().can_undo);
    }

    #[test]
    fn a_gesture_that_returns_to_start_records_nothing() {
        let mut e = Editor::new(0);
        e.begin_gesture();
        e.resize_boundary(2, 300);
        e.resize_boundary(2, 0);
        assert!(!e.end_gesture());
        assert!(!e.view().can_undo);
    }

    #[test]
    fn edge_resize_changes_only_one_segment() {
        let mut e = Editor::new(0);
        let uid = e.current()[5].uid;
        e.begin_gesture();
        assert!(e.resize_edge(uid, -1000));
        e.end_gesture();
        assert_eq!(e.current()[5].width_mm, 2300);
        assert_eq!(e.view().delta_mm, -1000);
    }

    #[test]
    fn drop_index_uses_segment_midpoints() {
        let e = Editor::new(0); // 3300 2400 3300 3300 2400 3300
        assert_eq!(e.drop_index(0.0, 0), 0);
        assert_eq!(e.drop_index(1000.0, 0), 0);
        assert_eq!(e.drop_index(1700.0, 0), 1);
        assert_eq!(e.drop_index(17999.0, 0), 6);
        // Excluding the first segment shifts every midpoint left.
        let first = e.current()[0].uid;
        assert_eq!(e.drop_index(1000.0, first), 0);
        assert_eq!(e.drop_index(1300.0, first), 1);
    }

    #[test]
    fn new_edit_after_undo_discards_redo() {
        let mut e = Editor::new(0);
        e.add(kind("bike"), 0);
        e.undo();
        assert!(e.view().can_redo);
        e.add(kind("bus"), 0);
        assert!(!e.view().can_redo);
        assert_eq!(e.view().revisions.len(), 1);
    }

    #[test]
    fn reset_is_a_revision_and_undoable() {
        let mut e = Editor::new(0);
        assert!(!e.reset());
        e.add(kind("bike"), 0);
        assert!(e.reset());
        assert!(!e.view().changed);
        assert_eq!(e.view().revisions.len(), 2);
        assert!(e.undo());
        assert!(e.view().changed);
    }

    #[test]
    fn removing_selected_selects_a_neighbour() {
        let mut e = Editor::new(0);
        let uid = e.current()[2].uid;
        let next = e.current()[3].uid;
        e.select(Some(uid));
        e.remove(uid);
        assert_eq!(e.view().selected, Some(next));
    }

    #[test]
    fn selection_moves_and_clamps() {
        let mut e = Editor::new(0);
        e.select_relative(1);
        assert_eq!(e.view().selected, Some(e.current()[0].uid));
        e.select_relative(-5);
        assert_eq!(e.view().selected, Some(e.current()[0].uid));
        e.select_relative(99);
        assert_eq!(e.view().selected, Some(e.current().last().unwrap().uid));
    }

    #[test]
    fn checks_report_edges_and_access() {
        let mut e = Editor::new(0);
        assert!(e.view().checks.iter().all(|c| c.ok));
        e.move_to(e.current()[0].uid, 2);
        assert!(!e.view().checks[1].ok);
    }

    #[test]
    fn outcome_shares_sum_to_about_one_hundred() {
        let e = Editor::new(1);
        let sum: i32 = e.view().outcomes.share.iter().map(|s| s.pct).sum();
        assert!((98..=102).contains(&sum), "{sum}");
    }

    #[test]
    fn loading_a_sample_clears_history() {
        let mut e = Editor::new(0);
        e.add(kind("bike"), 0);
        e.load_sample(2);
        assert!(!e.view().can_undo);
        assert_eq!(e.view().row_mm, 12000);
    }

    #[test]
    fn new_pieces_get_their_kinds_defaults() {
        let e = Editor::new(0);
        for s in e.current() {
            let k = &KINDS[s.kind];
            assert_eq!(s.material, k.materials[0], "{}", k.id);
            assert_eq!(s.curb.is_some(), k.has_curb, "{}", k.id);
        }
        let mut e = Editor::new(0);
        let uid = e.add(kind("bike"), 0);
        let s = e.current().iter().find(|s| s.uid == uid).unwrap();
        assert_eq!(s.curb, Some(DEFAULT_CURB));
    }

    #[test]
    fn every_kinds_materials_are_valid_and_start_with_a_default() {
        for k in KINDS.iter() {
            assert!(!k.materials.is_empty(), "{}", k.id);
            assert!(k.materials.iter().all(|&m| m < MATERIALS.len()), "{}", k.id);
        }
        assert!(DEFAULT_CURB < CURBS.len());
    }

    #[test]
    fn set_material_accepts_allowed_and_refuses_others() {
        let mut e = Editor::new(0);
        let parking = e.current()[1].uid;
        let permeable = MATERIALS.iter().position(|m| m.id == "permeable").unwrap();
        let grass = MATERIALS.iter().position(|m| m.id == "grass").unwrap();
        assert!(!e.set_material(parking, grass)); // not a parking surface
        assert!(!e.view().can_undo);
        assert!(e.set_material(parking, permeable));
        assert_eq!(e.view().segments[1].material, "permeable");
        assert_eq!(says(&e.view().revisions[0].label), "Parking surface: permeable paving");
        assert!(!e.set_material(parking, permeable)); // unchanged
        assert!(!e.set_material(9999, permeable)); // unknown piece
    }

    #[test]
    fn a_planting_strip_starts_with_street_trees_and_can_take_the_others() {
        let mut e = Editor::new(0);
        let uid = e.add(kind("planting"), 0);
        fn id(e: &Editor, uid: u32) -> &'static str {
            e.view().segments.iter().find(|s| s.uid == uid).unwrap().material
        }
        assert_eq!(id(&e, uid), "trees");
        let grass = MATERIALS.iter().position(|m| m.id == "grass").unwrap();
        let trees = MATERIALS.iter().position(|m| m.id == "trees").unwrap();
        assert!(e.set_material(uid, grass));
        assert_eq!(id(&e, uid), "grass");
        assert!(e.set_material(uid, trees));
        let walk = e.current()[1].uid;
        assert!(!e.set_material(walk, trees)); // trees are for planting strips only
    }

    #[test]
    fn street_furniture_is_added_like_any_piece_and_follows_the_older_kinds() {
        let shoulder = kind_index("shoulder").unwrap();
        let mut e = Editor::new(0);
        for (n, id) in ["bikerack", "bikeshare", "pole", "busshelter", "busstation", "bench", "terrace", "streetlamp"].into_iter().enumerate() {
            let k = kind_index(id).unwrap();
            assert_eq!(k, shoulder + 1 + n, "{id} is appended, so stored kind indices hold");
            let before = e.current().len();
            let uid = e.add(k, 1);
            assert_eq!(e.current().len(), before + 1);
            let s = e.current().iter().find(|s| s.uid == uid).unwrap();
            assert_eq!(s.width_mm, KINDS[k].default_mm);
            assert!(e.remove(uid));
        }
        // None of them counts as the sidewalk the edges check asks for.
        let rack = kind_index("bikerack").unwrap();
        let mut e = Editor::new(0);
        e.add(rack, 0);
        assert!(!e.view().checks[1].ok);
    }

    #[test]
    fn material_changes_undo_and_redo() {
        let mut e = Editor::new(0);
        let lane = e.current()[2].uid;
        assert!(e.set_material(lane, 1)); // concrete
        assert!(e.undo());
        assert_eq!(e.view().segments[2].material, "asphalt");
        assert!(e.redo());
        assert_eq!(e.view().segments[2].material, "concrete");
    }

    #[test]
    fn driving_lanes_must_have_a_direction_and_bike_lanes_may() {
        let mut e = Editor::new(0);
        let dirs: Vec<_> = e.view().segments.iter().filter(|s| s.kind == kind_index("travel").unwrap()).map(|s| s.direction).collect();
        assert_eq!(dirs, [Some("toward"), Some("away")]); // traffic keeps right
        let lane = e.current().iter().find(|s| s.kind == kind_index("travel").unwrap()).unwrap().uid;
        assert!(!e.set_direction(lane, None)); // a driving lane cannot be undirected
        assert!(!e.set_direction(lane, Some(9)));
        assert!(!e.set_direction(lane, Some(1))); // unchanged
        assert!(e.set_direction(lane, Some(0)));
        assert_eq!(says(&e.view().revisions[0].label), "Driving lane direction: away from you");
        assert!(e.undo());
        let walk = e.current()[0].uid;
        assert!(!e.set_direction(walk, Some(0))); // sidewalks have none
        let bike = e.add(kind_index("bike").unwrap(), 1);
        assert_eq!(e.view().segments[1].direction, None);
        assert!(e.set_direction(bike, Some(0)));
        assert!(e.set_direction(bike, None));
        assert_eq!(says(&e.view().revisions.last().unwrap().label), "Bike lane direction: two-way");
        assert!(e.undo());
        assert_eq!(e.view().segments[1].direction, Some("away"));
    }

    #[test]
    fn a_region_sets_which_side_traffic_keeps_to() {
        let dirs =
            |e: &Editor| -> Vec<_> { e.view().segments.iter().filter(|s| s.direction.is_some() && s.kind == kind("travel")).map(|s| s.direction).collect() };
        let mut e = Editor::new(0);
        assert_eq!(e.view().region, "canada");
        assert!(e.view().checks.iter().find(|c| c.id == "side").unwrap().ok);
        let uk = REGIONS.iter().position(|r| r.id == "united-kingdom").unwrap();
        assert!(!e.set_region(99));
        assert!(!e.set_region(0)); // unchanged
        // an untouched street is laid out again for the new side
        assert!(e.set_region(uk));
        assert_eq!(dirs(&e), [Some("away"), Some("toward")]);
        assert!(e.view().checks.iter().find(|c| c.id == "side").unwrap().ok);
        assert!(!e.view().can_undo);
        // once edited, a change of side mirrors the lanes as one undoable change
        e.add(kind("bike"), 0);
        assert!(e.set_region(0));
        assert_eq!(dirs(&e), [Some("toward"), Some("away")]);
        assert_eq!(says(&e.view().revisions.last().unwrap().label), "Traffic keeps right");
        assert!(e.view().checks.iter().find(|c| c.id == "side").unwrap().ok);
        // a region on the same side leaves the lanes alone
        let us = REGIONS.iter().position(|r| r.id == "united-states").unwrap();
        let revs = e.view().revisions.len();
        assert!(e.set_region(us));
        assert_eq!(e.view().revisions.len(), revs);
        assert_eq!(dirs(&e), [Some("toward"), Some("away")]);
        // undo puts the lanes back, so they now run against the region's side
        assert!(e.undo());
        assert_eq!(dirs(&e), [Some("away"), Some("toward")]);
        let side = e.view().checks.into_iter().find(|c| c.id == "side").unwrap();
        assert!(!side.ok);
        assert_eq!(says(&side.label), "Traffic keeps right");
        assert!(e.redo());
        // a one-way street has nothing to conflict
        let lanes: Vec<u32> = e.current().iter().filter(|s| s.kind == kind("travel")).map(|s| s.uid).collect();
        assert!(e.set_direction(lanes[0], Some(0)));
        assert!(e.view().checks.iter().find(|c| c.id == "side").unwrap().ok);
        // starting over keeps the region
        e.load_sample(0);
        assert_eq!(e.view().region, "united-states");
    }

    #[test]
    fn a_piece_can_take_a_different_type_at_certain_times() {
        let mut e = Editor::new(0);
        let parking = e.current()[1].uid;
        let bus = kind("bus");
        let base_pph = e.view().outcomes.capacity_pph;
        assert!(e.add_variant(parking));
        assert_eq!(says(&e.view().revisions[0].label), "Parking is transit lane 07:00-10:00");
        assert_eq!(e.view().segments[1].width_mm, 3000); // a bus lane needs 3.0 m
        // shown at midday it is still parking; at 08:00 it is a bus lane
        assert_eq!(e.view().segments[1].kind, kind("parking"));
        assert_eq!(e.view().outcomes.capacity_pph, base_pph);
        assert!(e.set_time(8 * 60 + 7)); // snaps to a quarter hour
        assert_eq!(e.view().time_min, 8 * 60);
        assert_eq!(e.view().segments[1].kind, bus);
        assert_eq!(e.view().segments[1].base_kind, kind("parking"));
        assert_eq!(e.view().segments[1].active_variant, Some(0));
        assert!(e.view().outcomes.capacity_pph > base_pph);
        assert!(e.set_time(10 * 60)); // the window does not include its end
        assert_eq!(e.view().segments[1].kind, kind("parking"));
        // the time is a setting: no revision, and it survives undo
        assert_eq!(e.view().revisions.len(), 1);
        assert!(e.undo());
        assert!(e.view().segments[1].variants.is_empty());
        assert!(e.redo());
    }

    #[test]
    fn variant_windows_are_quarter_hours_and_never_overlap() {
        let mut e = Editor::new(0);
        let parking = e.current()[1].uid;
        assert!(e.add_variant(parking)); // 07:00-10:00
        assert!(e.add_variant(parking)); // takes the next free window, 16:00-19:00
        let w = |e: &Editor| -> Vec<_> { e.view().segments[1].variants.iter().map(|v| (v.from_min, v.to_min)).collect() };
        assert_eq!(w(&e), [(420, 600), (960, 1140)]);
        assert!(!e.set_variant_time(parking, 1, 500, 700)); // overlaps the first
        assert!(!e.set_variant_time(parking, 1, 601, 700)); // not a quarter hour
        assert!(!e.set_variant_time(parking, 1, 700, 700)); // empty
        assert!(!e.set_variant_time(parking, 5, 700, 800)); // no such window
        assert!(e.set_variant_time(parking, 1, 22 * 60, 5 * 60)); // past midnight
        e.set_time(23 * 60);
        assert_eq!(e.view().segments[1].active_variant, Some(1));
        e.set_time(2 * 60);
        assert_eq!(e.view().segments[1].active_variant, Some(1));
        e.set_time(12 * 60);
        assert_eq!(e.view().segments[1].active_variant, None);
        assert!(e.remove_variant(parking, 1));
        assert!(!e.remove_variant(parking, 1));
    }

    #[test]
    fn only_roadway_pieces_change_type_and_the_width_must_suit_every_type() {
        let mut e = Editor::new(0);
        assert!(!e.add_variant(e.current()[0].uid)); // a sidewalk is not roadway
        let parking = e.current()[1].uid;
        assert!(e.add_variant(parking));
        let alts = e.view().segments[1].alt_kinds.clone();
        assert!(!alts.contains(&kind("sidewalk")) && !alts.contains(&kind("parking")));
        assert!(!e.set_variant_kind(parking, 0, kind("sidewalk")));
        assert!(!e.set_variant_kind(parking, 0, kind("bus"))); // unchanged
        // parking (2.1 to 3.0 m) as a bus lane (3.0 m or more) is 3.0 m wide
        assert_eq!(e.view().segments[1].width_mm, 3000);
        assert_eq!((e.view().segments[1].min_mm, e.view().segments[1].max_mm), (3000, 3000));
        e.set_width(parking, 99_000);
        assert_eq!(e.view().segments[1].width_mm, 3000);
        // a type that shares no width with the others is not offered
        assert!(!e.view().segments[1].alt_kinds.contains(&kind("bike")) || KINDS[kind("bike")].max_mm >= 3000);
        assert!(e.set_variant_kind(parking, 0, kind("loading")));
        assert_eq!(e.view().segments[1].width_mm, 3000);
    }

    #[test]
    fn a_bike_lane_is_a_bike_lane_at_every_hour() {
        let mut e = Editor::from_street(
            &street_of(StreetClass::Local, &["sidewalk", "travel", "bike", "parking", "sidewalk"]),
            &street_of(StreetClass::Local, &["sidewalk", "travel", "bike", "parking", "sidewalk"]),
            0,
        );
        let (bike, parking) = (e.current()[2].uid, e.current()[3].uid);
        // a bike lane takes no window
        assert!(e.view().segments[2].alt_kinds.is_empty());
        assert!(!e.add_variant(bike));
        // no piece is offered, or accepts, bike as another time's type
        assert!(!e.view().segments[3].alt_kinds.contains(&kind("bike")));
        assert!(e.add_variant(parking));
        assert!(!e.set_variant_kind(parking, 0, kind("bike")));
        assert!(!e.view().segments[3].variants.iter().any(|v| v.kind == kind("bike")));
    }

    #[test]
    fn a_saved_street_with_a_timed_bike_lane_still_loads_and_the_windows_can_be_removed() {
        let mut s = street_of(StreetClass::Local, &["sidewalk", "travel", "bike", "parking", "sidewalk"]);
        let win = |k: &str| Variant { kind: kind(k), material: KINDS[kind(k)].materials[0], direction: None, from_min: 420, to_min: 540, days: None };
        s.segments[2].variants.push(win("parking")); // a bike lane that is parking at some hours
        s.segments[3].variants.push(win("bike")); // parking that is a bike lane at some hours
        assert!(s.is_sound());
        let mut e = Editor::from_street(&s, &s, 0);
        let v = e.view();
        assert_eq!((v.segments[2].variants.len(), v.segments[3].variants.len()), (1, 1));
        let (bike, parking) = (e.current()[2].uid, e.current()[3].uid);
        assert!(!e.add_variant(bike));
        assert!(e.remove_variant(bike, 0));
        assert!(e.remove_variant(parking, 0));
        assert!(e.view().segments.iter().all(|g| g.variants.is_empty()));
    }

    #[test]
    fn surface_edits_go_to_whichever_type_is_shown() {
        let mut e = Editor::new(0);
        let parking = e.current()[1].uid;
        assert!(e.add_variant(parking));
        let concrete = MATERIALS.iter().position(|m| m.id == "concrete").unwrap();
        e.set_time(8 * 60);
        assert!(e.set_material(parking, concrete)); // the bus lane's surface
        assert_eq!(e.view().segments[1].material, "concrete");
        e.set_time(12 * 60);
        assert_eq!(e.view().segments[1].material, "asphalt"); // parking is untouched
    }

    #[test]
    fn a_bike_lane_may_have_a_bus_boarding_island() {
        let mut e = Editor::new(0);
        let bike = e.add(kind("bike"), 1);
        let island = CURBS.iter().position(|c| c.id == "island").unwrap();
        assert!(e.set_curb(bike, Some(island)));
        assert_eq!(e.view().segments[1].curb, Some("island"));
        assert_eq!(says(&e.view().revisions.last().unwrap().label), "Bike lane curb: bus boarding island");
        for k in KINDS.iter().filter(|k| k.has_curb) {
            assert!(!k.curbs.is_empty() && k.curbs.iter().all(|&c| c < CURBS.len()), "{}", k.id);
            assert!(k.curbs.contains(&DEFAULT_CURB), "{}", k.id);
        }
        assert!(KINDS.iter().filter(|k| !k.has_curb).all(|k| k.curbs.is_empty()));
    }

    #[test]
    fn a_transit_lane_carries_buses_or_trams() {
        let mut e = Editor::new(0);
        let lane = e.current()[2].uid;
        assert!(!e.set_tram(lane, true)); // a driving lane
        let bus = e.add(kind_index("bus").unwrap(), 2);
        assert!(!e.view().segments[2].tram);
        assert!(e.set_tram(bus, true));
        assert!(e.view().segments[2].tram);
        assert_eq!(says(&e.view().revisions.last().unwrap().label), "Transit lane vehicle: tram");
        assert!(!e.set_tram(bus, true)); // unchanged
        assert!(e.undo());
        assert!(!e.view().segments[2].tram);
    }

    #[test]
    fn a_street_kept_with_the_old_sidewalk_shelter_flag_still_loads() {
        let street = Street::sample(0, Side::Right);
        let mut v: serde_json::Value = serde_json::to_value(&street).unwrap();
        for seg in v["segments"].as_array_mut().unwrap() {
            seg.as_object_mut().unwrap().insert("shelter".into(), true.into());
        }
        let back: Street = serde_json::from_value(v).unwrap();
        assert!(back.is_sound());
    }

    #[test]
    fn set_curb_only_on_kinds_that_have_one() {
        let mut e = Editor::new(0);
        let walk = e.current()[0].uid;
        let lane = e.current()[2].uid;
        assert!(!e.set_curb(lane, Some(0))); // driving lanes have no curb
        assert!(!e.set_curb(walk, Some(99))); // not in the table
        let island = CURBS.iter().position(|c| c.id == "island").unwrap();
        assert!(!e.set_curb(walk, Some(island))); // a boarding island is for bike lanes
        assert!(!e.set_curb(walk, Some(DEFAULT_CURB))); // unchanged
        assert!(e.set_curb(walk, Some(0)));
        assert_eq!(e.view().segments[0].curb, Some("granite"));
        assert_eq!(says(&e.view().revisions[0].label), "Sidewalk curb: granite");
        assert!(e.set_curb(walk, CURBS.iter().position(|c| c.id == "planted")));
        assert_eq!(e.view().segments[0].curb, Some("planted"));
        assert!(e.undo());
        assert!(e.set_curb(walk, CURBS.iter().position(|c| c.id == "bikefriendly")));
        assert_eq!(e.view().segments[0].curb, Some("bikefriendly"));
        assert!(e.undo());
        assert!(e.set_curb(walk, CURBS.iter().position(|c| c.id == "kassel")));
        assert_eq!(says(&e.view().revisions[1].label), "Sidewalk curb: bus-friendly curb");
        assert!(e.undo());
        assert!(e.set_curb(walk, None));
        assert_eq!(e.view().segments[0].curb, None);
        assert_eq!(says(&e.view().revisions[1].label), "Sidewalk curb: none");
        assert!(e.undo());
        assert_eq!(e.view().segments[0].curb, Some("granite"));
    }

    #[test]
    fn material_and_curb_count_as_changes_from_today() {
        let mut e = Editor::new(0);
        assert!(!e.view().changed);
        e.set_curb(e.current()[0].uid, None);
        assert!(e.view().changed);
    }

    #[test]
    fn a_street_survives_a_round_trip_through_json() {
        let mut e = Editor::new(1);
        let uid = e.current()[2].uid;
        assert!(e.set_width(uid, 2700));
        let street = e.snapshot();
        let back: Street = serde_json::from_str(&serde_json::to_string(&street).unwrap()).unwrap();
        assert_eq!(back, street);
        assert!(back.is_sound());
    }

    #[test]
    fn an_editor_on_a_held_street_keeps_today_and_the_earlier_changes() {
        let today = Street::sample(0, Side::Right);
        let mut e = Editor::new(0);
        let uid = e.current()[2].uid;
        assert!(e.set_width(uid, 2700));
        let now = e.snapshot();
        let mut again = Editor::from_street(&today, &now, 0);
        let v = again.view();
        assert!(v.changed);
        assert_eq!(v.revisions.len(), 1);
        assert_eq!(says(&v.revisions[0].label), "Earlier changes");
        assert_eq!(again.snapshot(), now);
        // Start over goes back to the street as first laid out, and can be undone.
        assert!(again.reset());
        assert_eq!(again.snapshot().segments, today.segments);
        assert!(again.undo());
        assert_eq!(again.snapshot().segments, now.segments);
        // A street nobody has touched has no earlier changes to show.
        assert!(!Editor::from_street(&today, &today, 0).view().changed);
    }

    #[test]
    fn a_held_street_is_mirrored_for_the_side_of_the_road() {
        let right = Street::sample(1, Side::Right);
        let left = right.for_side(Side::Left);
        assert_eq!(left.side, Side::Left);
        assert_ne!(left.segments, right.segments);
        assert_eq!(left.for_side(Side::Right), right);
        // An untouched street read at the left is the sample laid out for the left.
        assert_eq!(Street::sample(1, Side::Left), left);
        let e = Editor::from_street(&right, &right, 3);
        assert_eq!(e.snapshot(), left);
    }

    #[test]
    fn a_street_seen_from_its_other_end_turns_through_half_a_circle() {
        let s = Street::sample(1, Side::Right);
        let r = s.reversed();
        assert_eq!(r.reversed(), s);
        assert_eq!(r.segments.first().map(|g| g.kind), s.segments.last().map(|g| g.kind));
        // A lane going away from the viewer comes toward the viewer from the far end.
        let lane = |st: &Street, i: usize| st.segments.iter().filter(|g| g.direction.is_some()).nth(i).unwrap().direction;
        assert_eq!(lane(&s, 0).map(|d| 1 - d), lane(&r, 3));
    }

    #[test]
    fn a_street_with_indices_outside_the_catalogue_is_not_sound() {
        let mut s = Street::sample(0, Side::Right);
        s.segments[0].kind = 99;
        assert!(!s.is_sound());
        let mut s = Street::sample(0, Side::Right);
        s.segments[1].material = 99;
        assert!(!s.is_sound());
    }

    #[test]
    fn a_kept_tram_lane_whose_window_names_no_kind_is_not_sound_rather_than_a_panic() {
        let mut s = Street::sample(0, Side::Right);
        let travel = s.segments.iter().position(|g| KINDS[g.kind].id != "bus").unwrap();
        s.segments[travel].tram = true;
        s.segments[travel].variants = vec![Variant { kind: 9_999, material: 0, direction: None, from_min: 0, to_min: 60, days: None }];
        assert!(!s.is_sound());
    }

    /// A street of these kinds at these widths (kind id, mm), made as an import makes it.
    fn street_widths(class: StreetClass, pieces: &[(&str, i32)]) -> Street {
        let mut s = street_of(class, &pieces.iter().map(|p| p.0).collect::<Vec<_>>());
        for (g, p) in s.segments.iter_mut().zip(pieces) {
            g.width_mm = p.1;
        }
        s
    }

    #[test]
    fn making_a_street_pedestrian_turns_every_motor_piece_into_the_chosen_kind_in_one_edit_and_keeps_the_bike_lane() {
        for id in PEDESTRIAN_KINDS {
            let want = kind(id);
            let s = street_of(StreetClass::Arterial, &["sidewalk", "parking", "bus", "travel", "bike", "loading", "shoulder", "sidewalk"]);
            let mut e = Editor::from_street(&s, &s, 0);
            let before: Vec<(u32, i32)> = e.view().segments.iter().map(|g| (g.uid, g.width_mm)).collect();
            assert!(e.make_pedestrian(want), "{id}");
            let v = e.view();
            assert_eq!(ids(&e), ["sidewalk", id, id, id, "bike", id, id, "sidewalk"], "the bike lane stays");
            assert_eq!(v.segments.iter().map(|g| (g.uid, g.width_mm)).collect::<Vec<_>>(), before, "the same pieces at the same widths");
            assert!(v.segments.iter().all(|g| g.variants.is_empty() && g.direction.is_none()));
            assert_eq!(v.segments[1].curb.is_some(), KINDS[want].has_curb, "the kind's own curb");
            assert_eq!(v.segments[1].material, MATERIALS[KINDS[want].materials[0]].id, "the kind's own surface");
            assert_eq!((v.class, v.has_motor), (StreetClass::Pedestrian, false));
            assert_eq!(v.revisions.len(), 1);
            assert_eq!(says(&v.revisions[0].label), "Made pedestrian");
            assert!(e.undo());
            assert_eq!((e.view().class, e.view().has_motor), (StreetClass::Arterial, true));
            assert_eq!(ids(&e), ["sidewalk", "parking", "bus", "travel", "bike", "loading", "shoulder", "sidewalk"]);
        }
    }

    #[test]
    fn a_bike_lane_that_is_motor_at_some_hour_is_made_pedestrian_too() {
        let timed = Piece {
            kind: kind("bike"),
            width_mm: 2400,
            direction: None,
            variants: vec![Window { kind: kind("parking"), from_min: 420, to_min: 540, days: None }],
        };
        let walk = Piece { kind: kind("sidewalk"), width_mm: 3000, direction: None, variants: Vec::new() };
        let s = Street::imported(StreetClass::Local, Side::Right, &[walk.clone(), timed, walk]);
        assert!(s.has_motor());
        let mut e = Editor::from_street(&s, &s, 0);
        assert!(e.make_pedestrian(kind("planting")));
        assert_eq!(ids(&e), ["sidewalk", "planting", "sidewalk"]);
        assert!(e.view().segments.iter().all(|g| g.variants.is_empty()));
    }

    #[test]
    fn making_pedestrian_is_refused_where_no_piece_is_motor_or_the_kind_is_not_one_of_the_three() {
        let walkway = street_of(StreetClass::Local, &["sidewalk", "planting"]);
        let mut e = Editor::from_street(&walkway, &walkway, 0);
        assert!(!e.make_pedestrian(kind("sidewalk")));
        assert_eq!(e.view().revisions.len(), 0);
        let busy = street_of(StreetClass::Local, &["sidewalk", "travel", "sidewalk"]);
        let mut e = Editor::from_street(&busy, &busy, 0);
        for bad in [kind("bench"), kind("travel"), kind("bike"), KINDS.len(), usize::MAX] {
            assert!(!e.make_pedestrian(bad), "{bad}");
        }
        assert_eq!(e.view().revisions.len(), 0);
    }

    /// The check of this id on a pedestrian street of these pieces.
    fn ped_check(pieces: &[(&str, i32)], id: &str) -> Check {
        let s = street_widths(StreetClass::Pedestrian, pieces);
        Editor::from_street(&s, &s, 0).view().checks.into_iter().find(|c| c.id == id).unwrap()
    }

    #[test]
    fn a_pedestrian_street_has_room_for_emergency_vehicles_on_an_unbroken_run_of_sidewalk_of_three_metres() {
        for (pieces, ok) in [
            (&[("sidewalk", 3000)][..], true),
            (&[("sidewalk", 2000), ("sidewalk", 1500)], true),
            (&[("sidewalk", 2900)], false),
            (&[("sidewalk", 2000), ("bench", 700), ("sidewalk", 2000)], false),
            (&[("sidewalk", 2000), ("buffer", 600), ("sidewalk", 2000)], false),
            (&[("sidewalk", 2000), ("planting", 1800), ("sidewalk", 2000)], false),
            (&[("terrace", 4000), ("pole", 600)], false),
        ] {
            let c = ped_check(pieces, "access");
            assert_eq!(c.ok, ok, "{pieces:?}");
            assert_eq!(c.detail.key, if ok { "check-access-ped-ok" } else { "check-access-ped-bad" }, "{pieces:?}");
            assert_eq!(c.detail.args, [("amount", Arg::Length(3000))]);
        }
    }

    #[test]
    fn a_pedestrian_street_asks_for_no_sidewalk_on_both_sides_and_a_street_with_a_motor_piece_is_judged_as_before() {
        let c = ped_check(&[("planting", 3000), ("terrace", 3000)], "edges");
        assert!(c.ok);
        assert_eq!(c.detail.key, "check-detail-no-edges-pedestrian");
        let busy = street_widths(StreetClass::Local, &[("planting", 3000), ("travel", 3300)]);
        let checks = Editor::from_street(&busy, &busy, 0).view().checks;
        let edges = checks.iter().find(|c| c.id == "edges").unwrap();
        assert!(!edges.ok, "a street of traffic with no sidewalk on both sides still fails");
        let access = checks.iter().find(|c| c.id == "access").unwrap();
        assert_eq!(access.detail.key, "check-access-ok", "and keeps its lane test");
        assert!(ped_check(&[("sidewalk", 3000)], "fits").ok, "fits is judged as ever");
    }

    #[test]
    fn the_view_says_whether_the_street_has_a_motor_piece_at_any_hour() {
        let s = street_of(StreetClass::Local, &["sidewalk", "travel", "sidewalk"]);
        let mut e = Editor::from_street(&s, &s, 0);
        assert!(e.view().has_motor);
        let lane = e.view().segments[1].uid;
        assert!(e.remove(lane));
        assert!(!e.view().has_motor);
    }
    // ---- zones in the history ------------------------------------------

    fn sample_street() -> Street {
        Street::sample(0, Side::Right)
    }

    /// An editor on the sample street divided into three zones, all on the base section; the zoning is one
    /// earlier revision.
    fn zoned_editor() -> Editor {
        let today = sample_street();
        let now = zoned(today.clone(), &[0, 20_000, 40_000], 60_000);
        Editor::from_street(&today, &now, 0)
    }

    #[test]
    fn make_pedestrian_acts_on_the_selected_zone_and_the_street_stays_local_until_the_last_zone_with_a_motor_piece_loses_it() {
        let mut e = zoned_editor();
        let sidewalk = crate::shared::catalogue::kind_index("sidewalk").unwrap();
        let base = e.snapshot().segments;
        assert_eq!(e.view().class, StreetClass::Local);
        assert!(e.view().has_motor);
        assert!(e.make_pedestrian(sidewalk));
        let s = e.snapshot();
        assert_eq!((s.class, s.class_before), (StreetClass::Local, None), "zones 1 and 2 still carry cars");
        assert!(!e.view().has_motor, "the selected zone has no motor piece left");
        assert_eq!(e.view().class, StreetClass::Local);
        assert!(!e.make_pedestrian(sidewalk), "nothing left to change in zone 0");
        assert_eq!(s.zones[1].segments, None);
        assert_eq!(s.zones[2].segments, None);
        assert_eq!(s.segments, base);
        assert!(e.select_zone(1));
        assert!(e.view().has_motor, "has_motor follows the selected zone");
        assert!(e.make_pedestrian(sidewalk));
        assert_eq!(e.snapshot().class, StreetClass::Local);
        assert!(e.select_zone(2));
        assert!(e.make_pedestrian(sidewalk));
        let s = e.snapshot();
        assert_eq!((s.class, s.class_before), (StreetClass::Pedestrian, Some(StreetClass::Local)));
        assert_eq!(e.view().class, StreetClass::Pedestrian);
    }

    #[test]
    fn a_measure_is_arranged_in_the_selected_zone_only() {
        let mut e = zoned_editor();
        let base = e.snapshot().segments;
        assert!(e.select_zone(1));
        let code = e.view().measures.iter().find(|m| m.can_apply && !m.present).expect("a measure fits").code;
        assert!(e.apply_measure(code));
        let s = e.snapshot();
        assert_ne!(s.zones[1].segments.as_deref(), None, "zone 1 has its own section");
        assert_eq!(s.zones[0].segments, None);
        assert_eq!(s.zones[2].segments, None);
        assert_eq!(s.segments, base, "the base is unchanged");
        assert!(e.view().measures.iter().find(|m| m.code == code).unwrap().present);
        assert!(e.select_zone(0));
        assert!(!e.view().measures.iter().find(|m| m.code == code).unwrap().present, "zone 0 is not arranged");
        assert!(e.view().measures.iter().find(|m| m.code == code).unwrap().can_apply, "can_apply is the selected zone's");
    }

    #[test]
    fn the_class_change_is_reported_only_when_the_street_class_changes() {
        let mut e = zoned_editor();
        let sidewalk = crate::shared::catalogue::kind_index("sidewalk").unwrap();
        for zone in 0..3 {
            assert!(e.select_zone(zone));
            let before = e.view().class;
            assert!(e.make_pedestrian(sidewalk));
            let after = e.view().class;
            assert_eq!(before != after, zone == 2, "zone {zone}");
        }
    }

    #[test]
    fn an_edit_acts_on_the_selected_zone_only_and_gives_it_a_section_of_its_own() {
        let mut e = zoned_editor();
        let base = e.snapshot().segments;
        assert!(e.select_zone(1));
        let (uid, w) = (base[0].uid, base[0].width_mm);
        assert!(e.set_width(uid, w + 100));
        let s = e.snapshot();
        let own = s.zones[1].segments.as_ref().expect("zone 1 has a section of its own");
        assert_eq!(own[0].width_mm, w + 100);
        assert_eq!(own.iter().map(|g| g.uid).collect::<Vec<_>>(), base.iter().map(|g| g.uid).collect::<Vec<_>>());
        assert_eq!(s.zones[0].segments, None);
        assert_eq!(s.zones[2].segments, None);
        assert_eq!(s.segments, base, "the base is unchanged");
    }

    #[test]
    fn a_zone_edited_back_to_the_base_is_stored_as_the_base_again() {
        let mut e = zoned_editor();
        let g = e.snapshot().segments[0].clone();
        assert!(e.select_zone(1));
        assert!(e.set_width(g.uid, g.width_mm + 100));
        assert!(e.snapshot().zones[1].segments.is_some());
        assert!(e.set_width(g.uid, g.width_mm));
        assert_eq!(e.snapshot().zones[1].segments, None);
    }

    #[test]
    fn a_piece_selected_in_one_zone_is_cleared_by_choosing_another_zone_and_selecting_a_piece_works_in_the_new_one() {
        let mut e = zoned_editor();
        let uids: Vec<u32> = e.snapshot().segments.iter().map(|g| g.uid).collect();
        e.select(Some(uids[1]));
        assert_eq!(e.view().selected, Some(uids[1]));
        assert!(e.select_zone(2));
        assert_eq!(e.view().selected, None);
        e.select(Some(uids[2]));
        assert_eq!(e.view().selected, Some(uids[2]));
        assert!(e.select_zone(2));
        assert_eq!(e.view().selected, Some(uids[2]), "choosing the zone already chosen keeps the piece");
        assert!(!e.select_zone(3), "no such zone");
        assert_eq!(e.zone(), 2);
    }

    #[test]
    fn undo_redo_and_start_over_restore_the_zones_exactly_and_the_selected_zone_falls_back_when_it_no_longer_exists() {
        let today = sample_street();
        let mut e = zoned_editor();
        let three = e.snapshot();
        let g = three.segments[0].clone();
        assert!(e.select_zone(2));
        assert!(e.set_width(g.uid, g.width_mm + 100));
        let edited = e.snapshot();
        assert_eq!(edited.zones.len(), 3);
        assert!(e.undo());
        assert_eq!(e.snapshot(), three);
        assert_eq!(e.zone(), 2, "three zones still exist");
        assert!(e.undo());
        assert_eq!(e.snapshot(), today);
        assert!(e.zone() <= 1);
        assert_eq!(e.zone(), 0);
        assert!(e.redo() && e.redo());
        assert_eq!(e.snapshot(), edited);
        assert!(e.select_zone(2));
        assert!(e.reset());
        assert_eq!(e.snapshot(), today, "the street as first laid out: one zone");
        assert_eq!(e.zone(), 0);
    }

    #[test]
    fn a_drag_in_a_zone_is_one_revision_and_leaves_the_other_zones_alone() {
        let mut e = zoned_editor();
        let before = e.view().revisions.len();
        let zones_before = e.snapshot().zones;
        assert!(e.select_zone(0));
        e.begin_gesture();
        assert!(e.resize_boundary(1, 100));
        assert!(e.resize_boundary(1, 200));
        assert!(e.resize_boundary(1, 300));
        assert!(e.end_gesture());
        assert_eq!(e.view().revisions.len(), before + 1);
        let s = e.snapshot();
        assert!(s.zones[0].segments.is_some());
        assert_eq!(s.zones[1], zones_before[1]);
        assert_eq!(s.zones[2], zones_before[2]);
        assert_eq!(s.segments, sample_street().segments, "the base is unchanged");
    }

    #[test]
    fn a_drag_that_changes_nothing_records_nothing_and_leaves_no_own_section() {
        let mut e = zoned_editor();
        let before = e.snapshot();
        assert!(e.select_zone(1));
        e.begin_gesture();
        e.resize_boundary(1, 0);
        assert!(!e.end_gesture());
        assert_eq!(e.snapshot(), before);
    }

    #[test]
    fn a_snapshot_of_a_zoned_editor_builds_the_same_editor_again() {
        let today = sample_street();
        let mut e = zoned_editor();
        let g = e.snapshot().segments[0].clone();
        assert!(e.select_zone(1));
        assert!(e.set_width(g.uid, g.width_mm + 100));
        let snap = e.snapshot();
        assert_eq!((snap.length_mm, snap.zones.len()), (Some(60_000), 3));
        let again = Editor::from_street(&today, &snap, 0);
        assert_eq!(again.snapshot(), snap);
        assert_eq!(again.view().revisions.len(), 1, "the earlier changes are one revision");
        assert_eq!(again.view().revisions[0].label, Said::new("rev-earlier"));
    }

    #[test]
    fn the_class_follows_all_zones_through_the_history() {
        let mut base = street_of(StreetClass::Local, &["sidewalk", "sidewalk"]);
        let mut with_lane = base.segments.clone();
        with_lane.insert(1, Segment::new(50, kind("travel"), 3300));
        base.next_uid = 51;
        base.zones = vec![Zone { from_mm: 0, segments: None, name: None }, Zone { from_mm: 20_000, segments: Some(with_lane), name: None }];
        base.length_mm = Some(40_000);
        base.class = StreetClass::Local;
        let s = base.settled();
        assert_eq!(s.class, StreetClass::Local, "zone 0 has no motor piece, zone 1 has");
        let mut e = Editor::from_street(&s, &s, 0);
        assert_eq!(e.view().class, StreetClass::Local);
        assert!(e.select_zone(1));
        assert!(e.remove(50));
        assert_eq!(e.view().class, StreetClass::Pedestrian);
        assert_eq!(e.snapshot().class_before, Some(StreetClass::Local));
        assert!(e.undo());
        assert_eq!((e.view().class, e.snapshot().class_before), (s.class, s.class_before));
        assert_eq!(e.snapshot(), s);
    }

    // ---- checks, status and the view read every zone --------------------

    /// `zoned_editor()` with zone 1 widened until the street no longer fits.
    fn zoned_editor_with_zone_1_too_wide() -> Editor {
        let mut e = zoned_editor();
        let pieces = e.snapshot().segments;
        assert!(e.select_zone(1));
        for g in &pieces {
            e.set_width(g.uid, g.bounds().1);
        }
        assert!(e.select_zone(0));
        e
    }

    #[test]
    fn a_street_of_several_zones_fails_a_check_when_any_zone_fails_it_and_names_the_zone() {
        let mut e = zoned_editor_with_zone_1_too_wide();
        let fits = |e: &Editor| e.view().checks.into_iter().find(|c| c.id == "fits").unwrap();
        let (street, zone0) = (fits(&e), e.view().delta_mm);
        assert!(!street.ok, "zone 1 does not fit, so the street does not");
        assert_eq!(street.zone, Some(Said::new("zone-unnamed").with("n", Arg::Num(2))));
        assert!(zone0 <= 0, "the status line is the selected zone's, and zone 0 fits");
        assert!(e.select_zone(1));
        assert!(e.view().delta_mm > 0);
        // the others pass in every zone and name no zone
        let v = e.view();
        let edges = v.checks.iter().find(|c| c.id == "edges").unwrap();
        assert!(edges.ok && edges.zone.is_none());
    }

    #[test]
    fn a_one_zone_street_has_the_checks_and_the_status_it_had_and_names_no_zone() {
        let e = Editor::new(0);
        let v = e.view();
        assert!(v.checks.iter().all(|c| c.zone.is_none()));
        let segs: Vec<Segment> = e.snapshot().segments.iter().map(|s| s.at(v.time_min)).collect();
        let want = checks(&segs, v.row_mm, REGIONS[0].drive_side, false, false);
        let key = |cs: &[Check]| cs.iter().map(|c| (c.id, c.ok, c.label.clone(), c.detail.clone())).collect::<Vec<_>>();
        assert_eq!(key(&v.checks), key(&want));
        assert_eq!(v.zones.zones.len(), 1);
    }

    #[test]
    fn a_failing_check_of_a_street_with_every_zone_failing_names_the_first() {
        let mut e = zoned_editor_with_zone_1_too_wide();
        assert!(e.select_zone(2));
        for g in e.snapshot().segments {
            e.set_width(g.uid, g.bounds().1);
        }
        let fits = e.view().checks.into_iter().find(|c| c.id == "fits").unwrap();
        assert_eq!(fits.zone, Some(Said::new("zone-unnamed").with("n", Arg::Num(2))));
    }

    #[test]
    fn shares_and_capacity_are_those_of_the_selected_zone() {
        let mut e = zoned_editor();
        let plain = e.view().outcomes;
        let g = e.snapshot().segments;
        assert!(e.select_zone(1));
        for p in &g {
            e.set_width(p.uid, p.bounds().1);
        }
        let v1 = e.view().outcomes;
        assert_ne!(v1.capacity_pph, plain.capacity_pph);
        assert_eq!(v1.existing_capacity_pph, plain.existing_capacity_pph);
        assert!(e.select_zone(0));
        let v0 = e.view().outcomes;
        assert_eq!(
            (v0.capacity_pph, v0.share.iter().map(|s| s.mm).collect::<Vec<_>>()),
            (plain.capacity_pph, plain.share.iter().map(|s| s.mm).collect::<Vec<_>>())
        );
    }

    #[test]
    fn the_view_lists_zones_with_label_range_and_whether_each_has_its_own_section() {
        let today = sample_street();
        let now = zoned(today.clone(), &[0, 20_000, 45_000], 60_000);
        let mut e = Editor::from_street(&today, &now, 0);
        let z = e.view().zones;
        assert_eq!((z.length_mm, z.from_end, z.selected), (60_000, ZoneEnd::First, 0));
        assert_eq!(
            z.zones.iter().map(|g| (g.index, g.from_mm, g.to_mm, g.own, g.cut_off)).collect::<Vec<_>>(),
            vec![(0, 0, 20_000, false, false), (1, 20_000, 45_000, false, false), (2, 45_000, 60_000, false, false)]
        );
        assert_eq!(z.zones[1].label, Said::new("zone-unnamed").with("n", Arg::Num(2)));
        assert!(e.select_zone(1));
        let g = e.snapshot().segments[0].clone();
        assert!(e.set_width(g.uid, g.width_mm + 100));
        assert!(e.name_zone(Some("Market")));
        let z = e.view().zones;
        assert_eq!(z.selected, 1);
        assert_eq!(z.zones.iter().map(|g| g.own).collect::<Vec<_>>(), vec![false, true, false]);
        assert_eq!(z.zones[1].label, named_said("Market"));
    }

    #[test]
    fn a_street_with_no_zones_lists_one_zone_as_long_as_the_street_loaded() {
        let e = Editor::new(0).with_frame(Some(ZoneFrame { length_mm: 33_000, from_end: ZoneEnd::Last }));
        let z = e.view().zones;
        assert_eq!((z.length_mm, z.from_end, z.zones.len()), (33_000, ZoneEnd::Last, 1));
        assert_eq!((z.zones[0].from_mm, z.zones[0].to_mm), (0, 33_000));
        assert_eq!(Editor::new(0).view().zones.length_mm, 0);
    }

    #[test]
    fn changed_is_true_for_a_street_with_zones_even_when_no_section_differs_from_the_base() {
        let mut e = zoned_editor();
        assert!(e.view().changed);
        assert!(e.reset());
        assert!(!e.view().changed);
    }

    #[test]
    fn a_zone_beyond_the_loaded_end_is_cut_off_cannot_be_selected_and_is_kept() {
        let today = sample_street();
        let now = zoned(today.clone(), &[0, 20_000, 45_000], 60_000);
        let mut e = Editor::from_street(&today, &now, 0).with_frame(Some(ZoneFrame { length_mm: 40_000, from_end: ZoneEnd::First }));
        let z = e.view().zones;
        assert_eq!(z.elsewhere, Some(Elsewhere { drawn_mm: 60_000, loaded_mm: 40_000 }));
        assert_eq!(z.zones.iter().map(|g| g.cut_off).collect::<Vec<_>>(), vec![false, false, true]);
        assert!(!e.select_zone(2));
        assert_eq!(e.zone(), 0);
        let s = e.snapshot();
        assert_eq!((s.zones.len(), s.length_mm), (3, Some(60_000)));
        // a street drawn on this very length says nothing
        let same = Editor::from_street(&today, &now, 0).with_frame(Some(ZoneFrame { length_mm: 60_500, from_end: ZoneEnd::First }));
        assert_eq!(same.view().zones.elsewhere, None);
        // nor does one anchored at the other end, but it still differs
        let other = Editor::from_street(&today, &now, 0).with_frame(Some(ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::Last }));
        assert_eq!(other.view().zones.elsewhere, Some(Elsewhere { drawn_mm: 60_000, loaded_mm: 60_000 }));
    }

    #[test]
    fn applying_a_frame_that_cuts_off_the_selected_zone_falls_back_to_the_first_zone() {
        let today = sample_street();
        let now = zoned(today.clone(), &[0, 20_000, 45_000], 60_000);
        let mut e = Editor::from_street(&today, &now, 0);
        assert!(e.select_zone(2));
        let e = e.with_frame(Some(ZoneFrame { length_mm: 40_000, from_end: ZoneEnd::First }));
        assert_eq!(e.zone(), 0, "zone 2 starts beyond the loaded end");
        let mut e = Editor::from_street(&today, &now, 0);
        assert!(e.select_zone(1));
        assert_eq!(e.with_frame(Some(ZoneFrame { length_mm: 40_000, from_end: ZoneEnd::First })).zone(), 1, "a zone still loaded stays selected");
    }

    #[test]
    fn a_street_cut_at_both_ends_says_split_is_unavailable() {
        let e = zoned_editor();
        assert_eq!(e.view().zones.split, Some(Said::new("zone-refused-no-anchor")));
        let framed = zoned_editor().with_frame(Some(ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::First }));
        assert_eq!(framed.view().zones.split, None);
    }

    #[test]
    fn the_frame_is_kept_outside_the_history() {
        let frame = ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::Last };
        let e = zoned_editor().with_frame(Some(frame));
        assert_eq!(e.frame(), Some(frame));
        assert_eq!(zoned_editor().frame(), None);
    }

    fn with_speed(speed: Speed) -> Editor {
        let mut s = Street::sample(0, Side::Right);
        s.speed = speed;
        Editor::from_street(&s, &s, 0)
    }

    #[test]
    fn set_speed_changes_the_view_and_is_undoable() {
        let mut e = with_speed(Speed::Kmh(50));
        assert!(e.set_speed(Some(30)));
        assert_eq!(e.view().speed_kmh, Some(30));
        assert_eq!(e.view().revisions.last().unwrap().label, Said::new("rev-speed").with("kmh", Arg::Num(30)));
        assert!(e.undo());
        assert_eq!(e.view().speed_kmh, Some(50));
        assert!(e.redo());
        assert_eq!(e.view().speed_kmh, Some(30));
    }

    #[test]
    fn set_speed_refuses_a_value_outside_the_choices() {
        let mut e = with_speed(Speed::Kmh(50));
        assert!(!e.set_speed(Some(35)));
        assert!(e.view().revisions.is_empty());
    }

    #[test]
    fn set_speed_to_the_same_value_is_no_edit() {
        let mut e = with_speed(Speed::Kmh(50));
        assert!(!e.set_speed(Some(50)));
        assert!(e.view().revisions.is_empty());
    }

    #[test]
    fn set_speed_none_makes_it_not_set() {
        let mut e = with_speed(Speed::Kmh(50));
        assert!(e.set_speed(None));
        assert_eq!(e.view().speed_kmh, None);
        assert_eq!(e.snapshot().speed, Speed::NotSet);
        assert_eq!(e.view().revisions.last().unwrap().label, Said::new("rev-speed-not-set"));
        assert!(!e.set_speed(None));
    }

    #[test]
    fn undo_and_start_over_restore_the_speed() {
        let s = zoned(with_speed(Speed::Kmh(50)).snapshot(), &[0], 100_000);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 100_000, from_end: ZoneEnd::First }));
        assert!(e.set_speed(Some(30)));
        assert!(e.split_zone(Some(50_000)).is_ok());
        assert!(e.undo());
        assert!(e.undo());
        assert_eq!(e.view().speed_kmh, Some(50));
        assert!(e.set_speed(Some(30)));
        assert!(e.reset());
        assert_eq!(e.view().speed_kmh, Some(50));
    }

    #[test]
    fn an_imported_speed_outside_the_choices_is_kept_through_other_edits() {
        let mut e = with_speed(Speed::Kmh(25));
        let g = e.view().segments[0].uid;
        let w = e.view().segments[0].width_mm;
        assert!(e.set_width(g, w + 100));
        assert_eq!(e.view().speed_kmh, Some(25));
        assert_eq!(e.snapshot().speed, Speed::Kmh(25));
    }

    #[test]
    fn the_view_carries_the_street_side() {
        let uk = REGIONS.iter().position(|r| r.drive_side == Side::Left).unwrap();
        let s = Street::sample(0, Side::Left);
        assert_eq!(Editor::from_street(&s, &s, uk).view().side, Side::Left);
    }
}
