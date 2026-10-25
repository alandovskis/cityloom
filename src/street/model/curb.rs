//! Curbside programmes: the parking or loading lane at a street's edge, divided along the street into an ordered
//! list of uses (a loading bay, a tree, a few parking spaces, a patio), each with an extent.
//!
//! They belong to the street, not to a section: a position is measured along the whole street from its anchored end
//! (`Street::from_end`), as a zone boundary is, so splitting, merging or moving zones leaves them where they are.
//! A street with none is written, drawn and checked exactly as before.

use serde::{Deserialize, Serialize};

use super::{Arg, Content, Editor, SNAP_MM, Said, Segment, Street, ZoneFrame, snap};
use crate::shared::catalogue::{KINDS, Side};

/// What a stretch of curbside is used for. Each is drawn as an existing catalogue kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Use {
    Parking,
    Loading,
    Tree,
    Planting,
    RainGarden,
    Patio,
    BikeParking,
    Platform,
}

impl Use {
    /// The catalogue kind it is drawn as.
    pub fn kind_id(self) -> &'static str {
        match self {
            Use::Parking => "parking",
            Use::Loading => "loading",
            Use::Tree | Use::Planting | Use::RainGarden => "planting",
            Use::Patio => "terrace",
            Use::BikeParking => "bikerack",
            Use::Platform => "sidewalk",
        }
    }

    /// The catalogue material it is drawn in, where it is not the kind's own.
    pub fn material(self) -> Option<&'static str> {
        match self {
            Use::Tree => Some("trees"),
            Use::RainGarden => Some("planted"),
            _ => None,
        }
    }

    /// Whether it stands at sidewalk height (a curb extension) rather than lying on the roadway.
    pub fn raised(self) -> bool {
        !matches!(self, Use::Parking | Use::Loading)
    }

    /// The shortest it should be, in millimetres. Placeholders, UNVERIFIED: a parking space of 5.5 m, a loading
    /// bay of 9 m, a tree pit or planting bed of 2 m, a rain garden or patio of 4 m, a bike corral of 2 m and a
    /// transit platform of 9 m. Shorter is allowed and fails the `curb-length` check.
    pub fn min_mm(self) -> i32 {
        match self {
            Use::Parking => 5_500,
            Use::Loading | Use::Platform => 9_000,
            Use::Tree | Use::Planting | Use::BikeParking => 2_000,
            Use::RainGarden | Use::Patio => 4_000,
        }
    }

    /// The message that names it.
    pub fn name_key(self) -> &'static str {
        match self {
            Use::Parking => "curb-use-parking",
            Use::Loading => "curb-use-loading",
            Use::Tree => "curb-use-tree",
            Use::Planting => "curb-use-planting",
            Use::RainGarden => "curb-use-rain-garden",
            Use::Patio => "curb-use-patio",
            Use::BikeParking => "curb-use-bike-parking",
            Use::Platform => "curb-use-platform",
        }
    }
}

/// One use of the curbside, along `length_mm` of the street from `from_mm`.
///
/// `side` is the side of the section, not the region's driving side: `Left` is the first piece of the section's
/// list (the street's left looking from its first end), `Right` the last. It does not change with the end the
/// positions are measured from; `Street::reversed` swaps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Instance {
    pub side: Side,
    /// Millimetres from the anchored end (`Street::from_end`) to where it starts, a multiple of `SNAP_MM`.
    pub from_mm: i32,
    pub length_mm: i32,
    #[serde(rename = "use")]
    pub use_: Use,
}

impl Instance {
    fn to_mm(&self) -> i32 {
        self.from_mm + self.length_mm
    }
}

/// Why a curbside edit was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurbRefusal {
    /// Both ends of the street are cut by the map's box, so there is no end to measure from.
    NoAnchor,
    /// The programme was drawn on a road of another length or anchored at its other end.
    Elsewhere,
    /// It would overlap another use on its side.
    Overlap,
    /// It would start before the street or run past its end.
    TooLong,
    /// The side's curbside lane changes with the time of day there.
    TimeWindows,
    /// A drag is under way.
    Dragging,
}

