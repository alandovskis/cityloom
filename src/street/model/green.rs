//! Street trees and green infrastructure: a tree pit, a planting bed, a rain garden or a bioswale, placed along a
//! street and across its section.
//!
//! Like the curbside uses (`curb.rs`) and the transit stops (`stop.rs`) they belong to the street, not to a
//! section: a position along is measured from the anchored end (`Street::from_end`), so splitting, merging or moving
//! zones leaves them where they are. Across, `offset_mm` is the element's centre from the section's left edge.
//! A street with none is written, drawn and checked exactly as before.

use serde::{Deserialize, Serialize};

use super::curb::CurbRefusal;
use super::{Arg, Editor, SNAP_MM, Said, Segment, Street, ZoneFrame, snap, total};
use crate::shared::catalogue::is_roadway;

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
    /// Its centre would lie on a piece of the roadway.
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

impl Green {
    fn place(&self) -> Place {
        (self.from_mm, self.offset_mm)
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
        if sections.iter().any(|segs| host_at(segs, candidate.offset_mm).is_some_and(|h| is_roadway(segs[h].kind))) {
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
    /// no size is a medium one; any other kind has none. One revision; the new element is the chosen one.
    pub fn add_green(&mut self, g: Green) -> Result<(), GreenRefusal> {
        let frame = self.green_gate(false)?;
        let size = if g.kind == GreenKind::Tree { g.size.or(Some(TreeSize::M)) } else { None };
        let g = Green {
            from_mm: snap(g.from_mm),
            length_mm: snap(g.length_mm).max(SNAP_MM),
            offset_mm: snap(g.offset_mm),
            width_mm: snap(g.width_mm).max(SNAP_MM),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::Side;
    use crate::shared::catalogue::kind_index;
    use crate::street::model::fixtures::zoned;
    use crate::street::model::{Editor, Segment, ZoneEnd, ZoneFrame};

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
        assert_eq!(e.add_green(pit(10_000, 13_500)), Ok(()));
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
        e.add_green(Green { size: Some(TreeSize::L), ..pit(10_000, 1_500) }).unwrap();
        e.move_green_across(0, 6_300 + 750).unwrap();
        let g = e.snapshot().green[0];
        assert_eq!((g.offset_mm, g.size, g.width_mm, g.from_mm), (7_100, Some(TreeSize::L), 1_500, 10_000));
        assert_eq!(e.move_green_across(0, 4_000), Err(GreenRefusal::OnRoadway), "a travel lane is no place for it");
        assert_eq!(e.snapshot().green[0].offset_mm, 7_100);
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
}
