//! What the keys do on the junction page, as decisions the components carry out.

/// A number a key can step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Bearing,
    Corner,
    Setback,
    Cycle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Select the next (1) or previous (-1) thing round the junction.
    Select(i32),
    /// Step the selected number up or down.
    Step(Step, i32),
    /// Take away the selected part.
    Remove,
    /// Select nothing.
    Deselect,
}

/// What a key does on the plan, with whether the browser's own use of it is
/// to be stopped. `selected` is the kind of the thing selected, if any.
pub fn on_plan(key: &str, shift: bool, selected: Option<&str>) -> Option<(Action, bool)> {
    let step = |dir: i32| {
        let what = match selected? {
            "arm" => Step::Bearing,
            "corner" => Step::Corner,
            "crossing" => Step::Setback,
            "cycle" => Step::Cycle,
            _ => return None,
        };
        Some((Action::Step(what, dir), true))
    };
    match key {
        "ArrowRight" | "ArrowLeft" => {
            let dir = if key == "ArrowRight" { 1 } else { -1 };
            // Shift turns the selected street instead of moving to the next thing.
            if shift && selected == Some("arm") { Some((Action::Step(Step::Bearing, dir), true)) } else { Some((Action::Select(dir), true)) }
        }
        "+" | "=" => step(1),
        "-" | "_" => step(-1),
        "Delete" | "Backspace" => Some((Action::Remove, true)),
        "Escape" => selected.map(|_| (Action::Deselect, false)),
        _ => None,
    }
}

/// A key pressed on the page, as far as switching the view cares.
#[derive(Clone, Copy, Debug)]
pub struct Press<'a> {
    pub key: &'a str,
    pub ctrl: bool,
    pub meta: bool,
    pub alt: bool,
    /// The key is held down and this is a repeat of it.
    pub repeat: bool,
    /// An input method is composing text.
    pub composing: bool,
    /// A field has the key (the person is typing) or another handler has taken it.
    pub taken: bool,
}

/// Whether a key pressed on the page switches the drawing between the plan and the 2.5D: `V`, pressed once,
/// without Ctrl, Cmd or Alt, not while composing, and not taken by a field or by another handler. It is the
/// letter on the person's layout (`key`, not `code`); the toggle's buttons are there for a layout without it.
pub fn switches_view(p: &Press) -> bool {
    !(p.ctrl || p.meta || p.alt) && p.key.eq_ignore_ascii_case("v") && !p.repeat && !p.composing && !p.taken
}

/// Which radio of a group of `count` a key pressed on the radio at `current` chooses, by the WAI-ARIA radio group
/// pattern: Down and Right the next, Up and Left the previous (both wrapping), Home the first, End the last. A key with
/// Ctrl, Cmd or Alt, a repeat, a composing input method or one another handler has taken is not ours. `None` is no choice,
/// which the radio chosen already is not: the view chooses whatever this names and moves the focus to it.
pub fn radio_choice(p: &Press, count: usize, current: usize) -> Option<usize> {
    if count == 0 || p.ctrl || p.meta || p.alt || p.repeat || p.composing || p.taken {
        return None;
    }
    match p.key {
        "ArrowDown" | "ArrowRight" => Some((current + 1) % count),
        "ArrowUp" | "ArrowLeft" => Some((current + count - 1) % count),
        "Home" => Some(0),
        "End" => Some(count - 1),
        _ => None,
    }
}

