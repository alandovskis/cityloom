//! What the Profiles section says in words: it only hands the model's `Said` data to the page's language.

use crate::shared::i18n::{Args, I18n};
use crate::shared::said::Said;
use crate::shared::units::Units;
use crate::standards::model::Field;

/// What the model said, in words, for a view (see `shared::said`).
pub fn say(i18n: &I18n, units: Units, said: &Said) -> String {
    crate::shared::said::say(i18n, units, said)
}

/// The same, for a command: nothing is watched.
pub fn say_now(i18n: &I18n, units: Units, said: &Said) -> String {
    crate::shared::said::say_now(i18n, units, said)
}

/// The label of a length field: the figure's name and the unit it is typed in.
pub fn length_label(i18n: &I18n, units: Units, field: Field) -> String {
    let name = i18n.tr(field.key(), &Args::new());
    i18n.tr("standards-length-label", &Args::new().str("field", name).str("unit", units.word()))
}
