//! Why stops, traffic calming and green cannot be added to the street as it is, for the add menu to say beside the
//! group it greys.

use super::curb::CurbRefusal;
use super::{Editor, Said};

/// Why the things that are not pieces cannot be added to the street as it is; `None` where they can.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct Unavailable {
    pub stops: Option<Said>,
    pub calming: Option<Said>,
    pub green: Option<Said>,
}

impl Editor {
    /// The first cause for each group, from the same functions that lay out its row.
    pub(super) fn unavailable(&self) -> Unavailable {
        let length_mm = self.states[self.cursor].content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(0);
        let stops = self.stops_view().sides.is_empty().then(|| Said::new(if length_mm <= 0 { "stop-refused-no-anchor" } else { "add-stops-no-lane" }));
        let calming = self.calming_strip_view().cells.is_empty().then(|| Said::new("calming-refused-no-anchor"));
        let green = (!self.green_row().can_add).then(|| {
            Said::new(match self.curb_frame(true) {
                Err(CurbRefusal::NoAnchor) => "green-refused-no-anchor",
                Err(CurbRefusal::Elsewhere) => "green-refused-elsewhere",
                _ => "add-green-no-host",
            })
        });
        Unavailable { stops, calming, green }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::{Side, StreetClass, kind_index};
    use crate::street::model::{Piece, Street, StreetExt, Use, ZoneEnd, ZoneFrame};

    fn street_of(ids: &[(&str, i32)]) -> Street {
        let pieces: Vec<Piece> =
            ids.iter().map(|&(id, width_mm)| Piece { kind: kind_index(id).unwrap(), width_mm, direction: None, variants: Vec::new() }).collect();
        Street::imported(StreetClass::Local, Side::Right, &pieces)
    }

    fn editor(ids: &[(&str, i32)], length_mm: Option<i32>) -> Editor {
        let s = street_of(ids);
        Editor::from_street(&s, &s, 0).with_frame(length_mm.map(|length_mm| ZoneFrame { length_mm, from_end: ZoneEnd::First }))
    }

    fn sample_editor(sample: usize, length_mm: Option<i32>) -> Editor {
        let s = Street::sample(sample, Side::Right);
        Editor::from_street(&s, &s, 0).with_frame(length_mm.map(|length_mm| ZoneFrame { length_mm, from_end: ZoneEnd::First }))
    }

    const ALL: [(&str, i32); 3] = [("sidewalk", 3000), ("bus", 3300), ("sidewalk", 3000)];

    #[test]
    fn an_anchored_street_with_a_bus_lane_and_a_sidewalk_can_take_everything() {
        let e = editor(&ALL, Some(40_000));
        assert_eq!(e.view().unavailable, Unavailable::default());
    }

    #[test]
    fn a_street_with_no_length_says_so_for_stops_calming_and_green() {
        let e = editor(&ALL, None);
        let u = e.view().unavailable;
        assert_eq!(u.stops, Some(Said::new("stop-refused-no-anchor")));
        assert_eq!(u.calming, Some(Said::new("calming-refused-no-anchor")));
        assert_eq!(u.green, Some(Said::new("green-refused-no-anchor")), "no anchor wins over any other cause");
    }

    #[test]
    fn a_street_with_no_lane_a_bus_can_stop_in_says_so() {
        let e = editor(&[("sidewalk", 3000), ("bike", 1800), ("sidewalk", 3000)], Some(40_000));
        assert_eq!(e.view().unavailable.stops, Some(Said::new("add-stops-no-lane")));
    }

    #[test]
    fn a_street_of_roadway_only_has_no_host_for_green() {
        let e = editor(&[("travel", 3300), ("travel", 3300)], Some(40_000));
        assert_eq!(e.view().unavailable.green, Some(Said::new("add-green-no-host")));
    }

    #[test]
    fn a_street_drawn_on_a_road_of_another_length_says_elsewhere_for_green() {
        let mut e = Editor::from_street(&Street::sample(0, Side::Right), &Street::sample(0, Side::Right), 0)
            .with_frame(Some(ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::First }));
        e.add_curb(Side::Right, 0, 6_000, Use::Parking).unwrap();
        let drawn = e.snapshot();
        let e = Editor::from_street(&drawn, &drawn, 0).with_frame(Some(ZoneFrame { length_mm: 30_000, from_end: ZoneEnd::First }));
        assert_eq!(e.view().unavailable.green, Some(Said::new("green-refused-elsewhere")));
    }

    #[test]
    fn a_reason_is_given_exactly_where_the_menu_used_to_hide_a_group() {
        for s in 0..crate::street::model::SAMPLES.len() {
            for frame in [None, Some(60_000)] {
                let v = sample_editor(s, frame).view();
                assert_eq!(v.unavailable.stops.is_some(), v.stops.sides.is_empty(), "{s} {frame:?}");
                assert_eq!(v.unavailable.calming.is_some(), v.calming_strip.cells.is_empty(), "{s} {frame:?}");
                assert_eq!(v.unavailable.green.is_some(), !v.green.can_add, "{s} {frame:?}");
            }
        }
    }
}
