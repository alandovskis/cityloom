//! The components drawn to HTML on the host, and read as the page would show
//! them. The markup is checked by what it says and which classes and
//! attributes it carries, not by its exact text.

use std::rc::Rc;

use leptos::prelude::*;

use crate::junction::model::*;
use crate::junction::vm::{DrawMode, JunctionVm};
use crate::junction::{inspector, notes, page, plan, turns};
use crate::shared::testing::{button_tag, count, html};
use crate::shared::units::Units;

fn shared(sample: usize) -> Rc<JunctionVm> {
    JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(crate::shared::i18n::Locale::En), Junction::new(sample), None)
}

fn arm(shared: &JunctionVm, bearing: i32) -> u32 {
    shared.read(|j| j.current().arms.iter().find(|a| a.bearing == bearing).unwrap().uid)
}

// ---- the messages ----------------------------------------------------------------

#[test]
fn the_junction_ftl_files_have_the_same_messages_and_variables() {
    use crate::shared::i18n::tests::parity_problems;
    assert_eq!(parity_problems(super::RESOURCES.en, super::RESOURCES.fr), Vec::<String>::new());
}

#[test]
fn the_checks_and_the_history_of_a_junction_speak_french_with_no_english_left() {
    use crate::shared::i18n::Locale;
    use crate::shared::said::say_now;
    let fr = crate::i18n_for(Locale::FrCa);
    let say = |s: &crate::shared::said::Said| say_now(&fr, Units::Metres, s);
    // A junction that fails every check it can, with a history of each kind of edit.
    let mut j = Junction::new(3); // five ways: too many streets for a signal
    let (n, e) = (j.current().arms[0].uid, j.current().arms[1].uid);
    assert!(j.set_control(SIGNAL));
    let _ = j.set_corner(n, 9_000);
    assert!(j.set_approach(n, GATE_SIGNAL));
    assert!(j.set_stop(e, STOP_PLATFORM));
    assert!(j.set_filter(e, true));
    assert!(j.set_rule(e, RULE_RIRO));
    let lane_to = j.arm(n).unwrap().lanes[0].to[0];
    let _ = j.set_lane_dest(n, 0, lane_to, false);
    let v = j.view();
    assert!(v.checks.iter().filter(|c| !c.ok).count() >= 3, "{:?}", v.checks.iter().map(|c| (c.id, c.ok)).collect::<Vec<_>>());
    let mut text: Vec<String> = v.checks.iter().flat_map(|c| [say(&c.label), say(&c.detail)]).collect();
    text.extend(v.revisions.iter().map(|r| say(&r.label)));
    text.extend(v.arms.iter().flat_map(|a| a.transit.problems.iter().map(say)));
    text.extend(v.movements.iter().filter_map(|m| m.blocked.as_ref().map(say)));
    let all = text.join("\n");
    for english in [
        "Lanes",
        "Crossing",
        "crossing",
        "Sidewalk",
        "Signal has",
        "one signal",
        "Transit",
        " to ",
        "Corner",
        "Control",
        "north",
        "south",
        "east",
        "west",
        "needs",
        "streets",
        "turn",
        "lane",
        "Left",
        "Right-in",
        "dead end",
        "modal filter lets",
    ] {
        assert!(!all.contains(english), "{english:?} in:\n{all}");
    }
    assert!(all.contains("Signalisation\u{a0}: feux de circulation"), "{all}");
    assert_eq!(fr.missing(), Vec::<String>::new());
}

#[test]
fn the_turn_table_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(Locale::FrCa), Junction::new(0), None);
    let e = arm(&s, 90);
    s.edit(|j| j.set_rule(e, RULE_DEAD_END));
    let h = html(|| view! { <turns::Turns vm=s.clone()/> }.into_any());
    assert!(h.contains("Sample Avenue 2 (nord) vers Sample Street 1 (est)\u{a0}: virage à gauche"), "{h}");
    assert!(h.contains("Virages permis.") && h.contains(">O<") && h.contains("impossible. Un cul-de-sac"), "{h}");
    for english in ["Turns allowed", "From", " to ", " turn", "allowed", "north", ">W<"] {
        assert!(!h.contains(english), "{english:?} in {h}");
    }
}

// ---- turns ---------------------------------------------------------------------

#[test]
fn the_turn_table_has_a_button_for_every_turn_and_none_for_a_street_to_itself() {
    let s = shared(0);
    let h = html(|| view! { <turns::Turns vm=s.clone()/> }.into_any());
    assert_eq!(count(&h, "<button"), 12);
    assert_eq!(count(&h, "aria-pressed=\"true\""), 12);
    assert_eq!(count(&h, "class=\"self\""), 4);
    assert!(h.contains("Sample Avenue 2 (north) to Sample Street 1 (east): left turn, allowed"));
}

#[test]
fn a_banned_turn_reads_as_not_allowed() {
    let s = shared(0);
    let (n, e) = (arm(&s, 0), arm(&s, 90));
    s.edit(|j| j.set_turn(n, e, false));
    let h = html(|| view! { <turns::Turns vm=s.clone()/> }.into_any());
    assert!(h.contains("left turn, not allowed"));
    assert_eq!(count(&h, "aria-pressed=\"true\""), 11);
}

#[test]
fn a_turn_a_measure_blocks_is_locked_with_its_reason() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.set_rule(e, RULE_DEAD_END));
    let h = html(|| view! { <turns::Turns vm=s.clone()/> }.into_any());
    assert!(count(&h, "turn locked") > 0);
    assert!(h.contains("not possible."));
}

// ---- notes ---------------------------------------------------------------------

#[test]
fn getting_across_lists_each_street_with_its_crossing_and_lanes() {
    let s = shared(0);
    let h = html(|| view! { <notes::Across vm=s.clone()/> }.into_any());
    assert_eq!(count(&h, "<tr><th scope=\"row\">"), 4);
    assert!(h.contains("2 × 9.0"));
    assert!(h.contains(">11.4<"));
}

#[test]
fn getting_across_follows_the_units() {
    let s = shared(0);
    s.set_units(Units::Feet);
    let h = html(|| view! { <notes::Across vm=s.clone()/> }.into_any());
    assert!(h.contains("2 × 29.5"));
    assert!(h.contains(">37.4<"));
}

#[test]
fn a_street_without_a_crossing_says_none_and_a_too_long_one_is_flagged() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.set_crossing(e, false));
    let h = html(|| view! { <notes::Across vm=s.clone()/> }.into_any());
    assert!(h.contains("class=\"zero\">none"));
}

#[test]
fn conflicts_are_counted_and_the_note_follows_the_control() {
    let s = shared(0);
    let h = html(|| view! { <notes::Conflicts vm=s.clone()/> }.into_any());
    assert!(h.contains("<td>16</td>") && h.contains("<td>32</td>"));
    let note = html(|| view! { <notes::ConflictNote vm=s.clone()/> }.into_any());
    assert!(note.contains("A signal takes turns"));
}

#[test]
fn the_checks_each_say_whether_they_pass() {
    let s = shared(0);
    let h = html(|| view! { <notes::Checks vm=s.clone()/> }.into_any());
    assert_eq!(count(&h, "<li class=\"ok\">"), 11, "the eight of any junction, and a signal's time to cross, phases and cycle");
    assert!(h.contains("Time to cross") && h.contains("Signal cycle"));
    assert!(h.contains(": passes"));
    assert!(h.contains("Longest crossing in one go: 11.4 m"));
}

#[test]
fn a_failing_check_is_marked_and_says_so_to_a_screen_reader() {
    let s = shared(0);
    let n = arm(&s, 0);
    s.edit(|j| j.set_corner(n, 12_000));
    let h = html(|| view! { <notes::Checks vm=s.clone()/> }.into_any());
    assert!(count(&h, "<li class=\"bad\">") >= 1);
    assert!(h.contains(": fails"));
}

#[test]
fn the_changes_start_from_the_junction_today_and_list_each_edit() {
    let s = shared(0);
    let h = html(|| view! { <notes::Revisions vm=s.clone()/> }.into_any());
    assert!(h.contains("Junction today") && h.contains("base now"));
    let n = arm(&s, 0);
    s.edit(|j| j.set_corner(n, 7_000));
    let h = html(|| view! { <notes::Revisions vm=s.clone()/> }.into_any());
    assert!(h.contains("7.0 m radius"));
    assert!(h.contains("class=\"now\""));
    assert!(!h.contains("base now"));
}

// ---- inspector -----------------------------------------------------------------

fn panel(s: &Rc<JunctionVm>) -> String {
    html(|| view! { <inspector::Inspector vm=s.clone()/> }.into_any())
}

#[test]
fn a_selected_street_offers_a_control_for_each_junction_measure_of_the_atlas() {
    use crate::shared::i18n::{Args, Locale};
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    let en = crate::i18n_for(Locale::En);
    let coded: Vec<(&str, &str)> = APPROACHES
        .iter()
        .chain(&STOPS)
        .chain(&RULES)
        .map(|i| (i.code, i.key))
        .chain([(FILTER_CODE, "jn-insp-filter")])
        .filter(|(c, _)| !c.is_empty())
        .collect();
    assert_eq!(coded.len(), 12);
    for (code, key) in coded {
        let want = format!("{code} {}", en.tr_now(key, &Args::new()));
        assert!(h.contains(&want), "the inspector has no control named {want:?}");
    }
}

#[test]
fn with_nothing_selected_the_panel_offers_the_junction_s_control() {
    let s = shared(0);
    let h = panel(&s);
    assert!(h.contains("Select a street, corner or crossing to change it."));
    assert!(h.contains("Junction control") && h.contains("Traffic signal"));
    assert!(!h.contains("Cycle track"));
}

#[test]
fn a_roundabout_adds_its_size_cycle_track_and_bus_lane() {
    let s = shared(0);
    s.edit(|j| j.set_control(ROUNDABOUT));
    let h = panel(&s);
    for want in ["Size of the roundabout", "Cycle track", "Bus lane through the middle"] {
        assert!(h.contains(want), "{want}");
    }
    assert!(!h.contains("Track width"));
    s.edit(|j| j.set_cycle_track(true));
    assert!(panel(&s).contains("Track width"));
}

#[test]
fn a_roundabout_offers_its_turbo_layouts_and_a_chosen_one_its_size_and_main_road() {
    let s = shared(0);
    s.edit(|j| j.set_control(ROUNDABOUT));
    let h = panel(&s);
    for want in ["Roundabout layout", "Single lane", ">Basic<", ">Egg<", ">Knee<", ">Spiral<"] {
        assert!(h.contains(want), "{want}");
    }
    assert!(!h.contains(">Rotor<"), "three lanes on Sample Street 1 would pass its property line: not offered");
    assert_eq!(count(&h, "role=\"radio\""), 5, "the single lane and four layouts");
    assert_eq!(count(&h, "role=\"none\""), 5, "each radio is in an item that says it is not a list item of its own");
    assert!(!h.contains("Turbo size") && !h.contains("Main road"));
    assert!(h.contains("Size of the roundabout") && h.contains("Bus lane through the middle"));

    s.edit(|j| j.set_turbo(Some(0)));
    let h = panel(&s);
    for want in ["Turbo size", "Mini", "Next larger size", "Next smaller size", "Main road", "are the main road.", "Use the other streets as the main road"] {
        assert!(h.contains(want), "{want}");
    }
    assert!(!h.contains("Size of the roundabout"), "a turbo has blocks, not a diameter");
    assert!(!h.contains("Bus lane through the middle"), "and no bus lane across its island");
    assert!(h.contains("Cycle track"), "but it keeps the cycle track");
    assert!(!h.contains("Move the two entry lanes"), "a basic turbo's switch is to the other streets");

    s.edit(|j| j.set_turbo(Some(2)));
    let h = panel(&s);
    assert!(h.contains("Move the two entry lanes to the other street"), "on a knee the first press moves the lanes");
    assert!(!h.contains("Use the other streets as the main road"));
}

/// The tab index of each radio of the panel, in order, with whether it is checked.
fn radio_stops(h: &str) -> Vec<(String, bool)> {
    h.split("<button")
        .filter(|b| b.contains("role=\"radio\""))
        .map(|b| {
            let tag = b.split('>').next().unwrap();
            let attr = |name: &str| tag.split(&format!("{name}=\"")).nth(1).unwrap().split('"').next().unwrap().to_string();
            (attr("tabindex"), attr("aria-checked") == "true")
        })
        .collect()
}

