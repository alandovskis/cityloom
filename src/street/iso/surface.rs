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

    /// A colour's relative luminance, from `#rrggbb` (WCAG 2.x).
    fn luminance(hex: &str) -> f64 {
        let c = |i: usize| {
            let v = u8::from_str_radix(&hex[1 + 2 * i..3 + 2 * i], 16).unwrap() as f64 / 255.0;
            if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * c(0) + 0.7152 * c(1) + 0.0722 * c(2)
    }

    fn contrast(a: &str, b: &str) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// `(light, dark)` of the token `--name: light-dark(#..., #...)` in a stylesheet.
    fn token(css: &str, name: &str) -> (String, String) {
        let at = css.find(&format!("--{name}: light-dark(")).unwrap_or_else(|| panic!("--{name}")) + name.len() + 15;
        let mut parts = css[at..css[at..].find(')').unwrap() + at].split(", ");
        (parts.next().unwrap().to_string(), parts.next().unwrap().to_string())
    }

    #[test]
    fn the_paint_keeps_3_to_1_against_every_materials_base_colour_in_both_themes() {
        let (app, editor) = (include_str!("../../../web/app.css"), include_str!("../../../web/editor.css"));
        let edge = token(editor, "iso-paint-edge").0;
        for m in crate::shared::catalogue::MATERIALS {
            let (light, dark) = token(app, &format!("mat-{}", m.id));
            for paint in ["white", "yellow"] {
                let (pl, pd) = token(editor, &format!("iso-paint-{paint}"));
                // Light: the paint's fill or the dark hairline round it; dark: the fill alone, which has no hairline.
                assert!(contrast(&pl, &light).max(contrast(&edge, &light)) >= 3.0, "{} {paint} light", m.id);
                assert!(contrast(&pd, &dark) >= 3.0, "{} {paint} dark", m.id);
            }
        }
    }
}