impl CurbRefusal {
    pub fn said(self) -> Said {
        Said::new(match self {
            CurbRefusal::NoAnchor => "zone-refused-no-anchor",
            CurbRefusal::Elsewhere => "curb-refused-elsewhere",
            CurbRefusal::Overlap => "curb-refused-overlap",
            CurbRefusal::TooLong => "curb-refused-too-long",
            CurbRefusal::TimeWindows => "curb-refused-time-windows",
            CurbRefusal::Dragging => "curb-refused-dragging",
        })
    }
}

/// Where the curbside piece of a side is.
enum Lane {
    Piece(usize),
    /// A parking or loading piece that changes with the time of day.
    Timed,
    Absent,
}

fn locate(segs: &[Segment], side: Side) -> Lane {
    let order: Vec<usize> = match side {
        Side::Left => (0..segs.len()).collect(),
        Side::Right => (0..segs.len()).rev().collect(),
    };
    for i in order {
        let s = &segs[i];
        if matches!(KINDS[s.kind].id, "parking" | "loading") {
            return if s.variants.is_empty() { Lane::Piece(i) } else { Lane::Timed };
        }
        if s.has_motor() {
            break;
        }
    }
    Lane::Absent
}

/// The curbside piece of `side` in a section: the parking or loading piece nearest that side's edge with only
/// non-motor pieces between it and the edge (so a parking-protected bike lane counts). `None` when there is none, or
/// when that piece changes with the time of day, so a gap never changes meaning with the clock.
pub fn curbside(segs: &[Segment], side: Side) -> Option<usize> {
    match locate(segs, side) {
        Lane::Piece(i) => Some(i),
        _ => None,
    }
}

impl Content {
    /// The section in force at a position along the street: the zone whose span holds it, else the base.
    fn section_at(&self, at_mm: i32, length_mm: i32) -> &[Segment] {
        (0..self.zone_count())
            .find(|&z| {
                let (from, to) = self.span(z, length_mm);
                from <= at_mm && at_mm < to
            })
            .map_or(&self.segments, |z| self.section(z))
    }

    /// The instances in their canonical order: left side first, then by position.
    pub(super) fn sort_curb(&mut self) {
        self.curb.sort_by_key(|i| (i.side == Side::Right, i.from_mm));
    }
}

impl Street {
    /// Swaps the sides of the instances for a street seen from its other end. Their positions stay measured from
    /// the same physical end (`reverse_zones` flips the label of that end).
    pub(super) fn reverse_curb(&mut self) {
        for i in &mut self.curb {
            i.side = match i.side {
                Side::Left => Side::Right,
                Side::Right => Side::Left,
            };
        }
        self.curb.sort_by_key(|i| (i.side == Side::Right, i.from_mm));
    }

    /// Whether the instances can be placed: they need the length they were drawn against, and lie in front of it.
    pub(super) fn curb_is_sound(&self) -> bool {
        self.curb.is_empty() || (self.length_mm.is_some() && self.curb.iter().all(|i| i.from_mm >= 0 && i.length_mm > 0))
    }
}

impl Editor {
    fn curb_frame(&self, allow_drag: bool) -> Result<ZoneFrame, CurbRefusal> {
        if self.gesture.is_some() && !allow_drag {
            return Err(CurbRefusal::Dragging);
        }
        let frame = self.frame.ok_or(CurbRefusal::NoAnchor)?;
        if self.elsewhere().is_some() {
            return Err(CurbRefusal::Elsewhere);
        }
        Ok(frame)
    }