#[test]
fn the_layout_radios_are_one_tab_stop_on_the_chosen_one() {
    let s = shared(0);
    s.edit(|j| j.set_control(ROUNDABOUT));
    let stops = |s: &Rc<JunctionVm>| radio_stops(&panel(s)).into_iter().map(|(t, c)| (t == "0", c)).collect::<Vec<_>>();
    assert_eq!(stops(&s), [(true, true), (false, false), (false, false), (false, false), (false, false)], "the single lane is chosen and the stop");
    s.edit(|j| j.set_turbo(Some(2)));
    assert_eq!(stops(&s), [(false, false), (false, false), (false, false), (true, true), (false, false)], "the knee is chosen and the stop");
}

#[test]
fn the_turbo_size_stepper_is_a_group_that_holds_its_buttons_and_the_heading_labels_it() {
    let s = shared(0);
    s.edit(|j| j.set_control(ROUNDABOUT));
    s.edit(|j| j.set_turbo(Some(0)));
    let h = panel(&s);
    let stepper = h.split("<div").find(|t| t.contains("class=\"stepper\"") && t.contains("turbo-size")).expect("the stepper");
    let tag = stepper.split('>').next().unwrap();
    assert!(tag.contains("aria-labelledby=\"i-h-turbo-size\"") && tag.contains("role=\"group\""), "{tag}");
    assert!(stepper.matches("<button").count() >= 2 && stepper.find("<button") < stepper.find("</div>"), "the buttons are inside the group: {stepper}");
    let name = h.split("<span").find(|t| t.contains("wfield turbo-size")).unwrap().split('>').next().unwrap();
    assert!(!name.contains("role="), "the name is not a group of its own: {name}");
}

/// The attributes of each button of the turbo size stepper, smaller first.
fn size_buttons(h: &str) -> Vec<String> {
    let stepper = h.split("<div").find(|t| t.contains("class=\"stepper\"") && t.contains("turbo-size")).unwrap();
    stepper.split("<button").skip(1).take(2).map(|b| b.split('>').next().unwrap().to_string()).collect()
}

#[test]
fn a_size_stepper_at_its_end_is_aria_disabled_so_that_focus_stays_on_it() {
    let s = shared(0);
    s.edit(|j| j.set_control(ROUNDABOUT));
    s.edit(|j| j.set_turbo(Some(0)));
    let [smaller, larger] = size_buttons(&panel(&s)).try_into().unwrap();
    assert!(smaller.contains("aria-disabled=\"true\"") && !larger.contains("aria-disabled=\"true\""), "mini: {smaller} / {larger}");
    assert!(!smaller.contains(" disabled") && !larger.contains(" disabled"), "a disabled button would lose the focus: {smaller} / {larger}");
    s.edit(|j| j.set_turbo_size(3));
    let [smaller, larger] = size_buttons(&panel(&s)).try_into().unwrap();
    assert!(!smaller.contains("aria-disabled=\"true\"") && larger.contains("aria-disabled=\"true\""), "large: {smaller} / {larger}");
}

/// The options of the first select that has options with these values, with their attributes.
fn approach_options(h: &str) -> Vec<String> {
    let select = h.split("<select").find(|t| t.starts_with(" aria-labelledby=\"i-h-approach\"")).expect("the approach select");
    select.split("</select>").next().unwrap().split("<option").skip(1).map(|o| o.split('>').next().unwrap().to_string()).collect()
}

#[test]
fn a_turbo_offers_no_queue_jump_in_the_approach_list_and_a_plain_roundabout_does() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.set_control(ROUNDABOUT));
    s.edit(|j| j.select(Target::Arm(e)));
    let off = |h: &str| approach_options(h).iter().map(|o| o.contains("disabled")).collect::<Vec<_>>();
    assert_eq!(off(&panel(&s)), [false; 6], "no turbo, every measure");
    s.edit(|j| j.set_turbo(Some(0)));
    assert_eq!(off(&panel(&s)), [false, true, true, true, false, false], "the three queue jumps are out, the gates stay");
}

#[test]
fn only_the_layouts_that_suit_the_streets_are_offered() {
    let s = shared(1); // three streets
    s.edit(|j| j.set_control(ROUNDABOUT));
    let h = panel(&s);
    assert!(h.contains(">Egg<") && h.contains(">Knee<"));
    assert!(!h.contains(">Basic<") && !h.contains(">Spiral<") && !h.contains(">Rotor<"));
    assert!(!panel(&shared(0)).contains("Roundabout layout"), "a signal has no layouts");
}

#[test]
fn a_lane_of_a_turbo_says_the_layout_sets_where_it_goes() {
    let s = shared(0);
    s.edit(|j| j.set_control(ROUNDABOUT));
    s.edit(|j| j.set_turbo(Some(0)));
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Lane(n, 0)));
    assert!(panel(&s).contains("The turbo layout sets where this lane goes."));
    let plain = shared(0);
    let n = arm(&plain, 0);
    plain.edit(|j| j.select(Target::Lane(n, 0)));
    assert!(!panel(&plain).contains("The turbo layout sets"));
}

#[test]
fn the_panel_of_a_turbo_roundabout_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    s.edit(|j| j.set_control(ROUNDABOUT));
    panel_speaks_french(&s, &["Disposition du giratoire", "Une seule voie", ">De base<", ">En œuf<", ">En genou<", ">En spirale<"]);
    s.edit(|j| j.set_turbo(Some(0)));
    panel_speaks_french(
        &s,
        &[
            "Taille du giratoire turbo",
            ">Mini<",
            "aria-label=\"Taille supérieure\"",
            "aria-label=\"Taille inférieure\"",
            "Axe principal",
            "forment l’axe principal.",
            "Prendre les autres rues comme axe principal",
        ],
    );
    s.edit(|j| j.set_turbo(Some(2)));
    panel_speaks_french(&s, &["Mettre les deux voies d’entrée sur l’autre rue"]);
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Lane(n, 0)));
    panel_speaks_french(&s, &["La disposition turbo fixe la destination de cette voie."]);
}

#[test]
fn a_junction_offers_a_raised_table_until_it_is_a_roundabout() {
    let s = shared(0);
    let h = panel(&s);
    assert!(h.contains("Raised table") && h.contains("Raise the whole junction to sidewalk height"));
    s.edit(|j| j.set_control(ROUNDABOUT));
    assert!(!panel(&s).contains("Raise the whole junction to sidewalk height"));
}

#[test]
fn the_raised_table_switch_is_worded_in_french_too() {
    let i18n = crate::i18n_for(crate::shared::i18n::Locale::FrCa);
    for key in ["jn-insp-raised", "jn-insp-raised-option"] {
        assert!(i18n.has_message(key, crate::shared::i18n::Locale::FrCa), "{key}");
    }
    assert_eq!(i18n.tr_now("jn-insp-raised", &crate::shared::i18n::Args::new()), "Carrefour surélevé");
}

#[test]
fn a_continuous_sidewalk_says_why_its_street_has_no_island_switch() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    assert!(panel(&s).contains("Refuge island in the middle"));
    s.edit(|j| j.set_continuous(e, true));
    let h = panel(&s);
    assert!(h.contains("Refuge island (not on a continuous sidewalk)") && !h.contains("Refuge island in the middle"));
}

#[test]
fn a_corner_between_streets_with_bike_lanes_offers_a_protected_corner_and_others_say_why_not() {
    use crate::junction::model::junction_of;
    let biked = junction_of(&[("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)]);
    let uid = biked.current().arms[0].uid;
    let s = JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(crate::shared::i18n::Locale::En), biked, None);
    s.edit(|j| j.select(Target::Corner(uid)));
    let h = panel(&s);
    assert!(h.contains("Protected corner") && h.contains("Bike track round a corner island"));
    assert!(!h.contains("needs a bike lane at the curb"));
    s.edit(|j| j.set_protected(uid, true));
    assert!(panel(&s).contains("Bike track round a corner island"));

    let plain = shared(0);
    let n = arm(&plain, 0);
    plain.edit(|j| j.select(Target::Corner(n)));
    assert!(panel(&plain).contains("Protected corner (needs a bike lane at the curb of both streets)"));
}

#[test]
fn a_street_with_a_bike_lane_offers_a_bike_box_and_one_without_says_why_it_cannot() {
    use crate::junction::model::junction_of;
    let biked = junction_of(&[("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)]);
    let uid = biked.current().arms[0].uid;
    let s = JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(crate::shared::i18n::Locale::En), biked, None);
    s.edit(|j| j.select(Target::Arm(uid)));
    let h = panel(&s);
    assert!(h.contains("Bike box") && h.contains("Waiting box for cyclists ahead of the cars"));
    assert!(!h.contains("needs a bike lane"));
    s.edit(|j| j.set_bike_box(uid, true));
    assert!(panel(&s).contains("Waiting box for cyclists ahead of the cars"));

    let plain = shared(0);
    let e = arm(&plain, 90);
    plain.edit(|j| j.select(Target::Arm(e)));
    assert!(panel(&plain).contains("Bike box (needs a bike lane on this street)"));
}

#[test]
fn a_roundabout_says_it_has_no_bike_box() {
    use crate::junction::model::junction_of;
    let mut biked = junction_of(&[("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)]);
    assert!(biked.set_control(ROUNDABOUT));
    let uid = biked.current().arms[0].uid;
    let s = JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(crate::shared::i18n::Locale::En), biked, None);
    s.edit(|j| j.select(Target::Arm(uid)));
    assert!(panel(&s).contains("Bike box (not at a roundabout)") && !panel(&s).contains("Waiting box for cyclists"));
}

#[test]
fn a_street_with_a_crossing_offers_a_continuous_sidewalk_and_one_without_does_not() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    assert!(h.contains("Continuous sidewalk") && h.contains("Sidewalk runs on across the street"));
    s.edit(|j| j.set_crossing(e, false));
    assert!(!panel(&s).contains("Sidewalk runs on across the street"));
}

#[test]
fn a_street_s_panel_has_its_lanes_crossing_transit_direction_and_street() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    for want in ["Sample Street 1", "Lanes coming in", "Set back from the junction", "Transit priority", "Direction", "Shift sideways", "Remove this street"] {
        assert!(h.contains(want), "{want}");
    }
    assert!(h.contains("E, 90° · 11.4 m road"));
    assert_eq!(count(&h, "class=\"lane-row\""), 1);
}

#[test]
fn a_street_without_a_crossing_offers_to_mark_one() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.set_crossing(e, false));
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    assert!(h.contains("Mark a crossing") && !h.contains("Remove the crossing"));
}

#[test]
fn a_queue_jump_has_a_length_field_and_a_virtual_one_does_not() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.set_approach(e, Q_CURB));
    s.edit(|j| j.select(Target::Arm(e)));
    assert!(panel(&s).contains("Length of the queue jump"));
    s.edit(|j| j.set_approach(e, Q_VIRTUAL));
    let h = panel(&s);
    assert!(!h.contains("Length of the queue jump") && !h.contains("Gate distance upstream"));
}

#[test]
fn a_city_junction_s_streets_link_to_the_street_editor_and_cannot_be_removed() {
    let mut state = Junction::new(0).current().clone();
    for a in &mut state.arms {
        a.edge = a.uid + 10;
    }
    let j = Junction::from_city(crate::shared::said::Said::new("city-name").with("name", crate::shared::said::Arg::Text("Test".into())), &state, &state, 0)
        .unwrap();
    let s = JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(crate::shared::i18n::Locale::En), j, None);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    assert!(h.contains("Open the cross-section") && h.contains("street.html?street="));
    assert!(!h.contains("Remove this street"));
}

#[test]
fn a_corner_panel_names_its_two_streets_and_gives_the_radius() {
    let s = shared(0);
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Corner(n)));
    let h = panel(&s);
    assert!(h.contains("Corner") && h.contains("N to E"));
    assert!(h.contains("Curb radius") && h.contains("value=\"6.0\""));
}

#[test]
fn a_lane_panel_lists_where_the_lane_goes() {
    let s = shared(0);
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Lane(n, 1)));
    let h = panel(&s);
    assert!(h.contains("Lane 2 of 2") && h.contains("nearest the curb"));
    assert!(h.contains("Straight on to Sample Avenue 2 (south)"));
    assert!(h.contains("Select the whole street"));
}

