//! The raised layer as shapes: every strip standing above the roadway with its faces, and the curbside uses lifted into
//! strips of their own, in the order the eye needs among themselves.

use crate::shared::catalogue::{KINDS, kind_key};
use crate::shared::diagram::{Face, Layer, LayerKind, Pt, Shape};
use crate::shared::i18n::{Args, I18n};
use crate::street::model::SegView;

use super::surface::top;
use crate::street::iso::curbside::{self, Lift};
use crate::street::iso::order::{self, Bx};
use crate::street::iso::projection::{Projection, Reach, drawn_mm};
use crate::street::iso::raised::{Strip, beside, curb_span, curb_top_mm, height_mm, inner_face, strip_box};
use crate::street::iso::taper::Draw;

/// A face of a piece: a quadrilateral given by its corners, in the tint of the kind `kind`, or of a curb when there is
/// none, with the hatch `hatch` over it.
fn plane(corners: [(f64, f64, f64); 4], kind: Option<&str>, face: Face, hatch: &str) -> Shape {
    let name = match face {
        Face::Top => "top",
        Face::Front => "front",
        Face::Side => "side",
        Face::Ramp => "ramp",
    };
    Shape::Plane {
        pts: corners.map(|(x, y, z)| Pt::new(x, y, z)).to_vec(),
        class: match kind {
            Some(k) => format!("iso-{name} k-{k}"),
            None => "iso-curb".to_string(),
        },
        face,
        labelled: kind.is_none(),
        hatch: Some(hatch.to_string()),
    }
}

/// The piece at `i` as a strip of kind `id` and material `m` over `along` of the reach: its front, the side face the eye
/// sees where the piece beside it is lower, and its top. `edge` gives the piece's edges across where a transition moves
/// them.
fn slab(p: &Projection, segs: &[SegView], i: usize, id: &str, m: &str, along: (f64, f64), edge: Option<&dyn Fn(f64) -> (f64, f64)>) -> Vec<Shape> {
    let s = &segs[i];
    let (n, l) = along;
    let (x0, x1) = edge.map_or((s.x_mm as f64, (s.x_mm + s.width_mm) as f64), |e| e(n));
    let h = drawn_mm(height_mm(id));
    let hatch = format!("h-{id}");
    let mut out = Vec::new();
    if h > 0.0 {
        out.push(plane([(x0, 0.0, n), (x1, 0.0, n), (x1, h, n), (x0, h, n)], Some(id), Face::Front, &hatch));
        if let Some(right) = inner_face(p, x0, x1) {
            let (far_x0, far_x1) = edge.map_or((x0, x1), |e| e(l));
            let (x, xl, low) = if right { (x1, far_x1, beside(segs, i, right)) } else { (x0, far_x0, beside(segs, i, right)) };
            if low < h {
                out.push(plane([(x, low, n), (xl, low, l), (xl, h, l), (x, h, n)], Some(id), Face::Side, &hatch));
            }
        }
    }
    let sub = p.over(Reach { near: n, far: l });
    match edge {
        Some(edge) => out.extend(top(&sub, id, m, h, edge, false)),
        None => out.extend(top(&sub, id, m, h, &|_| (x0, x1), true)),
    }
    out
}

/// The curb of the piece at `i` along one edge (`right`): a stripe on the piece's top when as high as it, else a block
/// with its own front, side and top.
fn curb(p: &Projection, segs: &[SegView], i: usize, right: bool) -> Vec<Shape> {
    let s = &segs[i];
    let Some(c) = s.curb else { return Vec::new() };
    let id = KINDS[s.kind].id;
    let (h, n, l) = (drawn_mm(height_mm(id)), p.reach.near, p.reach.far);
    let top = curb_top_mm(c, id);
    let (x0, x1) = curb_span(s, c, right);
    let hatch = format!("c-{c}");
    let mut out = Vec::new();
    if top > h {
        out.push(plane([(x0, 0.0, n), (x1, 0.0, n), (x1, top, n), (x0, top, n)], None, Face::Front, &hatch));
        if let Some(face_right) = inner_face(p, x0, x1) {
            // A block's face on the piece's own side stands on the piece; its other face stands on what is beside it.
            let (x, low) = (if face_right { x1 } else { x0 }, if face_right == right { beside(segs, i, right) } else { h });
            out.push(plane([(x, low, n), (x, low, l), (x, top, l), (x, top, n)], None, Face::Side, &hatch));
        }
    }
    out.push(plane([(x0, top, n), (x1, top, n), (x1, top, l), (x0, top, l)], None, Face::Top, &hatch));
    out
}

fn group(class: &str, uid: Option<u32>, name: String, shapes: Vec<Shape>) -> Shape {
    Shape::Group { class: class.into(), uid, data: None, hidden: false, title: Some(name), shapes }
}

/// A strip of the street as a group of its own with the piece's name for hovering: its slab, or one of its curbs. A
/// piece of the section across the boundary (`across`) carries no uid.
fn strip(p: &Projection, segs: &[SegView], i18n: &I18n, (_, i, edge): Strip, across: bool) -> Shape {
    let s = &segs[i];
    let name = i18n.tr(&kind_key(KINDS[s.kind].id), &Args::new());
    let shapes = match edge {
        None => slab(p, segs, i, KINDS[s.kind].id, s.material, (p.reach.near, p.reach.far), None),
        Some(right) => curb(p, segs, i, right),
    };
    group(if across { "iso-piece iso-across" } else { "iso-piece" }, if across { None } else { Some(s.uid) }, name, shapes)
}

