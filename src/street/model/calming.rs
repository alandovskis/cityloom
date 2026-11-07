//! Traffic calming: raised tables and raised crossings along a street, each an extent of the roadway lifted to a
//! plateau with a ramp at either end.
//!
//! They belong to the street, not to a section: a position is measured along the whole street from its anchored end
//! (`Street::from_end`), as a zone boundary and a curbside use are, and a table spans the whole width of the roadway,
//! so it has no side. A street with none is written, drawn and checked exactly as before.

use serde::{Deserialize, Serialize};

use super::{Arg, Content, Editor, SNAP_MM, Said, ZoneFrame, snap};
use crate::shared::catalogue::{Side, is_roadway};

/// How high the plateau of a raised table stands above the roadway, in millimetres. A placeholder, UNVERIFIED
/// (a speed table is commonly 75 to 100 mm; TAC and the MTQ guides are not checked).
pub const TABLE_HEIGHT_MM: i32 = 100;
/// How long each ramp is, in millimetres. A placeholder, UNVERIFIED.
pub const RAMP_MM: i32 = 1_500;
/// The shortest a raised table should be, ramps included, in millimetres. A placeholder, UNVERIFIED; a shorter one
/// is allowed and fails its check.
pub const MIN_TABLE_MM: i32 = 6_000;
/// The shortest plateau between the two ramps, in millimetres. A placeholder, UNVERIFIED.
pub const MIN_PLATEAU_MM: i32 = 3_000;
/// The highest speed limit a raised table is meant for, in km/h. A placeholder, UNVERIFIED.
pub const MAX_TABLE_SPEED_KMH: u8 = 50;
/// How far from a transit stop's boarding area a table must stand, in millimetres. A placeholder, UNVERIFIED.
pub const CALMING_STOP_CLEAR_MM: i32 = 5_000;
/// How far from a junction's reach a table must stand, in millimetres. A placeholder, UNVERIFIED.
pub const CALMING_JUNCTION_CLEAR_MM: i32 = 5_000;
/// The length a new element is given when none is typed, in millimetres. A placeholder, UNVERIFIED.
pub const DEFAULT_TABLE_MM: i32 = 7_000;

/// What a raised element is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A raised table: a speed table across the roadway.
    Table,
    /// A raised pedestrian crossing: the plateau is the crossing.
    Crossing,
}

impl Kind {
    /// Every kind, in the order a choice lists them.
    pub const ALL: [Kind; 2] = [Kind::Table, Kind::Crossing];

    /// The kind's stable name, as saved and as the choice's value.
    pub fn id(self) -> &'static str {
        match self {
            Kind::Table => "table",
            Kind::Crossing => "crossing",
        }
    }

    /// The message that names it.
    pub fn name_key(self) -> &'static str {
        match self {
            Kind::Table => "calming-kind-table",
            Kind::Crossing => "calming-kind-crossing",
        }
    }
}

/// One raised element, along `length_mm` of the street from `from_mm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Calming {
    /// Millimetres from the anchored end (`Street::from_end`) to where it starts, a multiple of `SNAP_MM`.
    pub from_mm: i32,
    pub length_mm: i32,
    pub kind: Kind,
}

impl Calming {
    pub fn to_mm(&self) -> i32 {
        self.from_mm + self.length_mm
    }
}

/// Why an edit of a raised element was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalmingRefusal {
    /// Both ends of the street are cut by the map's box, so there is no end to measure from.
    NoAnchor,
    /// The elements were drawn on a road of another length or anchored at its other end.
    Elsewhere,
    /// A drag is under way.
    Dragging,
    /// It would start before the street or run past its end.
    Outside,
    /// It would overlap another element.
    Overlap,
    /// It would be shorter than the kind's minimum.
    TooShort { min_mm: i32 },
    /// The street is a freeway.
    Freeway,
    /// The street has no roadway to raise.
    NoRoadway,
    /// The roadway changes with the time of day there.
    TimeWindows,
}