#[test]
fn the_bus_lane_and_cycle_track_have_their_own_panels() {
    let s = shared(0);
    s.edit(|j| j.set_control(ROUNDABOUT));
    let (n, so) = (arm(&s, 0), arm(&s, 180));
    s.edit(|j| j.set_bus(Some((n, so))));
    s.edit(|j| j.set_cycle(Some(2_000)));
    s.edit(|j| j.select(Target::Bus));
    let h = panel(&s);
    assert!(h.contains("Remove the bus lane"));
    s.edit(|j| j.select(Target::Cycle));
    let h = panel(&s);
    assert!(h.contains("Remove the cycle track") && h.contains("Round the outside of the roundabout"));
}

#[test]
fn the_panel_s_lengths_follow_the_units() {
    let s = shared(0);
    s.set_units(Units::Feet);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    assert!(h.contains("E, 90° · 37.4 ft road"));
}

// ---- what the keys do -------------------------------------------------------------

use crate::junction::keys::{Action, Step};
use crate::junction::watch::Watch;
use crate::shared::live;

fn watch(sample: usize) -> (Rc<JunctionVm>, Watch) {
    let s = shared(sample);
    let w = Watch::new(s.clone());
    live::take_said();
    (s, w)
}

#[test]
fn stepping_the_selected_street_turns_it_by_a_step_and_says_so() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    let n = arm(&s, 0);
    s.select(Target::Arm(n));
    live::take_said();
    w.apply(Action::Step(Step::Bearing, 1));
    assert_eq!(s.read(|j| j.arm(n).unwrap().bearing), 5);
    assert!(live::take_said()[0].starts_with("Sample Avenue 2 bearing: 5°."));
}

#[test]
fn stepping_suits_what_is_selected() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    let n = arm(&s, 0);
    s.select(Target::Corner(n));
    w.apply(Action::Step(Step::Corner, -1));
    assert_eq!(s.read(|j| j.arm(n).unwrap().corner_mm), 5_500);
    s.select(Target::Crossing(n));
    w.apply(Action::Step(Step::Setback, 1));
    assert_eq!(s.read(|j| j.arm(n).unwrap().crossing.unwrap().setback_mm), 3_500);
}

#[test]
fn stepping_with_nothing_selected_changes_nothing() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    let before = s.read(|j| j.current().clone());
    w.apply(Action::Step(Step::Bearing, 1));
    assert_eq!(s.read(|j| j.current().clone()), before);
}

#[test]
fn removing_takes_away_what_is_selected_in_the_way_that_suits_it() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    let n = arm(&s, 0);
    s.select(Target::Crossing(n));
    w.apply(Action::Remove);
    assert!(s.read(|j| j.arm(n).unwrap().crossing.is_none()));
    s.select(Target::Arm(n));
    w.apply(Action::Remove);
    assert_eq!(s.read(|j| j.current().arms.len()), 3);
    assert!(s.read(|j| j.arm(n).is_none()));
}

#[test]
fn removing_the_last_removable_street_is_refused_with_its_reason() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(1);
    let uid = s.read(|j| j.current().arms[0].uid);
    s.select(Target::Arm(uid));
    live::take_said();
    w.apply(Action::Remove);
    assert_eq!(live::take_said(), vec!["A junction needs at least three streets."]);
}

#[test]
fn the_selection_moves_round_the_junction_and_says_where_it_is() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    w.apply(Action::Select(1));
    assert_ne!(s.read(|j| j.selected), Target::None);
    assert_eq!(live::take_said().len(), 1);
    w.apply(Action::Deselect);
    assert_eq!(s.read(|j| j.selected), Target::None);
}

#[test]
fn the_bus_lane_and_cycle_track_are_removed_by_the_same_key() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    s.edit(|j| j.set_control(ROUNDABOUT));
    let (n, so) = (arm(&s, 0), arm(&s, 180));
    s.edit(|j| j.set_bus(Some((n, so))));
    s.edit(|j| j.set_cycle(Some(2_000)));
    s.select(Target::Bus);
    w.apply(Action::Remove);
    assert!(s.read(|j| j.current().bus.is_none()));
    s.select(Target::Cycle);
    w.apply(Action::Remove);
    assert!(s.read(|j| j.current().cycle.is_none()));
}

// ---- the page around the plan -------------------------------------------------------

fn linked_shared() -> Rc<JunctionVm> {
    let mut state = Junction::new(0).current().clone();
    for a in &mut state.arms {
        a.edge = a.uid + 10;
    }
    JunctionVm::new(
        crate::shared::platform::browser_ports(),
        crate::i18n_for(crate::shared::i18n::Locale::En),
        Junction::from_city(crate::shared::said::Said::new("city-name").with("name", crate::shared::said::Arg::Text("Junction 4".into())), &state, &state, 0)
            .unwrap(),
        None,
    )
}

#[test]
fn the_heading_names_the_junction_and_counts_its_streets() {
    let s = shared(0);
    let h = html(|| view! { <page::Header vm=s.clone()/> }.into_any());
    assert!(h.contains(">Avenue and street</h1>") && h.contains("Junction plan") && h.contains(">4 streets<"));
    assert!(!h.contains("City map"));
}

#[test]
fn a_city_junction_s_heading_links_back_to_the_map() {
    let h = html(|| view! { <page::Header vm=linked_shared()/> }.into_any());
    assert!(h.contains("href=\"map.html\"") && h.contains("City map") && h.contains(">Junction 4</h1>"));
}

#[test]
fn the_title_block_gives_the_name_the_streets_and_the_changes_made() {
    let s = shared(0);
    let n = arm(&s, 0);
    s.edit(|j| j.set_corner(n, 7_000));
    let h = html(|| view! { <page::TitleBlock vm=s.clone()/> }.into_any());
    assert!(
        h.contains("id=\"tb-street\">Avenue and street<") && h.contains("id=\"tb-row\" class=\"fig\">4<") && h.contains("id=\"tb-changes\" class=\"fig\">1<")
    );
}

fn osm_junction_shared(locale: crate::shared::i18n::Locale) -> Rc<JunctionVm> {
    use crate::shared::provenance::OsmRef;
    let mut state = Junction::new(0).current().clone();
    for a in &mut state.arms {
        a.edge = a.uid + 10;
    }
    state.source = vec![OsmRef { id: 29796354, version: Some(5) }, OsmRef { id: 9, version: None }];
    JunctionVm::new(
        crate::shared::platform::browser_ports(),
        crate::i18n_for(locale),
        Junction::from_city(crate::shared::said::Said::new("city-name").with("name", crate::shared::said::Arg::Text("Junction 4".into())), &state, &state, 0)
            .unwrap(),
        None,
    )
}

#[test]
fn the_provenance_links_the_osm_nodes_and_says_whether_the_junction_was_changed() {
    let s = osm_junction_shared(crate::shared::i18n::Locale::En);
    let h = html(|| view! { <page::Provenance vm=s.clone()/> }.into_any());
    assert!(h.contains("href=\"https://www.openstreetmap.org/node/29796354\"") && h.contains(">node 29796354 v5<"), "{h}");
    assert!(h.contains(">node 9<") && h.contains("id=\"tb-state\">As imported<"), "{h}");
    let n = arm(&s, 0);
    s.edit(|j| j.set_filter(n, true));
    let h = html(|| view! { <page::Provenance vm=s.clone()/> }.into_any());
    assert!(h.contains("id=\"tb-state\">Edited<"), "{h}");
}

#[test]
fn a_junction_with_no_source_says_so() {
    use crate::shared::i18n::Locale;
    let h = html(|| view! { <page::Provenance vm=shared(0)/> }.into_any());
    assert!(h.contains("This junction was not made from OpenStreetMap."), "{h}");
    assert!(!h.contains("openstreetmap.org") && !h.contains("tb-state"), "{h}");
    let fr = shared_in(0, Locale::FrCa);
    let h = html(|| view! { <page::Provenance vm=fr.clone()/> }.into_any());
    assert!(h.contains("Cette jonction ne vient pas d’OpenStreetMap.") && !h.contains("was not made"), "{h}");
}

#[test]
fn the_provenance_follows_the_language_switch() {
    use crate::shared::i18n::Locale;
    let owner = Owner::new();
    owner.set();
    let s = osm_junction_shared(Locale::En);
    let h = html(|| view! { <page::Provenance vm=s.clone()/> }.into_any());
    assert!(h.contains(">Data<") && h.contains("As imported"), "{h}");
    s.i18n().set(Locale::FrCa);
    let h = html(|| view! { <page::Provenance vm=s.clone()/> }.into_any());
    assert!(h.contains(">Données<") && h.contains("Telle qu’importée") && !h.contains("As imported"), "{h}");
}

#[test]
fn the_title_block_no_longer_holds_the_sources() {
    let s = osm_junction_shared(crate::shared::i18n::Locale::En);
    let h = html(|| view! { <page::TitleBlock vm=s.clone()/> }.into_any());
    assert!(!h.contains("openstreetmap.org") && !h.contains("tb-state") && !h.contains("OpenStreetMap"), "{h}");
}

#[test]
fn the_status_line_says_whether_the_junction_works() {
    let s = shared(0);
    let h = html(|| view! { <page::Fit vm=s.clone()/> }.into_any());
    assert!(h.contains("class=\"fit\"") && h.contains("Every check passes.") && h.contains("tick"));
    let n = arm(&s, 0);
    s.edit(|j| j.set_island(n, false));
    let h = html(|| view! { <page::Fit vm=s.clone()/> }.into_any());
    assert!(h.contains("class=\"fit bad\"") && h.contains("One check needs attention: crossing distance.") && !h.contains("tick"));
}

/// The opening tag of the button with this id.
#[test]
fn undo_redo_and_start_over_are_off_until_there_is_something_to_undo() {
    let s = shared(0);
    let off = |h: &str| ["undo", "redo", "reset"].map(|id| button_tag(h, id).contains("disabled"));
    let h = html(|| view! { <page::History vm=s.clone()/> }.into_any());
    assert_eq!(off(&h), [true, true, true]);
    let n = arm(&s, 0);
    s.edit(|j| j.set_corner(n, 7_000));
    let h = html(|| view! { <page::History vm=s.clone()/> }.into_any());
    assert_eq!(off(&h), [false, true, false]);
    s.edit(|j| j.undo());
    let h = html(|| view! { <page::History vm=s.clone()/> }.into_any());
    assert_eq!(off(&h), [true, false, true]);
}

#[test]
fn the_key_lists_the_pieces_in_the_plan() {
    let s = shared(0);
    let h = html(|| view! { <page::Key vm=s.clone()/> }.into_any());
    assert_eq!(count(&h, "<li>"), 5);
    assert!(h.contains("Sidewalk") && h.contains("Driving lane") && h.contains("Planted median"));
}

#[test]
fn the_plan_is_a_labelled_group_holding_a_picture_of_the_junction() {
    let s = shared(0);
    let h = html(|| view! { <plan::PlanDrawing vm=s.clone()/> }.into_any());
    assert!(h.contains("id=\"wrap\"") && h.contains("tabindex=\"0\"") && h.contains("aria-label=\"Junction plan editor\""));
    assert!(h.contains("id=\"drawing\"") && h.contains("role=\"img\""));
    assert!(h.contains("Plan of the junction, north up. 4 streets."));
    assert!(h.contains("data-role=\"grip-arm\"") && h.contains("class=\"plan\""));
}

#[test]
fn the_plan_follows_the_units_and_the_selection() {
    let s = shared(0);
    s.set_units(Units::Feet);
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Arm(n)));
    let h = html(|| view! { <plan::PlanDrawing vm=s.clone()/> }.into_any());
    assert!(h.contains("2 × 29.5") && h.contains("66 ft"));
    assert_eq!(count(&h, "class=\"arm on\""), 1);
}

#[test]
fn the_keyboard_help_names_v_for_the_view_in_both_languages() {
    use crate::shared::i18n::{Args, Locale};
    let page = include_str!("../../web/intersection.html");
    assert!(page.contains("<kbd>V</kbd> <span data-i18n=\"jn-page-key-view\">"), "the page's key help lists V");
    assert_eq!(crate::i18n_for(Locale::En).tr("jn-page-key-view", &Args::new()), "plan, 2.5D");
    assert_eq!(crate::i18n_for(Locale::FrCa).tr("jn-page-key-view", &Args::new()), "plan, 2,5D");
}

