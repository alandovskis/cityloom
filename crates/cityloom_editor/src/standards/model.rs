//! The figures a standards profile can change, what makes a profile valid, and the profile the street page reads.
//!
//! `Standards::default()` is today's built-in set, so a street with no profile chosen is checked exactly as before.
//! The model holds no wording: a refusal, a warning or the note a check carries is `Said` data.
//!
//! The active profile is a per-page value (`ACTIVE`, the wasm module is single-threaded and each test thread has its
//! own): the street page activates the browser's chosen profile when it opens (`store::activate`), and the street
//! model reads it through the accessors below, falling back to the built-in figure when none is active.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::shared::catalogue::{KINDS, StreetClass, kind_index};
use crate::shared::said::{Arg, Said};

/// How near a pedestrian crossing parking or loading may stand, in millimetres. The Code de la sécurité routière of
/// Québec forbids stopping within 5 m of a crosswalk; the article is UNVERIFIED.
pub const CROSSING_CLEARANCE_MM: i32 = 5_000;

/// The buffer a bike lane needs between it and the parking beside it, so that opening a door does not open it into
/// the lane, in millimetres: 500 mm, as the MTQ (Tome I, ch. 15, § 15.4.2.2) and the Ville de Montréal's lane-width
/// tool have it (the vélorue sheet suggests 1 m ideally beside a contra-flow lane, which is not modelled).
pub const DOOR_BUFFER_MM: i32 = 500;

/// The walkway a patio leaves on its sidewalk, in millimetres: 1 800 mm, the Ville de Montréal's café-terrasse sheet
/// for a local street (2 100 mm on a collector or an arterial, which is not modelled).
pub const PATIO_WALK_MM: i32 = 1_800;

/// The shortest stop: a boarding platform for one single-unit bus, 8 m in the Ville de Montréal's bus-stop sheet (an
/// articulated bus needs 16 m, which is not modelled; the length is to be checked with the STM).
pub const MIN_STOP_MM: i32 = 8_000;

/// The width an emergency vehicle needs: an unbroken lane (or, on a pedestrian street, run of sidewalk), in millimetres.
pub const ACCESS_LANE_MM: i32 = 3000;

/// The speed limits an editor sets, in kilometres an hour.
pub const SPEED_CHOICES: [u16; 12] = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];

/// The longest name of a profile, in characters.
pub const MAX_NAME: usize = 60;
/// The most speed limits a profile offers.
pub const MAX_SPEEDS: usize = 24;
/// The slowest and fastest speed a profile may name, in kilometres an hour.
pub const SPEED_RANGE_KMH: (u16, u16) = (5, 130);
/// The narrowest and widest a piece may be made, whatever a profile says, in millimetres.
pub const WIDTH_RANGE_MM: (i32, i32) = (100, 30_000);

/// A figure a profile can change, which names it in a refusal, a warning or a note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    AccessLane,
    CrossingClearance,
    DoorBuffer,
    PatioWalk,
    StopMin,
    SpeedChoices,
    Width,
    TargetSpeed,
}

impl Field {
    /// Every figure that is one length, in the order the form lists them.
    pub const LENGTHS: [Field; 5] = [Field::AccessLane, Field::CrossingClearance, Field::DoorBuffer, Field::PatioWalk, Field::StopMin];

