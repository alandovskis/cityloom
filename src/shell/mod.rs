//! What every page shares: the settings menu (units, region, theme), the details
//! and notes sidebars and the notes tabs. The view-model decides; the view binds
//! the markup each page keeps for them.

pub mod view;
pub mod vm;

pub use view::mount;
pub use vm::Target;
