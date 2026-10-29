//! Street trees and green infrastructure: a tree pit, a planting bed, a rain garden or a bioswale, placed along a
//! street and across its section.
//!
//! Like the curbside uses (`curb.rs`) and the transit stops (`stop.rs`) they belong to the street, not to a
//! section: a position along is measured from the anchored end (`Street::from_end`), so splitting, merging or moving
//! zones leaves them where they are. Across, `offset_mm` is the element's centre from the section's left edge.
//! A street with none is written, drawn and checked exactly as before.

use serde::{Deserialize, Serialize};

use super::curb::{CurbRefusal, Failure, check_rows};
use super::{Arg, Check, Editor, SNAP_MM, Said, Segment, Street, ZoneEnd, ZoneFrame, snap, total};
use crate::shared::catalogue::{KINDS, is_roadway};

/// The walkway a sidewalk keeps beside an element standing on it, in millimetres.
///
/// UNVERIFIED (placeholder): 1 800 mm, two people passing.
pub const CLEAR_PATH_MM: i32 = 1_800;

/// How far from a junction a tree or a bed keeps back, so that drivers and people crossing see each other, in
/// millimetres.
///
/// UNVERIFIED (placeholder): 10 000 mm.
pub const SIGHTLINE_MM: i32 = 10_000;

/// The shortest a bioswale can be and still carry water, in millimetres.
///
/// UNVERIFIED (placeholder): 10 000 mm.
pub const BIOSWALE_MIN_MM: i32 = 10_000;

/// The narrowest a default tree pit is made to leave the walkway beside it, in millimetres; a sidewalk that would
/// leave less gets the pit as wide as itself.
///
/// UNVERIFIED (placeholder): 600 mm.
const MIN_TREE_WIDTH_MM: i32 = 600;

/// What an element is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GreenKind {
    /// One tree in its pit.
    Tree,
    /// A planting bed.
    Bed,
    /// A planted basin that takes the street's runoff.
    RainGarden,
    /// A long planted channel that carries and soaks up runoff.
    Bioswale,
}

impl GreenKind {
    /// Every kind, in the order a choice lists them.
    pub const ALL: [GreenKind; 4] = [GreenKind::Tree, GreenKind::Bed, GreenKind::RainGarden, GreenKind::Bioswale];

    /// The message that names it.
    pub fn name_key(self) -> &'static str {
        match self {
            GreenKind::Tree => "green-kind-tree",
            GreenKind::Bed => "green-kind-bed",
            GreenKind::RainGarden => "green-kind-rain-garden",
            GreenKind::Bioswale => "green-kind-bioswale",
        }
    }

    /// The message that counts them: "2 trees".
    pub fn count_key(self) -> &'static str {
        match self {
            GreenKind::Tree => "green-count-tree",
            GreenKind::Bed => "green-count-bed",
            GreenKind::RainGarden => "green-count-rain-garden",
            GreenKind::Bioswale => "green-count-bioswale",
        }
    }

    /// How long a new element is along the street.
    ///
    /// UNVERIFIED (placeholder): a tree pit 1 500 mm square, a bed 4 000 mm, a rain garden 6 000 mm, a bioswale
    /// 20 000 mm.
    pub fn default_length_mm(self) -> i32 {
        match self {
            GreenKind::Tree => 1_500,
            GreenKind::Bed => 4_000,
            GreenKind::RainGarden => 6_000,
            GreenKind::Bioswale => 20_000,
        }
    }
}

/// The size a tree grows to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TreeSize {
    S,
    M,
    L,
}

impl TreeSize {
    /// The width of the crown, which is also the spacing between trees.
    ///
    /// UNVERIFIED (placeholder): 6 000, 8 000 and 10 000 mm.
    pub fn canopy_mm(self) -> i32 {
        match self {
            TreeSize::S => 6_000,
            TreeSize::M => 8_000,
            TreeSize::L => 10_000,
        }
    }

    /// The side of the square pit a new tree of this size is given, enough soil for its size.
    ///
    /// UNVERIFIED (placeholder): 1 500, 1 900 and 2 300 mm, each at least the size's least area.
    pub fn default_pit_mm(self) -> i32 {
        match self {
            TreeSize::S => 1_500,
            TreeSize::M => 1_900,
            TreeSize::L => 2_300,
        }
    }

    /// The least area of soil a pit gives this tree, in square millimetres.
    ///
    /// UNVERIFIED (placeholder): 2.25, 3.5 and 5 square metres.
    pub fn min_pit_mm2(self) -> i64 {
        match self {
            TreeSize::S => 2_250_000,
            TreeSize::M => 3_500_000,
            TreeSize::L => 5_000_000,
        }
    }
}

/// One element, `length_mm` along the street from `from_mm`, centred `offset_mm` from the section's left edge and
/// `width_mm` across. `size` is set for a tree and only for a tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Green {
    pub kind: GreenKind,
    /// Millimetres from the anchored end (`Street::from_end`) to where it starts, a multiple of `SNAP_MM`.
    pub from_mm: i32,
    pub length_mm: i32,
    /// The element's centre across, from the left edge of the section (`SegView.x_mm`). The anchored end does not
    /// flip it.
    pub offset_mm: i32,
    pub width_mm: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<TreeSize>,
}

/// Why a change to the elements was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GreenRefusal {
    /// Both ends of the street are cut by the map's box, so there is no end to measure from.
    NoAnchor,
    /// The elements were drawn on a road of another length or anchored at its other end.
    Elsewhere,
    /// A drag is under way.
    Dragging,
    /// It would reach past the street's ends, or past the section's edges across.
    OffStreet,
    /// Some of it would lie over a piece of the roadway.
    OnRoadway,
    /// It would overlap another element.
    Overlap,
}

impl GreenRefusal {
    /// The refusal, in words.
    pub fn said(self) -> Said {
        Said::new(match self {
            GreenRefusal::NoAnchor => "zone-refused-no-anchor",
            GreenRefusal::Elsewhere => "green-refused-elsewhere",
            GreenRefusal::Dragging => "green-refused-dragging",
            GreenRefusal::OffStreet => "green-refused-off-street",
            GreenRefusal::OnRoadway => "green-refused-on-roadway",
            GreenRefusal::Overlap => "green-refused-overlap",
        })
    }
}

impl From<CurbRefusal> for GreenRefusal {
    /// The gate (`Editor::curb_frame`) raises only its first three; the others are the curbside's own rules and have
    /// the nearest meaning here.
    fn from(r: CurbRefusal) -> GreenRefusal {
        match r {
            CurbRefusal::NoAnchor => GreenRefusal::NoAnchor,
            CurbRefusal::Elsewhere => GreenRefusal::Elsewhere,
            CurbRefusal::Dragging => GreenRefusal::Dragging,
            CurbRefusal::Overlap => GreenRefusal::Overlap,
            CurbRefusal::TooLong | CurbRefusal::TimeWindows => GreenRefusal::OffStreet,
        }
    }
}

/// The piece of the section under `offset_mm` across, counted from its left edge (a piece owns its left edge; the far
/// edge of the section belongs to the last piece). `None` outside the section.
pub fn host_at(segs: &[Segment], offset_mm: i32) -> Option<usize> {
    if offset_mm < 0 {
        return None;
    }
    let mut edge = 0;
    for (i, s) in segs.iter().enumerate() {
        edge += s.width_mm;
        if offset_mm < edge || (offset_mm == edge && i + 1 == segs.len()) {
            return Some(i);
        }
    }
    None
}

/// The span across, in millimetres from the section's left edge, that piece `host` fills.
pub fn fills(segs: &[Segment], host: usize) -> (i32, i32) {
    let from_mm = segs[..host].iter().map(|s| s.width_mm).sum();
    (from_mm, from_mm + segs[host].width_mm)
}

/// Whether the span from `left2` to `right2` (twice the offsets across, so that an odd width needs no rounding) lies
/// over any piece of the roadway in `segs`. Touching an edge is not reaching.
fn reaches_roadway(segs: &[Segment], left2: i64, right2: i64) -> bool {
    let mut edge = 0i64;
    segs.iter().any(|s| {
        let (from2, to2) = (2 * edge, 2 * (edge + i64::from(s.width_mm)));
        edge += i64::from(s.width_mm);
        is_roadway(s.kind) && left2 < to2 && from2 < right2
    })
}

/// Where an element is, to find it again after the list is sorted: `(from_mm, offset_mm)`.
type Place = (i32, i32);

/// The element the person chose, and the move that the next revision records (as `Choice` does for stops).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct GreenChoice {
    at: Option<Place>,
    moved: Option<(Place, Place)>,
}

impl GreenChoice {
    fn pick(&mut self, at: Place) {
        self.at = Some(at);
    }

    fn clear_if(&mut self, at: Place) {
        if self.at == Some(at) {
            self.at = None;
        }
    }

    /// The element at `from` moves to `to`: the choice follows it, and a drag records one move, from where it began.
    fn follow(&mut self, from: Place, to: Place) {
        if self.at == Some(from) {
            self.at = Some(to);
        }
        let began = self.moved.filter(|&(_, end)| end == from).map_or(from, |(began, _)| began);
        self.moved = Some((began, to));
    }

    pub(super) fn take_moved(&mut self) -> Option<(Place, Place)> {
        self.moved.take()
    }

    pub(super) fn forget_moved(&mut self) {
        self.moved = None;
    }

    /// A drag was cancelled: the element is back where it started, so is the choice.
    pub(super) fn cancel(&mut self) {
        if let Some(moved) = self.moved.take() {
            self.undo(Some(moved));
        }
    }

