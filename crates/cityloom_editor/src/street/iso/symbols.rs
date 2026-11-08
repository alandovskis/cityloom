//! The shapes painted on a lane, in millimetres on the ground plane, with no projection: the lane arrow, the
//! reserved-lane diamond with "BUS" in block letters, and the bike symbol with its small arrow(s).
//!
//! **Every size here is an estimate.** The official drawings are in the MTQ Répertoire des dispositifs
//! (https://www.rsr.transports.gouv.qc.ca/Dispositifs/MarquesChaussee.aspx), which could not be read when this was
//! written; the figures were chosen to read at the scale of the drawing and to stay inside a 1.8 m lane. Replace
//! them with the official dimensions when they are had.

use std::f64::consts::TAU;

use super::markings::Symbol;

/// A polygon in millimetres: across the lane, and along the street, from the symbol's origin (the lane's centre
/// line, `ARROW_AT_MM` along the stretch). Along grows away from the viewer.
pub(super) type Shape = Vec<(f64, f64)>;

/// A lane this wide or wider gets its symbols at full size; a narrower lane a proportionally smaller one. With the
/// widest thing in a symbol 1 650 mm across (an estimate, like every size here), a symbol is never wider than 0.92 of
/// its lane.
pub(super) const FULL_WIDTH_MM: f64 = 1_800.0;

/// The scale a lane `width_mm` wide gets.
pub(super) fn fit(width_mm: f64) -> f64 {
    (width_mm / FULL_WIDTH_MM).clamp(0.0, 1.0)
}

// ---- the lane arrow (estimate) ----
// A straight-ahead arrow 4 m long: a 200 mm shaft and a head 1 m wide and 1.2 m long.
/// The size of the lane arrow. Every size in this group is an estimate pending the official drawings in the Répertoire des
/// dispositifs (rsr.transports.gouv.qc.ca).
const AHEAD_HALF_LENGTH_MM: f64 = 2_000.0;
const AHEAD_SHAFT_HALF_MM: f64 = 100.0;
const AHEAD_HEAD_HALF_MM: f64 = 500.0;
const AHEAD_HEAD_LENGTH_MM: f64 = 1_200.0;

// ---- the reserved-lane diamond (estimate) ----
// A lozenge 900 mm wide and 2.4 m long, from 2.5 m before the origin to 100 mm before it.
/// The size of the reserved-lane diamond. Every size in this group is an estimate pending the official drawings in the Répertoire des
/// dispositifs (rsr.transports.gouv.qc.ca).
const DIAMOND: [(f64, f64); 4] = [(0.0, -2_500.0), (450.0, -1_300.0), (0.0, -100.0), (-450.0, -1_300.0)];

// ---- "BUS" (estimate) ----
// Block letters 450 mm wide and 1.2 m tall with 150 mm strokes, 150 mm apart, centred across the lane, starting
// 300 mm beyond the diamond. Each bar is (across from, along from, across to, along to) in the letter's own box.
type Bar = (f64, f64, f64, f64);
/// The size of the letters of "BUS". Every size in this group is an estimate pending the official drawings in the Répertoire des
/// dispositifs (rsr.transports.gouv.qc.ca).
const LETTER_ADVANCE_MM: f64 = 600.0;
const LETTERS_FROM_MM: f64 = 300.0;
const B: [Bar; 6] = [
    (0.0, 0.0, 150.0, 1_200.0),
    (0.0, 1_050.0, 420.0, 1_200.0),
    (0.0, 525.0, 420.0, 675.0),
    (0.0, 0.0, 420.0, 150.0),
    (300.0, 675.0, 450.0, 1_050.0),
    (300.0, 150.0, 450.0, 525.0),
];
const U: [Bar; 3] = [(0.0, 0.0, 150.0, 1_200.0), (300.0, 0.0, 450.0, 1_200.0), (0.0, 0.0, 450.0, 150.0)];
const S: [Bar; 5] =
    [(0.0, 1_050.0, 450.0, 1_200.0), (0.0, 675.0, 150.0, 1_050.0), (0.0, 525.0, 450.0, 675.0), (300.0, 150.0, 450.0, 525.0), (0.0, 0.0, 450.0, 150.0)];

