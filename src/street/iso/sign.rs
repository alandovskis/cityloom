//! The speed limit sign: a post on the street's kerbside edge with a white panel facing the eye, "MAXIMUM" over the limit.

use std::fmt::Write;

use crate::shared::catalogue::{KINDS, Side};
use crate::shared::i18n::{Args, I18n};
use crate::shared::iso::format::esc;
use crate::street::model::{SegView, View};

use super::projection::{Projection, drawn_mm, points};
use super::raised::height_mm;
use super::solids::{Block, Solid, block};

/// The panel's width, its height and how high its bottom is over the piece's drawn top, in millimetres. Estimates,
/// pending the drawing of the speed limit sign in MTQ Tome V (the real panel is taller for its two lines).
const PANEL_W_MM: f64 = 600.0;
const PANEL_H_MM: f64 = 750.0;
const PANEL_FROM_TOP_MM: f64 = 2_000.0;
/// The post's side, in millimetres (an estimate).
const POST_MM: f64 = 75.0;
/// The narrowest the panel is drawn on the screen, in pixels: the sign is a symbol that has to read, not a thing at
/// scale, so when its true size is smaller the whole sign (panel, post and text) is drawn larger by one factor.
pub(super) const MIN_PANEL_PX: f64 = 32.0;
/// How far along the run's near end the sign stands, in millimetres: near, so that it is in front.
const AT_MM: f64 = 2_000.0;

/// The piece the sign stands on: the outermost raised piece (a sidewalk, a median, planting) counting from the edge of
/// the street on the `side` of the road traffic keeps, or, when nothing is raised, the piece at that edge.
fn site(segs: &[SegView], side: Side) -> Option<&SegView> {
    let mut from_edge: Vec<&SegView> = segs.iter().collect();
    from_edge.sort_by_key(|s| s.x_mm);
    if side == Side::Right {
        from_edge.reverse();
    }
    from_edge.iter().copied().find(|s| height_mm(KINDS[s.kind].id) > 0).or(from_edge.first().copied())
}