impl CalmingRefusal {
    /// The refusal, in words.
    pub fn said(self) -> Said {
        let key = match self {
            CalmingRefusal::NoAnchor => "calming-refused-no-anchor",
            CalmingRefusal::Elsewhere => "calming-refused-elsewhere",
            CalmingRefusal::Dragging => "calming-refused-dragging",
            CalmingRefusal::Outside => "calming-refused-outside",
            CalmingRefusal::Overlap => "calming-refused-overlap",
            CalmingRefusal::TooShort { .. } => "calming-refused-too-short",
            CalmingRefusal::Freeway => "calming-refused-freeway",
            CalmingRefusal::NoRoadway => "calming-refused-no-roadway",
            CalmingRefusal::TimeWindows => "calming-refused-time-windows",
        };
        let said = Said::new(key);
        match self {
            CalmingRefusal::TooShort { min_mm } => said.with("min", super::Arg::Length(min_mm)),
            _ => said,
        }
    }
}

impl Content {
    /// The elements in their canonical order: by position.
    pub(super) fn sort_calming(&mut self) {
        self.calming.sort_by_key(|c| c.from_mm);
    }
}

impl super::Street {
    /// Whether the elements can be placed: they need the length they were drawn against, and lie in front of it.
    pub(super) fn calming_is_sound(&self) -> bool {
        self.calming.is_empty() || (self.length_mm.is_some() && self.calming.iter().all(|c| c.from_mm >= 0 && c.length_mm > 0))
    }
}

/// One raised element as the strip and the checks read it.
#[derive(Clone, Debug, Serialize)]
pub struct CalmingView {
    pub from_mm: i32,
    pub length_mm: i32,
    pub kind: Kind,
    /// The kind's name.
    pub name: Said,
    /// Whether it is the chosen one.
    pub selected: bool,
    /// Whether the elements were drawn on a road of another length or from its other end (shown, not rearranged).
    pub elsewhere: bool,
}

impl Editor {
    /// Whether `candidate` may stand where it is, ignoring the element `own` (the one being changed).
    fn calming_fits(&self, candidate: &Calming, own: Option<usize>, frame: ZoneFrame) -> Result<(), CalmingRefusal> {
        if self.class().is_freeway() {
            return Err(CalmingRefusal::Freeway);
        }
        let content = &self.states[self.cursor].content;
        if candidate.from_mm < 0 || candidate.to_mm() > frame.length_mm {
            return Err(CalmingRefusal::Outside);
        }
        if candidate.length_mm < MIN_TABLE_MM {
            return Err(CalmingRefusal::TooShort { min_mm: MIN_TABLE_MM });
        }
        let overlaps = content.calming.iter().enumerate().any(|(k, o)| Some(k) != own && o.from_mm < candidate.to_mm() && candidate.from_mm < o.to_mm());
        if overlaps {
            return Err(CalmingRefusal::Overlap);
        }
        // Every zone the element covers: a roadway that changes with the clock cannot be raised, and there must be one.
        let sections = content.sections_over(candidate.from_mm, candidate.to_mm(), frame.length_mm);
        let timed = |g: &super::Segment| !g.variants.is_empty() && (is_roadway(g.kind) || g.variants.iter().any(|v| is_roadway(v.kind)));
        if sections.iter().any(|section| section.iter().any(timed)) {
            return Err(CalmingRefusal::TimeWindows);
        }
        if sections.iter().any(|section| !section.iter().any(|g| is_roadway(g.kind))) {
            return Err(CalmingRefusal::NoRoadway);
        }
        Ok(())
    }

    fn calming_label(key: &'static str, c: &Calming) -> Said {
        Said::new(key).with("kind", Arg::MsgLower(c.kind.name_key())).with("at", Arg::Length(c.from_mm))
    }

    /// Records a change of the elements as one revision, writing the length and end they are measured against.
    fn calming_change(&mut self, label: Said, frame: ZoneFrame, f: impl FnOnce(&mut Vec<Calming>)) {
        self.edit_content(label, |c| {
            f(&mut c.calming);
            c.sort_calming();
            c.from_end = frame.from_end;
            c.length_mm = Some(frame.length_mm);
            true
        });
    }

    /// Changes element `i` (an index into the snapshot's `calming`) into `change(old)` if it fits, as one revision.
    /// Nothing happens for an index that is not one.
    fn calming_replace(&mut self, i: usize, label_key: &'static str, change: impl FnOnce(&Calming) -> Calming) -> Result<(), CalmingRefusal> {
        let frame = self.curb_frame(false).map_err(refusal)?;
        let Some(&old) = self.states[self.cursor].content.calming.get(i) else { return Ok(()) };
        let new = change(&old);
        self.calming_fits(&new, Some(i), frame)?;
        self.calming_change(Self::calming_label(label_key, &new), frame, |list| list[i] = new);
        Ok(())
    }

