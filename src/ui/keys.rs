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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    Undo,
    Redo,
}

/// What Ctrl or Cmd with a key does anywhere on the page.
pub fn shortcut(key: &str, shift: bool) -> Option<Shortcut> {
    match key.to_lowercase().as_str() {
        "z" if shift => Some(Shortcut::Redo),
        "z" => Some(Shortcut::Undo),
        "y" => Some(Shortcut::Redo),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn control_z_undoes_shift_z_and_y_redo() {
        assert_eq!(shortcut("z", false), Some(Shortcut::Undo));
        assert_eq!(shortcut("Z", false), Some(Shortcut::Undo));
        assert_eq!(shortcut("z", true), Some(Shortcut::Redo));
        assert_eq!(shortcut("y", false), Some(Shortcut::Redo));
        assert_eq!(shortcut("x", false), None);
    }
}
