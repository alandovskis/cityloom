//! The 2.5D as shapes: builders that give each layer of the drawing as `shared::diagram` data, which the renderer
//! writes (`iso_svg` puts them together).

mod markings;
mod objects;
mod outline;
mod raised;
mod signs;
mod surface;

pub(super) use markings::markings;
pub(super) use objects::{block, objects};
pub(super) use outline::outline;
pub(super) use raised::raised;
pub(super) use signs::signs;
pub(super) use surface::surface;

#[cfg(test)]
pub(super) mod testing {
    use crate::shared::catalogue::Side;
    use crate::street::iso::curbside::{self, Over};
    use crate::street::iso::projection::{Projection, Reach, STRETCH_MM};
    use crate::street::iso::span_mm;
    use crate::street::iso::taper::Draw;
    use crate::street::iso::testing::{furnished, joined, joined_from_last, marked};
    use crate::street::model::{Editor, SegView, ShownInstance, ShownStop, StopKind, Street, Use, View, ZoneEnd, ZoneFrame};

    /// A street with a curbside programme and a frame, so that its uses are shown.
    pub fn curbed() -> Editor {
        let s = Street::sample(0, Side::Right);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::First }));
        e.add_curb(Side::Right, 10_000, 6_000, Use::Patio).unwrap();
        e.add_curb(Side::Right, 20_000, 6_000, Use::Parking).unwrap();
        e.add_curb(Side::Left, 8_000, 6_000, Use::Tree).unwrap();
        e
    }

    /// A street with bike parking, trees and a bus shelter, under a frame, so that racks, a tree and a shelter are shown.
    pub fn racked() -> Editor {
        let mut e = curbed();
        e.add_curb(Side::Left, 20_000, 8_000, Use::BikeParking).unwrap();
        e.add(crate::shared::catalogue::kind_index("busshelter").unwrap(), 1);
        e
    }

    pub const CURB: [(&str, i32, Option<usize>); 4] = [("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("sidewalk", 3000, None)];
    pub const BULB: [(&str, i32, Option<usize>); 5] =
        [("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0)), ("parking", 2400, None), ("sidewalk", 3000, None)];
    pub const ISLAND: [(&str, i32, Option<usize>); 6] = [
        ("sidewalk", 3000, None),
        ("travel", 3300, Some(1)),
        ("bus", 3300, Some(0)),
        ("buffer", 1800, None),
        ("bike", 1800, Some(0)),
        ("sidewalk", 3000, None),
    ];

    /// A street of `pieces` with a stop of `kind` on its right, with a shelter when asked.
    pub fn stopped(pieces: &[(&str, i32, Option<usize>)], kind: StopKind, shelter: bool, from_end: ZoneEnd) -> Editor {
        use crate::shared::catalogue::{REGIONS, StreetClass, kind_index};
        use crate::street::model::Piece;
        let pieces: Vec<Piece> =
            pieces.iter().map(|&(id, width_mm, direction)| Piece { kind: kind_index(id).unwrap(), width_mm, direction, variants: Vec::new() }).collect();
        let s = Street::imported(StreetClass::Local, Side::Right, &pieces);
        let region = REGIONS.iter().position(|r| r.drive_side == Side::Right).unwrap();
        let mut e = Editor::from_street(&s, &s, region).with_frame(Some(ZoneFrame { length_mm: 60_000, from_end }));
        e.add_stop(Side::Right, 10_000, 12_000, kind).unwrap();
        if shelter {
            e.set_stop_shelter(0, true).unwrap();
        }
        e
    }

    /// Every run with what stands on it: the view's own curbside uses and stops are the selected run's, the neighbour's
    /// has none. The objects layers are compared on these.
    pub fn each_object_run(mut f: impl FnMut(&str, &Projection, &[SegView], &[ShownInstance], &[ShownStop])) {
        let mut views: Vec<(String, View)> = (0..4).map(|i| (format!("sample {i}"), Editor::new(i).view())).collect();
        views.extend([
            ("furnished".to_string(), furnished().view()),
            ("marked".to_string(), marked().view()),
            ("wide".to_string(), wide()),
            ("curbed".to_string(), curbed().view()),
            ("racked".to_string(), racked().view()),
            ("curb stop".to_string(), stopped(&CURB, StopKind::Curb, false, ZoneEnd::First).view()),
            ("curb stop shelter".to_string(), stopped(&CURB, StopKind::Curb, true, ZoneEnd::First).view()),
            ("extension shelter".to_string(), stopped(&BULB, StopKind::Bulb, true, ZoneEnd::First).view()),
            ("island shelter".to_string(), stopped(&ISLAND, StopKind::Island, true, ZoneEnd::First).view()),
            ("joined 0".to_string(), joined(true, 0)),
            ("joined 1".to_string(), joined(true, 1)),
            ("joined same".to_string(), joined(false, 1)),
            ("joined last 0".to_string(), joined_from_last(true, 0)),
            ("joined last 1".to_string(), joined_from_last(true, 1)),
        ]);
        for (name, v) in &views {
            for width in [1050.0, 280.0] {
                let base = Projection::fit(width, span_mm(v));
                let name = format!("{name} at {width}");
                match &v.zones.joint {
                    None => f(&name, &base.over(Reach::STRETCH), &v.segments, &v.curb_shown, &v.stops_shown),
                    Some(j) => {
                        let l = STRETCH_MM;
                        let (near, far) = (Reach { near: 0.0, far: l / 2.0 }, Reach { near: l / 2.0, far: l });
                        let (own, across) = if j.far { (near, far) } else { (far, near) };
                        f(&name, &base.over(own), &v.segments, &v.curb_shown, &v.stops_shown);
                        f(&name, &base.over(across), &j.segments, &[], &[]);
                    }
                }
            }
        }
    }

    /// Sample Street 1 with a first piece 8 m wide: the design runs over the street.
    pub fn wide() -> View {
        let mut e = Editor::new(0);
        let u = e.view().segments[0].uid;
        assert!(e.set_width(u, 8_000));
        assert!(e.view().delta_mm > 0, "the design runs over the street");
        e.view()
    }

    /// Every sample the layers are compared on, with the runs each is drawn in: the view, the projection over the
    /// run's reach, its pieces, whether they are the neighbour's, the transition it holds and what is laid over it.
    pub fn each_run(mut f: impl FnMut(&str, &Projection, &[SegView], bool, Option<&Draw>, &[Over])) {
        let views: Vec<(String, View)> = (0..4)
            .map(|i| (format!("sample {i}"), Editor::new(i).view()))
            .chain([
                ("furnished".to_string(), furnished().view()),
                ("marked".to_string(), marked().view()),
                ("wide".to_string(), wide()),
                ("curbed".to_string(), curbed().view()),
                ("joined 0".to_string(), joined(true, 0)),
                ("joined 1".to_string(), joined(true, 1)),
                ("joined same".to_string(), joined(false, 1)),
                ("joined last 0".to_string(), joined_from_last(true, 0)),
                ("joined last 1".to_string(), joined_from_last(true, 1)),
            ])
            .collect();
        for (name, v) in &views {
            for width in [1050.0, 280.0] {
                let base = Projection::fit(width, span_mm(v));
                let name = format!("{name} at {width}");
                let uses =
                    |across: bool| if across { curbside::overs(&v.segments, &[], &[]) } else { curbside::overs(&v.segments, &v.curb_shown, &v.stops_shown) };
                match &v.zones.joint {
                    None => f(&name, &base.over(Reach::STRETCH), &v.segments, false, None, &uses(false)),
                    Some(j) => {
                        let l = STRETCH_MM;
                        let (near, far) = (Reach { near: 0.0, far: l / 2.0 }, Reach { near: l / 2.0, far: l });
                        let (own, across) = if j.far { (near, far) } else { (far, near) };
                        let own_draw = j.taper.as_ref().filter(|t| t.host_is_selected).map(|t| Draw::new(t, j.far));
                        let across_draw = j.taper.as_ref().filter(|t| !t.host_is_selected).map(|t| Draw::new(t, !j.far));
                        f(&name, &base.over(own), &v.segments, false, own_draw.as_ref(), &uses(false));
                        // The neighbour's pieces carry the same shown uses of the view only for the selected run.
                        let none: Vec<Over> = Vec::new();
                        f(&name, &base.over(across), &j.segments, true, across_draw.as_ref(), &none);
                    }
                }
            }
        }
    }
}
