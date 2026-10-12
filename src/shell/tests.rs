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
fn the_region_list_is_made_of_the_options_it_is_given() {
    use crate::shell::view::region_options_html;
    use crate::shell::vm::RegionOption;
    let html =
        region_options_html(&[RegionOption { id: "canada", label: "Canada (droite)".into() }, RegionOption { id: "japan", label: "Japon (gauche)".into() }]);
    assert_eq!(html, "<option value=\"canada\">Canada (droite)</option><option value=\"japan\">Japon (gauche)</option>");
}

#[test]
fn the_version_label_says_the_crate_version_in_both_languages() {
    let version = env!("CARGO_PKG_VERSION");
    assert_eq!(crate::shell::view::version_label(&crate::i18n_for(Locale::En)), format!("Version {version}"));
    assert_eq!(crate::shell::view::version_label(&crate::i18n_for(Locale::FrCa)), format!("Version {version}"));
}

#[test]
fn every_app_page_shows_the_version_in_its_footer() {
    for (name, html) in [
        ("index", include_str!("../../web/index.html")),
        ("map", include_str!("../../web/map.html")),
        ("street", include_str!("../../web/street.html")),
        ("intersection", include_str!("../../web/intersection.html")),
    ] {
        assert_eq!(html.matches("data-version").count(), 1, "{name}.html");
        let footer = html.split("<footer").nth(1).unwrap_or_else(|| panic!("{name}.html has no footer")).split("</footer>").next().unwrap();
        assert!(footer.contains("data-version"), "{name}.html: the version is not in the footer");
    }
}

#[test]
fn the_share_menu_is_a_button_and_a_hidden_menu_holding_print_in_the_language_of_the_page() {
    use crate::shared::testing::html;
    use crate::shell::share_view::ShareMenu;
    use leptos::prelude::*;

    for (locale, print) in [(Locale::En, "Print this sheet"), (Locale::FrCa, "Imprimer cette fiche")] {
        let i18n = crate::i18n_for(locale);
        let page = html(move || view! { <ShareMenu i18n=i18n/> }.into_any());
        assert!(page.contains(r#"id="share-btn""#) && page.contains(r#"aria-controls="share-menu""#), "{page}");
        assert!(page.contains(r#"id="share-btn-label""#) && page.contains(r#"id="share-heading""#), "{page}");
        let menu = &page[page.find(r#"id="share-menu""#).expect("the menu")..];
        assert!(menu[..menu.find('>').unwrap()].contains("hidden"), "the menu starts hidden: {page}");
        let print_at = menu.find(r#"id="print""#).expect("Print is in the menu");
        assert!(menu[print_at..].contains(print), "{locale:?} says {print}: {page}");
    }
}
