//! What the street page's details panel shows: one level at a time (the street, a piece, or one of the things along
//! the street), and the path to it. Pure: it reads the street's view and says nothing in words of its own; the
//! view-model words the path.

use crate::shared::catalogue::KINDS;
use crate::shared::said::{Arg, Said};
use crate::street::model::{CalmingKindExt, GreenKindExt, StopKindExt};
use crate::street::model::{StripCell, View};

/// A level of the details panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// What is set for the street as a whole.
    Street,
    /// The selected piece, of the section of the stretch picked.
    Piece,
    /// The chosen stretch of a curbside strip: a use or a free stretch.
    Curb,
    /// The chosen stretch of a stops row.
    Stop,
    /// The chosen stretch of the raised elements' strip.
    Calming,
    /// The chosen tree or planting.
    Green,
}

impl Level {
    /// Every level, in the order of the panel's markup.
    pub const ALL: [Level; 6] = [Level::Street, Level::Piece, Level::Curb, Level::Stop, Level::Calming, Level::Green];

    /// The word the level's part of the page is named by (`p-<id>`).
    pub fn id(self) -> &'static str {
        match self {
            Level::Street => "street",
            Level::Piece => "piece",
            Level::Curb => "curb",
            Level::Stop => "stop",
            Level::Calming => "calming",
            Level::Green => "green",
        }
    }

    /// The level above it: everything belongs to the street.
    pub fn parent(self) -> Option<Level> {
        match self {
            Level::Street => None,
            _ => Some(Level::Street),
        }
    }
}

/// The message for a free stretch of a strip, on the path.
const GAP_KEY: &str = "st-path-gap";
/// The message that is only its `name`: a kind of piece, or a stop's own name.
const STEP_KEY: &str = "st-path-step";

/// What is chosen at `level` in `v`, as the path names it; `None` when nothing is (the street always is).
fn chosen(level: Level, v: &View) -> Option<Said> {
    match level {
        Level::Street => Some(Said::new("st-path-street")),
        Level::Piece => {
            let uid = v.selected?;
            v.segments.iter().find(|s| s.uid == uid).map(|s| Said::new(STEP_KEY).with("name", Arg::Kind(KINDS[s.kind].id)))
        }
        Level::Curb => {
            let cell = v.curb.sides.iter().find_map(|s| s.cells.get(s.selected?))?;
            Some(if cell.instance().is_some() { cell.label().clone() } else { Said::new(GAP_KEY) })
        }
        Level::Stop => {
            let cell = v.stops.sides.iter().find_map(|s| s.cells.get(s.selected?))?;
            Some(match (&cell.name, cell.kind) {
                (Some(name), Some(_)) => Said::new(STEP_KEY).with("name", Arg::Text(name.clone())),
                (None, Some(kind)) => Said::new(kind.name_key()),
                (_, None) => Said::new(GAP_KEY),
            })
        }
        Level::Calming => {
            let cell = v.calming_strip.cells.get(v.calming_strip.selected?)?;
            Some(cell.kind.map_or_else(|| Said::new(GAP_KEY), |k| Said::new(k.name_key())))
        }
        Level::Green => {
            let cell = v.green.cells.get(v.green.selected?)?;
            Some(Said::new(cell.kind.name_key()))
        }
    }
}

/// The level shown when `asked` is the one the person reached last: that one while something is chosen there, else
/// the nearest level above it that has.
pub fn shown(asked: Level, v: &View) -> Level {
    let mut level = asked;
    while chosen(level, v).is_none() {
        match level.parent() {
            Some(up) => level = up,
            None => break,
        }
    }
    level
}

/// The path to `level`, from the street down: each step's level and what it names.
pub fn path(level: Level, v: &View) -> Vec<(Level, Said)> {
    let mut steps = Vec::new();
    let mut at = Some(level);
    while let Some(l) = at {
        if let Some(said) = chosen(l, v) {
            steps.push((l, said));
        }
        at = l.parent();
    }
    steps.reverse();
    steps
}

/// The piece an edit chose that was not in the section before it (a piece just added: Undo and Redo never choose
/// one), to show. A strip's choice is never read here: a strip keeps its choice while its row comes and goes with the pieces,
/// so a choice that reappears was not made by the edit (the commands that add to a strip ask for their level).
pub fn new_piece(before: &View, after: &View) -> Option<u32> {
    let uid = after.selected?;
    (before.selected != Some(uid) && !before.segments.iter().any(|s| s.uid == uid)).then_some(uid)
}
