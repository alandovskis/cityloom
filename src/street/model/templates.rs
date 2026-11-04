//! Street design templates: ready-made cross-sections a street can be arranged as from the Transform menu.
//! A template is not stored and not a kind of piece: applying one lays ordinary pieces out in the selected zone
//! (`Editor::apply_template`), fitted to the street's width, and the street is then edited, checked and undone
//! like any other. Pure data and rules, no wording (a refusal is `Said` data).
//!
//! Every figure and every arrangement here is a placeholder, UNVERIFIED: the pieces are CityLoom's own
//! simplification of what each sort of street usually has, not the guidance of any manual (TAC, MTQ, NACTO).

use serde::Serialize;

use super::{Arg, Said, Segment};
use crate::shared::catalogue::{KINDS, Side, StreetClass, kind_index};
use crate::street::measures::{Dir, Spec, lay_out, sp};

/// The templates, in the order the Transform menu lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Template {
    /// Wide sidewalks with patios and loading at the curb, a lane each way.
    Commercial,
    /// Walking first: a single shared lane between planting and sidewalks.
    School,
    /// Quiet street where bikes have a lane each way behind planting.
    Greenway,
    /// A bus lane at each curb and a lane each way for the rest.
    BusCorridor,
    /// Wide lanes and loading at both curbs for trucks.
    Industrial,
}

impl Template {
    pub const ALL: [Template; 5] = [Template::Commercial, Template::School, Template::Greenway, Template::BusCorridor, Template::Industrial];

    /// The id the page and the messages use (`template-<id>`, `template-refused-*`).
    pub fn id(self) -> &'static str {
        match self {
            Template::Commercial => "commercial",
            Template::School => "school",
            Template::Greenway => "greenway",
            Template::BusCorridor => "bus-corridor",
            Template::Industrial => "industrial",
        }
    }

    /// The message that names the template.
    pub fn name_key(self) -> &'static str {
        match self {
            Template::Commercial => "template-commercial",
            Template::School => "template-school",
            Template::Greenway => "template-greenway",
            Template::BusCorridor => "template-bus-corridor",
            Template::Industrial => "template-industrial",
        }
    }

    pub fn from_id(id: &str) -> Option<Template> {
        Template::ALL.into_iter().find(|t| t.id() == id)
    }

    /// The pieces from the left edge to the right, with which way each runs and which may be left out of a street
    /// too narrow for all of them. Sidewalks are never left out.
    fn specs(self) -> Vec<Spec> {
        use Dir::*;
        let (sidewalk, planting, terrace, loading) = (|| sp("sidewalk", None, false), || sp("planting", None, true), || sp("terrace", None, true), || sp("loading", None, true));
        let lane = || sp("travel", Half, false);
        match self {
            Template::Commercial => vec![sidewalk(), terrace(), loading(), lane(), lane(), loading(), terrace(), sidewalk()],
            Template::School => vec![sidewalk(), planting(), sp("travel", Fixed(0), false), planting(), sidewalk()],
            Template::Greenway => vec![sidewalk(), planting(), sp("bike", Half, false), lane(), lane(), sp("bike", Half, false), planting(), sidewalk()],
            Template::BusCorridor => vec![sidewalk(), planting(), sp("bus", Half, false), lane(), lane(), sp("bus", Half, false), planting(), sidewalk()],
            Template::Industrial => vec![sidewalk(), sp("loading", None, false), lane(), lane(), sp("loading", None, false), sidewalk()],
        }
    }

    fn kind(spec: &Spec) -> usize {
        kind_index(spec.kind).expect("templates use catalogue kinds")
    }

    /// The narrowest street the template fits: its pieces that cannot be left out, each at its least.
    pub fn min_mm(self) -> i32 {
        self.specs().iter().filter(|s| !s.removable).map(|s| KINDS[Template::kind(s)].min_mm).sum()
    }

    /// The widest street it fits: every piece at its most.
    pub fn max_mm(self) -> i32 {
        self.specs().iter().map(|s| KINDS[Template::kind(s)].max_mm).sum()
    }

    /// Whether the template can be applied to a street of this class and width.
    pub fn applicable(self, class: StreetClass, row_mm: i32) -> Result<(), TemplateRefusal> {
        if class.is_freeway() {
            return Err(TemplateRefusal::Freeway);
        }
        if row_mm < self.min_mm() {
            return Err(TemplateRefusal::TooNarrow { min_mm: self.min_mm(), row_mm });
        }
        if row_mm > self.max_mm() {
            return Err(TemplateRefusal::TooWide { max_mm: self.max_mm(), row_mm });
        }
        Ok(())
    }

    /// The template's pieces fitted to `row_mm`: ordinary segments with fresh uids from `next_uid` and the
    /// directions of the side of the road traffic keeps. The widths add up to `row_mm`, less the part under
    /// 100 mm that does not make a step. None when `applicable` says no.
    pub fn arrange(self, class: StreetClass, row_mm: i32, side: Side, next_uid: &mut u32) -> Result<Vec<Segment>, TemplateRefusal> {
        self.applicable(class, row_mm)?;
        let too_narrow = TemplateRefusal::TooNarrow { min_mm: self.min_mm(), row_mm };
        lay_out(self.specs(), Vec::new(), Vec::new(), row_mm, side, next_uid).ok_or(too_narrow)
    }
}