    pub(super) fn undo(&mut self, moved: Option<(Place, Place)>) {
        if let Some((from, to)) = moved
            && self.at == Some(to)
        {
            self.at = Some(from);
        }
    }

    pub(super) fn redo(&mut self, moved: Option<(Place, Place)>) {
        if let Some((from, to)) = moved
            && self.at == Some(from)
        {
            self.at = Some(to);
        }
    }
}

pub(super) fn sort_green(green: &mut [Green]) {
    green.sort_by_key(|g| (g.from_mm, g.offset_mm));
}

impl Street {
    pub(super) fn sort_green(&mut self) {
        sort_green(&mut self.green);
    }

    /// The total width of the section that lies at `along_mm` from the anchored end: the zone's own section, or
    /// the base. Sections are laid out from x = 0 and may total less than `row_mm`.
    fn section_total_at(&self, along_mm: i32) -> i32 {
        let zone = self.zones.iter().rev().find(|z| z.from_mm <= along_mm);
        total(zone.and_then(|z| z.segments.as_deref()).unwrap_or(&self.segments))
    }

    /// Mirrors the elements across for a street seen from its other end. Their positions along stay measured from
    /// the same physical end (`reverse_zones` flips the label of that end); the section is reversed, so each
    /// centre is measured from the other edge, of the section under the element.
    pub(super) fn reverse_green(&mut self) {
        let mirrored: Vec<Green> =
            self.green.iter().map(|g| Green { offset_mm: self.section_total_at(g.from_mm.saturating_add(g.length_mm / 2)) - g.offset_mm, ..*g }).collect();
        self.green = mirrored;
        self.sort_green();
    }

    /// Whether the elements can be placed: they need the length they were drawn against, and each lies in front
    /// of it with a size exactly when it is a tree.
    pub(super) fn green_is_sound(&self) -> bool {
        self.green.is_empty()
            || (self.length_mm.is_some()
                && self.green.iter().all(|g| {
                    g.from_mm >= 0
                        && g.length_mm > 0
                        && g.width_mm > 0
                        && (0..=self.section_total_at(g.from_mm.saturating_add(g.length_mm / 2))).contains(&g.offset_mm)
                        && g.size.is_some() == (g.kind == GreenKind::Tree)
                }))
    }
}

/// How far apart the middles of two elements are, along the street and across it, in millimetres: what crowns need
/// between them.
fn centre_gap_mm(a: &Green, b: &Green) -> i32 {
    let along = f64::from(b.centre_mm()) - f64::from(a.centre_mm());
    let across = f64::from(b.offset_mm) - f64::from(a.offset_mm);
    along.hypot(across).round() as i32
}

impl Green {
    fn place(&self) -> Place {
        (self.from_mm, self.offset_mm)
    }

    fn centre_mm(&self) -> i32 {
        self.from_mm.saturating_add(self.length_mm / 2)
    }

    /// How the checks name it: "Tree at 42 m, on sidewalk", the piece being the one under its centre in `segs`.
    pub fn named(&self, segs: &[Segment]) -> Said {
        let kind = Arg::Msg(self.kind.name_key());
        match host_at(segs, self.offset_mm) {
            Some(h) => {
                Said::new("green-instance").with("kind", kind).with("at", Arg::Length(self.from_mm)).with("across", Arg::KindLower(KINDS[segs[h].kind].id))
            }
            None => Said::new("green-instance-off").with("kind", kind).with("at", Arg::Length(self.from_mm)),
        }
    }

    fn to_mm(&self) -> i32 {
        self.from_mm.saturating_add(self.length_mm)
    }
}

impl Editor {
    fn green_gate(&self, allow_drag: bool) -> Result<ZoneFrame, GreenRefusal> {
        Ok(self.curb_frame(allow_drag)?)
    }

    /// Whether `candidate` may stand where it is, ignoring the element `own` (the one being changed). The checks run
    /// in order: off the street, on the roadway, overlapping.
    fn green_fits(&self, candidate: &Green, own: Option<usize>, frame: ZoneFrame) -> Result<(), GreenRefusal> {
        let content = &self.states[self.cursor].content;
        if candidate.from_mm < 0 || candidate.to_mm() > frame.length_mm {
            return Err(GreenRefusal::OffStreet);
        }
        let sections = content.sections_over(candidate.from_mm, candidate.to_mm(), frame.length_mm);
        // Twice the centre, so that an odd width needs no rounding.
        let (left2, right2) =
            (2 * i64::from(candidate.offset_mm) - i64::from(candidate.width_mm), 2 * i64::from(candidate.offset_mm) + i64::from(candidate.width_mm));
        if sections.iter().any(|segs| left2 < 0 || right2 > 2 * i64::from(total(segs))) {
            return Err(GreenRefusal::OffStreet);
        }
        // Any part of the rectangle over a roadway piece, in any section it overlaps, not only the centre.
        if sections.iter().any(|segs| reaches_roadway(segs, left2, right2)) {
            return Err(GreenRefusal::OnRoadway);
        }
        let overlaps = content.green.iter().enumerate().any(|(k, o)| {
            Some(k) != own
                && o.from_mm < candidate.to_mm()
                && candidate.from_mm < o.to_mm()
                && 2 * i64::from(o.offset_mm) - i64::from(o.width_mm) < right2
                && left2 < 2 * i64::from(o.offset_mm) + i64::from(o.width_mm)
        });
        if overlaps {
            return Err(GreenRefusal::Overlap);
        }
        Ok(())
    }

    /// A revision's words: what the element is and where along the street it starts.
    fn green_label(key: &'static str, g: &Green) -> Said {
        Said::new(key).with("kind", Arg::MsgLower(g.kind.name_key())).with("at", Arg::Length(g.from_mm))
    }

    /// Records a change of the elements as one revision, writing the length and end they are measured against.
    fn green_change(&mut self, label: Said, frame: ZoneFrame, f: impl FnOnce(&mut Vec<Green>)) {
        self.edit_content(label, |c| {
            f(&mut c.green);
            sort_green(&mut c.green);
            c.from_end = frame.from_end;
            c.length_mm = Some(frame.length_mm);
            true
        });
    }

    /// Changes element `i` (an index into the snapshot's `green`) into what `change` makes of it, if that fits, as one
    /// revision. Nothing happens for an index that is not one.
    fn green_replace(&mut self, i: usize, label_key: &'static str, change: impl FnOnce(&Green) -> Green) -> Result<(), GreenRefusal> {
        let frame = self.green_gate(false)?;
        let Some(old) = self.states[self.cursor].content.green.get(i).copied() else { return Ok(()) };
        let new = change(&old);
        self.green_fits(&new, Some(i), frame)?;
        self.green_change(Self::green_label(label_key, &new), frame, |green| green[i] = new);
        Ok(())
    }

    /// Adds an element, its position, length and width snapped to `SNAP_MM` (the lengths at least that). A tree with
    /// no size is a medium one; any other kind has none. A tree with no length or width (0) gets the pit its size
    /// needs (`TreeSize::default_pit_mm`). One revision; the new element is the chosen one.
    pub fn add_green(&mut self, g: Green) -> Result<(), GreenRefusal> {
        let frame = self.green_gate(false)?;
        let size = if g.kind == GreenKind::Tree { g.size.or(Some(TreeSize::M)) } else { None };
        // A tree that comes with no length or width gets the pit its size needs.
        let pit = size.map(TreeSize::default_pit_mm);
        let g = Green {
            from_mm: snap(g.from_mm),
            length_mm: snap(if g.length_mm <= 0 { pit.unwrap_or(0) } else { g.length_mm }).max(SNAP_MM),
            offset_mm: snap(g.offset_mm),
            width_mm: snap(if g.width_mm <= 0 { pit.unwrap_or(0) } else { g.width_mm }).max(SNAP_MM),
            size,
            ..g
        };
        self.green_fits(&g, None, frame)?;
        self.green_choice.pick(g.place());
        self.green_change(Self::green_label("rev-green-add", &g), frame, |green| green.push(g));
        Ok(())
    }

    /// Moves element `i` along the street to start at `from_mm` (snapped). Inside a gesture it is shown as it goes
    /// and recorded as one revision at the end.
    pub fn move_green(&mut self, i: usize, from_mm: i32) -> Result<(), GreenRefusal> {
        self.green_move_to(i, |old| Green { from_mm: snap(from_mm), ..*old })
    }

    /// Moves element `i` across the section to be centred `offset_mm` from its left edge (snapped), as `move_green`
    /// does.
    pub fn move_green_across(&mut self, i: usize, offset_mm: i32) -> Result<(), GreenRefusal> {
        self.green_move_to(i, |old| Green { offset_mm: snap(offset_mm), ..*old })
    }

    fn green_move_to(&mut self, i: usize, change: impl FnOnce(&Green) -> Green) -> Result<(), GreenRefusal> {
        let frame = self.green_gate(true)?;
        let Some(old) = self.states[self.cursor].content.green.get(i).copied() else { return Ok(()) };
        let new = change(&old);
        self.green_fits(&new, Some(i), frame)?;
        if new == old {
            return Ok(());
        }
        self.green_choice.follow(old.place(), new.place());
        let label = Self::green_label("rev-green-move", &new);
        if self.gesture.is_some() {
            let c = &mut self.states[self.cursor].content;
            c.green[i] = new;
            sort_green(&mut c.green);
            c.from_end = frame.from_end;
            c.length_mm = Some(frame.length_mm);
            self.pending_label = Some(label);
        } else {
            self.green_change(label, frame, |green| green[i] = new);
        }
        Ok(())
    }