    /// The message that names the figure.
    pub fn key(self) -> &'static str {
        match self {
            Field::AccessLane => "standards-field-access-lane",
            Field::CrossingClearance => "standards-field-crossing-clearance",
            Field::DoorBuffer => "standards-field-door-buffer",
            Field::PatioWalk => "standards-field-patio-walk",
            Field::StopMin => "standards-field-stop-min",
            Field::SpeedChoices => "standards-field-speed-choices",
            Field::Width => "standards-field-width",
            Field::TargetSpeed => "standards-field-target-speed",
        }
    }

    /// The widest range a profile may give a length figure, in millimetres; none for a figure that is no length.
    pub fn range_mm(self) -> Option<(i32, i32)> {
        match self {
            Field::AccessLane => Some((2_000, 6_000)),
            Field::CrossingClearance => Some((0, 15_000)),
            Field::DoorBuffer => Some((0, 2_000)),
            Field::PatioWalk => Some((1_000, 4_000)),
            Field::StopMin => Some((3_000, 30_000)),
            Field::Width => Some(WIDTH_RANGE_MM),
            Field::SpeedChoices | Field::TargetSpeed => None,
        }
    }

    /// The built-in figure, for a length figure.
    pub fn builtin_mm(self) -> Option<i32> {
        match self {
            Field::AccessLane => Some(ACCESS_LANE_MM),
            Field::CrossingClearance => Some(CROSSING_CLEARANCE_MM),
            Field::DoorBuffer => Some(DOOR_BUFFER_MM),
            Field::PatioWalk => Some(PATIO_WALK_MM),
            Field::StopMin => Some(MIN_STOP_MM),
            _ => None,
        }
    }
}

/// The widths a kind of piece may take, in millimetres.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidthRange {
    pub min_mm: i32,
    pub max_mm: i32,
}

/// The thresholds a profile sets. Whatever it does not set is the built-in figure (`Default`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Standards {
    /// The width an emergency vehicle needs (the `access` check).
    pub access_lane_mm: i32,
    /// How near a crossing parking or loading may stand (`curb-crossing`).
    pub curb_crossing_clearance_mm: i32,
    /// The buffer between a bike lane and the parking beside it (`curb-bike`).
    pub curb_door_buffer_mm: i32,
    /// The walkway a patio leaves (`curb-sidewalk`).
    pub curb_patio_walk_mm: i32,
    /// The shortest transit stop (`stop-length`).
    pub stop_min_mm: i32,
    /// The speed limits the street page offers, rising.
    pub speed_choices: Vec<u16>,
    /// The widths of the kinds of piece that differ from the catalogue's, by kind id.
    pub kind_widths: BTreeMap<String, WidthRange>,
    /// The speed a street of a class should not exceed (`speed-target`), by class id; a class with none is not checked.
    pub target_kmh: BTreeMap<String, u16>,
}

impl Default for Standards {
    fn default() -> Self {
        Standards {
            access_lane_mm: ACCESS_LANE_MM,
            curb_crossing_clearance_mm: CROSSING_CLEARANCE_MM,
            curb_door_buffer_mm: DOOR_BUFFER_MM,
            curb_patio_walk_mm: PATIO_WALK_MM,
            stop_min_mm: MIN_STOP_MM,
            speed_choices: SPEED_CHOICES.to_vec(),
            kind_widths: BTreeMap::new(),
            target_kmh: BTreeMap::new(),
        }
    }
}

/// Why a profile, or one of its figures, is refused. Data only; `said` gives the reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    NameEmpty,
    NameTooLong {
        max: usize,
    },
    NameTaken,
    /// A length outside what is plausible (millimetres).
    Length {
        field: Field,
        min_mm: i32,
        max_mm: i32,
    },
    /// A speed outside what is plausible (kilometres an hour).
    Speed {
        min_kmh: u16,
        max_kmh: u16,
    },
    SpeedsEmpty,
    SpeedsMany {
        max: usize,
    },
    SpeedsOrder,
    UnknownKind(String),
    UnknownClass(String),
    /// The smallest width is above the largest.
    WidthOrder {
        kind: &'static str,
    },
    /// The kind's own default width is outside the range.
    WidthDefault {
        kind: &'static str,
        default_mm: i32,
    },
    /// A figure typed that is no number.
    NotANumber,
    /// There is no such profile.
    Unknown,
    /// The browser keeps no more profiles (the limit) or will not keep them (storage blocked).
    TooMany {
        max: usize,
    },
    Storage,
    /// The profiles were written by a newer build, which this one leaves alone.
    Newer,
    /// The text is no exported profile.
    ImportUnreadable,
    /// The exported profile's schema version is not one this build reads.
    ImportVersion {
        found: u32,
    },
}

