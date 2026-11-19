//! The plan's data: the whole street seen from above, first end on the left, as runs of one section each, the
//! boundaries between them and the things along it. Every position here counts from the plan's left end (the
//! street's first end), whatever end the street is anchored at; the editor's anchored millimetres are converted with
//! `Editor::left_mm`. What the plan draws and how a press on it is read are the view's (Task 7, Task 8).

use serde::Serialize;

use super::{CalmingStripView, CurbView, Editor, GreenRowView, Host, Pick, Run, Said, Side, StopsView, ZoneEnd};

/// The plan of the street. Runs, boundaries and objects come in left-to-right order.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlanView {
    /// The street's length as loaded; `None` when neither end is an end to measure from (no anchor): the plan is then
    /// drawn schematic, the shown run alone and nothing along it.
    pub length_mm: Option<i32>,
    pub runs: Vec<PlanRun>,
    /// One between each two runs, none inside a run.
    pub boundaries: Vec<PlanBoundary>,
    pub objects: Vec<PlanObject>,
    /// The picked stretch, `(from, to)` with `from < to`; `None` for the whole street.
    pub pick: Option<(i32, i32)>,
    /// The left end, then the right end, is cut by the edge of the area.
    pub cut: [bool; 2],
    /// The changes along the street were drawn on a road of another length or from its other end: nothing may be
    /// rearranged.
    pub elsewhere: bool,
}

/// A part of the street that looks the same (`Editor::runs`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlanRun {
    pub from_mm: i32,
    pub to_mm: i32,
    /// The run's first zone, in anchored order.
    pub zone: usize,
    /// The run's pieces at the shown time, in the section's order: top to bottom on the plan.
    pub bands: Vec<PlanBand>,
    /// Its section breaks no check and it hosts no failing transition.
    pub ok: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PlanBand {
    pub uid: u32,
    /// Index into `KINDS`, at the shown time.
    pub kind: usize,
    /// Index into `MATERIALS`, at the shown time.
    pub material: usize,
    pub width_mm: i32,
}

/// Where one run meets the next.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PlanBoundary {
    pub at_mm: i32,
    /// The zone the boundary starts, in anchored order (the one `move_boundary` moves the start of).
    pub zone: usize,
    /// The transition's taper, `(from, to)`, at its true length on the side of the run that holds it; it may reach
    /// past that run (a failing transition) or past the street's end.
    pub taper: Option<(i32, i32)>,
    /// The boundary may be dragged: the street has an end to measure from and its changes were drawn on this road.
    pub movable: bool,
}

/// What an object on the plan is: a cell of the view the details panel already reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PlanThing {
    /// Cell `cell` of the curbside strip on `side` (`View.curb`).
    Curb { side: Side, cell: usize },
    /// Cell `cell` of the stops' strip on `side` (`View.stops`).
    Stop { side: Side, cell: usize },
    /// Cell `cell` of the raised elements' strip (`View.calming_strip`).
    Calming { cell: usize },
    /// Cell `i` of the Green row (`View.green`).
    Green { i: usize },
}

/// A thing along the street, cut to the plan's length.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlanObject {
    pub thing: PlanThing,
    pub from_mm: i32,
    pub to_mm: i32,
    /// The side it stands on, for a curbside use or a stop; `None` for what lies across or on a piece.
    pub side: Option<Side>,
    pub label: Said,
    pub chosen: bool,
}

