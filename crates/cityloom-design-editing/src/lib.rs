//! `cityloom-design-editing` — the proposed change and the machine that
//! applies it (Unit U5, "Streetmix at city scale").
//!
//! This crate owns three components (`components.md`): [`design_overlay`]
//! (`DesignOverlay` — proposed lane edits, additions, removals, and revert
//! semantics), [`editing_session`] (`EditingSession` — selection and undo
//! state machine), and [`corridor_planner`] (`CorridorPlanner` —
//! connectivity, fit assessment, correspondence, bulk apply).
//!
//! This crate has **no I/O of any kind** — every operation is pure,
//! in-memory domain logic (BR1.1). No method in its public API is `async`
//! or returns a `Future`; nothing here ever crosses an `.await` point.
//! It depends only on `street-core` (the imported baseline) and
//! `cityloom-street-import`'s `Correction`/`CorrectionOverlay` domain
//! types (BR3.1's revert-through-corrections resolution) — never that
//! crate's fetch/transport/adapter machinery.

#![forbid(unsafe_code)]

pub mod corridor_planner;
pub mod design_overlay;
pub mod editing_session;
pub mod entities;

pub use corridor_planner::CorridorPlanner;
pub use design_overlay::DesignOverlay;
pub use editing_session::{EditingSession, MAX_UNDO_ENTRIES};
pub use entities::{
    Anchor, CorrespondenceResult, CorridorApplyOutcome, CorridorSelection, Design, EditFinding,
    EditOutcome, EditSession, FitState, FitStatus, LaneAttribute, LaneEdit, LaneEditKind,
    UndoEntry, UndoRecord,
};