/// Whether the radio at `index` is the one a radio group stops the Tab key on: the one chosen, or the first when none is.
pub fn radio_tab_stop(chosen: Option<usize>, index: usize) -> bool {
    chosen.unwrap_or(0) == index
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: &str) -> Press<'_> {
        Press { key, ctrl: false, meta: false, alt: false, repeat: false, composing: false, taken: false }
    }

    #[test]
    fn v_switches_the_view_once_and_only_when_nothing_else_has_the_key() {
        assert!(switches_view(&press("v")) && switches_view(&press("V")));
        assert!(!switches_view(&press("b")));
        assert!(!switches_view(&Press { ctrl: true, ..press("v") }));
        assert!(!switches_view(&Press { meta: true, ..press("v") }));
        assert!(!switches_view(&Press { alt: true, ..press("v") }));
        assert!(!switches_view(&Press { repeat: true, ..press("v") }));
        assert!(!switches_view(&Press { composing: true, ..press("v") }));
        assert!(!switches_view(&Press { taken: true, ..press("v") }));
    }

    #[test]
    fn the_arrows_move_the_selection_round_the_junction() {
        assert_eq!(on_plan("ArrowRight", false, None), Some((Action::Select(1), true)));
        assert_eq!(on_plan("ArrowLeft", false, Some("corner")), Some((Action::Select(-1), true)));
        assert_eq!(on_plan("ArrowRight", true, Some("corner")), Some((Action::Select(1), true)));
    }

    #[test]
    fn shift_and_an_arrow_turn_the_selected_street() {
        assert_eq!(on_plan("ArrowRight", true, Some("arm")), Some((Action::Step(Step::Bearing, 1), true)));
        assert_eq!(on_plan("ArrowLeft", true, Some("arm")), Some((Action::Step(Step::Bearing, -1), true)));
    }

    #[test]
    fn plus_and_minus_step_what_is_selected_in_the_way_that_suits_it() {
        for (kind, step) in [("arm", Step::Bearing), ("corner", Step::Corner), ("crossing", Step::Setback), ("cycle", Step::Cycle)] {
            assert_eq!(on_plan("+", false, Some(kind)), Some((Action::Step(step, 1), true)), "{kind}");
            assert_eq!(on_plan("=", false, Some(kind)), Some((Action::Step(step, 1), true)));
            assert_eq!(on_plan("-", false, Some(kind)), Some((Action::Step(step, -1), true)));
            assert_eq!(on_plan("_", false, Some(kind)), Some((Action::Step(step, -1), true)));
        }
    }

    #[test]
    fn plus_and_minus_do_nothing_for_a_lane_a_bus_lane_or_when_nothing_is_selected() {
        for sel in [Some("lane"), Some("bus"), None] {
            assert_eq!(on_plan("+", false, sel), None);
            assert_eq!(on_plan("-", false, sel), None);
        }
    }

    #[test]
    fn delete_and_backspace_remove_and_the_browser_is_kept_from_going_back() {
        assert_eq!(on_plan("Delete", false, Some("arm")), Some((Action::Remove, true)));
        assert_eq!(on_plan("Backspace", false, None), Some((Action::Remove, true)));
    }

    #[test]
    fn escape_clears_a_selection_and_leaves_the_key_alone() {
        assert_eq!(on_plan("Escape", false, Some("arm")), Some((Action::Deselect, false)));
        assert_eq!(on_plan("Escape", false, None), None);
    }

    #[test]
    fn other_keys_are_not_ours() {
        assert_eq!(on_plan("a", false, Some("arm")), None);
        assert_eq!(on_plan("Tab", false, None), None);
    }

    #[test]
    fn the_arrows_choose_the_next_and_previous_radio_and_wrap() {
        assert_eq!(radio_choice(&press("ArrowDown"), 5, 1), Some(2));
        assert_eq!(radio_choice(&press("ArrowRight"), 5, 1), Some(2));
        assert_eq!(radio_choice(&press("ArrowUp"), 5, 1), Some(0));
        assert_eq!(radio_choice(&press("ArrowLeft"), 5, 1), Some(0));
        assert_eq!(radio_choice(&press("ArrowDown"), 5, 4), Some(0), "past the last is the first");
        assert_eq!(radio_choice(&press("ArrowUp"), 5, 0), Some(4), "before the first is the last");
        assert_eq!(radio_choice(&press("Home"), 5, 3), Some(0));
        assert_eq!(radio_choice(&press("End"), 5, 3), Some(4));
    }

    #[test]
    fn other_keys_and_pressed_modifiers_are_not_a_choice_of_a_radio() {
        assert_eq!(radio_choice(&press("a"), 5, 1), None);
        assert_eq!(radio_choice(&press("Tab"), 5, 1), None);
        assert_eq!(radio_choice(&press(" "), 5, 1), None);
        assert_eq!(radio_choice(&press("ArrowDown"), 0, 0), None, "no radios, no choice");
        for held in [
            Press { ctrl: true, ..press("ArrowDown") },
            Press { meta: true, ..press("ArrowDown") },
            Press { alt: true, ..press("ArrowDown") },
            Press { repeat: true, ..press("ArrowDown") },
            Press { composing: true, ..press("ArrowDown") },
            Press { taken: true, ..press("ArrowDown") },
        ] {
            assert_eq!(radio_choice(&held, 5, 1), None, "{held:?}");
        }
    }

    #[test]
    fn a_radio_group_has_one_tab_stop_the_chosen_radio_or_else_the_first() {
        let stops = |chosen| (0..4).filter(|i| radio_tab_stop(chosen, *i)).collect::<Vec<_>>();
        assert_eq!(stops(Some(2)), [2]);
        assert_eq!(stops(Some(0)), [0]);
        assert_eq!(stops(None), [0]);
    }
}