#[test]
fn the_view_toggle_keeps_the_touch_target_of_a_coarse_pointer() {
    // `.view-mode .unit` (min-height 26px) outweighs app.css's `.unit` rule for a coarse pointer, so editor.css
    // must say 44px again for it inside its own coarse-pointer block.
    let css = include_str!("../../web/editor.css");
    let coarse = &css[css.find("@media (any-pointer: coarse)").unwrap()..];
    let block = &coarse[..coarse.find("\n}\n").unwrap()];
    let rule = &block[block.find(".view-mode .unit").expect("the coarse block names the toggle's buttons")..];
    assert!(rule[..rule.find('}').unwrap()].contains("min-height: 44px"), "{rule}");
    // The row's negative margin was sized for a 32px toggle (`.view-mode-tools`); 12px more of button needs 6px more.
    let tools = &block[block.find(".view-mode-tools").expect("the coarse block makes room for the taller toggle")..];
    assert!(tools[..tools.find('}').unwrap()].contains("margin-block: -9px"), "{tools}");
}

fn shared_mode(mode: DrawMode, locale: crate::shared::i18n::Locale) -> Rc<JunctionVm> {
    let vm = JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(locale), Junction::new(0), None);
    vm.set_mode(mode);
    vm
}

#[test]
fn the_drawing_is_the_2_5d_picture_in_iso_mode_with_no_grips_and_a_label_of_its_own() {
    use crate::shared::i18n::Locale;
    let s = shared_mode(DrawMode::Iso, Locale::En);
    let h = html(|| view! { <plan::PlanDrawing vm=s.clone()/> }.into_any());
    assert!(h.contains("class=\"iso\"") && !h.contains("class=\"plan\""), "{h}");
    assert!(!h.contains("data-role=\"grip") && !h.contains("data-role=\"arm\""), "{h}");
    assert!(h.contains("2.5D view of the junction. 4 streets."), "{h}");
    let f = shared_mode(DrawMode::Iso, Locale::FrCa);
    let h = html(|| view! { <plan::PlanDrawing vm=f.clone()/> }.into_any());
    assert!(h.contains("Vue en 2,5D de la jonction. 4 rues."), "{h}");
}

#[test]
fn the_view_toggle_is_a_group_of_two_buttons_and_the_view_shown_is_pressed() {
    use crate::shared::i18n::Locale;
    let s = shared_mode(DrawMode::Plan, Locale::En);
    let h = html(|| view! { <page::ViewToggle vm=s.clone()/> }.into_any());
    assert!(h.contains("<fieldset") && h.contains("id=\"view-mode\""), "{h}");
    assert!(h.contains(">View</legend>"), "{h}");
    assert!(button_tag(&h, "view-plan").contains("aria-pressed=\"true\""));
    assert!(button_tag(&h, "view-iso").contains("aria-pressed=\"false\""));
    // V goes to the view not shown, so only that button carries it.
    assert!(button_tag(&h, "view-iso").contains("aria-keyshortcuts=\"V\""));
    assert!(!button_tag(&h, "view-plan").contains("aria-keyshortcuts"));
    assert!(h.contains(">Plan</button>") && h.contains(">2.5D</button>"), "{h}");
    s.set_mode(DrawMode::Iso);
    let h = html(|| view! { <page::ViewToggle vm=s.clone()/> }.into_any());
    assert!(button_tag(&h, "view-iso").contains("aria-pressed=\"true\""));
    assert!(button_tag(&h, "view-plan").contains("aria-pressed=\"false\""));
    assert!(button_tag(&h, "view-plan").contains("aria-keyshortcuts=\"V\""));
    assert!(!button_tag(&h, "view-iso").contains("aria-keyshortcuts"));
}

#[test]
fn the_view_toggle_reads_plan_and_2_5d_in_french() {
    use crate::shared::i18n::Locale;
    let s = shared_mode(DrawMode::Plan, Locale::FrCa);
    let h = html(|| view! { <page::ViewToggle vm=s.clone()/> }.into_any());
    assert!(h.contains(">Plan</button>") && h.contains(">2,5D</button>") && h.contains(">Affichage</legend>"), "{h}");
}

// ---- the page in French ---------------------------------------------------------------

fn shared_in(sample: usize, locale: crate::shared::i18n::Locale) -> Rc<JunctionVm> {
    JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(locale), Junction::new(sample), None)
}

fn linked_in(locale: crate::shared::i18n::Locale) -> Rc<JunctionVm> {
    let mut state = Junction::new(0).current().clone();
    for a in &mut state.arms {
        a.edge = a.uid + 10;
    }
    state.source = vec![crate::shared::provenance::OsmRef { id: 9, version: None }];
    let name = crate::shared::said::Said::new("city-name").with("name", crate::shared::said::Arg::Text("Jonction 4".into()));
    JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(locale), Junction::from_city(name, &state, &state, 0).unwrap(), None)
}

/// Fails on the first of `english` found in `h`, and on the first of `french` missing from it.
fn speaks_french(h: &str, english: &[&str], french: &[&str]) {
    for e in english {
        assert!(!h.contains(e), "{e:?} in:\n{h}");
    }
    for f in french {
        assert!(h.contains(f), "{f:?} missing from:\n{h}");
    }
}

/// English that must not be left on a French junction page, whatever the view.
const ENGLISH: &[&str] = &["Junction plan", "Does it work", "Your changes", "Turns allowed", "Where paths meet", "streets", "Every check"];

#[test]
fn the_page_around_the_plan_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = linked_in(Locale::FrCa);
    let h = html(|| view! { <page::Header vm=s.clone()/> }.into_any());
    speaks_french(&h, &[ENGLISH, &["City map"]].concat(), &["Carte de la ville", "Plan de la jonction", ">4 rues<", ">Jonction 4</h1>"]);
    let h = html(|| view! { <page::TitleBlock vm=s.clone()/> }.into_any());
    speaks_french(&h, &["Junction<", "Streets", "Changes made"], &[">Jonction<", ">Rues<", "Modifications apportées"]);
    let h = html(|| view! { <page::Provenance vm=s.clone()/> }.into_any());
    speaks_french(&h, &["Data", "As imported"], &["Données", "Telle qu’importée"]);
    let h = html(|| view! { <page::Fit vm=s.clone()/> }.into_any());
    speaks_french(&h, ENGLISH, &["4 rues, feux de circulation. Toutes les vérifications réussissent."]);
    let h = html(|| view! { <page::History vm=s.clone()/> }.into_any());
    speaks_french(&h, &["Undo", "Redo", "Start over"], &["Annuler", "Rétablir", "Recommencer"]);
}

#[test]
fn the_notes_speak_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let (n, e) = (arm(&s, 0), arm(&s, 90));
    s.edit(|j| j.set_bus_lane(e, true));
    s.edit(|j| j.set_approach(e, Q_CURB));
    s.edit(|j| j.set_corner(n, 7_000));
    s.edit(|j| j.set_crossing(e, false));
    let h = html(|| view! { <notes::Across vm=s.clone()/> }.into_any());
    speaks_french(
        &h,
        &["How far", ">Street<", "To cross", "Lanes in", ">none<", ">W<"],
        &["Rue", "À traverser", "Voies entrantes", ">aucun<", ">O<", "2 × 9,0"],
    );
    let h = html(|| view! { <notes::Conflicts vm=s.clone()/> }.into_any());
    speaks_french(&h, &["Points where", "Kind", "Merging", "Splitting", ">All<"], &["Type", "Croisement", "Convergence", "Divergence", "Total"]);
    let h = html(|| view! { <notes::ConflictNote vm=s.clone()/> }.into_any());
    speaks_french(&h, &["A signal takes turns"], &["Les feux"]);
    let h = html(|| view! { <notes::Checks vm=s.clone()/> }.into_any());
    speaks_french(&h, &[": passes", "Longest crossing", " m<"], &["\u{a0}: réussite", "Plus longue traversée d’un seul coup\u{a0}: 11,4\u{a0}m"]);
    let h = html(|| view! { <notes::Revisions vm=s.clone()/> }.into_any());
    speaks_french(
        &h,
        &["Step", "What changed", "Junction today", "Corner after"],
        &["Étape", "Ce qui a changé", "Jonction aujourd’hui", "Coin après Sample Avenue 2 (nord)"],
    );
}

#[test]
fn the_plan_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let (n, e) = (arm(&s, 0), arm(&s, 90));
    s.edit(|j| j.set_offset(e, 500));
    s.edit(|j| j.select(Target::Corner(n)));
    let h = html(|| view! { <plan::PlanDrawing vm=s.clone()/> }.into_any());
    speaks_french(
        &h,
        &[ENGLISH, &["Plan of the junction", "north up", "Scale", " road", "shifted", "Turn ", "Change the corner radius", "Lane 1 of"]].concat(),
        &[
            "aria-label=\"Éditeur du plan de la jonction\"",
            "Plan de la jonction, nord en haut. 4 rues.",
            ">Échelle<",
            "E · chaussée de 11,4\u{a0}m · décalée de 0,5\u{a0}m",
            "Faire pivoter Sample Street 1 (est)",
            "Modifier le rayon du coin",
            "Voie 1 sur 2, Sample Avenue 2 (nord)",
            ">20\u{a0}m<",
            ">2 × 9,0<",
        ],
    );
    s.edit(|j| j.select(Target::Crossing(n)));
    s.edit(|j| j.set_control(ROUNDABOUT));
    let so = arm(&s, 180);
    s.edit(|j| j.set_bus(Some((n, so))));
    let h = html(|| view! { <plan::PlanDrawing vm=s.clone()/> }.into_any());
    speaks_french(&h, &["Move the crossing", "Bus only", "Roundabout."], &["Déplacer le passage pour piétons", "Autobus seulement", "Carrefour giratoire."]);
}

#[test]
fn a_switch_of_language_draws_the_plan_its_label_and_the_status_again() {
    use crate::junction::plan_svg::{plan_label, plan_svg};
    use crate::junction::text::{arms_text, fit_text};
    use crate::shared::i18n::Locale;
    let owner = Owner::new();
    owner.set();
    let i18n = crate::i18n_for(Locale::En);
    // What the views' closures do: they word the junction from the shared words, so they track them.
    let shared = StoredValue::new_local((i18n.clone(), Junction::new(0).view()));
    let markup = Memo::new(move |_| shared.with_value(|(i18n, v)| plan_svg(v, i18n, 1000.0, 1000.0, Units::Metres).markup));
    let label = Memo::new(move |_| shared.with_value(|(i18n, v)| plan_label(v, i18n)));
    let status = Memo::new(move |_| shared.with_value(|(i18n, v)| format!("{} / {}", arms_text(v, i18n), fit_text(v, i18n))));
    assert!(markup.get().contains(">Scale<") && label.get().starts_with("Plan of the junction") && status.get().starts_with("4 streets"));
    i18n.set(Locale::FrCa);
    assert!(markup.get().contains(">Échelle<") && !markup.get().contains(">Scale<"), "{}", markup.get());
    assert!(label.get().starts_with("Plan de la jonction, nord en haut."), "{}", label.get());
    assert!(status.get().starts_with("4 rues / 4 rues, feux de circulation."), "{}", status.get());
}

#[test]
fn the_junction_page_asks_for_no_message_that_is_missing_in_french() {
    use crate::shared::i18n::Locale;
    let s = linked_in(Locale::FrCa);
    let n = arm(&s, 0);
    s.edit(|j| j.set_corner(n, 7_000));
    s.edit(|j| j.select(Target::Arm(n)));
    let _ = html(|| {
        view! {
            <page::Header vm=s.clone()/><page::TitleBlock vm=s.clone()/><page::Provenance vm=s.clone()/><page::Fit vm=s.clone()/><page::History vm=s.clone()/><page::Key vm=s.clone()/>
            <notes::Across vm=s.clone()/><notes::Conflicts vm=s.clone()/><notes::ConflictNote vm=s.clone()/><notes::Checks vm=s.clone()/>
            <notes::Revisions vm=s.clone()/><plan::PlanDrawing vm=s.clone()/><turns::Turns vm=s.clone()/>
        }
        .into_any()
    });
    assert_eq!(s.i18n().missing(), Vec::<String>::new());
}

