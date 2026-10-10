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
        assert_eq!(say("city-edge-between", Args::new().str("from", "X").str("to", "Y")), "X to Y");
        assert_eq!(say("city-edge-name", Args::new().str("kind", "K").str("ends", "X to Y")), "K · X to Y");
        assert_eq!(say("city-unnamed", Args::new().str("class", "local")), "Unnamed local street");
        let street = || Args::new().str("street", "Main Street");
        assert_eq!(say("city-end-of", street()), "End of Main Street");
        assert_eq!(say("city-connection-on", street()), "Connection on Main Street");
        assert_eq!(say("city-map-edge", Args::new()), "Edge of the map");
        assert_eq!(say("city-the-end-of", street()), "the end of Main Street");
        assert_eq!(say("city-a-connection-on", street()), "a connection on Main Street");
        assert_eq!(say("city-the-edge-of-the-map", Args::new()), "the edge of the map");
        assert_eq!(say("city-edge-through", Args::new()), "through the city");
    }

    #[test]
    fn the_french_edge_name_reads_with_each_sentence_form() {
        let i = crate::i18n_for(Locale::FrCa);
        let street = || Args::new().str("street", "Main");
        let end = i.tr_now("city-the-end-of", &street());
        let connection = i.tr_now("city-a-connection-on", &street());
        let edge = i.tr_now("city-the-edge-of-the-map", &Args::new());
        let between = |from: &str, to: &str| i.tr_now("city-edge-between", &Args::new().str("from", from).str("to", to));
        assert_eq!(between(&end, &connection), "entre le bout de Main et un raccordement sur Main");
        assert_eq!(between(&edge, &end), "entre la limite de la carte et le bout de Main");
        assert_eq!(between(&connection, &edge), "entre un raccordement sur Main et la limite de la carte");
        let name = i.tr_now("city-edge-name", &Args::new().str("kind", "K").str("ends", between(&end, &edge)));
        assert_eq!(name, "K · entre le bout de Main et la limite de la carte");
    }

    #[test]
    fn the_french_unnamed_street_agrees_with_rue() {
        let i = crate::i18n_for(Locale::FrCa);
        let say = |class: &str| i.tr_now("city-unnamed", &Args::new().str("class", i.tr_now(class, &Args::new())));
        assert_eq!(say("class-local"), "Rue locale sans nom");
        assert_eq!(say("class-collector"), "Rue collectrice sans nom");
        assert_eq!(say("class-arterial"), "Rue artérielle sans nom");
    }

    #[test]
    fn the_french_plural_treats_one_as_singular() {
        let i = crate::i18n_for(Locale::FrCa);
        let many = |n| i.tr_now("city-junction-of-many", &Args::new().str("a", "A").str("b", "B").num("rest", n));
        assert!(many(1).ends_with("1 autre"), "{}", many(1));
        assert!(many(3).ends_with("3 autres"), "{}", many(3));
    }
}
