//! The cross-section editing model. Pure Rust, no browser types, so it is
//! tested natively. All lengths are integer millimetres.

use serde::Serialize;

use crate::catalogue::{KINDS, Kind, Mode, SAMPLES, kind_index};

/// Widths snap to this step when dragged.
pub const SNAP_MM: i32 = 100;
/// Sidewalk on each edge is a sheet check, not a hard rule.
const ACCESS_LANE_MM: i32 = 3000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub uid: u32,
    pub kind: usize,
    pub width_mm: i32,
}

#[derive(Clone, Debug)]
struct State {
    label: String,
    segments: Vec<Segment>,
}

pub struct Editor {
    sample: usize,
    row_mm: i32,
    states: Vec<State>,
    cursor: usize,
    next_uid: u32,
    selected: Option<u32>,
    gesture: Option<Vec<Segment>>,
    pending_label: String,
}

fn snap(v: i32) -> i32 {
    ((v as f64 / SNAP_MM as f64).round() as i32) * SNAP_MM
}

fn total(segments: &[Segment]) -> i32 {
    segments.iter().map(|s| s.width_mm).sum()
}

fn letter(n: usize) -> String {
    // A, B, ... Z, AA, AB ...
    let mut n = n;
    let mut out = String::new();
    loop {
        out.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    out
}

impl Editor {
    pub fn new(sample: usize) -> Editor {
        let mut e = Editor {
            sample: 0,
            row_mm: 0,
            states: Vec::new(),
            cursor: 0,
            next_uid: 1,
            selected: None,
            gesture: None,
            pending_label: String::new(),
        };
        e.load_sample(sample);
        e
    }

    pub fn load_sample(&mut self, sample: usize) {
        let sample = sample.min(SAMPLES.len() - 1);
        let s = &SAMPLES[sample];
        self.sample = sample;
        self.row_mm = s.row_mm;
        self.next_uid = 1;
        let segments = s
            .segments
            .iter()
            .map(|(id, w)| {
                let uid = self.next_uid;
                self.next_uid += 1;
                Segment {
                    uid,
                    kind: kind_index(id).expect("sample uses catalogue kinds"),
                    width_mm: *w,
                }
            })
            .collect();
        self.states = vec![State {
            label: "Existing".into(),
            segments,
        }];
        self.cursor = 0;
        self.selected = None;
        self.gesture = None;
    }

    fn current(&self) -> &Vec<Segment> {
        &self.states[self.cursor].segments
    }

    fn current_mut(&mut self) -> &mut Vec<Segment> {
        &mut self.states[self.cursor].segments
    }

    /// Runs an edit against a copy, then either records it as a revision,
    /// or, inside a gesture, applies it without recording.
    fn edit<F>(&mut self, label: String, f: F) -> bool
    where
        F: FnOnce(&mut Vec<Segment>) -> bool,
    {
        if self.gesture.is_some() {
            let changed = f(self.current_mut());
            if changed {
                self.pending_label = label;
            }
            return changed;
        }
        let mut next = self.current().clone();
        if !f(&mut next) || next == *self.current() {
            return false;
        }
        self.push(label, next);
        true
    }

    fn push(&mut self, label: String, segments: Vec<Segment>) {
        self.states.truncate(self.cursor + 1);
        self.states.push(State { label, segments });
        self.cursor += 1;
    }

    // ---- gestures ------------------------------------------------------

    /// Starts a drag. Edits until `end_gesture` collapse into one revision.
    pub fn begin_gesture(&mut self) {
        if self.gesture.is_none() {
            self.gesture = Some(self.current().clone());
            self.pending_label.clear();
        }
    }

    pub fn end_gesture(&mut self) -> bool {
        let Some(baseline) = self.gesture.take() else {
            return false;
        };
        let now = self.current().clone();
        if now == baseline {
            return false;
        }
        // The live state was mutated in place; restore it, then record it.
        *self.current_mut() = baseline;
        let label = std::mem::take(&mut self.pending_label);
        self.push(label, now);
        true
    }

    pub fn cancel_gesture(&mut self) {
        if let Some(baseline) = self.gesture.take() {
            *self.current_mut() = baseline;
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
        let remaining = self.row_mm - total(self.current());
        let width = if remaining >= k.min_mm {
            (remaining / SNAP_MM * SNAP_MM).clamp(k.min_mm, k.default_mm)
        } else {
            k.default_mm
        };
        let uid = self.new_uid();
        let seg = Segment {
            uid,
            kind,
            width_mm: width,
        };
        self.edit(format!("Add {}", k.name.to_lowercase()), |segs| {
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
        let name = KINDS[self.current()[pos].kind].name.to_lowercase();
        let neighbour = if pos + 1 < self.current().len() {
            Some(self.current()[pos + 1].uid)
        } else if pos > 0 {
            Some(self.current()[pos - 1].uid)
        } else {
            None
        };
        let changed = self.edit(format!("Remove {name}"), |segs| {
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
        let name = KINDS[self.current()[pos].kind].name.to_lowercase();
        let changed = self.edit(format!("Move {name}"), |segs| {
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

    fn clamp_width(kind: &Kind, w: i32) -> i32 {
        w.clamp(kind.min_mm, kind.max_mm)
    }

    /// Sets a width in millimetres, rounded to 10 mm and kept in the kind's
    /// allowed range.
    pub fn set_width(&mut self, uid: u32, width_mm: i32) -> bool {
        let Some(pos) = self.current().iter().position(|s| s.uid == uid) else {
            return false;
        };
        let kind = &KINDS[self.current()[pos].kind];
        let w = Self::clamp_width(kind, ((width_mm as f64 / 10.0).round() as i32) * 10);
        let label = format!("Resize {}", kind.name.to_lowercase());
        self.edit(label, |segs| {
            segs[pos].width_mm = w;
            true
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
        let Some(base) = self.gesture.clone() else {
            return false;
        };
        if left_index + 1 >= base.len() {
            return false;
        }
        let (l, r) = (&base[left_index], &base[left_index + 1]);
        let (lk, rk) = (&KINDS[l.kind], &KINDS[r.kind]);
        let pair = l.width_mm + r.width_mm;
        // Left width limits from both segments' allowed ranges.
        let lo = lk.min_mm.max(pair - rk.max_mm);
        let hi = lk.max_mm.min(pair - rk.min_mm);
        if lo > hi {
            return false;
        }
        let left = snap(l.width_mm + delta_mm).clamp(lo, hi);
        let label = format!("Resize {} and {}", lk.name.to_lowercase(), rk.name.to_lowercase());
        let (lu, ru) = (l.uid, r.uid);
        self.apply_widths(label, &[(lu, left), (ru, pair - left)])
    }

    /// Resizes one segment from its gesture-start width, taking the space
    /// from, or giving it to, the unassigned width of the street.
    pub fn resize_edge(&mut self, uid: u32, delta_mm: i32) -> bool {
        let Some(base) = self.gesture.clone() else {
            return false;
        };
        let Some(s) = base.iter().find(|s| s.uid == uid) else {
            return false;
        };
        let kind = &KINDS[s.kind];
        let w = Self::clamp_width(kind, snap(s.width_mm + delta_mm));
        let label = format!("Resize {}", kind.name.to_lowercase());
        self.apply_widths(label, &[(uid, w)])
    }

    fn apply_widths(&mut self, label: String, widths: &[(u32, i32)]) -> bool {
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
        self.reselect();
        true
    }

    /// Returns the proposal to the existing street as a new revision.
    pub fn reset(&mut self) -> bool {
        self.cancel_gesture();
        let existing = self.states[0].segments.clone();
        if existing == *self.current() {
            return false;
        }
        self.push("Reset to existing".into(), existing);
        self.selected = None;
        true
    }

    fn reselect(&mut self) {
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
        let pos = self
            .selected
            .and_then(|u| segs.iter().position(|s| s.uid == u));
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
        let existing = &self.states[0].segments;
        View {
            name: SAMPLES[self.sample].name,
            sample: self.sample,
            row_mm: self.row_mm,
            total_mm,
            delta_mm: total_mm - self.row_mm,
            segments: self.seg_views(segs),
            existing: self.seg_views(existing),
            existing_total_mm: total(existing),
            selected: self.selected,
            outcomes: outcomes(segs, existing),
            checks: checks(segs, self.row_mm),
            revisions: self.states[1..=self.cursor]
                .iter()
                .enumerate()
                .map(|(i, s)| Revision {
                    letter: letter(i),
                    label: s.label.clone(),
                })
                .collect(),
            can_undo: self.cursor > 0,
            can_redo: self.cursor + 1 < self.states.len(),
            changed: segs != existing,
        }
    }

    fn seg_views(&self, segs: &[Segment]) -> Vec<SegView> {
        let mut x = 0;
        segs.iter()
            .map(|s| {
                let k = &KINDS[s.kind];
                let v = SegView {
                    uid: s.uid,
                    kind: s.kind,
                    width_mm: s.width_mm,
                    x_mm: x,
                    min_mm: k.min_mm,
                    max_mm: k.max_mm,
                };
                x += s.width_mm;
                v
            })
            .collect()
    }
}

// ---- outcomes and checks ------------------------------------------------

fn capacity(segs: &[Segment]) -> i32 {
    segs.iter()
        .map(|s| KINDS[s.kind].people_per_hour_per_m * s.width_mm / 1000)
        .sum()
}

fn shares(segs: &[Segment]) -> Vec<Share> {
    let t = total(segs).max(1);
    Mode::ALL
        .iter()
        .map(|m| {
            let mm: i32 = segs
                .iter()
                .filter(|s| KINDS[s.kind].mode == *m)
                .map(|s| s.width_mm)
                .sum();
            Share {
                mode: *m,
                label: m.label(),
                mm,
                pct: (mm as f64 * 100.0 / t as f64).round() as i32,
            }
        })
        .collect()
}

fn outcomes(segs: &[Segment], existing: &[Segment]) -> Outcomes {
    Outcomes {
        share: shares(segs),
        existing_share: shares(existing),
        capacity_pph: capacity(segs),
        existing_capacity_pph: capacity(existing),
    }
}

fn checks(segs: &[Segment], row_mm: i32) -> Vec<Check> {
    let total_mm = total(segs);
    let delta = total_mm - row_mm;
    let fits = delta <= 0;
    let edge_walk = |s: Option<&Segment>| s.is_some_and(|s| KINDS[s.kind].id == "sidewalk");
    let both_edges = edge_walk(segs.first()) && edge_walk(segs.last());
    let access = segs.iter().any(|s| {
        matches!(KINDS[s.kind].id, "travel" | "bus") && s.width_mm >= ACCESS_LANE_MM
    });
    vec![
        Check {
            id: "fits",
            ok: fits,
            amount_mm: delta.abs(),
            label: "Fits the right-of-way",
            detail: if fits {
                if delta == 0 {
                    "Exactly full".into()
                } else {
                    format!("{} mm unassigned", -delta)
                }
            } else {
                format!("{delta} mm over")
            },
        },
        Check {
            id: "edges",
            ok: both_edges,
            amount_mm: 0,
            label: "Sidewalk at each edge",
            detail: if both_edges {
                "Both edges".into()
            } else {
                "An edge has no sidewalk".into()
            },
        },
        Check {
            id: "access",
            ok: access,
            amount_mm: ACCESS_LANE_MM,
            label: "Emergency vehicle lane",
            detail: if access {
                "A lane of 3.0 m or more".into()
            } else {
                "No lane of 3.0 m or more".into()
            },
        },
    ]
}

// ---- serialised view ----------------------------------------------------

#[derive(Serialize)]
pub struct SegView {
    pub uid: u32,
    pub kind: usize,
    pub width_mm: i32,
    pub x_mm: i32,
    pub min_mm: i32,
    pub max_mm: i32,
}

#[derive(Serialize)]
pub struct Share {
    pub mode: Mode,
    pub label: &'static str,
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
    /// The length the detail speaks of, so the page can print it in its units.
    pub amount_mm: i32,
    pub label: &'static str,
    pub detail: String,
}

#[derive(Serialize)]
pub struct Revision {
    pub letter: String,
    pub label: String,
}

#[derive(Serialize)]
pub struct View {
    pub name: &'static str,
    pub sample: usize,
    pub row_mm: i32,
    pub total_mm: i32,
    pub delta_mm: i32,
    pub segments: Vec<SegView>,
    pub existing: Vec<SegView>,
    pub existing_total_mm: i32,
    pub selected: Option<u32>,
    pub outcomes: Outcomes,
    pub checks: Vec<Check>,
    pub revisions: Vec<Revision>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub changed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(e: &Editor) -> Vec<&'static str> {
        e.current().iter().map(|s| KINDS[s.kind].id).collect()
    }

    fn kind(id: &str) -> usize {
        kind_index(id).unwrap()
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
        assert_eq!(v.checks[0].detail, "1800 mm over");
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
    fn revision_letters_run_past_z() {
        assert_eq!(letter(0), "A");
        assert_eq!(letter(25), "Z");
        assert_eq!(letter(26), "AA");
        assert_eq!(letter(27), "AB");
    }

    #[test]
    fn loading_a_sample_clears_history() {
        let mut e = Editor::new(0);
        e.add(kind("bike"), 0);
        e.load_sample(2);
        assert!(!e.view().can_undo);
        assert_eq!(e.view().row_mm, 12000);
    }
}