/// Why a template cannot be applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TemplateRefusal {
    /// A limited-access road has no sidewalks to arrange and none of these streets.
    Freeway,
    TooNarrow {
        min_mm: i32,
        row_mm: i32,
    },
    TooWide {
        max_mm: i32,
        row_mm: i32,
    },
}

impl TemplateRefusal {
    /// The reason, as `Said` data (`template-refused-*`).
    pub fn said(self, template: Template) -> Said {
        let name = |s: Said| s.with("name", Arg::Said(Box::new(Said::new(template.name_key()))));
        match self {
            TemplateRefusal::Freeway => name(Said::new("template-refused-freeway")),
            TemplateRefusal::TooNarrow { min_mm, row_mm } => {
                name(Said::new("template-refused-narrow")).with("min", Arg::Length(min_mm)).with("row", Arg::Length(row_mm))
            }
            TemplateRefusal::TooWide { max_mm, row_mm } => {
                name(Said::new("template-refused-wide")).with("max", Arg::Length(max_mm)).with("row", Arg::Length(row_mm))
            }
        }
    }
}

/// A template as the view carries it: whether the selected zone's street can take it, and if not why.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct TemplateView {
    pub template: Template,
    pub refusal: Option<TemplateRefusal>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::{DirectionRule, is_roadway};
    use crate::street::model::{Editor, SAMPLES, Street};

    fn arranged(t: Template, row_mm: i32, side: Side) -> Vec<Segment> {
        t.arrange(StreetClass::Local, row_mm, side, &mut 1).unwrap_or_else(|r| panic!("{t:?} at {row_mm}: {r:?}"))
    }

    #[test]
    fn every_template_lays_out_plain_pieces_that_fill_a_wide_street() {
        for t in Template::ALL {
            for row in [t.min_mm().max(15_000), 18_000, 24_000] {
                if t.applicable(StreetClass::Local, row).is_err() {
                    continue;
                }
                let segs = arranged(t, row, Side::Right);
                let sum: i32 = segs.iter().map(|s| s.width_mm).sum();
                assert!((row - 100..=row).contains(&sum), "{t:?} at {row}: {sum}");
                for s in &segs {
                    let k = &KINDS[s.kind];
                    assert!((k.min_mm..=k.max_mm).contains(&s.width_mm), "{t:?}: {} {}", k.id, s.width_mm);
                    assert!(s.variants.is_empty());
                    assert_eq!(s.direction.is_some(), k.direction == DirectionRule::Required || (k.direction == DirectionRule::Optional && s.direction.is_some()));
                }
                let uids: std::collections::HashSet<u32> = segs.iter().map(|s| s.uid).collect();
                assert_eq!(uids.len(), segs.len(), "uids are unique");
            }
        }
    }

    #[test]
    fn a_template_has_its_own_pieces() {
        let ids = |t: Template| arranged(t, 20_000, Side::Right).iter().map(|s| KINDS[s.kind].id).collect::<Vec<_>>();
        assert_eq!(ids(Template::School), ["sidewalk", "planting", "travel", "planting", "sidewalk"]);
        assert_eq!(ids(Template::BusCorridor), ["sidewalk", "planting", "bus", "travel", "travel", "bus", "planting", "sidewalk"]);
        assert_eq!(ids(Template::Greenway), ["sidewalk", "planting", "bike", "travel", "travel", "bike", "planting", "sidewalk"]);
        assert_eq!(ids(Template::Industrial), ["sidewalk", "loading", "travel", "travel", "loading", "sidewalk"]);
        assert!(ids(Template::Commercial).contains(&"terrace"));
    }

    #[test]
    fn directions_follow_the_side_of_the_road_traffic_keeps() {
        for (side, left) in [(Side::Right, 1), (Side::Left, 0)] {
            let dirs = |t| arranged(t, 20_000, side).iter().filter(|s| s.direction.is_some()).map(|s| s.direction.unwrap()).collect::<Vec<_>>();
            // Left half first: with traffic on the right, the left half runs toward the viewer (1).
            assert_eq!(dirs(Template::BusCorridor), [left, left, 1 - left, 1 - left], "{side:?}");
            assert_eq!(dirs(Template::Greenway), [left, left, 1 - left, 1 - left], "{side:?}");
            assert_eq!(dirs(Template::Commercial), [left, 1 - left], "{side:?}");
        }
    }

    #[test]
    fn a_narrow_street_loses_what_can_go_and_a_street_below_the_minimum_is_refused_with_both_figures() {
        let segs = arranged(Template::BusCorridor, Template::BusCorridor.min_mm(), Side::Right);
        assert!(segs.iter().all(|s| KINDS[s.kind].id != "planting"));
        assert!(segs.iter().filter(|s| is_roadway(s.kind)).count() == 4);
        let r = Template::BusCorridor.applicable(StreetClass::Local, 9_000);
        assert_eq!(r, Err(TemplateRefusal::TooNarrow { min_mm: Template::BusCorridor.min_mm(), row_mm: 9_000 }));
        let said = r.unwrap_err().said(Template::BusCorridor);
        assert_eq!(said.key, "template-refused-narrow");
    }

    #[test]
    fn a_freeway_takes_none_and_a_street_wider_than_the_template_can_use_is_refused() {
        for t in Template::ALL {
            assert_eq!(t.applicable(StreetClass::Motorway, 30_000), Err(TemplateRefusal::Freeway));
            let wide = t.max_mm() + 100;
            assert_eq!(t.applicable(StreetClass::Local, wide), Err(TemplateRefusal::TooWide { max_mm: t.max_mm(), row_mm: wide }));
        }
    }

    #[test]
    fn the_editor_applies_a_template_to_the_selected_zone_as_one_undoable_edit_and_the_street_stays_sound() {
        let mut e = Editor::new(0);
        let before = e.snapshot();
        assert_eq!(SAMPLES[0].row_mm, 18_000);
        assert_eq!(e.apply_template(Template::Greenway), Ok(()));
        let v = e.view();
        assert_eq!(v.revisions.len(), 1);
        assert!(v.delta_mm <= 0 && v.delta_mm > -100, "fits: {}", v.delta_mm);
        assert_eq!(v.segments.iter().filter(|s| s.kind == kind_index("bike").unwrap()).count(), 2);
        assert!(e.undo());
        assert_eq!(e.snapshot().segments, before.segments);
        assert!(e.redo());
        assert!(e.reset());
        assert_eq!(e.snapshot().segments, before.segments);
    }

    #[test]
    fn a_refused_template_changes_nothing() {
        let mut e = Editor::new(2);
        let before = e.snapshot();
        assert!(matches!(e.apply_template(Template::BusCorridor), Err(TemplateRefusal::TooNarrow { .. })));
        assert_eq!(e.snapshot(), before);
        assert!(e.view().revisions.is_empty());
        let mut f = Editor::new(3);
        assert_eq!(f.apply_template(Template::School), Err(TemplateRefusal::Freeway));
    }

    #[test]
    fn the_view_lists_all_five_with_the_reason_of_each_that_does_not_fit() {
        let v = Editor::new(2).view();
        assert_eq!(v.templates.iter().map(|t| t.template).collect::<Vec<_>>(), Template::ALL);
        let refused: Vec<Template> = v.templates.iter().filter(|t| t.refusal.is_some()).map(|t| t.template).collect();
        assert!(refused.contains(&Template::BusCorridor) && !refused.contains(&Template::School), "{refused:?}");
    }

    #[test]
    fn a_template_applies_to_the_selected_zone_only() {
        let today = Street::sample(0, Side::Right);
        let mut e = Editor::from_street(&today, &crate::street::model::zoned(today.clone(), &[0, 20_000, 40_000], 60_000), 0);
        let base = e.snapshot().segments;
        assert!(e.select_zone(1));
        assert_eq!(e.apply_template(Template::School), Ok(()));
        let s = e.snapshot();
        assert_eq!(s.segments, base);
        assert!(s.zones[1].segments.is_some());
        assert_eq!(s.zones[0].segments, None);
    }
}
