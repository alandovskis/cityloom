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

    /// `(light, dark)` of the token `--name` in a stylesheet: `light-dark(#.., #..)`, or one colour for both themes.
    /// A value that is no `#rrggbb` (`transparent`) is `None`.
    fn token(css: &str, name: &str) -> (Option<String>, Option<String>) {
        let at = css.find(&format!("--{name}: ")).unwrap_or_else(|| panic!("--{name}")) + name.len() + 4;
        let value = &css[at..at + css[at..].find(';').unwrap()];
        let hex = |v: &str| Some(v.trim().to_string()).filter(|v| v.starts_with('#') && v.len() == 7);
        match value.strip_prefix("light-dark(") {
            Some(rest) => {
                let mut parts = rest.trim_end_matches(')').split(", ");
                (hex(parts.next().unwrap()), hex(parts.next().unwrap()))
            }
            None => (hex(value), hex(value)),
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
        // A halo of the panel's colour keeps the hover and the selection outlines seen on any material.
        for rule in [".iso-piece:hover .iso-top {", ".iso-sel,\n#drawing .iso .sel-box {"] {
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