    /// Adds an element from `from_mm` (snapped to `SNAP_MM`) for `length_mm` (snapped), as one revision, and chooses
    /// it. Returns its index.
    pub fn add_calming(&mut self, kind: Kind, from_mm: i32, length_mm: i32) -> Result<usize, CalmingRefusal> {
        let frame = self.curb_frame(false).map_err(refusal)?;
        let c = Calming { from_mm: snap(from_mm), length_mm: snap(length_mm).max(SNAP_MM), kind };
        self.calming_fits(&c, None, frame)?;
        self.calming_choice.pick(Side::Left, c.from_mm);
        self.calming_change(Self::calming_label("rev-calming-add", &c), frame, |list| list.push(c));
        Ok(self.states[self.cursor].content.calming.iter().position(|o| *o == c).unwrap_or(0))
    }

    /// Changes what element `i` is.
    pub fn set_calming_kind(&mut self, i: usize, kind: Kind) -> Result<(), CalmingRefusal> {
        self.calming_replace(i, "rev-calming-kind", |old| Calming { kind, ..*old })
    }

    /// Gives element `i` a new length (snapped), from where it starts.
    pub fn set_calming_length(&mut self, i: usize, length_mm: i32) -> Result<(), CalmingRefusal> {
        self.calming_replace(i, "rev-calming-resize", |old| Calming { length_mm: snap(length_mm).max(SNAP_MM), ..*old })
    }

    /// Moves element `i` to start at `from_mm` (snapped). Inside a gesture it is shown as it goes and recorded as one
    /// revision at the end.
    pub fn move_calming(&mut self, i: usize, from_mm: i32) -> Result<(), CalmingRefusal> {
        let frame = self.curb_frame(true).map_err(refusal)?;
        let Some(&old) = self.states[self.cursor].content.calming.get(i) else { return Ok(()) };
        let new = Calming { from_mm: snap(from_mm), ..old };
        self.calming_fits(&new, Some(i), frame)?;
        if new == old {
            return Ok(());
        }
        let label = Self::calming_label("rev-calming-move", &new);
        // The choice follows the element, and a drag records one move, from where it began to where it is.
        self.calming_choice.follow(Side::Left, old.from_mm, new.from_mm);
        if self.gesture.is_some() {
            let c = &mut self.states[self.cursor].content;
            c.calming[i] = new;
            c.sort_calming();
            c.from_end = frame.from_end;
            c.length_mm = Some(frame.length_mm);
            self.pending_label = Some(label);
        } else {
            self.calming_change(label, frame, |list| list[i] = new);
        }
        Ok(())
    }

    /// Removes element `i`.
    pub fn remove_calming(&mut self, i: usize) -> Result<(), CalmingRefusal> {
        let frame = self.curb_frame(false).map_err(refusal)?;
        let Some(&old) = self.states[self.cursor].content.calming.get(i) else { return Ok(()) };
        self.calming_change(Self::calming_label("rev-calming-remove", &old), frame, |list| {
            list.remove(i);
        });
        Ok(())
    }

    /// The stretches of the street (from, to, in millimetres from the anchored end) no element covers. Empty when
    /// the street has no length to lay them along.
    pub fn calming_free_stretches(&self) -> Vec<(i32, i32)> {
        let content = &self.states[self.cursor].content;
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(0);
        let mut out = Vec::new();
        let mut at = 0;
        for c in &content.calming {
            if c.from_mm > at {
                out.push((at, c.from_mm));
            }
            at = at.max(c.to_mm());
        }
        if length_mm > at {
            out.push((at, length_mm));
        }
        out
    }

    /// The chosen element's index, if one is chosen.
    pub fn calming_sel(&self) -> Option<usize> {
        let (_, at) = self.calming_choice.at?;
        self.states[self.cursor].content.calming.iter().position(|c| c.from_mm == at)
    }

    /// Chooses element `i`, or none. A setting, not history; the choice is kept by place, so it stays on the element
    /// as the others around it change.
    pub fn select_calming(&mut self, i: Option<usize>) {
        self.calming_choice.at = i.and_then(|i| self.states[self.cursor].content.calming.get(i)).map(|c| (Side::Left, c.from_mm));
    }