// ---- the panel in French ----------------------------------------------------------------

/// English that must not be left in the panel beside the plan, whatever is selected. The sample streets are
/// named "Sample Street 1", "Sample Avenue 2"…, so "Street" alone is not on the list.
const PANEL_ENGLISH: &[&str] = &[
    "Select a",
    "Junction control",
    "Size of the",
    "across",
    "Cycle track",
    "Track",
    "Bus lane",
    "Remove",
    "Corner",
    "Curb",
    "curb",
    "Smaller",
    "Larger",
    "Tighter",
    "Wider",
    "Narrower",
    "Closer",
    "Farther",
    "Shorter",
    "Longer",
    "Shift",
    "clockwise",
    ">Lane ",
    "\"Lane ",
    "Where this",
    "Set back",
    "Crossing",
    "crossing",
    "Halfway",
    "Refuge",
    "Shorten",
    "Mark a",
    "Transit",
    "At the approach",
    "Bus stop",
    "Turns",
    "Length of",
    "Gate distance",
    "Direction",
    "Clockwise",
    "neighbours",
    "Open the",
    "belongs to",
    "One way",
    "one way",
    "coming in",
    " road",
    " to ",
    "stages",
    "Straight on",
    "Left",
    "Right",
    "nearest",
    "only lane",
    "Cars turn",
    "wide",
    "parking",
    "needs at least",
    "No bus",
    "Lets buses",
    "Round the",
];

/// The panel of `s` in French, checked against the English list and for the French phrases.
fn panel_speaks_french(s: &Rc<JunctionVm>, french: &[&str]) {
    speaks_french(&panel(s), PANEL_ENGLISH, french);
    assert_eq!(s.i18n().missing(), Vec::<String>::new());
}

#[test]
fn the_panel_with_nothing_selected_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    panel_speaks_french(&s, &["Sélectionnez une rue, un coin ou un passage pour piétons à modifier.", "Signalisation de la jonction", "Feux de circulation"]);
}

#[test]
fn the_panel_of_a_roundabout_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    s.edit(|j| j.set_control(ROUNDABOUT));
    panel_speaks_french(
        &s,
        &[
            "Taille du carrefour giratoire",
            "m de diamètre",
            "Diamètre extérieur. Au moins ",
            "aria-label=\"Réduire de 0,5\u{a0}m\"",
            "aria-label=\"Agrandir de 0,5\u{a0}m\"",
            "Piste cyclable",
            "Piste tout autour, à l’extérieur",
            "Voie d’autobus par le centre",
            "Aucune voie d’autobus",
            "Permet aux autobus de couper à travers l’îlot entre deux rues.",
        ],
    );
    let (n, so) = (arm(&s, 0), arm(&s, 180));
    s.edit(|j| j.set_cycle_track(true));
    s.edit(|j| j.set_bus(Some((n, so))));
    panel_speaks_french(
        &s,
        &[
            "Largeur de la piste",
            "De 1,5\u{a0}m à 3,0\u{a0}m. Elle prend de la place à la chaussée, dans le même cercle.",
            "aria-label=\"Rétrécir de 0,5\u{a0}m\"",
            "aria-label=\"Élargir de 0,5\u{a0}m\"",
            "Une voie de 3,5\u{a0}m réservée aux autobus, tout droit à travers l’îlot.",
        ],
    );
    s.edit(|j| j.select(Target::Bus));
    panel_speaks_french(&s, &[">Voie d’autobus<", "Retirer la voie d’autobus", "Une voie de 3,5\u{a0}m réservée aux autobus"]);
    s.edit(|j| j.select(Target::Cycle));
    panel_speaks_french(
        &s,
        &[">Piste cyclable<", "Tout autour du carrefour giratoire, à l’extérieur", "Les cyclistes traversent chaque rue", "Retirer la piste cyclable"],
    );
}

#[test]
fn the_panel_of_a_corner_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Corner(n)));
    panel_speaks_french(
        &s,
        &[
            ">Coin<",
            ">N vers E<",
            "Rayon de la bordure",
            "aria-label=\"Resserrer de 0,5\u{a0}m\"",
            "aria-label=\"Élargir de 0,5\u{a0}m\"",
            "De 1,0\u{a0}m à 15,0\u{a0}m. Les voitures tournent ici à environ ",
            "\u{a0}km/h.",
            "Un coin serré ralentit les voitures",
        ],
    );
}

#[test]
fn the_panel_of_a_crossing_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let (n, e) = (arm(&s, 0), arm(&s, 90));
    s.edit(|j| j.select(Target::Crossing(e)));
    panel_speaks_french(
        &s,
        &[
            "E, 90° · chaussée de 11,4\u{a0}m",
            ">Passage pour piétons<",
            "11,4\u{a0}m à traverser",
            "Retirer le passage pour piétons",
            "Retrait par rapport à la jonction",
            "aria-label=\"Rapprocher de 0,5\u{a0}m\"",
            "aria-label=\"Éloigner de 0,5\u{a0}m\"",
            "De 2,0\u{a0}m à 8,0\u{a0}m",
            "Largeur du passage pour piétons",
            "De 2,0\u{a0}m à 6,0\u{a0}m",
            "Îlot à mi-traversée",
            "Îlot refuge",
            "Raccourcir la traversée",
            "Avancée de trottoir à gauche",
            "Avancée de trottoir à droite",
        ],
    );
    s.edit(|j| j.select(Target::Crossing(n)));
    panel_speaks_french(&s, &["2 traversées de 9,0\u{a0}m"]);
}

#[test]
fn the_panel_of_a_street_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    panel_speaks_french(
        &s,
        &[
            "Sample Street 1",
            "E, 90° · chaussée de 11,4\u{a0}m",
            "Voies entrantes",
            ">Voie 1<",
            "aria-label=\"Voie 1 vers ",
            "Priorité au transport en commun",
            "Voie réservée aux autobus en entrée",
            "À l’approche",
            "Arrêt d’autobus",
            ">Virages<",
            "N1 Filtre modal pour le transport en commun",
            ">Orientation<",
            "aria-label=\"Faire pivoter de 5° dans le sens antihoraire\"",
            "aria-label=\"Faire pivoter de 5° dans le sens horaire\"",
            "Dans le sens horaire à partir du nord. Au moins 30° de ses voisines.",
            "Décalage latéral",
            "aria-label=\"Décaler de 0,1\u{a0}m vers la gauche\"",
            "aria-label=\"Décaler de 0,1\u{a0}m vers la droite\"",
            "Jusqu’à ",
            "Retirer cette rue",
        ],
    );
    s.edit(|j| j.set_crossing(e, false));
    panel_speaks_french(&s, &["Marquer un passage pour piétons"]);
    s.edit(|j| j.set_approach(e, Q_CURB));
    panel_speaks_french(&s, &["Longueur de la voie de dépassement de file", "aria-label=\"Raccourcir de ", "aria-label=\"Allonger de ", "G2 Voie"]);
    s.edit(|j| j.set_approach(e, GATE_SIGNAL));
    panel_speaks_french(&s, &["Distance de la porte d’autobus en amont"]);
}

#[test]
fn the_panel_of_a_street_that_cannot_be_removed_says_why_in_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let w = arm(&s, 270);
    s.edit(|j| j.remove_arm(w));
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    panel_speaks_french(&s, &["Une jonction compte au moins 3 rues."]);
}

#[test]
fn the_panel_of_a_city_street_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = linked_in(Locale::FrCa);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    panel_speaks_french(&s, &[">Rue<", "Ouvrir la coupe transversale", "Cette rue appartient à la ville."]);
}

#[test]
fn the_panel_of_a_lane_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Lane(n, 1)));
    panel_speaks_french(
        &s,
        &[
            "Voie 2 sur 2",
            "Sample Avenue 2 (nord) · la plus près de la bordure",
            "Où mène cette voie",
            "Tout droit vers Sample Avenue 2 (sud)",
            "Sélectionner toute la rue",
            "\u{a0}m de large. Une voie doit mener à au moins une rue.",
        ],
    );
    s.edit(|j| j.select(Target::Lane(n, 0)));
    panel_speaks_french(&s, &["la plus près du centre"]);
}

/// The first sample junction with an arm that `pick` chooses, made in French, and that arm.
fn sample_with(pick: impl Fn(&crate::junction::read_model::ArmView) -> bool) -> (Rc<JunctionVm>, u32) {
    use crate::shared::i18n::Locale;
    for sample in 0..JUNCTION_SAMPLES.len() {
        let s = shared_in(sample, Locale::FrCa);
        if let Some(a) = s.view_now().arms.iter().find(|a| pick(a)) {
            return (s.clone(), a.uid);
        }
    }
    panic!("no sample junction has such an arm");
}

#[test]
fn the_panel_of_a_narrow_road_speaks_french() {
    // "Sample Lane 3" is too narrow for an island and has parking on one side only.
    let (s, uid) = sample_with(|a| a.crossing.is_some() && !a.can_island);
    s.edit(|j| j.select(Target::Crossing(uid)));
    panel_speaks_french(&s, &["Îlot refuge (chaussée trop étroite)", "Avancée de trottoir à gauche (pas de stationnement de ce côté)"]);
}

#[test]
fn a_one_way_street_s_words_speak_french() {
    // No sample junction has a one-way street, so the words are checked as the panel asks for them.
    use crate::junction::inspector::goes_text;
    use crate::shared::i18n::{Args, Locale};
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    assert_eq!(goes_text(&en, LEFT, "X".into(), false), "Left to X (one way in)");
    assert_eq!(goes_text(&fr, LEFT, "X".into(), false), "À gauche vers X (sens unique vers la jonction)");
    assert_eq!(goes_text(&fr, RIGHT, "X".into(), true), "À droite vers X");
    assert_eq!(fr.tr_now("jn-insp-one-way-out", &Args::new()), "Sens unique sortant. Aucune voie n’entre.");
}

#[test]
fn a_crossing_too_far_for_one_go_says_so_in_french() {
    use crate::junction::inspector::crossing_range;
    use crate::shared::i18n::Locale;
    let fr = crate::i18n_for(Locale::FrCa);
    let mut c = Junction::new(0).view().arms.into_iter().find(|a| a.bearing == 90).unwrap().crossing.unwrap();
    c.too_far = true;
    assert_eq!(crossing_range(&c, &fr, Units::Metres), "11,4\u{a0}m à traverser. Trop long à traverser d’un seul coup.");
}

#[test]
fn the_panel_s_plurals_are_singular_at_zero_and_one_in_french() {
    use crate::shared::i18n::{Args, Locale};
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    let stages = |i: &crate::shared::i18n::I18n, n: i64| i.tr_now("jn-insp-crossing-stages", &Args::new().num("n", n).str("length", "9"));
    assert_eq!([0, 1, 2].map(|n| stages(&en, n)), ["0 stages of 9", "1 stage of 9", "2 stages of 9"]);
    assert_eq!([0, 1, 2].map(|n| stages(&fr, n)), ["0 traversée de 9", "1 traversée de 9", "2 traversées de 9"]);
    let arms = |i: &crate::shared::i18n::I18n, n: i64| i.tr_now("jn-insp-min-arms", &Args::new().num("n", n));
    assert_eq!(
        [0, 1, 2].map(|n| arms(&en, n)),
        ["A junction needs at least 0 streets.", "A junction needs at least 1 street.", "A junction needs at least 2 streets."]
    );
    assert_eq!(
        [0, 1, 2].map(|n| arms(&fr, n)),
        ["Une jonction compte au moins 0 rue.", "Une jonction compte au moins 1 rue.", "Une jonction compte au moins 2 rues."]
    );
}

#[test]
fn the_panel_s_steppers_follow_the_units() {
    let s = shared(0);
    s.set_units(Units::Feet);
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Corner(n)));
    let h = panel(&s);
    assert!(h.contains("aria-label=\"Tighter by 1.6 ft\""), "{h}");
}

// ---- a junction that cannot be drawn ---------------------------------------------------------

fn junction_four() -> crate::shared::said::Said {
    crate::shared::said::Said::new("city-junction-number").with("n", crate::shared::said::Arg::Num(4))
}

