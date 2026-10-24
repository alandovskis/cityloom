//! The surface layer: the top of every roadway piece, at the height of the ground, with its tint and hatch. The
//! markings are painted over it and a raised piece stands over both.

use crate::shared::i18n::I18n;
use crate::street::model::SegView;

use super::projection::Projection;
use super::raised::{Strip, group, piece_with, sliver};
use super::taper::Draw;

/// The strips split into the roadway's (a slab drawn at the ground) and the rest, each keeping its order.
pub(super) fn split(strips: Vec<Strip>) -> (Vec<Strip>, Vec<Strip>) {
    strips.into_iter().partition(|&(top, _, edge)| edge.is_none() && top == 0.0)
}

/// The roadway pieces as one group. Each keeps its `<g class="iso-piece" data-uid>`.
/// Where the run holds a transition (`draw`), the pieces that change taper, and the ones the host lacks open in it.
pub(super) fn layer(p: &Projection, segs: &[SegView], i18n: &I18n, flat: &[Strip], across: bool, draw: Option<&Draw>) -> String {
    let Some(draw) = draw else { return group("<g class=\"iso-surface\">", p, segs, i18n, flat, across) };
    let mut out = String::from("<g class=\"iso-surface\">");
    for &strip in flat {
        let edge = draw.shape_of(&segs[strip.1]).map(|shape| move |z: f64| draw.edges(shape, p.reach, z));
        piece_with(&mut out, p, segs, i18n, strip, across, edge.as_ref().map(|f| f as &dyn Fn(f64) -> (f64, f64)));
    }
    for (kind, shape) in draw.slivers() {
        sliver(&mut out, p, i18n, kind, &|z| draw.edges(shape, p.reach, z));
    }
    out.push_str("</g>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::KINDS;
    use crate::shared::contrast::{contrast, token};
    use crate::street::iso::raised::{height_mm, strips};
    use crate::street::model::Editor;

    #[test]
    fn the_roadway_slabs_are_the_surface_and_everything_else_stands_above_it() {
        for sample in 0..4 {
            let v = Editor::new(sample).view();
            let p = Projection::fit(1050.0, v.row_mm.max(v.total_mm));
            let (flat, standing) = split(strips(&v.segments, &p));
            assert!(flat.iter().all(|&(top, i, edge)| top == 0.0 && edge.is_none() && height_mm(KINDS[v.segments[i].kind].id) == 0), "sample {sample}");
            assert!(standing.iter().all(|&(top, _, edge)| top > 0.0 || edge.is_some()), "sample {sample}");
            assert_eq!(flat.len() + standing.len(), strips(&v.segments, &p).len());
            assert_eq!(flat.len(), v.segments.iter().filter(|s| height_mm(KINDS[s.kind].id) == 0).count());
            // Both keep the order of the strips: from the farthest from the middle of the street to the nearest.
            let order = strips(&v.segments, &p);
            for part in [&flat, &standing] {
                let at: Vec<usize> = part.iter().map(|s| order.iter().position(|o| o == s).unwrap()).collect();
                assert!(at.windows(2).all(|w| w[0] < w[1]), "sample {sample}");
            }
        }
    }

    #[test]
    fn a_material_is_one_colour_in_both_themes() {
        let app = include_str!("../../../web/app.css");
        for m in crate::shared::catalogue::MATERIALS {
            for name in [format!("mat-{}", m.id), format!("mat-{}-line", m.id), format!("mat-{}-alt", m.id)] {
                let (light, dark) = token(app, &name);
                assert!(light.is_some() && light == dark, "--{name}");
            }
        }
    }

    #[test]
    fn the_outlines_are_seen_on_every_material() {
        let css = include_str!("../../../web/editor.css");
        // A halo of the panel's colour keeps the hover outline and the junction's selection box seen on any material.
        for rule in [".iso-piece:hover .iso-top {", "#drawing .iso .sel-box {"] {
            let at = css.find(rule).unwrap_or_else(|| panic!("{rule}"));
            assert!(css[at..at + css[at..].find('}').unwrap()].contains("drop-shadow(0 0 1px var(--panel))"), "{rule}");
        }
    }

    #[test]
    fn the_paint_keeps_3_to_1_against_every_materials_colour_in_both_themes() {
        let (app, editor) = (include_str!("../../../web/app.css"), include_str!("../../../web/editor.css"));
        let (edge_light, edge_dark) = token(editor, "iso-paint-edge");
        for m in crate::shared::catalogue::MATERIALS {
            let (mat, _) = token(app, &format!("mat-{}", m.id));
            let mat = mat.unwrap();
            for paint in ["white", "yellow"] {
                let (pl, pd) = token(editor, &format!("iso-paint-{paint}"));
                // The paint's fill, or the hairline round it, is 3:1 against the surface.
                for (theme, fill, edge) in [("light", pl, &edge_light), ("dark", pd, &edge_dark)] {
                    let best = [fill, edge.clone()].iter().flatten().map(|c| contrast(c, &mat)).fold(1.0, f64::max);
                    assert!(best >= 3.0, "{} {paint} {theme}: {best:.2}", m.id);
                }
            }
        }
    }
}
