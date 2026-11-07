//! Traffic calming: raised tables and raised crossings along a street, each an extent of the roadway lifted to a
//! plateau with a ramp at either end.
//!
//! They belong to the street, not to a section: a position is measured along the whole street from its anchored end
//! (`Street::from_end`), as a zone boundary and a curbside use are, and a table spans the whole width of the roadway,
//! so it has no side. A street with none is written, drawn and checked exactly as before.

use serde::{Deserialize, Serialize};

use super::stop_end::stop_gap_mm;
use super::{Arg, Check, Content, Editor, SNAP_MM, Said, ZoneEnd, ZoneFrame, snap, zone_label_mid};
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

/// One cell of the raised elements' strip: an element, or a free stretch between two.
#[derive(Clone, Debug, Serialize)]
pub struct CalmingCell {
    /// An index into the street's `calming` for an element, `None` for a free stretch.
    pub element: Option<usize>,
    pub from_mm: i32,
    pub to_mm: i32,
    pub kind: Option<Kind>,
    /// What the cell says: the kind's name, or "Free" for a stretch.
    pub label: Said,
    /// The element breaks no raised-element check (a free stretch always passes).
    pub ok: bool,
}

impl super::StripCell for CalmingCell {
    fn instance(&self) -> Option<usize> {
        self.element
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

/// The strip of the raised elements: the elements and the free stretches along the whole street, in order, and the
/// chosen cell. No cells without a length to lay them along.
#[derive(Clone, Debug, Default, Serialize)]
pub struct CalmingStripView {
    pub cells: Vec<CalmingCell>,
    pub selected: Option<usize>,
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

    /// Chooses the cell that holds `at_mm`, an element or a free stretch (a free stretch is where one is added).
    /// A setting, not history.
    pub fn select_calming_at(&mut self, at_mm: i32) {
        self.calming_choice.pick(Side::Left, at_mm);
    }

    /// The strip of the elements: none unless the street has a length to lay them along.
    pub(super) fn calming_strip_view(&self) -> CalmingStripView {
        let content = &self.states[self.cursor].content;
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(0);
        if length_mm <= 0 {
            return CalmingStripView::default();
        }
        let failing: Vec<usize> = self.calming_failures().iter().map(|f| f.element).collect();
        let gap = |from_mm: i32, to_mm: i32| CalmingCell { element: None, from_mm, to_mm, kind: None, label: Said::new("calming-gap"), ok: true };
        let items = content.calming.iter().enumerate().map(|(k, c)| (k, c.from_mm, c.to_mm()));
        let cells = super::curb::strip_cells(length_mm, items, gap, |k| {
            let c = &content.calming[k];
            CalmingCell {
                element: Some(k),
                from_mm: c.from_mm,
                to_mm: c.to_mm(),
                kind: Some(c.kind),
                label: Said::new(c.kind.name_key()),
                ok: !failing.contains(&k),
            }
        });
        let selected = super::curb::selected_cell(&cells, self.calming_choice.at, Side::Left);
        CalmingStripView { cells, selected }
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

impl Calming {
    /// How the checks name it: "raised table at 20 m".
    fn named(&self) -> Said {
        Said::new("calming-instance").with("kind", Arg::MsgLower(self.kind.name_key())).with("at", Arg::Length(self.from_mm))
    }

    /// The same, in the zone where it fails.
    fn named_in(&self, zone: Said) -> Said {
        Said::new("calming-instance-zone")
            .with("kind", Arg::MsgLower(self.kind.name_key()))
            .with("at", Arg::Length(self.from_mm))
            .with("zone", Arg::Said(Box::new(zone)))
    }
}

/// The length at least which an element's ramps leave a plateau: the shorter of the two requirements is no help.
const fn needed_mm() -> i32 {
    let by_plateau = MIN_PLATEAU_MM + 2 * RAMP_MM;
    if by_plateau > MIN_TABLE_MM { by_plateau } else { MIN_TABLE_MM }
}

/// One rule one element breaks, with the way the check names the element when that is not just the element.
struct Fail {
    element: usize,
    rule: usize,
    detail: Said,
    named: Option<Said>,
}

impl Editor {
    /// The rows of the raised elements, after the stops' (only when the street has elements): length and roadway,
    /// speed, clearance of the transit stops and, where the city has told an end how far its junction reaches,
    /// clearance of the junction. A row fails with the first element that breaks it, which it names.
    ///
    /// A street with no speed set is a note (the row passes and says so), never a failure. Every figure is a
    /// placeholder, UNVERIFIED. Positions are measured from the anchored end, as the stops' are, and `ends` (node
    /// `a`, node `b`) is read from that end, so the same street gives the same answer whichever end it is anchored at.
    pub(super) fn calming_checks(&self) -> Vec<Check> {
        let content = &self.states[self.cursor].content;
        let fails = self.calming_failures();
        let mut rows = vec![
            ("calming-length", "check-label-calming-length", "check-detail-calming-length-ok"),
            ("calming-speed", "check-label-calming-speed", "check-detail-calming-speed-unknown"),
            ("calming-stop", "check-label-calming-stop", "check-detail-calming-stop-ok"),
        ];
        if self.stops_see_a_junction() {
            rows.push(("calming-junction", "check-label-calming-junction", "check-detail-calming-junction-ok"));
        }
        rows.iter()
            .enumerate()
            .map(|(rule, &(id, label, ok_detail))| match fails.iter().find(|f| f.rule == rule) {
                Some(f) => {
                    let element = &content.calming[f.element];
                    Check { id, ok: false, label: Said::new(label), detail: f.detail.clone(), zone: Some(f.named.clone().unwrap_or_else(|| element.named())) }
                }
                None => {
                    let detail = match (rule, content.speed.kmh()) {
                        (1, Some(kmh)) => Said::new("check-detail-calming-speed-ok").with("kmh", Arg::Num(i64::from(kmh))),
                        _ => Said::new(ok_detail).with("max", Arg::Num(i64::from(MAX_TABLE_SPEED_KMH))),
                    };
                    Check { id, ok: true, label: Said::new(label), detail, zone: None }
                }
            })
            .collect()
    }

    /// Every rule each element breaks, element by element in order: `rule` indexes the rows of `calming_checks`.
    fn calming_failures(&self) -> Vec<Fail> {
        let content = &self.states[self.cursor].content;
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(i32::MAX);
        // The far end is measured from the frame's length, as the junction measures it.
        let end_length_mm = self.frame.map_or(length_mm, |f| f.length_mm);
        let (near, far) = match content.from_end {
            ZoneEnd::First => (self.ends[0], self.ends[1]),
            ZoneEnd::Last => (self.ends[1], self.ends[0]),
        };
        let mut out: Vec<Fail> = Vec::new();
        for (k, c) in content.calming.iter().enumerate() {
            let mut fail = |rule: usize, detail: Said, named: Option<Said>| {
                if !out.iter().any(|f| f.element == k && f.rule == rule) {
                    out.push(Fail { element: k, rule, detail, named });
                }
            };
            if c.length_mm < MIN_TABLE_MM || c.length_mm - 2 * RAMP_MM < MIN_PLATEAU_MM {
                fail(
                    0,
                    Said::new("check-detail-calming-length")
                        .with("length", Arg::Length(c.length_mm))
                        .with("ramp", Arg::Length(RAMP_MM))
                        .with("min", Arg::Length(needed_mm())),
                    None,
                );
            }
            // The roadway to raise, in every zone the element overlaps: the zone that has none is named.
            if content.zone_count() < 2 {
                if !content.segments.iter().any(|g| is_roadway(g.kind)) {
                    fail(0, Said::new("check-detail-calming-roadway"), None);
                }
            } else {
                for z in 0..content.zone_count() {
                    let (from, to) = content.span(z, length_mm);
                    if from < c.to_mm() && c.from_mm < to && !content.section(z).iter().any(|g| is_roadway(g.kind)) {
                        fail(0, Said::new("check-detail-calming-roadway"), Some(c.named_in(zone_label_mid(&content.zones[z], z + 1))));
                    }
                }
            }
            if let Some(kmh) = content.speed.kmh().filter(|&n| n > u16::from(MAX_TABLE_SPEED_KMH)) {
                fail(
                    1,
                    Said::new("check-detail-calming-speed").with("kmh", Arg::Num(i64::from(kmh))).with("max", Arg::Num(i64::from(MAX_TABLE_SPEED_KMH))),
                    None,
                );
            }
            // A stop's boarding area is along the stop, on either side: the element spans the whole roadway. The stop
            // the detail names is the one with the smallest gap (an overlap is a negative one), the first of equals.
            let nearest = content
                .stops
                .iter()
                .map(|s| ((c.from_mm - s.to_mm()).max(s.from_mm - c.to_mm()), s))
                .filter(|(gap, _)| *gap < CALMING_STOP_CLEAR_MM)
                .min_by_key(|(gap, _)| *gap);
            if let Some((gap, s)) = nearest {
                if gap < 0 {
                    fail(2, Said::new("check-detail-calming-stop-overlap").with("stop", Arg::Said(Box::new(s.named()))), None);
                } else {
                    fail(
                        2,
                        Said::new("check-detail-calming-stop-near")
                            .with("stop", Arg::Said(Box::new(s.named())))
                            .with("gap", Arg::Length(gap))
                            .with("need", Arg::Length(CALMING_STOP_CLEAR_MM)),
                        None,
                    );
                }
            }
            for (end, from_end_mm) in [(near, c.from_mm), (far, end_length_mm.saturating_sub(c.to_mm()))] {
                if end.reach_mm <= 0 {
                    continue;
                }
                let gap = stop_gap_mm(from_end_mm, end.reach_mm);
                if gap < 0 {
                    fail(3, Said::new("check-detail-calming-junction-inside"), None);
                } else if gap < CALMING_JUNCTION_CLEAR_MM {
                    let key = if end.crossing { "check-detail-calming-junction-crossing" } else { "check-detail-calming-junction-mouth" };
                    fail(3, Said::new(key).with("gap", Arg::Length(gap)).with("need", Arg::Length(CALMING_JUNCTION_CLEAR_MM)), None);
                }
            }
        }
        out
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

    #[test]
    fn remove_works_on_a_freeway_street_where_nothing_can_be_added() {
        let mut s = Street::sample(0, Side::Right);
        s.class = StreetClass::Motorway;
        s.length_mm = Some(LEN);
        s.calming = vec![table(10_000)];
        let mut e = Editor::from_street(&s, &s, 0).with_frame(frame());
        assert_eq!(e.add_calming(Kind::Table, 30_000, 7_000), Err(CalmingRefusal::Freeway));
        assert_eq!(e.remove_calming(0), Ok(()));
        assert!(e.snapshot().calming.is_empty());
    }

    #[test]
    fn a_roadway_that_changes_with_the_clock_refuses_a_move_and_a_resize() {
        let mut e = editor();
        e.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        let parking = e.current().iter().find(|g| g.kind == crate::shared::catalogue::kind_index("parking").unwrap()).unwrap().uid;
        assert!(e.add_variant(parking));
        assert_eq!(e.move_calming(0, 20_000), Err(CalmingRefusal::TimeWindows));
        assert_eq!(e.set_calming_length(0, 8_000), Err(CalmingRefusal::TimeWindows));
        assert_eq!(e.set_calming_kind(0, Kind::Crossing), Err(CalmingRefusal::TimeWindows));
        assert_eq!(e.snapshot().calming, vec![table(10_000)]);
    }

    #[test]
    fn the_choice_after_start_over_and_then_undo_is_the_element_at_its_place() {
        let mut e = editor();
        e.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        e.add_calming(Kind::Crossing, 30_000, 6_000).unwrap();
        e.select_calming(Some(0));
        assert!(e.reset());
        assert_eq!(e.calming_sel(), None);
        assert!(e.undo());
        assert_eq!(e.snapshot().calming.len(), 2);
        assert_eq!(e.calming_sel(), Some(0), "the choice is a place, so the element standing there is chosen again");
    }

    #[test]
    fn the_strip_has_a_cell_per_element_and_per_free_stretch_and_a_chosen_one() {
        let mut e = editor();
        assert!(e.view().calming_strip.cells.iter().all(|c| c.element.is_none()), "one free stretch");
        assert_eq!(e.view().calming_strip.cells.len(), 1);
        e.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        let v = e.view().calming_strip;
        assert_eq!(
            v.cells.iter().map(|c| (c.element, c.from_mm, c.to_mm)).collect::<Vec<_>>(),
            [(None, 0, 10_000), (Some(0), 10_000, 17_000), (None, 17_000, LEN)]
        );
        assert_eq!(v.selected, Some(1), "the new element is the chosen cell");
        e.select_calming_at(3_000);
        assert_eq!(e.view().calming_strip.selected, Some(0), "a free stretch can be chosen");
        assert_eq!(e.calming_sel(), None);
        let bare = Street::sample(0, Side::Right);
        assert!(Editor::from_street(&bare, &bare, 0).view().calming_strip.cells.is_empty(), "no anchor, no strip");
    }

    #[test]
    fn a_failing_element_marks_its_cell() {
        let mut e = editor();
        e.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        e.add_calming(Kind::Table, 30_000, 6_000).unwrap();
        e.set_calming_length(1, 6_000).unwrap();
        assert!(e.view().calming_strip.cells.iter().filter(|c| c.element.is_some()).all(|c| c.ok));
        let mut f = editor();
        f.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        f.add_stop(Side::Right, 20_000, 9_000, StopKind::Curb).unwrap();
        let cells = f.view().calming_strip.cells;
        assert!(!cells.iter().find(|c| c.element == Some(0)).unwrap().ok, "the stop 3 m away fails its check");
        assert!(cells.iter().filter(|c| c.element.is_none()).all(|c| c.ok));
    }

    #[test]
    fn two_failing_stops_name_the_nearer_one() {
        // The table runs 20 000 to 27 000. A stop from 31 000 (gap 4 000) and one from 40 000 would be clear, so use
        // one before it at 6 000 to 16 000 (gap 4 000) and a nearer one after at 29 000 (gap 2 000).
        let mut e = editor();
        // Stops are listed left side first, so the farther one comes first.
        e.add_stop(Side::Left, 6_000, 10_000, StopKind::Curb).unwrap();
        e.add_stop(Side::Right, 29_000, 9_000, StopKind::Curb).unwrap();
        e.add_calming(Kind::Table, 20_000, 7_000).unwrap();
        let c = check(&e, "calming-stop");
        assert!(!c.ok);
        assert_eq!(arg_mm(&c.detail, "gap"), Some(2_000), "the stop with the smallest gap, not the first by position");
        let Some((_, Arg::Said(stop))) = c.detail.args.iter().find(|(n, _)| *n == "stop") else { panic!("the stop is named") };
        assert_eq!(arg_mm(stop, "at"), Some(29_000), "{stop:?}");
    }

    // ---- the design checks ----

    use super::super::stop::StopKind;
    use super::super::{Check, EndInfo};
    use crate::street::model::Speed;

    fn check(e: &Editor, id: &str) -> Check {
        e.view().checks.into_iter().find(|c| c.id == id).unwrap_or_else(|| panic!("no check {id}"))
    }

    fn arg_mm(said: &Said, name: &str) -> Option<i32> {
        said.args.iter().find_map(|(n, a)| match a {
            Arg::Length(v) if *n == name => Some(*v),
            _ => None,
        })
    }

    fn with_table(from_mm: i32, length_mm: i32) -> Editor {
        let mut e = editor();
        e.add_calming(Kind::Table, from_mm, length_mm).unwrap();
        e
    }

    fn reach(crossing: bool, reach_mm: i32) -> EndInfo {
        EndInfo { crossing, reach_mm }
    }

    fn at_speed(speed: Speed, from_end: ZoneEnd) -> Editor {
        let mut s = Street::sample(0, Side::Right);
        s.speed = speed;
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end }));
        e.add_calming(Kind::Table, 20_000, 7_000).unwrap();
        e
    }

    #[test]
    fn no_calming_adds_no_checks() {
        let e = editor().with_ends([reach(true, 9_000); 2]);
        assert!(e.view().checks.iter().all(|c| !c.id.starts_with("calming-")));
    }

    #[test]
    fn the_rows_follow_the_stop_rows_and_the_junction_row_needs_a_reach() {
        let e = with_table(20_000, 7_000);
        let ids: Vec<&str> = e.view().checks.iter().map(|c| c.id).filter(|i| i.starts_with("calming-")).collect();
        assert_eq!(ids, ["calming-length", "calming-speed", "calming-stop"]);
        let e = with_table(20_000, 7_000).with_ends([reach(false, 9_000), EndInfo::default()]);
        let ids: Vec<&str> = e.view().checks.iter().map(|c| c.id).filter(|i| i.starts_with("calming-")).collect();
        assert_eq!(ids, ["calming-length", "calming-speed", "calming-stop", "calming-junction"]);
    }

    #[test]
    fn an_element_that_is_long_enough_passes_length() {
        let c = check(&with_table(20_000, MIN_TABLE_MM), "calming-length");
        assert!(c.ok);
        assert_eq!(c.detail.key, "check-detail-calming-length-ok");
        assert!(c.zone.is_none());
    }

    #[test]
    fn a_short_element_fails_length_naming_itself_and_the_minimum() {
        // `add_calming` refuses a short one, so a short one is a saved one: shorten the minimum by editing the state.
        let mut e = with_table(20_000, 7_000);
        e.edit_content(Said::new("test"), |c| {
            c.calming[0].length_mm = MIN_TABLE_MM - SNAP_MM;
            true
        });
        let c = check(&e, "calming-length");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-calming-length");
        assert_eq!(arg_mm(&c.detail, "length"), Some(MIN_TABLE_MM - SNAP_MM));
        assert_eq!(arg_mm(&c.detail, "min"), Some(MIN_TABLE_MM));
        assert_eq!(arg_mm(&c.detail, "ramp"), Some(RAMP_MM));
        let zone = c.zone.unwrap();
        assert_eq!(zone.key, "calming-instance");
        assert_eq!(zone.args.len(), 2);
    }

    #[test]
    fn speed_fails_above_the_limit_passes_at_it_and_is_a_note_when_unknown() {
        let fail = check(&at_speed(Speed::Kmh(60), ZoneEnd::First), "calming-speed");
        assert!(!fail.ok);
        assert_eq!(fail.detail.key, "check-detail-calming-speed");
        assert!(fail.detail.args.contains(&("kmh", Arg::Num(60))));
        assert!(fail.detail.args.contains(&("max", Arg::Num(i64::from(MAX_TABLE_SPEED_KMH)))));
        assert_eq!(fail.zone.unwrap().key, "calming-instance");
        let pass = check(&at_speed(Speed::Kmh(50), ZoneEnd::First), "calming-speed");
        assert!(pass.ok);
        assert_eq!(pass.detail.key, "check-detail-calming-speed-ok");
        for unknown in [Speed::NotSet, Speed::Undecided] {
            let note = check(&at_speed(unknown, ZoneEnd::First), "calming-speed");
            assert!(note.ok, "a street with no speed is never failed");
            assert_eq!(note.detail.key, "check-detail-calming-speed-unknown");
            assert!(note.zone.is_none());
        }
    }

    fn with_stop(table_from: i32, stop_from: i32, stop_len: i32) -> Editor {
        let mut e = editor();
        e.add_stop(Side::Right, stop_from, stop_len, StopKind::Curb).unwrap();
        e.add_calming(Kind::Table, table_from, 7_000).unwrap();
        e
    }

    #[test]
    fn a_table_over_a_boarding_area_fails_naming_the_stop() {
        // The stop runs 22 000 to 34 000; the table 20 000 to 27 000.
        let c = check(&with_stop(20_000, 22_000, 12_000), "calming-stop");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-calming-stop-overlap");
        let Some((_, Arg::Said(stop))) = c.detail.args.iter().find(|(n, _)| *n == "stop") else { panic!("the stop is named") };
        assert_eq!(stop.key, "stop-instance");
    }

    #[test]
    fn five_metres_clear_of_a_stop_passes_and_4_9_fails() {
        // The table ends at 27 000; a stop from 32 000 is 5 000 clear, from 31 900 it is 4 900.
        let c = check(&with_stop(20_000, 32_000, 9_000), "calming-stop");
        assert!(c.ok, "{:?}", c.detail);
        assert_eq!(c.detail.key, "check-detail-calming-stop-ok");
        let c = check(&with_stop(20_000, 31_900, 9_000), "calming-stop");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-calming-stop-near");
        assert_eq!(arg_mm(&c.detail, "gap"), Some(4_900));
        assert_eq!(arg_mm(&c.detail, "need"), Some(CALMING_STOP_CLEAR_MM));
        // The same on the other side of the table: a stop ending 5 000 before it.
        assert!(check(&with_stop(20_000, 6_000, 9_000), "calming-stop").ok);
        let c = check(&with_stop(20_000, 6_100, 9_000), "calming-stop");
        assert!(!c.ok);
        assert_eq!(arg_mm(&c.detail, "gap"), Some(4_900));
    }

    #[test]
    fn a_street_with_no_stops_passes_the_stop_check() {
        assert!(check(&with_table(20_000, 7_000), "calming-stop").ok);
    }

    #[test]
    fn junction_clearance_is_measured_past_the_reach_at_each_end() {
        let near = |from_mm: i32, crossing: bool| with_table(from_mm, 7_000).with_ends([reach(crossing, 9_000), EndInfo::default()]);
        let ok = check(&near(14_000, true), "calming-junction");
        assert!(ok.ok);
        assert_eq!(ok.detail.key, "check-detail-calming-junction-ok");
        let c = check(&near(13_900, true), "calming-junction");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-calming-junction-crossing");
        assert_eq!(arg_mm(&c.detail, "gap"), Some(4_900));
        assert_eq!(arg_mm(&c.detail, "need"), Some(CALMING_JUNCTION_CLEAR_MM));
        assert_eq!(check(&near(13_900, false), "calming-junction").detail.key, "check-detail-calming-junction-mouth");
        assert_eq!(check(&near(5_000, false), "calming-junction").detail.key, "check-detail-calming-junction-inside");
        // The far end: the table ends at 60 000 - x; its far side is x from node b.
        let far = |from_mm: i32| with_table(from_mm, 7_000).with_ends([EndInfo::default(), reach(true, 9_000)]);
        assert!(check(&far(60_000 - 14_000 - 7_000), "calming-junction").ok);
        assert!(!check(&far(60_000 - 13_900 - 7_000), "calming-junction").ok);
    }

    #[test]
    fn a_zone_boundary_names_the_failing_zone() {
        let mut e = with_table(25_000, 10_000);
        e.split_zone(Some(30_000)).unwrap();
        e.select_zone(1);
        assert!(check(&e, "calming-length").ok);
        // The second zone loses its roadway; the table reaches into it.
        let roadway: Vec<u32> = e.view().segments.iter().filter(|s| crate::shared::catalogue::is_roadway(s.kind)).map(|s| s.uid).collect();
        for uid in roadway {
            assert!(e.remove(uid));
        }
        let c = check(&e, "calming-length");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-calming-roadway");
        let named = c.zone.unwrap();
        assert_eq!(named.key, "calming-instance-zone");
        let Some((_, Arg::Said(zone))) = named.args.iter().find(|(n, _)| *n == "zone") else { panic!("the zone is named") };
        assert_eq!((zone.key, zone.args.clone()), ("zone-unnamed-mid", vec![("n", Arg::Num(2))]));
        // A table wholly in the first zone is not affected.
        let mut e = with_table(5_000, 10_000);
        e.split_zone(Some(30_000)).unwrap();
        e.select_zone(1);
        for uid in e.view().segments.iter().filter(|s| crate::shared::catalogue::is_roadway(s.kind)).map(|s| s.uid).collect::<Vec<_>>() {
            e.remove(uid);
        }
        assert!(check(&e, "calming-length").ok);
    }

    #[test]
    fn the_checks_are_the_same_whichever_end_the_street_is_anchored_at() {
        let make = |from_end: ZoneEnd, table: i32, stop: i32| {
            let s = Street::sample(0, Side::Right);
            let mut e =
                Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end })).with_ends([reach(true, 9_000), reach(false, 7_000)]);
            e.add_stop(Side::Right, stop, 9_000, StopKind::Curb).unwrap();
            e.add_calming(Kind::Table, table, 7_000).unwrap();
            e
        };
        // Physically: the table 12 000 to 19 000 from node a, a stop 22 000 to 31 000 (3 000 clear of the table).
        let first = make(ZoneEnd::First, 12_000, 22_000);
        let last = make(ZoneEnd::Last, LEN - 19_000, LEN - 31_000);
        let said = |e: &Editor| {
            // The stop is named by its own place, measured from the anchored end, so only its gap is compared.
            e.view()
                .checks
                .into_iter()
                .filter(|c| c.id.starts_with("calming-"))
                .map(|c| (c.id, c.ok, c.detail.key, arg_mm(&c.detail, "gap")))
                .collect::<Vec<_>>()
        };
        assert_eq!(said(&first), said(&last));
        assert!(!check(&first, "calming-stop").ok);
        assert!(!check(&first, "calming-junction").ok);
    }

    #[test]
    fn the_wording_is_in_both_languages() {
        use crate::shared::i18n::Locale;
        use crate::shared::units::Units;
        let mut e = with_table(20_000, 7_000).with_ends([reach(true, 17_000), EndInfo::default()]);
        e.add_stop(Side::Right, 27_000, 9_000, StopKind::Curb).unwrap();
        let checks = e.view().checks;
        for locale in [Locale::En, Locale::FrCa] {
            let i18n = crate::i18n_for(locale);
            for c in checks.iter().filter(|c| c.id.starts_with("calming-")) {
                let units = Units::default();
                for said in [&c.label, &c.detail] {
                    let text = crate::shared::said::say_now(&i18n, units, said);
                    assert!(!text.is_empty() && !text.contains("calming"), "{}: {text}", c.id);
                }
                let label = crate::street::text::check_label(c);
                let text = crate::shared::said::say_now(&i18n, units, &label);
                assert!(!text.contains("calming-"), "{text}");
            }
        }
        let en = crate::i18n_for(Locale::En);
        let fr = crate::i18n_for(Locale::FrCa);
        let stop = check(&e, "calming-stop");
        let junction = check(&e, "calming-junction");
        assert!(!stop.ok && !junction.ok);
        let say = |i: &std::rc::Rc<crate::shared::i18n::I18n>, s: &Said| crate::shared::said::say_now(i, Units::default(), s);
        assert_eq!(say(&en, &crate::street::text::check_label(&stop)), "Raised elements clear of transit stops: raised table at 20.0 m");
        assert!(say(&fr, &crate::street::text::check_label(&stop)).starts_with("Éléments surélevés dégagés des arrêts"));
        assert!(say(&en, &junction.detail).contains("pedestrian crossing"));
        assert!(say(&fr, &junction.detail).contains("passage pour piétons"));
    }

    #[test]
    fn an_unnamed_stop_of_every_kind_reads_right_in_both_languages() {
        use crate::shared::i18n::Locale;
        use crate::shared::units::Units;
        let say = |locale, s: &Said| crate::shared::said::say_now(&crate::i18n_for(locale), Units::default(), s);
        let mut got = Vec::new();
        for kind in StopKind::ALL {
            // Overlapping: the table 20 000 to 27 000, the stop from 22 000. Near: the stop from 31 900.
            for (case, at) in [("overlap", 22_000), ("near", 31_900)] {
                let c = check(&with_stop_kind(kind, at), "calming-stop");
                assert!(!c.ok);
                got.push((kind, case, say(Locale::En, &c.detail), say(Locale::FrCa, &c.detail)));
            }
        }
        let want: Vec<(StopKind, &str, &str, &str)> = EXPECTED.to_vec();
        let got: Vec<(StopKind, &str, &str, &str)> = got.iter().map(|(k, c, e, f)| (*k, *c, e.as_str(), f.as_str())).collect();
        assert_eq!(got, want);
    }

    fn with_stop_kind(kind: StopKind, at: i32) -> Editor {
        let mut e = editor();
        e.add_stop(Side::Right, at, 9_000, kind).unwrap();
        e.add_calming(Kind::Table, 20_000, 7_000).unwrap();
        e
    }

    const EXPECTED: &[(StopKind, &str, &str, &str)] = &[
        (
            StopKind::Curb,
            "overlap",
            "The raised element overlaps the boarding area of a transit stop. Stop concerned: curb stop at 22.0 m, right.",
            "L’élément surélevé chevauche l’aire d’embarquement d’un arrêt de transport en commun. Arrêt en cause\u{a0}: arrêt au trottoir à 22,0\u{a0}m, à droite.",
        ),
        (
            StopKind::Curb,
            "near",
            "The raised element is 4.9 m from the nearest transit stop; 5.0 m or more is needed. Stop concerned: curb stop at 31.9 m, right.",
            "L’élément surélevé est à 4,9\u{a0}m de l’arrêt de transport en commun le plus proche; il en faut 5,0\u{a0}m ou plus. Arrêt en cause\u{a0}: arrêt au trottoir à 31,9\u{a0}m, à droite.",
        ),
        (
            StopKind::Bulb,
            "overlap",
            "The raised element overlaps the boarding area of a transit stop. Stop concerned: curb extension at 22.0 m, right.",
            "L’élément surélevé chevauche l’aire d’embarquement d’un arrêt de transport en commun. Arrêt en cause\u{a0}: avancée de trottoir à 22,0\u{a0}m, à droite.",
        ),
        (
            StopKind::Bulb,
            "near",
            "The raised element is 4.9 m from the nearest transit stop; 5.0 m or more is needed. Stop concerned: curb extension at 31.9 m, right.",
            "L’élément surélevé est à 4,9\u{a0}m de l’arrêt de transport en commun le plus proche; il en faut 5,0\u{a0}m ou plus. Arrêt en cause\u{a0}: avancée de trottoir à 31,9\u{a0}m, à droite.",
        ),
        (
            StopKind::Island,
            "overlap",
            "The raised element overlaps the boarding area of a transit stop. Stop concerned: boarding island at 22.0 m, right.",
            "L’élément surélevé chevauche l’aire d’embarquement d’un arrêt de transport en commun. Arrêt en cause\u{a0}: îlot d’embarquement à 22,0\u{a0}m, à droite.",
        ),
        (
            StopKind::Island,
            "near",
            "The raised element is 4.9 m from the nearest transit stop; 5.0 m or more is needed. Stop concerned: boarding island at 31.9 m, right.",
            "L’élément surélevé est à 4,9\u{a0}m de l’arrêt de transport en commun le plus proche; il en faut 5,0\u{a0}m ou plus. Arrêt en cause\u{a0}: îlot d’embarquement à 31,9\u{a0}m, à droite.",
        ),
    ];
}
