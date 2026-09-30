//! The cross-section editing model. Pure Rust, no browser types, so it is
//! tested natively. All lengths are integer millimetres.

use serde::Serialize;

use crate::catalogue::{CURBS, DEFAULT_CURB, DIRECTIONS, DirectionRule, REGIONS, Side, KINDS, MATERIALS, Mode, SAMPLES, kind_index};

/// Widths snap to this step when dragged.
pub const SNAP_MM: i32 = 100;
/// Sidewalk on each edge is a sheet check, not a hard rule.
const ACCESS_LANE_MM: i32 = 3000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub uid: u32,
    pub kind: usize,
    pub width_mm: i32,
    /// Index into `MATERIALS`; always one the kind allows.
    pub material: usize,
    /// Index into `CURBS`. `None` is a flush edge, and is always the value for
    /// a kind without a curb.
    pub curb: Option<usize>,
    /// Index into `DIRECTIONS`. Always `Some` where the kind requires a
    /// direction (a driving lane), and `None` for a kind without one. Where it
    /// is optional (a bike lane) `None` means two-way.
    pub direction: Option<usize>,
    /// Other types this piece takes at certain times of day. Outside every
    /// window it is the type above. Windows never overlap.
    pub variants: Vec<Variant>,
}

/// A different type a piece takes between two times of day.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub kind: usize,
    /// Index into `MATERIALS`; always one the kind allows.
    pub material: usize,
    /// Start of the window, in minutes after midnight, a multiple of 15.
    pub from_min: i32,
    /// End of the window (not included). Earlier than `from_min` means the
    /// window runs past midnight.
    pub to_min: i32,
}

/// Time of day is kept in quarter hours; a window is a set of those.
const SLOT_MIN: i32 = 15;
const SLOTS: i32 = 24 * 60 / SLOT_MIN;

fn window_mask(from_min: i32, to_min: i32) -> u128 {
    let (from, to) = (from_min / SLOT_MIN, to_min / SLOT_MIN);
    let range = |a: i32, b: i32| (a..b).fold(0u128, |m, i| m | (1 << i));
    if from < to { range(from, to) } else { range(from, SLOTS) | range(0, to) }
}

fn valid_window(from_min: i32, to_min: i32) -> bool {
    let ok = |m: i32| (0..24 * 60).contains(&m) && m % SLOT_MIN == 0;
    ok(from_min) && ok(to_min) && from_min != to_min
}

fn clock(min: i32) -> String {
    format!("{:02}:{:02}", min / 60, min % 60)
}

impl Segment {
    /// The variant in force at `time_min`, if any.
    fn variant_at(&self, time_min: i32) -> Option<usize> {
        let slot = 1u128 << (time_min.rem_euclid(24 * 60) / SLOT_MIN);
        self.variants.iter().position(|v| window_mask(v.from_min, v.to_min) & slot != 0)
    }

    fn kind_at(&self, time_min: i32) -> usize {
        self.variant_at(time_min).map_or(self.kind, |i| self.variants[i].kind)
    }

    /// The range of widths every type of this piece allows.
    fn bounds(&self) -> (i32, i32) {
        self.bounds_except(None)
    }

    /// Like `bounds`, leaving out the variant at `skip`.
    fn bounds_except(&self, skip: Option<usize>) -> (i32, i32) {
        let variants = self.variants.iter().enumerate().filter(|(i, _)| Some(*i) != skip).map(|(_, v)| v.kind);
        std::iter::once(self.kind)
            .chain(variants)
            .fold((0, i32::MAX), |(lo, hi), k| (lo.max(KINDS[k].min_mm), hi.min(KINDS[k].max_mm)))
    }

    /// The piece as it is at `time_min`: its type, surface and direction then,
    /// with no variants of its own.
    fn at(&self, time_min: i32) -> Segment {
        let vi = self.variant_at(time_min);
        let kind = vi.map_or(self.kind, |i| self.variants[i].kind);
        let k = &KINDS[kind];
        Segment {
            uid: self.uid,
            kind,
            width_mm: self.width_mm,
            material: vi.map_or(self.material, |i| self.variants[i].material),
            curb: if k.has_curb { self.curb } else { None },
            direction: match k.direction {
                DirectionRule::None => None,
                DirectionRule::Required => Some(self.direction.unwrap_or(0)),
                DirectionRule::Optional => self.direction,
            },
            variants: Vec::new(),
        }
    }

    /// Kinds this piece may take at other times: other roadway types that
    /// share some width with its own types. Empty for a piece that is not
    /// roadway. Taking one may move the width into the shared range.
    fn alt_kinds(&self) -> Vec<usize> {
        self.alt_kinds_except(None)
    }