    /// Whether `candidate` may stand where it is, ignoring the instance `own` (the one being changed).
    fn curb_fits(&self, candidate: &Instance, own: Option<usize>, frame: ZoneFrame) -> Result<(), CurbRefusal> {
        let content = &self.states[self.cursor].content;
        if candidate.from_mm < 0 || candidate.to_mm() > frame.length_mm {
            return Err(CurbRefusal::TooLong);
        }
        let overlaps = content
            .curb
            .iter()
            .enumerate()
            .any(|(k, o)| Some(k) != own && o.side == candidate.side && o.from_mm < candidate.to_mm() && candidate.from_mm < o.to_mm());
        if overlaps {
            return Err(CurbRefusal::Overlap);
        }
        if matches!(locate(content.section_at(candidate.from_mm, frame.length_mm), candidate.side), Lane::Timed) {
            return Err(CurbRefusal::TimeWindows);
        }
        Ok(())
    }

    fn curb_label(key: &'static str, i: &Instance) -> Said {
        Said::new(key).with("use", Arg::MsgLower(i.use_.name_key())).with("at", Arg::Length(i.from_mm))
    }

    /// Records a change of the instances as one revision, writing the length and end they are measured against.
    fn curb_change(&mut self, label: Said, frame: ZoneFrame, f: impl FnOnce(&mut Vec<Instance>)) {
        self.edit_content(label, |c| {
            f(&mut c.curb);
            c.sort_curb();
            c.from_end = frame.from_end;
            c.length_mm = Some(frame.length_mm);
            true
        });
    }

    /// Adds a use on `side` from `from_mm` (snapped to `SNAP_MM`) for `length_mm` (snapped, at least `SNAP_MM`).
    pub fn add_curb(&mut self, side: Side, from_mm: i32, length_mm: i32, use_: Use) -> Result<(), CurbRefusal> {
        let frame = self.curb_frame(false)?;
        let i = Instance { side, from_mm: snap(from_mm), length_mm: snap(length_mm).max(SNAP_MM), use_ };
        self.curb_fits(&i, None, frame)?;
        self.curb_change(Self::curb_label("rev-curb-add", &i), frame, |curb| curb.push(i));
        Ok(())
    }

    /// Changes the use of instance `i` (an index into the snapshot's `curb`). Nothing happens for an index that is not one.
    pub fn set_curb_use(&mut self, i: usize, use_: Use) -> Result<(), CurbRefusal> {
        let frame = self.curb_frame(false)?;
        let Some(&old) = self.states[self.cursor].content.curb.get(i) else { return Ok(()) };
        let new = Instance { use_, ..old };
        self.curb_fits(&new, Some(i), frame)?;
        self.curb_change(Self::curb_label("rev-curb-use", &new), frame, |curb| curb[i] = new);
        Ok(())
    }

    /// Gives instance `i` a new length (snapped, at least `SNAP_MM`), from where it starts.
    pub fn resize_curb(&mut self, i: usize, length_mm: i32) -> Result<(), CurbRefusal> {
        let frame = self.curb_frame(false)?;
        let Some(&old) = self.states[self.cursor].content.curb.get(i) else { return Ok(()) };
        let new = Instance { length_mm: snap(length_mm).max(SNAP_MM), ..old };
        self.curb_fits(&new, Some(i), frame)?;
        self.curb_change(Self::curb_label("rev-curb-resize", &new), frame, |curb| curb[i] = new);
        Ok(())
    }

    /// Moves instance `i` to start at `from_mm` (snapped). Inside a gesture it is shown as it goes and recorded as one
    /// revision at the end.
    pub fn move_curb(&mut self, i: usize, from_mm: i32) -> Result<(), CurbRefusal> {
        let frame = self.curb_frame(true)?;
        let Some(&old) = self.states[self.cursor].content.curb.get(i) else { return Ok(()) };
        let new = Instance { from_mm: snap(from_mm), ..old };
        self.curb_fits(&new, Some(i), frame)?;
        if new == old {
            return Ok(());
        }
        let label = Self::curb_label("rev-curb-move", &new);
        if self.gesture.is_some() {
            let c = &mut self.states[self.cursor].content;
            c.curb[i] = new;
            c.sort_curb();
            c.from_end = frame.from_end;
            c.length_mm = Some(frame.length_mm);
            self.pending_label = Some(label);
        } else {
            self.curb_change(label, frame, |curb| curb[i] = new);
        }
        Ok(())
    }

