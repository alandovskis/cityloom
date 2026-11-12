//! The person's projects: named containers, each tied to an area.

pub mod migrate;
pub mod model;
pub mod store;

#[cfg(test)]
mod tests;

pub use migrate::migrate_legacy;
pub use model::{Project, Refused};
pub use store::{DAMAGED_KEY, LAST_KEY, PROJECTS_KEY, ProjectStore, last_changed};