    pub(super) fn calming_view(&self) -> Vec<CalmingView> {
        let selected = self.calming_sel();
        let elsewhere = self.elsewhere().is_some();
        self.states[self.cursor]
            .content
            .calming
            .iter()
            .enumerate()
            .map(|(k, c)| CalmingView {
                from_mm: c.from_mm,
                length_mm: c.length_mm,
                kind: c.kind,
                name: Said::new(c.kind.name_key()),
                selected: selected == Some(k),
                elsewhere,
            })
            .collect()
    }
}

/// The curbside frame's refusals, as a raised element words them.
fn refusal(r: super::CurbRefusal) -> CalmingRefusal {
    match r {
        super::CurbRefusal::Dragging => CalmingRefusal::Dragging,
        super::CurbRefusal::Elsewhere => CalmingRefusal::Elsewhere,
        _ => CalmingRefusal::NoAnchor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::model::Street;

    fn table(from_mm: i32) -> Calming {
        Calming { from_mm, length_mm: 7_000, kind: Kind::Table }
    }

    #[test]
    fn kind_ids_are_stable() {
        assert_eq!(Kind::ALL, [Kind::Table, Kind::Crossing]);
        assert_eq!(Kind::Table.id(), "table");
        assert_eq!(Kind::Crossing.id(), "crossing");
        assert_eq!(serde_json::to_string(&Kind::Table).unwrap(), r#""table""#);
        assert_eq!(serde_json::to_string(&Kind::Crossing).unwrap(), r#""crossing""#);
    }

    #[test]
    fn a_street_without_calming_serialises_as_before() {
        let s = Street::sample(0, Side::Right);
        let text = serde_json::to_string(&s).unwrap();
        assert!(!text.contains("calming"), "{text}");
        let back: Street = serde_json::from_str(&text).unwrap();
        assert!(back.calming.is_empty());
    }

    #[test]
    fn a_street_with_calming_round_trips_in_a_pinned_shape() {
        let mut s = Street::sample(0, Side::Right);
        s.calming = vec![table(10_000), Calming { from_mm: 30_000, length_mm: 6_000, kind: Kind::Crossing }];
        assert_eq!(
            serde_json::to_string(&s.calming).unwrap(),
            r#"[{"from_mm":10000,"length_mm":7000,"kind":"table"},{"from_mm":30000,"length_mm":6000,"kind":"crossing"}]"#
        );
        let back: Street = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.calming, s.calming);
    }

    #[test]
    fn a_refusal_says_its_own_message_and_a_short_one_names_the_minimum() {
        assert_eq!(CalmingRefusal::Overlap.said().key, "calming-refused-overlap");
        assert_eq!(CalmingRefusal::TimeWindows.said().key, "calming-refused-time-windows");
        let short = CalmingRefusal::TooShort { min_mm: MIN_TABLE_MM }.said();
        assert_eq!(short.key, "calming-refused-too-short");
        assert_eq!(short, Said::new("calming-refused-too-short").with("min", super::super::Arg::Length(MIN_TABLE_MM)));
    }

    #[test]
    fn a_street_with_elements_is_sound_only_with_the_length_they_were_drawn_on() {
        let mut s = Street::sample(0, Side::Right);
        s.calming = vec![table(0)];
        assert!(!s.is_sound());
        s.length_mm = Some(60_000);
        assert!(s.is_sound());
    }

    use super::super::ZoneEnd;
    use crate::shared::catalogue::StreetClass;

    const LEN: i32 = 60_000;

    fn frame() -> Option<ZoneFrame> {
        Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::First })
    }

    fn editor() -> Editor {
        let s = Street::sample(0, Side::Right);
        Editor::from_street(&s, &s, 0).with_frame(frame())
    }

    fn keys(e: &Editor) -> Vec<&'static str> {
        e.view().revisions.iter().map(|r| r.label.key).collect()
    }

    #[test]
    fn add_snaps_to_100_mm_and_keeps_the_list_ordered() {
        let mut e = editor();
        assert_eq!(e.add_calming(Kind::Table, 30_049, 7_040).unwrap(), 0);
        assert_eq!(e.add_calming(Kind::Crossing, 10_051, 6_000).unwrap(), 0, "it lands before the first");
        let s = e.snapshot();
        assert_eq!(
            s.calming,
            vec![Calming { from_mm: 10_100, length_mm: 6_000, kind: Kind::Crossing }, Calming { from_mm: 30_000, length_mm: 7_000, kind: Kind::Table }]
        );
        assert_eq!((s.from_end, s.length_mm), (ZoneEnd::First, Some(LEN)), "the length is written with the elements");
        assert_eq!(keys(&e), ["rev-calming-add", "rev-calming-add"]);
        assert_eq!(e.calming_sel(), Some(0), "a new element is the chosen one");
    }

    #[test]
    fn every_refusal_says_its_own_message() {
        // No real end.
        let s = Street::sample(0, Side::Right);
        let mut bare = Editor::from_street(&s, &s, 0);
        assert_eq!(bare.add_calming(Kind::Table, 10_000, 7_000), Err(CalmingRefusal::NoAnchor));
        assert_eq!(CalmingRefusal::NoAnchor.said().key, "calming-refused-no-anchor");
        assert!(bare.snapshot().calming.is_empty());
        // Outside, overlap and too short.
        let mut e = editor();
        assert_eq!(e.add_calming(Kind::Table, LEN - 6_000, 7_000,), Err(CalmingRefusal::Outside));
        assert_eq!(e.add_calming(Kind::Table, -100, 7_000), Err(CalmingRefusal::Outside));
        assert_eq!(e.add_calming(Kind::Table, 10_000, MIN_TABLE_MM - 100), Err(CalmingRefusal::TooShort { min_mm: MIN_TABLE_MM }));
        e.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        assert_eq!(e.add_calming(Kind::Crossing, 16_900, 6_000), Err(CalmingRefusal::Overlap));
        assert_eq!(e.add_calming(Kind::Crossing, 4_100, 6_000), Err(CalmingRefusal::Overlap));
        e.add_calming(Kind::Crossing, 17_000, 6_000).unwrap();
        e.add_calming(Kind::Crossing, 4_000, 6_000).unwrap();
        assert_eq!(e.snapshot().calming.len(), 3, "touching is not overlapping");
        let said = CalmingRefusal::TooShort { min_mm: MIN_TABLE_MM }.said();
        assert_eq!(said.args.iter().find_map(|(n, a)| matches!(a, Arg::Length(MIN_TABLE_MM)).then_some(*n)), Some("min"));
        // Elsewhere.
        let mut drawn = editor();
        drawn.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        let mut other = Editor::from_street(&s, &drawn.snapshot(), 0).with_frame(Some(ZoneFrame { length_mm: LEN + 5_000, from_end: ZoneEnd::First }));
        assert_eq!(other.remove_calming(0), Err(CalmingRefusal::Elsewhere));
        assert_eq!(other.add_calming(Kind::Table, 30_000, 7_000), Err(CalmingRefusal::Elsewhere));
        // Dragging.
        let mut d = editor();
        d.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        d.begin_gesture();
        assert_eq!(d.add_calming(Kind::Table, 30_000, 7_000), Err(CalmingRefusal::Dragging));
        assert_eq!(d.remove_calming(0), Err(CalmingRefusal::Dragging));
        assert_eq!(d.set_calming_length(0, 8_000), Err(CalmingRefusal::Dragging));
        assert_eq!(d.set_calming_kind(0, Kind::Crossing), Err(CalmingRefusal::Dragging));
        d.cancel_gesture();
    }

    #[test]
    fn a_freeway_a_street_with_no_roadway_and_a_timed_roadway_refuse() {
        let mut s = Street::sample(0, Side::Right);
        s.class = StreetClass::Motorway;
        let mut e = Editor::from_street(&s, &s, 0).with_frame(frame());
        assert_eq!(e.add_calming(Kind::Table, 10_000, 7_000), Err(CalmingRefusal::Freeway));
        assert_eq!(CalmingRefusal::Freeway.said().key, "calming-refused-freeway");

        // No roadway: only the zone's section decides.
        let walk = Street::imported(
            StreetClass::Local,
            Side::Right,
            &[crate::street::model::Piece {
                kind: crate::shared::catalogue::kind_index("sidewalk").unwrap(),
                width_mm: 6_000,
                direction: None,
                variants: Vec::new(),
            }],
        );
        let mut e = Editor::from_street(&walk, &walk, 0).with_frame(frame());
        assert_eq!(e.add_calming(Kind::Table, 10_000, 7_000), Err(CalmingRefusal::NoRoadway));

        // A zone the element covers with no roadway refuses.
        let mut e = editor();
        e.split_zone(Some(30_000)).unwrap();
        e.select_zone(1);
        let roadway: Vec<u32> = e.current().iter().filter(|g| crate::shared::catalogue::is_roadway(g.kind)).map(|g| g.uid).collect();
        for uid in roadway {
            e.remove(uid);
        }
        assert_eq!(e.add_calming(Kind::Table, 25_000, 7_000), Err(CalmingRefusal::NoRoadway), "it reaches into zone 2");
        e.add_calming(Kind::Table, 20_000, 7_000).unwrap();

        // A roadway that changes with the clock.
        let mut e = editor();
        let parking = e.current().iter().find(|g| g.kind == crate::shared::catalogue::kind_index("parking").unwrap()).unwrap().uid;
        assert!(e.add_variant(parking));
        assert_eq!(e.add_calming(Kind::Table, 10_000, 7_000), Err(CalmingRefusal::TimeWindows));
        assert_eq!(CalmingRefusal::TimeWindows.said().key, "calming-refused-time-windows");
        assert!(e.snapshot().calming.is_empty());
    }

    #[test]
    fn an_index_that_is_not_an_element_does_nothing() {
        let mut e = editor();
        assert_eq!(e.move_calming(2, 0), Ok(()));
        assert_eq!(e.set_calming_length(2, 7_000), Ok(()));
        assert_eq!(e.set_calming_kind(2, Kind::Crossing), Ok(()));
        assert_eq!(e.remove_calming(2), Ok(()));
        assert_eq!(e.view().revisions.len(), 0);
    }

    #[test]
    fn each_edit_is_one_revision_with_its_label() {
        let mut e = editor();
        e.add_calming(Kind::Table, 20_000, 7_000).unwrap();
        e.set_calming_kind(0, Kind::Crossing).unwrap();
        e.set_calming_length(0, 8_000).unwrap();
        e.move_calming(0, 25_000).unwrap();
        assert_eq!(keys(&e), ["rev-calming-add", "rev-calming-kind", "rev-calming-resize", "rev-calming-move"]);
        assert_eq!(e.snapshot().calming, vec![Calming { from_mm: 25_000, length_mm: 8_000, kind: Kind::Crossing }]);
        // Nothing changed, nothing recorded.
        e.set_calming_kind(0, Kind::Crossing).unwrap();
        e.set_calming_length(0, 8_040).unwrap();
        e.move_calming(0, 25_040).unwrap();
        assert_eq!(e.view().revisions.len(), 4);
        // A change that would overlap or run out is refused.
        e.add_calming(Kind::Table, 40_000, 7_000).unwrap();
        assert_eq!(e.set_calming_length(0, 15_100), Err(CalmingRefusal::Overlap));
        assert_eq!(e.set_calming_length(0, 5_900), Err(CalmingRefusal::TooShort { min_mm: MIN_TABLE_MM }));
        assert_eq!(e.move_calming(1, LEN - 6_000), Err(CalmingRefusal::Outside));
        e.remove_calming(0).unwrap();
        assert_eq!(e.view().revisions.last().unwrap().label.key, "rev-calming-remove");
        assert_eq!(e.snapshot().calming.len(), 1);
    }

    #[test]
    fn the_free_stretches_are_the_gaps_between_the_elements() {
        let mut e = editor();
        assert_eq!(e.calming_free_stretches(), vec![(0, LEN)]);
        e.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        e.add_calming(Kind::Table, 17_000, 6_000).unwrap();
        assert_eq!(e.calming_free_stretches(), vec![(0, 10_000), (23_000, LEN)]);
        let bare = Street::sample(0, Side::Right);
        assert!(Editor::from_street(&bare, &bare, 0).calming_free_stretches().is_empty(), "no anchor, no stretches");
    }

    #[test]
    fn undo_redo_and_start_over_restore_the_elements_and_the_selection() {
        let mut e = editor();
        e.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        e.add_calming(Kind::Crossing, 30_000, 6_000).unwrap();
        assert_eq!(e.calming_sel(), Some(1));
        e.select_calming(Some(0));
        assert_eq!(e.calming_sel(), Some(0));
        assert_eq!(e.view().revisions.len(), 2, "choosing is not history");
        e.move_calming(0, 12_000).unwrap();
        assert_eq!(e.calming_sel(), Some(0), "the choice follows the element");
        assert!(e.undo());
        assert_eq!(e.calming_sel(), Some(0));
        assert_eq!(e.snapshot().calming[0].from_mm, 10_000);
        assert!(e.redo());
        assert_eq!(e.calming_sel(), Some(0));
        assert_eq!(e.snapshot().calming[0].from_mm, 12_000);
        assert!(e.undo() && e.undo());
        assert_eq!(e.snapshot().calming.len(), 1, "the second add is undone");
        assert_eq!(e.calming_sel(), Some(0), "the element is still chosen");
        assert!(e.undo());
        assert_eq!(e.calming_sel(), None, "the chosen element is gone: nothing is chosen");
        assert!(e.redo() && e.redo() && e.redo());
        assert_eq!(e.snapshot().calming[0].from_mm, 12_000);
        assert!(e.reset());
        assert!(e.snapshot().calming.is_empty());
        assert_eq!(e.calming_sel(), None);
        assert!(e.undo());
        assert_eq!(e.snapshot().calming.len(), 2);
    }

    #[test]
    fn a_cancelled_move_restores_the_place() {
        let mut e = editor();
        e.add_calming(Kind::Table, 20_000, 7_000).unwrap();
        e.begin_gesture();
        e.move_calming(0, 22_000).unwrap();
        e.move_calming(0, 24_000).unwrap();
        assert_eq!((e.snapshot().calming[0].from_mm, e.calming_sel()), (24_000, Some(0)));
        e.cancel_gesture();
        assert_eq!((e.snapshot().calming[0].from_mm, e.calming_sel()), (20_000, Some(0)));
        e.begin_gesture();
        e.move_calming(0, 30_000).unwrap();
        assert!(e.end_gesture());
        assert_eq!(e.view().revisions.len(), 2, "one revision for the drag");
        assert_eq!(e.calming_sel(), Some(0));
        assert!(e.undo());
        assert_eq!((e.snapshot().calming[0].from_mm, e.calming_sel()), (20_000, Some(0)));
    }

    #[test]
    fn an_elsewhere_street_shows_but_does_not_rearrange() {
        let s = Street::sample(0, Side::Right);
        let mut drawn = editor();
        drawn.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        let mut other = Editor::from_street(&s, &drawn.snapshot(), 0).with_frame(Some(ZoneFrame { length_mm: LEN + 5_000, from_end: ZoneEnd::First }));
        let v = other.view();
        assert_eq!(v.calming.len(), 1);
        assert!(v.calming[0].elsewhere);
        assert_eq!(other.move_calming(0, 20_000), Err(CalmingRefusal::Elsewhere));
        assert_eq!(other.set_calming_kind(0, Kind::Crossing), Err(CalmingRefusal::Elsewhere));
        assert_eq!(other.snapshot().calming.len(), 1);
        assert!(!drawn.view().calming[0].elsewhere);
    }

    #[test]
    fn the_view_names_each_element_and_marks_the_chosen_one() {
        let mut e = editor();
        e.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        e.add_calming(Kind::Crossing, 30_000, 6_000).unwrap();
        e.select_calming(Some(0));
        let v = e.view().calming;
        assert_eq!((v[0].from_mm, v[0].length_mm, v[0].kind, v[0].selected), (10_000, 7_000, Kind::Table, true));
        assert_eq!((v[0].name.key, v[1].name.key, v[1].selected), ("calming-kind-table", "calming-kind-crossing", false));
        e.select_calming(None);
        assert!(e.view().calming.iter().all(|c| !c.selected));
        e.select_calming(Some(9));
        assert_eq!(e.calming_sel(), None);
    }

    #[test]
    fn a_reversed_street_keeps_the_same_physical_place() {
        let mut e = editor();
        e.add_calming(Kind::Table, 12_000, 7_000).unwrap();
        let s = e.snapshot();
        let r = s.reversed();
        assert_eq!(r.calming, s.calming, "no side to swap, measured from the same physical end");
        assert_eq!(r.from_end, ZoneEnd::Last);
        assert_eq!(r.reversed(), s);
    }
}
