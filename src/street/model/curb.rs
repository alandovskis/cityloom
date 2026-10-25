//! Curbside programmes: the parking or loading lane at a street's edge, divided along the street into an ordered
//! list of uses (a loading bay, a tree, a few parking spaces, a patio), each with an extent.
//!
//! They belong to the street, not to a section: a position is measured along the whole street from its anchored end
//! (`Street::from_end`), as a zone boundary is, so splitting, merging or moving zones leaves them where they are.
//! A street with none is written, drawn and checked exactly as before.

use serde::{Deserialize, Serialize};

use super::{ACCESS_LANE_MM, Arg, Check, Content, Editor, SNAP_MM, Said, Segment, Street, ZoneEnd, ZoneFrame, snap, widest_walkway_mm};
use crate::shared::catalogue::{KINDS, Side};

/// How near a pedestrian crossing parking or loading may stand, in millimetres. The Code de la sécurité routière of
/// Québec forbids stopping within 5 m of a crosswalk; the article is UNVERIFIED.
pub const CROSSING_CLEARANCE_MM: i32 = 5_000;

/// The buffer a bike lane needs between it and the parking beside it, so that opening a door does not open it into
/// the lane, in millimetres. UNVERIFIED.
pub const DOOR_BUFFER_MM: i32 = 600;

/// What the city says of one end of a street.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EndInfo {
    /// The end is a junction with a pedestrian crossing on this street's arm.
    pub crossing: bool,
}

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
    /// Every use, in the order a choice lists them.
    pub const ALL: [Use; 8] = [Use::Parking, Use::Loading, Use::Tree, Use::Planting, Use::RainGarden, Use::Patio, Use::BikeParking, Use::Platform];

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

/// What an edit works on, which words its refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subject {
    /// A use of the curbside lane.
    Use,
    /// A transit stop.
    Stop,
}

impl CurbRefusal {
    /// The refusal of a curbside edit, in the curbside's words.
    pub fn said(self) -> Said {
        self.said_for(Subject::Use)
    }

    /// The refusal of an edit of `subject`, in the words that fit it.
    pub fn said_for(self, subject: Subject) -> Said {
        let (curb, stop) = match self {
            CurbRefusal::NoAnchor => ("zone-refused-no-anchor", "stop-refused-no-anchor"),
            CurbRefusal::Elsewhere => ("curb-refused-elsewhere", "stop-refused-elsewhere"),
            CurbRefusal::Overlap => ("curb-refused-overlap", "stop-refused-overlap"),
            CurbRefusal::TooLong => ("curb-refused-too-long", "stop-refused-too-long"),
            // A stop does not depend on the time of day, so stop edits never raise this; the arm keeps `said_for` total.
            CurbRefusal::TimeWindows => ("curb-refused-time-windows", "stop-refused-time-windows"),
            CurbRefusal::Dragging => ("curb-refused-dragging", "stop-refused-dragging"),
        };
        Said::new(if subject == Subject::Stop { stop } else { curb })
    }
}

/// One cell of a strip: a use or a stop, or the gap between two.
pub trait StripCell {
    /// An index into the street's list (`curb`, `stops`) for a cell that is one, `None` for a gap.
    fn instance(&self) -> Option<usize>;
    fn from_mm(&self) -> i32;
    fn to_mm(&self) -> i32;
    /// What the cell says: its name, or "Free" for a gap.
    fn label(&self) -> &Said;
    /// It breaks no check (a gap always passes).
    fn ok(&self) -> bool;
}

/// The strips of a street as the view carries them: one per side that has something to put along it.
#[derive(Serialize)]
pub struct StripView<C> {
    pub sides: Vec<SideView<C>>,
}

#[derive(Serialize)]
pub struct SideView<C> {
    pub side: Side,
    /// The things and the gaps between them along the whole street, in order.
    pub cells: Vec<C>,
    /// The chosen cell, on the side the person chose it.
    pub selected: Option<usize>,
}

/// The curbside strips: one per side that has a curbside lane somewhere along the street, or has uses.
pub type CurbView = StripView<CurbCell>;
pub type CurbSideView = SideView<CurbCell>;

