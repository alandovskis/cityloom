//! Transit stops: a stretch of a street's edge where buses stop, drawn as a curb stop, a bulb or a boarding island.
//!
//! Like the curbside uses (`curb.rs`) they belong to the street, not to a section: a position is measured along the
//! whole street from its anchored end (`Street::from_end`), so splitting, merging or moving zones leaves them where
//! they are. A stop may overlap a curbside use; two stops on one side may not overlap each other. A street with
//! none is written, drawn and checked exactly as before.

use serde::{Deserialize, Serialize};

use super::curb::{CurbRefusal, Failure, SideView, StripCell, StripView, check_rows, curbside, selected_cell, side_word, strip_cells, toward_edge};
use super::{Arg, Check, Content, Editor, SNAP_MM, Said, Segment, Street, ZoneFrame, snap, widest_walkway_mm};
use crate::shared::catalogue::{KINDS, REGIONS, Side, is_motor};

/// The most characters a stop's name keeps.
const MAX_NAME_CHARS: usize = 80;

/// How a stop is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopKind {
    /// The bus stops in its lane against the curb.
    Curb,
    /// The sidewalk reaches out over the parking lane to the bus lane.
    Bulb,
    /// The bus stops beside an island with a bike lane behind it.
    Island,
}

impl StopKind {
    /// Every kind, in the order a choice lists them.
    pub const ALL: [StopKind; 3] = [StopKind::Curb, StopKind::Bulb, StopKind::Island];

    /// The message that names it.
    pub fn name_key(self) -> &'static str {
        match self {
            StopKind::Curb => "stop-kind-curb",
            StopKind::Bulb => "stop-kind-bulb",
            StopKind::Island => "stop-kind-island",
        }
    }
}

/// One transit stop, along `length_mm` of the street from `from_mm`.
///
/// `side` is the side of the section, as for a curbside `Instance`: `Left` is the first piece of the section's list,
/// `Right` the last, whatever the end the positions are measured from; `Street::reversed` swaps it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stop {
    pub side: Side,
    /// Millimetres from the anchored end (`Street::from_end`) to where it starts, a multiple of `SNAP_MM`.
    pub from_mm: i32,
    pub length_mm: i32,
    pub kind: StopKind,
    /// Whether it has a shelter.
    pub shelter: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Stop {
    pub(super) fn to_mm(&self) -> i32 {
        self.from_mm + self.length_mm
    }
}

impl Content {
    /// The stops in their canonical order: left side first, then by position.
    pub(super) fn sort_stops(&mut self) {
        self.stops.sort_by_key(|s| (s.side == Side::Right, s.from_mm));
    }
}

impl Street {
    /// Swaps the sides of the stops for a street seen from its other end. Their positions stay measured from the
    /// same physical end (`reverse_zones` flips the label of that end).
    pub(super) fn reverse_stops(&mut self) {
        for s in &mut self.stops {
            s.side = match s.side {
                Side::Left => Side::Right,
                Side::Right => Side::Left,
            };
        }
        self.stops.sort_by_key(|s| (s.side == Side::Right, s.from_mm));
    }

    /// Whether the stops can be placed: they need the length they were drawn against, and lie in front of it.
    pub(super) fn stops_are_sound(&self) -> bool {
        self.stops.is_empty() || (self.length_mm.is_some() && self.stops.iter().all(|s| s.from_mm >= 0 && s.length_mm > 0))
    }
}

impl Editor {
    /// Whether `candidate` may stand where it is, ignoring the stop `own` (the one being changed).
    fn stop_fits(&self, candidate: &Stop, own: Option<usize>, frame: ZoneFrame) -> Result<(), CurbRefusal> {
        let content = &self.states[self.cursor].content;
        if candidate.from_mm < 0 || candidate.to_mm() > frame.length_mm {
            return Err(CurbRefusal::TooLong);
        }
        let overlaps = content
            .stops
            .iter()
            .enumerate()
            .any(|(k, o)| Some(k) != own && o.side == candidate.side && o.from_mm < candidate.to_mm() && candidate.from_mm < o.to_mm());
        if overlaps {
            return Err(CurbRefusal::Overlap);
        }
        Ok(())
    }

    fn stop_label(key: &'static str, s: &Stop) -> Said {
        Said::new(key).with("kind", Arg::MsgLower(s.kind.name_key())).with("at", Arg::Length(s.from_mm))
    }

    /// Records a change of the stops as one revision, writing the length and end they are measured against.
    fn stop_change(&mut self, label: Said, frame: ZoneFrame, f: impl FnOnce(&mut Vec<Stop>)) {
        self.edit_content(label, |c| {
            f(&mut c.stops);
            c.sort_stops();
            c.from_end = frame.from_end;
            c.length_mm = Some(frame.length_mm);
            true
        });
    }

    /// Changes stop `i` (an index into the snapshot's `stops`) into `new` if it fits, as one revision. Nothing
    /// happens for an index that is not one.
    fn stop_replace(&mut self, i: usize, label_key: &'static str, change: impl FnOnce(&Stop) -> Stop) -> Result<(), CurbRefusal> {
        let frame = self.curb_frame(false)?;
        let Some(old) = self.states[self.cursor].content.stops.get(i).cloned() else { return Ok(()) };
        let new = change(&old);
        self.stop_fits(&new, Some(i), frame)?;
        self.stop_change(Self::stop_label(label_key, &new), frame, |stops| stops[i] = new);
        Ok(())
    }

    /// Adds a stop on `side` from `from_mm` (snapped to `SNAP_MM`) for `length_mm` (snapped, at least `SNAP_MM`),
    /// without a shelter or a name.
    pub fn add_stop(&mut self, side: Side, from_mm: i32, length_mm: i32, kind: StopKind) -> Result<(), CurbRefusal> {
        let frame = self.curb_frame(false)?;
        let s = Stop { side, from_mm: snap(from_mm), length_mm: snap(length_mm).max(SNAP_MM), kind, shelter: false, name: None };
        self.stop_fits(&s, None, frame)?;
        self.stop_choice.pick(s.side, s.from_mm);
        self.stop_change(Self::stop_label("rev-stop-add", &s), frame, |stops| stops.push(s));
        Ok(())
    }

    /// Changes how stop `i` is built.
    pub fn set_stop_kind(&mut self, i: usize, kind: StopKind) -> Result<(), CurbRefusal> {
        self.stop_replace(i, "rev-stop-kind", |old| Stop { kind, ..old.clone() })
    }

