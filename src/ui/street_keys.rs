//! What the keys do on the street's section, as decisions the components carry out.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Select the next (1) or previous (-1) piece.
    Select(i32),
    /// Move the selected piece one place along.
    Move(i32),
    /// Change the selected piece's width by this many millimetres.
    Nudge(i32),
    /// Go to the width field.
    EditWidth,
    /// Take the selected piece away.
    Remove,
    /// Let go of a drag, or else select nothing.
    Escape,
}

/// What a key does on the section, with whether the browser's own use of it is
/// to be stopped. `selected` is whether a piece is selected.
pub fn on_section(key: &str, shift: bool, selected: bool) -> Option<(Action, bool)> {
    match key {
        "ArrowLeft" | "ArrowRight" => {
            let d = if key == "ArrowLeft" { -1 } else { 1 };
            Some((if shift { Action::Move(d) } else { Action::Select(d) }, true))
        }
        "+" | "=" => Some((Action::Nudge(if shift { 500 } else { 100 }), true)),
        "-" | "_" => Some((Action::Nudge(if shift { -500 } else { -100 }), true)),
        "Enter" => selected.then_some((Action::EditWidth, true)),
        "Delete" | "Backspace" => Some((Action::Remove, true)),
        "Escape" => Some((Action::Escape, false)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arrows_select_and_shift_with_an_arrow_moves_the_piece() {
        assert_eq!(on_section("ArrowRight", false, false), Some((Action::Select(1), true)));
        assert_eq!(on_section("ArrowLeft", false, true), Some((Action::Select(-1), true)));
        assert_eq!(on_section("ArrowRight", true, true), Some((Action::Move(1), true)));
        assert_eq!(on_section("ArrowLeft", true, true), Some((Action::Move(-1), true)));
    }

    #[test]
    fn plus_and_minus_nudge_by_a_tenth_of_a_metre_and_with_shift_by_half() {
        assert_eq!(on_section("+", false, true), Some((Action::Nudge(100), true)));
        assert_eq!(on_section("=", false, true), Some((Action::Nudge(100), true)));
        assert_eq!(on_section("+", true, true), Some((Action::Nudge(500), true)));
        assert_eq!(on_section("-", false, true), Some((Action::Nudge(-100), true)));
        assert_eq!(on_section("_", true, true), Some((Action::Nudge(-500), true)));
    }

    #[test]
    fn enter_goes_to_the_width_only_when_a_piece_is_selected() {
        assert_eq!(on_section("Enter", false, true), Some((Action::EditWidth, true)));
        assert_eq!(on_section("Enter", false, false), None);
    }

    #[test]
    fn delete_removes_and_escape_lets_go_without_stopping_the_browser() {
        assert_eq!(on_section("Delete", false, true), Some((Action::Remove, true)));
        assert_eq!(on_section("Backspace", false, false), Some((Action::Remove, true)));
        assert_eq!(on_section("Escape", false, true), Some((Action::Escape, false)));
    }

    #[test]
    fn other_keys_are_not_ours() {
        assert_eq!(on_section("a", false, true), None);
        assert_eq!(on_section("Tab", false, false), None);
    }
}
