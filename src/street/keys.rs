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

/// Whether a key pressed on the page (outside a field, which the page checks) switches the drawing between
/// the section and the 2.5D: `V`, without Ctrl, Cmd or Alt.
pub fn is_view_toggle(key: &str, ctrl: bool, meta: bool, alt: bool) -> bool {
    !(ctrl || meta || alt) && key.eq_ignore_ascii_case("v")
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

/// Whether a key pressed on the page switches the view: `V`, pressed once, not while composing, and not
/// taken by a field or by another handler. It is the letter on the person's layout (`key`, not `code`), so a
/// layout without a Latin V does not have it; the toggle's buttons are there for that.
pub fn switches_view(p: &Press) -> bool {
    is_view_toggle(p.key, p.ctrl, p.meta, p.alt) && !p.repeat && !p.composing && !p.taken
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
    fn v_switches_the_view_but_not_with_ctrl_cmd_or_alt_and_is_no_key_of_the_section() {
        assert!(is_view_toggle("v", false, false, false));
        assert!(is_view_toggle("V", false, false, false));
        assert!(!is_view_toggle("v", true, false, false));
        assert!(!is_view_toggle("v", false, true, false));
        assert!(!is_view_toggle("v", false, false, true));
        assert!(!is_view_toggle("b", false, false, false));
        assert_eq!(on_section("v", false, true), None);
    }

    #[test]
    fn v_held_down_composing_or_taken_by_a_field_or_another_handler_does_not_switch_the_view() {
        let v = Press { key: "v", ctrl: false, meta: false, alt: false, repeat: false, composing: false, taken: false };
        assert!(switches_view(&v));
        assert!(!switches_view(&Press { repeat: true, ..v }), "a repeat of a key held down");
        assert!(!switches_view(&Press { composing: true, ..v }), "an input method composing");
        assert!(!switches_view(&Press { taken: true, ..v }), "a field, or a handler that took it");
        assert!(!switches_view(&Press { ctrl: true, ..v }) && !switches_view(&Press { key: "b", ..v }));
    }
}