    /// Gives stop `i` a shelter, or takes it away.
    pub fn set_stop_shelter(&mut self, i: usize, on: bool) -> Result<(), CurbRefusal> {
        let frame = self.curb_frame(false)?;
        let Some(old) = self.states[self.cursor].content.stops.get(i).cloned() else { return Ok(()) };
        let new = Stop { shelter: on, ..old };
        let state = Arg::Msg(if on { "stop-word-sheltered" } else { "stop-word-unsheltered" });
        self.stop_change(Self::stop_label("rev-stop-shelter", &new).with("state", state), frame, |stops| stops[i] = new);
        Ok(())
    }

    /// Names stop `i`; `None`, or a name with nothing in it, takes the name away. A long name is cut.
    pub fn name_stop(&mut self, i: usize, name: Option<&str>) -> Result<(), CurbRefusal> {
        let name = name.map(str::trim).filter(|n| !n.is_empty()).map(|n| n.chars().take(MAX_NAME_CHARS).collect::<String>());
        self.stop_replace(i, "rev-stop-name", |old| Stop { name, ..old.clone() })
    }

    /// Gives stop `i` a new length (snapped, at least `SNAP_MM`), from where it starts.
    pub fn resize_stop(&mut self, i: usize, length_mm: i32) -> Result<(), CurbRefusal> {
        self.stop_replace(i, "rev-stop-resize", |old| Stop { length_mm: snap(length_mm).max(SNAP_MM), ..old.clone() })
    }

    /// Moves stop `i` to start at `from_mm` (snapped). Inside a gesture it is shown as it goes and recorded as one
    /// revision at the end.
    pub fn move_stop(&mut self, i: usize, from_mm: i32) -> Result<(), CurbRefusal> {
        let frame = self.curb_frame(true)?;
        let Some(old) = self.states[self.cursor].content.stops.get(i).cloned() else { return Ok(()) };
        let new = Stop { from_mm: snap(from_mm), ..old.clone() };
        self.stop_fits(&new, Some(i), frame)?;
        if new == old {
            return Ok(());
        }
        let label = Self::stop_label("rev-stop-move", &new);
        // The choice follows the stop, and a drag records one move, from where it began to where it is.
        self.stop_choice.follow(old.side, old.from_mm, new.from_mm);
        if self.gesture.is_some() {
            let c = &mut self.states[self.cursor].content;
            c.stops[i] = new;
            c.sort_stops();
            c.from_end = frame.from_end;
            c.length_mm = Some(frame.length_mm);
            self.pending_label = Some(label);
        } else {
            self.stop_change(label, frame, |stops| stops[i] = new);
        }
        Ok(())
    }

    /// Removes stop `i`.
    pub fn remove_stop(&mut self, i: usize) -> Result<(), CurbRefusal> {
        let frame = self.curb_frame(false)?;
        let Some(old) = self.states[self.cursor].content.stops.get(i).cloned() else { return Ok(()) };
        self.stop_change(Self::stop_label("rev-stop-remove", &old), frame, |stops| {
            stops.remove(i);
        });
        Ok(())
    }
}

/// The shortest stop: one standard bus. Unverified (TAC / STM to check).
pub const MIN_STOP_MM: i32 = 9_000;

/// The boarding area a wheelchair ramp needs. Unverified (TAC / STM to check).
const BOARDING_MM: i32 = 2_400;

/// How deep a shelter stands on the boarding area. Unverified (TAC / STM to check).
const SHELTER_DEPTH_MM: i32 = 1_500;

/// The least a bike lane behind an island may be. Unverified (TAC / STM to check).
const ISLAND_BIKE_MM: i32 = 1_500;

/// The piece the bus uses at a stop on `side` of a section: the `bus` or `travel` piece nearest that edge whose
/// traffic goes the way traffic goes on that half (a piece with no direction counts), or `None`.
///
/// A section is seen from the street's first end, and a piece's direction 0 ("away") runs from the first end to the
/// last, whichever end positions are measured from (`default_directions`: with traffic on the right the last
/// pieces run away and the first ones toward, with traffic on the left the first run away). So the half that
/// `driving` names carries direction 0, the other half direction 1, and the anchor plays no part.
pub(crate) fn bus_lane(segs: &[Segment], side: Side, driving: Side) -> Option<usize> {
    let flow = usize::from(side != driving);
    let nearest_first: Vec<usize> = match side {
        Side::Left => (0..segs.len()).collect(),
        Side::Right => (0..segs.len()).rev().collect(),
    };
    nearest_first.into_iter().find(|&i| matches!(KINDS[segs[i].kind].id, "bus" | "travel") && segs[i].direction.is_none_or(|d| d == flow))
}

/// What a stop of some kind stands on, beside the bus's lane.
struct Platform {
    /// The boarding area: the sidewalk run (curb stop), the parking or loading piece (bulb), the pieces between the
    /// lane and the bike lane (island).
    width_mm: i32,
    /// A bike lane between the bus's lane and the platform, which riders would cross.
    bike_crosses: bool,
    /// The width of the bike lane that passes behind an island.
    bike_behind_mm: Option<i32>,
}

/// What `kind` stands on beside `lane` on `side`, or `None` when the section lacks what the kind needs (spec §2).
fn platform(segs: &[Segment], lane: usize, side: Side, kind: StopKind) -> Option<Platform> {
    let outward = toward_edge(segs, lane, side);
    let id = |s: &&Segment| KINDS[s.kind].id;
    match kind {
        // Nothing with a motor on it between the lane and the edge: the bus stops at the curb.
        StopKind::Curb => (!outward.iter().any(|s| is_motor(s.kind))).then(|| Platform {
            width_mm: widest_walkway_mm(outward.iter().copied()),
            bike_crosses: outward.iter().any(|s| id(s) == "bike"),
            bike_behind_mm: None,
        }),
        // A parking or loading piece before any other motor piece.
        StopKind::Bulb => {
            let mut bike_crosses = false;
            for s in &outward {
                match id(s) {
                    "bike" => bike_crosses = true,
                    "parking" | "loading" => return Some(Platform { width_mm: s.width_mm, bike_crosses, bike_behind_mm: None }),
                    _ if is_motor(s.kind) => return None,
                    _ => {}
                }
            }
            None
        }
        // Parking, loading or buffer pieces, then a bike lane.
        StopKind::Island => {
            let mut width_mm = 0;
            for s in &outward {
                match id(s) {
                    "parking" | "loading" | "buffer" => width_mm += s.width_mm,
                    "bike" => return (width_mm > 0).then_some(Platform { width_mm, bike_crosses: false, bike_behind_mm: Some(s.width_mm) }),
                    _ => return None,
                }
            }
            None
        }
    }
}

