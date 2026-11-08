//! The press targets as shapes: for each piece of the selected run, its silhouette with no paint of its own, over the
//! whole drawing, so that a press lands on the piece whatever stands over it and a far, narrow piece is no harder to
//! hit than its stroke is wide.

use crate::shared::catalogue::{KINDS, kind_key};
use crate::shared::diagram::{Layer, LayerKind, Shape};
use crate::shared::i18n::{Args, I18n};
use crate::street::model::SegView;

use crate::street::iso::projection::Projection;
use crate::street::iso::scene::outline::silhouette;

/// One group per piece, carrying its uid and its kind's name, narrowest last so that, where the targets' strokes
/// overlap, the narrow piece is the one hit. `p` is the projection over the selected run's reach.
pub(in crate::street::iso) fn hits(p: &Projection, segs: &[SegView], i18n: &I18n) -> Layer {
    let mut order: Vec<usize> = (0..segs.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(segs[i].width_mm));
    let shapes = order
        .into_iter()
        .map(|i| {
            let s = &segs[i];
            Shape::Group {
                class: "iso-hit".into(),
                uid: Some(s.uid),
                data: None,
                hidden: false,
                title: Some(i18n.tr(&kind_key(KINDS[s.kind].id), &Args::new())),
                shapes: vec![Shape::Polygon { pts: silhouette(p, segs, i), class: "iso-hit-area".into(), face: None, texture: None }],
            }
        })
        .collect();
    Layer { kind: LayerKind::Hit, shapes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::iso::projection::{Projection, Reach};
    use crate::street::iso::scene::testing::each_run;
    use crate::street::iso::testing::en;

    #[test]
    fn a_target_per_piece_with_its_uid_and_name_the_narrowest_last() {
        let mut runs = 0;
        each_run(|name, p, segs, across, _, _| {
            if across {
                return;
            }
            let l = hits(p, segs, &en());
            assert_eq!(l.kind, LayerKind::Hit, "{name}");
            assert_eq!(l.shapes.len(), segs.len(), "{name}");
            let mut widths = Vec::new();
            for sh in &l.shapes {
                let Shape::Group { uid, title, shapes, class, .. } = sh else { panic!("{name}: a group") };
                let uid = uid.expect("a uid");
                let seg = segs.iter().find(|s| s.uid == uid).unwrap();
                widths.push(seg.width_mm);
                assert_eq!(class, "iso-hit");
                assert!(title.as_deref().is_some_and(|t| !t.is_empty()), "{name}");
                assert!(matches!(&shapes[..], [Shape::Polygon { class, texture: None, .. }] if class == "iso-hit-area"), "{name}");
            }
            assert!(widths.windows(2).all(|w| w[0] >= w[1]), "{name}: {widths:?}");
            runs += 1;
        });
        assert!(runs > 10);
    }

    #[test]
    fn the_target_of_a_piece_is_the_outline_s_silhouette() {
        let v = crate::street::model::Editor::new(0).view();
        let p = Projection::fit(1050.0, v.row_mm.max(v.total_mm)).over(Reach::STRETCH);
        let l = hits(&p, &v.segments, &en());
        let Shape::Group { uid, shapes, .. } = &l.shapes[0] else { panic!() };
        let i = v.segments.iter().position(|s| Some(s.uid) == *uid).unwrap();
        let Shape::Polygon { pts, .. } = &shapes[0] else { panic!() };
        assert_eq!(pts, &silhouette(&p, &v.segments, i));
    }
}
