//! What the keys do on the controls, as decisions the components carry out.

/// A key pressed on the page, as far as a radio group cares.
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

/// Which radio of a group of `count` a key pressed on the radio at `current` chooses, by the WAI-ARIA radio group
/// pattern: Down and Right the next, Up and Left the previous (both wrapping), Home the first, End the last. A key with
/// Ctrl, Cmd or Alt, a repeat, a composing input method or one another handler has taken is not ours.
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

/// Which radio a key chooses in a group where none is checked that can be chosen: Down, Right and Home the first, Up,
/// Left and End the last.
pub fn radio_choice_from_none(p: &Press, count: usize) -> Option<usize> {
    match p.key {
        "ArrowDown" | "ArrowRight" => radio_choice(p, count, count.wrapping_sub(1)),
        _ => radio_choice(p, count, 0),
    }
}

/// Whether the radio at `index` is the one a radio group stops the Tab key on: the one chosen, or the first when none is.
pub fn radio_tab_stop(chosen: Option<usize>, index: usize) -> bool {
    chosen.unwrap_or(0) == index
}

/// Where a key pressed on the option `focused` moves the focus and the choice, as an index among all the options shown,
/// of which `enabled` says those that can be chosen. A key held down keeps moving.
pub fn radio_move(p: &Press, enabled: &[bool], focused: usize) -> Option<usize> {
    let press = Press { repeat: false, ..*p };
    let open: Vec<usize> = (0..enabled.len()).filter(|&i| enabled[i]).collect();
    let to = match open.iter().position(|&i| i == focused) {
        Some(at) => radio_choice(&press, open.len(), at),
        None => radio_choice_from_none(&press, open.len()),
    };
    to.map(|to| open[to])
}

/// Whether a press on an option is a choice: one that can be chosen and is not chosen already.
pub fn radio_picks(chosen: bool, enabled: bool) -> bool {
    enabled && !chosen
}

/// The step an arrow key makes in a number field, as a number field would.
pub fn step_key(key: &str) -> Option<i32> {
    match key {
        "ArrowUp" => Some(1),
        "ArrowDown" => Some(-1),
        _ => None,
    }
}

/// How a key ends a number field: Enter keeps what was typed, Escape puts the value back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldEnd {
    Commit,
    Revert,
}

pub fn field_end(key: &str) -> Option<FieldEnd> {
    match key {
        "Enter" => Some(FieldEnd::Commit),
        "Escape" => Some(FieldEnd::Revert),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const THREE: &[bool] = &[true, true, true];

    fn press(key: &str) -> Press<'_> {
        Press { key, ctrl: false, meta: false, alt: false, repeat: false, composing: false, taken: false }
    }

    #[test]
    fn the_arrows_move_from_the_focused_option_and_wrap_and_home_and_end_go_to_the_ends() {
        for next in ["ArrowDown", "ArrowRight"] {
            assert_eq!(radio_move(&press(next), THREE, 0), Some(1), "{next}");
            assert_eq!(radio_move(&press(next), THREE, 2), Some(0), "{next}: past the last is the first");
        }
        for previous in ["ArrowUp", "ArrowLeft"] {
            assert_eq!(radio_move(&press(previous), THREE, 0), Some(2), "{previous}: before the first is the last");
        }
        assert_eq!(radio_move(&press("Home"), THREE, 1), Some(0));
        assert_eq!(radio_move(&press("End"), THREE, 1), Some(2));
        assert_eq!(radio_move(&press("a"), THREE, 1), None);
    }

    #[test]
    fn a_key_held_down_keeps_moving() {
        assert_eq!(radio_move(&Press { repeat: true, ..press("ArrowDown") }, THREE, 0), Some(1));
    }

    #[test]
    fn a_modifier_a_composition_or_a_taken_key_moves_nothing() {
        for held in [
            Press { ctrl: true, ..press("ArrowDown") },
            Press { meta: true, ..press("ArrowDown") },
            Press { alt: true, ..press("ArrowDown") },
            Press { composing: true, ..press("ArrowDown") },
            Press { taken: true, ..press("ArrowDown") },
        ] {
            assert_eq!(radio_move(&held, THREE, 0), None, "{held:?}");
        }
        assert_eq!(radio_move(&press("ArrowDown"), &[], 0), None, "no options, nowhere to move");
    }

    #[test]
    fn from_an_option_that_cannot_be_chosen_the_keys_go_to_the_ends_and_never_to_it() {
        let custom = &[false, true, true, true];
        for (key, to) in [("ArrowDown", 1), ("Home", 1), ("ArrowUp", 3), ("End", 3)] {
            assert_eq!(radio_move(&press(key), custom, 0), Some(to), "{key}");
        }
        assert_eq!(radio_move(&press("ArrowDown"), custom, 3), Some(1));
        assert_eq!(radio_move(&press("ArrowUp"), custom, 1), Some(3));
        for key in ["ArrowDown", "ArrowUp", "Home", "End"] {
            assert_eq!(radio_move(&press(key), &[false, false, false], 0), None, "{key}: none can be chosen");
        }
    }

    #[test]
    fn choosing_the_option_already_chosen_or_one_that_cannot_be_is_no_choice() {
        assert!(radio_picks(false, true));
        for (chosen, enabled) in [(true, true), (false, false), (true, false)] {
            assert!(!radio_picks(chosen, enabled), "chosen {chosen}, enabled {enabled}");
        }
        let stops = |chosen| (0..4).filter(|i| radio_tab_stop(chosen, *i)).collect::<Vec<_>>();
        assert_eq!(stops(Some(2)), [2]);
        assert_eq!(stops(None), [0]);
    }

    #[test]
    fn the_arrow_keys_step_a_number_and_enter_and_escape_end_it() {
        assert_eq!((step_key("ArrowUp"), step_key("ArrowDown"), step_key("Enter")), (Some(1), Some(-1), None));
        assert_eq!(field_end("Enter"), Some(FieldEnd::Commit));
        assert_eq!(field_end("Escape"), Some(FieldEnd::Revert));
        assert_eq!(field_end("ArrowUp"), None);
    }
}
