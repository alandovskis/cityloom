//! The city's places as GeoJSON for the map: each street along its real centreline, each junction a point
//! (with `radius_m`), and where streets meet what joins them: a disc where two meet (a point with `joint_m`), a
//! shape over their ends where more do (a polygon).

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::city::model::{CityView, EdgeView, NodeView};
use crate::city::projection::Projection;
use crate::restriction::model::{LonLat, Restriction};
use crate::shared::catalogue::Side;

/// How a place stands: needing attention beats having been changed.
fn status(ok: bool, edited: bool) -> &'static str {
    if !ok {
        "bad"
    } else if edited {
        "changed"
    } else {
        "ok"
    }
}

/// A coordinate to six decimals of a degree: about a tenth of a metre.
fn r6(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

fn position(p: &Projection, x_mm: i32, y_mm: i32) -> [f64; 2] {
    let [lon, lat] = p.lon_lat(x_mm, y_mm);
    [r6(lon), r6(lat)]
}

/// The streets and junctions of the city as a GeoJSON feature collection: each street a line, each junction a point
/// with `radius_m`, and each joint (`joints`) a point with `joint_m` or a polygon, with the `hot` of the place it is drawn as. A street in `conflicts` (by uid) has what
/// the rule of an area it is inside does not allow.
pub fn places(v: &CityView, p: &Projection, conflicts: &BTreeSet<u32>) -> String {
    let mut features: Vec<Value> = Vec::with_capacity(v.edges.len() + v.nodes.len());
    for e in &v.edges {
        features.push(json!({
            "type": "Feature",
            "properties": { "hot": format!("s-{}", e.uid), "status": status(e.ok, e.edited), "width_m": e.row_mm as f64 / 1000.0, "class": e.class.id(), "conflict": conflicts.contains(&e.uid) },
            "geometry": { "type": "LineString", "coordinates": e.shape_mm.iter().map(|q| position(p, q[0], q[1])).collect::<Vec<_>>() },
        }));
    }
    // what each junction's ground is, for its press target (`target_m`)
    let mut grounds: BTreeMap<u32, Ground> = BTreeMap::new();
    for (whose, join) in joints(v) {
        let mut properties = match whose {
            Whose::Street(e) => json!({ "hot": format!("s-{}", e.uid), "status": status(e.ok, e.edited), "conflict": conflicts.contains(&e.uid) }),
            Whose::Junction(n) => json!({ "hot": format!("j-{}", n.uid), "status": status(n.ok, n.edited), "conflict": false, "ground": true }),
            Whose::Meeting { node, .. } => json!({ "hot": format!("m-{node}"), "status": "ok", "conflict": false }),
        };
        let (geometry, ground) = match join {
            Join::Disc { end: [x, y], row_mm } => {
                let joint_m = row_mm as f64 / 2000.0;
                properties["joint_m"] = json!(joint_m);
                (json!({ "type": "Point", "coordinates": position(p, x, y) }), Ground::Disc(joint_m))
            }
            Join::Middle { ring } => {
                let coordinates: Vec<[f64; 2]> = ring.iter().map(|q| position(p, q[0].round() as i32, q[1].round() as i32)).collect();
                (json!({ "type": "Polygon", "coordinates": [coordinates] }), Ground::Shape)
            }
        };
        if let Whose::Junction(n) = whose {
            grounds.insert(n.uid, ground);
        }
        features.push(json!({ "type": "Feature", "properties": properties, "geometry": geometry }));
    }
    for n in v.nodes.iter().filter(|n| n.junction) {
        features.push(json!({
            "type": "Feature",
            "properties": { "hot": format!("j-{}", n.uid), "status": status(n.ok, n.edited), "radius_m": target_m(n, grounds.get(&n.uid)) },
            "geometry": { "type": "Point", "coordinates": position(p, n.x_mm, n.y_mm) },
        }));
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

/// What a junction's ground is drawn as.
enum Ground {
    /// A disc this many metres round.
    Disc(f64),
    /// A shape, pressed for itself (`places-ground`).
    Shape,
}

/// How far round its point a junction is pressed (`radius_m`, never less than `style.rs`'s floor): its own radius,
/// or its ground's where that is a bigger disc; nothing where its ground is a shape, which is pressed for itself, so
/// a wide street there past a narrow one's corners is pressed as the street.
fn target_m(n: &NodeView, ground: Option<&Ground>) -> f64 {
    let own = n.radius_mm as f64 / 1000.0;
    match ground {
        None => own,
        Some(Ground::Disc(r)) => own.max(*r),
        Some(Ground::Shape) => 0.0,
    }
}

/// Whose a joint is: it is drawn in that place's colour and state.
enum Whose<'a> {
    Street(&'a EdgeView),
    Junction(&'a NodeView),
    /// The shape where three streets or more meet that are not a junction, by its node's uid: it lies under every
    /// street's end (`places-middle`, under the streets' lines) and is no one street's, so it takes no street's state
    /// and is never lit or pressed; it is editable where every street that meets there is (`meetings`, by `streets`).
    Meeting {
        node: u32,
        streets: Vec<u32>,
    },
}

/// How streets ending at one node are joined there.
enum Join {
    /// Two streets: a disc at the end of the narrower, as wide as it (`row_mm`). The streets are drawn with square
    /// ends, so streets meeting at an angle are joined round, and the wider does not bulge past the narrower where it
    /// goes on narrower.
    Disc { end: [i32; 2], row_mm: i32 },
    /// Three streets or more whose kerbs meet: the ground they share at their node (`middle`, a closed, simple ring
    /// round it, in layout millimetres), over every corner of their square ends, so wide streets crossing at an angle leave no corner sticking out, and a
    /// narrow street off a wide one adds nothing past the wide one's sides.
    Middle { ring: Vec<[f64; 2]> },
}

/// Where streets meet, how they are joined (`Join`): one for each node two streets or more end at. An end no other
/// street meets (a dead end, or where the street leaves the map) stays square, so a short wide stub is not rounded. At
/// a junction the join is the junction's ground: it takes the junction's colour and state, so it lights and opens with
/// the junction it is pressed for. Elsewhere a disc is the street's of the narrower end (`style.rs` draws it under the
/// lines, filling only the gap their square ends leave) and a shape is its own (`Whose::Meeting`), drawn under the
/// lines too (`places-middle`), filling only the gaps between their ends.
fn joints(v: &CityView) -> Vec<(Whose<'_>, Join)> {
    let mut ends: BTreeMap<u32, Vec<(&EdgeView, [i32; 2], Option<[i32; 2]>)>> = BTreeMap::new();
    for e in &v.edges {
        let (Some(&first), Some(&last)) = (e.shape_mm.first(), e.shape_mm.last()) else { continue };
        ends.entry(e.a).or_default().push((e, first, e.shape_mm.iter().copied().find(|q| *q != first)));
        ends.entry(e.b).or_default().push((e, last, e.shape_mm.iter().rev().copied().find(|q| *q != last)));
    }
    let junctions: BTreeMap<u32, &NodeView> = v.nodes.iter().filter(|n| n.junction).map(|n| (n.uid, n)).collect();
    ends.into_iter()
        .filter(|(_, at)| at.len() > 1)
        .map(|(node, mut at)| {
            at.sort_by_key(|(e, _, _)| (Reverse(e.row_mm), e.uid));
            let (e, end, _) = at[1];
            let ring = if at.len() > 2 { middle(end, at.iter().filter_map(|&(e, _, next)| Some((next?, e.row_mm)))) } else { None };
            let whose = match (junctions.get(&node), &ring) {
                (Some(n), _) => Whose::Junction(n),
                (None, Some(_)) => Whose::Meeting { node, streets: at.iter().map(|(e, _, _)| e.uid).collect() },
                (None, None) => Whose::Street(e),
            };
            (whose, ring.map_or(Join::Disc { end, row_mm: e.row_mm }, |ring| Join::Middle { ring }))
        })
        .collect()
}

/// The ground streets share where they meet at `at`, each street given by the next point along it and its width: a
/// closed ring, anticlockwise round `at`, or `None` where no two streets' kerbs meet. Streets leaving within half a
/// degree of each other are one, as wide as the widest. The shape is the union of pieces that each hold `at`:
/// - between two streets next to each other round the node, 10 to 170 degrees apart, the ground up to where their
///   kerbs meet ahead, no further along either than twice the narrower's half-width, and less the nearer they are to
///   10 or 170 degrees (nothing there), so a near-straight pair of different widths makes no long taper and the shape
///   has no step at either bound;
/// - along each street, from its square end as far as that ground reaches along it, and at least a centimetre;
/// - across the outside of a bend, or between streets within 10 degrees of straight on or of the same way, the
///   triangle from the node to their ends' corners.
///
/// Two streets crossing make the square where their kerbs cross, a side street off a through street the box of the
/// side street across it, and every corner of every street's square end is under it; nothing reaches past a kerb but
/// between two streets next to each other (into a sharp angle, or the inside of a bend where they differ in width)
/// and across the outside of a bend. Each piece holds the node, so their union is star-shaped from it: its outline is
/// where a ray from the node leaves the last piece, traced at every corner of a piece and every crossing of two pieces'
/// edges and just either side of each, so it follows the outline where it steps along a ray; points within a
/// millimetre of the line between their neighbours are dropped. That makes it a simple ring.
fn middle(at: [i32; 2], streets: impl Iterator<Item = ([i32; 2], i32)>) -> Option<Vec<[f64; 2]>> {
    let o = [at[0] as f64, at[1] as f64];
    // each street: its bearing, its direction and half its width
    let mut arms: Vec<(f64, [f64; 2], f64)> = streets
        .filter_map(|(next, row_mm)| {
            let (dx, dy) = (next[0] as f64 - o[0], next[1] as f64 - o[1]);
            let len = dx.hypot(dy);
            (len > 0.0 && row_mm > 0).then(|| (dy.atan2(dx), [dx / len, dy / len], row_mm as f64 / 2.0))
        })
        .collect();
    // streets leaving within half a degree of each other are one, as wide as the widest: a footway beside a road
    // changes nothing the road makes, whichever side of it it falls
    arms.sort_by(|a, b| a.0.total_cmp(&b.0).then(b.2.total_cmp(&a.2)));
    let same = (0.5f64).to_radians();
    let mut merged: Vec<(f64, [f64; 2], f64)> = Vec::with_capacity(arms.len());
    for arm in arms {
        match merged.last_mut() {
            Some(last) if arm.0 - last.0 < same => {
                if arm.2 > last.2 {
                    *last = arm;
                }
            }
            _ => merged.push(arm),
        }
    }
    if merged.len() > 1 && merged[0].0 + std::f64::consts::TAU - merged[merged.len() - 1].0 < same {
        let last = merged.pop().unwrap();
        if last.2 > merged[0].2 {
            merged[0] = last;
        }
    }
    let arms = merged;
    let n = arms.len();
    if n < 2 {
        return None;
    }
    let cross = |a: [f64; 2], b: [f64; 2]| a[0] * b[1] - a[1] * b[0];
    // a point `t` along a street from the node and `side` half-widths to its left (+1) or right (-1), from the node
    let kerb = |(_, d, h): (f64, [f64; 2], f64), side: f64, t: f64| [side * h * -d[1] + t * d[0], side * h * d[0] + t * d[1]];
    let mut reach = vec![0.0f64; n];
    let mut pieces: Vec<Vec<[f64; 2]>> = Vec::with_capacity(3 * n);
    let mut met = false;
    for i in 0..n {
        let j = (i + 1) % n;
        let (ai, aj) = (arms[i], arms[j]);
        let (c, ahead) = (cross(ai.1, aj.1), ai.1[0] * aj.1[0] + ai.1[1] * aj.1[1]);
        // how far from straight on or the same way they are: kerbs meet only between 10 and 170 degrees apart, and how
        // far their ground reaches grows from nothing there to twice the narrower's half-width 20 degrees further in,
        // so a near-straight pair of different widths makes no long taper and the shape has no step at either bound
        let apart = c.atan2(ahead).to_degrees();
        let room = ((apart - 10.0).min(170.0 - apart) / 20.0).clamp(0.0, 1.0);
        if c > 0.0 && room > 0.0 {
            // i's left kerb and j's right kerb meet: the ground between them up to there, or the cap
            let cap = 2.0 * ai.2.min(aj.2) * room;
            let between = [kerb(aj, -1.0, 0.0)[0] - kerb(ai, 1.0, 0.0)[0], kerb(aj, -1.0, 0.0)[1] - kerb(ai, 1.0, 0.0)[1]];
            let (t, u) = ((cross(between, aj.1) / c).clamp(0.0, cap), (cross(between, ai.1) / c).clamp(0.0, cap));
            reach[i] = reach[i].max(t);
            reach[j] = reach[j].max(u);
            met = true;
            let (l0, l, r, r0) = (kerb(ai, 1.0, 0.0), kerb(ai, 1.0, t), kerb(aj, -1.0, u), kerb(aj, -1.0, 0.0));
            pieces.extend([vec![[0.0, 0.0], l0, l], vec![[0.0, 0.0], l, r], vec![[0.0, 0.0], r, r0]]);
        } else {
            // within 10 degrees of straight on or of the same way, or the outside of a bend: the triangle to their ends'
            // corners
            pieces.push(vec![[0.0, 0.0], kerb(ai, 1.0, 0.0), kerb(aj, -1.0, 0.0)]);
        }
    }
    if !met {
        return None;
    }
    for (&arm, &t) in arms.iter().zip(&reach) {
        // at least a centimetre, so the pieces hold the node itself, not only on their edges
        let t = t.max(10.0);
        pieces.push(vec![kerb(arm, -1.0, 0.0), kerb(arm, -1.0, t), kerb(arm, 1.0, t), kerb(arm, 1.0, 0.0)]);
    }
    let edges: Vec<([f64; 2], [f64; 2])> = pieces.iter().flat_map(|p| (0..p.len()).map(move |k| (p[k], p[(k + 1) % p.len()]))).collect();
    // the bearings to trace at: every corner of a piece, and every crossing of two pieces' edges
    let mut bearings: Vec<f64> = Vec::new();
    let mut at_point = |q: [f64; 2]| {
        if q[0].hypot(q[1]) > 0.5 {
            bearings.push(q[1].atan2(q[0]));
        }
    };
    for &(a, _) in &edges {
        at_point(a);
    }
    for (k, &(a, b)) in edges.iter().enumerate() {
        for &(p, q) in &edges[k + 1..] {
            let (e, f) = ([b[0] - a[0], b[1] - a[1]], [q[0] - p[0], q[1] - p[1]]);
            let d = cross(e, f);
            if d.abs() <= 1e-9 * e[0].hypot(e[1]) * f[0].hypot(f[1]) {
                continue; // in a line: no crossing to trace at
            }
            let g = [p[0] - a[0], p[1] - a[1]];
            let (w, v) = (cross(g, f) / d, cross(g, e) / d);
            if (0.0..=1.0).contains(&w) && (0.0..=1.0).contains(&v) {
                at_point([a[0] + w * e[0], a[1] + w * e[1]]);
            }
        }
    }
    bearings.sort_by(f64::total_cmp);
    bearings.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    // -pi and pi are one bearing: traced twice, the outline would double back along it
    if bearings.len() > 1 && bearings[0] + std::f64::consts::TAU - bearings[bearings.len() - 1] < 1e-9 {
        bearings.pop();
    }
    // how far a ray from the node on bearing `b` goes before it leaves every piece
    let out = |b: f64| {
        let u = [b.cos(), b.sin()];
        edges
            .iter()
            .filter_map(|&(a, c)| {
                let e = [c[0] - a[0], c[1] - a[1]];
                let d = cross(u, e);
                if d.abs() < 1e-9 {
                    // along the edge: as far as its far end on the ray
                    return (cross(a, u).abs() < 1e-6).then(|| (a[0] * u[0] + a[1] * u[1]).max(c[0] * u[0] + c[1] * u[1]));
                }
                let (t, w) = (cross(a, e) / d, cross(a, u) / d);
                (t >= -1e-6 && (-1e-9..=1.0 + 1e-9).contains(&w)).then_some(t)
            })
            .fold(0.0f64, f64::max)
    };
    // at each bearing, the outline's point just before it, at it and just after it: where the outline runs along a
    // ray from the node (a piece's side, a street's square end), it steps in or out there, and follows the step. Just
    // either side is less than a third of the way to the next bearing, so the points keep their order round the node.
    let m = bearings.len();
    let mut ring: Vec<[f64; 2]> = Vec::with_capacity(3 * m + 1);
    for k in 0..m {
        let b = bearings[k];
        let gap = |other: f64| (other - b).rem_euclid(std::f64::consts::TAU);
        let (before, after) = if m > 1 { (gap(bearings[(k + m - 1) % m]), gap(bearings[(k + 1) % m])) } else { (1.0, 1.0) };
        let (before, after) = ((std::f64::consts::TAU - before).min(3e-7) / 3.0, after.min(3e-7) / 3.0);
        for b in [b - before, b, b + after] {
            let r = out(b);
            let q = [o[0] + r * b.cos(), o[1] + r * b.sin()];
            if ring.last().is_none_or(|p: &[f64; 2]| (p[0] - q[0]).hypot(p[1] - q[1]) > 0.5) {
                ring.push(q);
            }
        }
    }
    if ring.len() > 1 && (ring[0][0] - ring[ring.len() - 1][0]).hypot(ring[0][1] - ring[ring.len() - 1][1]) <= 0.5 {
        ring.pop();
    }
    // a point within a millimetre of the line between its neighbours adds nothing: the points of a straight run, and
    // the tip of a needle where a point just either side of a bearing lands a hair in from the outline
    let near_line = |q: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
        let t = (((q[0] - a[0]) * ex + (q[1] - a[1]) * ey) / (ex * ex + ey * ey).max(1e-12)).clamp(0.0, 1.0);
        (a[0] + t * ex - q[0]).hypot(a[1] + t * ey - q[1]) < 1.0
    };
    let mut k = 0;
    while ring.len() > 3 && k < ring.len() {
        let n = ring.len();
        if near_line(ring[k], ring[(k + n - 1) % n], ring[(k + 1) % n]) {
            ring.remove(k);
            k = k.saturating_sub(1);
        } else {
            k += 1;
        }
    }
    if ring.len() < 3 {
        return None;
    }
    ring.push(ring[0]);
    Some(ring)
}

/// The shapes where three streets or more meet that are not a junction (`Whose::Meeting`, as `joints` makes them),
/// each with the `hot` ids of the streets that meet there, sorted: what makes it editable (`editable::with_meetings`).
pub fn meetings(v: &CityView) -> Vec<(String, Vec<String>)> {
    joints(v)
        .into_iter()
        .filter_map(|(whose, _)| match whose {
            Whose::Meeting { node, streets } => {
                let mut hot: Vec<String> = streets.into_iter().map(|uid| format!("s-{uid}")).collect();
                hot.sort();
                hot.dedup();
                Some((format!("m-{node}"), hot))
            }
            _ => None,
        })
        .collect()
}

/// How far a zone tick reaches past each edge of the street, in millimetres.
pub const TICK_OVERHANG_MM: i32 = 2_000;

/// The point `d_mm` along a polyline, in layout millimetres, and the unit tangent of the segment it lies on. `None`
/// at the start or at or past the end, or on a segment of no length.
fn point_at(shape: &[[i32; 2]], d_mm: f64) -> Option<([f64; 2], [f64; 2])> {
    if d_mm <= 0.0 {
        return None;
    }
    let mut left = d_mm;
    for w in shape.windows(2) {
        let (dx, dy) = ((w[1][0] - w[0][0]) as f64, (w[1][1] - w[0][1]) as f64);
        let len = dx.hypot(dy);
        if left < len {
            let t = [dx / len, dy / len];
            return Some(([w[0][0] as f64 + t[0] * left, w[0][1] as f64 + t[1] * left], t));
        }
        left -= len;
    }
    None
}

/// The part of a polyline between `from_mm` and `to_mm` along it, in layout millimetres: the point at each end and the
/// polyline's own vertices between. Empty when it has no length or the span is empty.
fn stretch(shape: &[[i32; 2]], from_mm: f64, to_mm: f64) -> Vec<[f64; 2]> {
    let mut points: Vec<[f64; 2]> = Vec::new();
    let mut start = 0.0;
    for w in shape.windows(2) {
        let (dx, dy) = ((w[1][0] - w[0][0]) as f64, (w[1][1] - w[0][1]) as f64);
        let len = dx.hypot(dy);
        let end = start + len;
        if len > 0.0 && end > from_mm && start < to_mm {
            let at = |d: f64| {
                let k = (d - start) / len;
                [w[0][0] as f64 + dx * k, w[0][1] as f64 + dy * k]
            };
            if points.is_empty() {
                points.push(at(from_mm.max(start)));
            }
            points.push(at(to_mm.min(end)));
        }
        start = end;
    }
    points
}

/// What marks each of a street's zone boundaries, as GeoJSON. A boundary with a transition (its number in
/// `zone_bands_mm`) is a band: a line along the centreline over the transition, `band: true`. Any other boundary is a
/// short bar across the street, centred on the centreline, along the normal, `TICK_OVERHANG_MM` past each edge. Each
/// says its street (`s-<uid>`) and which boundary it is (`n`, from 1).
pub fn zone_ticks(v: &CityView, p: &Projection) -> String {
    let mut features: Vec<Value> = Vec::new();
    for e in &v.edges {
        let half = e.row_mm as f64 / 2.0 + TICK_OVERHANG_MM as f64;
        let street = format!("s-{}", e.uid);
        for (i, &d) in e.zone_breaks_mm.iter().enumerate() {
            if let Some(&(_, from, to)) = e.zone_bands_mm.iter().find(|b| b.0 == i + 1) {
                let line = stretch(&e.shape_mm, from as f64, to as f64);
                if line.len() >= 2 {
                    let coordinates: Vec<[f64; 2]> = line.iter().map(|q| position(p, q[0].round() as i32, q[1].round() as i32)).collect();
                    features.push(json!({
                        "type": "Feature",
                        "properties": { "street": street, "n": i + 1, "band": true },
                        "geometry": { "type": "LineString", "coordinates": coordinates },
                    }));
                    continue;
                }
            }
            let Some(([x, y], [tx, ty])) = point_at(&e.shape_mm, d as f64) else { continue };
            let end = |side: f64| position(p, (x - ty * half * side).round() as i32, (y + tx * half * side).round() as i32);
            features.push(json!({
                "type": "Feature",
                "properties": { "street": street, "n": i + 1 },
                "geometry": { "type": "LineString", "coordinates": [end(1.0), end(-1.0)] },
            }));
        }
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

/// The offset of each vertex of `line` from itself, `across_mm` to the right of the direction of travel, in layout
/// millimetres. A vertex between two segments moves along the normalised sum of their right normals, scaled by
/// `1 / cos(half the turn)` to keep the offset line `across_mm` from both, but by no more than 2, so a sharp turn
/// does not throw a spike.
fn offset_line(line: &[[f64; 2]], across_mm: f64) -> Vec<[f64; 2]> {
    let normal = |a: [f64; 2], b: [f64; 2]| {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = dx.hypot(dy);
        [-dy / len, dx / len]
    };
    line.iter()
        .enumerate()
        .map(|(i, &q)| {
            let before = (i > 0).then(|| normal(line[i - 1], q));
            let after = line.get(i + 1).map(|&next| normal(q, next));
            let (n, scale) = match (before, after) {
                (Some(a), Some(b)) => {
                    let sum = [a[0] + b[0], a[1] + b[1]];
                    let len = sum[0].hypot(sum[1]);
                    if len < 1e-9 {
                        (a, 1.0)
                    } else {
                        let m = [sum[0] / len, sum[1] / len];
                        (m, (1.0 / (m[0] * a[0] + m[1] * a[1])).min(2.0))
                    }
                }
                (Some(n), None) | (None, Some(n)) => (n, 1.0),
                (None, None) => ([0.0, 0.0], 1.0),
            };
            [q[0] + n[0] * across_mm * scale, q[1] + n[1] * across_mm * scale]
        })
        .collect()
}

/// The ring of the strip beside a polyline, between `from_mm` and `to_mm` along it (as `stretch` measures) and
/// `lo_mm` to `hi_mm` across it (positive to the right looking along the shape), in layout millimetres. Closed, and
/// wound anticlockwise on the map (clockwise in layout millimetres, where y runs south). Empty when the span has no
/// length.
fn offset_ring(shape: &[[i32; 2]], from_mm: f64, to_mm: f64, lo_mm: f64, hi_mm: f64) -> Vec<[f64; 2]> {
    let line = stretch(shape, from_mm, to_mm);
    if line.len() < 2 {
        return Vec::new();
    }
    let (lo, hi) = (offset_line(&line, lo_mm), offset_line(&line, hi_mm));
    let mut ring: Vec<[f64; 2]> = vec![lo[0]];
    ring.extend(hi.iter().copied());
    ring.extend(lo.iter().rev().take(lo.len() - 1).copied());
    ring.push(lo[0]);
    ring
}

fn ring_coordinates(p: &Projection, ring: &[[f64; 2]]) -> Vec<[f64; 2]> {
    ring.iter().map(|q| position(p, q[0].round() as i32, q[1].round() as i32)).collect()
}

fn side_id(side: Side) -> &'static str {
    match side {
        Side::Left => "left",
        Side::Right => "right",
    }
}

/// The curbside uses of every street as a FeatureCollection of polygons beside the street, each saying its street
/// (`hot`, `s-<uid>`), the use (`use`, its snake-case id) and the `side`.
pub fn curbside(v: &CityView, p: &Projection) -> String {
    let mut features: Vec<Value> = Vec::new();
    for e in &v.edges {
        for u in &e.curb_mm {
            let ring = offset_ring(&e.shape_mm, u.from_mm as f64, u.to_mm as f64, u.lo_mm as f64, u.hi_mm as f64);
            if ring.is_empty() {
                continue;
            }
            features.push(json!({
                "type": "Feature",
                "properties": { "hot": format!("s-{}", e.uid), "use": u.use_, "side": side_id(u.side) },
                "geometry": { "type": "Polygon", "coordinates": [ring_coordinates(p, &ring)] },
            }));
        }
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

/// The transit stops of every street: for each, its platform as a polygon (`part: "platform"`) and a point (`part:
/// "dot"`) in the middle of its length on the platform's outer edge, with its street (`hot`), `kind` and `side`, and
/// its `name` when it has one. Nothing here depends on the language.
pub fn stops(v: &CityView, p: &Projection) -> String {
    let mut features: Vec<Value> = Vec::new();
    for e in &v.edges {
        for s in &e.stops_mm {
            let ring = offset_ring(&e.shape_mm, s.from_mm as f64, s.to_mm as f64, s.lo_mm as f64, s.hi_mm as f64);
            let outer = if s.side == Side::Right { s.hi_mm } else { s.lo_mm } as f64;
            let Some(([x, y], [tx, ty])) = point_at(&e.shape_mm, (s.from_mm as f64 + s.to_mm as f64) / 2.0) else { continue };
            if ring.is_empty() {
                continue;
            }
            let mut platform = json!({ "part": "platform", "hot": format!("s-{}", e.uid), "kind": s.kind, "side": side_id(s.side) });
            features.push(json!({
                "type": "Feature",
                "properties": platform.clone(),
                "geometry": { "type": "Polygon", "coordinates": [ring_coordinates(p, &ring)] },
            }));
            platform["part"] = json!("dot");
            let mut properties = platform;
            if let Some(name) = &s.name {
                properties["name"] = json!(name);
            }
            features.push(json!({
                "type": "Feature",
                "properties": properties,
                "geometry": { "type": "Point", "coordinates": position(p, (x - ty * outer).round() as i32, (y + tx * outer).round() as i32) },
            }));
        }
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

/// A ring of corners as GeoJSON wants a polygon's: closed, the first corner again at the end.
fn closed(ring: &[LonLat]) -> Vec<[f64; 2]> {
    let mut c: Vec<[f64; 2]> = ring.iter().map(|p| [r6(p[0]), r6(p[1])]).collect();
    if let Some(&first) = c.first() {
        c.push(first);
    }
    c
}

/// The restrictions as GeoJSON: each a polygon with its id and rule, the selected one marked and its corners as
/// points to drag. `moving` is the selected one's outline while a corner of it is dragged.
pub fn areas(list: &[Restriction], selected: Option<&str>, moving: Option<&[LonLat]>) -> String {
    let mut features: Vec<Value> = Vec::new();
    for r in list {
        let on = selected == Some(r.id.as_str());
        let ring = if on { moving.unwrap_or(&r.ring) } else { &r.ring };
        features.push(json!({
            "type": "Feature",
            "properties": { "area": r.id, "rule": r.rule.id(), "selected": on },
            "geometry": { "type": "Polygon", "coordinates": [closed(ring)] },
        }));
        if on {
            for (i, c) in ring.iter().enumerate() {
                features.push(json!({
                    "type": "Feature",
                    "properties": { "area": r.id, "i": i },
                    "geometry": { "type": "Point", "coordinates": [r6(c[0]), r6(c[1])] },
                }));
            }
        }
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

/// The outline being drawn: the area it would close once it has three corners, the path so far, and each corner,
/// the first marked (pressing it closes the outline).
pub fn draft(corners: &[LonLat]) -> String {
    let at = |c: &LonLat| [r6(c[0]), r6(c[1])];
    let mut features: Vec<Value> = Vec::new();
    if corners.len() >= 3 {
        features.push(json!({ "type": "Feature", "properties": { "part": "fill" }, "geometry": { "type": "Polygon", "coordinates": [closed(corners)] } }));
    }
    if corners.len() >= 2 {
        features.push(json!({ "type": "Feature", "properties": { "part": "path" }, "geometry": { "type": "LineString", "coordinates": corners.iter().map(at).collect::<Vec<_>>() } }));
    }
    for (i, c) in corners.iter().enumerate() {
        features.push(json!({ "type": "Feature", "properties": { "i": i, "first": i == 0 }, "geometry": { "type": "Point", "coordinates": at(c) } }));
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

/// West, south, east and north of everything the places draw, in degrees.
pub fn bounds(v: &CityView, p: &Projection) -> Option<[f64; 4]> {
    let streets = v.edges.iter().flat_map(|e| e.shape_mm.iter().map(|q| (q[0], q[1])));
    let junctions = v.nodes.iter().filter(|n| n.junction).map(|n| (n.x_mm, n.y_mm));
    streets.chain(junctions).map(|(x, y)| p.lon_lat(x, y)).fold(None, |b, [lon, lat]| match b {
        None => Some([lon, lat, lon, lat]),
        Some([w, s, e, n]) => Some([w.min(lon), s.min(lat), e.max(lon), n.max(lat)]),
    })
}

/// The `hot` id of each place and the box of what the map draws of it (west, south, east, north, in degrees): a
/// street's drawn shape, a junction's point. The same order as `places`.
pub fn place_boxes(v: &CityView, p: &Projection) -> Vec<(String, [f64; 4])> {
    let mut boxes = Vec::with_capacity(v.edges.len() + v.nodes.len());
    for e in &v.edges {
        let mut shape = e.shape_mm.iter().map(|q| position(p, q[0], q[1]));
        let Some([lon, lat]) = shape.next() else { continue };
        let b = shape.fold([lon, lat, lon, lat], |[w, s, east, n], [lon, lat]| [w.min(lon), s.min(lat), east.max(lon), n.max(lat)]);
        boxes.push((format!("s-{}", e.uid), b));
    }
    for n in v.nodes.iter().filter(|n| n.junction) {
        let [lon, lat] = position(p, n.x_mm, n.y_mm);
        boxes.push((format!("j-{}", n.uid), [lon, lat, lon, lat]));
    }
    boxes
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use cityloom_model::{
        Control, Direction, Edge, Junction, Lane, LaneKind, Network, NodeId, NodeRef, Point, Provenance, Road, RoadId, RoadType, WayRef, Zone,
    };
    use serde_json::Value;

    use super::*;
    use crate::city::model::City;
    use crate::city::projection::Projection;
    use crate::place::area::Area;
    use crate::restriction::model::{Restriction, Rule};

    /// One street that bends: east for 80 m, then north-east to a point 100 m east and 50 m north.
    fn bent_road() -> Network {
        let node = |id: u32, x_m, y_m| Junction {
            id: NodeId(id),
            provenance: Provenance::of_nodes([NodeRef::new(id as i64)]),
            position: Point::new(x_m, y_m),
            junction: false,
            control: Control::None,
        };
        let lane = |way| Lane { kind: LaneKind::Driving, direction: way, width_m: 3.0, buffer_type: None, windows: vec![], shape: vec![] };
        Network {
            left_hand: false,
            corridors: vec![],
            junctions: vec![node(1, 0.0, 0.0), node(2, 100.0, 50.0)],
            roads: vec![Road {
                edge: Edge { id: RoadId(1), from: NodeId(1), to: NodeId(2) },
                provenance: Provenance::of_ways([WayRef::new(7)]),
                name: "Bend Street".into(),
                road_type: RoadType::Local,
                zones: vec![Zone::whole(vec![lane(Direction::Forward), lane(Direction::Backward)])],
                points: vec![Point::new(0.0, 0.0), Point::new(80.0, 0.0), Point::new(100.0, 50.0)],
            }],
        }
    }

    fn projection(v: &crate::city::model::CityView) -> Projection {
        Projection::new(Area::new("Bendville", 45.5, -73.6).bounds(), v.origin_m.unwrap())
    }

    fn parse(json: &str) -> Value {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn a_street_is_a_line_along_its_centreline_with_the_place_it_opens_and_how_it_stands() {
        let v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let fc = parse(&places(&v, &p, &BTreeSet::new()));
        let features: Vec<&Value> = fc["features"].as_array().unwrap().iter().filter(|f| f["properties"]["joint_m"].is_null()).collect();
        assert_eq!(features.len(), 1, "one street and no junction");
        let f = &features[0];
        assert_eq!(f["geometry"]["type"], "LineString");
        assert_eq!(f["properties"]["hot"], "s-1");
        assert_eq!(f["properties"]["status"], "ok");
        assert_eq!(f["properties"]["width_m"].as_f64().unwrap(), v.edges[0].row_mm as f64 / 1000.0);
        let line = f["geometry"]["coordinates"].as_array().unwrap();
        assert_eq!(line.len(), 3, "the bend is kept");
        let round = |x: f64| (x * 1e6).round() / 1e6;
        let first = p.lon_lat(v.edges[0].shape_mm[0][0], v.edges[0].shape_mm[0][1]);
        assert_eq!(line[0], serde_json::json!([round(first[0]), round(first[1])]));
        // the middle point is off the chord between the ends: the street is not drawn straight
        let (a, m, b) = (&line[0], &line[1], &line[2]);
        let lerp = a[1].as_f64().unwrap()
            + (b[1].as_f64().unwrap() - a[1].as_f64().unwrap()) * (m[0].as_f64().unwrap() - a[0].as_f64().unwrap())
                / (b[0].as_f64().unwrap() - a[0].as_f64().unwrap());
        assert!((m[1].as_f64().unwrap() - lerp).abs() > 1e-5, "{m} on the chord from {a} to {b}");
    }

    #[test]
    fn a_street_says_its_class_so_that_the_map_can_tell_a_pedestrian_street() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let class =
            |v: &crate::city::model::CityView| parse(&places(v, &p, &BTreeSet::new()))["features"][0]["properties"]["class"].as_str().unwrap().to_string();
        assert_eq!(class(&v), "local");
        v.edges[0].class = crate::shared::catalogue::StreetClass::Pedestrian;
        assert_eq!(class(&v), "pedestrian");
    }

    #[test]
    fn a_street_that_fails_or_has_been_changed_says_so() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let status =
            |v: &crate::city::model::CityView| parse(&places(v, &p, &BTreeSet::new()))["features"][0]["properties"]["status"].as_str().unwrap().to_string();
        assert_eq!(status(&v), "ok");
        v.edges[0].edited = true;
        assert_eq!(status(&v), "changed");
        v.edges[0].ok = false;
        assert_eq!(status(&v), "bad", "failing wins over changed");
    }

    #[test]
    fn a_junction_is_a_point_with_its_radius_and_a_street_end_is_not_drawn() {
        let v = City::new().view(0); // the sample city has junctions and gates
        let p = Projection::new(Area::new("Sample", 45.5, -73.6).bounds(), [0.0, 0.0]);
        let fc = parse(&places(&v, &p, &BTreeSet::new()));
        let points: Vec<&Value> =
            fc["features"].as_array().unwrap().iter().filter(|f| f["geometry"]["type"] == "Point" && f["properties"]["joint_m"].is_null()).collect();
        assert_eq!(points.len(), v.nodes.iter().filter(|n| n.junction).count());
        assert!(points.iter().all(|f| f["properties"]["hot"].as_str().unwrap().starts_with("j-")));
        // pressed round its point as far as its own radius, except where its ground is a shape, pressed for itself
        let shaped: BTreeSet<&str> =
            fc["features"].as_array().unwrap().iter().filter(|f| f["geometry"]["type"] == "Polygon").filter_map(|f| f["properties"]["hot"].as_str()).collect();
        assert!(!shaped.is_empty(), "the sample city has junctions three streets meet at");
        for f in &points {
            let (hot, r) = (f["properties"]["hot"].as_str().unwrap(), f["properties"]["radius_m"].as_f64().unwrap());
            assert!(if shaped.contains(hot) { r == 0.0 } else { r > 0.0 }, "{hot}: {r}");
        }
        let streets = fc["features"].as_array().unwrap().iter().filter(|f| f["geometry"]["type"] == "LineString").count();
        assert_eq!(streets, v.edges.len());
    }

    #[test]
    fn each_place_has_the_box_of_its_drawn_shape_and_a_junction_a_point() {
        let v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let boxes = place_boxes(&v, &p);
        assert_eq!(boxes.len(), 1, "one street and no junction");
        let line = parse(&places(&v, &p, &BTreeSet::new()))["features"][0]["geometry"]["coordinates"].clone();
        let (lons, lats): (Vec<f64>, Vec<f64>) = line.as_array().unwrap().iter().map(|c| (c[0].as_f64().unwrap(), c[1].as_f64().unwrap())).unzip();
        let min = |v: &[f64]| v.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(boxes[0], ("s-1".to_string(), [min(&lons), min(&lats), max(&lons), max(&lats)]));

        let city = City::new().view(0); // the sample city has junctions
        let pc = Projection::new(Area::new("Sample", 45.5, -73.6).bounds(), [0.0, 0.0]);
        let boxes = place_boxes(&city, &pc);
        let junctions: Vec<_> = boxes.iter().filter(|(h, _)| h.starts_with("j-")).collect();
        assert_eq!(junctions.len(), city.nodes.iter().filter(|n| n.junction).count());
        assert!(junctions.iter().all(|(_, b)| b[0] == b[2] && b[1] == b[3]), "a junction is a point");
        assert_eq!(boxes.len(), city.edges.len() + junctions.len());
    }

    #[test]
    fn the_bounds_hold_every_point_of_every_street() {
        let v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let [w, s, e, n] = bounds(&v, &p).unwrap();
        for c in parse(&places(&v, &p, &BTreeSet::new()))["features"][0]["geometry"]["coordinates"].as_array().unwrap() {
            let (lon, lat) = (c[0].as_f64().unwrap(), c[1].as_f64().unwrap());
            assert!(w - 1e-6 <= lon && lon <= e + 1e-6 && s - 1e-6 <= lat && lat <= n + 1e-6, "{c} outside {w} {s} {e} {n}");
        }
        assert!(w < e && s < n);
    }

    #[test]
    fn a_street_in_conflict_with_an_area_says_so() {
        let v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let conflict = |set: BTreeSet<u32>| parse(&places(&v, &p, &set))["features"][0]["properties"]["conflict"].as_bool().unwrap();
        assert!(!conflict(BTreeSet::new()));
        assert!(conflict([1].into()));
    }

    fn area(id: &str, rule: Rule) -> Restriction {
        Restriction { id: id.into(), name: String::new(), rule, ring: vec![[-73.6, 45.5], [-73.59, 45.5], [-73.59, 45.51]] }
    }

    #[test]
    fn an_area_is_a_closed_polygon_with_its_id_and_rule_and_the_selected_one_has_its_corners() {
        let list = [area("r1", Rule::CarFree), area("r2", Rule::SpeedLimit { kmh: 30 })];
        let fc = parse(&areas(&list, Some("r2"), None));
        let f = fc["features"].as_array().unwrap();
        assert_eq!(f.len(), 2 + 3, "two outlines and the selected one's three corners");
        assert_eq!(f[0]["geometry"]["type"], "Polygon");
        assert_eq!(f[0]["properties"]["area"], "r1");
        assert_eq!(f[0]["properties"]["rule"], "car_free");
        assert_eq!(f[0]["properties"]["selected"], false);
        let ring = f[0]["geometry"]["coordinates"][0].as_array().unwrap();
        assert_eq!(ring.len(), 4, "closed: the first corner again");
        assert_eq!(ring[0], ring[3]);
        assert_eq!(f[1]["properties"]["selected"], true);
        assert_eq!(f[2]["geometry"]["type"], "Point");
        assert_eq!((f[2]["properties"]["area"].as_str(), f[2]["properties"]["i"].as_u64()), (Some("r2"), Some(0)));
    }

    #[test]
    fn the_selected_area_is_drawn_where_its_corner_is_being_dragged() {
        let list = [area("r1", Rule::CarFree)];
        let moved = [[-73.61, 45.5], [-73.59, 45.5], [-73.59, 45.51]];
        let fc = parse(&areas(&list, Some("r1"), Some(&moved)));
        assert_eq!(fc["features"][0]["geometry"]["coordinates"][0][0], serde_json::json!([-73.61, 45.5]));
        assert_eq!(fc["features"][1]["geometry"]["coordinates"], serde_json::json!([-73.61, 45.5]));
        let fc = parse(&areas(&list, None, Some(&moved)));
        assert_eq!(fc["features"][0]["geometry"]["coordinates"][0][0], serde_json::json!([-73.6, 45.5]), "only the selected one moves");
    }

    #[test]
    fn the_outline_being_drawn_is_its_corners_the_path_and_once_it_can_close_its_area() {
        assert!(parse(&draft(&[]))["features"].as_array().unwrap().is_empty());
        let one = parse(&draft(&[[-73.6, 45.5]]));
        assert_eq!(one["features"].as_array().unwrap().len(), 1);
        assert_eq!(one["features"][0]["properties"]["first"], true);
        let three = parse(&draft(&[[-73.6, 45.5], [-73.59, 45.5], [-73.59, 45.51]]));
        let parts: Vec<String> = three["features"].as_array().unwrap().iter().map(|f| f["geometry"]["type"].as_str().unwrap().to_string()).collect();
        assert_eq!(parts, ["Polygon", "LineString", "Point", "Point", "Point"]);
        assert_eq!(three["features"][0]["properties"]["part"], "fill");
        assert_eq!(three["features"][1]["properties"]["part"], "path");
        assert_eq!(three["features"][4]["properties"]["i"], 2);
    }

    /// One straight street, east for 100 m, 8 m wide.
    fn straight_road() -> Network {
        let mut n = bent_road();
        n.junctions[1].position = Point::new(100.0, 0.0);
        n.roads[0].points = vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0)];
        n.roads[0].base_lanes_mut().iter_mut().for_each(|l| l.width_m = 4.0);
        n
    }

    /// Two streets end to end at node 2: the first from node 1 with lanes `first_m` wide, the second on to node 3 with
    /// lanes `second_m` wide, turning by `turn_deg` there (0: straight on).
    fn two_streets(first_m: f64, second_m: f64, turn_deg: f64) -> Network {
        let mut n = straight_road();
        n.roads[0].base_lanes_mut().iter_mut().for_each(|l| l.width_m = first_m);
        let (s, c) = turn_deg.to_radians().sin_cos();
        let mut third = n.junctions[1].clone();
        third.id = NodeId(3);
        third.provenance = Provenance::of_nodes([NodeRef::new(3)]);
        third.position = Point::new(100.0 + 100.0 * c, 100.0 * s);
        n.junctions.push(third.clone());
        let mut road = n.roads[0].clone();
        road.edge = Edge { id: RoadId(2), from: NodeId(2), to: NodeId(3) };
        road.provenance = Provenance::of_ways([WayRef::new(8)]);
        road.name = "Next Street".into();
        road.points = vec![Point::new(100.0, 0.0), third.position];
        road.base_lanes_mut().iter_mut().for_each(|l| l.width_m = second_m);
        n.roads.push(road);
        n
    }

    /// Streets from one node at the origin, one for each `(lane width, bearing in degrees)`, each 100 m long. The node is
    /// not marked a junction in the data, so the city does not take it for one, however many streets meet there.
    fn star(arms: &[(f64, f64)]) -> Network {
        let mut n = straight_road();
        let template = n.roads[0].clone();
        let mut node = n.junctions[0].clone();
        n.junctions.truncate(1);
        n.roads.clear();
        for (i, &(lane_m, deg)) in arms.iter().enumerate() {
            let id = i as u32 + 2;
            let (s, c) = deg.to_radians().sin_cos();
            node.id = NodeId(id);
            node.provenance = Provenance::of_nodes([NodeRef::new(id as i64)]);
            node.position = Point::new(100.0 * c, 100.0 * s);
            n.junctions.push(node.clone());
            let mut road = template.clone();
            road.edge = Edge { id: RoadId(id), from: NodeId(1), to: NodeId(id) };
            road.provenance = Provenance::of_ways([WayRef::new(id as i64 + 10)]);
            road.name = format!("Arm {id}");
            road.points = vec![Point::new(0.0, 0.0), node.position];
            road.base_lanes_mut().iter_mut().for_each(|l| l.width_m = lane_m);
            n.roads.push(road);
        }
        n
    }

    /// The shapes of `places` where three streets or more meet, as (feature, ring in metres east and north of `centre`).
    fn middles(fc: &Value, p: &Projection, centre: [i32; 2]) -> Vec<(Value, Vec<[f64; 2]>)> {
        let to_m = metres_from(p, centre);
        fc["features"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["geometry"]["type"] == "Polygon")
            .map(|f| {
                let ring = f["geometry"]["coordinates"][0].as_array().unwrap().iter().map(|c| to_m([c[0].as_f64().unwrap(), c[1].as_f64().unwrap()])).collect();
                (f.clone(), ring)
            })
            .collect()
    }

    /// Metres east and north of the layout point `centre`, from a longitude and latitude.
    fn metres_from(p: &Projection, centre: [i32; 2]) -> impl Fn([f64; 2]) -> [f64; 2] {
        let [lon0, lat0] = p.lon_lat(centre[0], centre[1]);
        move |[lon, lat]| [(lon - lon0) * 111_195.0 * lat0.to_radians().cos(), (lat - lat0) * 111_195.0]
    }

    /// The node every street of `v` ends at, in layout millimetres.
    fn hub(v: &crate::city::model::CityView) -> [i32; 2] {
        let e = &v.edges[0];
        [e.shape_mm[0], *e.shape_mm.last().unwrap()]
            .into_iter()
            .find(|q| v.edges.iter().all(|o| o.shape_mm[0] == *q || *o.shape_mm.last().unwrap() == *q))
            .expect("the streets meet at one node")
    }

    /// Whether `q` is inside the closed `ring` (either winding) or within `slack` of its edge.
    fn holds(ring: &[[f64; 2]], q: [f64; 2], slack: f64) -> bool {
        let near = ring.windows(2).any(|w| {
            let (a, b) = (w[0], w[1]);
            let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
            let t = (((q[0] - a[0]) * ex + (q[1] - a[1]) * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
            (a[0] + t * ex - q[0]).hypot(a[1] + t * ey - q[1]) <= slack
        });
        let crossings = ring.windows(2).filter(|w| {
            let (a, b) = (w[0], w[1]);
            (a[1] > q[1]) != (b[1] > q[1]) && q[0] < a[0] + (q[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0])
        });
        near || crossings.count() % 2 == 1
    }

    /// The corners of each street's square end at `centre`, in metres east and north of it: half its width either side.
    fn corners(v: &crate::city::model::CityView, p: &Projection, centre: [i32; 2]) -> Vec<[f64; 2]> {
        let to_m = metres_from(p, centre);
        v.edges
            .iter()
            .flat_map(|e| {
                let next = if e.shape_mm[0] == centre { e.shape_mm[1] } else { e.shape_mm[e.shape_mm.len() - 2] };
                let [x, y] = to_m(p.lon_lat(next[0], next[1]));
                let len = x.hypot(y);
                let h = e.row_mm as f64 / 2000.0;
                [[-y / len * h, x / len * h], [y / len * h, -x / len * h]]
            })
            .collect()
    }

    #[test]
    fn where_three_streets_or_more_meet_one_shape_over_their_ends_covers_every_corner_of_them() {
        let v = City::from_network(&star(&[(3.0, 0.0), (5.0, 100.0), (8.0, 215.0), (4.0, 290.0), (3.0, 330.0), (6.0, 160.0)]), "Bendville").view(0);
        assert!(v.nodes.iter().all(|n| !n.junction), "six streets at a node not marked a junction");
        let (p, centre) = (projection(&v), hub(&v));
        let fc = parse(&places(&v, &p, &BTreeSet::new()));
        let found = middles(&fc, &p, centre);
        assert_eq!(found.len(), 1, "one shape where they meet");
        assert!(joints(&fc).is_empty(), "and no disc there: {:?}", joints(&fc));
        let (f, ring) = &found[0];
        assert_eq!(ring.first(), ring.last(), "a closed ring");
        for q in corners(&v, &p, centre) {
            assert!(holds(ring, q, 0.2), "the corner {q:?} of a street's square end is under the shape {ring:?}");
        }
        let node = v.nodes.iter().find(|n| [n.x_mm, n.y_mm] == centre).expect("the node they meet at");
        assert_eq!(f["properties"]["hot"], format!("m-{}", node.uid), "its own, not one street's: hovering a street never lights the others' ends");
        assert_eq!(f["properties"]["status"], "ok");
        assert_eq!(f["properties"]["conflict"], false);
        // it is editable where every street that meets there is (`editable::with_meetings`), so it is never solid
        // under a muted street's translucent line
        let mut streets: Vec<String> = v.edges.iter().map(|e| format!("s-{}", e.uid)).collect();
        streets.sort();
        assert_eq!(meetings(&v), vec![(format!("m-{}", node.uid), streets)]);
        assert!(place_boxes(&v, &p).iter().all(|(h, _)| !h.starts_with("m-")), "no box of its own");
        // under every street's end, it takes none of their states
        let mut v = v;
        v.edges.iter_mut().for_each(|e| (e.edited, e.ok) = (true, false));
        let fc = parse(&places(&v, &p, &v.edges.iter().map(|e| e.uid).collect()));
        let f = &middles(&fc, &p, centre)[0].0;
        assert_eq!((&f["properties"]["status"], &f["properties"]["conflict"]), (&json!("ok"), &json!(false)));
        assert!(f["properties"]["ground"].is_null() && f["properties"]["radius_m"].is_null(), "not a junction");
    }

    #[test]
    fn a_narrow_street_meeting_a_wide_one_makes_no_shape_past_the_wide_streets_sides() {
        // a wide street straight through, east-west, and a narrow one off it to the north
        let v = City::from_network(&star(&[(6.0, 0.0), (6.0, 180.0), (1.5, 90.0)]), "Bendville").view(0);
        let (p, centre) = (projection(&v), hub(&v));
        let fc = parse(&places(&v, &p, &BTreeSet::new()));
        let found = middles(&fc, &p, centre);
        assert_eq!(found.len(), 1);
        let wide = v.edges.iter().map(|e| e.row_mm).max().unwrap() as f64 / 1000.0;
        let narrow = v.edges.iter().map(|e| e.row_mm).min().unwrap() as f64 / 1000.0;
        for q in &found[0].1 {
            assert!(q[1].abs() <= wide / 2.0 + 0.2, "inside the wide street's sides: {q:?}");
            assert!(q[0].abs() <= narrow / 2.0 + 0.2, "no further along the wide street than the narrow one is wide: {q:?}");
        }
    }

    /// The joints of `places`: the discs that round the streets' ends, as (hot, radius in metres, position).
    fn joints(fc: &Value) -> Vec<(String, f64, Value)> {
        let all = fc["features"].as_array().unwrap();
        all.iter()
            .filter(|f| f["properties"]["joint_m"].is_number())
            .map(|f| (f["properties"]["hot"].as_str().unwrap().to_string(), f["properties"]["joint_m"].as_f64().unwrap(), f["geometry"]["coordinates"].clone()))
            .collect()
    }

    #[test]
    fn streets_meeting_at_an_angle_are_joined_by_a_disc_as_wide_as_they_are() {
        let v = City::from_network(&two_streets(4.0, 4.0, 60.0), "Bendville").view(0);
        let p = projection(&v);
        let fc = parse(&places(&v, &p, &BTreeSet::new()));
        let (first, second) = (&v.edges[0], &v.edges[1]);
        let shared = [first.a, first.b].into_iter().find(|n| [second.a, second.b].contains(n)).expect("the streets share a node");
        let end = |e: &crate::city::model::EdgeView| if e.a == shared { e.shape_mm[0] } else { *e.shape_mm.last().unwrap() };
        assert_eq!(end(first), end(second));
        let there = serde_json::json!(position(&p, end(first)[0], end(first)[1]));
        let found: Vec<_> = joints(&fc).into_iter().filter(|(_, _, c)| *c == there).collect();
        assert_eq!(found.len(), 1, "one joint where they meet: {found:?}");
        let (hot, r, _) = &found[0];
        assert!([first.uid, second.uid].iter().any(|uid| format!("s-{uid}") == *hot), "the joint is one of the two streets': {hot}");
        assert!((*r - first.row_mm as f64 / 2000.0).abs() < 1e-9, "{r}");
    }

    #[test]
    fn a_wide_street_going_on_narrower_is_joined_only_as_wide_as_the_narrower_so_it_does_not_bulge() {
        let v = City::from_network(&two_streets(6.0, 3.0, 0.0), "Bendville").view(0);
        let p = projection(&v);
        let fc = parse(&places(&v, &p, &BTreeSet::new()));
        let narrow = v.edges.iter().min_by_key(|e| e.row_mm).unwrap();
        let wide = v.edges.iter().max_by_key(|e| e.row_mm).unwrap();
        assert!(narrow.row_mm < wide.row_mm);
        let shared: Vec<_> =
            joints(&fc).into_iter().filter(|(_, _, c)| *c == serde_json::json!(position(&p, narrow.shape_mm[0][0], narrow.shape_mm[0][1]))).collect();
        assert_eq!(shared.len(), 1, "one joint where they meet: {shared:?}");
        assert_eq!(shared[0].0, format!("s-{}", narrow.uid), "it is the narrower street's, and takes its state");
        assert!((shared[0].1 - narrow.row_mm as f64 / 2000.0).abs() < 1e-9);
    }

    #[test]
    fn a_street_end_no_other_street_meets_is_left_square_so_a_short_wide_stub_does_not_become_a_disc() {
        let v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        assert_eq!(joints(&parse(&places(&v, &p, &BTreeSet::new()))), vec![], "a dead end, or where a street leaves the map");
        let v = City::from_network(&two_streets(4.0, 4.0, 60.0), "Bendville").view(0);
        let found = joints(&parse(&places(&v, &projection(&v), &BTreeSet::new())));
        assert_eq!(found.len(), 1, "only where the two meet: {found:?}");
    }

    #[test]
    fn at_a_junction_the_shape_where_its_streets_meet_is_its_ground_so_it_lights_and_opens_with_the_junction() {
        let mut v = City::new().view(0); // the sample city has junctions
        v.nodes.iter_mut().filter(|n| n.junction).for_each(|n| n.edited = true);
        let p = Projection::new(Area::new("Sample", 45.5, -73.6).bounds(), [0.0, 0.0]);
        let fc = parse(&places(&v, &p, &v.edges.iter().map(|e| e.uid).collect()));
        let all = fc["features"].as_array().unwrap();
        let mut grounds = 0;
        for n in v.nodes.iter().filter(|n| n.junction) {
            if v.edges.iter().filter(|e| e.a == n.uid).chain(v.edges.iter().filter(|e| e.b == n.uid)).count() < 3 {
                continue;
            }
            let there: Vec<&Value> = all.iter().filter(|f| f["properties"]["hot"] == format!("j-{}", n.uid) && f["properties"]["radius_m"].is_null()).collect();
            assert_eq!(there.len(), 1, "one shape at junction {}, and it is the junction's: {there:?}", n.uid);
            let ground = there[0];
            assert_eq!(ground["geometry"]["type"], "Polygon");
            assert_eq!(ground["properties"]["ground"], true);
            assert_eq!(ground["properties"]["status"], "changed", "it takes the junction's state");
            assert_eq!(ground["properties"]["conflict"], false, "a street's conflict is not the junction's");
            grounds += 1;
        }
        assert!(grounds > 0, "the sample city has junctions three streets meet at");
    }

    #[test]
    fn where_a_junctions_ground_is_a_shape_its_press_target_is_the_shape_and_no_wider_circle() {
        let mut v = City::new().view(0); // the sample city has junctions
        let n = v.nodes.iter().find(|n| n.junction && v.edges.iter().filter(|e| e.a == n.uid || e.b == n.uid).count() >= 3).unwrap();
        let (uid, radius_mm, centre) = (n.uid, n.radius_mm, [n.x_mm, n.y_mm]);
        // one street there far wider than the widest was, at an odd width, halved
        let wide_mm = 4 * radius_mm + 1;
        v.edges.iter_mut().find(|e| e.a == uid || e.b == uid).unwrap().row_mm = wide_mm;
        let p = Projection::new(Area::new("Sample", 45.5, -73.6).bounds(), [0.0, 0.0]);
        let fc = parse(&places(&v, &p, &BTreeSet::new()));
        let target = fc["features"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["properties"]["hot"] == format!("j-{uid}") && f["properties"]["radius_m"].is_number())
            .unwrap()["properties"]["radius_m"]
            .as_f64()
            .unwrap();
        assert!(middles(&fc, &p, centre).iter().any(|(f, _)| f["properties"]["hot"] == format!("j-{uid}")), "the junction's ground is a shape");
        // the shape is pressed for itself (`places-ground`), so the target round the point adds nothing to it: no
        // more than the 24 px every junction has, or it would take the wide street past the narrow one's corners
        assert_eq!(target, 0.0, "only the floor of the target is round the point");
    }

    #[test]
    fn where_only_two_of_a_junctions_streets_are_in_the_place_its_ground_is_a_disc_its_press_target_covers() {
        let mut v = City::from_network(&two_streets(4.0, 4.0, 60.0), "Bendville").view(0);
        let shared = v.edges[0].b;
        let n = v.nodes.iter_mut().find(|n| n.uid == shared).unwrap();
        (n.junction, n.edited, n.radius_mm) = (true, true, 1_000); // its other streets are outside the place
        let wide_mm = 9_001; // both wider than the junction's radius, at an odd width
        v.edges.iter_mut().for_each(|e| e.row_mm = wide_mm);
        let p = projection(&v);
        let fc = parse(&places(&v, &p, &v.edges.iter().map(|e| e.uid).collect()));
        let mine: Vec<&Value> = fc["features"].as_array().unwrap().iter().filter(|f| f["properties"]["hot"] == format!("j-{shared}")).collect();
        let ground = mine.iter().find(|f| f["properties"]["joint_m"].is_number()).expect("a disc, its ground");
        assert_eq!(
            (&ground["properties"]["ground"], &ground["properties"]["status"], &ground["properties"]["conflict"]),
            (&json!(true), &json!("changed"), &json!(false))
        );
        let target = mine.iter().find(|f| f["properties"]["radius_m"].is_number()).unwrap()["properties"]["radius_m"].as_f64().unwrap();
        assert!((ground["properties"]["joint_m"].as_f64().unwrap() - wide_mm as f64 / 2000.0).abs() < 1e-9);
        assert!((target - wide_mm as f64 / 2000.0).abs() < 1e-9, "the press target is as big as the ground: {target}");
    }

    #[test]
    fn where_two_streets_meet_end_to_end_the_joint_is_a_streets_and_not_ground() {
        let v = City::from_network(&two_streets(4.0, 4.0, 60.0), "Bendville").view(0);
        assert!(v.nodes.iter().all(|n| !n.junction), "two streets end to end are not a junction");
        let fc = parse(&places(&v, &projection(&v), &BTreeSet::new()));
        let joint = fc["features"].as_array().unwrap().iter().find(|f| f["properties"]["joint_m"].is_number()).unwrap();
        assert!(joint["properties"]["hot"].as_str().unwrap().starts_with("s-"));
        assert!(joint["properties"]["ground"].is_null());
    }

    #[test]
    fn a_joint_says_how_its_street_stands_so_it_is_drawn_in_the_streets_colour() {
        let mut v = City::from_network(&two_streets(4.0, 4.0, 60.0), "Bendville").view(0);
        let p = projection(&v);
        v.edges.iter_mut().for_each(|e| e.edited = true);
        let fc = parse(&places(&v, &p, &v.edges.iter().map(|e| e.uid).collect()));
        let all = fc["features"].as_array().unwrap();
        let joint = all.iter().find(|f| f["properties"]["joint_m"].is_number()).unwrap();
        assert_eq!(joint["properties"]["status"], "changed");
        assert_eq!(joint["properties"]["conflict"], true);
        assert_eq!(joint["geometry"]["type"], "Point");
        assert!(joint["properties"]["radius_m"].is_null(), "a joint is not a junction");
    }

    /// `middle` at the origin for streets of these widths (mm) on these bearings (degrees), if it makes a shape, checked
    /// for what every shape must be: a closed, simple ring, anticlockwise, round the node (strictly inside it), over
    /// every corner of every street's square end, and no further out than twice the widest half-width along a street.
    fn middle_of(arms: &[(i32, f64)]) -> Option<Vec<[f64; 2]>> {
        let streets = arms.iter().map(|&(row_mm, deg)| {
            let (s, c) = deg.to_radians().sin_cos();
            ([(100_000.0 * c).round() as i32, (100_000.0 * s).round() as i32], row_mm)
        });
        let ring = middle([0, 0], streets)?;
        let case = format!("{arms:?}: {ring:?}");
        assert_eq!(ring.first(), ring.last(), "closed: {case}");
        let n = ring.len() - 1;
        assert!(n >= 3, "{case}");
        let edges: Vec<([f64; 2], [f64; 2])> = (0..n).map(|i| (ring[i], ring[i + 1])).collect();
        let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
        for i in 0..n {
            for j in i + 2..n {
                if i == 0 && j == n - 1 {
                    continue; // they share the first point
                }
                let ((p, q), (r, t)) = (edges[i], edges[j]);
                // a sign change counts only past float noise: the outline has runs of points in a line
                let side = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
                    let v = cross(o, a, b);
                    if v.abs() <= 1e-6 * (a[0] - o[0]).hypot(a[1] - o[1]) * (b[0] - o[0]).hypot(b[1] - o[1]) { 0.0 } else { v.signum() }
                };
                let crosses = side(p, q, r) * side(p, q, t) < 0.0 && side(r, t, p) * side(r, t, q) < 0.0;
                assert!(!crosses, "simple: edges {i} and {j} cross: {case}");
            }
        }
        // and does not double back: no point lies on an edge it is not an end of
        for (k, q) in ring[..n].iter().enumerate() {
            for (i, &(a, b)) in edges.iter().enumerate() {
                if i == k || (i + 1) % n == k {
                    continue;
                }
                let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
                let t = (((q[0] - a[0]) * ex + (q[1] - a[1]) * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
                let d = (a[0] + t * ex - q[0]).hypot(a[1] + t * ey - q[1]);
                assert!(d > 0.01, "point {k} lies on edge {i}: {case}");
            }
        }
        let area: f64 = edges.iter().map(|(a, b)| a[0] * b[1] - b[0] * a[1]).sum::<f64>() / 2.0;
        assert!(area > 0.0, "anticlockwise, with area: {area} {case}");
        assert!(holds(&ring, [0.0, 0.0], 0.0) && !holds_near_edge(&ring, [0.0, 0.0], 1.0), "the node is inside: {case}");
        let widest = arms.iter().map(|a| a.0).max().unwrap() as f64 / 2.0;
        for &(row_mm, deg) in arms {
            let (s, c) = deg.to_radians().sin_cos();
            let h = row_mm as f64 / 2.0;
            // under it, or on a street as wide or wider leaving within half a degree, which it was taken into
            let taken = |q: [f64; 2]| {
                arms.iter().any(|&(w, d)| {
                    let (ds, dc) = d.to_radians().sin_cos();
                    // up to half a degree apart (the code measures it on the rounded points): its corner can sit that
                    // much behind the wider one's end
                    let behind = 2.0 + h * (0.51f64).to_radians().sin();
                    let apart = (d - deg).rem_euclid(360.0);
                    w >= row_mm
                        && d != deg
                        && apart.min(360.0 - apart) < 0.51
                        && q[0] * dc + q[1] * ds >= -behind
                        && (dc * q[1] - ds * q[0]).abs() <= w as f64 / 2.0 + 2.0
                })
            };
            for q in [[-s * h, c * h], [s * h, -c * h]] {
                assert!(holds(&ring, q, 2.0) || taken(q), "the corner {q:?} of the street at {deg} degrees is under it: {case}");
            }
        }
        // nothing past a kerb: every point of the outline is on a street, or between two streets next to each other
        // round the node, up to where their kerbs meet (no further than the cap), or across the outside of a bend
        let mut by_bearing: Vec<(f64, [f64; 2], f64)> = arms
            .iter()
            .map(|&(row_mm, deg)| {
                let (s, c) = deg.to_radians().sin_cos();
                let (x, y) = ((100_000.0 * c).round(), (100_000.0 * s).round());
                let len = x.hypot(y);
                (y.atan2(x), [x / len, y / len], row_mm as f64 / 2.0)
            })
            .collect();
        by_bearing.sort_by(|a, b| a.0.total_cmp(&b.0).then(b.2.total_cmp(&a.2)));
        // streets leaving within half a degree of each other are one, as wide as the widest
        let same = (0.5f64).to_radians();
        let mut one: Vec<(f64, [f64; 2], f64)> = Vec::new();
        for arm in by_bearing {
            match one.last_mut() {
                Some(last) if arm.0 - last.0 < same => {
                    if arm.2 > last.2 {
                        *last = arm;
                    }
                }
                _ => one.push(arm),
            }
        }
        if one.len() > 1 && one[0].0 + std::f64::consts::TAU - one[one.len() - 1].0 < same {
            let last = one.pop().unwrap();
            if last.2 > one[0].2 {
                one[0] = last;
            }
        }
        let by_bearing = one;
        let kerb = |(_, d, h): (f64, [f64; 2], f64), side: f64, t: f64| [side * h * -d[1] + t * d[0], side * h * d[0] + t * d[1]];
        let x = |a: [f64; 2], b: [f64; 2]| a[0] * b[1] - a[1] * b[0];
        let in_triangle = |q: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
            let (d1, d2, d3) = (cross(a, b, q), cross(b, c, q), cross(c, a, q));
            let slack = 2.0 * ((b[0] - a[0]).hypot(b[1] - a[1]) + (c[0] - b[0]).hypot(c[1] - b[1]) + (a[0] - c[0]).hypot(a[1] - c[1]));
            !((d1 < -slack || d2 < -slack || d3 < -slack) && (d1 > slack || d2 > slack || d3 > slack))
        };
        let allowed = |q: [f64; 2]| {
            let on_street = by_bearing.iter().any(|&(_, d, h)| q[0] * d[0] + q[1] * d[1] >= -2.0 && x(d, q).abs() <= h + 2.0);
            on_street
                || (0..by_bearing.len()).any(|i| {
                    // between two streets next to each other round the node, no further along either kerb than twice
                    // the widest half-width; across the outside of a bend, the triangle to their ends' corners
                    let (ai, aj) = (by_bearing[i], by_bearing[(i + 1) % by_bearing.len()]);
                    let o = [0.0, 0.0];
                    if x(ai.1, aj.1) > 0.0 {
                        let r = 2.0 * widest;
                        let (l0, l, rr, r0) = (kerb(ai, 1.0, 0.0), kerb(ai, 1.0, r), kerb(aj, -1.0, r), kerb(aj, -1.0, 0.0));
                        // in order round the node: a narrower street's corner can lie inside a wider one's band
                        let mut fan = [l0, l, rr, r0];
                        let from_i = |p: &[f64; 2]| (p[1].atan2(p[0]) - ai.0).rem_euclid(std::f64::consts::TAU);
                        fan.sort_by(|a, b| from_i(a).total_cmp(&from_i(b)));
                        fan.windows(2).any(|w| in_triangle(q, o, w[0], w[1]))
                    } else {
                        in_triangle(q, o, kerb(ai, 1.0, 0.0), kerb(aj, -1.0, 0.0))
                    }
                })
        };
        for (a, b) in &edges {
            for k in 0..=8 {
                let f = k as f64 / 8.0;
                let q = [a[0] + f * (b[0] - a[0]), a[1] + f * (b[1] - a[1])];
                assert!(allowed(q), "{q:?} of the outline is past a kerb: {case}");
            }
        }
        let far = (2.0 * widest).hypot(widest) + 2.0;
        assert!(ring.iter().all(|q| q[0].hypot(q[1]) <= far), "no further than {far}: {case}");
        Some(ring)
    }

    /// Whether `q` is within `slack` of an edge of the closed `ring`.
    fn holds_near_edge(ring: &[[f64; 2]], q: [f64; 2], slack: f64) -> bool {
        ring.windows(2).any(|w| {
            let (a, b) = (w[0], w[1]);
            let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
            let t = (((q[0] - a[0]) * ex + (q[1] - a[1]) * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
            (a[0] + t * ex - q[0]).hypot(a[1] + t * ey - q[1]) <= slack
        })
    }

    /// The area of the closed `ring`, in square millimetres.
    fn area_of(ring: &[[f64; 2]]) -> f64 {
        ring.windows(2).map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1]).sum::<f64>().abs() / 2.0
    }

    #[test]
    fn two_streets_straight_on_make_no_shape() {
        assert!(middle([0, 0], [([0, 1_000], 4_000), ([0, -1_000], 4_000)].into_iter()).is_none());
    }

    #[test]
    fn where_two_streets_cross_the_shape_is_the_whole_ground_they_share_out_to_their_kerbs() {
        // two 8 m streets crossing square: the 8 m square where their kerbs meet, not the diamond of their ends' corners
        let ring = middle_of(&[(8_000, 0.0), (8_000, 90.0), (8_000, 180.0), (8_000, 270.0)]).unwrap();
        assert!((area_of(&ring) - 8_000.0 * 8_000.0).abs() < 1_000.0, "{ring:?}");
        assert!(ring.iter().all(|q| q[0].abs() <= 4_001.0 && q[1].abs() <= 4_001.0), "{ring:?}");
    }

    #[test]
    fn where_a_side_street_meets_a_through_street_the_shape_is_the_box_of_the_side_street_across_it() {
        // a 12 m street east-west, a 4 m one off it to the north: 4 m along the through street, all of it across
        let ring = middle_of(&[(12_000, 0.0), (4_000, 90.0), (12_000, 180.0)]).unwrap();
        assert!((area_of(&ring) - 4_000.0 * 12_000.0).abs() < 1_000.0, "{ring:?}");
        assert!(ring.iter().all(|q| q[0].abs() <= 2_001.0 && q[1].abs() <= 6_001.0), "{ring:?}");
    }

    #[test]
    fn a_through_street_that_bends_a_little_keeps_the_box_across_it_with_no_sudden_change() {
        // a 12 m street and a 6 m one off it at a right angle, the 12 m one going on bent from 35 degrees one way to 35
        // the other: the ground changes a little with each degree, with no step where the bend passes 10 degrees
        let box_at = |deg: i32| area_of(&middle_of(&[(12_000, 0.0), (6_000, 90.0), (12_000, deg as f64)]).unwrap());
        let straight = box_at(180);
        for deg in 145..215 {
            let (a, b) = (box_at(deg), box_at(deg + 1));
            assert!((a - b).abs() < 0.05 * straight, "{deg} to {} degrees: {a} to {b}", deg + 1);
        }
        assert!(box_at(210) > 0.75 * straight, "{} of {straight}", box_at(210));
    }

    #[test]
    fn a_street_off_a_through_street_at_a_sharp_angle_puts_nothing_past_the_through_streets_far_kerb() {
        // an 8 m street east-west, and a 4 m one leaving it 10 degrees off west, to the north
        let ring = middle_of(&[(8_000, 0.0), (8_000, 180.0), (4_000, 170.0)]).unwrap();
        assert!(ring.iter().all(|q| q[1] >= -4_000.0 - 1.0), "nothing south of the south kerb: {ring:?}");
    }

    #[test]
    fn streets_at_sharp_angles_make_a_simple_shape_that_reaches_into_the_angle_no_further_than_the_limit() {
        for arms in [
            vec![(8_000, 0.0), (8_000, 10.0), (8_000, 180.0)],
            vec![(8_000, 0.0), (8_000, 45.0), (8_000, 180.0), (8_000, 225.0)],
            vec![(8_000, 0.0), (8_000, 30.0), (8_000, 180.0), (8_000, 210.0)],
            vec![(8_000, 0.0), (8_000, 20.0), (8_000, 180.0), (8_000, 200.0)],
            vec![(8_000, 0.0), (8_000, 30.0), (8_000, 180.0)],
            vec![(8_000, 0.0), (8_000, 45.0), (8_000, 180.0)],
            vec![(12_000, 0.0), (5_000, 15.0), (12_000, 180.0)],
            vec![(12_000, 180.0), (6_000, 8.0), (6_000, -8.0)],
        ] {
            middle_of(&arms).unwrap(); // the checks of every shape
        }
        let ring = middle_of(&[(8_000, 0.0), (8_000, 25.0), (8_000, 180.0)]).unwrap();
        assert!(ring.iter().any(|q| q[0] > 4_000.0), "it reaches into the sharp angle: {ring:?}");
    }

    #[test]
    fn streets_bending_round_a_node_leave_no_hole_on_the_outside_of_the_bend() {
        for arms in [
            vec![(8_000, 0.0), (6_000, 50.0), (8_000, 100.0)],
            vec![(8_000, 0.0), (8_000, 60.0), (8_000, 120.0)],
            vec![(8_000, 0.0), (8_000, 90.0), (8_000, 150.0)],
            vec![(8_000, 0.0), (8_000, 30.0), (8_000, 60.0)],
            vec![(12_000, 0.0), (6_000, 90.0), (12_000, 170.0)],
            vec![(8_000, 0.0), (8_000, 120.0), (8_000, 240.0)],
        ] {
            middle_of(&arms).unwrap(); // the node and every corner under it
        }
    }

    #[test]
    fn a_street_that_changes_width_at_the_node_or_a_side_street_at_a_slant_puts_nothing_past_a_kerb() {
        for arms in [
            vec![(12_000, 0.0), (4_000, 90.0), (8_000, 180.0)],
            vec![(20_000, 0.0), (6_000, 90.0), (8_000, 180.0)],
            vec![(2_000, 0.0), (8_000, 90.0), (8_000, 180.0)],
            vec![(8_000, 0.0), (12_000, 0.0), (8_000, 90.0)],
            vec![(12_000, 0.0), (4_000, 60.0), (12_000, 180.0)],
            vec![(12_000, 0.0), (4_000, 45.0), (12_000, 180.0)],
            vec![(12_000, 0.0), (4_000, 120.0), (12_000, 180.0)],
        ] {
            let _ = middle_of(&arms); // if there is a shape, it passes the checks of every shape
        }
        // the one with a side street at a slant has a shape, round the node
        assert!(middle_of(&[(12_000, 0.0), (4_000, 60.0), (12_000, 180.0)]).is_some());
    }

    #[test]
    fn any_streets_meeting_make_a_shape_that_passes_the_checks_of_every_shape_or_none() {
        // two thousand meetings of 3 to 6 streets, 2 to 30 m wide, at any bearings, from a fixed seed
        let mut seed: u64 = 0x5eed_c17e;
        let mut next = move |n: u64| {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % n
        };
        for k in 0..2_000 {
            // half of them with streets on the compass points and between them, where coordinates are exact
            let bearing = |r: u64, b: u64| if k % 2 == 1 && r == 0 { 45.0 * (b % 8) as f64 } else { b as f64 / 10.0 };
            let arms: Vec<(i32, f64)> = (0..3 + next(4)).map(|_| (2_000 + 500 * next(57) as i32, bearing(next(2), next(3_600)))).collect();
            let _ = middle_of(&arms);
        }
    }

    #[test]
    fn a_street_that_bends_and_changes_width_at_the_node_makes_ground_with_no_sudden_change() {
        // through streets narrowing at the node, bent from 40 degrees one way to 40 the other, with a side street: the
        // ground changes a little with each degree, also where the bend passes 10 degrees
        for (wide, narrow, side) in [(12_000, 8_000, 4_000), (20_000, 8_000, 4_000)] {
            let ground = |deg: i32| middle_of(&[(wide, 0.0), (side, 90.0), (narrow, deg as f64)]).map_or(0.0, |r| area_of(&r));
            // a degree changes it by less than a twentieth of the square of the widest street
            let step = 0.05 * (wide as f64).powi(2);
            for deg in 140..220 {
                let (a, b) = (ground(deg), ground(deg + 1));
                assert!((a - b).abs() < step, "{wide}/{narrow}: {deg} to {} degrees: {a} to {b}", deg + 1);
            }
        }
        // two streets 10 degrees apart and the one going on: no step at 10 degrees either
        // (below 10 degrees no kerbs meet and the streets are joined by a disc instead)
        let sharp = |tenths: i32| middle_of(&[(8_000, 0.0), (8_000, tenths as f64 / 10.0), (8_000, 180.0)]).map(|r| area_of(&r));
        for tenths in 90..120 {
            if let (Some(a), Some(b)) = (sharp(tenths), sharp(tenths + 1)) {
                assert!((a - b).abs() < 2e6, "{tenths} tenths of a degree: {a} to {b}");
            }
        }
    }

    #[test]
    fn a_side_street_at_a_slant_lights_the_through_street_no_further_than_it_crosses_it() {
        // a 12 m street east-west and a 4 m one off it to the north-east: along the through street, the ground goes no
        // further than where the side street's far kerb leaves the through street's far kerb
        for deg in [15.0f64, 30.0, 45.0, 60.0] {
            let ring = middle_of(&[(12_000, 0.0), (4_000, deg), (12_000, 180.0)]).unwrap();
            let (s, t) = (deg.to_radians().sin(), deg.to_radians().tan());
            let along = 6_000.0 / t + 2_000.0 / s;
            assert!(ring.iter().all(|q| q[0].abs() <= along + 1.0), "{deg} degrees: no further than {along}: {ring:?}");
        }
    }

    #[test]
    fn a_footway_leaving_with_a_road_changes_nothing_the_road_makes() {
        // a 2 m way within a tenth of a degree of a 12 m street, either side of it, or the same bearing in either order
        let road = [(12_000, 0.0), (8_000, 90.0), (12_000, 180.0)];
        let alone = area_of(&middle_of(&road).unwrap());
        for way in [-0.1, 0.0, 0.1] {
            for arms in [[road[0], (2_000, way), road[1], road[2]], [(2_000, way), road[0], road[1], road[2]]] {
                let with = area_of(&middle_of(&arms).unwrap());
                assert!((with - alone).abs() < 0.01 * alone, "{arms:?}: {with} against {alone}");
            }
        }
    }

    #[test]
    fn streets_leaving_together_make_a_simple_shape_or_none() {
        for arms in [
            vec![(12_000, 0.0), (12_000, 180.0), (5_000, -4.0)],
            vec![(8_000, 0.0), (8_000, 0.0), (8_000, 90.0)],
            vec![(8_000, 0.0), (8_000, 3.0), (8_000, 180.0)],
            vec![(20_000, 270.0), (20_000, 100.19), (10_000, 270.0)],
            vec![(20_000, 90.0), (10_000, 180.0), (20_000, 270.0)],
        ] {
            let _ = middle_of(&arms); // if there is a shape, it passes the checks of every shape
        }
    }

    #[test]
    fn a_point_along_a_polyline_has_the_tangent_of_its_segment() {
        let shape = [[0, 0], [10_000, 0], [10_000, 10_000]];
        assert_eq!(point_at(&shape, 15_000.0), Some(([10_000.0, 5_000.0], [0.0, 1.0])));
        assert_eq!(point_at(&shape, 4_000.0), Some(([4_000.0, 0.0], [1.0, 0.0])));
        assert_eq!(point_at(&shape, 0.0), None);
        assert_eq!(point_at(&shape, 20_000.0), None);
        assert_eq!(point_at(&shape, 25_000.0), None);
        assert_eq!(point_at(&[[0, 0], [0, 0]], 1.0), None);
    }

    #[test]
    fn a_boundary_is_a_tick_across_the_street_two_metres_past_each_edge() {
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].zone_breaks_mm = vec![30_000];
        let fc = parse(&zone_ticks(&v, &p));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 1);
        let f = &features[0];
        assert_eq!(f["geometry"]["type"], "LineString");
        assert_eq!(f["properties"]["street"], format!("s-{}", v.edges[0].uid));
        assert_eq!(f["properties"]["n"], 1);
        let line = f["geometry"]["coordinates"].as_array().unwrap();
        assert_eq!(line.len(), 2);
        let c = |i: usize, j: usize| line[i][j].as_f64().unwrap();
        assert_eq!(c(0, 0), c(1, 0), "the tick runs north-south across an east-west street");
        let across_mm = v.edges[0].row_mm + 2 * TICK_OVERHANG_MM;
        let metres = (c(0, 1) - c(1, 1)).abs() * 111_195.0;
        assert!((metres - across_mm as f64 / 1000.0).abs() < 0.2, "{metres} m for {across_mm} mm");
        // centred on the centreline at 30 m
        let [x0, y0] = v.edges[0].shape_mm[0];
        let mid = position(&p, x0 + 30_000, y0);
        assert!((c(0, 0) - mid[0]).abs() < 2e-6 && ((c(0, 1) + c(1, 1)) / 2.0 - mid[1]).abs() < 2e-6);
    }

    #[test]
    fn a_band_runs_along_the_street_instead_of_across() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let shape = v.edges[0].shape_mm.clone();
        assert_eq!(shape.len(), 3);
        v.edges[0].zone_breaks_mm = vec![90_000];
        v.edges[0].zone_bands_mm = vec![(1, 70_000, 90_000)];
        let fc = parse(&zone_ticks(&v, &p));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 1, "the band replaces the tick");
        let f = &features[0];
        assert_eq!(f["geometry"]["type"], "LineString");
        assert_eq!(f["properties"]["band"], true);
        assert_eq!(f["properties"]["n"], 1);
        assert_eq!(f["properties"]["street"], format!("s-{}", v.edges[0].uid));
        let line = f["geometry"]["coordinates"].as_array().unwrap();
        let at = |x: i32, y: i32| position(&p, x, y);
        let (x0, y0) = (shape[0][0], shape[0][1]);
        assert_eq!(line.len(), 3, "its two ends and the bend between");
        assert_eq!(line[0], serde_json::json!(at(x0 + 70_000, y0)));
        assert_eq!(line[1], serde_json::json!(at(shape[1][0], shape[1][1])));
        let (dx, dy) = ((shape[2][0] - shape[1][0]) as f64, (shape[2][1] - shape[1][1]) as f64);
        let k = 10_000.0 / dx.hypot(dy);
        let end = at((shape[1][0] as f64 + dx * k).round() as i32, (shape[1][1] as f64 + dy * k).round() as i32);
        let got = &line[2];
        assert!((got[0].as_f64().unwrap() - end[0]).abs() < 2e-6 && (got[1].as_f64().unwrap() - end[1]).abs() < 2e-6);
    }

    #[test]
    fn a_boundary_without_a_transition_keeps_its_tick() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].zone_breaks_mm = vec![20_000, 60_000];
        v.edges[0].zone_bands_mm = vec![(2, 40_000, 60_000)];
        let fc = parse(&zone_ticks(&v, &p));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 2);
        assert!(features[0]["properties"].get("band").is_none());
        assert_eq!(features[0]["properties"]["n"], 1);
        assert_eq!(features[0]["geometry"]["coordinates"].as_array().unwrap().len(), 2);
        assert_eq!((features[1]["properties"]["n"].clone(), features[1]["properties"]["band"].clone()), (2.into(), true.into()));
    }

    #[test]
    fn a_clipped_band_does_not_take_the_previous_boundary() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].zone_breaks_mm = vec![40_000, 60_000];
        v.edges[0].zone_bands_mm = vec![(2, 40_000, 60_000)];
        let fc = parse(&zone_ticks(&v, &p));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 2);
        assert_eq!((features[0]["properties"]["n"].clone(), features[0]["properties"].get("band").is_none()), (1.into(), true));
        assert_eq!((features[1]["properties"]["n"].clone(), features[1]["properties"]["band"].clone()), (2.into(), true.into()));
    }

    #[test]
    fn a_street_without_boundaries_adds_nothing_and_places_is_unchanged() {
        let v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let fc = parse(&zone_ticks(&v, &p));
        assert_eq!(fc["type"], "FeatureCollection");
        assert!(fc["features"].as_array().unwrap().is_empty());
        assert!(!places(&v, &p, &BTreeSet::new()).contains("zone"));
    }

    use crate::street::model::{MapStop, MapUse, StopKind, Use};

    fn a_use(side: Side, from_mm: i32, to_mm: i32, lo_mm: i32, hi_mm: i32) -> MapUse {
        MapUse { side, use_: Use::Parking, from_mm, to_mm, lo_mm, hi_mm }
    }

    fn a_stop(name: Option<&str>) -> MapStop {
        MapStop { side: Side::Right, kind: StopKind::Bulb, name: name.map(String::from), from_mm: 20_000, to_mm: 32_000, lo_mm: 3_500, hi_mm: 6_000 }
    }

    fn ring_of(f: &Value) -> Vec<(f64, f64)> {
        f["geometry"]["coordinates"][0].as_array().unwrap().iter().map(|c| (c[0].as_f64().unwrap(), c[1].as_f64().unwrap())).collect()
    }

    fn signed_area(ring: &[(f64, f64)]) -> f64 {
        ring.windows(2).map(|w| w[0].0 * w[1].1 - w[1].0 * w[0].1).sum::<f64>() / 2.0
    }

    #[test]
    fn a_use_is_a_polygon_beside_the_street() {
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].curb_mm = vec![a_use(Side::Right, 10_000, 16_000, 3_500, 6_000)];
        let fc = parse(&curbside(&v, &p));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 1);
        let f = &features[0];
        assert_eq!(f["geometry"]["type"], "Polygon");
        assert_eq!(f["properties"]["hot"], format!("s-{}", v.edges[0].uid));
        assert_eq!(f["properties"]["use"], "parking");
        assert_eq!(f["properties"]["side"], "right");
        let ring = ring_of(f);
        assert_eq!(ring.len(), 5, "four corners, closed");
        assert_eq!(ring[0], ring[4]);
        let [x0, y0] = v.edges[0].shape_mm[0];
        let corner = |along: i32, across: i32| {
            let [lon, lat] = p.lon_lat(x0 + along, y0 + across);
            (lon, lat)
        };
        for (got, want) in ring[..4].iter().zip([corner(10_000, 3_500), corner(10_000, 6_000), corner(16_000, 6_000), corner(16_000, 3_500)]) {
            assert!((got.0 - want.0).abs() < 2e-6 && (got.1 - want.1).abs() < 2e-6, "{got:?} is not {want:?}");
        }
    }

    #[test]
    fn left_is_north_of_an_eastward_street() {
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].curb_mm = vec![a_use(Side::Left, 10_000, 16_000, -6_000, -3_500)];
        let ring = ring_of(&parse(&curbside(&v, &p))["features"][0]);
        let [x0, y0] = v.edges[0].shape_mm[0];
        let line_lat = p.lon_lat(x0, y0)[1];
        assert!(ring.iter().all(|c| c.1 > line_lat), "every corner north of the line");
    }

    #[test]
    fn a_bend_does_not_spike() {
        let shape = [[0, 0], [20_000, 0], [20_000, 20_000]];
        let ring = offset_ring(&shape, 10_000.0, 30_000.0, 3_500.0, 6_000.0);
        let near = |q: [f64; 2]| {
            shape
                .windows(2)
                .map(|w| {
                    let (ax, ay, bx, by) = (w[0][0] as f64, w[0][1] as f64, w[1][0] as f64, w[1][1] as f64);
                    let t = (((q[0] - ax) * (bx - ax) + (q[1] - ay) * (by - ay)) / ((bx - ax).powi(2) + (by - ay).powi(2))).clamp(0.0, 1.0);
                    (q[0] - (ax + t * (bx - ax))).hypot(q[1] - (ay + t * (by - ay)))
                })
                .fold(f64::INFINITY, f64::min)
        };
        assert!(ring.iter().all(|&q| near(q) <= 2.0 * 6_000.0 + 1e-6), "a vertex spikes: {ring:?}");
        assert_eq!(ring.first(), ring.last(), "closed");
        // layout y is south; the ring is wound anticlockwise on the map (y north), so clockwise in layout millimetres
        let layout_area: f64 = ring.windows(2).map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1]).sum::<f64>() / 2.0;
        assert!(layout_area < 0.0, "{layout_area}");
    }

    #[test]
    fn a_use_ring_is_wound_anticlockwise_on_the_map() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].curb_mm = vec![a_use(Side::Right, 10_000, 100_000, 3_500, 6_000)];
        assert!(signed_area(&ring_of(&parse(&curbside(&v, &p))["features"][0])) > 0.0);
    }

    #[test]
    fn a_stop_is_a_platform_and_a_dot_with_its_street() {
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].stops_mm = vec![a_stop(Some("Hôtel-de-Ville"))];
        let fc = parse(&stops(&v, &p));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 2);
        let hot = format!("s-{}", v.edges[0].uid);
        let (platform, dot) = (&features[0], &features[1]);
        assert_eq!((platform["geometry"]["type"].as_str(), platform["properties"]["part"].as_str()), (Some("Polygon"), Some("platform")));
        assert_eq!((dot["geometry"]["type"].as_str(), dot["properties"]["part"].as_str()), (Some("Point"), Some("dot")));
        for f in features {
            assert_eq!(f["properties"]["hot"], hot);
            assert_eq!(f["properties"]["kind"], "bulb");
            assert_eq!(f["properties"]["side"], "right");
        }
        assert_eq!(dot["properties"]["name"], "Hôtel-de-Ville");
        let [x0, y0] = v.edges[0].shape_mm[0];
        let [lon, lat] = p.lon_lat(x0 + 26_000, y0 + 6_000);
        let c = dot["geometry"]["coordinates"].as_array().unwrap();
        assert!((c[0].as_f64().unwrap() - lon).abs() < 2e-6 && (c[1].as_f64().unwrap() - lat).abs() < 2e-6, "the middle, on the outer edge");
    }

    #[test]
    fn an_unnamed_stop_has_no_name_property() {
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].stops_mm = vec![a_stop(None)];
        let fc = parse(&stops(&v, &p));
        assert!(fc["features"][1]["properties"].get("name").is_none());
    }

    #[test]
    fn a_city_without_uses_has_empty_collections() {
        let v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        assert!(parse(&curbside(&v, &p))["features"].as_array().unwrap().is_empty());
        assert!(parse(&stops(&v, &p))["features"].as_array().unwrap().is_empty());
    }

    #[test]
    fn places_are_unchanged_by_uses() {
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        let before = places(&v, &p, &BTreeSet::new());
        v.edges[0].curb_mm = vec![a_use(Side::Right, 10_000, 16_000, 3_500, 6_000)];
        v.edges[0].stops_mm = vec![a_stop(None)];
        assert_eq!(places(&v, &p, &BTreeSet::new()), before);
    }

    #[test]
    fn a_sharp_bend_is_held_at_twice_the_offset() {
        // a turn of about 166 degrees: 1 / cos(half the turn) is far above the cap of 2
        let line = [[0.0, 0.0], [20_000.0, 0.0], [0.0, 5_000.0]];
        let out = offset_line(&line, 3_000.0);
        let d = (out[1][0] - line[1][0]).hypot(out[1][1] - line[1][1]);
        assert!((d - 6_000.0).abs() < 1e-6, "{d}");
    }

    #[test]
    fn a_line_that_doubles_back_offsets_by_the_first_segments_normal() {
        let out = offset_line(&[[0.0, 0.0], [1_000.0, 0.0], [0.0, 0.0]], 3_000.0);
        assert_eq!(out[1], [1_000.0, 3_000.0], "the first segment's right normal, at scale 1");
    }

    #[test]
    fn a_left_stop_has_its_dot_on_the_low_edge_and_its_ring_anticlockwise() {
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].stops_mm = vec![MapStop { side: Side::Left, lo_mm: -6_000, hi_mm: -3_500, ..a_stop(None) }];
        let fc = parse(&stops(&v, &p));
        assert_eq!(fc["features"][0]["properties"]["side"], "left");
        assert!(signed_area(&ring_of(&fc["features"][0])) > 0.0);
        let [x0, y0] = v.edges[0].shape_mm[0];
        let [lon, lat] = p.lon_lat(x0 + 26_000, y0 - 6_000);
        let c = fc["features"][1]["geometry"]["coordinates"].as_array().unwrap();
        assert!((c[0].as_f64().unwrap() - lon).abs() < 2e-6 && (c[1].as_f64().unwrap() - lat).abs() < 2e-6);
    }
}
