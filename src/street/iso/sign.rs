//! A sign's post and panel, placed: the geometry the stop sign (`stops.rs`) is built from, a post that stands on a piece
//! and a white panel facing the eye on it. (The speed limit has no sign in the drawing: it is set on the street page's
//! Street tab.)

use crate::shared::diagram::{Pt, Shape};

use super::projection::Projection;
use super::solids::{Block, Solid};

/// The panel's width, its height and how high its bottom is over the piece's drawn top, in millimetres. Estimates,
/// pending the drawing of the signs in MTQ Tome V.
const PANEL_W_MM: f64 = 600.0;
const PANEL_H_MM: f64 = 750.0;
const PANEL_FROM_TOP_MM: f64 = 2_000.0;
/// The post's side, in millimetres (an estimate).
const POST_MM: f64 = 75.0;
/// The narrowest the panel is drawn on the screen, in pixels: the sign is a symbol that has to read, not a thing at
/// scale, so when its true size is smaller the whole sign (panel, post and glyph) is drawn larger by one factor.
pub(super) const MIN_PANEL_PX: f64 = 32.0;

/// A sign's post and the panel on it, placed.
pub(super) struct Panel {
    /// The middle of the post across the street, and the foot of the post.
    cx: f64,
    top: f64,
    z0: f64,
    post_w: f64,
    pub(super) bottom: f64,
    pub(super) upper: f64,
    pub(super) half: f64,
    pub(super) height: f64,
}

impl Panel {
    /// A sign whose post stands on the drawn height `top`, `cx` across the street and `z` along it. It is drawn at its
    /// estimated size (`PANEL_W_MM` and the rest) but never under `MIN_PANEL_PX` wide: where that is smaller, the
    /// whole sign is scaled up from its foot by one factor, so it stays legible at any width.
    pub(super) fn at(p: &Projection, cx: f64, top: f64, z: f64) -> Panel {
        let f = (MIN_PANEL_PX / (PANEL_W_MM * p.scale_at(z))).max(1.0);
        let (panel_w, panel_h, post_w) = (PANEL_W_MM * f, PANEL_H_MM * f, POST_MM * f);
        let z0 = z - post_w / 2.0;
        let (bottom, upper) = (top + PANEL_FROM_TOP_MM * f, top + (PANEL_FROM_TOP_MM + PANEL_H_MM) * f);
        Panel { cx, top, z0, post_w, bottom, upper, half: panel_w / 2.0, height: panel_h }
    }

    /// Whether the whole post stands in the reach.
    pub(super) fn in_reach(&self, p: &Projection) -> bool {
        self.z0 >= p.reach.near && self.z0 + self.post_w <= p.reach.far
    }

    /// The post, from its foot to the top of the panel.
    pub(super) fn post(&self, p: &Projection) -> Shape {
        let post = self.post_w / 2.0;
        super::scene::block(
            p,
            &Block { solid: Solid::Post, x: (self.cx - post, self.cx + post), y: (self.top, self.upper), z: (self.z0, self.z0 + self.post_w) },
        )
    }

    /// The white panel facing the eye (a flat rectangle on the screen).
    pub(super) fn panel(&self) -> Shape {
        let (cx, half, z0) = (self.cx, self.half, self.z0);
        let pts = [(cx - half, self.bottom), (cx + half, self.bottom), (cx + half, self.upper), (cx - half, self.upper)];
        Shape::Polygon { pts: pts.iter().map(|&(x, y)| Pt::new(x, y, z0)).collect(), class: "iso-sign-panel".into(), face: None, texture: None }
    }

    /// A rectangle on the panel, `u` across it and `v` up it, each from 0 to 1.
    pub(super) fn rect(&self, class: &str, u: (f64, f64), v: (f64, f64)) -> Shape {
        let x = |u: f64| self.cx + (u - 0.5) * 2.0 * self.half;
        let y = |v: f64| self.bottom + v * self.height;
        let pts = [(x(u.0), y(v.0)), (x(u.1), y(v.0)), (x(u.1), y(v.1)), (x(u.0), y(v.1))];
        Shape::Polygon { pts: pts.iter().map(|&(x, y)| Pt::new(x, y, self.z0)).collect(), class: class.into(), face: None, texture: None }
    }
}

#[cfg(test)]
mod tests {
    use crate::street::iso::iso_svg;
    use crate::street::iso::testing::{count, en};
    use crate::street::model::Editor;

    #[test]
    fn a_street_with_a_speed_draws_no_sign_for_it() {
        let s = Editor::new(0).snapshot();
        let mut e = Editor::from_street(&s, &s, 0);
        assert!(e.set_speed(Some(50)));
        let m = iso_svg(&e.view(), &en(), 1050.0).markup;
        assert_eq!(count(&m, "class=\"iso-sign\""), 0, "{m}");
        assert!(!m.contains("MAXIMUM") && !m.contains("iso-sign-text"), "the speed limit is set on the Street tab, not drawn");
        // Nor does the drawing's label say it: the label says what is drawn.
        let label = crate::street::iso::iso_label(&e.view(), &en(), crate::shared::units::Units::Metres);
        assert!(!label.contains("Speed"), "{label}");
        let fr = crate::i18n_for(crate::shared::i18n::Locale::FrCa);
        let label = crate::street::iso::iso_label(&e.view(), &fr, crate::shared::units::Units::Metres);
        assert!(!label.contains("vitesse"), "{label}");
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
