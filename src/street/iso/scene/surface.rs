//! The surface layer as shapes: the top of every roadway piece, one group per piece.

use crate::shared::catalogue::{KINDS, MATERIALS, kind_key};
use crate::shared::diagram::{Layer, LayerKind, Pt, Shape, Texture};
use crate::shared::i18n::{Args, I18n};
use crate::shared::iso::ground::{joints, tile_mm};
use crate::street::model::SegView;

use crate::street::iso::curbside::{Over, flat_parts};
use crate::street::iso::joints::runs;
use crate::street::iso::projection::{Projection, Reach, drawn_mm};
use crate::street::iso::raised::{Strip, height_mm};
use crate::street::iso::taper::Draw;

/// A textured top over the projection's reach, `h` high, its edges across given along the street (`edge`), then the
/// regular joints of its material over it when `joints_too` (only on a top with straight edges).
pub(in crate::street::iso) fn top(p: &Projection, id: &str, m: &str, h: f64, edge: &dyn Fn(f64) -> (f64, f64), joints_too: bool) -> Vec<Shape> {
    let (n, l) = (p.reach.near, p.reach.far);
    let ((x0, x1), (f0, f1)) = (edge(n), edge(l));
    let pts = vec![Pt::new(x0, h, n), Pt::new(x1, h, n), Pt::new(f1, h, l), Pt::new(f0, h, l)];
    let mut out = vec![Shape::Polygon {
        pts,
        class: format!("iso-top k-{id} mat-{m}"),
        face: None,
        texture: Some(Texture { material: m.to_string(), tile_mm: tile_mm(m) }),
    }];
    if let Some(j) = joints(m).filter(|_| joints_too) {
        let runs = runs(p, &j, x0, x1, h);
        if !runs.is_empty() {
            out.push(Shape::Joints { runs, class: format!("joints j-{m}") });
        }
    }
    out
}

fn piece(p: &Projection, segs: &[SegView], i18n: &I18n, strip: Strip, across: bool, draw: Option<&Draw>, shown: &[Over]) -> Shape {
    let (_, i, _) = strip;
    let s = &segs[i];
    let id = KINDS[s.kind].id;
    let taper = draw.and_then(|d| d.shape_of(s).map(|shape| move |z: f64| d.edges(shape, p.reach, z)));
    let taper = taper.as_ref().map(|f| f as &dyn Fn(f64) -> (f64, f64));
    let (x0, x1) = (s.x_mm as f64, (s.x_mm + s.width_mm) as f64);
    let shapes = match (taper, flat_parts(segs, i, p.reach, shown)) {
        (taper, Some(parts)) => parts
            .iter()
            .flat_map(|part| {
                let sub = p.over(Reach { near: part.along.0, far: part.along.1 });
                let id = KINDS[part.kind].id;
                match taper {
                    Some(edge) => top(&sub, id, part.material, 0.0, edge, false),
                    None => top(&sub, id, part.material, 0.0, &|_| (x0, x1), true),
                }
            })
            .collect(),
        (Some(edge), None) => top(p, id, s.material, drawn_mm(height_mm(id)), edge, false),
        (None, None) => top(p, id, s.material, drawn_mm(height_mm(id)), &|_| (x0, x1), true),
    };
    Shape::Group {
        class: if across { "iso-piece iso-across" } else { "iso-piece" }.into(),
        uid: if across { None } else { Some(s.uid) },
        data: None,
        hidden: false,
        title: Some(i18n.tr(&kind_key(id), &Args::new())),
        shapes,
    }
}