impl Refusal {
    pub fn said(&self) -> Said {
        match self {
            Refusal::NameEmpty => Said::new("standards-refused-name-empty"),
            Refusal::NameTooLong { max } => Said::new("standards-refused-name-long").with("max", Arg::Num(*max as i64)),
            Refusal::NameTaken => Said::new("standards-refused-name-taken"),
            Refusal::Length { field, min_mm, max_mm } => {
                Said::new("standards-refused-length").with("field", Arg::Msg(field.key())).with("min", Arg::Length(*min_mm)).with("max", Arg::Length(*max_mm))
            }
            Refusal::Speed { min_kmh, max_kmh } => {
                Said::new("standards-refused-speed").with("min", Arg::Num((*min_kmh).into())).with("max", Arg::Num((*max_kmh).into()))
            }
            Refusal::SpeedsEmpty => Said::new("standards-refused-speeds-empty"),
            Refusal::SpeedsMany { max } => Said::new("standards-refused-speeds-many").with("max", Arg::Num(*max as i64)),
            Refusal::SpeedsOrder => Said::new("standards-refused-speeds-order"),
            Refusal::UnknownKind(id) => Said::new("standards-refused-kind").with("id", Arg::Text(id.clone())),
            Refusal::UnknownClass(id) => Said::new("standards-refused-class").with("id", Arg::Text(id.clone())),
            Refusal::WidthOrder { kind } => Said::new("standards-refused-width-order").with("kind", Arg::KindLower(kind)),
            Refusal::WidthDefault { kind, default_mm } => {
                Said::new("standards-refused-width-default").with("kind", Arg::KindLower(kind)).with("default", Arg::Length(*default_mm))
            }
            Refusal::NotANumber => Said::new("standards-refused-number"),
            Refusal::Unknown => Said::new("standards-refused-unknown"),
            Refusal::TooMany { max } => Said::new("standards-refused-many").with("max", Arg::Num(*max as i64)),
            Refusal::Storage => Said::new("standards-refused-storage"),
            Refusal::Newer => Said::new("standards-refused-newer"),
            Refusal::ImportUnreadable => Said::new("standards-refused-import"),
            Refusal::ImportVersion { found } => Said::new("standards-refused-import-version").with("found", Arg::Num((*found).into())),
        }
    }
}

/// A name, trimmed: not empty and not longer than `MAX_NAME` characters.
pub fn clean_name(name: &str) -> Result<String, Refusal> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Refusal::NameEmpty);
    }
    if name.chars().count() > MAX_NAME {
        return Err(Refusal::NameTooLong { max: MAX_NAME });
    }
    Ok(name.to_string())
}

fn kind_static(id: &str) -> Option<&'static str> {
    kind_index(id).map(|k| KINDS[k].id)
}

impl Standards {
    /// The length figure `field` names, for the five that are one length.
    pub fn length_mm(&self, field: Field) -> Option<i32> {
        match field {
            Field::AccessLane => Some(self.access_lane_mm),
            Field::CrossingClearance => Some(self.curb_crossing_clearance_mm),
            Field::DoorBuffer => Some(self.curb_door_buffer_mm),
            Field::PatioWalk => Some(self.curb_patio_walk_mm),
            Field::StopMin => Some(self.stop_min_mm),
            _ => None,
        }
    }

    /// Sets the length figure `field` names; false for a figure that is no one length.
    pub fn set_length_mm(&mut self, field: Field, mm: i32) -> bool {
        let slot = match field {
            Field::AccessLane => &mut self.access_lane_mm,
            Field::CrossingClearance => &mut self.curb_crossing_clearance_mm,
            Field::DoorBuffer => &mut self.curb_door_buffer_mm,
            Field::PatioWalk => &mut self.curb_patio_walk_mm,
            Field::StopMin => &mut self.stop_min_mm,
            _ => return false,
        };
        *slot = mm;
        true
    }