    /// Removes instance `i`.
    pub fn remove_curb(&mut self, i: usize) -> Result<(), CurbRefusal> {
        let frame = self.curb_frame(false)?;
        let Some(&old) = self.states[self.cursor].content.curb.get(i) else { return Ok(()) };
        self.curb_change(Self::curb_label("rev-curb-remove", &old), frame, |curb| {
            curb.remove(i);
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Keep, Piece, ZoneEnd};
    use super::*;
    use crate::shared::catalogue::kind_index;

    const LEN: i32 = 60_000;

    fn frame() -> Option<ZoneFrame> {
        Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::First })
    }

    fn editor_on(s: &Street) -> Editor {
        Editor::from_street(s, s, 0).with_frame(frame())
    }

    fn editor() -> Editor {
        editor_on(&Street::sample(0, Side::Right))
    }

    fn piece(id: &str, width_mm: i32) -> Piece {
        Piece { kind: kind_index(id).unwrap(), width_mm, direction: None, variants: Vec::new() }
    }

    fn street_of(ids: &[(&str, i32)]) -> Street {
        let pieces: Vec<Piece> = ids.iter().map(|(id, w)| piece(id, *w)).collect();
        Street::imported(crate::shared::catalogue::StreetClass::Local, Side::Right, &pieces)
    }

    #[test]
    fn curbside_is_the_outer_parking_lane() {
        let s = Street::sample(0, Side::Right);
        assert_eq!(curbside(&s.segments, Side::Left), Some(1));
        assert_eq!(curbside(&s.segments, Side::Right), Some(4));
        // Trees and planting between the sidewalk and the lane do not hide it.
        let avenue = Street::sample(1, Side::Right);
        assert_eq!(curbside(&avenue.segments, Side::Left), Some(2));
        assert_eq!(curbside(&avenue.segments, Side::Right), Some(8));
        // A street with no parking has none.
        assert_eq!(curbside(&Street::sample(3, Side::Right).segments, Side::Left), None);
    }

    #[test]
    fn a_parking_protected_bike_lane_still_has_a_curbside_piece() {
        let s = street_of(&[("sidewalk", 2500), ("bike", 1800), ("parking", 2400), ("travel", 3300), ("travel", 3300), ("sidewalk", 2500)]);
        assert_eq!(curbside(&s.segments, Side::Left), Some(2));
        assert_eq!(curbside(&s.segments, Side::Right), None, "a travel lane is the first motor piece from the right");
    }

    #[test]
    fn no_curbside_when_the_lane_has_time_windows() {
        let mut s = Street::sample(0, Side::Right);
        let bus = kind_index("bus").unwrap();
        let mut e = Editor::from_street(&s, &s, 0);
        let uid = s.segments[1].uid;
        assert!(e.add_variant(uid), "a parking lane can take a bus window");
        let _ = bus;
        s = e.snapshot();
        assert_eq!(curbside(&s.segments, Side::Left), None);
        assert_eq!(curbside(&s.segments, Side::Right), Some(4));
        let mut e = editor_on(&s);
        assert_eq!(e.add_curb(Side::Left, 10_000, 6_000, Use::Parking), Err(CurbRefusal::TimeWindows));
        assert!(e.add_curb(Side::Right, 10_000, 6_000, Use::Parking).is_ok());
    }

