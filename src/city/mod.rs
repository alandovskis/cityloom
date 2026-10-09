//! The city the street, junction and map pages share: its network and what the
//! editors keep in it, how it is kept in storage, and how a page that edits one
//! place of it writes back.

pub mod binding;
pub mod import;
pub mod model;
pub mod store;

use crate::shared::i18n::Resources;

/// What the city says (the names it invents), in both languages.
pub const RESOURCES: Resources = Resources { en: include_str!("i18n/en.ftl"), fr: include_str!("i18n/fr.ftl") };

#[cfg(test)]
mod tests {
    use crate::shared::i18n::tests::parity_problems;
    use crate::shared::i18n::{Args, Locale};

    #[test]
    fn the_city_ftl_files_have_the_same_messages_and_variables() {
        assert_eq!(parity_problems(super::RESOURCES.en, super::RESOURCES.fr), Vec::<String>::new());
    }

    #[test]
    fn the_english_names_are_the_ones_the_model_builds_today() {
        let i = crate::i18n_for(Locale::En);
        let say = |key: &str, args: Args| i.tr_now(key, &args);
        assert_eq!(say("city-junction-number", Args::new().num("n", 4)), "Junction 4");
        assert_eq!(say("city-junction-of-one", Args::new().str("a", "Main Street")), "Main Street junction");
        assert_eq!(say("city-junction-of-three", Args::new().str("a", "A").str("b", "B").str("c", "C")), "A, B and C");
        assert_eq!(say("city-junction-of-many", Args::new().str("a", "A").str("b", "B").num("rest", 1)), "A, B and 1 more");
        assert_eq!(say("city-junction-of-many", Args::new().str("a", "A").str("b", "B").num("rest", 3)), "A, B and 3 more");
        assert_eq!(say("city-edge-between", Args::new().str("kind", "K").str("from", "X").str("to", "Y")), "K · X to Y");
        assert_eq!(say("city-unnamed", Args::new().str("class", "local")), "Unnamed local");
    }

    #[test]
    fn the_french_plural_treats_one_as_singular() {
        let i = crate::i18n_for(Locale::FrCa);
        let many = |n| i.tr_now("city-junction-of-many", &Args::new().str("a", "A").str("b", "B").num("rest", n));
        assert!(many(1).ends_with("1 autre"), "{}", many(1));
        assert!(many(3).ends_with("3 autres"), "{}", many(3));
    }
}