    /// Gives element `i` a new length and width (snapped, at least `SNAP_MM`), about the same start and centre.
    pub fn resize_green(&mut self, i: usize, length_mm: i32, width_mm: i32) -> Result<(), GreenRefusal> {
        self.green_replace(i, "rev-green-resize", |old| Green { length_mm: snap(length_mm).max(SNAP_MM), width_mm: snap(width_mm).max(SNAP_MM), ..*old })
    }

    /// Sets the size of tree `i`. Any other kind has no size, so nothing changes for it.
    pub fn set_green_size(&mut self, i: usize, size: TreeSize) -> Result<(), GreenRefusal> {
        let frame = self.green_gate(false)?;
        let Some(old) = self.states[self.cursor].content.green.get(i).copied() else { return Ok(()) };
        if old.kind != GreenKind::Tree {
            return Ok(());
        }
        let new = Green { size: Some(size), ..old };
        self.green_change(Self::green_label("rev-green-size", &new), frame, |green| green[i] = new);
        Ok(())
    }

    /// Removes element `i`.
    pub fn remove_green(&mut self, i: usize) -> Result<(), GreenRefusal> {
        let frame = self.green_gate(false)?;
        let Some(old) = self.states[self.cursor].content.green.get(i).copied() else { return Ok(()) };
        self.green_choice.clear_if(old.place());
        self.green_change(Self::green_label("rev-green-remove", &old), frame, |green| {
            green.remove(i);
        });
        Ok(())
    }

    /// Chooses element `i`. A setting, not history; an index that is none leaves nothing chosen.
    pub fn select_green(&mut self, i: usize) {
        self.green_choice.at = self.states[self.cursor].content.green.get(i).map(Green::place);
    }

    /// The index of the chosen element in the snapshot's `green`, if it is still there.
    pub fn selected_green(&self) -> Option<usize> {
        let at = self.green_choice.at?;
        self.states[self.cursor].content.green.iter().position(|g| g.place() == at)
    }

    /// Starts a drag of an element: the steps are shown and the end is one revision. False when a drag is under way.
    pub fn begin_green_drag(&mut self) -> bool {
        self.begin_curb_drag()
    }
}

/// What the person asks for when they add an element or change one. For an add, a field left `None` takes its
/// default (see `Editor::add_green_from`); for a change, a field left `None` keeps what the element has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GreenForm {
    pub kind: GreenKind,
    /// A tree's size; any other kind has none.
    pub size: Option<TreeSize>,
    pub length_mm: Option<i32>,
    pub width_mm: Option<i32>,
    /// Where it starts along the street, from the anchored end.
    pub at_mm: Option<i32>,
    /// The piece it stands on, an index into the section at `at_mm` (see `Editor::hosts_at`).
    pub host: Option<usize>,
    /// How far its centre lies from the middle of that piece, across (negative: toward the section's left edge).
    pub fine_mm: Option<i32>,
}

impl GreenForm {
    /// An element of `kind` with everything left to the defaults.
    pub fn new(kind: GreenKind) -> GreenForm {
        GreenForm { kind, size: None, length_mm: None, width_mm: None, at_mm: None, host: None, fine_mm: None }
    }
}

/// `mm` rounded down to the grid.
fn snap_down(mm: i32) -> i32 {
    mm.div_euclid(SNAP_MM) * SNAP_MM
}

/// `mm` rounded up to the grid.
fn snap_up(mm: i32) -> i32 {
    -snap_down(-mm)
}

/// How wide a new element is, and where its centre lies across, when it stands on piece `host` of `segs` with no
/// width or place asked for. `width_mm` is the width asked for, if any.
///
/// A tree has the pit of its size; any other kind is as wide as its host, except that on a sidewalk it leaves the
/// walkway (`CLEAR_PATH_MM`), the host's whole width when that leaves under `SNAP_MM`. On a sidewalk it stands
/// against the edge nearer the roadway, which leaves the widest walkway; on any other piece it is centred. A width
/// that was not asked for is rounded down to the grid and the centre is on the grid, so the rectangle stays within
/// the host: where no centre on the grid keeps it there, or the walkway it should leave, the width gives up one
/// grid step at a time.
pub fn default_across(segs: &[Segment], host: usize, kind: GreenKind, size: Option<TreeSize>, width_mm: Option<i32>) -> (i32, i32) {
    let (from, to) = fills(segs, host);
    let host_mm = to - from;
    let on_sidewalk = KINDS[segs[host].kind].id == "sidewalk";
    let path = host_mm - CLEAR_PATH_MM;
    // Whether the walkway is to be kept: a default width on a sidewalk that has room for one.
    let keeps_path = on_sidewalk
        && match kind {
            GreenKind::Tree => path >= MIN_TREE_WIDTH_MM,
            _ => path >= SNAP_MM,
        };
    let asked = width_mm.filter(|w| *w > 0).map(|w| snap(w).max(SNAP_MM));
    let target = asked.unwrap_or_else(|| {
        snap_down(match kind {
            GreenKind::Tree => {
                let pit = size.unwrap_or(TreeSize::M).default_pit_mm();
                if keeps_path { pit.min(path) } else { pit.min(host_mm) }
            }
            _ if keeps_path => path,
            _ => host_mm,
        })
        .max(SNAP_MM)
    });
    // The nearest roadway piece says which edge of a sidewalk is the road side; with none, it is centred.
    let road_right = on_sidewalk
        .then(|| {
            (1..segs.len()).find_map(|d| {
                let right = host + d < segs.len() && is_roadway(segs[host + d].kind);
                let left = host >= d && is_roadway(segs[host - d].kind);
                (left || right).then_some(right && !left)
            })
        })
        .flatten();
    // The centre on the grid for `width`, if one keeps the rectangle within the host.
    let centre_for = |width: i32| -> Option<i32> {
        let lo = snap_up((2 * from + width + 1).div_euclid(2));
        let hi = snap_down((2 * to - width).div_euclid(2));
        (lo <= hi).then(|| match road_right {
            Some(true) => hi,
            Some(false) => lo,
            None => snap((from + to) / 2).clamp(lo, hi),
        })
    };
    let leaves_path = |width: i32, centre: i32| {
        !(asked.is_none() && keeps_path && road_right.is_some()) || (centre - width / 2 - from).max(to - centre - width / 2) >= CLEAR_PATH_MM
    };
    let mut width = target;
    loop {
        match centre_for(width) {
            Some(centre) if leaves_path(width, centre) => return (centre, width),
            // An asked width is kept whatever it does; the checks speak.
            _ if asked.is_some() || width <= SNAP_MM => return (centre_for(width).unwrap_or_else(|| snap((from + to) / 2)), width),
            _ => width -= SNAP_MM,
        }
    }
}

/// One element as the Green row lists it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GreenCell {
    /// An index into the street's `green`.
    pub index: usize,
    pub from_mm: i32,
    pub to_mm: i32,
    /// The line of the row it lies on: elements side by side across take lines of their own.
    pub track: usize,
    pub kind: GreenKind,
    pub size: Option<TreeSize>,
    pub width_mm: i32,
    /// The piece under its centre, an index into the section at its start; none when the centre lies off the section.
    pub host: Option<usize>,
    /// How far its centre lies from the middle of that piece, across.
    pub fine_mm: i32,
    /// What it is called: its kind, where along and on what.
    pub label: Said,
    /// It breaks a check.
    pub failing: bool,
}

/// The Green row: every element along the street, in order.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct GreenRowView {
    /// The street's length, which the cells are laid along.
    pub length_mm: i32,
    pub cells: Vec<GreenCell>,
    /// The chosen element, an index into `cells`.
    pub selected: Option<usize>,
    /// How many lines the cells need.
    pub tracks: usize,
    /// An element can be added: the street has a length to lay it along and a piece to stand it on.
    pub can_add: bool,
}

impl Editor {
    fn green_length(&self) -> i32 {
        let content = &self.states[self.cursor].content;
        content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(0)
    }

    /// The Green row. Empty (and not offering an add) while the street's length is unknown.
    pub(super) fn green_row(&self) -> GreenRowView {
        let content = &self.states[self.cursor].content;
        let length_mm = self.green_length();
        if length_mm <= 0 {
            return GreenRowView::default();
        }
        let can_add = self.curb_frame(true).is_ok() && (0..content.zone_count()).any(|z| content.section(z).iter().any(|s| !is_roadway(s.kind)));
        let failing: Vec<usize> = self.green_failures().iter().map(|f| f.instance).collect();
        let mut track_ends: Vec<i32> = Vec::new();
        let cells: Vec<GreenCell> = content
            .green
            .iter()
            .enumerate()
            .map(|(k, g)| {
                let track = track_ends.iter().position(|&end| end <= g.from_mm).unwrap_or(track_ends.len());
                if track == track_ends.len() {
                    track_ends.push(0);
                }
                track_ends[track] = g.to_mm();
                let sections = content.sections_over(g.from_mm, g.to_mm(), length_mm);
                let segs = sections.first().copied().unwrap_or(&content.segments);
                let host = host_at(segs, g.offset_mm);
                let fine_mm = host.map_or(0, |h| {
                    let (from, to) = fills(segs, h);
                    g.offset_mm - (from + to) / 2
                });
                GreenCell {
                    index: k,
                    from_mm: g.from_mm,
                    to_mm: g.to_mm(),
                    track,
                    kind: g.kind,
                    size: g.size,
                    width_mm: g.width_mm,
                    host,
                    fine_mm,
                    label: g.named(segs),
                    failing: failing.contains(&k),
                }
            })
            .collect();
        GreenRowView { length_mm, tracks: track_ends.len(), selected: self.selected_green(), cells, can_add }
    }

