//! The hatch patterns that tell the pieces of a street apart without colour, and
//! the line-art elevation symbols drawn on the street's section.

/// One distinct texture per kind of piece.
pub const HATCH: [(&str, &str); 12] = [
    (
        "sidewalk",
        r##"<pattern id="h-sidewalk" width="8" height="8" patternUnits="userSpaceOnUse"><circle cx="2" cy="2" r="0.9"/><circle cx="6" cy="6" r="0.9"/></pattern>"##,
    ),
    (
        "planting",
        r##"<pattern id="h-planting" width="9" height="8" patternUnits="userSpaceOnUse"><path d="M1.5,7 L2.5,3 M5,7 L4.5,2.5 M8,7 L8.8,3.5"/></pattern>"##,
    ),
    ("bike", r##"<pattern id="h-bike" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><path d="M0,0 L0,6"/></pattern>"##),
    ("travel", r##"<pattern id="h-travel" width="14" height="12" patternUnits="userSpaceOnUse"><path d="M1,3 L6,3 M8,9 L13,9"/></pattern>"##),
    ("bus", r##"<pattern id="h-bus" width="4" height="4" patternUnits="userSpaceOnUse" patternTransform="rotate(-45)"><path d="M0,0 L0,4"/></pattern>"##),
    ("parking", r##"<pattern id="h-parking" width="6" height="5" patternUnits="userSpaceOnUse"><path d="M0,2.5 L6,2.5"/></pattern>"##),
    ("median", r##"<pattern id="h-median" width="7" height="7" patternUnits="userSpaceOnUse"><path d="M0,0 L7,0 M0,0 L0,7"/></pattern>"##),
    ("shoulder", r##"<pattern id="h-shoulder" width="12" height="10" patternUnits="userSpaceOnUse"><path d="M0,5 L4,5 M6,5 L7,5 M9,5 L10,5"/></pattern>"##),
    ("bikerack", r##"<pattern id="h-bikerack" width="10" height="8" patternUnits="userSpaceOnUse"><path d="M1,7 L1,3 Q5,0 9,3 L9,7"/></pattern>"##),
    ("bikeshare", r##"<pattern id="h-bikeshare" width="8" height="8" patternUnits="userSpaceOnUse"><circle cx="4" cy="4" r="2.2"/></pattern>"##),
    ("pole", r##"<pattern id="h-pole" width="6" height="8" patternUnits="userSpaceOnUse"><path d="M3,0 L3,8"/></pattern>"##),
    ("loading", r##"<pattern id="h-loading" width="10" height="8" patternUnits="userSpaceOnUse"><path d="M0,6 L5,1 L10,6"/></pattern>"##),
];

/// Surface materials: the surface swatches and the paving course along each slab.
pub const MATERIAL_HATCH: [(&str, &str); 8] = [
    (
        "asphalt",
        r##"<pattern id="m-asphalt" width="5" height="5" patternUnits="userSpaceOnUse"><circle cx="1.2" cy="1.2" r="0.7"/><circle cx="3.7" cy="3.7" r="0.7"/></pattern>"##,
    ),
    (
        "concrete",
        r##"<pattern id="m-concrete" width="12" height="10" patternUnits="userSpaceOnUse"><path d="M2,8 L4.5,3.5 L7,8 Z"/><circle cx="9.5" cy="3" r="0.8"/></pattern>"##,
    ),
    ("permeable", r##"<pattern id="m-permeable" width="8" height="8" patternUnits="userSpaceOnUse"><rect x="1.5" y="1.5" width="5" height="5"/></pattern>"##),
    ("brick", r##"<pattern id="m-brick" width="12" height="8" patternUnits="userSpaceOnUse"><path d="M0,0 H12 M0,4 H12 M3,0 V4 M9,4 V8"/></pattern>"##),
    ("grass", r##"<pattern id="m-grass" width="9" height="8" patternUnits="userSpaceOnUse"><path d="M1.5,7 L2.5,3 M5,7 L4.5,2.5 M8,7 L8.8,3.5"/></pattern>"##),
    (
        "trees",
        r##"<pattern id="m-trees" width="12" height="12" patternUnits="userSpaceOnUse"><circle cx="6" cy="6" r="4"/><circle cx="6" cy="6" r="0.8"/></pattern>"##,
    ),
    (
        "planted",
        r##"<pattern id="m-planted" width="12" height="10" patternUnits="userSpaceOnUse"><circle cx="3.5" cy="3.5" r="2"/><circle cx="9" cy="7.5" r="1.4"/></pattern>"##,
    ),
    (
        "gravel",
        r##"<pattern id="m-gravel" width="11" height="9" patternUnits="userSpaceOnUse"><circle cx="2" cy="2" r="1.1"/><circle cx="7.5" cy="3" r="0.6"/><circle cx="4.5" cy="7" r="0.9"/><circle cx="9.5" cy="7.5" r="0.5"/></pattern>"##,
    ),
];

/// Curbs.
pub const CURB_HATCH: [(&str, &str); 7] = [
    ("granite", r##"<pattern id="c-granite" width="5" height="5" patternUnits="userSpaceOnUse"><path d="M0,0 L5,5 M5,0 L0,5"/></pattern>"##),
    (
        "concrete",
        r##"<pattern id="c-concrete" width="5" height="5" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><path d="M0,0 L0,5"/></pattern>"##,
    ),
    (
        "asphalt",
        r##"<pattern id="c-asphalt" width="4" height="4" patternUnits="userSpaceOnUse"><circle cx="1" cy="1" r="0.7"/><circle cx="3" cy="3" r="0.7"/></pattern>"##,
    ),
    (
        "planted",
        r##"<pattern id="c-planted" width="6" height="6" patternUnits="userSpaceOnUse"><circle cx="1.8" cy="1.8" r="1.2"/><circle cx="4.6" cy="4.4" r="0.8"/></pattern>"##,
    ),
    (
        "bikefriendly",
        r##"<pattern id="c-bikefriendly" width="5" height="5" patternUnits="userSpaceOnUse"><path d="M0,5 L5,0"/><circle cx="1.2" cy="1.2" r="0.6"/></pattern>"##,
    ),
    ("island", r##"<pattern id="c-island" width="6" height="6" patternUnits="userSpaceOnUse"><path d="M0,3 H6 M3,0 V6"/></pattern>"##),
    ("kassel", r##"<pattern id="c-kassel" width="5" height="5" patternUnits="userSpaceOnUse" patternTransform="rotate(-45)"><path d="M0,0 L0,5"/></pattern>"##),
];

// ---- elevation symbols ------------------------------------------------------------
//
// Symbol origin is bottom-centre; y runs up as negative. Stroke width is
// constant on screen (see `.sym` in style.css), so a symbol can be scaled freely.

/// A group of drawing moved `x` along and `y` down and scaled by `s`.
fn at(x: f64, y: f64, s: f64, inner: &str) -> String {
    format!("<g transform=\"translate({x} {y}) scale({s})\">{inner}</g>")
}

fn person(x: f64, s: f64, variant: usize) -> String {
    at(
        x,
        0.0,
        s,
        &format!(
            "<circle cx=\"0\" cy=\"-58\" r=\"6\"/><path class=\"f-coat-{variant}\" d=\"M-7,-50 Q-8,-52 -5,-52 L5,-52 Q8,-52 7,-50 L9,-24 L-9,-24 Z\"/><path class=\"o\" d=\"M-4,-24 L-5,0 M4,-24 L5,0\"/>"
        ),
    )
}

fn tree(x: f64, y: f64, s: f64) -> String {
    at(
        x,
        y,
        s,
        "<path class=\"o\" d=\"M-3,0 L-3,-44 M3,0 L3,-44\"/><path class=\"f-tree\" d=\"M-22,-58 C-33,-62 -31,-86 -17,-88 C-15,-103 11,-107 17,-93 C33,-93 36,-71 24,-63 C21,-49 -13,-49 -22,-58 Z\"/><path class=\"o\" d=\"M0,-44 L0,-68 M0,-56 L-11,-68 M0,-60 L10,-76\"/>",
    )
}

fn shrub(x: f64, y: f64, s: f64) -> String {
    at(
        x,
        y,
        s,
        "<path class=\"f-tree\" d=\"M-15,0 C-18,-14 -7,-24 0,-19 C6,-26 18,-15 15,0 Z\"/><path class=\"o\" d=\"M0,-19 L0,-4 M-6,-14 L-4,-4 M7,-15 L5,-4\"/>",
    )
}

fn cyclist(x: f64, s: f64) -> String {
    at(
        x,
        0.0,
        s,
        "<circle cx=\"-16\" cy=\"-11\" r=\"11\"/><circle cx=\"16\" cy=\"-11\" r=\"11\"/><path class=\"o\" d=\"M-16,-11 L-4,-29 L11,-29 L16,-11 M-4,-29 L2,-11 L-16,-11 M11,-29 L9,-35 M6,-35 L13,-35\"/><path class=\"o\" d=\"M-4,-31 L4,-47 L11,-34 M1,-33 L4,-20 L2,-11\"/><circle cx=\"6\" cy=\"-53\" r=\"5\"/>",
    )
}

fn cone(x: f64, s: f64) -> String {
    at(
        x,
        0.0,
        s,
        "<path class=\"f-van\" d=\"M-9,0 L-4,-34 L4,-34 L9,0 Z\"/><path d=\"M-6.5,-14 L6.5,-14 M-5.2,-24 L5.2,-24\"/><path class=\"o\" d=\"M-13,0 L13,0\"/>",
    )
}

fn car(x: f64, s: f64) -> String {
    at(
        x,
        0.0,
        s,
        "<path class=\"f-car\" d=\"M-46,-8 L-46,-19 C-46,-23 -42,-24 -38,-25 L-26,-27 L-16,-38 C-14,-40 -12,-40 -8,-40 L14,-40 C18,-40 20,-39 22,-37 L32,-27 L42,-25 C46,-24 48,-22 48,-18 L48,-8 Z\"/><path d=\"M-14,-27 L-8,-36 L4,-36 L4,-27 Z\"/><path d=\"M9,-27 L9,-36 L16,-36 L27,-27 Z\"/><circle cx=\"-28\" cy=\"-8\" r=\"8\"/><circle cx=\"28\" cy=\"-8\" r=\"8\"/>",
    )
}

fn bus(x: f64, s: f64) -> String {
    let windows: String = [-60, -40, -20, 0, 20].iter().map(|wx| format!("<rect x=\"{wx}\" y=\"-58\" width=\"16\" height=\"20\"/>")).collect();
    at(
        x,
        0.0,
        s,
        &format!(
            "<path class=\"f-bus\" d=\"M-72,-7 L-72,-58 Q-72,-66 -64,-66 L64,-66 Q72,-66 72,-58 L72,-7 Z\"/>{windows}<path d=\"M42,-58 L58,-58 L58,-14 L42,-14 Z\"/><path class=\"o\" d=\"M-72,-44 L-64,-44\"/><circle cx=\"-46\" cy=\"-9\" r=\"9\"/><circle cx=\"44\" cy=\"-9\" r=\"9\"/>"
        ),
    )
}

fn van(x: f64, s: f64) -> String {
    at(
        x,
        0.0,
        s,
        "<path class=\"f-van\" d=\"M-52,-8 L-52,-58 L10,-58 L10,-8 Z\"/><path d=\"M10,-8 L10,-44 L30,-44 L44,-26 L54,-24 L54,-8 Z\"/><path d=\"M15,-40 L28,-40 L38,-27 L15,-27 Z\"/><circle cx=\"-30\" cy=\"-8\" r=\"8\"/><circle cx=\"34\" cy=\"-8\" r=\"8\"/>",
    )
}

fn post_p(x: f64, s: f64) -> String {
    at(
        x,
        0.0,
        s,
        "<path class=\"o\" d=\"M0,0 L0,-56\"/><rect x=\"-7\" y=\"-76\" width=\"14\" height=\"14\" rx=\"1\"/><path class=\"o\" d=\"M-2.5,-64 L-2.5,-74 L1.5,-74 Q4,-74 4,-71.5 Q4,-69 1.5,-69 L-2.5,-69\"/>",
    )
}

/// A bicycle parked, wheels and frame, no rider.
fn parked_bike(x: f64, s: f64) -> String {
    at(
        x,
        0.0,
        s,
        "<circle cx=\"-16\" cy=\"-11\" r=\"11\"/><circle cx=\"16\" cy=\"-11\" r=\"11\"/><path class=\"o\" d=\"M-16,-11 L-4,-29 L11,-29 L16,-11 M-4,-29 L2,-11 L-16,-11 M11,-29 L9,-35 M6,-35 L13,-35 M-4,-31 L-4,-35 M-8,-35 L0,-35\"/>",
    )
}

/// Inverted-U racks with a bike locked to the near one.
fn bike_rack(x: f64, s: f64) -> String {
    at(
        x,
        0.0,
        s,
        &(parked_bike(-6.0, 1.0)
            + "<path class=\"o\" d=\"M16,0 L16,-30 Q16,-38 24,-38 Q32,-38 32,-30 L32,0\"/><path class=\"o\" d=\"M-44,0 L-44,-30 Q-44,-38 -36,-38 Q-28,-38 -28,-30 L-28,0\"/>"),
    )
}

/// A bikeshare station: a docking kiosk with its screen and two docked bikes.
fn bikeshare(x: f64, s: f64) -> String {
    at(
        x,
        0.0,
        s,
        &("<path class=\"f-share\" d=\"M-8,0 L-8,-56 Q-8,-62 -2,-62 L2,-62 Q8,-62 8,-56 L8,0 Z\"/><rect x=\"-5\" y=\"-54\" width=\"10\" height=\"12\" rx=\"1\"/>".to_string()
            + &parked_bike(-38.0, 0.8)
            + &parked_bike(38.0, 0.8)),
    )
}

/// A utility pole: crossarm, insulators, a transformer and two wires.
fn pole(x: f64, s: f64) -> String {
    at(
        x,
        0.0,
        s,
        "<path class=\"o\" d=\"M0,0 L0,-120 M-20,-108 L20,-108 M-14,-96 L14,-96\"/><path class=\"o\" d=\"M-20,-108 L-20,-114 M20,-108 L20,-114 M-20,-114 Q-34,-104 -40,-108 M20,-114 Q34,-104 40,-108\"/><rect x=\"5\" y=\"-86\" width=\"12\" height=\"18\" rx=\"3\"/>",
    )
}

/// A tram: a longer body than the bus, with a pantograph and the rail under it.
fn tram(x: f64, s: f64) -> String {
    let windows: String = [-72, -52, -32, -12, 8, 28, 48].iter().map(|wx| format!("<rect x=\"{wx}\" y=\"-52\" width=\"14\" height=\"18\"/>")).collect();
    at(
        x,
        0.0,
        s,
        &format!(
            "<path class=\"o\" d=\"M-96,0 L96,0\"/><path class=\"f-bus\" d=\"M-84,-8 L-84,-52 Q-84,-60 -76,-60 L76,-60 Q84,-60 84,-52 L84,-8 Z\"/>{windows}<path class=\"o\" d=\"M-8,-60 L-2,-78 M8,-60 L2,-78 M-14,-78 L14,-78\"/><circle cx=\"-62\" cy=\"-6\" r=\"6\"/><circle cx=\"-34\" cy=\"-6\" r=\"6\"/><circle cx=\"34\" cy=\"-6\" r=\"6\"/><circle cx=\"62\" cy=\"-6\" r=\"6\"/>"
        ),
    )
}

/// A shelter: posts, a roof in the transit lane's colour, a back panel and a
/// bench, with a person waiting under it.
fn shelter(x: f64, s: f64) -> String {
    at(
        x,
        0.0,
        s,
        &("<path class=\"o\" d=\"M-34,0 L-34,-60 M34,0 L34,-60\"/><path class=\"o\" d=\"M-34,-56 L-34,-18 M34,-56 L34,-18 M-34,-18 L34,-18\"/><path class=\"f-bus\" d=\"M-42,-60 L42,-60 L42,-68 L-42,-68 Z\"/><path class=\"o\" d=\"M-24,-14 L-4,-14 M-20,-14 L-20,0 M-8,-14 L-8,0\"/>".to_string()
            + &person(18.0, 0.8, 1)),
    )
}

/// What a piece has beyond its kind, which changes what is drawn on it.
#[derive(Clone, Copy, Debug, Default)]
pub struct SymbolOpts<'a> {
    pub shelter: bool,
    pub material: &'a str,
    pub tram: bool,
}

/// The art for a segment `wm` metres wide, in drawing units at 55 px to the metre.
fn art(kind_id: &str, wm: f64, o: SymbolOpts) -> String {
    match kind_id {
        "sidewalk" => {
            if o.shelter {
                return shelter(0.0, 1.0);
            }
            let n = if wm < 2.2 {
                1
            } else if wm < 4.2 {
                2
            } else {
                3
            };
            let gap = wm * 55.0 * 0.3;
            let xs: Vec<f64> = match n {
                1 => vec![0.0],
                2 => vec![-gap * 0.55, gap * 0.55],
                _ => vec![-gap * 0.85, 0.0, gap * 0.85],
            };
            let sizes = [1.0, 0.86, 0.94];
            xs.iter().enumerate().map(|(i, x)| person(*x, sizes[i], i % 2)).collect()
        }
        "planting" => {
            if o.material == "trees" {
                tree(0.0, 0.0, 1.0)
            } else {
                shrub(0.0, 0.0, 1.0)
            }
        }
        "bike" => cyclist(0.0, 1.0),
        "travel" => car(0.0, 1.0),
        "bus" => {
            if o.tram {
                tram(0.0, 1.0)
            } else {
                bus(0.0, 1.0)
            }
        }
        "parking" => car(-8.0, 0.92) + &post_p(wm * 55.0 * 0.36, 0.9),
        "median" => {
            let w = (wm * 55.0 - 10.0).max(20.0) / 2.0;
            let kerb = format!("<rect class=\"f-kerb\" x=\"{}\" y=\"-10\" width=\"{}\" height=\"10\"/>", -w, w * 2.0);
            // The planting stands on the kerb.
            kerb + &if wm >= 2.4 { tree(0.0, -10.0, 0.9) } else { shrub(-w * 0.35, -10.0, 0.8) + &shrub(w * 0.35, -10.0, 0.8) }
        }
        "loading" => van(0.0, 1.0),
        "shoulder" => cone(0.0, 1.0),
        "bikerack" => bike_rack(0.0, 1.0),
        "bikeshare" => bikeshare(0.0, 1.0),
        "pole" => pole(0.0, 1.0),
        _ => String::new(),
    }
}

/// How wide the art is, in drawing units at 55 px to the metre, for shrinking
/// it to fit a narrow segment.
fn native_width(kind_id: &str, wm: f64, o: SymbolOpts) -> f64 {
    match kind_id {
        "sidewalk" => {
            if o.shelter {
                84.0
            } else if wm < 2.2 {
                24.0
            } else if wm < 4.2 {
                70.0
            } else {
                110.0
            }
        }
        "planting" => {
            if o.material == "trees" {
                74.0
            } else {
                34.0
            }
        }
        "bike" => 54.0,
        "travel" => 100.0,
        "bus" => {
            if o.tram {
                176.0
            } else {
                148.0
            }
        }
        "parking" => 112.0,
        "median" => {
            if wm >= 2.4 {
                62.0
            } else {
                52.0
            }
        }
        "loading" => 110.0,
        "shoulder" => 40.0,
        "bikerack" => 80.0,
        "bikeshare" => 108.0,
        "pole" => 80.0,
        _ => 100.0,
    }
}

/// The symbol for a segment of `kind_id`, standing at `cx` on the ground line `ground_y`,
/// `seg_px` wide on the page and `wm` metres wide. Empty when it would be too small to read.
pub fn symbol(kind_id: &str, cx: f64, ground_y: f64, px_per_m: f64, seg_px: f64, wm: f64, f: f64, o: SymbolOpts) -> String {
    let base = (px_per_m / 44.0).min(1.2 * f);
    let fit = seg_px * 0.86 / native_width(kind_id, wm, o);
    let k = base.min(fit);
    if k < 0.3 {
        return String::new();
    }
    format!("<g class=\"sym\" transform=\"translate({cx} {ground_y}) scale({k})\">{}</g>", art(kind_id, wm, o))
}

/// All the hatch patterns as one JSON object of three: kinds, materials, curbs, each
/// from the pattern's name to its SVG. The pages that still draw in script read them.
pub fn hatches_json() -> String {
    let table = |t: &[(&str, &str)]| t.iter().map(|(k, v)| ((*k).to_string(), serde_json::Value::String((*v).to_string()))).collect::<serde_json::Map<_, _>>();
    serde_json::json!({ "HATCH": table(&HATCH), "MATERIAL_HATCH": table(&MATERIAL_HATCH), "CURB_HATCH": table(&CURB_HATCH) }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::KINDS;

    fn sym(kind: &str, wm: f64, o: SymbolOpts) -> String {
        symbol(kind, 100.0, 300.0, wm * 55.0, wm * 55.0, wm, 1.0, o)
    }

    #[test]
    fn every_kind_of_piece_has_a_hatch_and_a_symbol() {
        for k in &KINDS {
            assert!(HATCH.iter().any(|(id, svg)| *id == k.id && svg.contains(&format!("id=\"h-{}\"", k.id))), "hatch for {}", k.id);
            assert!(!art(k.id, 3.0, SymbolOpts::default()).is_empty(), "symbol for {}", k.id);
            // The fall-through arm of `native_width` is only for a kind with no symbol.
            assert!(
                ["sidewalk", "planting", "bike", "travel", "bus", "parking", "median", "loading", "shoulder", "bikerack", "bikeshare", "pole"].contains(&k.id),
                "{} has no width",
                k.id
            );
        }
    }

    #[test]
    fn every_pattern_carries_the_id_it_is_listed_under() {
        for (prefix, table) in [("h", &HATCH[..]), ("m", &MATERIAL_HATCH[..]), ("c", &CURB_HATCH[..])] {
            for (id, svg) in table {
                assert!(svg.starts_with(&format!("<pattern id=\"{prefix}-{id}\"")), "{prefix}-{id}");
                assert!(svg.ends_with("</pattern>"));
            }
        }
    }

    #[test]
    fn a_wider_sidewalk_holds_more_people() {
        let people = |wm: f64| sym("sidewalk", wm, SymbolOpts::default()).matches("f-coat-").count();
        assert_eq!((people(1.5), people(3.0), people(5.0)), (1, 2, 3));
    }

    #[test]
    fn a_sidewalk_with_a_shelter_shows_the_shelter_and_one_person_under_it() {
        let s = sym("sidewalk", 3.0, SymbolOpts { shelter: true, ..SymbolOpts::default() });
        assert!(s.contains("M-42,-60 L42,-60 L42,-68 L-42,-68 Z"));
        assert_eq!(s.matches("f-coat-").count(), 1);
    }

    #[test]
    fn planting_is_a_tree_or_a_shrub_by_its_surface() {
        let tree = sym("planting", 2.0, SymbolOpts { material: "trees", ..SymbolOpts::default() });
        let shrub = sym("planting", 2.0, SymbolOpts { material: "grass", ..SymbolOpts::default() });
        assert!(tree.contains("C-33,-62") && !shrub.contains("C-33,-62"));
        assert!(shrub.contains("M-15,0 C-18,-14"));
    }

    #[test]
    fn a_transit_lane_shows_a_bus_or_a_tram() {
        let bus = sym("bus", 3.5, SymbolOpts::default());
        let tram = sym("bus", 3.5, SymbolOpts { tram: true, ..SymbolOpts::default() });
        assert!(bus.contains("M-72,-7") && !bus.contains("M-84,-8"));
        assert!(tram.contains("M-84,-8"));
    }

    #[test]
    fn a_median_stands_its_planting_on_the_kerb() {
        let wide = sym("median", 3.0, SymbolOpts::default());
        assert!(wide.contains("class=\"f-kerb\"") && wide.contains("translate(0 -10) scale(0.9)"));
        let narrow = sym("median", 1.0, SymbolOpts::default());
        assert_eq!(narrow.matches("translate(").count(), 1 + 2 + 0, "{narrow}");
        assert_eq!(narrow.matches(" -10) scale(0.8)").count(), 2);
    }

    #[test]
    fn a_symbol_shrinks_to_fit_a_narrow_segment_and_goes_when_it_would_be_too_small() {
        let scale = |s: &str| -> f64 { s.split("scale(").nth(1).unwrap().split(')').next().unwrap().parse().unwrap() };
        let roomy = symbol("travel", 0.0, 0.0, 55.0 * 3.0, 3.0 * 55.0, 3.0, 1.0, SymbolOpts::default());
        let tight = symbol("travel", 0.0, 0.0, 55.0 * 3.0, 60.0, 3.0, 1.0, SymbolOpts::default());
        assert!(scale(&tight) < scale(&roomy));
        assert!((scale(&tight) - 60.0 * 0.86 / 100.0).abs() < 1e-9);
        assert_eq!(symbol("travel", 0.0, 0.0, 55.0, 20.0, 3.0, 1.0, SymbolOpts::default()), "");
    }

    #[test]
    fn a_symbol_is_never_drawn_bigger_than_the_page_s_factor_allows() {
        let big = symbol("bike", 0.0, 0.0, 400.0, 400.0, 3.0, 1.0, SymbolOpts::default());
        assert!(big.contains("scale(1.2)"));
        let small_page = symbol("bike", 0.0, 0.0, 400.0, 400.0, 3.0, 0.9, SymbolOpts::default());
        assert!(small_page.contains("scale(1.08)"));
    }

    #[test]
    fn the_hatches_are_sent_to_the_pages_as_three_named_tables() {
        let v: serde_json::Value = serde_json::from_str(&hatches_json()).unwrap();
        assert_eq!(v["HATCH"].as_object().unwrap().len(), HATCH.len());
        assert_eq!(v["MATERIAL_HATCH"].as_object().unwrap().len(), 8);
        assert_eq!(v["CURB_HATCH"].as_object().unwrap().len(), 7);
        assert!(v["HATCH"]["sidewalk"].as_str().unwrap().contains("h-sidewalk"));
    }
}