impl Editor {
    /// The plan, with the things along the street taken from the strips' views already made for this view.
    pub(super) fn plan_view(&self, curb: &CurbView, stops: &StopsView, calming: &CalmingStripView, green: &GreenRowView) -> PlanView {
        let content = &self.states[self.cursor].content;
        let failing = self.failing_transitions();
        let plan_run = |run: &Run, from_mm: i32, to_mm: i32| PlanRun {
            from_mm,
            to_mm,
            zone: run.zones.start,
            bands: content
                .section(run.zones.start)
                .iter()
                .map(|s| {
                    let now = s.at(self.time_min);
                    PlanBand { uid: s.uid, kind: now.kind, material: now.material, width_mm: s.width_mm }
                })
                .collect(),
            ok: run.zones.clone().all(|z| self.section_passes(content.section(z))) && !failing.iter().any(|(_, host, _)| run.zones.contains(host)),
        };
        let runs = self.runs();
        let Some(frame) = self.frame else {
            // No end to measure from: the shown run alone, with no positions and nothing along it.
            let shown = runs.iter().find(|r| r.zones.contains(&self.zone)).or(runs.first());
            return PlanView {
                length_mm: None,
                runs: shown.map(|r| plan_run(r, 0, 0)).into_iter().collect(),
                boundaries: Vec::new(),
                objects: Vec::new(),
                pick: None,
                cut: [true, true],
                elsewhere: false,
            };
        };
        let length = frame.length_mm;
        let mut plan_runs: Vec<PlanRun> = runs
            .iter()
            .map(|r| {
                let (from_mm, to_mm) = self.left_span(r.from_mm, r.to_mm);
                plan_run(r, from_mm, to_mm)
            })
            .collect();
        plan_runs.sort_by_key(|r| r.from_mm);
        let transitions = self.transitions();
        let movable = self.structure_frame(true).is_ok();
        let mut boundaries: Vec<PlanBoundary> = runs
            .windows(2)
            .map(|pair| {
                let (at, zone) = (pair[1].from_mm, pair[1].zones.start);
                // The transition between the last zone of one run and the first of the next, on the side of its host.
                let taper = transitions.iter().find(|(b, _)| *b + 1 == zone).map(|(_, t)| match t.host {
                    Host::Near => self.left_span(at - t.length_mm, at),
                    Host::Far => self.left_span(at, at + t.length_mm),
                });
                PlanBoundary { at_mm: self.left_mm(at), zone, taper, movable }
            })
            .collect();
        boundaries.sort_by_key(|b| b.at_mm);
        let mut objects = Vec::new();
        let mut put = |thing: PlanThing, from_mm: i32, to_mm: i32, side: Option<Side>, label: &Said, chosen: bool| {
            let (from_mm, to_mm) = self.left_span(from_mm, to_mm);
            // A thing drawn on a longer road may lie past the loaded end: it is cut to the plan, or left out.
            let (from_mm, to_mm) = (from_mm.max(0), to_mm.min(length));
            if from_mm < to_mm {
                objects.push(PlanObject { thing, from_mm, to_mm, side, label: label.clone(), chosen });
            }
        };
        for side in &curb.sides {
            for (cell, c) in side.cells.iter().enumerate().filter(|(_, c)| c.instance.is_some()) {
                put(PlanThing::Curb { side: side.side, cell }, c.from_mm, c.to_mm, Some(side.side), &c.label, side.selected == Some(cell));
            }
        }
        for side in &stops.sides {
            for (cell, c) in side.cells.iter().enumerate().filter(|(_, c)| c.instance.is_some()) {
                put(PlanThing::Stop { side: side.side, cell }, c.from_mm, c.to_mm, Some(side.side), &c.label, side.selected == Some(cell));
            }
        }
        for (cell, c) in calming.cells.iter().enumerate().filter(|(_, c)| c.element.is_some()) {
            put(PlanThing::Calming { cell }, c.from_mm, c.to_mm, None, &c.label, calming.selected == Some(cell));
        }
        for (i, c) in green.cells.iter().enumerate() {
            put(PlanThing::Green { i }, c.from_mm, c.to_mm, None, &c.label, green.selected == Some(i));
        }
        objects.sort_by_key(|o| o.from_mm);
        PlanView {
            length_mm: Some(length),
            runs: plan_runs,
            boundaries,
            objects,
            pick: match self.pick {
                Pick::Whole => None,
                Pick::Stretch { from_mm, to_mm } => Some(self.left_span(from_mm, to_mm)),
            },
            // The frame is measured from the first end when it is a real end, else from the last, so a frame from the
            // last end says the first is cut; the city says of each end whether the area's edge cuts it.
            cut: [frame.from_end == ZoneEnd::Last || self.ends[0].cut, self.ends[1].cut],
            elsewhere: self.elsewhere().is_some(),
        }
    }