    /// Element `i`, if there is one.
    pub fn green_at(&self, i: usize) -> Option<Green> {
        self.states[self.cursor].content.green.get(i).copied()
    }

    /// The section the street has at `at_mm` from the anchored end: the section of the zone there, or of the
    /// first zone an element starting there overlaps.
    fn section_at(&self, at_mm: i32) -> Option<&[Segment]> {
        let length_mm = self.green_length();
        if length_mm <= 0 || at_mm < 0 || at_mm >= length_mm {
            return None;
        }
        self.states[self.cursor].content.sections_over(at_mm, at_mm + 1, length_mm).first().copied()
    }

    /// The pieces an element starting at `at_mm` can stand on: the section there without its roadway pieces, each as
    /// an index into that section and a name.
    pub fn hosts_at(&self, at_mm: i32) -> Vec<(usize, Said)> {
        let Some(segs) = self.section_at(at_mm) else { return Vec::new() };
        segs.iter()
            .enumerate()
            .filter(|(_, s)| !is_roadway(s.kind))
            .map(|(i, s)| (i, Said::new("green-host-choice").with("kind", Arg::Kind(KINDS[s.kind].id)).with("width", Arg::Length(s.width_mm))))
            .collect()
    }

    /// The element `form` asks for, with its defaults filled in: where it starts (0), the piece it stands on (the
    /// first that can hold it), its width and place across (`default_across`) and its length (the kind's, a tree's
    /// pit). `own` is the element being changed, whose values stand in for the defaults.
    fn green_from_form(&self, form: &GreenForm, own: Option<&Green>) -> Result<Green, GreenRefusal> {
        let at_mm = form.at_mm.or(own.map(|g| g.from_mm)).unwrap_or(0);
        let size = if form.kind == GreenKind::Tree { form.size.or(own.and_then(|g| g.size)).or(Some(TreeSize::M)) } else { None };
        let Some(segs) = self.section_at(at_mm) else { return Err(GreenRefusal::OffStreet) };
        let hosts = self.hosts_at(at_mm);
        let host = match form.host {
            Some(h) => h,
            None => match own {
                Some(g) => host_at(segs, g.offset_mm).unwrap_or(0),
                None => hosts.first().map(|(i, _)| *i).ok_or(GreenRefusal::OnRoadway)?,
            },
        };
        if host >= segs.len() {
            return Err(GreenRefusal::OffStreet);
        }
        let width_mm = form.width_mm.or(own.map(|g| g.width_mm));
        let (offset_mm, width_mm) = default_across(segs, host, form.kind, size, width_mm);
        // Only a typed offset moves an element that stays on its piece.
        let own_host = own.and_then(|g| host_at(segs, g.offset_mm));
        let offset_mm = match (form.fine_mm, own) {
            (Some(fine), _) => {
                let (from, to) = fills(segs, host);
                (from + to) / 2 + fine
            }
            (None, Some(g)) if form.host.is_none() || form.host == own_host => g.offset_mm,
            _ => offset_mm,
        };
        // A tree with no length is as long as its soil needs (its size's least area over the width), at least its pit.
        let length_mm = form.length_mm.or(own.map(|g| g.length_mm)).unwrap_or_else(|| match size {
            Some(size) => {
                let soil = (size.min_pit_mm2() + i64::from(width_mm) - 1) / i64::from(width_mm.max(1));
                size.default_pit_mm().max(snap_up(i32::try_from(soil).unwrap_or(i32::MAX - SNAP_MM)))
            }
            None => form.kind.default_length_mm(),
        });
        Ok(Green { kind: form.kind, from_mm: at_mm, length_mm, offset_mm, width_mm, size })
    }

    /// Adds the element `form` asks for, with the defaults it leaves open (see `green_from_form`). One revision; the
    /// new element is the chosen one.
    pub fn add_green_from(&mut self, form: &GreenForm) -> Result<(), GreenRefusal> {
        self.green_gate(false)?;
        let g = self.green_from_form(form, None)?;
        self.add_green(g)
    }

    /// Changes element `i` as `form` says, a field it leaves open staying as it is (the kind never changes). One
    /// revision, whatever the number of fields; the element stays the chosen one.
    pub fn change_green(&mut self, i: usize, form: &GreenForm) -> Result<(), GreenRefusal> {
        let frame = self.green_gate(false)?;
        let Some(old) = self.green_at(i) else { return Ok(()) };
        let form = GreenForm { kind: old.kind, ..*form };
        let mut new = self.green_from_form(&form, Some(&old))?;
        new.from_mm = snap(new.from_mm);
        new.length_mm = snap(new.length_mm).max(SNAP_MM);
        new.width_mm = snap(new.width_mm).max(SNAP_MM);
        new.offset_mm = snap(new.offset_mm);
        self.green_fits(&new, Some(i), frame)?;
        if new == old {
            return Ok(());
        }
        self.green_choice.follow(old.place(), new.place());
        self.green_change(Self::green_label("rev-green-change", &new), frame, |green| green[i] = new);
        Ok(())
    }
}

/// The walkway left on one side of an element: the part of its host sidewalk on that side and the sidewalk pieces
/// that run on from it. `toward_left` says which side.
fn walkway_beside_mm(segs: &[Segment], host: usize, g: &Green, toward_left: bool) -> i32 {
    let (from, to) = fills(segs, host);
    let half = g.width_mm / 2;
    let own = if toward_left { (g.offset_mm - half - from).clamp(0, to - from) } else { (to - (g.offset_mm + g.width_mm - half)).clamp(0, to - from) };
    let beyond: Vec<&Segment> = if toward_left { segs[..host].iter().rev().collect() } else { segs[host + 1..].iter().collect() };
    let run: Vec<&Segment> = beyond.into_iter().take_while(|s| KINDS[s.kind].id == "sidewalk").collect();
    own + super::widest_walkway_mm(run.iter().copied())
}

impl Editor {
    /// The rows of the trees and planting, after the stops' (only when the street has some): one per rule, failing
    /// with the first element that breaks it, which it names in `zone`.
    ///
    /// An element is judged against the section of every zone it overlaps. Positions along are measured from the
    /// anchored end, and `ends` (node `a`, node `b`) is read from that end.
    pub(super) fn green_checks(&self) -> Vec<Check> {
        let content = &self.states[self.cursor].content;
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(i32::MAX);
        let rows = [
            ("green-host", "check-label-green-host", "check-detail-green-host-ok"),
            ("green-clear-path", "check-label-green-clear-path", "check-detail-green-clear-path-ok"),
            ("green-spacing", "check-label-green-spacing", "check-detail-green-spacing-ok"),
            ("green-soil", "check-label-green-soil", "check-detail-green-soil-ok"),
            ("green-sightline", "check-label-green-sightline", "check-detail-green-sightline-ok"),
            ("green-continuity", "check-label-green-continuity", "check-detail-green-continuity-ok"),
        ];
        check_rows(&rows, &self.green_failures(), |k| {
            let g = &content.green[k];
            let sections = content.sections_over(g.from_mm, g.to_mm(), length_mm);
            g.named(sections.first().copied().unwrap_or(&content.segments))
        })
    }