/// The speed limit sign: a post on the piece at the edge on the side traffic keeps, `AT_MM` into the run so that it is
/// in front, and on it a white panel facing the eye (a flat rectangle on the screen), "MAXIMUM" over the number. It is
/// drawn at its estimated size (`PANEL_W_MM` and the rest) but never under `MIN_PANEL_PX` wide: where that is smaller, the
/// whole sign is scaled up from its foot by one factor, so it stays legible at any width. `None` only without a speed.
pub(super) fn sign(v: &View, segs: &[SegView], p: &Projection, i18n: &I18n) -> Option<String> {
    let kmh = v.speed_kmh?;
    let piece = site(segs, v.side)?;
    let cx = piece.x_mm as f64 + piece.width_mm as f64 / 2.0;
    let top = drawn_mm(height_mm(KINDS[piece.kind].id));
    let z = p.reach.near + AT_MM;
    let f = (MIN_PANEL_PX / (PANEL_W_MM * p.scale_at(z))).max(1.0);
    let (panel_w, panel_h, post_w) = (PANEL_W_MM * f, PANEL_H_MM * f, POST_MM * f);
    let z0 = z - post_w / 2.0;
    let scale = p.scale_at(z0);
    let (bottom, upper) = (top + PANEL_FROM_TOP_MM * f, top + (PANEL_FROM_TOP_MM + PANEL_H_MM) * f);
    let (half, post) = (panel_w / 2.0, post_w / 2.0);
    let mut out = String::from("<g class=\"iso-sign\" aria-hidden=\"true\">");
    block(&mut out, p, &Block { solid: Solid::Post, x: (cx - post, cx + post), y: (top, upper), z: (z0, z0 + post_w) });
    let panel = points(p, &[(cx - half, bottom, z0), (cx + half, bottom, z0), (cx + half, upper, z0), (cx - half, upper, z0)]);
    write!(out, "<polygon class=\"iso-sign-panel\" points=\"{panel}\"/>").unwrap();
    // The words on the panel, centred on it: the first over the second, sized by the panel's width on the screen.
    let wide = panel_w * scale;
    let word = |from_bottom: f64, size: f64, text: &str, out: &mut String| {
        let (x, y) = p.at(cx, bottom + from_bottom, z0);
        write!(
            out,
            "<text class=\"iso-sign-text\" x=\"{}\" y=\"{}\" font-size=\"{}\">{}</text>",
            super::projection::r(x),
            super::projection::r(y),
            super::projection::r(size * wide),
            esc(text)
        )
        .unwrap();
    };
    word(panel_h * 0.74, 0.17, &i18n.tr("iso-sign-maximum", &Args::new()), &mut out);
    word(panel_h * 0.14, 0.46, &kmh.to_string(), &mut out);
    out.push_str("</g>");
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::units::Units;
    use crate::street::iso::testing::{count, en, joined};
    use crate::street::iso::{iso_label, iso_svg};
    use crate::street::model::Editor;

    fn with_speed(side: Side, kmh: Option<u16>) -> View {
        let s = Editor::new(0).snapshot();
        // The side traffic keeps comes from the region: Canada keeps right, the United Kingdom left.
        let mut e = Editor::from_street(&s, &s, if side == Side::Right { 0 } else { 3 });
        e.set_speed(kmh);
        e.view()
    }

    /// The sign's markup: from its group to the end of its number.
    fn sign_of(m: &str) -> &str {
        let at = m.find("<g class=\"iso-sign\"").expect("a sign");
        &m[at..at + m[at..].find("</text></g>").unwrap() + 11]
    }

    /// The x of the middle of the panel.
    fn panel_x(sign: &str) -> f64 {
        let at = sign.find("class=\"iso-sign-panel\" points=\"").unwrap() + 31;
        let xs: Vec<f64> = sign[at..at + sign[at..].find('"').unwrap()].split(' ').map(|p| p.split(',').next().unwrap().parse().unwrap()).collect();
        (xs.iter().cloned().fold(f64::MAX, f64::min) + xs.iter().cloned().fold(f64::MIN, f64::max)) / 2.0
    }

    #[test]
    fn a_street_with_a_speed_shows_one_sign_with_its_number() {
        let m = iso_svg(&with_speed(Side::Right, Some(50)), &en(), 1050.0).markup;
        assert_eq!(count(&m, "class=\"iso-sign\""), 1);
        let sign = sign_of(&m);
        assert!(sign.contains("aria-hidden=\"true\"") && sign.contains(">MAXIMUM<") && sign.contains(">50<"), "{sign}");
        assert!(!sign.contains("data-uid"));
        assert!(!m.contains("NaN"));
    }

    #[test]
    fn a_street_without_a_speed_shows_no_sign() {
        assert_eq!(count(&iso_svg(&with_speed(Side::Right, None), &en(), 1050.0).markup, "iso-sign"), 0);
    }

    #[test]
    fn the_sign_stands_on_the_right_edge_for_right_hand_traffic_and_left_for_left() {
        let centre = 1050.0 / 2.0;
        let right = panel_x(sign_of(&iso_svg(&with_speed(Side::Right, Some(50)), &en(), 1050.0).markup));
        let left = panel_x(sign_of(&iso_svg(&with_speed(Side::Left, Some(50)), &en(), 1050.0).markup));
        assert!(right > centre && left < centre, "{left} {right}");
    }

    /// The width of the panel on the screen, in pixels.
    fn panel_width(sign: &str) -> f64 {
        let at = sign.find("class=\"iso-sign-panel\" points=\"").unwrap() + 31;
        let xs: Vec<f64> = sign[at..at + sign[at..].find('"').unwrap()].split(' ').map(|p| p.split(',').next().unwrap().parse().unwrap()).collect();
        xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min)
    }

    #[test]
    fn the_sign_shows_at_ordinary_widths() {
        for row_mm in [18_000, 30_000] {
            for width in [700.0, 1050.0] {
                let mut v = with_speed(Side::Right, Some(50));
                v.row_mm = row_mm;
                let m = iso_svg(&v, &en(), width).markup;
                assert_eq!(count(&m, "class=\"iso-sign\""), 1, "{row_mm} {width}");
                assert!(panel_width(sign_of(&m)) >= MIN_PANEL_PX - 0.05, "{row_mm} {width}: {}", panel_width(sign_of(&m)));
            }
        }
    }

    #[test]
    fn the_panel_is_floored_at_32_px_on_the_screen() {
        // Small drawing: floored at 32 px.
        let small = iso_svg(&with_speed(Side::Right, Some(50)), &en(), 280.0).markup;
        assert!((MIN_PANEL_PX..MIN_PANEL_PX + 1.0).contains(&panel_width(sign_of(&small))), "{}", panel_width(sign_of(&small)));
    }

    #[test]
    fn with_a_joint_the_sign_is_in_the_selected_run_only() {
        let mut seen = [false; 2];
        for selected in [0, 1] {
            let mut v = joined(false, selected);
            v.speed_kmh = Some(50);
            let far = v.zones.joint.as_ref().unwrap().far;
            let m = iso_svg(&v, &en(), 1050.0).markup;
            let (own, across) = (m.find("<g class=\"iso-section\">").unwrap(), m.find("<g class=\"iso-section iso-across\">").unwrap());
            assert_eq!(count(&m, "class=\"iso-sign\""), 1, "{selected}");
            let at = m.find("class=\"iso-sign\"").unwrap();
            // The neighbour's group is drawn first when it lies far, and after the selected one otherwise.
            assert!(if far { at > own && own > across } else { at > own && at < across }, "{selected}: {own} {across} {at}");
            seen[usize::from(far)] = true;
        }
        assert_eq!(seen, [true, true], "both halves were tried");
    }

    #[test]
    fn the_iso_label_says_the_speed() {
        let v = with_speed(Side::Right, Some(50));
        assert_eq!(iso_label(&v, &en(), Units::Metres), "Sample Street 1 in 2.5D, along a 40 m stretch. 6 segments. Speed limit 50 km/h.");
        let fr = crate::i18n_for(crate::shared::i18n::Locale::FrCa);
        assert_eq!(iso_label(&v, &fr, Units::Metres), "Sample Street 1 en 2,5D, sur un tronçon de 40\u{a0}m. 6 éléments. Limite de vitesse\u{a0}: 50 km/h.");
        assert!(!iso_label(&with_speed(Side::Right, None), &en(), Units::Metres).contains("Speed"));
    }

    #[test]
    fn the_sign_s_text_is_4_5_to_1_on_its_panel() {
        use crate::shared::contrast::{contrast, token};
        let css = include_str!("../../../web/editor.css");
        let (panel, ink) = (token(css, "iso-sign-panel"), token(css, "iso-sign-ink"));
        for (p, i) in [(panel.0.unwrap(), ink.0.unwrap()), (panel.1.unwrap(), ink.1.unwrap())] {
            assert!(contrast(&p, &i) >= 4.5, "{p} {i}");
        }
        assert!(css.contains(".iso-sign-panel {"));
    }
}