    #[test]
    fn add_snaps_and_orders() {
        let mut e = editor();
        e.add_curb(Side::Right, 30_049, 6_040, Use::Loading).unwrap();
        e.add_curb(Side::Left, 5_000, 5_500, Use::Parking).unwrap();
        e.add_curb(Side::Right, 10_000, 2_000, Use::Tree).unwrap();
        let c = e.snapshot().curb;
        assert_eq!(
            c,
            vec![
                Instance { side: Side::Left, from_mm: 5_000, length_mm: 5_500, use_: Use::Parking },
                Instance { side: Side::Right, from_mm: 10_000, length_mm: 2_000, use_: Use::Tree },
                Instance { side: Side::Right, from_mm: 30_000, length_mm: 6_000, use_: Use::Loading },
            ]
        );
        let s = e.snapshot();
        assert_eq!((s.length_mm, s.from_end), (Some(LEN), ZoneEnd::First));
        assert_eq!(e.view().revisions.last().unwrap().label.key, "rev-curb-add");
    }

    #[test]
    fn overlap_is_refused() {
        let mut e = editor();
        e.add_curb(Side::Right, 10_000, 6_000, Use::Parking).unwrap();
        assert_eq!(e.add_curb(Side::Right, 15_000, 2_000, Use::Tree), Err(CurbRefusal::Overlap));
        assert_eq!(e.add_curb(Side::Right, 5_000, 5_100, Use::Tree), Err(CurbRefusal::Overlap));
        assert!(e.add_curb(Side::Right, 16_000, 2_000, Use::Tree).is_ok(), "touching is not overlapping");
        assert!(e.add_curb(Side::Left, 15_000, 2_000, Use::Tree).is_ok(), "the other side is free");
        assert_eq!(e.move_curb(1, 14_000), Err(CurbRefusal::Overlap));
        assert_eq!(e.resize_curb(1, 7_000), Err(CurbRefusal::Overlap));
        assert_eq!(e.snapshot().curb.len(), 3);
    }

    #[test]
    fn past_the_end_is_refused() {
        let mut e = editor();
        assert_eq!(e.add_curb(Side::Right, 55_000, 6_000, Use::Parking), Err(CurbRefusal::TooLong));
        assert_eq!(e.add_curb(Side::Right, -1_000, 6_000, Use::Parking), Err(CurbRefusal::TooLong));
        assert!(e.add_curb(Side::Right, 54_000, 6_000, Use::Parking).is_ok(), "up to the end is fine");
        assert_eq!(e.resize_curb(0, 7_000), Err(CurbRefusal::TooLong));
        assert_eq!(e.move_curb(0, 54_100), Err(CurbRefusal::TooLong));
    }

    #[test]
    fn curb_needs_a_real_end() {
        let s = Street::sample(0, Side::Right);
        let mut e = Editor::from_street(&s, &s, 0);
        assert_eq!(e.add_curb(Side::Right, 0, 6_000, Use::Parking), Err(CurbRefusal::NoAnchor));
        assert!(e.snapshot().curb.is_empty());
        // A programme drawn on a road of another length is not rearranged here.
        let mut e = editor();
        e.add_curb(Side::Right, 0, 6_000, Use::Parking).unwrap();
        let drawn = e.snapshot();
        let mut e = Editor::from_street(&drawn, &drawn, 0).with_frame(Some(ZoneFrame { length_mm: 30_000, from_end: ZoneEnd::First }));
        assert_eq!(e.add_curb(Side::Right, 10_000, 2_000, Use::Tree), Err(CurbRefusal::Elsewhere));
    }

    #[test]
    fn a_gesture_refuses_all_but_a_move() {
        let mut e = editor();
        e.add_curb(Side::Right, 10_000, 6_000, Use::Parking).unwrap();
        e.begin_gesture();
        assert_eq!(e.add_curb(Side::Right, 30_000, 2_000, Use::Tree), Err(CurbRefusal::Dragging));
        assert_eq!(e.set_curb_use(0, Use::Loading), Err(CurbRefusal::Dragging));
        assert_eq!(e.resize_curb(0, 7_000), Err(CurbRefusal::Dragging));
        assert_eq!(e.remove_curb(0), Err(CurbRefusal::Dragging));
        let before = e.view().revisions.len();
        e.move_curb(0, 20_000).unwrap();
        e.move_curb(0, 25_000).unwrap();
        assert!(e.end_gesture());
        assert_eq!(e.view().revisions.len(), before + 1, "a drag is one revision");
        assert_eq!(e.snapshot().curb[0].from_mm, 25_000);
    }