/// The roadway pieces `flat`, each a group of its own with its piece's name for a title and, outside the neighbour's
/// run (`across`), its uid. Where the run holds a transition (`draw`), the pieces that change taper, and the ones the
/// host lacks open in it, each in an `iso-taper` group with no uid.
pub(in crate::street::iso) fn surface(
    p: &Projection,
    segs: &[SegView],
    i18n: &I18n,
    flat: &[Strip],
    across: bool,
    draw: Option<&Draw>,
    shown: &[Over],
) -> Layer {
    let mut shapes: Vec<Shape> = flat.iter().map(|&strip| piece(p, segs, i18n, strip, across, draw, shown)).collect();
    if let Some(draw) = draw {
        for (kind, shape) in draw.slivers() {
            let k = &KINDS[kind];
            let edge = |z: f64| draw.edges(shape, p.reach, z);
            shapes.push(Shape::Group {
                class: "iso-piece iso-taper".into(),
                uid: None,
                data: None,
                hidden: false,
                title: Some(i18n.tr(&kind_key(k.id), &Args::new())),
                shapes: top(p, k.id, MATERIALS[k.materials[0]].id, drawn_mm(height_mm(k.id)), &edge, false),
            });
        }
    }
    Layer { kind: LayerKind::Surface, shapes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::diagram::render::layer_markup;
    use crate::street::iso::raised::strips;
    use crate::street::iso::scene::testing::each_run;
    use crate::street::iso::surface::split;
    use crate::street::iso::testing::en;
    use crate::street::model::Editor;

    #[test]
    fn the_surface_layer_holds_a_group_per_roadway_piece_and_a_sliver_per_piece_the_host_lacks() {
        let i18n = en();
        let (mut runs, mut slivers) = (0, 0);
        each_run(|name, p, segs, across, draw, shown| {
            let (flat, _) = split(strips(segs, p));
            let l = surface(p, segs, &i18n, &flat, across, draw, shown);
            assert_eq!(l.kind, LayerKind::Surface, "{name}");
            let extra = draw.map_or(0, |d| d.slivers().count());
            slivers += extra;
            assert_eq!(l.shapes.len(), flat.len() + extra, "{name}");
            let m = layer_markup(&l, p);
            assert!(m.starts_with("<g class=\"iso-surface\">") && m.ends_with("</g>"), "{name}");
            assert_eq!(m.matches("<g class=\"iso-piece iso-taper\">").count(), extra, "{name}");
            // The neighbour's pieces carry no uid and the selected zone's do.
            assert_eq!(m.contains("data-uid"), !across && !flat.is_empty(), "{name}");
            runs += 1;
        });
        assert!(runs >= 30, "{runs} runs");
        assert!(slivers > 0, "some sample opens a lane in a taper");
    }

    #[test]
    fn a_design_wider_than_the_street_keeps_every_point_inside_the_drawing_and_has_no_nan() {
        let v = crate::street::iso::scene::testing::wide();
        for width in [600.0, 1050.0, 1800.0] {
            let p = Projection::fit(width, v.row_mm.max(v.total_mm)).over(Reach::STRETCH);
            let (flat, _) = split(strips(&v.segments, &p));
            let l = surface(&p, &v.segments, &en(), &flat, false, None, &[]);
            let m = crate::shared::diagram::render::layer_markup(&l, &p);
            assert!(!m.contains("NaN") && !m.contains("inf"), "{width}");
            let all = crate::street::iso::testing::xs(&m);
            assert!(!all.is_empty());
            assert!(all.iter().all(|x| *x >= 0.0 && *x <= p.width), "{width}: {all:?}");
        }
    }

    #[test]
    fn an_empty_street_gives_an_empty_surface_layer() {
        let mut e = Editor::new(0);
        while let Some(uid) = e.view().segments.first().map(|s| s.uid) {
            assert!(e.remove(uid));
        }
        let v = e.view();
        let p = Projection::fit(1050.0, v.row_mm.max(v.total_mm)).over(Reach::STRETCH);
        let l = surface(&p, &v.segments, &en(), &[], false, None, &[]);
        assert!(l.shapes.is_empty());
        assert_eq!(layer_markup(&l, &p), "<g class=\"iso-surface\"></g>");
    }
}
