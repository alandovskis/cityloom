//! Project-specific design standards: a profile of local thresholds that the street page's checks read in place of
//! the built-in ones. The model (figures, validation, what is active), the store (the browser's profiles) and the
//! Profiles section live here; the street page reads only `model` and `store`.

pub mod model;
pub mod store;
