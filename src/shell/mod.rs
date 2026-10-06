//! What every page shares: the settings menu (units, region, theme), the details
//! and notes sidebars and the notes tabs. The view-model decides; the view binds
//! the markup each page keeps for them.

pub mod view;
pub mod vm;

use std::rc::Rc;

use wasm_bindgen::prelude::*;

pub use view::mount;
pub use vm::{Bare, Target};

/// Binds the shell of a page that has nothing of its own to bind: a junction that cannot be drawn still has
/// its settings menu and sidebars.
#[wasm_bindgen]
pub fn mount_bare_shell() {
    mount(Rc::new(Bare::default()), "details", true);
}