    /// The anchored span between `a_mm` and `b_mm`, from the plan's left end, smaller end first.
    fn left_span(&self, a_mm: i32, b_mm: i32) -> (i32, i32) {
        let (a, b) = (self.left_mm(a_mm), self.left_mm(b_mm));
        (a.min(b), a.max(b))
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::{two_zones, zoned};
    use super::super::{Calming, CalmingKind, EndInfo, Green, GreenKind, Instance, Stop, StopKind, Street, StreetExt, TreeSize, Use, ZoneFrame};
    use super::*;

    const LEN: i32 = 60_000;

    fn sample() -> Street {
        Street::sample(0, Side::Right)
    }

    /// The sample street divided at `starts` (anchored at `from_end`), drawn on a road `LEN` long; zone `i` has its
    /// last piece (the right sidewalk, so nothing beside it shifts and no taper is needed) widened by `widen[i]` mm.
    fn street(starts: &[i32], widen: &[i32], from_end: ZoneEnd) -> Street {
        let base = sample();
        let mut s = zoned(base.clone(), starts, LEN);
        for (zone, by) in s.zones.iter_mut().zip(widen) {
            if *by != 0 {
                let mut own = base.segments.clone();
                own.last_mut().expect("the sample has pieces").width_mm += by;
                zone.segments = Some(own);
            }
        }
        s.from_end = from_end;
        s.settled()
    }

    fn frame(length_mm: i32, from_end: ZoneEnd) -> Option<ZoneFrame> {
        Some(ZoneFrame { length_mm, from_end })
    }

    fn editor(now: &Street, frame: Option<ZoneFrame>) -> Editor {
        Editor::from_street(&sample(), now, 0).with_frame(frame)
    }

    fn plan(e: &Editor) -> PlanView {
        e.view().plan
    }

    /// Each run's `(from, to, zone)`.
    fn spans(p: &PlanView) -> Vec<(i32, i32, usize)> {
        p.runs.iter().map(|r| (r.from_mm, r.to_mm, r.zone)).collect()
    }

    /// Each boundary's `(at, zone)`.
    fn marks(p: &PlanView) -> Vec<(i32, usize)> {
        p.boundaries.iter().map(|b| (b.at_mm, b.zone)).collect()
    }

    fn last_width(r: &PlanRun) -> i32 {
        r.bands.last().expect("a run has pieces").width_mm
    }

    #[test]
    fn runs_are_left_to_right_with_their_sections() {
        let e = editor(&street(&[0, 20_000, 40_000], &[0, 100, 0], ZoneEnd::First), frame(LEN, ZoneEnd::First));
        let p = plan(&e);
        assert_eq!(p.length_mm, Some(LEN));
        assert_eq!(spans(&p), [(0, 20_000, 0), (20_000, 40_000, 1), (40_000, LEN, 2)]);
        let base = sample().segments;
        let bands: Vec<PlanBand> = base.iter().map(|s| PlanBand { uid: s.uid, kind: s.kind, material: s.material, width_mm: s.width_mm }).collect();
        assert_eq!(p.runs[0].bands, bands, "the section's pieces, in order, top to bottom");
        assert_eq!(last_width(&p.runs[1]), base.last().unwrap().width_mm + 100);
        // A run passes as its zone does on the zone strip: the widened one is wider than the street's room.
        let zones = e.view().zones.zones;
        assert_eq!(p.runs.iter().map(|r| r.ok).collect::<Vec<_>>(), [true, false, true]);
        assert_eq!(p.runs.iter().map(|r| zones[r.zone].ok).collect::<Vec<_>>(), [true, false, true]);
        assert_eq!(marks(&p), [(20_000, 1), (40_000, 2)]);
        assert!(p.boundaries.iter().all(|b| b.movable && b.taper.is_none()), "a wider sidewalk needs no taper");
        assert!(!p.elsewhere);

        // Drawn on a road 60 m long, loaded on one 58 m long: kept and shown, but no boundary may move.
        let elsewhere = plan(&editor(&street(&[0, 20_000, 40_000], &[0, 100, 0], ZoneEnd::First), frame(58_000, ZoneEnd::First)));
        assert!(elsewhere.elsewhere);
        assert_eq!(spans(&elsewhere), [(0, 20_000, 0), (20_000, 40_000, 1), (40_000, 58_000, 2)], "drawn at the loaded length");
        assert!(elsewhere.boundaries.iter().all(|b| !b.movable));
    }

    #[test]
    fn a_street_anchored_at_its_last_end_is_drawn_first_end_left() {
        // Zone 0 lies at the last end, so at the plan's right; zone 2 at its left.
        let e = editor(&street(&[0, 20_000, 40_000], &[0, 100, 200], ZoneEnd::Last), frame(LEN, ZoneEnd::Last));
        let p = plan(&e);
        assert_eq!(spans(&p), [(0, 20_000, 2), (20_000, 40_000, 1), (40_000, LEN, 0)]);
        let width = sample().segments.last().unwrap().width_mm;
        assert_eq!(p.runs.iter().map(last_width).collect::<Vec<_>>(), [width + 200, width + 100, width]);
        assert_eq!(marks(&p), [(20_000, 2), (40_000, 1)], "each boundary names the zone it starts, counted from the anchored end");
    }

    #[test]
    fn identical_neighbouring_zones_are_one_run_with_no_boundary() {
        let e = editor(&street(&[0, 20_000, 40_000], &[0, 0, 100], ZoneEnd::First), frame(LEN, ZoneEnd::First));
        let p = plan(&e);
        assert_eq!(spans(&p), [(0, 40_000, 0), (40_000, LEN, 2)]);
        assert_eq!(marks(&p), [(40_000, 2)]);
    }

    #[test]
    fn a_boundary_with_a_transition_carries_its_taper_at_true_length_on_the_host_side() {
        // Zone 2 (from 50 m) has one travel lane fewer and hosts the 53.3 m transition: it runs from the boundary
        // into zone 2, past the street's end, at its true length.
        let e = two_zones(true).with_frame(frame(100_000, ZoneEnd::First));
        let p = plan(&e);
        assert_eq!(marks(&p), [(50_000, 1)]);
        assert_eq!(p.boundaries[0].taper, Some((50_000, 103_300)));
        assert!(!p.runs[1].ok, "zone 2 is too short for its transition");

        // The same zones anchored at the last end: drawn mirrored, the taper on the side of the zone that holds it.
        let mut s = two_zones(true).snapshot();
        s.from_end = ZoneEnd::Last;
        let e = Editor::from_street(&s, &s, 0).with_frame(frame(100_000, ZoneEnd::Last));
        let p = plan(&e);
        assert_eq!(marks(&p), [(50_000, 1)]);
        // Zone 1, the one with a lane fewer, still hosts the 53.3 m taper; it now lies left of the boundary, so the
        // taper runs from the boundary leftward, past the plan's left end.
        assert_eq!(p.boundaries[0].taper, Some((-3_300, 50_000)));
    }

    /// The sample street, anchored at `from_end`, `LEN` long, with a parking use on the left, a stop on the right, a
    /// raised table and a tree, each at a place counted from the anchored end.
    fn with_things(from_end: ZoneEnd) -> Editor {
        let s = Street {
            from_end,
            length_mm: Some(LEN),
            curb: vec![Instance { side: Side::Left, from_mm: 10_000, length_mm: 6_000, use_: Use::Parking }],
            stops: vec![Stop { side: Side::Right, from_mm: 30_000, length_mm: 12_000, kind: StopKind::Curb, shelter: false, name: None }],
            calming: vec![Calming { from_mm: 20_000, length_mm: 7_000, kind: CalmingKind::Table, first: None }],
            green: vec![Green { kind: GreenKind::Tree, from_mm: 45_000, length_mm: 1_000, offset_mm: 1_500, width_mm: 1_000, size: Some(TreeSize::M) }],
            ..sample()
        };
        Editor::from_street(&s, &s, 0).with_frame(frame(LEN, from_end))
    }

    /// Each object's `(thing, from, to, side)`.
    fn places(p: &PlanView) -> Vec<(PlanThing, i32, i32, Option<Side>)> {
        p.objects.iter().map(|o| (o.thing, o.from_mm, o.to_mm, o.side)).collect()
    }

    #[test]
    fn objects_sit_at_their_places_on_their_sides() {
        let mut e = with_things(ZoneEnd::First);
        let v = e.view();
        let curb = v.curb.sides.iter().find(|s| s.side == Side::Left).expect("the left curbside");
        let stops = v.stops.sides.iter().find(|s| s.side == Side::Right).expect("the right stops");
        let cell_of = |cells: Vec<Option<usize>>| cells.iter().position(Option::is_some).expect("a cell");
        let use_cell = cell_of(curb.cells.iter().map(|c| c.instance).collect());
        let stop_cell = cell_of(stops.cells.iter().map(|c| c.instance).collect());
        let table_cell = cell_of(v.calming_strip.cells.iter().map(|c| c.element).collect());
        assert_eq!(
            places(&v.plan),
            [
                (PlanThing::Curb { side: Side::Left, cell: use_cell }, 10_000, 16_000, Some(Side::Left)),
                (PlanThing::Calming { cell: table_cell }, 20_000, 27_000, None),
                (PlanThing::Stop { side: Side::Right, cell: stop_cell }, 30_000, 42_000, Some(Side::Right)),
                (PlanThing::Green { i: 0 }, 45_000, 46_000, None),
            ]
        );
        assert_eq!(v.plan.objects[0].label, curb.cells[use_cell].label);
        assert_eq!(v.plan.objects[3].label, v.green.cells[0].label);
        assert!(v.plan.objects.iter().all(|o| !o.chosen));
        e.select_curb(Side::Left, 12_000);
        assert_eq!(plan(&e).objects.iter().map(|o| o.chosen).collect::<Vec<_>>(), [true, false, false, false]);

        // Anchored at the last end, every place is counted from the other end and the order turns round.
        let e = with_things(ZoneEnd::Last);
        let p = plan(&e);
        assert_eq!(
            places(&p).iter().map(|&(_, from, to, side)| (from, to, side)).collect::<Vec<_>>(),
            [(14_000, 15_000, None), (18_000, 30_000, Some(Side::Right)), (33_000, 40_000, None), (44_000, 50_000, Some(Side::Left))]
        );
        assert!(matches!(p.objects[0].thing, PlanThing::Green { .. }));
    }

    #[test]
    fn a_street_with_no_anchor_has_no_length_and_no_objects() {
        let mut s = street(&[0, 20_000, 40_000], &[0, 100, 0], ZoneEnd::First);
        s.curb = vec![Instance { side: Side::Left, from_mm: 10_000, length_mm: 6_000, use_: Use::Parking }];
        let e = editor(&s, None);
        assert!(!e.view().curb.sides.is_empty(), "the strips still lay the use along the length the zones were drawn on");
        let p = plan(&e);
        assert_eq!(p.length_mm, None);
        assert!(p.objects.is_empty());
        assert!(p.boundaries.is_empty());
        assert_eq!(p.runs.len(), 1, "the shown run alone, drawn schematic");
        assert_eq!(p.runs[0].zone, e.zone());
        assert_eq!(p.pick, None);
        assert_eq!(p.cut, [true, true]);
    }

    #[test]
    fn the_pick_is_given_in_left_positions_and_none_for_the_whole_street() {
        let mut e = editor(&street(&[0, 20_000, 40_000], &[0, 100, 200], ZoneEnd::Last), frame(LEN, ZoneEnd::Last));
        assert_eq!(plan(&e).pick, None);
        e.pick_stretch(10_000, 30_000).expect("a stretch");
        assert_eq!(plan(&e).pick, Some((30_000, 50_000)));
        let mut e = editor(&street(&[0, 20_000, 40_000], &[0, 100, 200], ZoneEnd::First), frame(LEN, ZoneEnd::First));
        e.pick_stretch(10_000, 30_000).expect("a stretch");
        assert_eq!(plan(&e).pick, Some((10_000, 30_000)));
        e.pick_whole();
        assert_eq!(plan(&e).pick, None);
    }

    #[test]
    fn cut_ends_are_marked() {
        let s = sample();
        assert_eq!(plan(&editor(&s, None)).cut, [true, true], "no end to measure from: both are cut");
        assert_eq!(plan(&editor(&s, frame(LEN, ZoneEnd::Last))).cut, [true, false], "measured from the last end: the first is cut");
        assert_eq!(plan(&editor(&s, frame(LEN, ZoneEnd::First))).cut, [false, false]);
        // Measured from the first end, the city says whether the last end is cut by the area's edge.
        let cut_at = |ends: [bool; 2]| editor(&s, frame(LEN, ZoneEnd::First)).with_ends(ends.map(|cut| EndInfo { cut, ..Default::default() }));
        assert_eq!(plan(&cut_at([false, true])).cut, [false, true]);
        assert_eq!(plan(&cut_at([false, false])).cut, [false, false]);
    }
}
