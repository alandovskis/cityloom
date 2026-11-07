//! What every slice uses: the catalogue and its units, the ports a view-model
//! reaches the browser through (and the browser's side of them), the reactive
//! core a view-model is built on, and the helpers a view binds with.

pub mod atlas;
pub mod bind;
pub mod catalogue;
#[cfg(test)]
pub mod contrast;
pub mod core;
pub mod diagram;
pub mod i18n;
pub mod iso;
pub mod keeper;
pub mod live;
pub mod platform;
pub mod ports;
pub mod provenance;
pub mod random;
pub mod said;
pub mod shortcut;
pub mod store;
pub mod symbols;
#[cfg(test)]
pub mod testing;
pub mod tick;
pub mod units;