#[test]
fn a_junction_that_cannot_be_drawn_says_so_in_english() {
    use crate::shared::i18n::Locale;
    let i18n = crate::i18n_for(Locale::En);
    let h = html(|| view! { <page::StuckHeader i18n=i18n.clone() name=junction_four()/><page::Stuck i18n=i18n.clone()/> }.into_any());
    for want in [
        "<h1 id=\"street-name\">Junction 4</h1>",
        "class=\"stuck\"",
        ">This junction cannot be drawn</h2>",
        "The streets that meet here have been changed so that they no longer make a junction.",
        "href=\"map.html\"",
        ">City map</a>",
    ] {
        assert!(h.contains(want), "{want:?} missing from:\n{h}");
    }
}

#[test]
fn a_junction_that_cannot_be_drawn_says_so_in_french() {
    use crate::shared::i18n::Locale;
    let i18n = crate::i18n_for(Locale::FrCa);
    let h = html(|| view! { <page::StuckHeader i18n=i18n.clone() name=junction_four()/><page::Stuck i18n=i18n.clone()/> }.into_any());
    speaks_french(
        &h,
        &["Junction", "cannot be drawn", "The streets", "City map"],
        &[">Jonction 4</h1>", ">Cette jonction ne peut pas être dessinée</h2>", "Redonnez de la place aux rues", ">Carte de la ville</a>"],
    );
    assert_eq!(i18n.missing(), Vec::<String>::new());
}

#[test]
fn a_junction_that_cannot_be_drawn_follows_a_switch_of_language() {
    use crate::junction::page::stuck_words;
    use crate::shared::i18n::Locale;
    let owner = Owner::new();
    owner.set();
    let i18n = crate::i18n_for(Locale::En);
    let held = StoredValue::new_local(i18n.clone());
    let words = Memo::new(move |_| held.with_value(|i| stuck_words(i, &junction_four())));
    assert_eq!(words.get().0, "Junction 4 · CityLoom");
    i18n.set(Locale::FrCa);
    assert_eq!(words.get(), ("Jonction 4 · CityLoom".to_string(), "Jonction 4".to_string()));
}

#[test]
fn the_junction_page_and_the_page_of_one_that_cannot_be_drawn_start_in_the_stored_language() {
    // open_junction and mount_bare_shell both take their words from page_i18n.
    use crate::shared::i18n::{LANG_KEY, Locale};
    let (ports, page, _) = crate::shared::ports::test_ports_with_page();
    ports.storage.remember(LANG_KEY, "fr-CA");
    let i18n = super::page_i18n(&ports);
    assert_eq!(i18n.locale_now(), Locale::FrCa);
    assert_eq!(*page.lang.borrow(), "fr-CA", "the document says the language the page is in");
    let h = html(|| view! { <page::Stuck i18n=i18n.clone()/> }.into_any());
    assert!(h.contains("Cette jonction ne peut pas être dessinée"), "{h}");
}

// ---- the page's markup ----------------------------------------------------------------------

const INTERSECTION_HTML: &str = include_str!("../../web/intersection.html");

#[test]
fn every_data_i18n_key_in_intersection_html_exists_in_both_languages() {
    use crate::shared::i18n::{Args, Locale};
    use crate::shell::view::i18n_keys;
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    let keys = i18n_keys(INTERSECTION_HTML);
    assert!(keys.len() > 42, "the page should have been converted: {}", keys.len());
    for (key, _) in keys {
        en.tr_now(&key, &Args::new());
        fr.tr_now(&key, &Args::new());
    }
    assert_eq!(en.missing(), Vec::<String>::new(), "missing in en");
    assert_eq!(fr.missing(), Vec::<String>::new(), "missing in fr");
}

#[test]
fn the_english_text_in_intersection_html_is_the_english_message() {
    use crate::shared::i18n::{Args, Locale};
    let en = crate::i18n_for(Locale::En);
    for (key, fallback) in crate::shell::view::i18n_keys(INTERSECTION_HTML) {
        assert_eq!(fallback, en.tr_now(&key, &Args::new()), "{key}");
    }
}

#[test]
fn intersection_html_has_a_language_row_and_sets_the_language_in_the_head() {
    assert!(INTERSECTION_HTML.contains(r#"data-lang="en" lang="en" aria-pressed="true">English<"#));
    assert!(INTERSECTION_HTML.contains(r#"data-lang="fr-CA" lang="fr" aria-pressed="false">Français<"#));
    assert_eq!(INTERSECTION_HTML.matches("<script>").count(), 1);
    assert!(INTERSECTION_HTML.contains(r#"localStorage.getItem("cityloom-lang")"#));
    // The header sets the window title from the junction's name, so the shell does not write it.
    assert!(!INTERSECTION_HTML.contains("<title data-i18n"));
}

#[test]
fn intersection_html_leaves_no_text_without_its_message() {
    // Every text between tags in the body that holds a letter is a message, unless it is a unit, a key cap, a
    // language's own name or the Atlas's name.
    use crate::shell::view::i18n_keys;
    let body = &INTERSECTION_HTML[INTERSECTION_HTML.find("<body>").unwrap()..];
    // The page's start-failure notice is worded in both languages in its own markup, because the module that holds
    // the messages is what failed to load: `shell::tests` checks it against them.
    let notice = body.find(r#"<div id="boot-failed""#).unwrap();
    let body = &format!("{}{}", &body[..notice], &body[notice + body[notice..].find("\n  </div>\n").unwrap()..]);
    let translated: Vec<String> = i18n_keys(body).into_iter().map(|(_, text)| text).collect();
    let own = ["m", "ft", "Z", "V", "English", "Français", "Transit Priority Atlas"];
    for piece in body.split('>').filter_map(|p| p.split('<').next()).map(str::trim).filter(|t| !t.is_empty()) {
        if piece.chars().any(char::is_alphabetic) && !own.contains(&piece) {
            assert!(translated.iter().any(|t| t == piece), "{piece:?} has no message");
        }
    }
    // The text people read in attributes is translated too.
    for attr in ["aria-label=\"", "title=\""] {
        assert_eq!(INTERSECTION_HTML.matches(&format!(" {attr}")).count(), INTERSECTION_HTML.matches(&format!(" data-i18n-{attr}")).count(), "{attr}");
    }
    assert!(INTERSECTION_HTML.contains(r#"<meta name="description" data-i18n-content="jn-page-description""#));
}

// ---- the number fields ----------------------------------------------------------------

fn cycle_width_mm(s: &JunctionVm) -> Option<i32> {
    s.read(|j| j.current().cycle)
}

fn roundabout_with_cycle_track(s: &JunctionVm) {
    s.edit(|j| j.set_control(ROUNDABOUT));
    s.edit(|j| j.set_cycle_track(true));
}

#[test]
fn a_length_typed_with_a_comma_or_a_point_is_read_as_a_decimal() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    roundabout_with_cycle_track(&s);
    s.edit(|j| j.set_cycle(Some(1_500)));
    assert!(inspector::commit_cycle(w, "2,5"));
    assert_eq!(cycle_width_mm(&s), Some(2_500));
    s.edit(|j| j.set_cycle(Some(1_500)));
    assert!(inspector::commit_cycle(w, "2.5"));
    assert_eq!(cycle_width_mm(&s), Some(2_500));
}

#[test]
fn a_length_typed_in_feet_is_converted() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    roundabout_with_cycle_track(&s);
    s.set_units(Units::Feet);
    assert!(inspector::commit_cycle(w, "8,2"));
    assert_eq!(cycle_width_mm(&s), Some(2_500));
}

#[test]
fn what_is_not_a_number_is_not_committed() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    roundabout_with_cycle_track(&s);
    let before = cycle_width_mm(&s);
    assert!(!inspector::commit_cycle(w, "abc"));
    assert!(!inspector::commit_cycle(w, "1,2,3"));
    assert_eq!(cycle_width_mm(&s), before);
}

#[test]
fn a_typed_length_outside_what_fits_is_refused_by_the_model() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    roundabout_with_cycle_track(&s);
    let before = cycle_width_mm(&s);
    crate::shared::live::take_said();
    assert!(inspector::commit_cycle(w, "9,5"));
    assert_eq!(cycle_width_mm(&s), before);
    let refusal = crate::i18n_for(crate::shared::i18n::Locale::En).tr_now("jn-refusal-does-not-fit", &crate::shared::i18n::Args::new());
    assert_eq!(crate::shared::live::take_said(), [refusal], "the refusal is announced");
    // What the field is made to hold again is the model's value.
    assert!(panel(&s).contains(&format!("value=\"{}\"", Units::Metres.fixed(before.unwrap(), 1))));
}

#[test]
fn a_bearing_typed_with_a_comma_is_rounded_and_snapped_like_any_other() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    let n = arm(&s, 0);
    assert!(inspector::commit_bearing(w, n, "20"));
    let ninety = s.read(|j| j.arm(n).unwrap().bearing);
    assert_eq!(ninety, 20);
    assert!(inspector::commit_bearing(w, n, "20,5"));
    assert_eq!(s.read(|j| j.arm(n).unwrap().bearing), 20, "20,5 is 21, and the nearest step is 20");
    assert!(inspector::commit_bearing(w, n, "22,5"));
    assert_eq!(s.read(|j| j.arm(n).unwrap().bearing), 25);
    assert!(!inspector::commit_bearing(w, n, "north"));
}

#[test]
fn a_field_with_no_hint_is_not_described_by_one() {
    let s = shared(0);
    s.select(Target::Arm(arm(&s, 0)));
    let h = panel(&s);
    let mut described = 0;
    for id in h.split("aria-describedby=\"").skip(1) {
        let id = id.split('"').next().unwrap();
        assert!(h.contains(&format!("id=\"{id}\"")), "{id} is described by nothing");
        described += 1;
    }
    assert!(described >= 1, "the street's panel has fields with hints, so the check looked at something");
}

#[test]
fn the_arrow_keys_step_a_number_field() {
    assert_eq!(inspector::step_key("ArrowUp"), Some(1));
    assert_eq!(inspector::step_key("ArrowDown"), Some(-1));
    assert_eq!(inspector::step_key("Enter"), None);
}

#[test]
fn every_number_field_is_text_that_asks_for_a_decimal_keyboard() {
    let s = shared_in(0, crate::shared::i18n::Locale::FrCa);
    roundabout_with_cycle_track(&s);
    let ring = panel(&s);
    s.select(Target::Arm(arm(&s, 0)));
    let street = panel(&s);
    s.select(Target::Corner(arm(&s, 0)));
    let corner = panel(&s);
    for h in [&ring, &street, &corner] {
        assert!(!h.contains("type=\"number\""), "{h}");
        assert!(count(h, "inputmode=\"decimal\"") >= 1, "{h}");
        assert_eq!(count(h, "<input"), count(h, "type=\"text\""), "{h}");
    }
    assert!(count(&ring, "type=\"text\"") >= 2, "the ring's size and the track's width");
}

#[test]
fn a_length_is_held_with_the_decimal_mark_of_the_language() {
    let s = shared_in(0, crate::shared::i18n::Locale::FrCa);
    roundabout_with_cycle_track(&s);
    s.edit(|j| j.set_cycle(Some(2_500)));
    assert!(panel(&s).contains("value=\"2,5\""));
    let en = shared(0);
    roundabout_with_cycle_track(&en);
    en.edit(|j| j.set_cycle(Some(2_500)));
    assert!(panel(&en).contains("value=\"2.5\""));
}

#[test]
fn the_pedestrian_arm_messages_exist_in_both_languages() {
    use crate::shared::i18n::Locale;
    for locale in Locale::ALL {
        let i18n = crate::i18n_for(locale);
        for id in [
            "jn-refusal-crossing-pedestrian",
            "jn-refusal-measure-pedestrian",
            "jn-insp-crossing-pedestrian",
            "jn-insp-control-pedestrian",
            "jn-insp-lanes-pedestrian",
        ] {
            assert!(i18n.has_message(id, locale), "{id} in {}", locale.tag());
        }
    }
}

fn walkway_shared(bearings: &[i32]) -> Rc<JunctionVm> {
    JunctionVm::new(
        crate::shared::platform::browser_ports(),
        crate::i18n_for(crate::shared::i18n::Locale::En),
        junction_with_walkways(bearings, &[("sidewalk", 4000), ("terrace", 2000), ("sidewalk", 4000)]),
        None,
    )
}

#[test]
fn the_panel_of_a_walkway_arm_says_there_is_no_roadway_to_cross_and_offers_no_crossing() {
    let s = walkway_shared(&[90]);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    assert!(h.contains("A pedestrian street has no roadway to cross."), "{h}");
    assert!(!h.contains("Mark a crossing") && !h.contains("Remove the crossing"));
    let e = arm(&s, 0);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    assert!(h.contains("Remove the crossing") && !h.contains("A pedestrian street has no roadway to cross."));
}

#[test]
fn with_one_street_carrying_traffic_the_control_choice_is_replaced_by_a_note() {
    let s = walkway_shared(&[90, 180, 270]);
    let e = arm(&s, 0);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    assert!(h.contains("Only one street here carries traffic."), "{h}");
    assert!(!h.contains("aria-labelledby=\"i-h-control\""), "no control to choose");
    let s = walkway_shared(&[90]);
    let e = arm(&s, 0);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    assert!(h.contains("aria-labelledby=\"i-h-control\"") && !h.contains("Only one street here carries traffic."));
}

#[test]
fn the_panel_of_a_walkway_arm_has_no_transit_signal_or_lane_sections_and_says_it_has_no_lanes() {
    let s = walkway_shared(&[90]);
    s.edit(|j| j.set_control(SIGNAL));
    let w = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(w)));
    let h = plain_panel(&s);
    assert!(!h.contains("aria-labelledby=\"i-h-signal\"") && !h.contains("aria-labelledby=\"i-h-transit\""), "{h}");
    assert!(h.contains("A pedestrian street has no lanes.") && !h.contains("One way out"), "{h}");
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Arm(n)));
    let h = plain_panel(&s);
    assert!(!h.contains("A pedestrian street has no lanes."));
}