    fn alt_kinds_except(&self, skip: Option<usize>) -> Vec<usize> {
        if !KINDS[self.kind].shares_road {
            return Vec::new();
        }
        let (lo, hi) = self.bounds_except(skip);
        (0..KINDS.len())
            .filter(|&k| {
                k != self.kind
                    && KINDS[k].shares_road
                    && lo.max(KINDS[k].min_mm) <= hi.min(KINDS[k].max_mm)
            })
            .collect()
    }
}

impl Segment {
    /// A piece with its kind's default surface and, where the kind has one,
    /// the default curb.
    pub fn new(uid: u32, kind: usize, width_mm: i32) -> Segment {
        let k = &KINDS[kind];
        Segment {
            uid,
            kind,
            width_mm,
            material: k.materials[0],
            curb: k.has_curb.then_some(DEFAULT_CURB),
            direction: (k.direction == DirectionRule::Required).then_some(0),
            variants: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
struct State {
    label: String,
    segments: Vec<Segment>,
}

pub struct Editor {
    sample: usize,
    /// Index into `REGIONS`. A setting of the sheet, not part of the history.
    region: usize,
    /// The time of day the sheet shows, in minutes. Also a setting, not history.
    time_min: i32,
    row_mm: i32,
    states: Vec<State>,
    cursor: usize,
    next_uid: u32,
    selected: Option<u32>,
    gesture: Option<Vec<Segment>>,
    pending_label: String,
}

fn required_direction(s: &Segment) -> bool {
    KINDS[s.kind].direction == DirectionRule::Required
}

/// Gives the driving lanes the directions a street in this region starts with:
/// with traffic on the right, lanes on the left half of the street come toward
/// the viewer and lanes on the right half go away; on the left it is the
/// other way round.
fn default_directions(segs: &mut [Segment], side: Side) {
    let lanes = segs.iter().filter(|s| required_direction(s)).count();
    for (i, s) in segs.iter_mut().filter(|s| required_direction(s)).enumerate() {
        let right_half = i * 2 >= lanes;
        s.direction = Some(usize::from(right_half == (side == Side::Left)));
    }
}

fn snap(v: i32) -> i32 {
    ((v as f64 / SNAP_MM as f64).round() as i32) * SNAP_MM
}

fn total(segments: &[Segment]) -> i32 {
    segments.iter().map(|s| s.width_mm).sum()
}

impl Editor {
    pub fn new(sample: usize) -> Editor {
        let mut e = Editor {
            sample: 0,
            region: 0,
            time_min: 12 * 60,
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
                Segment::new(
                    uid,
                    kind_index(id).expect("sample uses catalogue kinds"),
                    *w,
                )
            })
            .collect::<Vec<Segment>>();
        let mut segments = segments;
        // Driving lanes on the left half of the street run away, the rest toward.
        default_directions(&mut segments, REGIONS[self.region].drive_side);
        self.states = vec![State {
            label: "Street today".into(),
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
        let seg = Segment::new(uid, kind, width);
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
        let name = KINDS[self.current()[pos].kind_at(self.time_min)].name.to_lowercase();
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
        let name = KINDS[self.current()[pos].kind_at(self.time_min)].name.to_lowercase();
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

    /// Sets a width in millimetres, rounded to 10 mm and kept in the kind's
    /// allowed range.
    pub fn set_width(&mut self, uid: u32, width_mm: i32) -> bool {
        let Some(pos) = self.current().iter().position(|s| s.uid == uid) else {
            return false;
        };
        let seg = &self.current()[pos];
        let kind = &KINDS[seg.kind_at(self.time_min)];
        let (lo, hi) = seg.bounds();
        let w = (((width_mm as f64 / 10.0).round() as i32) * 10).clamp(lo, hi);
        let label = format!("Resize {}", kind.name.to_lowercase());
        self.edit(label, |segs| {
            segs[pos].width_mm = w;
            true
        })
    }

    /// Sets a piece's surface. Refused when the kind does not allow it.
    pub fn set_material(&mut self, uid: u32, material: usize) -> bool {
        let Some(pos) = self.current().iter().position(|s| s.uid == uid) else {
            return false;
        };
        let seg = &self.current()[pos];
        let active = seg.variant_at(self.time_min);
        let kind = &KINDS[seg.kind_at(self.time_min)];
        if !kind.materials.contains(&material) {
            return false;
        }
        let label = format!(
            "{} surface: {}",
            kind.name,
            MATERIALS[material].name.to_lowercase()
        );
        self.edit(label, |segs| {
            let target = match active {
                Some(vi) => &mut segs[pos].variants[vi].material,
                None => &mut segs[pos].material,
            };
            let changed = *target != material;
            *target = material;
            changed
        })
    }

    /// Sets a piece's curb; `None` is a flush edge. Refused when the kind has
    /// no curb or the curb is not in the table.
    pub fn set_curb(&mut self, uid: u32, curb: Option<usize>) -> bool {
        let Some(pos) = self.current().iter().position(|s| s.uid == uid) else {
            return false;
        };
        let kind = &KINDS[self.current()[pos].kind_at(self.time_min)];
        if !kind.has_curb || curb.is_some_and(|c| !kind.curbs.contains(&c)) {
            return false;
        }
        let what = curb.map_or("none".to_string(), |c| CURBS[c].name.to_lowercase());
        let label = format!("{} curb: {what}", kind.name);
        self.edit(label, |segs| {
            let changed = segs[pos].curb != curb;
            segs[pos].curb = curb;
            changed
        })
    }

    /// Sets which way a lane runs; `None` is two-way. Refused for a kind with
    /// no direction, for `None` where the kind requires one, and for an index
    /// outside `DIRECTIONS`.
    pub fn set_direction(&mut self, uid: u32, direction: Option<usize>) -> bool {
        let Some(pos) = self.current().iter().position(|s| s.uid == uid) else {
            return false;
        };
        let kind = &KINDS[self.current()[pos].kind_at(self.time_min)];
        let allowed = match kind.direction {
            DirectionRule::None => false,
            DirectionRule::Required => direction.is_some(),
            DirectionRule::Optional => true,
        };
        if !allowed || direction.is_some_and(|d| d >= DIRECTIONS.len()) {
            return false;
        }
        let what = direction.map_or("two-way".to_string(), |d| DIRECTIONS[d].name.to_lowercase());
        let label = format!("{} direction: {what}", kind.name);
        self.edit(label, |segs| {
            let changed = segs[pos].direction != direction;
            segs[pos].direction = direction;
            changed
        })
    }

    /// Sets the region, by index into `REGIONS`. The region itself is a
    /// setting and stays out of the history. When it changes which side of the
    /// road traffic keeps to, the street's lanes are mirrored to match: silently
    /// while the street is untouched (it is just laid out for that region), and
    /// as one undoable change once it has edits.
    pub fn set_region(&mut self, region: usize) -> bool {
        if region >= REGIONS.len() || region == self.region {
            return false;
        }
        let side = REGIONS[region].drive_side;
        let side_changed = side != REGIONS[self.region].drive_side;
        self.region = region;
        if !side_changed {
            return true;
        }
        if self.states.len() == 1 {
            default_directions(&mut self.states[0].segments, side);
        } else {
            let label = format!("Traffic keeps {}", if side == Side::Right { "right" } else { "left" });
            self.edit(label, |segs| {
                let mut changed = false;
                for s in segs.iter_mut() {
                    if let Some(d) = s.direction {
                        s.direction = Some(1 - d);
                        changed = true;
                    }
                }
                changed
            });
        }
        true
    }

    /// Sets the time of day the sheet shows, in minutes after midnight.
    /// Snapped to a quarter hour. A setting, so it stays out of the history.
    pub fn set_time(&mut self, time_min: i32) -> bool {
        let t = (time_min.clamp(0, 24 * 60 - 1) / SLOT_MIN) * SLOT_MIN;
        let changed = t != self.time_min;
        self.time_min = t;
        changed
    }

    fn seg_pos(&self, uid: u32) -> Option<usize> {
        self.current().iter().position(|s| s.uid == uid)
    }

    /// Gives a roadway piece a different type at certain times. It gets the
    /// first type that fits and the first free window of a few common ones
    /// (morning rush, evening rush, ...). Refused when the piece cannot change
    /// type or every window is taken.
    pub fn add_variant(&mut self, uid: u32) -> bool {
        let Some(pos) = self.seg_pos(uid) else {
            return false;
        };
        let seg = &self.current()[pos];
        let alts = seg.alt_kinds();
        let prefer = ["bus", "parking", "loading", "travel", "bike"];
        let Some(kind) = prefer
            .iter()
            .filter_map(|id| kind_index(id))
            .find(|k| alts.contains(k))
        else {
            return false;
        };
        let used = seg.variants.iter().fold(0u128, |m, v| m | window_mask(v.from_min, v.to_min));
        let windows = [(7 * 60, 10 * 60), (16 * 60, 19 * 60), (10 * 60, 16 * 60), (19 * 60, 7 * 60)];
        let Some(&(from_min, to_min)) = windows.iter().find(|(f, t)| window_mask(*f, *t) & used == 0) else {
            return false;
        };
        let name = KINDS[seg.kind].name;
        let label = format!(
            "{} is {} {}-{}",
            name,
            KINDS[kind].name.to_lowercase(),
            clock(from_min),
            clock(to_min)
        );
        self.edit(label, |segs| {
            let d = &mut segs[pos];
            d.variants.push(Variant { kind, material: KINDS[kind].materials[0], from_min, to_min });
            let (lo, hi) = d.bounds();
            d.width_mm = d.width_mm.clamp(lo, hi);
            true
        })
    }

    /// Changes the type of one of a piece's variants.
    pub fn set_variant_kind(&mut self, uid: u32, index: usize, kind: usize) -> bool {
        let Some(pos) = self.seg_pos(uid) else {
            return false;
        };
        let seg = &self.current()[pos];
        let Some(v) = seg.variants.get(index) else {
            return false;
        };
        if v.kind == kind || !seg.alt_kinds_except(Some(index)).contains(&kind) {
            return false;
        }
        let label = format!(
            "{} is {} {}-{}",
            KINDS[seg.kind].name,
            KINDS[kind].name.to_lowercase(),
            clock(v.from_min),
            clock(v.to_min)
        );
        self.edit(label, |segs| {
            let d = &mut segs[pos];
            d.variants[index].kind = kind;
            d.variants[index].material = KINDS[kind].materials[0];
            let (lo, hi) = d.bounds();
            d.width_mm = d.width_mm.clamp(lo, hi);
            true
        })
    }

    /// Moves the window of one of a piece's variants. Refused when a time is
    /// not on a quarter hour, the two are equal, or it overlaps another window.
    pub fn set_variant_time(&mut self, uid: u32, index: usize, from_min: i32, to_min: i32) -> bool {
        let Some(pos) = self.seg_pos(uid) else {
            return false;
        };
        let seg = &self.current()[pos];
        if index >= seg.variants.len() || !valid_window(from_min, to_min) {
            return false;
        }
        let mask = window_mask(from_min, to_min);
        let others = seg
            .variants
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .fold(0u128, |m, (_, v)| m | window_mask(v.from_min, v.to_min));
        if mask & others != 0 {
            return false;
        }
        let label = format!(
            "{} is {} {}-{}",
            KINDS[seg.kind].name,
            KINDS[seg.variants[index].kind].name.to_lowercase(),
            clock(from_min),
            clock(to_min)
        );
        self.edit(label, |segs| {
            let v = &mut segs[pos].variants[index];
            let changed = (v.from_min, v.to_min) != (from_min, to_min);
            v.from_min = from_min;
            v.to_min = to_min;
            changed
        })
    }

    pub fn remove_variant(&mut self, uid: u32, index: usize) -> bool {
        let Some(pos) = self.seg_pos(uid) else {
            return false;
        };
        let seg = &self.current()[pos];
        if index >= seg.variants.len() {
            return false;
        }
        let label = format!("Remove other times from {}", KINDS[seg.kind].name.to_lowercase());
        self.edit(label, |segs| {
            segs[pos].variants.remove(index);
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
        let (lk, rk) = (&KINDS[l.kind_at(self.time_min)], &KINDS[r.kind_at(self.time_min)]);
        let pair = l.width_mm + r.width_mm;
        // Left width limits from both segments' allowed ranges.
        let ((llo, lhi), (rlo, rhi)) = (l.bounds(), r.bounds());
        let lo = llo.max(pair - rhi);
        let hi = lhi.min(pair - rlo);
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
        let kind = &KINDS[s.kind_at(self.time_min)];
        let (lo, hi) = s.bounds();
        let w = snap(s.width_mm + delta_mm).clamp(lo, hi);
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
        // Everything that measures the street does so as it is at the shown time.
        let at = |v: &[Segment]| v.iter().map(|s| s.at(self.time_min)).collect::<Vec<_>>();
        let (now, existing_now) = (at(segs), at(existing));
        View {
            name: SAMPLES[self.sample].name,
            sample: self.sample,
            region: REGIONS[self.region].id,
            row_mm: self.row_mm,
            total_mm,
            delta_mm: total_mm - self.row_mm,
            time_min: self.time_min,
            segments: self.seg_views(segs),
            existing: self.seg_views(existing),
            existing_total_mm: total(existing),
            selected: self.selected,
            outcomes: outcomes(&now, &existing_now),
            checks: checks(&now, self.row_mm, REGIONS[self.region].drive_side),
            revisions: self.states[1..=self.cursor]
                .iter()
                .enumerate()
                .map(|(i, s)| Revision {
                    step: i + 1,
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
                let (min_mm, max_mm) = s.bounds();
                let n = s.at(self.time_min);
                let v = SegView {
                    uid: s.uid,
                    kind: n.kind,
                    base_kind: s.kind,
                    width_mm: s.width_mm,
                    x_mm: x,
                    min_mm,
                    max_mm,
                    material: MATERIALS[n.material].id,
                    curb: n.curb.map(|c| CURBS[c].id),
                    direction: n.direction.map(|d| DIRECTIONS[d].id),
                    variants: s
                        .variants
                        .iter()
                        .map(|v| VariantView {
                            kind: v.kind,
                            material: MATERIALS[v.material].id,
                            from_min: v.from_min,
                            to_min: v.to_min,
                        })
                        .collect(),
                    active_variant: s.variant_at(self.time_min),
                    alt_kinds: s.alt_kinds(),
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

fn checks(segs: &[Segment], row_mm: i32, side: Side) -> Vec<Check> {
    let total_mm = total(segs);
    let delta = total_mm - row_mm;
    let fits = delta <= 0;
    let edge_walk = |s: Option<&Segment>| s.is_some_and(|s| KINDS[s.kind].id == "sidewalk");
    let both_edges = edge_walk(segs.first()) && edge_walk(segs.last());
    let access = segs.iter().any(|s| {
        matches!(KINDS[s.kind].id, "travel" | "bus") && s.width_mm >= ACCESS_LANE_MM
    });
    // With traffic on the right, lanes coming toward the viewer (1) sit to the
    // left of lanes going away (0); with traffic on the left, the reverse. A
    // one-way street has nothing to conflict.
    let lanes: Vec<usize> = segs.iter().filter(|s| required_direction(s)).filter_map(|s| s.direction).collect();
    let (wrong_first, wrong_then) = if side == Side::Right { (0, 1) } else { (1, 0) };
    let keeps = lanes
        .iter()
        .position(|&d| d == wrong_first)
        .is_none_or(|i| !lanes[i..].contains(&wrong_then));
    let side_name = if side == Side::Right { "right" } else { "left" };
    vec![
        Check {
            id: "fits",
            ok: fits,
            amount_mm: delta.abs(),
            label: "Fits the street width",
            detail: if fits {
                if delta == 0 {
                    "Exactly full".into()
                } else {
                    format!("{} mm left to use", -delta)
                }
            } else {
                format!("{delta} mm over")
            },
        },
        Check {
            id: "edges",
            ok: both_edges,
            amount_mm: 0,
            label: "Sidewalk on both sides",
            detail: if both_edges {
                "Both sides".into()
            } else {
                "One side has no sidewalk".into()
            },
        },
        Check {
            id: "access",
            ok: access,
            amount_mm: ACCESS_LANE_MM,
            label: "Room for emergency vehicles",
            detail: if access {
                "A lane of 3.0 m or more".into()
            } else {
                "No lane of 3.0 m or more".into()
            },
        },
        Check {
            id: "side",
            ok: keeps,
            amount_mm: 0,
            label: if side == Side::Right { "Traffic keeps right" } else { "Traffic keeps left" },
            detail: if keeps {
                "Lanes run the way this region drives".into()
            } else {
                format!("A lane runs against traffic that keeps {side_name}")
            },
        },
    ]
}

// ---- serialised view ----------------------------------------------------

#[derive(Serialize)]
pub struct SegView {
    pub uid: u32,
    /// The type at the shown time.
    pub kind: usize,
    /// The type outside every variant's window.
    pub base_kind: usize,
    pub width_mm: i32,
    pub x_mm: i32,
    pub min_mm: i32,
    pub max_mm: i32,
    pub material: &'static str,
    pub curb: Option<&'static str>,
    pub direction: Option<&'static str>,
    pub variants: Vec<VariantView>,
    pub active_variant: Option<usize>,
    /// Types this piece may take at other times.
    pub alt_kinds: Vec<usize>,
}

#[derive(Serialize)]
pub struct VariantView {
    pub kind: usize,
    pub material: &'static str,
    pub from_min: i32,
    pub to_min: i32,
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
    pub step: usize,
    pub label: String,
}

#[derive(Serialize)]
pub struct View {
    pub name: &'static str,
    pub sample: usize,
    pub region: &'static str,
    /// The time of day shown, in minutes after midnight.
    pub time_min: i32,
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
    fn loading_a_sample_clears_history() {
        let mut e = Editor::new(0);
        e.add(kind("bike"), 0);
        e.load_sample(2);
        assert!(!e.view().can_undo);
        assert_eq!(e.view().row_mm, 12000);
    }

    #[test]
    fn new_pieces_get_their_kinds_defaults() {
        let e = Editor::new(0);
        for s in e.current() {
            let k = &KINDS[s.kind];
            assert_eq!(s.material, k.materials[0], "{}", k.id);
            assert_eq!(s.curb.is_some(), k.has_curb, "{}", k.id);
        }
        let mut e = Editor::new(0);
        let uid = e.add(kind("bike"), 0);
        let s = e.current().iter().find(|s| s.uid == uid).unwrap();
        assert_eq!(s.curb, Some(DEFAULT_CURB));
    }

    #[test]
    fn every_kinds_materials_are_valid_and_start_with_a_default() {
        for k in KINDS.iter() {
            assert!(!k.materials.is_empty(), "{}", k.id);
            assert!(k.materials.iter().all(|&m| m < MATERIALS.len()), "{}", k.id);
        }
        assert!(DEFAULT_CURB < CURBS.len());
    }

    #[test]
    fn set_material_accepts_allowed_and_refuses_others() {
        let mut e = Editor::new(0);
        let parking = e.current()[1].uid;
        let permeable = MATERIALS.iter().position(|m| m.id == "permeable").unwrap();
        let grass = MATERIALS.iter().position(|m| m.id == "grass").unwrap();
        assert!(!e.set_material(parking, grass)); // not a parking surface
        assert!(!e.view().can_undo);
        assert!(e.set_material(parking, permeable));
        assert_eq!(e.view().segments[1].material, "permeable");
        assert_eq!(e.view().revisions[0].label, "Parking surface: permeable paving");
        assert!(!e.set_material(parking, permeable)); // unchanged
        assert!(!e.set_material(9999, permeable)); // unknown piece
    }

    #[test]
    fn material_changes_undo_and_redo() {
        let mut e = Editor::new(0);
        let lane = e.current()[2].uid;
        assert!(e.set_material(lane, 1)); // concrete
        assert!(e.undo());
        assert_eq!(e.view().segments[2].material, "asphalt");
        assert!(e.redo());
        assert_eq!(e.view().segments[2].material, "concrete");
    }

    #[test]
    fn driving_lanes_must_have_a_direction_and_bike_lanes_may() {
        let mut e = Editor::new(0);
        let dirs: Vec<_> = e.view().segments.iter().filter(|s| s.kind == kind_index("travel").unwrap()).map(|s| s.direction).collect();
        assert_eq!(dirs, [Some("toward"), Some("away")]); // traffic keeps right
        let lane = e.current().iter().find(|s| s.kind == kind_index("travel").unwrap()).unwrap().uid;
        assert!(!e.set_direction(lane, None)); // a driving lane cannot be undirected
        assert!(!e.set_direction(lane, Some(9)));
        assert!(!e.set_direction(lane, Some(1))); // unchanged
        assert!(e.set_direction(lane, Some(0)));
        assert_eq!(e.view().revisions[0].label, "Driving lane direction: away from you");
        assert!(e.undo());
        let walk = e.current()[0].uid;
        assert!(!e.set_direction(walk, Some(0))); // sidewalks have none
        let bike = e.add(kind_index("bike").unwrap(), 1);
        assert_eq!(e.view().segments[1].direction, None);
        assert!(e.set_direction(bike, Some(0)));
        assert!(e.set_direction(bike, None));
        assert_eq!(e.view().revisions.last().unwrap().label, "Bike lane direction: two-way");
        assert!(e.undo());
        assert_eq!(e.view().segments[1].direction, Some("away"));
    }

    #[test]
    fn a_region_sets_which_side_traffic_keeps_to() {
        let dirs = |e: &Editor| -> Vec<_> {
            e.view().segments.iter().filter(|s| s.direction.is_some() && s.kind == kind("travel")).map(|s| s.direction).collect()
        };
        let mut e = Editor::new(0);
        assert_eq!(e.view().region, "canada");
        assert!(e.view().checks.iter().find(|c| c.id == "side").unwrap().ok);
        let uk = REGIONS.iter().position(|r| r.id == "united-kingdom").unwrap();
        assert!(!e.set_region(99));
        assert!(!e.set_region(0)); // unchanged
        // an untouched street is laid out again for the new side
        assert!(e.set_region(uk));
        assert_eq!(dirs(&e), [Some("away"), Some("toward")]);
        assert!(e.view().checks.iter().find(|c| c.id == "side").unwrap().ok);
        assert!(!e.view().can_undo);
        // once edited, a change of side mirrors the lanes as one undoable change
        e.add(kind("bike"), 0);
        assert!(e.set_region(0));
        assert_eq!(dirs(&e), [Some("toward"), Some("away")]);
        assert_eq!(e.view().revisions.last().unwrap().label, "Traffic keeps right");
        assert!(e.view().checks.iter().find(|c| c.id == "side").unwrap().ok);
        // a region on the same side leaves the lanes alone
        let us = REGIONS.iter().position(|r| r.id == "united-states").unwrap();
        let revs = e.view().revisions.len();
        assert!(e.set_region(us));
        assert_eq!(e.view().revisions.len(), revs);
        assert_eq!(dirs(&e), [Some("toward"), Some("away")]);
        // undo puts the lanes back, so they now run against the region's side
        assert!(e.undo());
        assert_eq!(dirs(&e), [Some("away"), Some("toward")]);
        let side = e.view().checks.into_iter().find(|c| c.id == "side").unwrap();
        assert!(!side.ok);
        assert_eq!(side.label, "Traffic keeps right");
        assert!(e.redo());
        // a one-way street has nothing to conflict
        let lanes: Vec<u32> = e.current().iter().filter(|s| s.kind == kind("travel")).map(|s| s.uid).collect();
        assert!(e.set_direction(lanes[0], Some(0)));
        assert!(e.view().checks.iter().find(|c| c.id == "side").unwrap().ok);
        // starting over keeps the region
        e.load_sample(0);
        assert_eq!(e.view().region, "united-states");
    }

    #[test]
    fn a_piece_can_take_a_different_type_at_certain_times() {
        let mut e = Editor::new(0);
        let parking = e.current()[1].uid;
        let bus = kind("bus");
        let base_pph = e.view().outcomes.capacity_pph;
        assert!(e.add_variant(parking));
        assert_eq!(e.view().revisions[0].label, "Parking is bus lane 07:00-10:00");
        assert_eq!(e.view().segments[1].width_mm, 3000); // a bus lane needs 3.0 m
        // shown at midday it is still parking; at 08:00 it is a bus lane
        assert_eq!(e.view().segments[1].kind, kind("parking"));
        assert_eq!(e.view().outcomes.capacity_pph, base_pph);
        assert!(e.set_time(8 * 60 + 7)); // snaps to a quarter hour
        assert_eq!(e.view().time_min, 8 * 60);
        assert_eq!(e.view().segments[1].kind, bus);
        assert_eq!(e.view().segments[1].base_kind, kind("parking"));
        assert_eq!(e.view().segments[1].active_variant, Some(0));
        assert!(e.view().outcomes.capacity_pph > base_pph);
        assert!(e.set_time(10 * 60)); // the window does not include its end
        assert_eq!(e.view().segments[1].kind, kind("parking"));
        // the time is a setting: no revision, and it survives undo
        assert_eq!(e.view().revisions.len(), 1);
        assert!(e.undo());
        assert!(e.view().segments[1].variants.is_empty());
        assert!(e.redo());
    }

    #[test]
    fn variant_windows_are_quarter_hours_and_never_overlap() {
        let mut e = Editor::new(0);
        let parking = e.current()[1].uid;
        assert!(e.add_variant(parking)); // 07:00-10:00
        assert!(e.add_variant(parking)); // takes the next free window, 16:00-19:00
        let w = |e: &Editor| -> Vec<_> { e.view().segments[1].variants.iter().map(|v| (v.from_min, v.to_min)).collect() };
        assert_eq!(w(&e), [(420, 600), (960, 1140)]);
        assert!(!e.set_variant_time(parking, 1, 500, 700)); // overlaps the first
        assert!(!e.set_variant_time(parking, 1, 601, 700)); // not a quarter hour
        assert!(!e.set_variant_time(parking, 1, 700, 700)); // empty
        assert!(!e.set_variant_time(parking, 5, 700, 800)); // no such window
        assert!(e.set_variant_time(parking, 1, 22 * 60, 5 * 60)); // past midnight
        e.set_time(23 * 60);
        assert_eq!(e.view().segments[1].active_variant, Some(1));
        e.set_time(2 * 60);
        assert_eq!(e.view().segments[1].active_variant, Some(1));
        e.set_time(12 * 60);
        assert_eq!(e.view().segments[1].active_variant, None);
        assert!(e.remove_variant(parking, 1));
        assert!(!e.remove_variant(parking, 1));
    }

    #[test]
    fn only_roadway_pieces_change_type_and_the_width_must_suit_every_type() {
        let mut e = Editor::new(0);
        assert!(!e.add_variant(e.current()[0].uid)); // a sidewalk is not roadway
        let parking = e.current()[1].uid;
        assert!(e.add_variant(parking));
        let alts = e.view().segments[1].alt_kinds.clone();
        assert!(!alts.contains(&kind("sidewalk")) && !alts.contains(&kind("parking")));
        assert!(!e.set_variant_kind(parking, 0, kind("sidewalk")));
        assert!(!e.set_variant_kind(parking, 0, kind("bus"))); // unchanged
        // parking (2.1 to 3.0 m) as a bus lane (3.0 m or more) is 3.0 m wide
        assert_eq!(e.view().segments[1].width_mm, 3000);
        assert_eq!((e.view().segments[1].min_mm, e.view().segments[1].max_mm), (3000, 3000));
        e.set_width(parking, 99_000);
        assert_eq!(e.view().segments[1].width_mm, 3000);
        // a type that shares no width with the others is not offered
        assert!(!e.view().segments[1].alt_kinds.contains(&kind("bike")) || KINDS[kind("bike")].max_mm >= 3000);
        assert!(e.set_variant_kind(parking, 0, kind("loading")));
        assert_eq!(e.view().segments[1].width_mm, 3000);
    }

    #[test]
    fn surface_edits_go_to_whichever_type_is_shown() {
        let mut e = Editor::new(0);
        let parking = e.current()[1].uid;
        assert!(e.add_variant(parking));
        let concrete = MATERIALS.iter().position(|m| m.id == "concrete").unwrap();
        e.set_time(8 * 60);
        assert!(e.set_material(parking, concrete)); // the bus lane's surface
        assert_eq!(e.view().segments[1].material, "concrete");
        e.set_time(12 * 60);
        assert_eq!(e.view().segments[1].material, "asphalt"); // parking is untouched
    }

    #[test]
    fn a_bike_lane_may_have_a_bus_boarding_island() {
        let mut e = Editor::new(0);
        let bike = e.add(kind("bike"), 1);
        let island = CURBS.iter().position(|c| c.id == "island").unwrap();
        assert!(e.set_curb(bike, Some(island)));
        assert_eq!(e.view().segments[1].curb, Some("island"));
        assert_eq!(e.view().revisions.last().unwrap().label, "Bike lane curb: bus boarding island");
        for k in KINDS.iter().filter(|k| k.has_curb) {
            assert!(!k.curbs.is_empty() && k.curbs.iter().all(|&c| c < CURBS.len()), "{}", k.id);
            assert!(k.curbs.contains(&DEFAULT_CURB), "{}", k.id);
        }
        assert!(KINDS.iter().filter(|k| !k.has_curb).all(|k| k.curbs.is_empty()));
    }

    #[test]
    fn set_curb_only_on_kinds_that_have_one() {
        let mut e = Editor::new(0);
        let walk = e.current()[0].uid;
        let lane = e.current()[2].uid;
        assert!(!e.set_curb(lane, Some(0))); // driving lanes have no curb
        assert!(!e.set_curb(walk, Some(99))); // not in the table
        let island = CURBS.iter().position(|c| c.id == "island").unwrap();
        assert!(!e.set_curb(walk, Some(island))); // a boarding island is for bike lanes
        assert!(!e.set_curb(walk, Some(DEFAULT_CURB))); // unchanged
        assert!(e.set_curb(walk, Some(0)));
        assert_eq!(e.view().segments[0].curb, Some("granite"));
        assert_eq!(e.view().revisions[0].label, "Sidewalk curb: granite");
        assert!(e.set_curb(walk, CURBS.iter().position(|c| c.id == "planted")));
        assert_eq!(e.view().segments[0].curb, Some("planted"));
        assert!(e.undo());
        assert!(e.set_curb(walk, CURBS.iter().position(|c| c.id == "bikefriendly")));
        assert_eq!(e.view().segments[0].curb, Some("bikefriendly"));
        assert!(e.undo());
        assert!(e.set_curb(walk, CURBS.iter().position(|c| c.id == "kassel")));
        assert_eq!(e.view().revisions[1].label, "Sidewalk curb: bus-friendly curb");
        assert!(e.undo());
        assert!(e.set_curb(walk, None));
        assert_eq!(e.view().segments[0].curb, None);
        assert_eq!(e.view().revisions[1].label, "Sidewalk curb: none");
        assert!(e.undo());
        assert_eq!(e.view().segments[0].curb, Some("granite"));
    }

    #[test]
    fn material_and_curb_count_as_changes_from_today() {
        let mut e = Editor::new(0);
        assert!(!e.view().changed);
        e.set_curb(e.current()[0].uid, None);
        assert!(e.view().changed);
    }
}
