//! The controls the editor's pages share: a group of radio buttons, a stepper, a section of the details, a question
//! asked before something is done. Each word is a signal of the caller's: the crate holds no wording, and depends on no
//! other crate of the workspace.

pub mod keys;

mod confirm;
mod icons;
mod radio;
mod section;
mod stepper;

pub use {confirm::*, icons::*, radio::*, section::*, stepper::*};

#[cfg(test)]
mod testing;
