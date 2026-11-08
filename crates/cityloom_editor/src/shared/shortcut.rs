//! The keys that do the same on every page.

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
    fn control_z_undoes_shift_z_and_y_redo() {
        assert_eq!(shortcut("z", false), Some(Shortcut::Undo));
        assert_eq!(shortcut("Z", false), Some(Shortcut::Undo));
        assert_eq!(shortcut("z", true), Some(Shortcut::Redo));
        assert_eq!(shortcut("y", false), Some(Shortcut::Redo));
        assert_eq!(shortcut("x", false), None);
    }
}