    #[test]
    fn edits_change_one_instance_each() {
        let mut e = editor();
        e.add_curb(Side::Right, 10_000, 6_000, Use::Parking).unwrap();
        e.set_curb_use(0, Use::Loading).unwrap();
        e.resize_curb(0, 9_000).unwrap();
        e.move_curb(0, 20_000).unwrap();
        assert_eq!(e.snapshot().curb, vec![Instance { side: Side::Right, from_mm: 20_000, length_mm: 9_000, use_: Use::Loading }]);
        let ids: Vec<_> = e.view().revisions.iter().map(|r| r.label.key.to_string()).collect();
        assert_eq!(&ids[ids.len() - 4..], ["rev-curb-add", "rev-curb-use", "rev-curb-resize", "rev-curb-move"]);
        e.remove_curb(0).unwrap();
        assert!(e.snapshot().curb.is_empty());
        let s = e.snapshot();
        assert_eq!((s.length_mm, s.from_end), (None, ZoneEnd::First), "no programme and no zones leaves no anchor");
        assert_eq!(e.view().revisions.last().unwrap().label.key, "rev-curb-remove");
    }

    #[test]
    fn zone_edits_leave_instances_in_place() {
        let mut e = editor();
        e.add_curb(Side::Right, 12_000, 5_500, Use::Parking).unwrap();
        e.split_zone(Some(20_000)).unwrap();
        e.split_zone(Some(40_000)).unwrap();
        assert_eq!(e.snapshot().curb.len(), 1);
        e.select_zone(1);
        e.move_boundary(25_000).unwrap();
        assert_eq!(e.snapshot().curb, vec![Instance { side: Side::Right, from_mm: 12_000, length_mm: 5_500, use_: Use::Parking }]);
        e.merge_zone(true, Keep::Selected).unwrap();
        assert_eq!(e.snapshot().curb.len(), 1);
        // A zone with no parking on the instance's side does not drop it.
        e.select_zone(0);
        assert!(e.make_pedestrian(kind_index("sidewalk").unwrap()));
        assert_eq!(e.snapshot().curb.len(), 1);
        // Merging back to no zones keeps the anchor the instance needs.
        let mut e = editor();
        e.add_curb(Side::Right, 12_000, 5_500, Use::Parking).unwrap();
        e.split_zone(Some(20_000)).unwrap();
        e.merge_zone(false, Keep::Selected).unwrap();
        let s = e.snapshot();
        assert!(s.zones.is_empty());
        assert_eq!((s.length_mm, s.from_end), (Some(LEN), ZoneEnd::First));
    }

    #[test]
    fn undo_and_start_over_restore_the_programme() {
        let s = Street::sample(0, Side::Right);
        let mut e = editor_on(&s);
        e.add_curb(Side::Right, 12_000, 5_500, Use::Parking).unwrap();
        e.add_curb(Side::Left, 2_000, 2_000, Use::Tree).unwrap();
        assert!(e.undo());
        assert_eq!(e.snapshot().curb.len(), 1);
        assert!(e.redo());
        assert_eq!(e.snapshot().curb.len(), 2);
        assert!(e.reset());
        assert_eq!(e.snapshot().curb, Vec::new());
        assert_eq!(e.snapshot().length_mm, None);
        assert!(e.undo());
        assert_eq!(e.snapshot().curb.len(), 2);
    }