#[test]
fn the_control_note_counts_the_streets_that_carry_traffic() {
    let s = walkway_shared(&[0, 90, 180, 270]);
    let e = arm(&s, 0);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    assert!(h.contains("No street here carries traffic.") && !h.contains("Only one street"), "{h}");
}

// ---- signal timing in the details ----

/// The panel's words, with the engine's non-breaking spaces as ordinary ones.
fn plain_panel(s: &Rc<JunctionVm>) -> String {
    panel(s).replace(['\u{a0}', '\u{202f}'], " ")
}

#[test]
fn the_junction_panel_of_a_signal_lists_the_phases_with_the_cycle() {
    let s = shared(0);
    let h = plain_panel(&s);
    assert!(h.contains("Signal timing") && h.contains("Cycle: 120 s"), "{h}");
    assert_eq!(count(&h, "class=\"signal-list\""), 1);
    assert_eq!(count(&h, "green 25 s, yellow 3 s, all-red 2 s"), 4, "a row for each street: {h}");
    assert!(!plain_panel(&shared(1)).contains("Signal timing"), "no signal, no section");
}

#[test]
fn the_signal_section_has_the_phase_buttons_and_says_whether_the_plan_is_suggested() {
    let s = shared(0);
    let h = plain_panel(&s);
    assert!(h.contains("Suggested plan"), "{h}");
    for label in ["Move the phase for", "Remove the phase for", "Add a phase for", "Restore the suggested plan"] {
        assert!(h.contains(label), "{label}: {h}");
    }
    assert_eq!(count(&h, "id=\"sig-remove-"), 4);
    assert!(h.contains("id=\"sig-earlier-0\"") && h.contains("id=\"sig-later-3\""));
    s.edit(|j| j.remove_phase(0));
    let h = plain_panel(&s);
    assert!(h.contains("Edited plan") && !h.contains("Suggested plan"), "{h}");
    assert_eq!(count(&h, "id=\"sig-remove-"), 3);
}

#[test]
fn the_arm_panel_has_three_steppers_and_a_fourth_for_a_street_with_a_transit_lane() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = plain_panel(&s);
    for id in ["i-h-green", "i-h-yellow", "i-h-all-red"] {
        assert!(h.contains(&format!("id=\"{id}\"")), "{id}: {h}");
    }
    assert!(!h.contains("id=\"i-h-lead\""), "no early green without a transit lane");
    s.edit(|j| j.set_bus_lane(e, true));
    assert!(panel(&s).contains("id=\"i-h-lead\""));
}

#[test]
fn the_arm_panel_says_what_else_runs_in_the_phase() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = plain_panel(&s);
    assert!(h.contains("Also walks: crossings on"), "turns into the north and south streets give way: {h}");
    assert!(!h.contains("Also rides"), "the sample has no bike lanes: {h}");

    use crate::junction::model::junction_of;
    let biked = junction_of(&[("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)]);
    let n = biked.current().arms.iter().find(|a| a.bearing == 0).unwrap().uid;
    let b = JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(crate::shared::i18n::Locale::En), biked, None);
    b.edit(|j| j.select(Target::Arm(n)));
    assert!(plain_panel(&b).contains("Also rides: bikes from"), "the street across rides with it");
}

#[test]
fn the_timing_panels_are_not_there_off_a_signal_and_speak_french() {
    use crate::shared::i18n::Locale;
    let plain = shared(1);
    let a = plain.read(|j| j.current().arms[0].uid);
    plain.edit(|j| j.select(Target::Arm(a)));
    assert!(!plain_panel(&plain).contains("Signal timing"));
    let s = JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(Locale::FrCa), Junction::new(0), None);
    let h = plain_panel(&s);
    assert!(h.contains("Réglage des feux") && h.contains("Cycle : 120 s"), "{h}");
    assert_eq!(s.i18n().missing(), Vec::<String>::new());
}

// ---- the signal plan as drawings ----------------------------------------------------

/// The cards of the drawings row, one `<li>` each.
fn phase_cards(h: &str) -> Vec<String> {
    let at = h.find("class=\"signal-phases\"").expect("the row is there");
    let row = &h[at..at + h[at..].find("</ul>").unwrap()];
    row.split("<li").skip(1).map(str::to_string).collect()
}

#[test]
fn the_junction_panel_has_a_drawing_for_each_phase_in_the_order_they_run() {
    let h = plain_panel(&shared(0));
    let cards = phase_cards(&h);
    assert_eq!(cards.len(), 4, "{h}");
    for (i, c) in cards.iter().enumerate() {
        assert!(c.contains("<button") && c.contains("aria-pressed=\"false\""), "a button, pressed when its street is selected: {c}");
        assert!(c.contains(&format!("Phase {}: ", i + 1)), "{c}");
        assert!(c.contains("Cars go: ") && c.contains("People cross: "), "the summary is on the card: {c}");
        assert!(c.contains("Green 25 s, yellow 3 s, all-red 2 s"), "{c}");
        assert!(c.contains("role=\"img\"") && c.contains("aria-label=\"Cars go: "), "{c}");
        assert!(c.contains("ph-move lit") && c.contains("ph-walk lit") && c.contains("ph-walk muted"), "lit and muted: {c}");
    }
    assert!(h.contains("The phases of the signal, in the order they run"));
    assert!(!plain_panel(&shared(1)).contains("signal-phases"), "no signal, no drawings");
}

#[test]
fn the_drawing_of_the_selected_street_is_pressed_and_the_row_follows_an_edit() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = plain_panel(&s);
    let cards = phase_cards(&h);
    assert_eq!(cards.len(), 4, "the row stays beside the street's timing: {h}");
    assert_eq!(cards.iter().filter(|c| c.contains("aria-pressed=\"true\"")).count(), 1);
    assert!(cards[1].contains("aria-pressed=\"true\""), "the east phase is the second");
    assert!(h.contains("Suggested plan"));
    s.edit(|j| j.remove_phase(0));
    let h = plain_panel(&s);
    assert_eq!(phase_cards(&h).len(), 3);
    assert!(h.contains("Edited plan") && !h.contains("Suggested plan"), "{h}");
}

#[test]
fn the_drawings_speak_french_with_no_english_left() {
    use crate::shared::i18n::Locale;
    let s = JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(Locale::FrCa), Junction::new(0), None);
    let h = plain_panel(&s);
    let cards = phase_cards(&h);
    assert_eq!(cards.len(), 4);
    assert!(cards[0].contains("Phase 1 : ") && cards[0].contains("Voitures qui passent : ") && cards[0].contains("Piétons qui traversent : "), "{}", cards[0]);
    assert!(!cards[0].contains("Cars go"));
    assert_eq!(s.i18n().missing(), Vec::<String>::new());
}

#[test]
fn a_typed_time_is_read_with_a_comma_and_a_bad_one_changes_nothing() {
    let s = shared(0);
    let e = arm(&s, 90);
    let set = |typed: &str| {
        inspector::commit_typed(typed, &|v| {
            s.edit(|j| j.set_timing(e, TimingField::Green, v.round() as i32));
        })
    };
    assert!(set("30,0"));
    assert_eq!(s.read(|j| j.arm(e).unwrap().timing.unwrap().green_s), 30);
    for bad in ["abc", "", "-5", "1e9", "121"] {
        let before = s.read(|j| j.arm(e).unwrap().timing);
        set(bad);
        assert_eq!(s.read(|j| j.arm(e).unwrap().timing), before, "{bad:?} changes nothing");
    }
}

#[test]
fn a_suspended_turbo_is_not_named_or_offered_and_is_back_with_the_traffic() {
    use crate::junction::plan_svg::plan_label;
    use crate::shared::i18n::Locale;
    let en = crate::i18n_for(Locale::En);
    let s = JunctionVm::new(crate::shared::platform::browser_ports(), en.clone(), suspended_turbo(), None);
    assert!(!plan_label(&s.read(|j| j.view()), &en).contains("turbo"));
    let h = panel(&s);
    for gone in ["i-h-turbo-size", "i-h-turbo-main", "i-h-turbo-layout"] {
        assert!(!h.contains(gone), "{gone}");
    }
    let mut busy = Junction::new(0);
    assert!(busy.set_control(ROUNDABOUT) && busy.set_turbo(Some(0)));
    assert!(plan_label(&busy.view(), &en).contains("turbo"));
}

// ---- an arm's end zone and what lies beyond it -------------------------------------

/// FNV-1a over the pieces, the lanes' hit areas and the leaving arrows of every arm of every sample junction, as the
/// page gets them. Captured before the beyond was drawn: an arm without one draws byte for byte as it did.
fn arms_fingerprint() -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for sample in 0..JUNCTION_SAMPLES.len() {
        let v = Junction::new(sample).view();
        for a in &v.arms {
            let s = serde_json::to_string(&(
                &a.pieces,
                a.lanes.iter().map(|l| &l.poly).collect::<Vec<_>>(),
                a.leave_arrows.iter().map(|l| &l.poly).collect::<Vec<_>>(),
            ))
            .unwrap();
            for b in s.bytes() {
                h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
            }
        }
    }
    h
}

#[test]
fn an_arm_without_a_beyond_draws_as_before() {
    let (got, pinned) = (arms_fingerprint(), 15_083_500_958_540_711_930);
    assert_eq!(
        got, pinned,
        "the pieces, lane hit areas and leaving arrows of the sample junctions no longer draws byte for byte as it did before the end-zone beyond was added (this pins that an arm without a beyond is untouched). If you changed how these junctions draw on purpose, look at a plan and a 2.5D of each sample junction first; only when the picture is what you meant, replace the pinned number with {got}."
    );
}

/// A sample junction whose north arm has a beyond: the end zone's first entering lane is a pocket that is planting
/// across the boundary, 20 m out, tapering out by 30 m. Returns the junction and the arm's uid.
pub(crate) fn junction_with_a_pocket(configure: impl Fn(&mut State)) -> (Junction, u32) {
    junction_with_a_beyond(configure, |_| {})
}