    /// Every rule each element breaks, element by element in order: `rule` indexes the six rows of `green_checks`.
    fn green_failures(&self) -> Vec<Failure> {
        let content = &self.states[self.cursor].content;
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(i32::MAX);
        let (near, far) = match content.from_end {
            ZoneEnd::First => (self.ends[0], self.ends[1]),
            ZoneEnd::Last => (self.ends[1], self.ends[0]),
        };
        let mut out: Vec<Failure> = Vec::new();
        for (k, g) in content.green.iter().enumerate() {
            let mut fail = |rule: usize, detail: Said| Failure::record(&mut out, k, rule, detail);
            let sections = content.sections_over(g.from_mm, g.to_mm(), length_mm);
            let mut host_kinds: Vec<&'static str> = Vec::new();
            for segs in &sections {
                let Some(h) = host_at(segs, g.offset_mm) else {
                    fail(0, Said::new("check-detail-green-host-off"));
                    continue;
                };
                let host = &segs[h];
                let host_id = KINDS[host.kind].id;
                host_kinds.push(host_id);
                if is_roadway(host.kind) {
                    fail(0, Said::new("check-detail-green-host-roadway").with("host", Arg::KindLower(host_id)));
                } else if g.width_mm > host.width_mm {
                    fail(
                        0,
                        Said::new("check-detail-green-host-wide")
                            .with("width", Arg::Length(g.width_mm))
                            .with("host", Arg::KindLower(host_id))
                            .with("have", Arg::Length(host.width_mm)),
                    );
                } else {
                    // Position, not only width: the whole span across has to lie within the piece.
                    let (from, to) = fills(segs, h);
                    let over2 = (2 * from - (2 * g.offset_mm - g.width_mm)).max(2 * g.offset_mm + g.width_mm - 2 * to);
                    if over2 > 0 {
                        fail(0, Said::new("check-detail-green-host-overhang").with("over", Arg::Length((over2 + 1) / 2)).with("host", Arg::KindLower(host_id)));
                    }
                }
                if host_id == "sidewalk" {
                    let have = walkway_beside_mm(segs, h, g, true).max(walkway_beside_mm(segs, h, g, false));
                    if have < CLEAR_PATH_MM {
                        fail(1, Said::new("check-detail-green-clear-path").with("have", Arg::Length(have)).with("need", Arg::Length(CLEAR_PATH_MM)));
                    }
                }
            }
            if let (GreenKind::Tree, Some(size)) = (g.kind, g.size) {
                for (j, o) in content.green.iter().enumerate() {
                    let Some(o_size) = o.size.filter(|_| o.kind == GreenKind::Tree && j != k) else { continue };
                    // The pair is recorded against the one nearer the anchored end.
                    if (g.centre_mm(), k) > (o.centre_mm(), j) {
                        continue;
                    }
                    let gap = centre_gap_mm(g, o);
                    let need = size.canopy_mm().max(o_size.canopy_mm());
                    if gap < need {
                        let osegs = content.sections_over(o.from_mm, o.to_mm(), length_mm);
                        let named = o.named(osegs.first().copied().unwrap_or(&content.segments));
                        let gsegs = content.sections_over(g.from_mm, g.to_mm(), length_mm);
                        let this = g.named(gsegs.first().copied().unwrap_or(&content.segments));
                        fail(
                            2,
                            Said::new("check-detail-green-spacing")
                                .with("tree", Arg::Said(Box::new(this)))
                                .with("gap", Arg::Length(gap))
                                .with("other", Arg::Said(Box::new(named)))
                                .with("need", Arg::Length(need)),
                        );
                    }
                }
                if i64::from(g.length_mm) * i64::from(g.width_mm) < size.min_pit_mm2() {
                    fail(
                        3,
                        Said::new("check-detail-green-soil")
                            .with("length", Arg::Length(g.length_mm))
                            .with("width", Arg::Length(g.width_mm))
                            .with("size", Arg::Text(format!("{size:?}"))),
                    );
                }
            }
            if matches!(g.kind, GreenKind::Tree | GreenKind::Bed) {
                let ends = [(near, g.from_mm), (far, length_mm.saturating_sub(g.to_mm()))];
                for (end, gap) in ends {
                    if end.junction && gap < SIGHTLINE_MM {
                        fail(4, Said::new("check-detail-green-sightline").with("gap", Arg::Length(gap.max(0))).with("need", Arg::Length(SIGHTLINE_MM)));
                    }
                }
            }
            if g.kind == GreenKind::Bioswale {
                if g.length_mm < BIOSWALE_MIN_MM {
                    fail(
                        5,
                        Said::new("check-detail-green-continuity-short").with("length", Arg::Length(g.length_mm)).with("min", Arg::Length(BIOSWALE_MIN_MM)),
                    );
                }
                if host_kinds.windows(2).any(|w| w[0] != w[1]) {
                    fail(5, Said::new("check-detail-green-continuity-host"));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::Side;
    use crate::shared::catalogue::kind_index;
    use crate::street::model::fixtures::zoned;
    use crate::street::model::{Editor, EndInfo, Segment, ZoneEnd, ZoneFrame};

    fn tree(from_mm: i32, offset_mm: i32) -> Green {
        Green { kind: GreenKind::Tree, from_mm, length_mm: 1_500, offset_mm, width_mm: 1_500, size: Some(TreeSize::M) }
    }

    fn bed(from_mm: i32, offset_mm: i32) -> Green {
        Green { kind: GreenKind::Bed, from_mm, length_mm: 4_000, offset_mm, width_mm: 2_000, size: None }
    }

    fn with_green(green: Vec<Green>) -> Street {
        Street { green, length_mm: Some(100_000), ..Street::sample(0, Side::Right) }
    }

    #[test]
    fn a_street_without_green_serialises_as_before() {
        let json = serde_json::to_string(&Street::sample(0, Side::Right)).unwrap();
        assert!(!json.contains("green"), "{json}");
    }

    #[test]
    fn green_round_trips() {
        let s = with_green(vec![tree(3_000, 1_000), bed(10_000, 5_000)]);
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"green\""));
        assert!(json.contains("\"kind\":\"tree\"") && json.contains("\"size\":\"M\""));
        let back: Street = serde_json::from_str(&json).unwrap();
        assert_eq!(back.green, s.green);
        assert_eq!(back.green[1].size, None);
        assert!(back.is_sound());
    }

    #[test]
    fn a_tree_without_a_size_is_not_sound() {
        let mut t = tree(3_000, 1_000);
        t.size = None;
        assert!(!with_green(vec![t]).is_sound());
        let mut b = bed(3_000, 1_000);
        b.size = Some(TreeSize::S);
        assert!(!with_green(vec![b]).is_sound());
        assert!(with_green(vec![tree(3_000, 1_000)]).is_sound());
    }

    #[test]
    fn an_element_without_a_recorded_length_is_not_sound() {
        let mut s = with_green(vec![tree(3_000, 1_000)]);
        s.length_mm = None;
        assert!(!s.is_sound());
    }

    #[test]
    fn reversing_keeps_each_element_where_it_stands() {
        let s = with_green(vec![tree(3_000, 1_000), bed(10_000, 5_000)]);
        let once = s.reversed();
        let total_mm = total(&s.segments);
        assert_eq!(once.from_end, ZoneEnd::Last);
        assert_eq!(once.green.iter().map(|g| (g.from_mm, g.length_mm)).collect::<Vec<_>>(), vec![(3_000, 1_500), (10_000, 4_000)]);
        assert_eq!(once.green.iter().map(|g| g.offset_mm).collect::<Vec<_>>(), vec![total_mm - 1_000, total_mm - 5_000]);
        assert_eq!(once.reversed(), s);
    }

    #[test]
    fn reversing_mirrors_by_the_section_under_the_element() {
        // The second zone has a section of its own that totals less than the street's row.
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 50_000], 100_000);
        let mut own: Vec<Segment> = s.segments.clone();
        let removed = own.remove(1).width_mm;
        s.zones[1].segments = Some(own);
        s.row_mm = total(&s.segments);
        assert!(removed > 0);
        s.green = vec![tree(10_000, 1_000), tree(60_000, 1_000)];
        let once = s.reversed();
        assert_eq!(once.green[0].offset_mm, total(&s.segments) - 1_000);
        assert_eq!(once.green[1].offset_mm, total(&s.segments) - removed - 1_000);
        assert_eq!(once.reversed(), s);
    }