    #[test]
    fn a_street_without_instances_serializes_as_before() {
        let s = Street::sample(0, Side::Right);
        let v = serde_json::to_value(&s).unwrap();
        assert!(v.get("curb").is_none());
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            r#"{"class":"local","name":"Sample Street 1","row_mm":18000,"side":"right","segments":[{"uid":1,"kind":0,"width_mm":3300,"material":1,"curb":1,"direction":null,"tram":false,"variants":[]},{"uid":2,"kind":5,"width_mm":2400,"material":0,"curb":null,"direction":null,"tram":false,"variants":[]},{"uid":3,"kind":3,"width_mm":3300,"material":0,"curb":null,"direction":1,"tram":false,"variants":[]},{"uid":4,"kind":3,"width_mm":3300,"material":0,"curb":null,"direction":0,"tram":false,"variants":[]},{"uid":5,"kind":5,"width_mm":2400,"material":0,"curb":null,"direction":null,"tram":false,"variants":[]},{"uid":6,"kind":0,"width_mm":3300,"material":1,"curb":1,"direction":null,"tram":false,"variants":[]}],"next_uid":7}"#,
            "bytes written before curb instances existed"
        );
        let mut with = s.clone();
        with.curb.push(Instance { side: Side::Right, from_mm: 0, length_mm: 5_500, use_: Use::Parking });
        assert!(serde_json::to_value(&with).unwrap().get("curb").is_some());
    }

    #[test]
    fn instance_round_trips() {
        let text = r#"{"side":"right","from_mm":12000,"length_mm":5500,"use":"parking"}"#;
        let i: Instance = serde_json::from_str(text).unwrap();
        assert_eq!(i, Instance { side: Side::Right, from_mm: 12_000, length_mm: 5_500, use_: Use::Parking });
        assert_eq!(serde_json::to_string(&i).unwrap(), text);
        for (u, name) in [(Use::RainGarden, "rain_garden"), (Use::BikeParking, "bike_parking"), (Use::Platform, "platform")] {
            assert_eq!(serde_json::to_string(&u).unwrap(), format!("\"{name}\""));
        }
    }

    #[test]
    fn a_street_reversed_keeps_the_instances_on_the_same_curb_and_end() {
        let mut e = editor();
        e.add_curb(Side::Left, 12_000, 5_500, Use::Parking).unwrap();
        let s = e.snapshot();
        let r = s.reversed();
        assert_eq!(r.curb, vec![Instance { side: Side::Right, from_mm: 12_000, length_mm: 5_500, use_: Use::Parking }]);
        assert_eq!(r.from_end, ZoneEnd::Last);
        assert_eq!(r.reversed(), s);
    }

    #[test]
    fn uses_are_drawn_as_catalogue_kinds() {
        for u in [Use::Parking, Use::Loading, Use::Tree, Use::Planting, Use::RainGarden, Use::Patio, Use::BikeParking, Use::Platform] {
            assert!(kind_index(u.kind_id()).is_some(), "{u:?}");
            if let Some(m) = u.material() {
                assert!(crate::shared::catalogue::MATERIALS.iter().any(|x| x.id == m), "{m}");
            }
        }
        assert_eq!((Use::Tree.kind_id(), Use::Tree.material()), ("planting", Some("trees")));
        assert_eq!((Use::RainGarden.kind_id(), Use::RainGarden.material()), ("planting", Some("planted")));
        assert_eq!((Use::Patio.kind_id(), Use::BikeParking.kind_id(), Use::Platform.kind_id()), ("terrace", "bikerack", "sidewalk"));
        assert!(!Use::Parking.raised() && !Use::Loading.raised() && Use::Tree.raised() && Use::Platform.raised());
        assert_eq!([Use::Parking.min_mm(), Use::Loading.min_mm(), Use::Tree.min_mm(), Use::Planting.min_mm()], [5_500, 9_000, 2_000, 2_000]);
        assert_eq!([Use::RainGarden.min_mm(), Use::Patio.min_mm(), Use::BikeParking.min_mm(), Use::Platform.min_mm()], [4_000, 4_000, 2_000, 9_000]);
    }
}
