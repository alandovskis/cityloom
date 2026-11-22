//! The junction page's views: the Leptos components, and the pure renderers that draw the junction as SVG.

pub mod compare;
pub mod drawing;
pub mod figure;
pub mod frame;
pub mod inspector;
pub mod iso;
pub mod iso_marks;
pub mod notes;
pub mod page;
pub mod phase_svg;
#[cfg(test)]
pub(in crate::junction) mod tests;
pub mod turns;
