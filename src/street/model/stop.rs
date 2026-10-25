//! Transit stops: a stretch of a street's edge where buses stop, drawn as a curb stop, a bulb or a boarding island.
//!
//! Like the curbside uses (`curb.rs`) they belong to the street, not to a section: a position is measured along the
//! whole street from its anchored end (`Street::from_end`), so splitting, merging or moving zones leaves them where
//! they are. A stop may overlap a curbside use; two stops on one side may not overlap each other. A street with
//! none is written, drawn and checked exactly as before.

use serde::{Deserialize, Serialize};

use super::curb::CurbRefusal;
use super::{Arg, Content, Editor, SNAP_MM, Said, Street, ZoneFrame, snap};
use crate::shared::catalogue::Side;

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
}