    /// Whether every figure is acceptable; the first that is not says why.
    pub fn validate(&self) -> Result<(), Refusal> {
        for field in Field::LENGTHS {
            let (min_mm, max_mm) = field.range_mm().expect("a length figure has a range");
            if !(min_mm..=max_mm).contains(&self.length_mm(field).expect("a length figure")) {
                return Err(Refusal::Length { field, min_mm, max_mm });
            }
        }
        let (lo, hi) = SPEED_RANGE_KMH;
        let speed = Refusal::Speed { min_kmh: lo, max_kmh: hi };
        if self.speed_choices.is_empty() {
            return Err(Refusal::SpeedsEmpty);
        }
        if self.speed_choices.len() > MAX_SPEEDS {
            return Err(Refusal::SpeedsMany { max: MAX_SPEEDS });
        }
        if self.speed_choices.iter().any(|n| !(lo..=hi).contains(n)) {
            return Err(speed);
        }
        if self.speed_choices.windows(2).any(|w| w[0] >= w[1]) {
            return Err(Refusal::SpeedsOrder);
        }
        let (wmin, wmax) = WIDTH_RANGE_MM;
        for (id, range) in &self.kind_widths {
            let kind = kind_static(id).ok_or_else(|| Refusal::UnknownKind(id.clone()))?;
            if range.min_mm < wmin || range.max_mm > wmax {
                return Err(Refusal::Length { field: Field::Width, min_mm: wmin, max_mm: wmax });
            }
            if range.min_mm > range.max_mm {
                return Err(Refusal::WidthOrder { kind });
            }
            let default_mm = KINDS[kind_index(kind).expect("a known kind")].default_mm;
            if !(range.min_mm..=range.max_mm).contains(&default_mm) {
                return Err(Refusal::WidthDefault { kind, default_mm });
            }
        }
        for (id, kmh) in &self.target_kmh {
            if !StreetClass::ALL.iter().any(|c| c.id() == id) {
                return Err(Refusal::UnknownClass(id.clone()));
            }
            if !(lo..=hi).contains(kmh) {
                return Err(speed);
            }
        }
        Ok(())
    }

    /// What is looser than the built-in figures, one warning for each: allowed, but shown.
    pub fn warnings(&self) -> Vec<Said> {
        let mut out = Vec::new();
        for field in Field::LENGTHS {
            let (value, builtin) = (self.length_mm(field).expect("a length figure"), field.builtin_mm().expect("a length figure"));
            if value < builtin {
                out.push(
                    Said::new("standards-warn-length")
                        .with("field", Arg::Msg(field.key()))
                        .with("value", Arg::Length(value))
                        .with("builtin", Arg::Length(builtin)),
                );
            }
        }
        let fastest = SPEED_CHOICES[SPEED_CHOICES.len() - 1];
        if let Some(&top) = self.speed_choices.iter().filter(|&&n| n > fastest).max() {
            out.push(
                Said::new("standards-warn-speeds")
                    .with("field", Arg::Msg(Field::SpeedChoices.key()))
                    .with("value", Arg::Num(top.into()))
                    .with("builtin", Arg::Num(fastest.into())),
            );
        }
        for (id, range) in &self.kind_widths {
            let Some(k) = kind_index(id) else { continue };
            let kind = &KINDS[k];
            if range.min_mm < kind.min_mm || range.max_mm > kind.max_mm {
                out.push(
                    Said::new("standards-warn-width")
                        .with("field", Arg::Msg(Field::Width.key()))
                        .with("kind", Arg::KindLower(kind.id))
                        .with("min", Arg::Length(range.min_mm))
                        .with("max", Arg::Length(range.max_mm))
                        .with("builtin_min", Arg::Length(kind.min_mm))
                        .with("builtin_max", Arg::Length(kind.max_mm)),
                );
            }
        }
        out
    }
}

/// The profile in force: its name, and its figures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Active {
    pub name: String,
    pub standards: Standards,
}