// ---- the bike symbol (estimate) ----
// Two wheels 600 mm across (12-sided rings with a 130 mm tyre) 1.2 m apart, a 100 mm frame between their hubs and
// a 500 mm handlebar over the front hub, with the front wheel toward +along; then a small arrow 1.6 m long, a
// 120 mm shaft and a head 500 mm wide and 600 mm long, beyond the front wheel. A two-way lane has two such arrows
// side by side, opposed.
/// The size of the bike symbol. Every size in this group is an estimate pending the official drawings in the Répertoire des
/// dispositifs (rsr.transports.gouv.qc.ca).
const WHEEL_OUTER_MM: f64 = 300.0;
const WHEEL_INNER_MM: f64 = 170.0;
const FRONT_HUB_MM: f64 = -400.0;
const REAR_HUB_MM: f64 = -1_600.0;
const SMALL_ARROW_FROM_MM: f64 = 100.0;
const SMALL_ARROW_TO_MM: f64 = 1_700.0;
const SMALL_ARROW_SHAFT_HALF_MM: f64 = 60.0;
const SMALL_ARROW_HEAD_HALF_MM: f64 = 250.0;
const SMALL_ARROW_HEAD_LENGTH_MM: f64 = 600.0;
const TWO_WAY_ARROWS_APART_MM: f64 = 300.0;

fn rect(u0: f64, v0: f64, u1: f64, v1: f64) -> Shape {
    vec![(u0, v0), (u1, v0), (u1, v1), (u0, v1)]
}

/// An arrow on the line `u` from `tail` to `tip` along the street.
fn arrow(u: f64, tail: f64, tip: f64, shaft: f64, head: f64, head_length: f64) -> Shape {
    let neck = tip - head_length * (tip - tail).signum();
    vec![(u - shaft, tail), (u + shaft, tail), (u + shaft, neck), (u + head, neck), (u, tip), (u - head, neck), (u - shaft, neck)]
}

/// A 12-sided ring round (`cu`, `cv`), as twelve quadrilaterals.
fn ring(cu: f64, cv: f64, r_out: f64, r_in: f64) -> Vec<Shape> {
    const SIDES: usize = 12;
    (0..SIDES)
        .map(|i| {
            let (a0, a1) = (TAU * i as f64 / SIDES as f64, TAU * (i + 1) as f64 / SIDES as f64);
            let at = |r: f64, a: f64| (cu + r * a.cos(), cv + r * a.sin());
            vec![at(r_in, a0), at(r_out, a0), at(r_out, a1), at(r_in, a1)]
        })
        .collect()
}

fn bus() -> Vec<Shape> {
    let mut out = vec![DIAMOND.to_vec()];
    let letters: [&[Bar]; 3] = [&B, &U, &S];
    for (n, bars) in letters.iter().enumerate() {
        let u = -825.0 + n as f64 * LETTER_ADVANCE_MM;
        out.extend(bars.iter().map(|&(u0, v0, u1, v1)| rect(u + u0, LETTERS_FROM_MM + v0, u + u1, LETTERS_FROM_MM + v1)));
    }
    out
}

