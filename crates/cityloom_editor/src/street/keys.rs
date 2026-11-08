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

/// Which piece the piece list's key chooses: `current` is the index of the selected piece, `len` the number of pieces.
/// The arrows go one along (from nothing, to the first or the last), Home and End to the ends; the ends do not wrap.
/// `None` for a key that is not the list's, or for no pieces.
pub fn piece_target(key: &str, current: Option<usize>, len: usize) -> Option<usize> {
    let last = len.checked_sub(1)?;
    match key {
        "ArrowRight" | "ArrowDown" => Some(current.map_or(0, |c| (c + 1).min(last))),
        "ArrowLeft" | "ArrowUp" => Some(current.map_or(last, |c| c.saturating_sub(1))),
        "Home" => Some(0),
        "End" => Some(last),
        _ => None,
    }
}

/// Which tab of a tab list of `len` the key pressed on the tab at `current` shows, by the WAI-ARIA tabs pattern: the
/// arrows go to the next or the previous and wrap, Home to the first, End to the last. `None` for any other key.
pub fn panel_tab_target(key: &str, current: usize, len: usize) -> Option<usize> {
    let last = len.checked_sub(1)?;
    match key {
        "ArrowRight" | "ArrowDown" => Some(if current >= last { 0 } else { current + 1 }),
        "ArrowLeft" | "ArrowUp" => Some(if current == 0 { last } else { current - 1 }),
        "Home" => Some(0),
        "End" => Some(last),
        _ => None,
    }
}

/// What a key does on the zone strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneAction {
    /// Select the next (1) or previous (-1) zone.
    Select(i32),
    /// Select the first (`last` false) or the last zone.
    SelectEdge { last: bool },
    /// Move the selected zone's start boundary by this many millimetres.
    Nudge(i32),
}

/// What a key does on the zone strip, with whether the browser's own use of it is to be stopped. Alt with an
/// arrow is the browser's Back and Forward, so it is always stopped; Ctrl or Meta is never ours.
pub fn on_strip(key: &str, shift: bool, alt: bool, ctrl: bool, meta: bool) -> Option<(ZoneAction, bool)> {
    if ctrl || meta {
        return None;
    }
    let d = match key {
        "ArrowLeft" => -1,
        "ArrowRight" => 1,
        "Home" if !alt => return Some((ZoneAction::SelectEdge { last: false }, true)),
        "End" if !alt => return Some((ZoneAction::SelectEdge { last: true }, true)),
        _ => return None,
    };
    Some(if alt { (ZoneAction::Nudge(d * if shift { 10_000 } else { 1_000 }), true) } else { (ZoneAction::Select(d), true) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panel_tabs_follow_the_tabs_pattern() {
        assert_eq!(panel_tab_target("ArrowRight", 0, 2), Some(1));
        assert_eq!(panel_tab_target("ArrowRight", 1, 2), Some(0), "wraps");
        assert_eq!(panel_tab_target("ArrowLeft", 0, 2), Some(1), "wraps");
        assert_eq!(panel_tab_target("ArrowLeft", 1, 2), Some(0));
        assert_eq!(panel_tab_target("Home", 1, 2), Some(0));
        assert_eq!(panel_tab_target("End", 0, 2), Some(1));
        assert_eq!(panel_tab_target("a", 0, 2), None);
        assert_eq!(panel_tab_target("Home", 0, 0), None);
    }

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
    fn arrows_select_zones_home_and_end_go_to_the_ends() {
        assert_eq!(on_strip("ArrowRight", false, false, false, false), Some((ZoneAction::Select(1), true)));
        assert_eq!(on_strip("ArrowLeft", false, false, false, false), Some((ZoneAction::Select(-1), true)));
        assert_eq!(on_strip("Home", false, false, false, false), Some((ZoneAction::SelectEdge { last: false }, true)));
        assert_eq!(on_strip("End", false, false, false, false), Some((ZoneAction::SelectEdge { last: true }, true)));
    }

    #[test]
    fn alt_arrows_nudge_the_start_boundary_by_a_metre_and_with_shift_by_ten_and_stop_the_browsers_back_and_forward() {
        assert_eq!(on_strip("ArrowLeft", false, true, false, false), Some((ZoneAction::Nudge(-1_000), true)));
        assert_eq!(on_strip("ArrowRight", false, true, false, false), Some((ZoneAction::Nudge(1_000), true)));
        assert_eq!(on_strip("ArrowLeft", true, true, false, false), Some((ZoneAction::Nudge(-10_000), true)));
        assert_eq!(on_strip("ArrowRight", true, true, false, false), Some((ZoneAction::Nudge(10_000), true)));
    }

    #[test]
    fn ctrl_or_meta_with_an_arrow_is_not_ours() {
        for key in ["ArrowLeft", "ArrowRight", "Home", "End"] {
            assert_eq!(on_strip(key, false, false, true, false), None);
            assert_eq!(on_strip(key, false, false, false, true), None);
            assert_eq!(on_strip(key, false, true, true, false), None);
        }
    }

    #[test]
    fn other_keys_are_not_ours_on_the_strip() {
        for key in ["+", "v", "Tab", "Delete", "Enter", "Escape"] {
            assert_eq!(on_strip(key, false, false, false, false), None);
        }
    }

    #[test]
    fn alt_home_and_alt_end_are_not_the_strips() {
        assert_eq!(on_strip("Home", false, true, false, false), None);
        assert_eq!(on_strip("End", true, true, false, false), None);
    }

    #[test]
    fn v_is_still_no_key_of_the_strip() {
        assert_eq!(on_strip("v", false, false, false, false), None);
        assert_eq!(on_strip("V", true, false, false, false), None);
    }

    #[test]
    fn other_keys_are_not_ours() {
        assert_eq!(on_section("a", false, true), None);
        assert_eq!(on_section("Tab", false, false), None);
    }

    #[test]
    fn v_is_no_key_of_the_drawing() {
        assert_eq!(on_section("v", false, true), None);
    }

    #[test]
    fn the_piece_list_s_arrows_go_one_along_without_wrapping_and_home_and_end_to_the_ends() {
        assert_eq!(piece_target("ArrowRight", Some(1), 5), Some(2));
        assert_eq!(piece_target("ArrowDown", Some(1), 5), Some(2));
        assert_eq!(piece_target("ArrowLeft", Some(1), 5), Some(0));
        assert_eq!(piece_target("ArrowUp", Some(0), 5), Some(0), "no wrap at the first");
        assert_eq!(piece_target("ArrowRight", Some(4), 5), Some(4), "no wrap at the last");
        assert_eq!(piece_target("ArrowRight", None, 5), Some(0));
        assert_eq!(piece_target("ArrowLeft", None, 5), Some(4));
        assert_eq!(piece_target("Home", Some(3), 5), Some(0));
        assert_eq!(piece_target("End", Some(3), 5), Some(4));
        assert_eq!(piece_target("a", Some(3), 5), None);
        assert_eq!(piece_target("End", None, 0), None, "no pieces");
    }
}