/// Whether the section has what `kind` needs beside `lane` on `side`. A new stop starts as the first kind that fits
/// (`Editor::default_stop_kind`).
pub(crate) fn kind_fits(segs: &[Segment], lane: usize, side: Side, kind: StopKind) -> bool {
    platform(segs, lane, side, kind).is_some()
}

impl Stop {
    /// How the checks name it: "curb stop at 12 m, right", or "Mont-Royal (curb stop, 12 m)".
    fn named(&self) -> Said {
        let kind = Arg::MsgLower(self.kind.name_key());
        match &self.name {
            Some(name) => Said::new("stop-instance-named").with("name", Arg::Text(name.clone())).with("kind", kind).with("at", Arg::Length(self.from_mm)),
            None => Said::new("stop-instance").with("kind", kind).with("at", Arg::Length(self.from_mm)).with("side", Arg::Msg(side_word(self.side))),
        }
    }
}

/// A cell of a stops row: a stop or the gap between two.
#[derive(Serialize)]
pub struct StopCell {
    /// An index into the street's `stops` for a stop, `None` for a gap.
    pub instance: Option<usize>,
    pub from_mm: i32,
    pub to_mm: i32,
    /// How the stop is built; `None` for a gap.
    pub kind: Option<StopKind>,
    pub shelter: bool,
    pub name: Option<String>,
    /// What the cell says: the stop's name or kind, with its shelter; "Free" for a gap.
    pub label: Said,
    /// The stop breaks no stop check (a gap always passes).
    pub ok: bool,
}

impl StripCell for StopCell {
    fn instance(&self) -> Option<usize> {
        self.instance
    }
    fn from_mm(&self) -> i32 {
        self.from_mm
    }
    fn to_mm(&self) -> i32 {
        self.to_mm
    }
    fn label(&self) -> &Said {
        &self.label
    }
    fn ok(&self) -> bool {
        self.ok
    }
}

/// The stops rows: one per side that has a stop, a curbside lane or a lane a bus can use in its direction (a transit lane
/// or a plain driving lane) in some zone.
pub type StopsView = StripView<StopCell>;
pub type StopSideView = SideView<StopCell>;

impl Stop {
    /// What its cell says: "Mont-Royal (curb stop)" or "Curb stop", and ", with a shelter" when it has one.
    fn cell_label(&self) -> Said {
        let kind = Arg::Msg(self.kind.name_key());
        let base = match &self.name {
            Some(name) => Said::new("stop-label-named").with("name", Arg::Text(name.clone())).with("kind", Arg::MsgLower(self.kind.name_key())),
            None => Said::new("stop-label").with("kind", kind),
        };
        if self.shelter { Said::new("stop-label-sheltered").with("label", Arg::Said(Box::new(base))) } else { base }
    }
}

impl Editor {
    /// The stops rows: a cell per stop and per gap along the whole street, for each side that can have a stop.
    pub(super) fn stops_view(&self) -> StopsView {
        let content = &self.states[self.cursor].content;
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(0);
        if length_mm <= 0 {
            // Without a length there is nothing to lay the stops along.
            return StripView { sides: Vec::new() };
        }
        let driving = REGIONS[self.region].drive_side;
        let failing: Vec<usize> = self.stop_failures().iter().map(|f| f.instance).collect();
        let sides = [Side::Left, Side::Right]
            .into_iter()
            .filter(|&side| {
                content.stops.iter().any(|s| s.side == side)
                    || (0..content.zone_count()).any(|z| {
                        let segs = content.section(z);
                        curbside(segs, side).is_some() || bus_lane(segs, side, driving).is_some()
                    })
            })
            .map(|side| {
                let gap = |from_mm: i32, to_mm: i32| StopCell {
                    instance: None,
                    from_mm,
                    to_mm,
                    kind: None,
                    shelter: false,
                    name: None,
                    label: Said::new("curb-gap"),
                    ok: true,
                };
                let items = content.stops.iter().enumerate().filter(|(_, s)| s.side == side).map(|(k, s)| (k, s.from_mm, s.to_mm()));
                let cells = strip_cells(length_mm, items, gap, |k| {
                    let s = &content.stops[k];
                    StopCell {
                        instance: Some(k),
                        from_mm: s.from_mm,
                        to_mm: s.to_mm(),
                        kind: Some(s.kind),
                        shelter: s.shelter,
                        name: s.name.clone(),
                        label: s.cell_label(),
                        ok: !failing.contains(&k),
                    }
                });
                let selected = selected_cell(&cells, self.stop_choice.at, side);
                SideView { side, cells, selected }
            })
            .collect();
        StripView { sides }
    }

    /// Chooses the cell of `side`'s stops row that holds `at_mm`. A setting, not history.
    pub fn select_stop(&mut self, side: Side, at_mm: i32) {
        self.stop_choice.pick(side, at_mm);
    }

    /// Starts a drag of a stop: the steps are shown and the end is one revision. False when a drag is under way.
    pub fn begin_stop_drag(&mut self) -> bool {
        self.begin_curb_drag()
    }

    /// The kind a new stop on `side` starting at `from_mm` begins as: the first of the kinds (curb stop, curb
    /// extension, boarding island) that every zone it would overlap can take, else the curb stop.
    pub fn default_stop_kind(&self, side: Side, from_mm: i32) -> StopKind {
        let content = &self.states[self.cursor].content;
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(i32::MAX);
        let driving = REGIONS[self.region].drive_side;
        let sections = content.sections_over(from_mm, from_mm.saturating_add(MIN_STOP_MM), length_mm);
        StopKind::ALL
            .into_iter()
            .find(|&kind| {
                !sections.is_empty() && sections.iter().all(|segs| bus_lane(segs, side, driving).is_some_and(|lane| kind_fits(segs, lane, side, kind)))
            })
            .unwrap_or(StopKind::Curb)
    }
}