    #[test]
    fn an_element_is_sound_only_within_the_section_under_it_and_stays_sound_reversed() {
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 50_000], 100_000);
        let mut own: Vec<Segment> = s.segments.clone();
        own.remove(1);
        s.zones[1].segments = Some(own);
        let short = total(s.zones[1].segments.as_deref().unwrap());
        assert!(short < s.row_mm);
        s.green = vec![tree(60_000, short + 1)];
        assert!(!s.is_sound(), "past the section under it, though within row_mm");
        s.green = vec![tree(60_000, short), tree(10_000, s.row_mm)];
        assert!(s.is_sound());
        assert!(s.reversed().is_sound());
    }

    #[test]
    fn at_end_drops_the_elements() {
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 50_000], 100_000);
        s.green = vec![tree(3_000, 1_000)];
        assert!(s.at_end(true).green.is_empty());
    }

    // ---- the editor's commands -------------------------------------------------------------------------------

    const LEN: i32 = 60_000;

    fn editor() -> Editor {
        let s = Street::sample(0, Side::Right);
        Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::First }))
    }

    /// A street of sidewalk 3000, travel 3300, median 1500, travel 3300, bike 1800, sidewalk 3000.
    fn medianed() -> Editor {
        let mut s = Street::sample(0, Side::Right);
        let kinds = [("sidewalk", 3_000), ("travel", 3_300), ("median", 1_500), ("travel", 3_300), ("bike", 1_800), ("sidewalk", 3_000)];
        s.segments = kinds.iter().enumerate().map(|(k, (id, w))| Segment::new(k as u32 + 1, kind_index(id).unwrap(), *w)).collect();
        s.next_uid = 7;
        s.row_mm = total(&s.segments);
        Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::First }))
    }

    fn pit(from_mm: i32, offset_mm: i32) -> Green {
        tree(from_mm, offset_mm)
    }

    #[test]
    fn a_tree_is_placed_on_the_sidewalk() {
        let mut e = editor();
        e.add_green(pit(10_049, 1_650)).unwrap();
        let s = e.snapshot();
        assert_eq!(s.green, vec![Green { from_mm: 10_000, offset_mm: 1_700, ..pit(10_000, 1_700) }]);
        assert_eq!((s.length_mm, s.from_end), (Some(LEN), ZoneEnd::First), "the street's length is written with the elements");
        assert_eq!(e.view().revisions.len(), 1);
        assert_eq!(e.selected_green(), Some(0), "the new element is the chosen one");
    }

    #[test]
    fn an_element_without_a_junction_end_is_refused_with_no_anchor() {
        let s = Street::sample(0, Side::Right);
        let mut e = Editor::from_street(&s, &s, 0);
        assert_eq!(e.add_green(pit(10_000, 1_650)), Err(GreenRefusal::NoAnchor));
        assert!(e.snapshot().green.is_empty());
    }

    #[test]
    fn an_element_on_a_travel_lane_is_refused_on_roadway() {
        let mut e = editor();
        assert_eq!(e.add_green(pit(10_000, 7_400)), Err(GreenRefusal::OnRoadway));
        assert!(e.snapshot().green.is_empty());
    }

    #[test]
    fn a_bike_lane_is_roadway_too() {
        let mut e = medianed();
        // The bike lane lies at 11 100 to 12 900 mm.
        assert_eq!(e.add_green(pit(10_000, 12_000)), Err(GreenRefusal::OnRoadway));
        assert_eq!(e.add_green(pit(10_000, 14_000)), Ok(()));
    }

    #[test]
    fn an_element_past_the_end_is_refused_off_street() {
        let mut e = editor();
        assert_eq!(e.add_green(pit(LEN - 1_000, 1_650)), Err(GreenRefusal::OffStreet), "past the end along");
        assert_eq!(e.add_green(pit(-1_000, 1_650)), Err(GreenRefusal::OffStreet), "before the start");
        assert_eq!(e.add_green(pit(10_000, 300)), Err(GreenRefusal::OffStreet), "past the left edge across");
        assert_eq!(e.add_green(pit(10_000, 17_800)), Err(GreenRefusal::OffStreet), "past the right edge across");
        assert!(e.snapshot().green.is_empty());
    }

    #[test]
    fn elements_side_by_side_across_do_not_overlap() {
        let mut e = medianed();
        // Two beds along the same stretch, one on each sidewalk.
        e.add_green(bed(10_000, 1_500)).unwrap();
        e.add_green(bed(10_000, 14_000)).unwrap();
        assert_eq!(e.snapshot().green.len(), 2);
    }

    #[test]
    fn overlapping_rectangles_are_refused() {
        let mut e = editor();
        e.add_green(bed(10_000, 1_700)).unwrap();
        assert_eq!(e.add_green(pit(12_000, 1_700)), Err(GreenRefusal::Overlap), "inside along and across");
        assert_eq!(e.add_green(pit(13_000, 2_200)), Err(GreenRefusal::Overlap), "the bed holds 13 000 along and 1 450 to 2 950 across meets its 700 to 2 700");
        assert_eq!(e.snapshot().green.len(), 1);
    }

    #[test]
    fn touching_elements_are_allowed() {
        let mut e = editor();
        e.add_green(bed(10_000, 1_700)).unwrap();
        e.add_green(pit(14_000, 1_700)).unwrap();
        e.add_green(pit(8_500, 1_700)).unwrap();
        // The bed is 2 000 wide across, from 700 to 2 700: this one's edge meets it there.
        e.add_green(Green { width_mm: 600, ..pit(10_000, 3_000) }).unwrap();
        assert_eq!(e.snapshot().green.len(), 4);
    }

    #[test]
    fn moving_across_onto_the_median_keeps_the_size() {
        let mut e = medianed();
        // The median is 6 300 to 7 800; a pit 1 400 wide fits it on the grid, one 1 500 wide cannot.
        e.add_green(Green { size: Some(TreeSize::L), width_mm: 1_400, ..pit(10_000, 1_500) }).unwrap();
        e.move_green_across(0, 6_300 + 700).unwrap();
        let g = e.snapshot().green[0];
        assert_eq!((g.offset_mm, g.size, g.width_mm, g.from_mm), (7_000, Some(TreeSize::L), 1_400, 10_000));
        assert_eq!(e.move_green_across(0, 7_200), Err(GreenRefusal::OnRoadway), "its edge would reach the lane");
        assert_eq!(e.move_green_across(0, 4_000), Err(GreenRefusal::OnRoadway), "a travel lane is no place for it");
        assert_eq!(e.snapshot().green[0].offset_mm, 7_000);
    }

    #[test]
    fn undo_and_redo_keep_the_moved_element_chosen() {
        let mut e = editor();
        e.add_green(pit(10_000, 1_700)).unwrap();
        e.add_green(pit(30_000, 1_700)).unwrap();
        assert_eq!(e.selected_green(), Some(1));
        e.move_green(1, 5_000).unwrap();
        assert_eq!(e.snapshot().green.iter().map(|g| g.from_mm).collect::<Vec<_>>(), vec![5_000, 10_000], "sorted");
        assert_eq!(e.selected_green(), Some(0), "the choice follows the moved element past the other");
        assert!(e.undo());
        assert_eq!(e.snapshot().green.iter().map(|g| g.from_mm).collect::<Vec<_>>(), vec![10_000, 30_000]);
        assert_eq!(e.selected_green(), Some(1));
        assert!(e.redo());
        assert_eq!(e.selected_green(), Some(0));
    }

    #[test]
    fn a_cancelled_drag_puts_the_choice_back() {
        let mut e = editor();
        e.add_green(pit(10_000, 1_700)).unwrap();
        e.add_green(pit(30_000, 1_700)).unwrap();
        assert!(e.begin_green_drag());
        e.move_green(1, 5_000).unwrap();
        e.move_green(0, 4_000).unwrap();
        e.cancel_gesture();
        assert_eq!(e.snapshot().green.iter().map(|g| g.from_mm).collect::<Vec<_>>(), vec![10_000, 30_000]);
        assert_eq!(e.selected_green(), Some(1));
    }

    #[test]
    fn a_drag_refuses_another_add() {
        let mut e = editor();
        e.add_green(pit(10_000, 1_700)).unwrap();
        assert!(e.begin_green_drag());
        assert!(!e.begin_green_drag(), "a drag is under way");
        e.move_green(0, 12_000).unwrap();
        assert_eq!(e.add_green(pit(30_000, 1_700)), Err(GreenRefusal::Dragging));
        assert_eq!(e.remove_green(0), Err(GreenRefusal::Dragging));
        assert!(e.end_gesture());
        assert_eq!(e.view().revisions.len(), 2, "the drag is one revision");
    }

    #[test]
    fn green_edits_and_other_drags_exclude_each_other() {
        use crate::street::model::curb::{CurbRefusal, Use};
        let mut e = editor();
        e.begin_gesture();
        assert_eq!(e.add_green(pit(10_000, 1_700)), Err(GreenRefusal::Dragging), "a zone drag is under way");
        e.cancel_gesture();
        assert!(e.begin_curb_drag());
        assert_eq!(e.add_green(pit(10_000, 1_700)), Err(GreenRefusal::Dragging), "a curb drag is under way");
        e.cancel_gesture();
        assert!(e.begin_stop_drag());
        assert_eq!(e.add_green(pit(10_000, 1_700)), Err(GreenRefusal::Dragging), "a stop drag is under way");
        e.cancel_gesture();
        assert!(e.snapshot().green.is_empty());
        assert!(e.begin_green_drag());
        assert_eq!(e.add_curb(Side::Right, 10_000, 6_000, Use::Parking), Err(CurbRefusal::Dragging), "a green drag is under way");
        e.cancel_gesture();
    }

    #[test]
    fn a_tree_without_a_size_is_medium_and_a_bed_has_none() {
        let mut e = editor();
        e.add_green(Green { size: None, ..pit(10_000, 1_700) }).unwrap();
        e.add_green(Green { size: Some(TreeSize::L), ..bed(20_000, 1_700) }).unwrap();
        let green = e.snapshot().green;
        assert_eq!(green[0].size, Some(TreeSize::M));
        assert_eq!(green[1].size, None, "a bed's size is dropped");
    }

    #[test]
    fn a_street_drawn_elsewhere_is_refused() {
        let s = Street::sample(0, Side::Right);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::First }));
        e.add_green(pit(10_000, 1_700)).unwrap();
        let drawn = e.snapshot();
        let mut e = Editor::from_street(&drawn, &drawn, 0).with_frame(Some(ZoneFrame { length_mm: 30_000, from_end: ZoneEnd::First }));
        assert_eq!(e.add_green(pit(20_000, 1_700)), Err(GreenRefusal::Elsewhere));
    }

    #[test]
    fn resizing_sizing_and_removing() {
        let mut e = editor();
        e.add_green(bed(10_000, 1_700)).unwrap();
        e.add_green(pit(16_000, 1_700)).unwrap();
        e.resize_green(0, 6_049, 2_000).unwrap();
        assert_eq!(e.snapshot().green[0].length_mm, 6_000);
        assert_eq!(e.resize_green(0, 6_100, 2_000), Err(GreenRefusal::Overlap), "into the pit at 16 000");
        assert_eq!(e.resize_green(0, 100_000, 2_000), Err(GreenRefusal::OffStreet));
        e.set_green_size(1, TreeSize::S).unwrap();
        assert_eq!(e.snapshot().green[1].size, Some(TreeSize::S));
        e.set_green_size(0, TreeSize::L).unwrap();
        assert_eq!(e.snapshot().green[0].size, None, "only a tree has a size");
        e.select_green(0);
        e.remove_green(0).unwrap();
        assert_eq!(e.snapshot().green.len(), 1);
        assert_eq!(e.selected_green(), None, "the removed element is not chosen any more");
    }

    #[test]
    fn the_refusals_have_words_in_both_languages() {
        for r in
            [GreenRefusal::NoAnchor, GreenRefusal::Elsewhere, GreenRefusal::Dragging, GreenRefusal::OffStreet, GreenRefusal::OnRoadway, GreenRefusal::Overlap]
        {
            assert!(!r.said().key.is_empty());
        }
        assert_eq!(GreenRefusal::NoAnchor.said().key, "zone-refused-no-anchor");
        assert_eq!(GreenRefusal::OnRoadway.said().key, "green-refused-on-roadway");
    }

    #[test]
    fn host_at_finds_the_piece_across() {
        let e = medianed();
        let segs = e.snapshot().segments;
        assert_eq!(host_at(&segs, 0), Some(0));
        assert_eq!(host_at(&segs, 2_999), Some(0));
        assert_eq!(host_at(&segs, 3_000), Some(1));
        assert_eq!(host_at(&segs, 7_000), Some(2));
        assert_eq!(host_at(&segs, total(&segs)), Some(5), "the far edge belongs to the last piece");
        assert_eq!(host_at(&segs, total(&segs) + 1), None);
        assert_eq!(host_at(&segs, -1), None);
        assert_eq!(fills(&segs, 2), (6_300, 7_800));
    }
    // ---- the span against the roadway and the host (final review C1) -------------------------------------------

    /// Whether `g` lies wholly within the piece `host` of `segs`.
    fn within_host(segs: &[Segment], host: usize, g: &Green) -> bool {
        let (from, to) = fills(segs, host);
        2 * g.offset_mm - g.width_mm >= 2 * from && 2 * g.offset_mm + g.width_mm <= 2 * to
    }

    #[test]
    fn a_bed_that_reaches_into_a_travel_lane_is_refused_though_its_centre_is_on_the_sidewalk() {
        // The review's probe: a 2 800 wide bed centred at 3 200 spans 1 800 to 4 600, 1.3 m into the lane.
        let mut e = editor();
        let overhang = Green { kind: GreenKind::Bed, from_mm: 10_000, length_mm: 4_000, offset_mm: 3_200, width_mm: 2_800, size: None };
        assert_eq!(e.add_green(overhang), Err(GreenRefusal::OnRoadway));
        assert!(e.snapshot().green.is_empty());
        // Moved across, a bed that fits is dragged into the lane by its edge.
        e.add_green(Green { offset_mm: 1_700, width_mm: 2_800, ..overhang }).unwrap();
        assert_eq!(e.move_green_across(0, 2_000), Err(GreenRefusal::OnRoadway));
        assert_eq!(e.resize_green(0, 4_000, 4_000), Err(GreenRefusal::OffStreet), "past the section's edge is still off the street");
    }

    #[test]
    fn an_element_is_judged_against_the_roadway_in_every_section_it_overlaps() {
        // Zone 2's sidewalk is 500 mm narrower: a tree at 2 500 across (1 750 to 3 250) fits zone 1's and reaches the lane in zone 2.
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 50_000], 100_000);
        let mut own = s.segments.clone();
        own[0].width_mm -= 500;
        s.zones[1].segments = Some(own);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 100_000, from_end: ZoneEnd::First }));
        let across = Green { length_mm: 4_000, ..small(48_000, 2_500) };
        assert_eq!(e.add_green(across), Err(GreenRefusal::OnRoadway));
        assert_eq!(e.add_green(Green { from_mm: 40_000, ..across }), Ok(()), "wholly in zone 1");
    }

    #[test]
    fn green_host_fails_when_the_span_leaves_its_piece_in_any_zone() {
        // Loaded as saved: a bed whose centre is on the sidewalk and whose edge is not.
        let overhang = Green { kind: GreenKind::Bed, from_mm: 10_000, length_mm: 4_000, offset_mm: 3_200, width_mm: 2_800, size: None };
        let s = Street { green: vec![overhang], length_mm: Some(LEN), ..Street::sample(0, Side::Right) };
        let e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::First }));
        let c = row(&e, "green-host");
        assert!(!c.ok, "narrower than the piece, yet 1.3 m past it");
        assert_eq!(c.detail.key, "check-detail-green-host-overhang");
        // The same bed inside the piece passes.
        let inside = Green { offset_mm: 1_900, ..overhang };
        let s = Street { green: vec![inside], ..s };
        let e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::First }));
        assert!(passes(&e, "green-host"));
        // A tree inside the first zone's piece that the second zone's narrower piece does not hold fails there.
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 50_000], 100_000);
        let mut own = s.segments.clone();
        own[0].width_mm -= 500;
        s.zones[1].segments = Some(own);
        s.green = vec![Green { length_mm: 4_000, ..small(48_000, 2_500) }];
        let e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 100_000, from_end: ZoneEnd::First }));
        assert!(!passes(&e, "green-host"), "its edge at 3 300 leaves the 2 800 piece of zone 2");
    }

    #[test]
    fn a_default_element_lies_wholly_within_its_host() {
        // A small tree on the 3.3 m sidewalk used to land 50 mm into the lane.
        let mut e = editor();
        e.add_green_from(&GreenForm { size: Some(TreeSize::S), at_mm: Some(10_000), host: Some(0), ..GreenForm::new(GreenKind::Tree) }).unwrap();
        let segs = e.snapshot().segments;
        let g = e.snapshot().green[0];
        assert!(within_host(&segs, 0, &g), "{g:?}");
        assert!(passes(&e, "green-host"));
        // The Laurier sidewalks, 1.5 m with 150 mm buffers, for every kind and size.
        let laurier: Vec<Segment> = [
            ("sidewalk", 1_500),
            ("buffer", 150),
            ("bike", 1_500),
            ("travel", 3_000),
            ("travel", 3_000),
            ("bike", 1_500),
            ("buffer", 150),
            ("sidewalk", 1_500),
        ]
        .iter()
        .enumerate()
        .map(|(k, (id, w))| Segment::new(k as u32 + 1, kind_index(id).unwrap(), *w))
        .collect();
        let mut s = Street::sample(0, Side::Right);
        s.segments = laurier;
        s.next_uid = 9;
        s.row_mm = total(&s.segments);
        for host in [0, 7] {
            for (kind, size) in [
                (GreenKind::Tree, Some(TreeSize::S)),
                (GreenKind::Tree, Some(TreeSize::M)),
                (GreenKind::Tree, Some(TreeSize::L)),
                (GreenKind::Bed, None),
                (GreenKind::RainGarden, None),
                (GreenKind::Bioswale, None),
            ] {
                let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 100_000, from_end: ZoneEnd::First }));
                e.add_green_from(&GreenForm { size, at_mm: Some(20_000), host: Some(host), ..GreenForm::new(kind) }).unwrap();
                let g = e.snapshot().green[0];
                assert!(within_host(&e.snapshot().segments, host, &g), "{kind:?} {size:?} on {host}: {g:?}");
                assert_eq!(g.width_mm % 100, 0, "{g:?}");
                assert_eq!(g.offset_mm % 100, 0, "{g:?}");
            }
        }
    }

    // ---- the checks ------------------------------------------------------------------------------------------

    fn small(from_mm: i32, offset_mm: i32) -> Green {
        Green { size: Some(TreeSize::S), ..tree(from_mm, offset_mm) }
    }

    fn row(e: &Editor, id: &str) -> crate::street::model::Check {
        e.view().checks.into_iter().find(|c| c.id == id).unwrap_or_else(|| panic!("no check {id}"))
    }

    fn passes(e: &Editor, id: &str) -> bool {
        row(e, id).ok
    }

    fn green_ids(e: &Editor) -> Vec<&'static str> {
        e.view().checks.iter().map(|c| c.id).filter(|i| i.starts_with("green-")).collect()
    }

    fn junction_at(first: bool, last: bool) -> [EndInfo; 2] {
        [EndInfo { junction: first, ..Default::default() }, EndInfo { junction: last, ..Default::default() }]
    }

    /// A 100 000 mm street with zones at 0 and 50 000; zone 2 has a section of its own, the first piece being `kind`.
    fn two_zone_editor(kind: &str, green: Vec<Green>) -> Editor {
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 50_000], 100_000);
        let mut own = s.segments.clone();
        own[0].kind = kind_index(kind).unwrap();
        s.zones[1].segments = Some(own);
        s.green = green;
        Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 100_000, from_end: ZoneEnd::First }))
    }

    #[test]
    fn a_street_without_green_has_the_checks_it_had() {
        let mut e = editor();
        let before: Vec<&str> = e.view().checks.iter().map(|c| c.id).collect();
        assert!(before.iter().all(|i| !i.starts_with("green-")));
        e.add_green(small(10_000, 1_700)).unwrap();
        assert_eq!(green_ids(&e), ["green-host", "green-clear-path", "green-spacing", "green-soil", "green-sightline", "green-continuity"]);
        e.remove_green(0).unwrap();
        let after: Vec<&str> = e.view().checks.iter().map(|c| c.id).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn a_street_with_a_sound_tree_passes_every_green_row() {
        let mut e = editor().with_ends(junction_at(true, true));
        e.add_green(Green { width_mm: 1_400, length_mm: 1_700, ..small(12_000, 2_600) }).unwrap();
        for id in green_ids(&e) {
            assert!(passes(&e, id), "{id}");
        }
    }

    #[test]
    fn green_host_fails_for_an_element_wider_than_its_piece() {
        let mut e = editor();
        e.add_green(Green { width_mm: 3_000, ..bed(10_000, 1_600) }).unwrap();
        assert!(passes(&e, "green-host"));
        // Placement refuses a wider one, so it is read as saved.
        let s = Street { green: vec![Green { width_mm: 4_000, ..bed(10_000, 1_650) }], length_mm: Some(LEN), ..Street::sample(0, Side::Right) };
        let e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::First }));
        let c = row(&e, "green-host");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-green-host-wide");
        assert_eq!(c.zone.unwrap().key, "green-instance");
    }

    #[test]
    fn a_zone_change_under_an_element_fails_green_host_and_keeps_it() {
        let tree_across = small(49_500, 1_650);
        let e = two_zone_editor("sidewalk", vec![tree_across]);
        assert!(passes(&e, "green-host"), "the same kind of piece under it in both zones");
        let e = two_zone_editor("travel", vec![tree_across]);
        let c = row(&e, "green-host");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-green-host-roadway");
        assert_eq!(e.snapshot().green, vec![tree_across], "the element stays");
    }

    #[test]
    fn green_clear_path_wants_the_walkway_left_beside_the_element() {
        // The sidewalk is 3 300 mm. A pit 1 400 wide centred at 2 500 leaves exactly 1 800 on its left.
        let mut e = editor();
        e.add_green(Green { width_mm: 1_400, ..small(10_000, 2_500) }).unwrap();
        assert!(passes(&e, "green-clear-path"), "exactly the clear path");
        e.move_green_across(0, 2_400).unwrap();
        let c = row(&e, "green-clear-path");
        assert!(!c.ok, "1 700 and 200 are left");
        assert_eq!(c.detail.key, "check-detail-green-clear-path");
        // A pit in the middle of the sidewalk leaves less than that on either side.
        let mut e = editor();
        e.add_green(small(10_000, 1_650)).unwrap();
        assert!(!passes(&e, "green-clear-path"), "950 and 850 are left");
    }

    #[test]
    fn green_clear_path_counts_the_sidewalk_that_runs_on() {
        let mut s = Street::sample(0, Side::Right);
        let kinds = [("sidewalk", 1_000), ("sidewalk", 3_000), ("travel", 3_300), ("travel", 3_300), ("sidewalk", 3_000)];
        s.segments = kinds.iter().enumerate().map(|(k, (id, w))| Segment::new(k as u32 + 1, kind_index(id).unwrap(), *w)).collect();
        s.next_uid = 6;
        s.row_mm = total(&s.segments);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::First }));
        // On the second sidewalk piece (1 000 to 4 000), at 3 200 across: 1 450 of it and the first piece are on its left.
        e.add_green(small(10_000, 3_200)).unwrap();
        assert!(passes(&e, "green-clear-path"));
    }

    #[test]
    fn spacing_uses_the_larger_size() {
        let mut e = editor();
        e.add_green(small(10_000, 1_700)).unwrap();
        e.add_green(small(18_000, 1_700)).unwrap();
        assert!(passes(&e, "green-spacing"), "8 000 apart, small crowns need 6 000");
        e.set_green_size(1, TreeSize::L).unwrap();
        let c = row(&e, "green-spacing");
        assert!(!c.ok, "a large crown needs 10 000");
        assert_eq!(c.detail.key, "check-detail-green-spacing");
        for who in ["tree", "other"] {
            assert_eq!(c.detail.args.iter().filter(|(n, _)| *n == who).count(), 1, "{who} is named in the detail");
        }
        assert!(c.zone.unwrap().args.iter().any(|(n, a)| *n == "at" && matches!(a, Arg::Length(10_000))), "the nearer the anchor is named first");
    }

    #[test]
    fn spacing_is_measured_between_centres_along_and_across() {
        let at = |at_mm, host| GreenForm { size: Some(TreeSize::M), at_mm: Some(at_mm), host: Some(host), ..GreenForm::new(GreenKind::Tree) };
        // The review's pair: a medium tree on each sidewalk, level with each other. The road lies between them.
        let mut e = editor();
        e.add_green_from(&at(20_000, 0)).unwrap();
        e.add_green_from(&at(20_000, 5)).unwrap();
        assert!(passes(&e, "green-spacing"), "14 m apart across, though level along");
        // Two on one sidewalk 5 m apart along are too close.
        e.add_green_from(&at(25_000, 0)).unwrap();
        let c = row(&e, "green-spacing");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-green-spacing");
    }

    #[test]
    fn the_gap_between_two_trees_is_the_distance_between_their_centres() {
        let a = Green { length_mm: 2_000, width_mm: 1_000, ..tree(10_000, 2_000) };
        let b = Green { length_mm: 2_000, width_mm: 1_000, ..tree(16_000, 8_000) };
        assert_eq!(centre_gap_mm(&a, &b), 8_485, "6 m along and 6 m across");
        assert_eq!(centre_gap_mm(&a, &Green { offset_mm: 2_000, ..b }), 6_000);
        assert_eq!(centre_gap_mm(&a, &a), 0);
    }

    #[test]
    fn soil_wants_the_pit_area_of_the_size() {
        let mut e = editor();
        e.add_green(small(10_000, 1_700)).unwrap();
        assert!(passes(&e, "green-soil"), "2.25 m2 for a small tree");
        e.set_green_size(0, TreeSize::M).unwrap();
        let c = row(&e, "green-soil");
        assert!(!c.ok, "3.5 m2 for a medium one");
        assert_eq!(c.detail.key, "check-detail-green-soil");
        e.resize_green(0, 2_400, 1_500).unwrap();
        assert!(passes(&e, "green-soil"));
    }

    #[test]
    fn sightline_keeps_planting_off_a_junction_end() {
        let mut e = editor().with_ends(junction_at(true, false));
        e.add_green(small(5_000, 1_700)).unwrap();
        let c = row(&e, "green-sightline");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-green-sightline");
        e.move_green(0, 10_000).unwrap();
        assert!(passes(&e, "green-sightline"), "exactly 10 000 from the end");
        let mut e = editor().with_ends(junction_at(false, true));
        e.add_green(bed(50_000, 1_700)).unwrap();
        assert!(!passes(&e, "green-sightline"), "a bed 6 000 from the far end");
        e.move_green(0, 46_000).unwrap();
        assert!(passes(&e, "green-sightline"));
    }

    #[test]
    fn sightline_reads_the_end_the_elements_are_measured_from() {
        let s = Street { green: vec![small(5_000, 1_700)], length_mm: Some(LEN), from_end: ZoneEnd::Last, ..Street::sample(0, Side::Right) };
        let at = |ends| Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::Last })).with_ends(ends);
        assert!(passes(&at(junction_at(true, false)), "green-sightline"), "the near end is node b");
        assert!(!passes(&at(junction_at(false, true)), "green-sightline"));
    }

    #[test]
    fn an_end_that_is_no_junction_has_no_sightline_rule() {
        let mut e = editor().with_ends(junction_at(false, false));
        e.add_green(small(0, 1_700)).unwrap();
        e.add_green(bed(LEN - 4_000, 1_700)).unwrap();
        assert!(passes(&e, "green-sightline"));
    }

    #[test]
    fn a_bioswale_must_be_long_enough() {
        let mut e = editor();
        e.add_green(Green { kind: GreenKind::Bioswale, from_mm: 20_000, length_mm: 10_000, offset_mm: 1_600, width_mm: 3_200, size: None }).unwrap();
        assert!(passes(&e, "green-continuity"), "exactly the minimum");
        e.resize_green(0, 8_000, 3_200).unwrap();
        let c = row(&e, "green-continuity");
        assert!(!c.ok);
        assert_eq!(c.detail.key, "check-detail-green-continuity-short");
    }

    #[test]
    fn a_bioswale_across_a_change_of_host_fails_continuity() {
        let swale = Green { kind: GreenKind::Bioswale, from_mm: 40_000, length_mm: 20_000, offset_mm: 1_650, width_mm: 3_300, size: None };
        let e = two_zone_editor("planting", vec![swale]);
        let c = row(&e, "green-continuity");
        assert!(!c.ok, "a sidewalk becomes planting under it");
        assert_eq!(c.detail.key, "check-detail-green-continuity-host");
        assert!(passes(&e, "green-host"));
        let same = two_zone_editor("sidewalk", vec![swale]);
        assert!(passes(&same, "green-continuity"));
    }

    #[test]
    fn a_green_element_is_named_by_its_kind_place_and_host() {
        let e = two_zone_editor("sidewalk", vec![small(12_000, 1_650)]);
        let g = e.snapshot().green[0];
        let said = g.named(&e.snapshot().segments);
        assert_eq!(said.key, "green-instance");
        let names: Vec<_> = said.args.iter().map(|(n, _)| *n).collect();
        assert_eq!(names, ["kind", "at", "across"]);
    }
    #[test]
    fn a_tree_added_with_no_pit_gets_one_big_enough_for_its_size() {
        for size in [TreeSize::S, TreeSize::M, TreeSize::L] {
            let side = size.default_pit_mm();
            assert!(i64::from(side) * i64::from(side) >= size.min_pit_mm2(), "{size:?}");
            let mut e = editor();
            e.add_green(Green { length_mm: 0, width_mm: 0, size: Some(size), ..tree(20_000, 1_700) }).unwrap();
            let g = e.snapshot().green[0];
            assert_eq!((g.length_mm, g.width_mm), (side, side), "{size:?}");
            assert!(passes(&e, "green-soil"), "{size:?}");
        }
        let mut e = editor();
        e.add_green(Green { length_mm: 0, width_mm: 0, size: None, ..tree(20_000, 1_700) }).unwrap();
        assert!(passes(&e, "green-soil"), "no size is medium");
    }

    #[test]
    fn a_tree_given_a_pit_keeps_it() {
        let mut e = editor();
        e.add_green(Green { size: Some(TreeSize::L), ..tree(20_000, 1_700) }).unwrap();
        let g = e.snapshot().green[0];
        assert_eq!((g.length_mm, g.width_mm), (1_500, 1_500));
    }

    #[test]
    fn an_element_whose_centre_is_off_the_section_is_named_without_a_host() {
        let e = editor();
        let segs = e.snapshot().segments;
        let g = small(12_000, total(&segs) + 1_000);
        let said = g.named(&segs);
        assert_eq!(said.key, "green-instance-off");
        assert_eq!(said.args.iter().map(|(n, _)| *n).collect::<Vec<_>>(), ["kind", "at"]);
    }
}
