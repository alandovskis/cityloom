//! A raised table or crossing as shapes: over each roadway piece the plateau (the piece's material, at the table's
//! height), a ramp at each end sloping from the roadway to it, and on a crossing the bars of a zebra. The element is one
//! group, `iso-table`, with its name for a title and no uid: the strip below the drawing is where it is chosen.

use crate::shared::catalogue::{KINDS, is_roadway};
use crate::shared::diagram::{Face, Pt, Shape};
use crate::shared::i18n::{Args, I18n};
use crate::street::model::{CalmingKind, SegView};

use super::surface::top;
use crate::street::iso::calming::{Laid, Table, ZEBRA_MM};
use crate::street::iso::projection::{Projection, Reach};

/// The ramp of the piece spanning `x0..x1` from `z0` to `z1`, at the table's heights there.
fn ramp(t: &Table, kind: &str, material: &str, x: (f64, f64), z: (f64, f64)) -> Shape {
    let (y0, y1) = (t.height_at(z.0), t.height_at(z.1));
    Shape::Plane {
        pts: vec![Pt::new(x.0, y0, z.0), Pt::new(x.1, y0, z.0), Pt::new(x.1, y1, z.1), Pt::new(x.0, y1, z.1)],
        class: format!("iso-ramp k-{kind} mat-{material}"),
        face: Face::Ramp,
        labelled: false,
        hatch: Some(format!("h-{kind}")),
    }
}

/// The bars of a zebra across `x`, along `z`, on the plateau.
fn zebra(x: (f64, f64), z: (f64, f64)) -> Vec<Shape> {
    let h = Table::height();
    let mut out = Vec::new();
    let mut at = x.0 + ZEBRA_MM / 2.0;
    while at < x.1 {
        let end = (at + ZEBRA_MM).min(x.1);
        if end - at >= ZEBRA_MM / 4.0 {
            out.push(Shape::Polygon {
                pts: vec![Pt::new(at, h, z.0), Pt::new(end, h, z.0), Pt::new(end, h, z.1), Pt::new(at, h, z.1)],
                class: "iso-zebra".into(),
                face: None,
                texture: None,
            });
        }
        at += 2.0 * ZEBRA_MM;
    }
    out
}

/// The element `t` laid over the pieces `laid` lifts.
pub(in crate::street::iso) fn table(p: &Projection, segs: &[SegView], i18n: &I18n, laid: &Laid, t: &Table) -> Shape {
    let h = Table::height();
    let plateau = (t.plateau.0.max(t.along.0), t.plateau.1.min(t.along.1));
    let mut shapes = Vec::new();
    for &i in &laid.pieces {
        let s = &segs[i];
        let (id, x) = (KINDS[s.kind].id, (s.x_mm as f64, (s.x_mm + s.width_mm) as f64));
        for z in [(t.along.0, t.plateau.0.min(t.along.1)), (t.plateau.1.max(t.along.0), t.along.1)] {
            if z.1 > z.0 {
                shapes.push(ramp(t, id, s.material, x, z));
            }
        }
        if plateau.1 > plateau.0 {
            let sub = p.over(Reach { near: plateau.0, far: plateau.1 });
            shapes.extend(top(&sub, id, s.material, h, &|_| x, false));
            if t.kind == CalmingKind::Crossing && is_roadway(s.kind) {
                shapes.extend(zebra(x, plateau));
            }
        }
    }
    Shape::Group { class: "iso-table".into(), uid: None, data: None, hidden: false, title: Some(i18n.tr(t.kind.name_key(), &Args::new())), shapes }
}