impl Editor {
    /// The rows of the stops, after the curbside ones (only when the street has stops): one per rule, failing with
    /// the first stop that breaks it, which it names in `zone`.
    ///
    /// A stop is judged against the section of every zone it overlaps: all the rules but the length fail when they
    /// fail in any of them. Its side is the section's (see `Stop`), and the lane is the one traffic uses on that half
    /// under the region's driving side (`bus_lane`).
    pub(super) fn stop_checks(&self) -> Vec<Check> {
        let content = &self.states[self.cursor].content;
        let rows = [
            ("stop-lane", "check-label-stop-lane", "check-detail-stop-lane-ok"),
            ("stop-kind", "check-label-stop-kind", "check-detail-stop-kind-ok"),
            ("stop-length", "check-label-stop-length", "check-detail-stop-length-ok"),
            ("stop-width", "check-label-stop-width", "check-detail-stop-width-ok"),
            ("stop-bike", "check-label-stop-bike", "check-detail-stop-bike-ok"),
            ("stop-shelter", "check-label-stop-shelter", "check-detail-stop-shelter-ok"),
        ];
        check_rows(&rows, &self.stop_failures(), |k| content.stops[k].named())
    }

    /// Every rule each stop breaks, stop by stop in order: `rule` indexes the rows of `stop_checks`.
    fn stop_failures(&self) -> Vec<Failure> {
        let content = &self.states[self.cursor].content;
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(i32::MAX);
        let driving = REGIONS[self.region].drive_side;
        let mut out: Vec<Failure> = Vec::new();
        for (k, s) in content.stops.iter().enumerate() {
            let mut fail = |rule: usize, detail: Said| Failure::record(&mut out, k, rule, detail);
            if s.length_mm < MIN_STOP_MM {
                fail(2, Said::new("check-detail-stop-length").with("length", Arg::Length(s.length_mm)).with("min", Arg::Length(MIN_STOP_MM)));
            }
            for segs in content.sections_over(s.from_mm, s.to_mm(), length_mm) {
                let Some(lane) = bus_lane(segs, s.side, driving) else {
                    fail(0, Said::new("check-detail-stop-lane"));
                    continue;
                };
                let Some(p) = platform(segs, lane, s.side, s.kind) else {
                    fail(1, Said::new("check-detail-stop-kind").with("kind", Arg::MsgLower(s.kind.name_key())));
                    continue;
                };
                if p.width_mm < BOARDING_MM {
                    fail(3, Said::new("check-detail-stop-width").with("have", Arg::Length(p.width_mm)).with("need", Arg::Length(BOARDING_MM)));
                }
                if p.bike_crosses {
                    fail(4, Said::new("check-detail-stop-bike-cross"));
                }
                if let Some(have) = p.bike_behind_mm.filter(|&w| w < ISLAND_BIKE_MM) {
                    fail(4, Said::new("check-detail-stop-bike-narrow").with("have", Arg::Length(have)).with("need", Arg::Length(ISLAND_BIKE_MM)));
                }
                if s.shelter && p.width_mm < BOARDING_MM + SHELTER_DEPTH_MM {
                    fail(
                        5,
                        Said::new("check-detail-stop-shelter").with("have", Arg::Length(p.width_mm)).with("need", Arg::Length(BOARDING_MM + SHELTER_DEPTH_MM)),
                    );
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::curb::{Instance, Use};
    use super::super::zones::Keep;
    use super::*;
    use crate::street::model::ZoneEnd;

    const LEN: i32 = 60_000;

    fn frame() -> Option<ZoneFrame> {
        Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::First })
    }

    fn editor() -> Editor {
        let s = Street::sample(0, Side::Right);
        Editor::from_street(&s, &s, 0).with_frame(frame())
    }

    fn stop(side: Side, from_mm: i32, length_mm: i32, kind: StopKind) -> Stop {
        Stop { side, from_mm, length_mm, kind, shelter: false, name: None }
    }

    #[test]
    fn add_snaps_and_orders() {
        let mut e = editor();
        e.add_stop(Side::Right, 30_049, 12_000, StopKind::Curb).unwrap();
        e.add_stop(Side::Left, 40_000, 9_040, StopKind::Bulb).unwrap();
        e.add_stop(Side::Right, 10_051, 9_000, StopKind::Island).unwrap();
        let s = e.snapshot();
        assert_eq!(
            s.stops,
            vec![
                stop(Side::Left, 40_000, 9_000, StopKind::Bulb),
                stop(Side::Right, 10_100, 9_000, StopKind::Island),
                stop(Side::Right, 30_000, 12_000, StopKind::Curb)
            ]
        );
        assert_eq!((s.from_end, s.length_mm), (ZoneEnd::First, Some(LEN)), "the street's length is written with the stops");
        assert_eq!(e.view().revisions.len(), 3);
        assert_eq!(e.view().revisions.last().unwrap().label.key, "rev-stop-add");
    }

    #[test]
    fn two_stops_on_a_side_cannot_overlap() {
        let mut e = editor();
        e.add_stop(Side::Right, 10_000, 10_000, StopKind::Curb).unwrap();
        assert_eq!(e.add_stop(Side::Right, 19_900, 9_000, StopKind::Curb), Err(CurbRefusal::Overlap));
        assert_eq!(e.add_stop(Side::Right, 5_000, 5_100, StopKind::Curb), Err(CurbRefusal::Overlap));
        e.add_stop(Side::Right, 20_000, 9_000, StopKind::Curb).unwrap();
        e.add_stop(Side::Right, 1_000, 9_000, StopKind::Curb).unwrap();
        assert_eq!(e.snapshot().stops.len(), 3, "touching is not overlapping");
        // A change that would overlap is refused too; a stop does not overlap itself.
        assert_eq!(e.resize_stop(0, 12_000), Err(CurbRefusal::Overlap));
        assert_eq!(e.move_stop(0, 5_000), Err(CurbRefusal::Overlap));
        e.resize_stop(0, 9_000).unwrap();
        e.move_stop(0, 0).unwrap();
    }

    #[test]
    fn a_stop_may_overlap_a_curb_use() {
        let mut e = editor();
        e.add_curb(Side::Right, 10_000, 6_000, Use::Parking).unwrap();
        e.add_stop(Side::Right, 12_000, 10_000, StopKind::Bulb).unwrap();
        // ...and a curb use may be added over a stop.
        e.add_curb(Side::Right, 20_000, 6_000, Use::Loading).unwrap();
        let s = e.snapshot();
        assert_eq!((s.curb.len(), s.stops.len()), (3 - 1, 1));
        assert_eq!(s.curb[0], Instance { side: Side::Right, from_mm: 10_000, length_mm: 6_000, use_: Use::Parking });
    }

    #[test]
    fn stops_on_opposite_sides_may_overlap() {
        let mut e = editor();
        e.add_stop(Side::Left, 10_000, 10_000, StopKind::Curb).unwrap();
        e.add_stop(Side::Right, 10_000, 10_000, StopKind::Curb).unwrap();
        assert_eq!(e.snapshot().stops.len(), 2);
    }

    #[test]
    fn past_the_end_is_refused() {
        let mut e = editor();
        assert_eq!(e.add_stop(Side::Right, LEN - 8_000, 9_000, StopKind::Curb), Err(CurbRefusal::TooLong));
        assert_eq!(e.add_stop(Side::Right, -1_000, 9_000, StopKind::Curb), Err(CurbRefusal::TooLong));
        e.add_stop(Side::Right, LEN - 9_000, 9_000, StopKind::Curb).unwrap();
        assert_eq!(e.resize_stop(0, 9_100), Err(CurbRefusal::TooLong));
        assert_eq!(e.move_stop(0, LEN - 8_000), Err(CurbRefusal::TooLong));
        assert_eq!(e.snapshot().stops.len(), 1);
    }

    #[test]
    fn a_stop_needs_a_real_end() {
        let s = Street::sample(0, Side::Right);
        let mut e = Editor::from_street(&s, &s, 0);
        assert_eq!(e.add_stop(Side::Right, 10_000, 9_000, StopKind::Curb), Err(CurbRefusal::NoAnchor));
        assert!(e.snapshot().stops.is_empty());
        // Stops drawn on a road of another length are kept but not rearranged from here.
        let mut drawn = editor();
        drawn.add_stop(Side::Right, 10_000, 9_000, StopKind::Curb).unwrap();
        let mut other = Editor::from_street(&s, &drawn.snapshot(), 0).with_frame(Some(ZoneFrame { length_mm: LEN + 5_000, from_end: ZoneEnd::First }));
        assert_eq!(other.remove_stop(0), Err(CurbRefusal::Elsewhere));
        assert_eq!(other.add_stop(Side::Left, 0, 9_000, StopKind::Curb), Err(CurbRefusal::Elsewhere));
        assert_eq!(other.snapshot().stops.len(), 1);
    }

    #[test]
    fn an_index_that_is_not_a_stop_does_nothing() {
        let mut e = editor();
        assert_eq!(e.set_stop_kind(3, StopKind::Bulb), Ok(()));
        assert_eq!(e.set_stop_shelter(3, true), Ok(()));
        assert_eq!(e.name_stop(3, Some("x")), Ok(()));
        assert_eq!(e.resize_stop(3, 9_000), Ok(()));
        assert_eq!(e.move_stop(3, 0), Ok(()));
        assert_eq!(e.remove_stop(3), Ok(()));
        assert_eq!(e.view().revisions.len(), 0);
    }

    #[test]
    fn each_edit_is_one_revision_with_its_label() {
        let mut e = editor();
        e.add_stop(Side::Right, 20_000, 12_000, StopKind::Curb).unwrap();
        e.set_stop_kind(0, StopKind::Bulb).unwrap();
        e.set_stop_shelter(0, true).unwrap();
        e.name_stop(0, Some("  Mont-Royal ")).unwrap();
        e.resize_stop(0, 10_000).unwrap();
        e.move_stop(0, 25_000).unwrap();
        let keys: Vec<&str> = e.view().revisions.iter().map(|r| r.label.key).collect();
        assert_eq!(keys, ["rev-stop-add", "rev-stop-kind", "rev-stop-shelter", "rev-stop-name", "rev-stop-resize", "rev-stop-move"]);
        let s = &e.snapshot().stops[0];
        assert_eq!((s.kind, s.shelter, s.name.as_deref(), s.length_mm, s.from_mm), (StopKind::Bulb, true, Some("Mont-Royal"), 10_000, 25_000));
        // Nothing changed, nothing recorded.
        e.set_stop_kind(0, StopKind::Bulb).unwrap();
        e.set_stop_shelter(0, true).unwrap();
        e.name_stop(0, Some("Mont-Royal")).unwrap();
        e.move_stop(0, 25_040).unwrap();
        assert_eq!(e.view().revisions.len(), 6);
        // A blank name takes the name away.
        e.name_stop(0, Some("   ")).unwrap();
        assert_eq!(e.snapshot().stops[0].name, None);
        e.remove_stop(0).unwrap();
        assert!(e.snapshot().stops.is_empty());
        assert_eq!(e.view().revisions.last().unwrap().label.key, "rev-stop-remove");
    }

    #[test]
    fn undo_and_start_over_restore_the_stops() {
        let mut e = editor();
        e.add_stop(Side::Right, 20_000, 12_000, StopKind::Curb).unwrap();
        let first = e.snapshot();
        e.set_stop_kind(0, StopKind::Island).unwrap();
        e.set_stop_shelter(0, true).unwrap();
        e.name_stop(0, Some("Mont-Royal")).unwrap();
        let full = e.snapshot();
        assert_eq!(
            full.stops[0],
            Stop { side: Side::Right, from_mm: 20_000, length_mm: 12_000, kind: StopKind::Island, shelter: true, name: Some("Mont-Royal".into()) }
        );
        assert!(e.undo());
        assert_eq!(e.snapshot().stops[0].name, None);
        assert!(e.undo());
        assert!(!e.snapshot().stops[0].shelter);
        assert!(e.undo());
        assert_eq!(e.snapshot(), first);
        assert!(e.redo() && e.redo() && e.redo());
        assert_eq!(e.snapshot(), full);
        assert!(e.reset());
        assert!(e.snapshot().stops.is_empty());
        assert_eq!((e.snapshot().from_end, e.snapshot().length_mm), (ZoneEnd::First, None), "no stops, no zones: no anchor kept");
        assert!(e.undo());
        assert_eq!(e.snapshot(), full, "Start over can be undone");
    }

    #[test]
    fn zone_edits_leave_stops_in_place() {
        let mut e = editor();
        e.add_stop(Side::Right, 20_000, 12_000, StopKind::Curb).unwrap();
        let stops = e.snapshot().stops;
        e.split_zone(Some(25_000)).unwrap();
        e.split_zone(Some(40_000)).unwrap();
        assert_eq!(e.snapshot().stops, stops);
        e.select_zone(1);
        e.move_boundary(30_000).unwrap();
        assert_eq!(e.snapshot().stops, stops);
        e.merge_zone(true, Keep::Selected).unwrap();
        assert_eq!(e.snapshot().stops, stops);
        // Merging back to no zones keeps the anchor the stop needs.
        let mut e = editor();
        e.add_stop(Side::Right, 20_000, 12_000, StopKind::Curb).unwrap();
        e.split_zone(Some(25_000)).unwrap();
        e.merge_zone(false, Keep::Selected).unwrap();
        let s = e.snapshot();
        assert!(s.zones.is_empty());
        assert_eq!((s.stops, s.length_mm, s.from_end), (stops, Some(LEN), ZoneEnd::First));
    }

    #[test]
    fn a_drag_is_one_revision_and_other_stop_edits_wait_for_it() {
        let mut e = editor();
        e.add_stop(Side::Right, 20_000, 9_000, StopKind::Curb).unwrap();
        e.begin_gesture();
        e.move_stop(0, 22_000).unwrap();
        e.move_stop(0, 24_000).unwrap();
        assert_eq!(e.snapshot().stops[0].from_mm, 24_000);
        assert_eq!(e.add_stop(Side::Left, 0, 9_000, StopKind::Curb), Err(CurbRefusal::Dragging));
        assert_eq!(e.remove_stop(0), Err(CurbRefusal::Dragging));
        assert!(e.end_gesture());
        assert_eq!(e.view().revisions.len(), 2);
        assert!(e.undo());
        assert_eq!(e.snapshot().stops[0].from_mm, 20_000);
        e.begin_gesture();
        e.move_stop(0, 30_000).unwrap();
        e.cancel_gesture();
        assert_eq!(e.snapshot().stops[0].from_mm, 20_000);
    }

    #[test]
    fn a_street_reversed_keeps_the_stops_on_the_same_curb_and_end() {
        let mut e = editor();
        e.add_stop(Side::Left, 12_000, 9_000, StopKind::Curb).unwrap();
        let s = e.snapshot();
        let r = s.reversed();
        assert_eq!(r.stops, vec![stop(Side::Right, 12_000, 9_000, StopKind::Curb)]);
        assert_eq!(r.from_end, ZoneEnd::Last);
        assert_eq!(r.reversed(), s);
        assert!(s.is_sound());
        let mut broken = s.clone();
        broken.length_mm = None;
        assert!(!broken.is_sound(), "stops need the length they were drawn against");
    }

    #[test]
    fn the_end_zone_street_carries_no_stops() {
        let mut e = editor();
        e.split_zone(Some(30_000)).unwrap();
        e.add_stop(Side::Right, 40_000, 9_000, StopKind::Curb).unwrap();
        assert!(e.snapshot().at_end(true).stops.is_empty());
    }

    #[test]
    fn a_street_without_stops_serializes_as_before() {
        let s = Street::sample(0, Side::Right);
        let v = serde_json::to_value(&s).unwrap();
        assert!(v.get("stops").is_none());
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            r#"{"class":"local","name":"Sample Street 1","row_mm":18000,"side":"right","segments":[{"uid":1,"kind":0,"width_mm":3300,"material":1,"curb":1,"direction":null,"tram":false,"variants":[]},{"uid":2,"kind":5,"width_mm":2400,"material":0,"curb":null,"direction":null,"tram":false,"variants":[]},{"uid":3,"kind":3,"width_mm":3300,"material":0,"curb":null,"direction":1,"tram":false,"variants":[]},{"uid":4,"kind":3,"width_mm":3300,"material":0,"curb":null,"direction":0,"tram":false,"variants":[]},{"uid":5,"kind":5,"width_mm":2400,"material":0,"curb":null,"direction":null,"tram":false,"variants":[]},{"uid":6,"kind":0,"width_mm":3300,"material":1,"curb":1,"direction":null,"tram":false,"variants":[]}],"next_uid":7}"#,
            "bytes written before stops existed"
        );
        let mut with = s.clone();
        with.stops.push(stop(Side::Right, 0, 9_000, StopKind::Curb));
        assert!(serde_json::to_value(&with).unwrap().get("stops").is_some());
    }

    #[test]
    fn stop_round_trips() {
        let text = r#"{"side":"right","from_mm":20000,"length_mm":12000,"kind":"bulb","shelter":true,"name":"Mont-Royal"}"#;
        let s: Stop = serde_json::from_str(text).unwrap();
        assert_eq!(s, Stop { side: Side::Right, from_mm: 20_000, length_mm: 12_000, kind: StopKind::Bulb, shelter: true, name: Some("Mont-Royal".into()) });
        assert_eq!(serde_json::to_string(&s).unwrap(), text);
        let bare = r#"{"side":"left","from_mm":0,"length_mm":9000,"kind":"island","shelter":false}"#;
        let s: Stop = serde_json::from_str(bare).unwrap();
        assert_eq!((s.kind, s.name.clone()), (StopKind::Island, None));
        assert_eq!(serde_json::to_string(&s).unwrap(), bare, "no name, no key");
        assert_eq!(serde_json::to_string(&StopKind::Curb).unwrap(), "\"curb\"");
    }

    // ---- the bus's lane and the checks ----

    use crate::shared::catalogue::kind_index;
    use crate::street::model::{Check, Piece};

    /// A street from `(kind, width, direction)` pieces, first piece first.
    fn layout(pieces: &[(&str, i32, Option<usize>)]) -> Street {
        let pieces: Vec<Piece> =
            pieces.iter().map(|&(id, width_mm, direction)| Piece { kind: kind_index(id).unwrap(), width_mm, direction, variants: Vec::new() }).collect();
        Street::imported(crate::shared::catalogue::StreetClass::Local, Side::Right, &pieces)
    }

    fn region(side: Side) -> usize {
        REGIONS.iter().position(|r| r.drive_side == side).unwrap()
    }

    fn editor_with(s: &Street, driving: Side, from_end: ZoneEnd) -> Editor {
        Editor::from_street(s, s, region(driving)).with_frame(Some(ZoneFrame { length_mm: LEN, from_end }))
    }

    /// Right-hand traffic: the bus runs away (direction 0) in the last lane; the curb is the right one.
    const CURB: [(&str, i32, Option<usize>); 4] = [("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("sidewalk", 3000, None)];

    fn check(e: &Editor, id: &str) -> Check {
        e.view().checks.into_iter().find(|c| c.id == id).unwrap_or_else(|| panic!("no check {id}"))
    }

    fn ok(e: &Editor, id: &str) -> bool {
        check(e, id).ok
    }

    fn with_stop(pieces: &[(&str, i32, Option<usize>)], kind: StopKind) -> Editor {
        let mut e = editor_with(&layout(pieces), Side::Right, ZoneEnd::First);
        e.add_stop(Side::Right, 10_000, 12_000, kind).unwrap();
        e
    }

    #[test]
    fn the_bus_lane_is_found_on_the_stop_side_under_both_anchors_and_driving_sides() {
        // The section is seen from the first end: with traffic on the right the last lanes run away (0), with
        // traffic on the left the first lanes do (the importer's forward lane, `default_directions`).
        for driving in [Side::Right, Side::Left] {
            // The sample's own directions: the lane on the driving side runs away.
            let sample = Street::sample(0, driving);
            let travel: Vec<usize> = (0..sample.segments.len()).filter(|&i| sample.segments[i].kind == kind_index("travel").unwrap()).collect();
            let (d_first, d_last) = (sample.segments[travel[0]].direction, sample.segments[travel[1]].direction);
            assert_eq!(d_last == Some(0), driving == Side::Right, "the sample puts the away lane on the driving side");
            let s = layout(&[("sidewalk", 3000, None), ("bus", 3300, d_first), ("bus", 3300, d_last), ("sidewalk", 3000, None)]);
            for from_end in [ZoneEnd::First, ZoneEnd::Last] {
                assert_eq!(bus_lane(&s.segments, Side::Left, driving), Some(1), "{driving:?} {from_end:?} left");
                assert_eq!(bus_lane(&s.segments, Side::Right, driving), Some(2), "{driving:?} {from_end:?} right");
                for side in [Side::Left, Side::Right] {
                    let mut e = editor_with(&s, driving, from_end);
                    e.add_stop(side, 10_000, 12_000, StopKind::Curb).unwrap();
                    assert!(ok(&e, "stop-lane"), "{driving:?} {from_end:?} {side:?}: the anchor does not matter");
                }
            }
        }
        // A one-way street running away: only the half that carries direction 0 has a lane for the bus.
        let s = layout(&[("sidewalk", 3000, None), ("bus", 3300, Some(0)), ("bus", 3300, Some(0)), ("sidewalk", 3000, None)]);
        assert_eq!(bus_lane(&s.segments, Side::Right, Side::Right), Some(2));
        assert_eq!(bus_lane(&s.segments, Side::Left, Side::Right), None, "the left half carries traffic toward");
        assert_eq!(bus_lane(&s.segments, Side::Left, Side::Left), Some(1));
        assert_eq!(bus_lane(&s.segments, Side::Right, Side::Left), None);
        // A two-way lane counts; a travel lane does too; a wrong-way lane at the edge is passed over.
        let s = layout(&[("sidewalk", 3000, None), ("travel", 3300, Some(0)), ("bus", 3300, Some(1)), ("sidewalk", 3000, None)]);
        assert_eq!(bus_lane(&s.segments, Side::Right, Side::Right), Some(1));
        let s = layout(&[("sidewalk", 3000, None), ("bus", 3300, None), ("sidewalk", 3000, None)]);
        assert_eq!(bus_lane(&s.segments, Side::Right, Side::Right), Some(1));
        assert_eq!(bus_lane(&s.segments, Side::Left, Side::Right), Some(1));
    }

    #[test]
    fn a_curb_stop_on_a_curbside_bus_lane_passes() {
        let e = with_stop(&CURB, StopKind::Curb);
        let ids: Vec<&str> = e.view().checks.iter().map(|c| c.id).filter(|i| i.starts_with("stop-")).collect();
        assert_eq!(ids, ["stop-lane", "stop-kind", "stop-length", "stop-width", "stop-bike", "stop-shelter"]);
        for c in e.view().checks.iter().filter(|c| c.id.starts_with("stop-")) {
            assert!(c.ok, "{} fails: {:?}", c.id, c.detail);
        }
    }

    #[test]
    fn no_stop_rows_without_stops() {
        let e = editor_with(&layout(&CURB), Side::Right, ZoneEnd::First);
        assert!(e.view().checks.iter().all(|c| !c.id.starts_with("stop-")));
    }

    #[test]
    fn stop_lane_needs_a_bus_lane_going_the_stop_side_s_way() {
        let none = [("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("travel", 3300, Some(1)), ("sidewalk", 3000, None)];
        let e = with_stop(&none, StopKind::Curb);
        let c = check(&e, "stop-lane");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-stop-lane");
        assert_eq!(c.zone.as_ref().unwrap().key, "stop-instance");
        assert!(ok(&with_stop(&CURB, StopKind::Curb), "stop-lane"));
        // The other checks do not pile on a stop with no lane.
        assert!(ok(&e, "stop-kind") && ok(&e, "stop-width") && ok(&e, "stop-bike"));
    }

    #[test]
    fn stop_kind_needs_what_the_kind_asks_of_the_section() {
        // A bulb needs parking or loading beside the lane; a curb stop must not have it; an island needs a bike lane
        // behind a parking, loading or buffer piece.
        let parked = [("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("parking", 2400, None), ("sidewalk", 3000, None)];
        let island = [
            ("sidewalk", 3000, None),
            ("travel", 3300, Some(1)),
            ("bus", 3300, Some(0)),
            ("parking", 2400, None),
            ("bike", 1800, None),
            ("sidewalk", 3000, None),
        ];
        let c = check(&with_stop(&CURB, StopKind::Bulb), "stop-kind");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-stop-kind");
        assert!(ok(&with_stop(&parked, StopKind::Bulb), "stop-kind"));
        assert!(!ok(&with_stop(&parked, StopKind::Curb), "stop-kind"));
        assert!(!ok(&with_stop(&parked, StopKind::Island), "stop-kind"));
        assert!(ok(&with_stop(&CURB, StopKind::Curb), "stop-kind"));
        assert!(ok(&with_stop(&island, StopKind::Island), "stop-kind"));
        let segs = layout(&island).segments;
        assert!(kind_fits(&segs, 2, Side::Right, StopKind::Island) && !kind_fits(&segs, 2, Side::Right, StopKind::Curb));
        assert!(!ok(&with_stop(&CURB, StopKind::Island), "stop-kind"));
    }

    #[test]
    fn stop_length_wants_nine_metres() {
        let mut e = editor_with(&layout(&CURB), Side::Right, ZoneEnd::First);
        e.add_stop(Side::Right, 10_000, 8_900, StopKind::Curb).unwrap();
        let c = check(&e, "stop-length");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-stop-length");
        e.resize_stop(0, 9_000).unwrap();
        assert!(ok(&e, "stop-length"));
    }

    #[test]
    fn stop_width_measures_the_boarding_area_of_each_kind() {
        // Curb: the sidewalk run beside the lane.
        let narrow = [("sidewalk", 2300, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("sidewalk", 2300, None)];
        let c = check(&with_stop(&narrow, StopKind::Curb), "stop-width");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-stop-width");
        let exact = [("sidewalk", 2400, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("sidewalk", 2400, None)];
        assert!(ok(&with_stop(&exact, StopKind::Curb), "stop-width"));
        // A planting strip breaks the sidewalk run.
        let broken = [("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("planting", 1800, None), ("sidewalk", 2000, None)];
        assert!(!ok(&with_stop(&broken, StopKind::Curb), "stop-width"));
        // Bulb: the parking piece it stands over.
        let bulb = |w: i32| [("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("parking", w, None), ("sidewalk", 3000, None)];
        assert!(!ok(&with_stop(&bulb(2100), StopKind::Bulb), "stop-width"));
        assert!(ok(&with_stop(&bulb(2400), StopKind::Bulb), "stop-width"));
        // Island: the pieces between the lane and the bike lane.
        let island = |w: i32| {
            [("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("parking", w, None), ("bike", 1800, None), ("sidewalk", 3000, None)]
        };
        assert!(!ok(&with_stop(&island(2100), StopKind::Island), "stop-width"));
        assert!(ok(&with_stop(&island(2400), StopKind::Island), "stop-width"));
    }

    #[test]
    fn stop_bike_fails_for_riders_crossing_the_boarding_area() {
        let bike = |w: i32| [("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("bike", w, None), ("sidewalk", 3000, None)];
        let c = check(&with_stop(&bike(1800), StopKind::Curb), "stop-bike");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-stop-bike-cross");
        assert!(ok(&with_stop(&CURB, StopKind::Curb), "stop-bike"));
        // A bulb over the parking with the bike lane between the bus and the parking.
        let inside = [
            ("sidewalk", 3000, None),
            ("travel", 3300, Some(1)),
            ("bus", 3300, Some(0)),
            ("bike", 1800, None),
            ("parking", 2400, None),
            ("sidewalk", 3000, None),
        ];
        assert!(!ok(&with_stop(&inside, StopKind::Bulb), "stop-bike"));
        // An island's bike lane passes behind it, if it is wide enough.
        let island = |w: i32| {
            [("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("parking", 2400, None), ("bike", w, None), ("sidewalk", 3000, None)]
        };
        let c = check(&with_stop(&island(1400), StopKind::Island), "stop-bike");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-stop-bike-narrow");
        assert!(ok(&with_stop(&island(1500), StopKind::Island), "stop-bike"));
    }

    #[test]
    fn stop_shelter_adds_its_depth_to_the_boarding_area() {
        let walk = |w: i32| [("sidewalk", w, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("sidewalk", w, None)];
        let mut e = with_stop(&walk(3800), StopKind::Curb);
        assert!(ok(&e, "stop-shelter"), "no shelter, nothing to fit");
        e.set_stop_shelter(0, true).unwrap();
        let c = check(&e, "stop-shelter");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-stop-shelter");
        let mut e = with_stop(&walk(3900), StopKind::Curb);
        e.set_stop_shelter(0, true).unwrap();
        assert!(ok(&e, "stop-shelter"));
    }

    #[test]
    fn a_stop_without_its_lane_fails_and_comes_back() {
        let mut e = editor_with(&layout(&CURB), Side::Right, ZoneEnd::First);
        e.add_stop(Side::Right, 45_000, 12_000, StopKind::Curb).unwrap();
        e.split_zone(Some(40_000)).unwrap();
        e.select_zone(1);
        let bus = e.view().segments.iter().find(|s| s.kind == kind_index("bus").unwrap()).unwrap().uid;
        assert!(e.remove(bus));
        assert!(!ok(&e, "stop-lane"));
        assert_eq!(e.snapshot().stops.len(), 1, "the stop is kept");
        assert!(e.undo());
        assert!(ok(&e, "stop-lane"));
    }

    #[test]
    fn a_stop_is_judged_in_every_zone_it_overlaps() {
        // Zone 1 (from 30 m) has a narrower sidewalk; a stop from 25 m to 37 m fails there only.
        let mut e = editor_with(&layout(&CURB), Side::Right, ZoneEnd::First);
        e.add_stop(Side::Right, 25_000, 12_000, StopKind::Curb).unwrap();
        e.split_zone(Some(30_000)).unwrap();
        assert!(ok(&e, "stop-width"));
        e.select_zone(1);
        let walk = e.view().segments.iter().rev().find(|s| s.kind == kind_index("sidewalk").unwrap()).unwrap().uid;
        assert!(e.set_width(walk, 2000));
        assert!(!ok(&e, "stop-width"), "the second zone breaks it");
        // The same stop wholly inside the first zone is not affected.
        let mut e = editor_with(&layout(&CURB), Side::Right, ZoneEnd::First);
        e.add_stop(Side::Right, 10_000, 12_000, StopKind::Curb).unwrap();
        e.split_zone(Some(30_000)).unwrap();
        e.select_zone(1);
        let walk = e.view().segments.iter().rev().find(|s| s.kind == kind_index("sidewalk").unwrap()).unwrap().uid;
        assert!(e.set_width(walk, 2000));
        assert!(ok(&e, "stop-width"));
    }

    #[test]
    fn a_stop_is_named_by_its_kind_and_place_or_by_its_name() {
        let mut e = with_stop(&[("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("travel", 3300, Some(1)), ("sidewalk", 3000, None)], StopKind::Curb);
        let named = check(&e, "stop-lane").zone.unwrap();
        assert_eq!(named.key, "stop-instance");
        assert_eq!(named.args.len(), 3);
        e.name_stop(0, Some("Mont-Royal <b>")).unwrap();
        let named = check(&e, "stop-lane").zone.unwrap();
        assert_eq!(named.key, "stop-instance-named");
        assert!(named.args.contains(&("name", Arg::Text("Mont-Royal <b>".into()))));
    }
}
