//! The shell's words and the markup that carries them.

use crate::shared::i18n::tests::parity_problems;
use crate::shared::i18n::{Args, Locale};
use crate::shell::RESOURCES;
use crate::shell::view::i18n_keys;

#[test]
fn i18n_keys_reads_the_text_and_the_attributes_of_a_tag() {
    let html = r#"<p><button data-i18n="a" data-i18n-title="b" title="T">X</button></p><span data-i18n-aria-label="c" aria-label="L">Y</span>"#;
    assert_eq!(i18n_keys(html), vec![("a".to_string(), "X".to_string()), ("b".to_string(), "T".to_string()), ("c".to_string(), "L".to_string())]);
}

#[test]
fn every_data_i18n_key_in_street_html_exists_in_both_languages() {
    let html = include_str!("../../web/street.html");
    let i = crate::i18n_for(Locale::En);
    let f = crate::i18n_for(Locale::FrCa);
    let keys = i18n_keys(html);
    assert!(keys.len() > 40, "the page should have been converted: {}", keys.len());
    for (key, _) in keys {
        i.tr_now(&key, &Args::new());
        f.tr_now(&key, &Args::new());
    }
    assert!(i.missing().is_empty(), "missing in en: {:?}", i.missing());
    assert!(f.missing().is_empty(), "missing in fr: {:?}", f.missing());
}

#[test]
fn the_english_text_in_street_html_is_the_english_message() {
    let html = include_str!("../../web/street.html");
    let i = crate::i18n_for(Locale::En);
    for (key, fallback) in i18n_keys(html) {
        assert_eq!(fallback, i.tr_now(&key, &Args::new()), "{key}");
    }
}

#[test]
fn the_shell_ftl_files_have_the_same_messages_and_variables() {
    assert_eq!(parity_problems(RESOURCES.en, RESOURCES.fr), Vec::<String>::new());
}

#[test]
fn street_html_has_a_language_row_and_sets_the_language_in_the_head() {
    let html = include_str!("../../web/street.html");
    assert!(html.contains(r#"data-lang="en" lang="en" aria-pressed="true">English<"#));
    assert!(html.contains(r#"data-lang="fr-CA" lang="fr" aria-pressed="false">Français<"#));
    assert_eq!(html.matches("<script>").count(), 1);
    assert!(html.contains(r#"localStorage.getItem("cityloom-lang")"#));
}

#[test]
fn the_street_ftl_files_have_the_same_messages_and_variables() {
    assert_eq!(parity_problems(crate::street::RESOURCES.en, crate::street::RESOURCES.fr), Vec::<String>::new());
}

#[test]
fn the_region_list_is_made_of_the_options_it_is_given() {
    use crate::shell::view::region_options_html;
    use crate::shell::vm::RegionOption;
    let html =
        region_options_html(&[RegionOption { id: "canada", label: "Canada (droite)".into() }, RegionOption { id: "japan", label: "Japon (gauche)".into() }]);
    assert_eq!(html, "<option value=\"canada\">Canada (droite)</option><option value=\"japan\">Japon (gauche)</option>");
}