thread_local! {
    static ACTIVE: RefCell<Option<Rc<Active>>> = const { RefCell::new(None) };
}

/// Makes `active` the profile the street model reads (None: the built-in figures).
pub fn set_active(active: Option<Active>) {
    ACTIVE.with(|a| *a.borrow_mut() = active.map(Rc::new));
}

fn current() -> Option<Rc<Active>> {
    ACTIVE.with(|a| a.borrow().clone())
}

/// The name of the profile in force, when there is one.
pub fn active_name() -> Option<String> {
    current().map(|a| a.name.clone())
}

/// The figures in force: the profile's, else the built-in set.
pub fn standards() -> Standards {
    current().map_or_else(Standards::default, |a| a.standards.clone())
}

fn read<T>(from_profile: impl FnOnce(&Standards) -> T, builtin: impl FnOnce() -> T) -> T {
    match current() {
        Some(a) => from_profile(&a.standards),
        None => builtin(),
    }
}

/// The width an emergency vehicle needs, in millimetres.
pub fn access_lane_mm() -> i32 {
    read(|s| s.access_lane_mm, || ACCESS_LANE_MM)
}

/// How near a crossing parking or loading may stand, in millimetres.
pub fn crossing_clearance_mm() -> i32 {
    read(|s| s.curb_crossing_clearance_mm, || CROSSING_CLEARANCE_MM)
}

/// The buffer between a bike lane and parking, in millimetres.
pub fn door_buffer_mm() -> i32 {
    read(|s| s.curb_door_buffer_mm, || DOOR_BUFFER_MM)
}

/// The walkway a patio leaves, in millimetres.
pub fn patio_walk_mm() -> i32 {
    read(|s| s.curb_patio_walk_mm, || PATIO_WALK_MM)
}

/// The shortest transit stop, in millimetres.
pub fn stop_min_mm() -> i32 {
    read(|s| s.stop_min_mm, || MIN_STOP_MM)
}

/// The speed limits the page offers.
pub fn speed_choices() -> Vec<u16> {
    read(|s| s.speed_choices.clone(), || SPEED_CHOICES.to_vec())
}

/// The narrowest and widest a piece of kind `kind` (an index into `KINDS`) may be, in millimetres.
pub fn kind_bounds(kind: usize) -> (i32, i32) {
    let k = &KINDS[kind];
    read(|s| s.kind_widths.get(k.id).map_or((k.min_mm, k.max_mm), |r| (r.min_mm, r.max_mm)), || (k.min_mm, k.max_mm))
}

/// The speed a street of `class` should not exceed, when the profile in force names one.
pub fn target_kmh(class: StreetClass) -> Option<u16> {
    read(|s| s.target_kmh.get(class.id()).copied(), || None)
}

/// What a check that reads the length figure `field` says of its source: the profile's name, the figure it used and the
/// built-in one. None when no profile is in force: the check is the built-in one and says nothing of it.
pub fn note(field: Field) -> Option<Said> {
    let a = current()?;
    let (value, builtin) = (a.standards.length_mm(field)?, field.builtin_mm()?);
    Some(Said::new("standards-note-length").with("name", Arg::Text(a.name.clone())).with("value", Arg::Length(value)).with("builtin", Arg::Length(builtin)))
}

/// The same for the widths of a kind of piece: the range in force and the built-in one. None with no profile, or for a
/// kind the profile leaves as it is.
pub fn note_width(kind: usize) -> Option<Said> {
    let a = current()?;
    let k = &KINDS[kind];
    let range = a.standards.kind_widths.get(k.id)?;
    Some(
        Said::new("standards-note-width")
            .with("name", Arg::Text(a.name.clone()))
            .with("min", Arg::Length(range.min_mm))
            .with("max", Arg::Length(range.max_mm))
            .with("builtin_min", Arg::Length(k.min_mm))
            .with("builtin_max", Arg::Length(k.max_mm)),
    )
}

