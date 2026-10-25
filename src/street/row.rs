//! The two rows of cells that lie along the street under the zone strip, the curbside uses and the transit stops, and
//! what they share: a strip of cells per side, one chosen cell, a drag of the thing in it. The view-model, the watch and
//! the strip component are written once against `Row`; each row says only what differs.

use super::model::{CurbCell, CurbRefusal, Editor, StopCell, StripCell, StripView, Subject, View};
use crate::shared::catalogue::Side;

pub trait Row: 'static {
    type Cell: StripCell + 'static;
    /// What an edit of the row's things works on, which words its refusals.
    const SUBJECT: Subject;
    /// The word that starts the ids of the row's strips (`curb-strip-left`) and cells (`curb-left-0`).
    const ID: &'static str;
    /// The message that names a cell for a screen reader.
    const CELL_KEY: &'static str;
    const CAPTION_KEY: &'static str;
    const STRIP_LABEL_KEY: &'static str;
    /// The messages said when a command needs a gap, or a thing, chosen first.
    const CHOOSE_GAP_KEY: &'static str;
    const CHOOSE_THING_KEY: &'static str;

    fn strips(v: &View) -> &StripView<Self::Cell>;
    fn select(e: &mut Editor, side: Side, at_mm: i32);
    fn begin_drag(e: &mut Editor) -> bool;
    /// Moves thing `i` to start at `from_mm`.
    fn move_to(e: &mut Editor, i: usize, from_mm: i32) -> Result<(), CurbRefusal>;
}

pub struct CurbRow;

impl Row for CurbRow {
    type Cell = CurbCell;
    const SUBJECT: Subject = Subject::Use;
    const ID: &'static str = "curb";
    const CELL_KEY: &'static str = "curb-cell";
    const CAPTION_KEY: &'static str = "st-curb-caption";
    const STRIP_LABEL_KEY: &'static str = "st-curb-strip-label";
    const CHOOSE_GAP_KEY: &'static str = "curb-choose-gap";
    const CHOOSE_THING_KEY: &'static str = "curb-choose-use";

    fn strips(v: &View) -> &StripView<CurbCell> {
        &v.curb
    }
    fn select(e: &mut Editor, side: Side, at_mm: i32) {
        e.select_curb(side, at_mm);
    }
    fn begin_drag(e: &mut Editor) -> bool {
        e.begin_curb_drag()
    }
    fn move_to(e: &mut Editor, i: usize, from_mm: i32) -> Result<(), CurbRefusal> {
        e.move_curb(i, from_mm)
    }
}

pub struct StopRow;

impl Row for StopRow {
    type Cell = StopCell;
    const SUBJECT: Subject = Subject::Stop;
    const ID: &'static str = "stop";
    const CELL_KEY: &'static str = "stop-cell";
    const CAPTION_KEY: &'static str = "st-stop-caption";
    const STRIP_LABEL_KEY: &'static str = "st-stop-strip-label";
    const CHOOSE_GAP_KEY: &'static str = "stop-choose-gap";
    const CHOOSE_THING_KEY: &'static str = "stop-choose-stop";

    fn strips(v: &View) -> &StripView<StopCell> {
        &v.stops
    }
    fn select(e: &mut Editor, side: Side, at_mm: i32) {
        e.select_stop(side, at_mm);
    }
    fn begin_drag(e: &mut Editor) -> bool {
        e.begin_stop_drag()
    }
    fn move_to(e: &mut Editor, i: usize, from_mm: i32) -> Result<(), CurbRefusal> {
        e.move_stop(i, from_mm)
    }
}

/// The part of a strip's and a cell's id that names a side (not the word a screen reader is told: that is `curb::side_word`).
pub fn side_slug(side: Side) -> &'static str {
    match side {
        Side::Left => "left",
        Side::Right => "right",
    }
}

pub fn strip_id<R: Row>(side: Side) -> String {
    format!("{}-strip-{}", R::ID, side_slug(side))
}

pub fn cell_id<R: Row>(side: Side, index: usize) -> String {
    format!("{}-{}-{index}", R::ID, side_slug(side))
}