fn bike(two_way: bool) -> Vec<Shape> {
    let mut out = ring(0.0, FRONT_HUB_MM, WHEEL_OUTER_MM, WHEEL_INNER_MM);
    out.extend(ring(0.0, REAR_HUB_MM, WHEEL_OUTER_MM, WHEEL_INNER_MM));
    out.push(rect(-50.0, REAR_HUB_MM, 50.0, FRONT_HUB_MM));
    out.push(rect(-250.0, FRONT_HUB_MM - 70.0, 250.0, FRONT_HUB_MM + 70.0));
    let small = |u: f64, forward: bool| {
        let (tail, tip) = if forward { (SMALL_ARROW_FROM_MM, SMALL_ARROW_TO_MM) } else { (SMALL_ARROW_TO_MM, SMALL_ARROW_FROM_MM) };
        arrow(u, tail, tip, SMALL_ARROW_SHAFT_HALF_MM, SMALL_ARROW_HEAD_HALF_MM, SMALL_ARROW_HEAD_LENGTH_MM)
    };
    if two_way {
        out.push(small(-TWO_WAY_ARROWS_APART_MM, true));
        out.push(small(TWO_WAY_ARROWS_APART_MM, false));
    } else {
        out.push(small(0.0, true));
    }
    out
}

/// The polygons of `symbol` on a lane `width_mm` wide, scaled by `fit` and, for an arrow or a bike that runs toward
/// the viewer, mirrored along the street. "BUS" is turned half round for a lane that runs toward the viewer, so it reads the way the lane runs.
pub(super) fn shapes(symbol: Symbol, width_mm: f64) -> Vec<Shape> {
    let (mut out, flip) = match symbol {
        Symbol::Ahead { toward } => {
            (vec![arrow(0.0, -AHEAD_HALF_LENGTH_MM, AHEAD_HALF_LENGTH_MM, AHEAD_SHAFT_HALF_MM, AHEAD_HEAD_HALF_MM, AHEAD_HEAD_LENGTH_MM)], toward)
        }
        Symbol::Bus { toward } => (bus(), toward),
        Symbol::Bike { two_way, toward } => (bike(two_way), toward && !two_way),
    };
    let k = fit(width_mm);
    let along = if flip { -k } else { k };
    // The word reads the way the lane runs, so a bus lane toward the viewer is turned half round, not mirrored.
    let across = if flip && matches!(symbol, Symbol::Bus { .. }) { -k } else { k };
    for shape in &mut out {
        for pt in shape.iter_mut() {
            *pt = (pt.0 * across, pt.1 * along);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTHS: [f64; 4] = [1_200.0, 1_800.0, 3_300.0, 4_000.0];

    fn all() -> Vec<Symbol> {
        vec![
            Symbol::Ahead { toward: false },
            Symbol::Ahead { toward: true },
            Symbol::Bus { toward: false },
            Symbol::Bus { toward: true },
            Symbol::Bike { two_way: true, toward: false },
            Symbol::Bike { two_way: false, toward: false },
            Symbol::Bike { two_way: false, toward: true },
        ]
    }

    fn extent(shapes: &[Shape]) -> ((f64, f64), (f64, f64)) {
        let pts = || shapes.iter().flatten();
        let fold = |f: fn(&(f64, f64)) -> f64| pts().map(f).fold((f64::MAX, f64::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
        (fold(|p| p.0), fold(|p| p.1))
    }

    #[test]
    fn a_lane_wide_enough_gets_the_symbol_at_full_size_and_a_narrower_one_a_smaller_symbol() {
        assert_eq!(fit(3_300.0), 1.0);
        assert_eq!(fit(1_800.0), 1.0);
        assert_eq!(fit(900.0), 0.5);
        assert_eq!(fit(0.0), 0.0);
        let full = shapes(Symbol::Ahead { toward: false }, 3_300.0);
        let half = shapes(Symbol::Ahead { toward: false }, 900.0);
        assert_eq!(extent(&full).1, (-2_000.0, 2_000.0));
        assert_eq!(extent(&half).1, (-1_000.0, 1_000.0));
    }

    #[test]
    fn the_lane_arrow_points_the_way_the_lane_runs() {
        let tip = |toward: bool| {
            let s = &shapes(Symbol::Ahead { toward }, 3_300.0)[0];
            *s.iter().find(|p| p.0 == 0.0 && p.1.abs() == 2_000.0).expect("a tip on the centre line")
        };
        assert_eq!(tip(false).1, 2_000.0, "away from the viewer, up the street");
        assert_eq!(tip(true).1, -2_000.0, "toward the viewer");
    }

    #[test]
    fn the_bus_symbol_is_a_diamond_with_the_letters_b_u_s_beyond_it() {
        let s = shapes(Symbol::Bus { toward: false }, 3_300.0);
        // The diamond, then B (6 bars), U (3), S (5).
        assert_eq!(s.len(), 1 + 6 + 3 + 5);
        assert_eq!(s[0].len(), 4);
        let diamond = extent(&s[..1]);
        let letters = extent(&s[1..]);
        assert!(letters.1.0 > diamond.1.1, "the letters start beyond the diamond: {letters:?} {diamond:?}");
        // The three letters are left to right: the U is centred.
        assert!(letters.0.0 < 0.0 && letters.0.1 > 0.0 && (letters.0.0 + letters.0.1).abs() < 1e-9);
    }

    #[test]
    fn the_bus_symbol_is_turned_half_round_for_a_lane_toward_the_viewer() {
        let away = shapes(Symbol::Bus { toward: false }, 3_300.0);
        let toward = shapes(Symbol::Bus { toward: true }, 3_300.0);
        assert_eq!(away.len(), toward.len());
        for (a, t) in away.iter().zip(&toward) {
            // Every corner is turned through 180 degrees about the symbol's origin, not mirrored along the street.
            assert!(a.iter().zip(t).all(|(a, t)| (a.0 + t.0).abs() < 1e-9 && (a.1 + t.1).abs() < 1e-9));
        }
        // The diamond leads for the lane that runs away and trails for the one that runs toward the viewer.
        assert!(extent(&away[..1]).1.1 < extent(&away[1..]).1.0);
        assert!(extent(&toward[..1]).1.0 > extent(&toward[1..]).1.1);
    }

    #[test]
    fn the_bike_symbol_has_two_wheels_a_frame_a_handlebar_and_one_arrow_or_two_opposed_ones() {
        let one = shapes(Symbol::Bike { two_way: false, toward: false }, 3_300.0);
        let two = shapes(Symbol::Bike { two_way: true, toward: false }, 3_300.0);
        // Two wheels of 12 pieces, the frame, the handlebar, and the arrow(s).
        assert_eq!(one.len(), 24 + 2 + 1);
        assert_eq!(two.len(), 24 + 2 + 2);
        // The one-way arrow is beyond the bike (away) and mirrored with it (toward).
        let away = shapes(Symbol::Bike { two_way: false, toward: false }, 3_300.0);
        let toward = shapes(Symbol::Bike { two_way: false, toward: true }, 3_300.0);
        let arrow_end = |s: &Vec<Shape>| s.last().unwrap().iter().map(|p| p.1).fold(0.0_f64, |a, b| if b.abs() > a.abs() { b } else { a });
        assert_eq!(arrow_end(&away), 1_700.0);
        assert_eq!(arrow_end(&toward), -1_700.0);
        // Two-way ignores `toward`.
        assert_eq!(shapes(Symbol::Bike { two_way: true, toward: true }, 3_300.0), two);
    }

    #[test]
    fn no_symbol_is_wider_than_its_lane() {
        for symbol in all() {
            for w in WIDTHS {
                let ((u0, u1), _) = extent(&shapes(symbol, w));
                assert!(u0 >= -w / 2.0 && u1 <= w / 2.0, "{symbol:?} in {w}: {u0}..{u1}");
            }
        }
    }

    #[test]
    fn every_symbol_lies_within_the_stretch_of_the_street_it_is_laid_on() {
        for symbol in all() {
            for w in WIDTHS {
                let (_, (v0, v1)) = extent(&shapes(symbol, w));
                assert!(v0 >= -2_500.0 && v1 <= 2_500.0, "{symbol:?} in {w}: {v0}..{v1}");
                assert!(shapes(symbol, w).iter().flatten().all(|p| p.0.is_finite() && p.1.is_finite()));
            }
        }
    }
}