#[derive(Serialize)]
pub struct CurbCell {
    /// An index into the street's `curb` for a use, `None` for a gap.
    pub instance: Option<usize>,
    pub from_mm: i32,
    pub to_mm: i32,
    /// The use, for a use; `None` for a gap.
    pub use_: Option<Use>,
    /// What the cell says: the use's name, or "Free" for a gap.
    pub label: Said,
    /// The use breaks no curbside check (a gap always passes).
    pub ok: bool,
}

impl StripCell for CurbCell {
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

/// The cells of one side along `length_mm`: `items` (an index into the list and where it starts and ends, in
/// order, not overlapping) with a gap before each, between them and after the last.
pub(super) fn strip_cells<C>(length_mm: i32, items: impl Iterator<Item = (usize, i32, i32)>, gap: impl Fn(i32, i32) -> C, item: impl Fn(usize) -> C) -> Vec<C> {
    let mut cells = Vec::new();
    let mut at = 0;
    for (k, from, to) in items {
        if from > at {
            cells.push(gap(at, from));
        }
        cells.push(item(k));
        at = at.max(to);
    }
    if at < length_mm {
        cells.push(gap(at, length_mm));
    }
    cells
}

/// The cell of `side` that holds the place the person chose, if they chose on this side.
pub(super) fn selected_cell<C: StripCell>(cells: &[C], chosen: Option<(Side, i32)>, side: Side) -> Option<usize> {
    let (_, mm) = chosen.filter(|(s, _)| *s == side)?;
    cells.iter().position(|c| c.from_mm() <= mm && mm < c.to_mm())
}

/// What the person chose on a strip, and the move that carries the choice through the history. The choice is a side
/// and a place along the street, found again in each view, so it stays on the thing as the cells around it change.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Choice {
    pub(super) at: Option<(Side, i32)>,
    /// The move under way in a gesture, or just made, that the next revision records: side, start and end.
    moved: Option<(Side, i32, i32)>,
}

impl Choice {
    pub(super) fn pick(&mut self, side: Side, at_mm: i32) {
        self.at = Some((side, at_mm));
    }

    /// The thing at (`side`, `from`) moves to `to`: the choice follows it, and the move is kept for the history. A
    /// drag records one move, from where it began to where it is.
    pub(super) fn follow(&mut self, side: Side, from: i32, to: i32) {
        if self.at == Some((side, from)) {
            self.at = Some((side, to));
        }
        let began = self.moved.filter(|&(s, _, end)| s == side && end == from).map_or(from, |(_, began, _)| began);
        self.moved = Some((side, began, to));
    }

    /// The move the revision being made records.
    pub(super) fn take_moved(&mut self) -> Option<(Side, i32, i32)> {
        self.moved.take()
    }

    pub(super) fn forget_moved(&mut self) {
        self.moved = None;
    }

    /// A drag was cancelled: the thing is back where it started, so is the choice.
    pub(super) fn cancel(&mut self) {
        if let Some((side, from, to)) = self.moved.take() {
            self.undo(Some((side, from, to)));
        }
    }

    /// The revision that made `moved` is undone.
    pub(super) fn undo(&mut self, moved: Option<(Side, i32, i32)>) {
        if let Some((side, from, to)) = moved
            && self.at == Some((side, to))
        {
            self.at = Some((side, from));
        }
    }

