//! The colour tokens the stylesheets declare, read by tests: the contrast the drawings and controls promise
//! (WCAG 1.4.3 for text, 1.4.11 for the rest) is measured from the CSS itself.

/// A colour's relative luminance, from `#rrggbb` (WCAG 2.x).
pub fn luminance(hex: &str) -> f64 {
    let c = |i: usize| {
        let v = u8::from_str_radix(&hex[1 + 2 * i..3 + 2 * i], 16).unwrap() as f64 / 255.0;
        if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * c(0) + 0.7152 * c(1) + 0.0722 * c(2)
}

pub fn contrast(a: &str, b: &str) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `(light, dark)` of the token `--name` in a stylesheet: `light-dark(#.., #..)`, or one colour for both themes.
/// A value that is no `#rrggbb` (`transparent`) is `None`.
pub fn token(css: &str, name: &str) -> (Option<String>, Option<String>) {
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

/// `a` laid over `b` at `alpha`.
pub fn blend(a: &str, b: &str, alpha: f64) -> String {
    let c = |h: &str, i: usize| u8::from_str_radix(&h[1 + 2 * i..3 + 2 * i], 16).unwrap() as f64;
    let m = |i: usize| (c(a, i) * alpha + c(b, i) * (1.0 - alpha)).round() as u8;
    format!("#{:02x}{:02x}{:02x}", m(0), m(1), m(2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_selection_outline_is_two_tones_and_one_is_3_to_1_against_every_material() {
        let (app, editor) = (include_str!("../../web/app.css"), include_str!("../../web/editor.css"));
        let (line, casing) = (token(editor, "iso-sel").0.unwrap(), token(editor, "iso-sel-casing").0.unwrap());
        // The two tones meet each other at 3:1, and whatever the material, one of them stands out from it.
        assert!(contrast(&line, &casing) >= 3.0, "line against casing: {:.2}", contrast(&line, &casing));
        for m in crate::shared::catalogue::MATERIALS {
            let mat = token(app, &format!("mat-{}", m.id)).0.unwrap();
            let best = contrast(&line, &mat).max(contrast(&casing, &mat));
            assert!(best >= 3.0, "{}: {best:.2}", m.id);
        }
        // Both drawn, the casing under the line.
        for rule in [".iso-sel-casing {", ".iso-sel {"] {
            assert!(editor.contains(rule), "{rule}");
        }
    }

    #[test]
    fn the_boundaries_of_the_pieces_are_3_to_1_against_the_plate_and_every_lane_fill_in_both_themes() {
        let (app, editor) = (include_str!("../../web/app.css"), include_str!("../../web/editor.css"));
        for rule in ["#drawing .piece {", "#drawing .iso-flat,\n#drawing .iso-top,\n#drawing .iso-side {"] {
            let at = editor.find(rule).unwrap_or_else(|| panic!("{rule}"));
            let body = &editor[at..at + editor[at..].find('}').unwrap()];
            let alpha: f64 = body.split("stroke-opacity: ").nth(1).unwrap().split(';').next().unwrap().parse().unwrap();
            let (ink_l, ink_d) = token(app, "ink");
            for (theme, ink, pick) in [("light", ink_l.unwrap(), 0), ("dark", ink_d.unwrap(), 1)] {
                for name in ["panel", "k-sidewalk", "k-planting", "k-bike", "k-travel", "k-bus", "k-parking", "k-median", "k-loading", "k-shoulder"] {
                    let t = token(app, name);
                    let under = if pick == 0 { t.0 } else { t.1 }.unwrap();
                    let seen = contrast(&blend(&ink, &under, alpha), &under);
                    assert!(seen >= 3.0, "{rule} {name} {theme}: {seen:.2}");
                }
            }
        }
    }

    #[test]
    fn the_accent_is_4_5_to_1_as_text_on_every_ground_it_is_set_on_and_under_its_ink() {
        let app = include_str!("../../web/app.css");
        let accent = token(app, "accent");
        let ink = token(app, "accent-ink");
        let wash = token(app, "accent-wash");
        for (theme, i) in [("light", 0), ("dark", 1)] {
            let pick = |t: &(Option<String>, Option<String>)| if i == 0 { t.0.clone() } else { t.1.clone() };
            let a = pick(&accent).unwrap();
            let mut grounds = vec![("panel", pick(&token(app, "panel"))), ("land", pick(&token(app, "land"))), ("fill", pick(&token(app, "fill")))];
            // The dark wash is translucent, so it is measured over the panel by the audit; the light one is a colour.
            grounds.push(("accent-wash", pick(&wash)));
            for (name, ground) in grounds {
                if let Some(g) = ground {
                    assert!(contrast(&a, &g) >= 4.5, "accent on {name} {theme}: {:.2}", contrast(&a, &g));
                }
            }
            let on = pick(&ink).unwrap();
            assert!(contrast(&a, &on) >= 4.5, "accent-ink on accent {theme}: {:.2}", contrast(&a, &on));
        }
    }

    /// The body of the rule whose selector line is `selector`.
    fn rule<'a>(css: &'a str, selector: &str) -> &'a str {
        let at = css.find(&format!("{selector} {{")).unwrap_or_else(|| panic!("{selector}"));
        &css[at..at + css[at..].find('}').unwrap()]
    }

    #[test]
    fn a_disabled_radio_option_does_not_look_pressable_and_keeps_its_checked_fill() {
        let editor = include_str!("../../web/editor.css");
        let rule_of = rule(editor, r#".opt[role="radio"][aria-disabled="true"]"#);
        assert!(rule_of.contains("cursor: default;"), "{rule_of}");
        // The checked fill is declared after the hover fill, so a checked option keeps it under the pointer.
        let (hover, checked) = (editor.find(".opt:hover {").unwrap(), editor.find(r#".opt[aria-checked="true"] {"#).unwrap());
        assert!(hover < checked);
    }

    #[test]
    fn the_drawing_group_shows_its_focus_in_the_accent_ring_not_its_faint_line() {
        let editor = include_str!("../../web/editor.css");
        let ring = rule(editor, ".wrap:focus-visible");
        assert!(ring.contains("outline: 2px solid var(--accent);"), "{ring}");
        // The accent is 3:1 against the plate and against the lightest and darkest grounds of the drawing.
        let app = include_str!("../../web/app.css");
        let accent = token(app, "accent");
        for (a, ground) in [
            (&accent.0, token(app, "panel").0),
            (&accent.1, token(app, "panel").1),
            (&accent.0, Some("#e9f3fb".to_string())),
            (&accent.1, Some("#17222e".to_string())),
        ] {
            assert!(contrast(a.as_ref().unwrap(), ground.as_ref().unwrap()) >= 3.0);
        }
    }

    #[test]
    fn the_zone_strip_text_selection_and_boundaries_are_measured_in_both_themes() {
        let (app, editor) = (include_str!("../../web/app.css"), include_str!("../../web/editor.css"));
        let pick = |t: &(Option<String>, Option<String>), i: usize| if i == 0 { t.0.clone() } else { t.1.clone() }.unwrap();
        for (theme, i) in [("light", 0), ("dark", 1)] {
            let (ink, sel, edge) = (pick(&token(editor, "zone-ink"), i), pick(&token(editor, "zone-sel"), i), pick(&token(editor, "zone-edge"), i));
            let (fill, own, panel) = (pick(&token(editor, "zone-fill"), i), pick(&token(editor, "zone-fill-own"), i), pick(&token(app, "panel"), i));
            for (name, ground) in [("zone-fill", &fill), ("zone-fill-own", &own)] {
                assert!(contrast(&ink, ground) >= 4.5, "zone-ink on {name} {theme}: {:.2}", contrast(&ink, ground));
            }
            for (what, c) in [("zone-sel", &sel), ("zone-edge", &edge)] {
                for (name, ground) in [("zone-fill", &fill), ("zone-fill-own", &own), ("panel", &panel)] {
                    assert!(contrast(c, ground) >= 3.0, "{what} against {name} {theme}: {:.2}", contrast(c, ground));
                }
            }
        }
        // Forced colours: the system's own colours, the chosen cell in Highlight and the edges in CanvasText.
        let at = editor.find("@media (forced-colors: active) {\n  .zone-cell").expect("forced-colours rules for the zone strip");
        let block = &editor[at..at + editor[at..].find("\n}\n").unwrap()];
        assert!(block.contains("outline-color: Highlight;") && block.contains("CanvasText"), "{block}");
        assert!(editor.contains("outline: 3px solid var(--zone-sel);"));
    }

    #[test]
    fn the_approach_mark_clears_three_to_one_on_the_road_and_the_ground_it_crosses_in_both_themes() {
        let (app, editor) = (include_str!("../../web/app.css"), include_str!("../../web/editor.css"));
        let pick = |t: &(Option<String>, Option<String>), i: usize| if i == 0 { t.0.clone() } else { t.1.clone() }.unwrap();
        let rule = editor.find("#drawing .approach-mark {").expect("the mark has a rule");
        let block = &editor[rule..rule + editor[rule..].find('}').unwrap()];
        assert!(block.contains("stroke: var(--ink-2);") && block.contains("pointer-events: none;"), "{block}");
        for (theme, i) in [("light", 0), ("dark", 1)] {
            let (ink, road) = (pick(&token(app, "ink-2"), i), pick(&token(app, "road"), i));
            assert!(contrast(&ink, &road) >= 3.0, "ink-2 on the road {theme}: {:.2}", contrast(&ink, &road));
            // the marks run across the whole property width, so over the sidewalk and planting fills as well
            for ground in ["k-sidewalk", "k-planting"] {
                let fill = pick(&token(app, ground), i);
                assert!(contrast(&ink, &fill) >= 3.0, "ink-2 on {ground} {theme}: {:.2}", contrast(&ink, &fill));
            }
        }
        let forced = editor.find("  #drawing .approach-mark {\n    stroke: CanvasText;").expect("forced-colours rule");
        assert!(editor[..forced].rfind("@media (forced-colors: active)").is_some());
    }

    #[test]
    fn the_signal_phase_drawings_are_3_to_1_against_their_road_in_both_themes_and_have_forced_colours() {
        let editor = include_str!("../../web/editor.css");
        let pick = |t: &(Option<String>, Option<String>), i: usize| if i == 0 { t.0.clone() } else { t.1.clone() }.unwrap();
        for (theme, i) in [("light", 0), ("dark", 1)] {
            let road = pick(&token(editor, "ph-road"), i);
            for name in ["ph-lit", "ph-muted"] {
                let ink = pick(&token(editor, name), i);
                assert!(contrast(&ink, &road) >= 3.0, "{name} on the road {theme}: {:.2}", contrast(&ink, &road));
            }
        }
        let forced = editor.find("  .phase-svg .ph-move .ph-line {\n    stroke: Highlight;").expect("forced-colours rule");
        assert!(editor[..forced].rfind("@media (forced-colors: active)").is_some());
        assert!(editor[..forced].contains(".phase-card[aria-current=\"true\"] {\n    outline: 2px solid Highlight;"));
        // The selected card is told apart by more than its colour: a thicker border and a check mark, both in the
        // accent, which is 3:1 against the panel and the wash (measured above).
        let card = rule(editor, ".phase-card[aria-current=\"true\"]");
        assert!(card.contains("border-color: var(--accent);") && card.contains("inset 0 0 0 2px var(--accent)"), "{card}");
        let tick = rule(editor, ".phase-card[aria-current=\"true\"]::after");
        assert!(tick.contains("content: \"\\2713\" / \"\";") && tick.contains("color: var(--accent);"), "{tick}");
        // The mark stands in room every card keeps at its top (so choosing a card moves nothing), clear of the drawing
        // and of its lit strokes: it sits 4px down and a line of the card's font is under 18px, so 22px is enough.
        assert!(tick.contains("inset-block-start: 4px;"), "{tick}");
        let base = rule(editor, ".phase-card");
        assert!(base.contains("padding-block-start: 22px;"), "{base}");
    }

    #[test]
    fn a_movement_that_meets_something_is_3_to_1_on_its_road_and_its_warning_text_4_5_to_1_with_forced_colours() {
        let (app, editor) = (include_str!("../../web/app.css"), include_str!("../../web/editor.css"));
        let pick = |t: &(Option<String>, Option<String>), i: usize| if i == 0 { t.0.clone() } else { t.1.clone() }.unwrap();
        for (theme, i) in [("light", 0), ("dark", 1)] {
            let (warn, road, panel) = (pick(&token(app, "warn"), i), pick(&token(editor, "ph-road"), i), pick(&token(app, "panel"), i));
            assert!(contrast(&warn, &road) >= 3.0, "warn on the phase road {theme}: {:.2}", contrast(&warn, &road));
            assert!(contrast(&warn, &panel) >= 4.5, "warning text on the panel {theme}: {:.2}", contrast(&warn, &panel));
        }
        let wash = token(app, "warn-wash").0.expect("a light wash");
        let warn = token(app, "warn").0.unwrap();
        assert!(contrast(&warn, &wash) >= 4.5, "warning text on its wash: {:.2}", contrast(&warn, &wash));
        let line = rule(editor, ".phase-svg .ph-move.warn .ph-line");
        assert!(line.contains("stroke: var(--warn);"), "{line}");
        let list = rule(editor, ".phase-conflicts");
        assert!(list.contains("color: var(--warn);") && list.contains("background: var(--warn-wash);"), "{list}");
        let forced = editor.find("  .phase-svg .ph-move.warn .ph-line {\n    stroke: Mark;").expect("forced-colours rule");
        assert!(editor[..forced].rfind("@media (forced-colors: active)").is_some());
    }

    #[test]
    fn the_stop_marks_clear_three_to_one_on_the_road_in_both_themes() {
        let (app, editor) = (include_str!("../../web/app.css"), include_str!("../../web/editor.css"));
        let pick = |t: &(Option<String>, Option<String>), i: usize| if i == 0 { t.0.clone() } else { t.1.clone() }.unwrap();
        for class in ["stop-pad", "stop-dwell", "stop-run", "stop-post", "stop-shelter"] {
            // the rule that names the class (alone or in a list) up to its closing brace
            let rule = editor.find(&format!("\n#drawing .{class}")).unwrap_or_else(|| panic!("{class} has a rule")) + 1;
            let block = &editor[rule..rule + editor[rule..].find('}').unwrap()];
            assert!(block.contains("pointer-events: none;") && block.contains("var(--accent"), "{class}: {block}");
            let forced = editor.find(&format!("\n  #drawing .{class}")).unwrap_or_else(|| panic!("{class} has a forced-colours rule"));
            assert!(editor[..forced].rfind("@media (forced-colors: active)").is_some_and(|m| !editor[m..forced].contains("\n}\n")), "{class}");
        }
        for (theme, i) in [("light", 0), ("dark", 1)] {
            let (accent, road) = (pick(&token(app, "accent"), i), pick(&token(app, "road"), i));
            assert!(contrast(&accent, &road) >= 3.0, "accent on the road {theme}: {:.2}", contrast(&accent, &road));
            // the wash under the outline (the dark one is translucent over the road, which is measured above)
            let wash = token(app, "accent-wash");
            if let Some(wash) = if i == 0 { wash.0 } else { wash.1 } {
                assert!(contrast(&accent, &wash) >= 3.0, "accent on its wash {theme}: {:.2}", contrast(&accent, &wash));
            }
            for ground in ["k-sidewalk", "k-planting"] {
                let fill = pick(&token(app, ground), i);
                assert!(contrast(&accent, &fill) >= 3.0, "accent on {ground} {theme}: {:.2}", contrast(&accent, &fill));
            }
        }
    }
}