/// The same for the speed target of a class. None with no profile or no target for the class.
pub fn note_target(class: StreetClass) -> Option<Said> {
    let a = current()?;
    let kmh = a.standards.target_kmh.get(class.id())?;
    Some(Said::new("standards-note-target").with("name", Arg::Text(a.name.clone())).with("kmh", Arg::Num((*kmh).into())))
}

/// The same for the speed limits on offer. None with no profile.
pub fn note_speeds() -> Option<Said> {
    let a = current()?;
    let list = a.standards.speed_choices.iter().map(u16::to_string).collect::<Vec<_>>().join(", ");
    Some(Said::new("standards-note-speeds").with("name", Arg::Text(a.name.clone())).with("list", Arg::Text(list)))
}

/// Puts the built-in figures back when dropped (a test that activates a profile holds one).
#[cfg(test)]
pub struct Restore;

#[cfg(test)]
impl Drop for Restore {
    fn drop(&mut self) {
        set_active(None);
    }
}

/// Activates a profile for the rest of the test.
#[cfg(test)]
pub fn activate_for_test(name: &str, standards: Standards) -> Restore {
    set_active(Some(Active { name: name.to_string(), standards }));
    Restore
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_standards_are_the_built_in_figures() {
        let s = Standards::default();
        assert_eq!(s.access_lane_mm, ACCESS_LANE_MM);
        assert_eq!(s.speed_choices, SPEED_CHOICES.to_vec());
        assert!(s.kind_widths.is_empty() && s.target_kmh.is_empty());
        assert_eq!(s.validate(), Ok(()));
        assert!(s.warnings().is_empty());
    }

    #[test]
    fn nothing_is_active_until_a_profile_is_chosen() {
        assert_eq!(active_name(), None);
        assert_eq!(access_lane_mm(), ACCESS_LANE_MM);
        assert_eq!(speed_choices(), SPEED_CHOICES.to_vec());
        assert_eq!(kind_bounds(0), (KINDS[0].min_mm, KINDS[0].max_mm));
        assert_eq!(note(Field::AccessLane), None);
    }

    #[test]
    fn an_active_profile_changes_what_is_read_and_is_named() {
        let mut s = Standards::default();
        s.access_lane_mm = 3500;
        s.kind_widths.insert("travel".into(), WidthRange { min_mm: 2800, max_mm: 3600 });
        let _guard = activate_for_test("City of X", s);
        assert_eq!(active_name().as_deref(), Some("City of X"));
        assert_eq!(access_lane_mm(), 3500);
        let travel = kind_index("travel").unwrap();
        assert_eq!(kind_bounds(travel), (2800, 3600));
        let said = note(Field::AccessLane).unwrap();
        assert_eq!(said.key, "standards-note-length");
        assert_eq!(said.args[0], ("name", Arg::Text("City of X".into())));
        assert_eq!(said.args[1], ("value", Arg::Length(3500)));
        assert_eq!(said.args[2], ("builtin", Arg::Length(ACCESS_LANE_MM)));
    }

    #[test]
    fn the_guard_puts_the_built_in_figures_back() {
        {
            let _guard = activate_for_test("X", Standards { access_lane_mm: 3500, ..Standards::default() });
            assert_eq!(access_lane_mm(), 3500);
        }
        assert_eq!(access_lane_mm(), ACCESS_LANE_MM);
    }

    #[test]
    fn a_figure_out_of_its_range_is_refused_with_the_range() {
        let s = Standards { access_lane_mm: 500, ..Standards::default() };
        assert_eq!(s.validate(), Err(Refusal::Length { field: Field::AccessLane, min_mm: 2_000, max_mm: 6_000 }));
        let said = Refusal::Length { field: Field::AccessLane, min_mm: 2_000, max_mm: 6_000 }.said();
        assert_eq!(said.key, "standards-refused-length");
    }

    #[test]
    fn speed_choices_must_be_a_rising_list_of_plausible_speeds() {
        let with = |v: Vec<u16>| Standards { speed_choices: v, ..Standards::default() }.validate();
        assert_eq!(with(vec![]), Err(Refusal::SpeedsEmpty));
        assert_eq!(with(vec![30, 30]), Err(Refusal::SpeedsOrder));
        assert_eq!(with(vec![40, 30]), Err(Refusal::SpeedsOrder));
        assert_eq!(with(vec![2, 30]), Err(Refusal::Speed { min_kmh: 5, max_kmh: 130 }));
        assert_eq!(with(vec![30, 200]), Err(Refusal::Speed { min_kmh: 5, max_kmh: 130 }));
        assert_eq!(with((1..=30).map(|n| n * 4).collect()), Err(Refusal::SpeedsMany { max: 24 }));
        assert_eq!(with(vec![30, 50]), Ok(()));
    }

    #[test]
    fn a_width_range_keeps_the_kinds_own_default_inside_it() {
        let travel = KINDS[kind_index("travel").unwrap()];
        let with = |min_mm, max_mm| {
            let mut s = Standards::default();
            s.kind_widths.insert("travel".into(), WidthRange { min_mm, max_mm });
            s.validate()
        };
        assert_eq!(with(3600, 3000), Err(Refusal::WidthOrder { kind: "travel" }));
        assert_eq!(with(travel.default_mm + 100, 4000), Err(Refusal::WidthDefault { kind: "travel", default_mm: travel.default_mm }));
        assert_eq!(with(2000, travel.default_mm - 100), Err(Refusal::WidthDefault { kind: "travel", default_mm: travel.default_mm }));
        assert_eq!(with(50, 4000), Err(Refusal::Length { field: Field::Width, min_mm: 100, max_mm: 30_000 }));
        assert_eq!(with(2500, 4200), Ok(()));
        let mut s = Standards::default();
        s.kind_widths.insert("nope".into(), WidthRange { min_mm: 2500, max_mm: 4200 });
        assert_eq!(s.validate(), Err(Refusal::UnknownKind("nope".into())));
    }

    #[test]
    fn a_target_speed_names_a_known_class_and_a_plausible_speed() {
        let mut s = Standards::default();
        s.target_kmh.insert("local".into(), 30);
        assert_eq!(s.validate(), Ok(()));
        s.target_kmh.insert("lane".into(), 30);
        assert_eq!(s.validate(), Err(Refusal::UnknownClass("lane".into())));
        s.target_kmh.remove("lane");
        s.target_kmh.insert("arterial".into(), 3);
        assert_eq!(s.validate(), Err(Refusal::Speed { min_kmh: 5, max_kmh: 130 }));
    }

    #[test]
    fn looser_than_built_in_is_allowed_with_a_warning_for_each_figure() {
        let s = Standards { access_lane_mm: 2500, curb_door_buffer_mm: 200, stop_min_mm: 6000, speed_choices: vec![30, 140 - 20, 125], ..Standards::default() };
        assert_eq!(s.validate(), Ok(()));
        let keys: Vec<_> = s.warnings().iter().map(|w| w.args[0].1.clone()).collect();
        assert!(keys.contains(&Arg::Msg(Field::AccessLane.key())));
        assert!(keys.contains(&Arg::Msg(Field::DoorBuffer.key())));
        assert!(keys.contains(&Arg::Msg(Field::StopMin.key())));
        assert!(keys.contains(&Arg::Msg(Field::SpeedChoices.key())));
        assert!(!keys.contains(&Arg::Msg(Field::CrossingClearance.key())));
        // a stricter figure warns of nothing
        assert!(Standards { access_lane_mm: 3500, ..Standards::default() }.warnings().is_empty());
    }

    #[test]
    fn a_name_is_trimmed_and_must_be_short_and_not_empty() {
        assert_eq!(clean_name("  City of X "), Ok("City of X".to_string()));
        assert_eq!(clean_name("   "), Err(Refusal::NameEmpty));
        assert_eq!(clean_name(&"x".repeat(61)), Err(Refusal::NameTooLong { max: 60 }));
    }
}
