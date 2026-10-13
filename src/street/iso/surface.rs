//! The surface layer: the top of every roadway piece, at the height of the ground, with its tint and hatch. The
//! markings are painted over it and a raised piece stands over both.

use crate::shared::i18n::I18n;
use crate::street::model::View;

use super::projection::Projection;
use super::raised::{Strip, group};

/// The strips split into the roadway's (a slab drawn at the ground) and the rest, each keeping its order.
pub(super) fn split(strips: Vec<Strip>) -> (Vec<Strip>, Vec<Strip>) {
    strips.into_iter().partition(|&(top, _, edge)| edge.is_none() && top == 0.0)
}

/// The roadway pieces as one group. Each keeps its `<g class="iso-piece" data-uid>`.
pub(super) fn layer(p: &Projection, v: &View, i18n: &I18n, flat: &[Strip]) -> String {
    group("<g class=\"iso-surface\">", p, v, i18n, flat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::KINDS;
    use crate::street::iso::raised::{height_mm, strips};
    use crate::street::model::Editor;

    #[test]
    fn the_roadway_slabs_are_the_surface_and_everything_else_stands_above_it() {
        for sample in 0..4 {
            let v = Editor::new(sample).view();
            let (flat, standing) = split(strips(&v));
            assert!(flat.iter().all(|&(top, i, edge)| top == 0.0 && edge.is_none() && height_mm(KINDS[v.segments[i].kind].id) == 0), "sample {sample}");
            assert!(standing.iter().all(|&(top, _, edge)| top > 0.0 || edge.is_some()), "sample {sample}");
            assert_eq!(flat.len() + standing.len(), strips(&v).len());
            assert_eq!(flat.len(), v.segments.iter().filter(|s| height_mm(KINDS[s.kind].id) == 0).count());
            // Both keep the lowest-to-highest order.
            assert!(standing.windows(2).all(|w| w[0].0 <= w[1].0));
        }
    }
}