    /// The revision that made `moved` is redone.
    pub(super) fn redo(&mut self, moved: Option<(Side, i32, i32)>) {
        if let Some((side, from, to)) = moved
            && self.at == Some((side, from))
        {
            self.at = Some((side, to));
        }
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
    /// The sections of the zones whose span overlaps `from_mm..to_mm`, or the base section when there are no zones.
    pub(super) fn sections_over(&self, from_mm: i32, to_mm: i32, length_mm: i32) -> Vec<&[Segment]> {
        if self.zone_count() < 2 {
            return vec![&self.segments];
        }
        (0..self.zone_count())
            .filter(|&z| {
                let (from, to) = self.span(z, length_mm);
                from < to_mm && from_mm < to
            })
            .map(|z| self.section(z))
            .collect()
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
    /// The curbside strips: a side has one when some zone's section has a curbside lane there, or it has uses
    /// (which can then still be taken away).
    pub(super) fn curb_view(&self) -> CurbView {
        let content = &self.states[self.cursor].content;
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(0);
        if length_mm <= 0 {
            // Without a length there is nothing to lay the uses along.
            return CurbView { sides: Vec::new() };
        }
        let failing: Vec<usize> = self.curb_failures().iter().map(|f| f.instance).collect();
        let sides = [Side::Left, Side::Right]
            .into_iter()
            .filter(|&side| content.curb.iter().any(|i| i.side == side) || (0..content.zone_count()).any(|z| curbside(content.section(z), side).is_some()))
            .map(|side| {
                let gap = |from_mm: i32, to_mm: i32| CurbCell { instance: None, from_mm, to_mm, use_: None, label: Said::new("curb-gap"), ok: true };
                let items = content.curb.iter().enumerate().filter(|(_, i)| i.side == side).map(|(k, i)| (k, i.from_mm, i.to_mm()));
                let cells = strip_cells(length_mm, items, gap, |k| {
                    let i = &content.curb[k];
                    CurbCell {
                        instance: Some(k),
                        from_mm: i.from_mm,
                        to_mm: i.to_mm(),
                        use_: Some(i.use_),
                        label: Said::new(i.use_.name_key()),
                        ok: !failing.contains(&k),
                    }
                });
                let selected = selected_cell(&cells, self.curb_choice.at, side);
                SideView { side, cells, selected }
            })
            .collect();
        CurbView { sides }
    }

    /// Chooses the cell of `side`'s strip that holds `at_mm`. A setting, not history; a cell is found again by its
    /// place, so it stays chosen as the uses around it change.
    pub fn select_curb(&mut self, side: Side, at_mm: i32) {
        self.curb_choice.pick(side, at_mm);
    }

    /// Whether a drag (of a zone boundary or a use) is under way.
    pub fn dragging(&self) -> bool {
        self.gesture.is_some()
    }

    /// Starts a drag of a use: the steps are shown and the end is one revision. False when a drag is under way.
    pub fn begin_curb_drag(&mut self) -> bool {
        if self.gesture.is_some() {
            return false;
        }
        self.begin_gesture();
        true
    }
}

impl Editor {
    pub(super) fn curb_frame(&self, allow_drag: bool) -> Result<ZoneFrame, CurbRefusal> {
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
        // The whole run, not only where it starts: a use may not reach into a lane that changes with the time of day.
        if content
            .sections_over(candidate.from_mm, candidate.to_mm(), frame.length_mm)
            .into_iter()
            .any(|section| matches!(locate(section, candidate.side), Lane::Timed))
        {
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
        self.curb_choice.pick(side, i.from_mm);
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
        // The choice follows the use, and a drag records one move, from where it began to where it is.
        self.curb_choice.follow(old.side, old.from_mm, new.from_mm);
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

/// The pieces of `segs` between the curbside piece `c` of `side` and that side's edge, nearest the lane first.
pub(super) fn toward_edge(segs: &[Segment], c: usize, side: Side) -> Vec<&Segment> {
    match side {
        Side::Left => segs[..c].iter().rev().collect(),
        Side::Right => segs[c + 1..].iter().collect(),
    }
}

/// The pieces of `segs` between the curbside piece `c` of `side` and the middle of the street, nearest the lane first.
fn toward_road(segs: &[Segment], c: usize, side: Side) -> Vec<&Segment> {
    match side {
        Side::Left => segs[c + 1..].iter().collect(),
        Side::Right => segs[..c].iter().rev().collect(),
    }
}

pub(super) fn side_word(side: Side) -> &'static str {
    match side {
        Side::Left => "side-word-left",
        Side::Right => "side-word-right",
    }
}

impl Instance {
    /// How the checks name it: "Patio at 42 m, right".
    fn named(&self) -> Said {
        Said::new("curb-instance")
            .with("use", Arg::Msg(self.use_.name_key()))
            .with("at", Arg::Length(self.from_mm))
            .with("side", Arg::Msg(side_word(self.side)))
    }
}

/// One rule one instance (a curbside use, a stop) breaks.
pub(super) struct Failure {
    /// An index into the snapshot's list of instances.
    pub(super) instance: usize,
    /// An index into the rows the rules make.
    pub(super) rule: usize,
    pub(super) detail: Said,
}

impl Failure {
    /// Records a failure of `rule` by instance `instance`, unless that pair is recorded already.
    pub(super) fn record(out: &mut Vec<Failure>, instance: usize, rule: usize, detail: Said) {
        if !out.iter().any(|f| f.instance == instance && f.rule == rule) {
            out.push(Failure { instance, rule, detail });
        }
    }
}

/// The rows of a programme: one per rule (`id`, label, detail when it passes), failing with the first instance that
/// breaks it, which `named` says in `zone`.
pub(super) fn check_rows(rows: &[(&'static str, &'static str, &'static str)], failures: &[Failure], named: impl Fn(usize) -> Said) -> Vec<Check> {
    rows.iter()
        .enumerate()
        .map(|(rule, &(id, label, ok_detail))| match failures.iter().find(|f| f.rule == rule) {
            None => Check { id, ok: true, label: Said::new(label), detail: Said::new(ok_detail), zone: None },
            Some(f) => Check { id, ok: false, label: Said::new(label), detail: f.detail.clone(), zone: Some(named(f.instance)) },
        })
        .collect()
}

impl Editor {
    /// The rows of the curbside programme, after the zones' own (only when the street has instances): one per rule,
    /// failing with the first instance that breaks it, which it names in `zone`.
    ///
    /// An instance is judged against the section of every zone its extent overlaps: the lane, bike and patio rules
    /// fail when they fail in any of them. The length and crossing rules depend on position only. Its side is the section's (see `Instance`),
    /// and `ends` (node `a`, node `b`) is read from the anchored end the positions are measured from.
    pub(super) fn curb_checks(&self) -> Vec<Check> {
        let content = &self.states[self.cursor].content;
        let failures = self.curb_failures();
        let rows = [
            ("curb-length", "check-label-curb-length", "check-detail-curb-length-ok"),
            ("curb-lane", "check-label-curb-lane", "check-detail-curb-lane-ok"),
            ("curb-crossing", "check-label-curb-crossing", "check-detail-curb-crossing-ok"),
            ("curb-bike", "check-label-curb-bike", "check-detail-curb-bike-ok"),
            ("curb-sidewalk", "check-label-curb-sidewalk", "check-detail-curb-sidewalk-ok"),
        ];
        check_rows(&rows, &failures, |k| content.curb[k].named())
    }

    /// Every rule each instance breaks, instance by instance in order: `rule` indexes the five rows of `curb_checks`.
    fn curb_failures(&self) -> Vec<Failure> {
        let content = &self.states[self.cursor].content;
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(i32::MAX);
        let (near, far) = match content.from_end {
            ZoneEnd::First => (self.ends[0], self.ends[1]),
            ZoneEnd::Last => (self.ends[1], self.ends[0]),
        };
        let mut out: Vec<Failure> = Vec::new();
        for (k, i) in content.curb.iter().enumerate() {
            let mut fail = |rule: usize, detail: Said| Failure::record(&mut out, k, rule, detail);
            if i.length_mm < i.use_.min_mm() {
                fail(
                    0,
                    Said::new("check-detail-curb-length")
                        .with("use", Arg::Msg(i.use_.name_key()))
                        .with("length", Arg::Length(i.length_mm))
                        .with("min", Arg::Length(i.use_.min_mm())),
                );
            }
            let stops = matches!(i.use_, Use::Parking | Use::Loading);
            let clear = |from_end: i32| from_end >= CROSSING_CLEARANCE_MM;
            if stops && ((near.crossing && !clear(i.from_mm)) || (far.crossing && !clear(length_mm.saturating_sub(i.to_mm())))) {
                fail(2, Said::new("check-detail-curb-crossing").with("amount", Arg::Length(CROSSING_CLEARANCE_MM)));
            }
            for segs in content.sections_over(i.from_mm, i.to_mm(), length_mm) {
                let Some(c) = curbside(segs, i.side) else {
                    fail(1, Said::new("check-detail-curb-lane"));
                    continue;
                };
                match i.use_ {
                    Use::Parking => {
                        // The bike lane on the curb side, and the buffer pieces between it and the lane.
                        let mut buffer_mm = 0;
                        for s in toward_edge(segs, c, i.side) {
                            match KINDS[s.kind].id {
                                "buffer" => buffer_mm += s.width_mm,
                                "bike" if buffer_mm < DOOR_BUFFER_MM => {
                                    fail(3, Said::new("check-detail-curb-bike-door").with("amount", Arg::Length(DOOR_BUFFER_MM)));
                                    break;
                                }
                                _ => break,
                            }
                        }
                    }
                    Use::Platform => {
                        for s in toward_road(segs, c, i.side) {
                            match KINDS[s.kind].id {
                                "bike" => {
                                    fail(3, Said::new("check-detail-curb-bike-platform"));
                                    break;
                                }
                                "travel" | "bus" => break,
                                _ => {}
                            }
                        }
                    }
                    Use::Patio => {
                        let between = match i.side {
                            Side::Left => &segs[..c],
                            Side::Right => &segs[c + 1..],
                        };
                        let have = widest_walkway_mm(between);
                        if have < ACCESS_LANE_MM {
                            fail(4, Said::new("check-detail-curb-sidewalk").with("have", Arg::Length(have)).with("need", Arg::Length(ACCESS_LANE_MM)));
                        }
                    }
                    _ => {}
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Keep, Piece};
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
        let mut e = Editor::from_street(&s, &s, 0);
        let uid = s.segments[1].uid;
        assert!(e.add_variant(uid), "a parking lane can take a bus window");
        s = e.snapshot();
        assert_eq!(curbside(&s.segments, Side::Left), None);
        assert_eq!(curbside(&s.segments, Side::Right), Some(4));
        let mut e = editor_on(&s);
        assert_eq!(e.add_curb(Side::Left, 10_000, 6_000, Use::Parking), Err(CurbRefusal::TimeWindows));
        assert!(e.add_curb(Side::Right, 10_000, 6_000, Use::Parking).is_ok());
    }

    #[test]
    fn a_use_may_not_reach_into_a_timed_lane() {
        let mut e = editor();
        e.split_zone(Some(33_000)).unwrap();
        e.select_zone(1);
        let uid = right_parking(&e);
        assert!(e.add_variant(uid), "zone 2's parking can take a bus window");
        assert_eq!(e.add_curb(Side::Right, 30_000, 6_000, Use::Parking), Err(CurbRefusal::TimeWindows), "it ends in zone 2");
        assert_eq!(e.add_curb(Side::Right, 36_000, 2_000, Use::Parking), Err(CurbRefusal::TimeWindows), "it lies in zone 2");
        assert!(e.add_curb(Side::Right, 27_000, 6_000, Use::Parking).is_ok(), "it ends where zone 2 begins");
        assert_eq!(e.resize_curb(0, 9_000), Err(CurbRefusal::TimeWindows), "growing into zone 2");
        assert_eq!(e.move_curb(0, 30_000), Err(CurbRefusal::TimeWindows), "moving into zone 2");
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

    // ---- checks ----

    fn check(e: &Editor, id: &str) -> crate::street::model::Check {
        e.view().checks.into_iter().find(|c| c.id == id).unwrap_or_else(|| panic!("no check {id}"))
    }

    fn ok(e: &Editor, id: &str) -> bool {
        check(e, id).ok
    }

    #[test]
    fn no_curb_rows_without_instances() {
        let e = editor();
        assert!(e.view().checks.iter().all(|c| !c.id.starts_with("curb-")));
        let mut e = editor();
        e.add_curb(Side::Right, 10_000, 6_000, Use::Parking).unwrap();
        let ids: Vec<&str> = e.view().checks.iter().map(|c| c.id).filter(|i| i.starts_with("curb-")).collect();
        assert_eq!(ids, ["curb-length", "curb-lane", "curb-crossing", "curb-bike", "curb-sidewalk"]);
        assert!(e.view().checks.iter().all(|c| c.ok), "a parking space in a parking lane passes every row");
    }

    #[test]
    fn curb_length_checks_the_minimum() {
        let mut e = editor();
        e.add_curb(Side::Right, 10_000, 4_000, Use::Loading).unwrap();
        let c = check(&e, "curb-length");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-curb-length");
        assert_eq!(c.zone.as_ref().unwrap().key, "curb-instance");
        e.resize_curb(0, 9_000).unwrap();
        assert!(ok(&e, "curb-length"));
        assert_eq!(check(&e, "curb-length").detail.key, "check-detail-curb-length-ok");
    }

    #[test]
    fn curb_lane_needs_a_curbside_piece_at_the_position() {
        let mut e = editor_on(&Street::sample(3, Side::Right));
        e.add_curb(Side::Left, 10_000, 6_000, Use::Parking).unwrap();
        assert!(!ok(&e, "curb-lane"));
        let mut e = editor();
        e.add_curb(Side::Left, 10_000, 6_000, Use::Parking).unwrap();
        assert!(ok(&e, "curb-lane"));
    }

    #[test]
    fn an_instance_without_a_lane_fails_and_comes_back() {
        let mut e = editor();
        e.add_curb(Side::Right, 50_000, 5_500, Use::Parking).unwrap();
        e.split_zone(Some(40_000)).unwrap();
        e.select_zone(1);
        let parking = right_parking(&e);
        assert!(e.remove(parking));
        assert!(!ok(&e, "curb-lane"), "zone 2 has no parking on the right any more");
        assert_eq!(e.snapshot().curb.len(), 1, "the instance is kept");
        assert!(e.undo());
        assert!(ok(&e, "curb-lane"));
        // The same instance in zone 1 was never affected.
        let mut e = editor();
        e.add_curb(Side::Right, 10_000, 5_500, Use::Parking).unwrap();
        e.split_zone(Some(40_000)).unwrap();
        e.select_zone(1);
        let uid = right_parking(&e);
        assert!(e.remove(uid));
        assert!(ok(&e, "curb-lane"));
    }

    /// The uid of the parking piece nearest the right edge in the selected zone.
    fn right_parking(e: &Editor) -> u32 {
        let parking = kind_index("parking").unwrap();
        e.view().segments.iter().rev().find(|s| s.base_kind == parking).expect("a parking piece").uid
    }

    fn crossing_at_first() -> [EndInfo; 2] {
        [EndInfo { crossing: true }, EndInfo { crossing: false }]
    }

    #[test]
    fn curb_crossing_needs_a_crossing_at_that_end() {
        // A crossing at the anchored end: parking 3 m from it fails, 5 m or more clear passes, the far end is free.
        let s = Street::sample(0, Side::Right);
        let mut e = editor_on(&s).with_ends(crossing_at_first());
        e.add_curb(Side::Right, 3_000, 5_500, Use::Parking).unwrap();
        assert!(!ok(&e, "curb-crossing"));
        assert_eq!(check(&e, "curb-crossing").detail.key, "check-detail-curb-crossing");
        e.move_curb(0, 5_000).unwrap();
        assert!(ok(&e, "curb-crossing"), "exactly 5 m clear is enough");
        e.move_curb(0, LEN - 3_000 - 5_500).unwrap();
        assert!(ok(&e, "curb-crossing"), "3 m from the end with no crossing");
        // Without a crossing at the near end, nothing fails there.
        let mut e = editor_on(&s).with_ends([EndInfo::default(); 2]);
        e.add_curb(Side::Right, 3_000, 5_500, Use::Parking).unwrap();
        assert!(ok(&e, "curb-crossing"));
        // A tree is not parking or loading.
        let mut e = editor_on(&s).with_ends(crossing_at_first());
        e.add_curb(Side::Right, 1_000, 2_000, Use::Tree).unwrap();
        assert!(ok(&e, "curb-crossing"));
    }

    #[test]
    fn the_ends_follow_the_anchored_end() {
        // Anchored at the last end the first node's crossing is at the far end of the measured positions.
        let s = Street::sample(0, Side::Right);
        let last = Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::Last });
        let mut e = Editor::from_street(&s, &s, 0).with_frame(last).with_ends(crossing_at_first());
        e.add_curb(Side::Right, 3_000, 5_500, Use::Parking).unwrap();
        assert!(ok(&e, "curb-crossing"), "3 m from the last end, where there is no crossing");
        e.move_curb(0, LEN - 3_000 - 5_500).unwrap();
        assert!(!ok(&e, "curb-crossing"), "3 m from the first end, where the crossing is");
        let mut e = Editor::from_street(&s, &s, 0).with_frame(last).with_ends([EndInfo { crossing: false }, EndInfo { crossing: true }]);
        e.add_curb(Side::Right, 3_000, 5_500, Use::Loading).unwrap();
        assert!(!ok(&e, "curb-crossing"));
    }

    #[test]
    fn curb_bike_door_zone_and_platform() {
        // Parking beside a bike lane on its curb side: doors open into it.
        let protected = street_of(&[("sidewalk", 2500), ("bike", 1800), ("parking", 2400), ("travel", 3300), ("travel", 3300), ("sidewalk", 2500)]);
        let mut e = editor_on(&protected);
        e.add_curb(Side::Left, 10_000, 6_000, Use::Parking).unwrap();
        assert!(!ok(&e, "curb-bike"));
        assert_eq!(check(&e, "curb-bike").detail.key, "check-detail-curb-bike-door");
        // A buffer of 600 mm between them clears it; a narrower one does not.
        for (w, clear) in [(600, true), (500, false)] {
            let s = street_of(&[("sidewalk", 2500), ("bike", 1800), ("buffer", w), ("parking", 2400), ("travel", 3300), ("travel", 3300), ("sidewalk", 2500)]);
            let mut e = editor_on(&s);
            e.add_curb(Side::Left, 10_000, 6_000, Use::Parking).unwrap();
            assert_eq!(ok(&e, "curb-bike"), clear, "buffer {w}");
        }
        // Loading does not open doors into the bike lane.
        let mut e = editor_on(&protected);
        e.add_curb(Side::Left, 10_000, 9_000, Use::Loading).unwrap();
        assert!(ok(&e, "curb-bike"));
        // A platform with a bike lane between it and the travel lane always fails, as a reminder.
        let boarding = street_of(&[("sidewalk", 2500), ("parking", 2400), ("bike", 1800), ("travel", 3300), ("travel", 3300), ("sidewalk", 2500)]);
        let mut e = editor_on(&boarding);
        e.add_curb(Side::Left, 10_000, 9_000, Use::Platform).unwrap();
        assert!(!ok(&e, "curb-bike"));
        assert_eq!(check(&e, "curb-bike").detail.key, "check-detail-curb-bike-platform");
        let mut e = editor();
        e.add_curb(Side::Left, 10_000, 9_000, Use::Platform).unwrap();
        assert!(ok(&e, "curb-bike"));
    }

    #[test]
    fn curb_sidewalk_keeps_room_beside_a_patio() {
        let narrow = street_of(&[("sidewalk", 2500), ("parking", 2400), ("travel", 3300), ("travel", 3300), ("parking", 2400), ("sidewalk", 2500)]);
        let mut e = editor_on(&narrow);
        e.add_curb(Side::Right, 10_000, 4_000, Use::Patio).unwrap();
        assert!(!ok(&e, "curb-sidewalk"));
        assert_eq!(check(&e, "curb-sidewalk").detail.key, "check-detail-curb-sidewalk");
        let wide = street_of(&[("sidewalk", 3000), ("parking", 2400), ("travel", 3300), ("travel", 3300), ("parking", 2400), ("sidewalk", 3000)]);
        let mut e = editor_on(&wide);
        e.add_curb(Side::Right, 10_000, 4_000, Use::Patio).unwrap();
        assert!(ok(&e, "curb-sidewalk"), "exactly 3 m is enough");
        // Only the patio's own side counts; a parking space there asks nothing of the sidewalk.
        let mut e = editor_on(&narrow);
        e.add_curb(Side::Right, 10_000, 5_500, Use::Parking).unwrap();
        assert!(ok(&e, "curb-sidewalk"));
    }

    #[test]
    fn the_failing_row_names_the_first_instance_in_both_languages() {
        use crate::shared::i18n::Locale;
        use crate::shared::units::Units;
        let mut e = editor();
        e.add_curb(Side::Left, 5_000, 5_500, Use::Parking).unwrap();
        e.add_curb(Side::Right, 42_000, 4_000, Use::Loading).unwrap();
        let c = check(&e, "curb-length");
        assert!(!c.ok);
        let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
        let say = |i18n, said: &Said| crate::street::text::say_now(i18n, Units::Metres, said);
        assert_eq!(say(&en, c.zone.as_ref().unwrap()), "Loading at 42.0 m, right");
        assert_eq!(say(&fr, c.zone.as_ref().unwrap()), "Zone de chargement à 42,0\u{a0}m, à droite");
        assert_eq!(say(&en, &c.detail), "Loading is 4.0 m long, under the 9.0 m it needs.");
        assert_eq!(say(&fr, &c.detail), "Zone de chargement\u{a0}: 4,0\u{a0}m de long, moins que les 9,0\u{a0}m demandés.");
    }

    #[test]
    fn an_instance_is_judged_in_every_zone_it_crosses() {
        // Parking from 30 m to 36 m across a boundary at 33 m; zone 2 has no parking on the right.
        let build = |at: i32| {
            let mut e = editor();
            e.add_curb(Side::Right, at, 6_000, Use::Parking).unwrap();
            e.split_zone(Some(33_000)).unwrap();
            e.select_zone(1);
            let uid = right_parking(&e);
            assert!(e.remove(uid));
            e
        };
        assert!(!ok(&build(30_000), "curb-lane"), "it reaches into zone 2");
        assert!(ok(&build(26_000), "curb-lane"), "it ends before the boundary");
        assert!(!ok(&build(33_000), "curb-lane"), "it lies wholly in zone 2");
    }

    #[test]
    fn a_bike_lane_in_the_second_zone_fails_the_door_rule() {
        let base = street_of(&[("sidewalk", 2500), ("parking", 2400), ("travel", 3300), ("travel", 3300), ("sidewalk", 2500)]);
        let protected = street_of(&[("sidewalk", 2500), ("bike", 1800), ("parking", 2400), ("travel", 3300), ("travel", 3300), ("sidewalk", 2500)]);
        let mut s = crate::street::model::zoned(base, &[0, 33_000], LEN);
        s.zones[1].segments = Some(protected.segments.clone());
        let mut e = editor_on(&s);
        e.add_curb(Side::Left, 26_000, 6_000, Use::Parking).unwrap();
        assert!(ok(&e, "curb-bike"), "wholly in zone 1");
        e.move_curb(0, 30_000).unwrap();
        assert!(!ok(&e, "curb-bike"), "reaches zone 2, where a bike lane is beside the parking");
    }

    #[test]
    fn a_narrow_sidewalk_in_the_second_zone_fails_the_patio_rule() {
        let wide = street_of(&[("sidewalk", 3000), ("parking", 2400), ("travel", 3300), ("travel", 3300), ("parking", 2400), ("sidewalk", 3000)]);
        let narrow = street_of(&[("sidewalk", 2500), ("parking", 2400), ("travel", 3300), ("travel", 3300), ("parking", 2400), ("sidewalk", 2500)]);
        let mut s = crate::street::model::zoned(wide, &[0, 33_000], LEN);
        s.zones[1].segments = Some(narrow.segments.clone());
        let mut e = editor_on(&s);
        e.add_curb(Side::Right, 26_000, 4_000, Use::Patio).unwrap();
        assert!(ok(&e, "curb-sidewalk"));
        e.move_curb(0, 30_000).unwrap();
        assert!(!ok(&e, "curb-sidewalk"));
    }
}