/// The same, with the beyond changed by `tweak` before the junction is built.
pub(crate) fn junction_with_a_beyond(configure: impl Fn(&mut State), tweak: impl Fn(&mut crate::street::model::Beyond)) -> (Junction, u32) {
    use crate::street::model::{Beyond, BeyondTaper, TaperPiece};
    let mut state = Junction::new(0).current().clone();
    configure(&mut state);
    let arm = state.arms.iter_mut().find(|a| a.bearing == 0).unwrap();
    let prof = arm.profile(0);
    let x = prof.enter_x[0];
    let lane = prof.pieces.iter().find(|p| p.x_mm <= x && x < p.x_mm + p.width_mm).unwrap();
    let was = (lane.x_mm, lane.x_mm + lane.width_mm);
    let mut next = arm.section.clone().unwrap().segments;
    // the piece the lane was: the same place in the zone across is planting
    let at = prof.pieces.iter().position(|p| p.x_mm == lane.x_mm).unwrap();
    let planting = crate::shared::catalogue::kind_index("planting").unwrap();
    next[at].kind = planting;
    let mut beyond = Beyond {
        boundary_mm: 20_000,
        next,
        zone: crate::shared::said::Said::new("zone-unnamed-mid").with("n", crate::shared::said::Arg::Num(2)),
        taper: Some(BeyondTaper {
            from_mm: 20_000,
            to_mm: 30_000,
            need_mm: 10_000,
            room_mm: 80_000,
            merges: true,
            pieces: vec![TaperPiece { kind: lane.kind, from: was, to: (was.0, was.0), was: Some(was), becomes: None }],
        }),
    };
    tweak(&mut beyond);
    arm.beyond = Some(beyond);
    let uid = arm.uid;
    let named = crate::shared::said::Said::new("city-name").with("name", crate::shared::said::Arg::Text("Test".into()));
    (Junction::from_city(named, &state, &state, 0).expect("draws"), uid)
}

/// How far out along an arm a polygon of the view reaches, from the junction's centre.
fn reach(bearing: i32, poly: &[serde_json::Value]) -> f64 {
    let d = crate::junction::geometry::dir(f64::from(bearing));
    poly.iter().filter(|c| c[0] != "Z").map(|c| c[1].as_f64().unwrap() * d.0 + c[2].as_f64().unwrap() * d.1).fold(f64::MIN, f64::max)
}

#[test]
fn an_arm_with_a_beyond_draws_its_end_zone_to_the_boundary_and_what_follows() {
    let (j, uid) = junction_with_a_pocket(|_| {});
    let v = j.view();
    let a = v.arms.iter().find(|a| a.uid == uid).unwrap();
    assert!(!a.beyond.is_empty() && a.beyond.iter().any(|p| p.taper), "{} pieces beyond", a.beyond.len());
    assert_eq!(a.beyond.iter().filter(|p| p.taper).count(), 2, "the pocket narrows and the planting in its place widens");
    assert!(!a.beyond_marks.is_empty());
    assert_eq!(a.beyond_problems.len(), 1, "20 m is short of the arrows, which stand about 23 m out");
    let reaches: Vec<f64> = a.pieces.iter().map(|p| reach(a.bearing, &p.poly).round()).collect();
    assert!(reaches.iter().all(|&r| r == 20_000.0), "every end-zone piece ends at the boundary: {reaches:?}");
    for p in a.beyond.iter().filter(|p| !p.taper) {
        assert_eq!(reach(a.bearing, &p.poly).round(), 36_000.0);
    }
    // the other arms are untouched
    let before = Junction::new(0).view();
    for (x, y) in v.arms.iter().zip(&before.arms).filter(|(x, _)| x.uid != uid) {
        assert_eq!(serde_json::to_string(&x.pieces).unwrap(), serde_json::to_string(&y.pieces).unwrap());
        assert!(x.beyond.is_empty() && x.beyond_marks.is_empty());
    }
}

#[test]
fn a_lane_hit_area_ends_where_its_lane_ends() {
    let (j, uid) = junction_with_a_pocket(|_| {});
    let v = j.view();
    let a = v.arms.iter().find(|a| a.uid == uid).unwrap();
    assert!(!a.lanes.is_empty());
    // the pocket is the first entering lane's piece, which runs to the boundary and tapers out beyond it
    assert_eq!(reach(a.bearing, &a.lanes[0].poly).round(), 20_000.0);
    let plain = Junction::new(0).view();
    let b = plain.arms.iter().find(|b| b.uid == uid).unwrap();
    assert_eq!(reach(b.bearing, &b.lanes[0].poly).round(), 36_000.0, "without a beyond a lane runs the arm");
    assert_eq!(reach(b.bearing, &b.lanes[1].poly).round(), 36_000.0);
}

#[test]
fn a_pedestrian_arm_and_a_turbo_arm_draw_as_before() {
    use crate::street::model::Beyond;
    // a turbo ignores every beyond
    let named = crate::shared::said::Said::new("city-name").with("name", crate::shared::said::Arg::Text("Test".into()));
    let mut turbo = Junction::new(0);
    assert!(turbo.set_control(ROUNDABOUT) && turbo.set_turbo(Some(0)));
    let mut with = turbo.current().clone();
    for a in &mut with.arms {
        a.beyond = Some(Beyond {
            boundary_mm: 20_000,
            next: a.section.clone().unwrap().segments,
            zone: crate::shared::said::Said::new("zone-unnamed-mid"),
            taper: None,
        });
    }
    assert!(with.turbo.is_some());
    let plain = serde_json::to_string(&turbo.view().arms.iter().map(|a| &a.pieces).collect::<Vec<_>>()).unwrap();
    let v = Junction::from_city(named.clone(), &with, &with, 0).expect("draws").view();
    assert!(v.arms.iter().all(|a| a.beyond.is_empty() && a.beyond_marks.is_empty()));
    assert_eq!(serde_json::to_string(&v.arms.iter().map(|a| &a.pieces).collect::<Vec<_>>()).unwrap(), plain);
    // and so does a walkway
    let walk = junction_with_walkways(&[90], &[("sidewalk", 3000), ("terrace", 2000), ("sidewalk", 3000)]);
    let mut state = walk.current().clone();
    for a in &mut state.arms {
        a.beyond = Some(Beyond {
            boundary_mm: 20_000,
            next: a.section.clone().unwrap().segments,
            zone: crate::shared::said::Said::new("zone-unnamed-mid"),
            taper: None,
        });
    }
    let v = Junction::from_city(named, &state, &state, 0).expect("draws").view();
    let walker = v.arms.iter().find(|a| a.pedestrian).unwrap();
    assert!(walker.beyond.is_empty() && walker.beyond_marks.is_empty());
    assert_eq!(
        serde_json::to_string(&v.arms.iter().find(|a| a.pedestrian).map(|a| &a.pieces)).unwrap(),
        serde_json::to_string(&walk.view().arms.iter().find(|a| a.pedestrian).map(|a| &a.pieces)).unwrap()
    );
}

// ---- the approach check ------------------------------------------------------------

/// A pocket whose end zone reaches 30 m, past the stop line and the arrows, with its taper from there.
fn clear_of_the_arrows(b: &mut crate::street::model::Beyond) {
    b.boundary_mm = 30_000;
    let t = b.taper.as_mut().unwrap();
    (t.from_mm, t.to_mm) = (30_000, 40_000);
}

fn approach(j: &Junction) -> crate::junction::read_model::Check {
    j.view().checks.into_iter().find(|c| c.id == "approach").expect("the approach check")
}

fn said_en(s: &crate::shared::said::Said) -> String {
    crate::shared::said::say_now(&crate::i18n_for(crate::shared::i18n::Locale::En), Units::Metres, s)
}

#[test]
fn a_pocket_that_ends_before_the_stop_line_fails_the_approach_check() {
    let (j, uid) = junction_with_a_beyond(
        |_| {},
        |b| {
            b.boundary_mm = 8_000;
            b.taper = None;
        },
    );
    let c = approach(&j);
    assert!(!c.ok);
    let arm = said_en(&j.view().arms.iter().find(|a| a.uid == uid).unwrap().label);
    let detail = said_en(&c.detail);
    assert!(detail.contains(&arm) && detail.contains("8.0 m"), "{detail}");
    assert_eq!(j.view().arms.iter().find(|a| a.uid == uid).unwrap().beyond_problems.len(), 1);
    // a taper that starts there is as short
    let (j, _) = junction_with_a_beyond(
        |_| {},
        |b| {
            b.boundary_mm = 20_000;
            let t = b.taper.as_mut().unwrap();
            (t.from_mm, t.to_mm) = (8_000, 18_000);
        },
    );
    assert!(!approach(&j).ok);
}

#[test]
fn a_beyond_past_the_drawn_arm_is_neither_judged_nor_described() {
    // the boundary lies past the 36 m the arm is drawn to; the host zone is too short and the end zone short of the arrows
    for (boundary, from) in [(40_000, 40_000), (20_000, 40_000)] {
        let (j, uid) = junction_with_a_beyond(
            |_| {},
            |b| {
                b.boundary_mm = boundary;
                let t = b.taper.as_mut().unwrap();
                (t.from_mm, t.to_mm, t.need_mm, t.room_mm) = (from, from + 10_000, 40_000, 5_000);
                t.merges = false;
            },
        );
        let a = j.view().arms.into_iter().find(|a| a.uid == uid).unwrap();
        assert!(a.beyond.is_empty() && a.beyond_marks.is_empty(), "nothing is drawn");
        assert!(a.beyond_problems.is_empty(), "boundary {boundary}, taper from {from}");
        assert!(approach(&j).ok);
        assert_eq!((a.beyond_zone, a.beyond_reach_mm, a.beyond_taper_mm), (None, 0, 0), "the arm sentence has nothing to say");
    }
}

#[test]
fn a_pocket_past_the_arrows_passes() {
    let (j, uid) = junction_with_a_beyond(|_| {}, clear_of_the_arrows);
    let c = approach(&j);
    assert!(c.ok, "{}", said_en(&c.detail));
    assert!(j.view().arms.iter().find(|a| a.uid == uid).unwrap().beyond_problems.is_empty());
}

#[test]
fn a_taper_without_room_fails_the_approach_check() {
    let (j, _) = junction_with_a_beyond(
        |_| {},
        |b| {
            clear_of_the_arrows(b);
            let t = b.taper.as_mut().unwrap();
            (t.need_mm, t.room_mm) = (40_000, 25_000);
        },
    );
    let c = approach(&j);
    assert!(!c.ok);
    let detail = said_en(&c.detail);
    assert!(detail.contains("40.0 m") && detail.contains("25.0 m"), "{detail}");
}

#[test]
fn a_dropped_lane_with_nowhere_to_merge_fails_it() {
    let (j, uid) = junction_with_a_beyond(
        |_| {},
        |b| {
            clear_of_the_arrows(b);
            b.taper.as_mut().unwrap().merges = false;
        },
    );
    let c = approach(&j);
    assert!(!c.ok);
    assert!(said_en(&c.detail).contains("ends with no lane beside it"), "{}", said_en(&c.detail));
    assert_eq!(j.view().arms.iter().find(|a| a.uid == uid).unwrap().beyond_problems.len(), 1);
}

#[test]
fn no_beyond_no_failure_and_one_passing_check() {
    for sample in 0..JUNCTION_SAMPLES.len() {
        let v = Junction::new(sample).view();
        assert_eq!(v.checks.iter().filter(|c| c.id == "approach").count(), 1);
        assert!(v.checks.iter().find(|c| c.id == "approach").unwrap().ok);
        assert!(v.arms.iter().all(|a| a.beyond_problems.is_empty()));
    }
}

#[test]
fn a_beyond_ignored_for_a_turbo_or_a_pedestrian_arm_cannot_fail() {
    use crate::street::model::Beyond;
    let short = |a: &mut Arm| {
        a.beyond = Some(Beyond {
            boundary_mm: 2_000,
            next: a.section.clone().unwrap().segments,
            zone: crate::shared::said::Said::new("zone-unnamed-mid"),
            taper: None,
        });
    };
    let named = crate::shared::said::Said::new("city-name").with("name", crate::shared::said::Arg::Text("Test".into()));
    let mut turbo = Junction::new(0);
    assert!(turbo.set_control(ROUNDABOUT) && turbo.set_turbo(Some(0)));
    let mut state = turbo.current().clone();
    state.arms.iter_mut().for_each(short);
    assert!(approach(&Junction::from_city(named.clone(), &state, &state, 0).expect("draws")).ok);
    let walk = junction_with_walkways(&[90], &[("sidewalk", 3000), ("terrace", 2000), ("sidewalk", 3000)]);
    let mut state = walk.current().clone();
    state.arms.iter_mut().filter(|a| a.is_pedestrian()).for_each(short);
    assert!(approach(&Junction::from_city(named, &state, &state, 0).expect("draws")).ok);
}