/// A curbside use lifted into a strip of its own, hovered by its name; on a tapering piece it follows the piece's edges.
fn lift(p: &Projection, segs: &[SegView], i18n: &I18n, l: &Lift, taper: Option<&Draw>) -> Shape {
    let name = i18n.tr(l.name_key, &Args::new());
    let edge = curbside::edges(segs, l, p, taper);
    let shapes = slab(p, segs, l.piece, KINDS[l.kind].id, l.material, l.along, edge.as_ref().map(|e| e as &dyn Fn(f64) -> (f64, f64)));
    group("iso-piece", Some(segs[l.piece].uid), name, shapes)
}

/// The strips `standing` above the roadway and the curbside uses `lifted` into strips, in the order they are painted in
/// among themselves (`order::paint_order`): a strip nearer the middle of the street covers the one beyond it.
pub(in crate::street::iso) fn raised(
    p: &Projection,
    segs: &[SegView],
    i18n: &I18n,
    standing: &[Strip],
    lifted: &[Lift],
    across: bool,
    taper: Option<&Draw>,
) -> Layer {
    let boxes: Vec<(Bx, bool)> =
        standing.iter().map(|&s| (strip_box(segs, s, p.reach), true)).chain(lifted.iter().map(|l| (curbside::lift_box(segs, l, p, taper), false))).collect();
    let shapes = order::paint_order(&boxes, p.centre_mm)
        .into_iter()
        .map(|k| match standing.get(k) {
            Some(&s) => strip(p, segs, i18n, s, across),
            None => lift(p, segs, i18n, &lifted[k - standing.len()], taper),
        })
        .collect();
    Layer { kind: LayerKind::Raised, shapes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::iso::raised::strips;
    use crate::street::iso::scene::testing::each_run;
    use crate::street::iso::surface::split;
    use crate::street::iso::testing::en;

    #[test]
    fn the_raised_layer_holds_a_group_per_standing_strip_and_per_lifted_use() {
        let i18n = en();
        let (mut runs, mut lifts) = (0, 0);
        each_run(|name, p, segs, across, draw, shown| {
            let (_, standing) = split(strips(segs, p));
            let lifted = curbside::lifted(shown);
            lifts += lifted.len();
            let layer = raised(p, segs, &i18n, &standing, &lifted, across, draw);
            assert_eq!(layer.kind, LayerKind::Raised, "{name}");
            assert_eq!(layer.shapes.len(), standing.len() + lifted.len(), "{name}");
            assert!(layer.shapes.iter().all(|s| matches!(s, Shape::Group { title: Some(_), .. })), "{name}: every strip is named for hovering");
            runs += 1;
        });
        assert!(runs >= 30, "{runs} runs");
        assert!(lifts > 0, "no sample lifts a use");
    }

    #[test]
    fn a_joint_with_a_taper_draws_the_taper_in_the_host_sections_raised_layer_as_before() {
        use crate::shared::catalogue::kind_index;
        use crate::shared::diagram::render::layer_markup;
        use crate::street::iso::span_mm;
        use crate::street::iso::testing::{joined, joined_from_last};
        use crate::street::model::Use;
        let i18n = en();
        let mut tapered = 0;
        for v in [joined(true, 0), joined(true, 1), joined_from_last(true, 0), joined_from_last(true, 1)] {
            let j = v.zones.joint.as_ref().expect("a joint");
            let t = j.taper.as_ref().expect("a transition");
            let l = crate::street::iso::projection::STRETCH_MM;
            let (near, far) = (Reach { near: 0.0, far: l / 2.0 }, Reach { near: l / 2.0, far: l });
            let (own, across) = if j.far { (near, far) } else { (far, near) };
            // The host's run holds the transition: the selected zone's when it is the host, else the neighbour's.
            let (segs, reach, is_across, draw) =
                if t.host_is_selected { (&v.segments, own, false, Draw::new(t, j.far)) } else { (&j.segments, across, true, Draw::new(t, !j.far)) };
            let p = Projection::fit(1050.0, span_mm(&v)).over(reach);
            let i = segs.iter().position(|s| draw.shape_of(s).is_some()).expect("a piece that tapers");
            let on = curbside::Over {
                piece: i,
                along: (reach.near + 1_000.0, reach.far - 1_000.0),
                raised: true,
                kind: kind_index("planting").unwrap(),
                material: "grass",
                name_key: Use::Planting.name_key(),
            };
            let (_, standing) = split(strips(segs, &p));
            let lifted = curbside::lifted(&[on]);
            let layer = raised(&p, segs, &i18n, &standing, &lifted, is_across, Some(&draw));
            // The lifted strip follows the tapering piece: without the transition it is drawn differently.
            let plain = layer_markup(&raised(&p, segs, &i18n, &standing, &lifted, is_across, None), &p);
            assert_ne!(layer_markup(&layer, &p), plain);
            tapered += 1;
        }
        assert_eq!(tapered, 4);
    }
}
