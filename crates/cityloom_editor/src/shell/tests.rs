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
    let html = include_str!("../../../../web/street.html");
    let i = crate::i18n_for(Locale::En);
    let f = crate::i18n_for(Locale::FrCa);
    let keys = i18n_keys(html);
    // The notes' words are the tray's (`street::view::notes_tray`), not the page's.
    assert!(keys.len() > 30, "the page should have been converted: {}", keys.len());
    for (key, _) in keys {
        i.tr_now(&key, &Args::new());
        f.tr_now(&key, &Args::new());
    }
    assert!(i.missing().is_empty(), "missing in en: {:?}", i.missing());
    assert!(f.missing().is_empty(), "missing in fr: {:?}", f.missing());
}

#[test]
fn the_english_text_in_street_html_is_the_english_message() {
    let html = include_str!("../../../../web/street.html");
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
    let html = include_str!("../../../../web/street.html");
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
        ("index", include_str!("../../../../web/index.html")),
        ("about", include_str!("../../../../web/about.html")),
        ("map", include_str!("../../../../web/map.html")),
        ("street", include_str!("../../../../web/street.html")),
        ("intersection", include_str!("../../../../web/intersection.html")),
    ] {
        assert_eq!(html.matches("data-version").count(), 1, "{name}.html");
        let footer = html.split("<footer").nth(1).unwrap_or_else(|| panic!("{name}.html has no footer")).split("</footer>").next().unwrap();
        assert!(footer.contains("data-version"), "{name}.html: the version is not in the footer");
    }
}

#[test]
fn the_share_menu_is_a_button_and_a_hidden_menu_holding_print_and_export_in_the_language_of_the_page() {
    use crate::shared::testing::html;
    use crate::shell::share_view::ShareMenu;
    use leptos::prelude::*;

    for (locale, print, export) in [(Locale::En, "Print this sheet", "Export design"), (Locale::FrCa, "Imprimer cette fiche", "Exporter la conception")] {
        let i18n = crate::i18n_for(locale);
        let page = html(move || view! { <ShareMenu i18n=i18n report=false/> }.into_any());
        assert!(page.contains(r#"id="share-btn""#) && page.contains(r#"aria-controls="share-menu""#), "{page}");
        assert!(page.contains(r#"id="share-btn-label""#) && page.contains(r#"id="share-heading""#), "{page}");
        let menu = &page[page.find(r#"id="share-menu""#).expect("the menu")..];
        assert!(menu[..menu.find('>').unwrap()].contains("hidden"), "the menu starts hidden: {page}");
        let print_at = menu.find(r#"id="print""#).expect("Print is in the menu");
        assert!(menu[print_at..].contains(print), "{locale:?} says {print}: {page}");
        let export_at = menu.find(r#"id="share-export""#).expect("Export design is in the menu");
        assert!(menu[export_at..].contains(export), "{locale:?} says {export}: {page}");
    }
}

#[test]
fn the_report_share_menu_has_the_download_item_and_no_print_item() {
    use crate::shared::testing::html;
    use crate::shell::share_view::ShareMenu;
    use leptos::prelude::*;

    for (locale, said) in [(Locale::En, "Download report (PDF)"), (Locale::FrCa, "Télécharger le rapport (PDF)")] {
        let i18n = crate::i18n_for(locale);
        let page = html(move || view! { <ShareMenu i18n=i18n report=true/> }.into_any());
        assert!(page.contains(r#"id="share-btn""#) && page.contains(r#"id="share-menu""#), "{page}");
        assert!(!page.contains(r#"id="print""#), "no Print item: {page}");
        let at = page.find(r#"id="report""#).expect("the report item");
        assert!(page[at..].contains(said), "{locale:?} says {said}: {page}");
    }
}

#[test]
fn the_editors_share_menu_is_unchanged() {
    use crate::shared::testing::html;
    use crate::shell::share_view::ShareMenu;
    use leptos::prelude::*;

    let i18n = crate::i18n_for(Locale::En);
    let page = html(move || view! { <ShareMenu i18n=i18n report=false/> }.into_any());
    assert!(page.contains(r#"id="print""#) && !page.contains(r#"id="report""#), "{page}");
    // The items are Print and Export design (#74), after the heading; the report variant swaps only Print.
    assert!(page.contains(r#"id="share-export""#), "{page}");
    assert_eq!(page.matches(r#"type="button""#).count(), 3, "the Share button, Print and Export design: {page}");
}

#[test]
fn the_share_variant_follows_the_slots_data_share() {
    use crate::shell::share_view::share_has_report;
    assert!(share_has_report(Some("report")));
    assert!(!share_has_report(Some("print")));
    assert!(!share_has_report(Some("")));
    assert!(!share_has_report(None));
}

#[test]
fn every_app_page_has_one_slot_for_the_sign_in_menu() {
    for (name, html) in [
        ("index", include_str!("../../../../web/index.html")),
        ("about", include_str!("../../../../web/about.html")),
        ("map", include_str!("../../../../web/map.html")),
        ("street", include_str!("../../../../web/street.html")),
        ("intersection", include_str!("../../../../web/intersection.html")),
    ] {
        assert_eq!(html.matches(r#"id="session-slot""#).count(), 1, "{name}.html");
    }
}

#[test]
fn the_sign_in_menu_is_drawn_for_each_state_in_each_language() {
    use crate::shared::ports::{AccountError, AccountReply, LinkedIdentity, Me, ProviderInfo, test_ports_full};
    use crate::shared::testing::html;
    use crate::shell::session::SessionVm;
    use crate::shell::session_view::SessionMenu;
    use leptos::prelude::*;

    let render = |locale: Locale, reply: Vec<AccountReply>| {
        let t = test_ports_full();
        let vm = SessionVm::new(t.ports.clone(), crate::i18n_for(locale));
        vm.start();
        for r in reply {
            t.account.answer(r);
        }
        html(move || view! { <SessionMenu vm=vm/> }.into_any())
    };
    let providers = AccountReply::Providers(Ok(vec![
        ProviderInfo { slug: "google".into(), name: "Google".into() },
        ProviderInfo { slug: "osm".into(), name: "OpenStreetMap".into() },
    ]));
    let out = render(Locale::En, vec![AccountReply::Me(Err(AccountError::SignedOut)), providers]);
    assert!(out.contains(r#"id="session-btn""#) && out.contains(">Sign in<"), "{out}");
    assert!(
        out.contains(r#"data-provider="google""#) && out.contains(">Google<") && out.contains(r#"data-provider="osm""#) && out.contains(">OpenStreetMap<"),
        "{out}"
    );
    assert!(out.contains(">Sign in with<"), "{out}");
    let menu = &out[out.find(r#"id="session-menu""#).unwrap()..];
    assert!(menu[..menu.find('>').unwrap()].contains("hidden"), "the menu starts closed");

    let me = Me {
        id: "u".into(),
        display_name: "Ada <b>".into(),
        store_on_server: None,
        identities: vec![LinkedIdentity { id: "a".into(), provider: "google".into(), label: None, created_at: String::new() }],
    };
    let out = render(Locale::En, vec![AccountReply::Me(Ok(me))]);
    assert!(out.contains("Ada &lt;b&gt;") && !out.contains("Ada <b>"), "the name is escaped: {out}");
    assert!(out.contains(r#"href="account.html""#) && out.contains(">Account<") && out.contains(">Sign out<"), "{out}");
    assert!(!out.contains("data-provider"), "{out}");

    let out = render(Locale::En, vec![AccountReply::Me(Err(AccountError::Unavailable))]);
    assert!(out.contains("disabled") && out.contains("Sign-in is not available right now."), "{out}");

    let out = render(Locale::En, vec![]);
    let at = out.find("session-box").unwrap();
    let box_tag = &out[out[..at].rfind('<').unwrap()..];
    assert!(box_tag[..box_tag.find('>').unwrap()].contains("pending"), "keeps its place until known: {out}");
    assert!(!box_tag[..box_tag.find('>').unwrap()].contains("hidden"), "not display:none: {out}");

    let out = render(
        Locale::FrCa,
        vec![AccountReply::Me(Err(AccountError::SignedOut)), AccountReply::Providers(Ok(vec![ProviderInfo { slug: "google".into(), name: "Google".into() }]))],
    );
    assert!(out.contains(">Se connecter<") && out.contains(">Se connecter avec<"), "{out}");
}

#[test]
fn the_editors_open_with_a_skip_link_to_the_drawing_that_shows_on_focus() {
    for (page, html) in [("street", include_str!("../../../../web/street.html")), ("intersection", include_str!("../../../../web/intersection.html"))] {
        // the body's tag may carry a class (`tray-page`)
        let tag = html.find("<body").unwrap();
        let body = &html[tag + html[tag..].find('>').unwrap() + 1..];
        assert!(
            body.trim_start().starts_with(r##"<a class="skip-link" href="#wrap" data-i18n="ui-skip-drawing">Skip to the drawing</a>"##),
            "{page}: the first thing in the body"
        );
    }
    let css = include_str!("../../../../web/app.css");
    assert!(
        css.contains(".skip-link {\n  position: fixed;") && css.contains(".skip-link:focus {\n  transform: none;\n}"),
        "off screen, then on screen when focused"
    );
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    assert_eq!(en.tr_now("ui-skip-drawing", &Args::new()), "Skip to the drawing");
    assert_eq!(fr.tr_now("ui-skip-drawing", &Args::new()), "Passer au dessin");
}

#[test]
fn the_details_panel_of_each_editor_opens_with_a_heading_its_sections_sit_under() {
    // The inspector's sections are level 3; without a level 2 above them the outline skips a level.
    for (page, html, key) in [
        ("street", include_str!("../../../../web/street.html"), "st-details"),
        ("intersection", include_str!("../../../../web/intersection.html"), "jn-page-details-panel"),
    ] {
        let aside = &html[html.find("id=\"details-panel\"").unwrap()..];
        let first = aside[aside.find('>').unwrap() + 1..].trim_start();
        assert!(first.starts_with(&format!("<h2 class=\"sr-only\" data-i18n=\"{key}\">")), "{page}: {}", &first[..80]);
    }
}

#[test]
fn the_street_page_s_details_come_after_the_drawing_in_reading_and_focus_order() {
    // They are drawn on the right of the drawing: a keyboard or a screen reader meets them in the same order.
    let html = include_str!("../../../../web/street.html");
    let at = |needle: &str| html.find(needle).unwrap_or_else(|| panic!("{needle}"));
    assert!(at("id=\"scroll\"") < at("id=\"inspector-toggle\""), "the details' toggle comes after the drawing");
    assert!(at("id=\"inspector-toggle\"") < at("id=\"details-panel\""), "the toggle comes just before the panel it opens");
    assert!(at("</main>") < at("id=\"details-panel\""), "the details panel comes after the drawing's column");
}

/// The pages that run the module say, in their own markup, that it could not start: the module holds every other
/// message, and it is what did not load. `web/boot.js` shows the block of the page's language.
#[test]
fn every_page_that_loads_the_module_has_its_start_failure_in_both_languages() {
    let pages = [
        ("index", include_str!("../../../../web/index.html")),
        ("about", include_str!("../../../../web/about.html")),
        ("map", include_str!("../../../../web/map.html")),
        ("street", include_str!("../../../../web/street.html")),
        ("intersection", include_str!("../../../../web/intersection.html")),
        ("account", include_str!("../../../../web/account.html")),
        ("compare", include_str!("../../../../web/compare.html")),
    ];
    let block_of = |html: &str| {
        let start = html.find(r#"<div id="boot-failed""#).expect("the page has a boot-failed block");
        html[start..start + html[start..].find("\n  </div>\n").expect("the block ends") + "\n  </div>\n".len()].to_string()
    };
    let first = block_of(pages[0].1);
    for (name, html) in pages {
        let block = block_of(html);
        assert_eq!(block, first, "{name}.html says it differently from the other pages");
        assert!(block.contains(r#"id="boot-failed" role="alert" hidden"#), "{name}.html: hidden until the script shows it, and then announced");
        for (lang, locale) in [("en", Locale::En), ("fr-CA", Locale::FrCa)] {
            let part = &block[block.find(&format!(r#"<div lang="{lang}" hidden>"#)).unwrap_or_else(|| panic!("{name}.html has no {lang} block"))..];
            let part = &part[..part.find("</div>").unwrap()];
            let i18n = crate::i18n_for(locale);
            // `data-boot` is read as `data-i18n` is, but the shell does not fill it in: each block keeps its own language.
            let texts = i18n_keys(&part.replace("data-boot=", "data-i18n="));
            for (key, shown) in &texts {
                assert_eq!(shown, &i18n.tr_now(key, &Args::new()), "{name}.html, {lang}, {key}");
            }
            assert_eq!(texts.len(), 2, "{name}.html, {lang}: the sentence and the way to reload");
        }
        // The entry script is itself a versioned address, which a deploy can retire under an open page: its failure to
        // load is told by the one script that is not versioned, `boot.js`, loaded before it.
        let boot = html.find(r#"<script src="boot.js"></script>"#).unwrap_or_else(|| panic!("{name}.html loads boot.js"));
        let entry = html.find(r#"<script type="module" src="v/{{build}}/"#).unwrap_or_else(|| panic!("{name}.html has its entry script"));
        assert!(boot < entry, "{name}.html: boot.js is loaded before the entry script");
        assert!(
            html[entry..].lines().next().unwrap().contains(r#" onerror="window.cityloomBootFailed&&cityloomBootFailed()"></script>"#),
            "{name}.html: the entry script tells a failed load"
        );
        assert!(html.contains(r#"<link rel="modulepreload" href="v/{{build}}/pkg/cityloom_editor.js">"#), "{name}.html preloads the glue");
        assert!(
            html.contains(r#"<link rel="preload" as="fetch" crossorigin href="v/{{build}}/pkg/cityloom_editor_bg.wasm">"#),
            "{name}.html preloads the module"
        );
    }
}

/// The top bar of every page: the logo goes to the list of projects, and the bar's links are Projects and About, the
/// page's own marked as current. A map is opened from its project, never from the bar.
#[test]
fn every_page_s_bar_links_the_projects_and_about_and_never_a_map() {
    let pages = [
        ("index", include_str!("../../../../web/index.html"), Some("index.html")),
        ("about", include_str!("../../../../web/about.html"), Some("about.html")),
        ("map", include_str!("../../../../web/map.html"), None),
        ("street", include_str!("../../../../web/street.html"), None),
        ("intersection", include_str!("../../../../web/intersection.html"), None),
        ("account", include_str!("../../../../web/account.html"), None),
        ("credits", include_str!("../../../../web/credits.html"), None),
        ("compare", include_str!("../../../../web/compare.html"), None),
    ];
    let link = |href: &str, key: &str, text: &str, current: bool| {
        let current = if current { r#" aria-current="page""# } else { "" };
        format!(r#"<a href="{href}" class="surface"{current} data-i18n="{key}">{text}</a>"#)
    };
    for (name, html, current) in pages {
        let bar = &html[html.find("<header class=\"bar\"").unwrap_or_else(|| panic!("{name}.html has no bar"))..];
        let bar = &bar[..bar.find("</header>").unwrap()];
        let brand = &bar[bar.find(r#"<a class="brand""#).expect("the logo")..];
        assert!(brand.starts_with(r#"<a class="brand" href="index.html" "#), "{name}.html: the logo goes to the list of projects");
        let nav = &bar[bar.find("<nav class=\"surfaces\"").expect("the bar's links")..];
        let nav = &nav[..nav.find("</nav>").unwrap()];
        let (projects, about) = (current == Some("index.html"), current == Some("about.html"));
        if name == "credits" {
            // The credits page is in English only: the same links, without the shell's keys.
            assert!(nav.contains(r#"aria-label="Pages""#), "{name}.html: {nav}");
            assert!(nav.contains(r#"<a href="index.html" class="surface">Projects</a>"#), "{name}.html: {nav}");
            assert!(nav.contains(r#"<a href="about.html" class="surface">About</a>"#), "{name}.html: {nav}");
        } else {
            assert!(nav.contains(r#"data-i18n-aria-label="ui-nav" aria-label="Pages""#), "{name}.html: {nav}");
            assert!(nav.contains(&link("index.html", "ui-nav-projects", "Projects", projects)), "{name}.html: {nav}");
            assert!(nav.contains(&link("about.html", "ui-nav-about", "About", about)), "{name}.html: {nav}");
        }
        assert_eq!(nav.matches("<a ").count(), 2, "{name}.html: Projects and About, nothing else: {nav}");
        assert_eq!(nav.matches("aria-current").count(), usize::from(current.is_some()), "{name}.html: {nav}");
        assert!(!bar.contains("map.html"), "{name}.html: no way to a map from the bar");
    }
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    let words = |i18n: &crate::shared::i18n::I18n| ["ui-nav", "ui-nav-projects", "ui-nav-about"].map(|k| i18n.tr_now(k, &Args::new()));
    assert_eq!(words(&en), ["Pages", "Projects", "About"]);
    assert_eq!(words(&fr), ["Pages", "Projets", "À propos"]);
}

/// The comparison page (`web/compare.html`): the skip link first in the body, to the comparison's heading, the slot both
/// halves mount into, the live region, the stylesheets in the order that lets `compare.css` win, and every word of the
/// markup in both languages.
#[test]
fn the_comparison_page_has_its_skip_link_slot_live_region_and_words_in_both_languages() {
    let html = include_str!("../../../../web/compare.html");
    let body = &html[html.find("<body").unwrap()..];
    let body = &body[body.find('>').unwrap() + 1..];
    let skip = r##"<a class="skip-link" href="#compare-h" data-i18n="cmp-skip">Skip to the comparison</a>"##;
    assert!(body.contains(skip), "the skip link goes to the comparison's heading");
    assert!(body.find(skip).unwrap() < body.find("<header").unwrap(), "the skip link comes before the bar");
    assert!(html.contains(r#"<main class="compare-page">"#) && html.contains(r#"<div id="compare-slot"></div>"#));
    assert!(html.contains(r#"<output id="live" class="sr-only" aria-live="polite"></output>"#));
    assert!(html.contains("<title>CityLoom comparison</title>"));
    let at = |sheet: &str| html.find(&format!(r#"href="v/{{{{build}}}}/{sheet}""#)).unwrap_or_else(|| panic!("{sheet}"));
    assert!(at("app.css") < at("editor.css") && at("editor.css") < at("compare.css"), "app, then the drawing's, then the page's own");
    assert!(html.contains(r#"src="v/{{build}}/compare.js""#));
    assert!(html.contains(r#"<svg id="defs""#), "the hatches the drawings are filled with are defined in the page");
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    for (key, shown) in i18n_keys(html) {
        let (e, f) = (en.tr_now(&key, &Args::new()), fr.tr_now(&key, &Args::new()));
        assert!(e != key && f != key, "{key} is a message in both languages");
        if key == "cmp-skip" {
            assert_eq!((shown.as_str(), e.as_str(), f.as_str()), ("Skip to the comparison", "Skip to the comparison", "Passer à la comparaison"));
        }
    }
}

#[test]
fn the_pages_that_read_the_city_preload_the_tile_index() {
    for (name, html) in [
        ("about", include_str!("../../../../web/about.html")),
        ("map", include_str!("../../../../web/map.html")),
        ("street", include_str!("../../../../web/street.html")),
        ("intersection", include_str!("../../../../web/intersection.html")),
        ("compare", include_str!("../../../../web/compare.html")),
    ] {
        assert!(html.contains(r#"<link rel="preload" as="fetch" crossorigin href="data/metro/index.json">"#), "{name}.html");
    }
}

#[test]
fn the_account_menu_says_whether_the_work_is_synced_in_both_languages() {
    use crate::shared::ports::{AccountReply, Me, WorkPage, WorkReply, test_ports_full};
    use crate::shared::testing::html;
    use crate::shell::session::SessionVm;
    use crate::shell::session_view::SessionMenu;
    use leptos::prelude::*;

    crate::sync::hub::reset();
    for (locale, pending, synced) in [(Locale::En, "Not synced yet", "Synced"), (Locale::FrCa, "Pas encore synchronisé", "Synchronisé")] {
        let t = test_ports_full();
        let vm = SessionVm::new(t.ports.clone(), crate::i18n_for(locale));
        vm.set_sync(crate::sync::SyncVm::new(t.ports.clone(), crate::i18n_for(locale)));
        vm.start();
        let me = Me { id: "u".into(), display_name: "Ada".into(), store_on_server: Some(true), identities: vec![] };
        t.account.answer(AccountReply::Me(Ok(me)));
        let draw = |vm: &std::rc::Rc<SessionVm>| {
            let vm = vm.clone();
            html(move || view! { <SessionMenu vm=vm/> }.into_any())
        };
        let out = draw(&vm);
        assert!(out.contains(r#"id="sync-status""#) && out.contains(pending), "{locale:?}: {out}");
        t.work.answer(WorkReply::Page(Ok(WorkPage { epoch: "e1".into(), rev: 0, more: false, records: vec![], user: None })));
        let out = draw(&vm);
        assert!(out.contains(&format!(">{synced}<")), "{locale:?}: {out}");
        let signed_out = {
            let t = test_ports_full();
            let vm = SessionVm::new(t.ports.clone(), crate::i18n_for(locale));
            // a sync is attached, and nothing is synced: the choice was never made
            vm.set_sync(crate::sync::SyncVm::new(t.ports.clone(), crate::i18n_for(locale)));
            vm.start();
            t.account.answer(AccountReply::Me(Ok(Me { store_on_server: None, id: "u".into(), display_name: "Ada".into(), identities: vec![] })));
            draw(&vm)
        };
        assert!(!signed_out.contains("sync-status"), "no status where nothing is synced: {signed_out}");
    }
}

#[test]
fn the_shell_builds_the_sync_and_the_session_reports_to_it() {
    let (ports, ..) = crate::shared::ports::test_ports_with_time();
    crate::sync::hub::reset();
    let vm = crate::shell::vm::ShellVm::new_with(
        ports,
        crate::i18n_for(Locale::En),
        std::rc::Rc::new(crate::shell::Bare::default()),
        "details-details",
        vec![],
        false,
        true,
    );
    assert_eq!(vm.sync().status(), crate::sync::Status::Off);
    assert!(crate::sync::hub::current().is_some(), "the page's sync is where the other pages reach it");
}

#[test]
fn the_shell_s_links_to_the_map_and_the_editors_stay_in_the_page_s_project() {
    let shell = |target: std::rc::Rc<dyn crate::shell::Target>| {
        crate::sync::hub::reset();
        let (ports, ..) = crate::shared::ports::test_ports_with_time();
        crate::shell::vm::ShellVm::new_with(ports, crate::i18n_for(Locale::En), target, "details-details", vec![], false, true)
    };
    let vm = shell(std::rc::Rc::new(crate::shell::Bare::in_project(Some("p1".into()))));
    assert_eq!(vm.link("map.html"), "map.html?project=p1");
    assert_eq!(vm.link("street.html"), "street.html?project=p1");
    assert_eq!(vm.link("index.html"), "index.html");
    assert_eq!(vm.link("account.html"), "account.html");
    assert_eq!(shell(std::rc::Rc::new(crate::shell::Bare::default())).link("map.html"), "map.html", "a page outside a project");
    // an editor's page is in the project its storage is
    let (ports, ..) = crate::shared::ports::test_ports();
    let street = crate::street::vm::StreetVm::new(
        crate::shared::scoped::in_project(ports, "p2"),
        crate::i18n_for(Locale::En),
        crate::street::model::Editor::new(0),
        None,
    );
    assert_eq!(shell(street).link("map.html"), "map.html?project=p2");
    crate::sync::hub::reset();
}

#[test]
fn a_page_opened_offline_learns_it_is_signed_in_when_the_network_returns_and_starts_the_sync_once() {
    use crate::shared::ports::{AccountCall, AccountError, AccountReply, Me, WorkAsk, WorkCall, WorkPage, WorkReply, test_ports_full};
    crate::sync::hub::reset();
    let t = test_ports_full();
    let vm = crate::shell::vm::ShellVm::new_with(
        t.ports.clone(),
        crate::i18n_for(Locale::En),
        std::rc::Rc::new(crate::shell::Bare::default()),
        "details-details",
        vec![],
        false,
        true,
    );
    vm.session().start();
    t.account.answer(AccountReply::Me(Err(AccountError::Unavailable)));
    assert_eq!(vm.session().state(), crate::shell::session::Session::Unavailable);
    vm.network_back();
    assert_eq!(t.account.calls(), [AccountCall::Me], "the network is back: who is signed in is asked again");
    let ada = Me { id: "u".into(), display_name: "Ada".into(), store_on_server: Some(true), identities: vec![] };
    t.account.answer(AccountReply::Me(Ok(ada)));
    assert_eq!(vm.session().state(), crate::shell::session::Session::SignedIn { name: "Ada".into() });
    assert_eq!(t.work.calls(), [WorkCall::Pull(WorkAsk::first(0))], "one sync start");
    t.work.answer(WorkReply::Page(Ok(WorkPage { epoch: "e1".into(), rev: 0, more: false, records: vec![], user: None })));
    vm.network_back();
    assert_eq!(t.account.calls(), [], "a person who is known is not asked again");
    assert_eq!(t.work.calls(), [WorkCall::Pull(WorkAsk::first(0).with_epoch(Some("e1".into())))], "it syncs as any known page does");
}

// ---- the notes' tray (`shell::tray`) ----------------------------------------------------------

fn tray_html(locale: Locale) -> String {
    use crate::shell::tray::{Tray, TrayPage, TrayTab};
    use leptos::prelude::*;
    let i18n = crate::i18n_for(locale);
    crate::shared::testing::html(move || {
        let pages = vec![
            TrayPage::new(TrayTab { id: "t-one", panel: "p-one", label: "ui-tab-checks", badge: Some("one-n-slot") }, view! { <p>"one"</p> }),
            TrayPage::new(TrayTab { id: "t-two", panel: "p-two", label: "ui-tab-changes", badge: None }, view! { <p>"two"</p> }),
        ];
        view! {
            <Tray i18n=i18n pages=pages status=vec!["first-slot", "second-slot"]>
                <div id="after-panels"></div>
            </Tray>
        }
        .into_any()
    })
}

/// The opening tag of the element with `id` in `html`.
fn tag_of(html: &str, id: &str) -> String {
    let at = html.find(&format!("id=\"{id}\"")).unwrap_or_else(|| panic!("no #{id}: {html}"));
    let start = html[..at].rfind('<').unwrap();
    let end = html[at..].find('>').unwrap() + at;
    html[start..=end].to_string()
}

fn has_all(tag: &str, attrs: &[&str]) {
    for a in attrs {
        assert!(tag.contains(a), "{a}: {tag}");
    }
}

#[test]
fn the_tray_is_the_notes_with_a_handle_of_status_tabs_and_toggle_over_a_body_of_panels() {
    let h = tray_html(Locale::En);
    assert!(h.starts_with("<aside "), "{h}");
    has_all(&tag_of(&h, "notes"), &[r#"class="notes tray""#, r#"aria-label="Notes""#]);
    // the handle: the page's status slots, then the tabs, then the toggle
    let handle = &h[h.find(r#"class="tray-handle""#).expect("a handle")..h.find(r#"id="notes-body""#).expect("a body")];
    let (first, second, tabs, toggle) = (
        handle.find(r#"id="first-slot""#).expect("first status slot"),
        handle.find(r#"id="second-slot""#).expect("second status slot"),
        handle.find(r#"role="tablist""#).expect("tabs"),
        handle.find(r#"id="notes-toggle""#).expect("toggle"),
    );
    assert!(first < second && second < tabs && tabs < toggle, "{handle}");
    for slot in ["first-slot", "second-slot"] {
        has_all(&tag_of(&h, slot), &["display: contents"]);
    }
    assert!(handle.contains(r#"aria-label="Notes sections""#), "{handle}");
    let one = crate::shared::testing::button_tag(&h, "t-one");
    has_all(&one, &[r#"role="tab""#, r#"class="tab""#, r#"aria-controls="p-one""#, r#"aria-selected="true""#]);
    assert!(!one.contains("tabindex"), "the first tab is chosen and in the tab order: {one}");
    let two = crate::shared::testing::button_tag(&h, "t-two");
    has_all(&two, &[r#"aria-controls="p-two""#, r#"aria-selected="false""#, r#"tabindex="-1""#]);
    assert!(h.contains(r#"<span>Checks</span><span id="one-n-slot""#), "a badge slot after the label: {h}");
    assert!(h.contains("<span>Changes</span>"), "{h}");
    assert_eq!(h.matches("one-n-slot").count(), 1, "a slot only where the tab asks for one");
    has_all(
        &crate::shared::testing::button_tag(&h, "notes-toggle"),
        &[
            r#"class="tray-toggle""#,
            r#"aria-controls="notes-body""#,
            r#"aria-expanded="true""#,
            r#"aria-keyshortcuts="]""#,
            r#"title="Show or hide the notes ( ] )""#,
        ],
    );
    assert!(handle.contains("<span>Notes</span>"), "{handle}");
    // the body: each tab's panel, drawn from the same page as its tab (so their ids agree), the first shown, then what
    // else the page gives
    has_all(&tag_of(&h, "notes-body"), &[r#"class="tray-body""#]);
    let body = &h[h.find(r#"id="notes-body""#).unwrap()..];
    let panels = concat!(
        r#"<div id="p-one" role="tabpanel" aria-labelledby="t-one" class="panel-tab"><p>one</p></div>"#,
        r#"<div id="p-two" role="tabpanel" aria-labelledby="t-two" hidden class="panel-tab"><p>two</p></div>"#,
    );
    let (at, after) = (body.find(panels).expect(body), body.find(r#"<div id="after-panels">"#).expect(body));
    assert!(at < after, "the page's other content follows the panels: {body}");
}

#[test]
fn the_tray_is_worded_in_french() {
    let h = tray_html(Locale::FrCa);
    assert!(h.contains(r#"aria-label="Notes""#) && h.contains("Sections des notes"), "{h}");
    assert!(h.contains("<span>Vérifications</span>") && h.contains("<span>Modifications</span>"), "{h}");
}

/// A copy of a shared design (#221) is only ever looked at: the top bar says so, and the links keep it so, whether or
/// not the address says `view=1`.
#[test]
fn a_copy_of_a_shared_design_is_shown_read_only_whatever_its_address_says() {
    crate::sync::hub::reset();
    let (ports, page, storage) = crate::shared::ports::test_ports_with_page();
    let root: std::rc::Rc<dyn crate::shared::ports::Storage> = storage.clone();
    let copy = crate::project::copies::keep(&root, "tok", "Shared", "q", &[]).unwrap();
    *page.path.borrow_mut() = format!("/street.html?street=7&project={copy}");
    let target = std::rc::Rc::new(crate::shell::Bare::in_project(Some(copy.clone())));
    let vm = crate::shell::vm::ShellVm::new_with(ports, crate::i18n_for(Locale::En), target, "details-details", vec![], false, true);
    assert!(vm.read_only());
    assert_eq!(vm.link("map.html"), format!("map.html?project={copy}&view=1"));
    crate::sync::hub::reset();
}

/// A shell over the street page of a copy of a shared design (#221), what it says recorded.
fn shell_of_a_copy(
    locale: Locale,
) -> (std::rc::Rc<crate::shell::vm::ShellVm>, std::rc::Rc<crate::shared::ports::RecordingAnnouncer>, std::rc::Rc<crate::shared::ports::MemoryStorage>, String) {
    crate::sync::hub::reset();
    let (mut ports, page, storage) = crate::shared::ports::test_ports_with_page();
    let said = std::rc::Rc::new(crate::shared::ports::RecordingAnnouncer::default());
    ports.announcer = said.clone();
    let root: std::rc::Rc<dyn crate::shared::ports::Storage> = storage.clone();
    let copy = crate::project::copies::keep(&root, "tok", "Shared", "area=45.5,-73.5&name=Plateau", &[("cityloom-restrictions", "{}")]).unwrap();
    *page.path.borrow_mut() = format!("/street.html?street=7&project={copy}&view=1");
    let target = std::rc::Rc::new(crate::shell::Bare::in_project(Some(copy.clone())));
    let vm = crate::shell::vm::ShellVm::new_with(ports, crate::i18n_for(locale), target, "details-details", vec![], false, true);
    (vm, said, storage, copy)
}

/// A copy of a shared design can be made one of the person's projects to edit (#223): it is said, and the same page of
/// the new project, opened to edit, is offered; the copy itself stays read-only.
#[test]
fn a_copy_of_a_shared_design_is_forked_to_a_project_of_ones_own_and_it_is_said() {
    let (vm, said, storage, copy) = shell_of_a_copy(Locale::En);
    assert!(vm.can_fork());
    assert_eq!(vm.forked(), None);
    vm.fork();
    let projects = crate::project::ProjectStore::new(storage.clone()).list();
    assert_eq!(projects.len(), 1, "one new project of the person's own");
    let id = projects[0].id.clone();
    assert_eq!(projects[0].name, "Shared");
    assert_eq!(said.take(), ["A copy of Shared is now one of your projects. Open it to make changes."]);
    assert_eq!(vm.forked(), Some(format!("street.html?street=7&project={id}")));
    assert!(!vm.can_fork(), "the copy is made: the page offers to open it");
    assert_eq!(vm.fork_error(), None);
    assert!(vm.read_only(), "this page still shows the shared design");
    assert!(crate::project::copies::is_copy(&*storage, &copy));
    crate::sync::hub::reset();
}

#[test]
fn a_fork_is_said_in_french() {
    let (vm, said, _, _) = shell_of_a_copy(Locale::FrCa);
    vm.fork();
    assert_eq!(said.take(), ["Une copie de Shared fait maintenant partie de vos projets. Ouvrez-la pour la modifier."]);
    crate::sync::hub::reset();
}

/// Only a copy of a shared design is forked: a project of the person's own opened with `view=1` is theirs already.
#[test]
fn a_project_of_ones_own_shown_read_only_offers_no_fork() {
    crate::sync::hub::reset();
    let (ports, page, storage) = crate::shared::ports::test_ports_with_page();
    let own = crate::project::ProjectStore::new(storage.clone()).create("Mine").unwrap();
    *page.path.borrow_mut() = format!("/map.html?project={}&view=1", own.id);
    let target = std::rc::Rc::new(crate::shell::Bare::in_project(Some(own.id.clone())));
    let vm = crate::shell::vm::ShellVm::new_with(ports, crate::i18n_for(Locale::En), target, "details-details", vec![], false, true);
    assert!(vm.read_only() && !vm.can_fork());
    vm.fork();
    assert_eq!(crate::project::ProjectStore::new(storage.clone()).list().len(), 1, "nothing made");
    crate::sync::hub::reset();
}

/// A fork the browser cannot keep, or of a copy gone meanwhile (another tab opened ten other links), is said, shown
/// beside the button, and leaves no project; the button stays to try again.
#[test]
fn a_fork_that_fails_is_said_and_leaves_nothing() {
    use crate::shared::ports::Storage;
    let (vm, said, storage, _) = shell_of_a_copy(Locale::En);
    storage.refuse("cityloom-restrictions@p1");
    vm.fork();
    let full = "No copy was made: this browser could not keep it. Free some space and try again.";
    assert_eq!(said.take(), [full]);
    assert_eq!(vm.fork_error().as_deref(), Some(full));
    assert!(vm.can_fork() && vm.forked().is_none());
    assert!(crate::project::ProjectStore::new(storage.clone()).list().is_empty());

    storage.refuse_nothing();
    storage.remember(crate::project::copies::COPIES_KEY, "");
    vm.fork();
    let gone = "No copy was made: this shared design is no longer kept on this device. Open its link again.";
    assert_eq!(said.take(), [gone]);
    assert_eq!(vm.fork_error().as_deref(), Some(gone));
    crate::sync::hub::reset();
}

/// A list of projects a newer CityLoom wrote cannot take the fork: freeing space would not help, reloading the page
/// (and so this CityLoom) would.
#[test]
fn a_fork_refused_by_a_newer_list_of_projects_says_to_reload() {
    use crate::shared::ports::Storage;
    for (locale, words) in [
        (Locale::En, "No copy was made: your projects were last changed by a newer CityLoom. Reload the page and try again."),
        (
            Locale::FrCa,
            "Aucune copie n’a été faite\u{a0}: vos projets ont été modifiés en dernier par une version plus récente de CityLoom. Rechargez la page et réessayez.",
        ),
    ] {
        let (vm, said, storage, _) = shell_of_a_copy(locale);
        storage.remember(crate::project::store::PROJECTS_KEY, "{\"version\":99,\"projects\":[]}");
        vm.fork();
        assert_eq!(said.take(), [words]);
        assert_eq!(vm.fork_error().as_deref(), Some(words));
        assert!(vm.can_fork(), "the button stays");
        crate::sync::hub::reset();
    }
}

fn fork_html(vm: std::rc::Rc<crate::shell::vm::ShellVm>) -> String {
    crate::shared::testing::html(move || {
        use leptos::prelude::IntoAny;
        leptos::view! { <crate::shell::view::ForkAction vm=vm/> }.into_any()
    })
}

/// A copy's pages say, in a notice of their own (the top bar has no room for it), that the design was shared and how to
/// change it, and hold the fork there; not a live region: what it does is announced when it is done.
#[test]
fn a_copy_s_pages_hold_the_fork_in_a_notice_of_its_own() {
    let notice = |locale: Locale| {
        let (vm, _, _, _) = shell_of_a_copy(locale);
        let html = crate::shared::testing::html(move || {
            use leptos::prelude::IntoAny;
            leptos::view! { <crate::shell::view::ForkNotice vm=vm/> }.into_any()
        });
        crate::sync::hub::reset();
        html
    };
    let en = notice(Locale::En);
    assert!(en.contains(r#"class="shell-notice fork-notice""#), "{en}");
    assert!(en.contains("This design was shared with you. To change it, make a copy of your own."), "{en}");
    assert!(en.contains(r#"id="fork-btn""#), "{en}");
    assert!(!en.contains("role="), "no live region: {en}");
    let fr = notice(Locale::FrCa);
    assert!(fr.contains("Cet aménagement a été partagé avec vous. Pour le modifier, faites-en une copie à vous."), "{fr}");

    // once forked, the notice says so where it can be seen, not only to a screen reader
    let (vm, _, _, _) = shell_of_a_copy(Locale::En);
    vm.fork();
    let forked = crate::shared::testing::html(move || {
        use leptos::prelude::IntoAny;
        leptos::view! { <crate::shell::view::ForkNotice vm=vm/> }.into_any()
    });
    crate::sync::hub::reset();
    assert!(forked.contains("A copy of Shared is now one of your projects. Open it to make changes."), "{forked}");
    assert!(!forked.contains("To change it, make a copy of your own."), "{forked}");
}

/// The notice of a copy offers the fork as a button (keyboard reachable, described by what it does); once forked, a
/// link to the new project in its place; a failure is shown beside the button.
#[test]
fn the_fork_is_a_described_button_then_a_link_to_the_new_project() {
    let (vm, _, storage, _) = shell_of_a_copy(Locale::En);
    let before = fork_html(vm.clone());
    assert!(before.contains(r#"<button type="button" id="fork-btn""#), "{before}");
    assert!(before.contains(r#"aria-describedby="fork-hint""#), "{before}");
    assert!(before.contains(">Edit a copy<"), "{before}");
    assert!(before.contains("Adds a copy of this design to your projects, where you can change it."), "{before}");
    assert!(!before.contains("fork-open"), "{before}");
    assert!(!before.contains(r#"id="fork-error""#), "no failure shown: {before}");
    vm.fork();
    let id = crate::project::ProjectStore::new(storage.clone()).list()[0].id.clone();
    let after = fork_html(vm.clone());
    assert!(!after.contains("fork-btn"), "{after}");
    assert!(after.contains(&format!(r#"href="street.html?street=7&amp;project={id}""#)), "{after}");
    assert!(after.contains(r#"id="fork-open""#) && after.contains(">Open your copy<"), "{after}");
    crate::sync::hub::reset();

    let (vm, _, storage, _) = shell_of_a_copy(Locale::FrCa);
    storage.refuse("cityloom-restrictions@p1");
    vm.fork();
    let failed = fork_html(vm);
    assert!(failed.contains(">Modifier une copie<"), "{failed}");
    assert!(failed.contains(r#"id="fork-error""#), "{failed}");
    assert!(failed.contains(r#"aria-describedby="fork-hint fork-error""#), "the button holds why it failed: {failed}");
    assert!(failed.contains("Aucune copie n’a été faite\u{a0}: ce navigateur n’a pas pu la conserver."), "{failed}");
    crate::sync::hub::reset();
}

/// A page whose address opens its project read-only (`view=1`): the shell's links to the project's pages keep it, and
/// its top bar says so in the page's language.
#[test]
fn a_page_shown_read_only_says_so_and_its_links_keep_it_read_only() {
    let shell = |path: &str| {
        crate::sync::hub::reset();
        let (ports, page, _) = crate::shared::ports::test_ports_with_page();
        *page.path.borrow_mut() = path.to_string();
        let target = std::rc::Rc::new(crate::shell::Bare::in_project(Some("p1".into())));
        crate::shell::vm::ShellVm::new_with(ports, crate::i18n_for(Locale::En), target, "details-details", vec![], false, true)
    };
    let vm = shell("/map.html?project=p1&view=1");
    assert!(vm.read_only());
    assert_eq!(vm.link("street.html?street=7"), "street.html?street=7&project=p1&view=1");
    assert_eq!(vm.link("index.html"), "index.html", "the list of projects is no page of the project");
    let vm = shell("/map.html?project=p1");
    assert!(!vm.read_only());
    assert_eq!(vm.link("street.html?street=7"), "street.html?street=7&project=p1");
    crate::sync::hub::reset();

    let tag = |locale: Locale| {
        let i18n = crate::i18n_for(locale);
        crate::shared::testing::html(move || {
            use leptos::prelude::IntoAny;
            leptos::view! { <crate::shell::view::ReadOnlyTag i18n=i18n/> }.into_any()
        })
    };
    let en = tag(Locale::En);
    assert!(en.contains(r#"id="read-only-tag""#), "{en}");
    assert!(en.contains(">Read-only<"), "{en}");
    assert!(en.contains("This design is open read-only: you can look at it, but not change it."), "{en}");
    let fr = tag(Locale::FrCa);
    assert!(fr.contains(">Lecture seule<"), "{fr}");
    assert!(fr.contains("Cet aménagement est ouvert en lecture seule\u{a0}: vous pouvez le consulter, mais pas le modifier."), "{fr}");
}

/// A street shown read-only, opened where the other drive side's region is kept: the shell gives the street that region
/// on load, which is not taken and says nothing. Choosing a region there is refused like any edit: it is said, the
/// street keeps its own, the menu shows that one, and neither the region kept nor "region chosen" is said or written.
#[test]
fn a_page_shown_read_only_takes_no_region_says_nothing_on_load_and_refuses_a_choice() {
    use crate::shared::ports::{RecordingAnnouncer, Storage};
    use crate::shell::vm::REGION_KEY;
    use std::rc::Rc;
    crate::sync::hub::reset();
    let (mut ports, page, storage) = crate::shared::ports::test_ports_with_page();
    let said = Rc::new(RecordingAnnouncer::default());
    ports.announcer = said.clone();
    *page.path.borrow_mut() = "/street.html?street=7&project=p1&view=1".into();
    storage.remember(REGION_KEY, "united-kingdom");
    let view: Rc<dyn Storage> = Rc::new(crate::shared::scoped::ScopedStorage::new(storage.clone(), "p1").only_to_read());
    let street = crate::street::vm::StreetVm::new(
        crate::shared::ports::Ports { storage: view, ..ports.clone() },
        crate::i18n_for(Locale::En),
        crate::street::model::Editor::new(0),
        None,
    );
    let before = street.read(|e| e.snapshot());
    let shell = crate::shell::vm::ShellVm::new_with(ports, crate::i18n_for(Locale::En), street.clone(), "details-details", vec![], false, true);
    assert!(shell.read_only() && shell.region_locked());
    assert!(said.take().is_empty(), "nothing said on load");
    assert_eq!(street.read(|e| e.snapshot()), before, "the design keeps its drive side");
    let own = shell.region();
    assert_ne!(own, "united-kingdom", "the menu shows the street's region");
    shell.choose_region("japan");
    assert_eq!(said.take(), ["This design is open read-only: nothing can be changed."]);
    assert_eq!((shell.region(), street.read(|e| e.snapshot())), (own, before));
    assert_eq!(storage.recall(REGION_KEY).as_deref(), Some("united-kingdom"), "the region kept is not changed");
    crate::sync::hub::reset();
}

/// A map shown read-only: its region is only how the city is shown (the person's preference, not the design), so it
/// is chosen, kept and said as on any page.
#[test]
fn a_map_shown_read_only_still_takes_a_region_keeps_it_and_says_it() {
    use crate::shared::ports::{RecordingAnnouncer, Storage};
    use crate::shell::vm::REGION_KEY;
    use std::rc::Rc;
    crate::sync::hub::reset();
    let (mut ports, page, storage) = crate::shared::ports::test_ports_with_page();
    let said = Rc::new(RecordingAnnouncer::default());
    ports.announcer = said.clone();
    *page.path.borrow_mut() = "/map.html?project=p1&view=1".into();
    let target = Rc::new(crate::shell::Bare::in_project(Some("p1".into())));
    let shell = crate::shell::vm::ShellVm::new_with(ports, crate::i18n_for(Locale::En), target, "details-details", vec![], false, true);
    assert!(shell.read_only() && !shell.region_locked());
    shell.choose_region("japan");
    assert_eq!(shell.region(), "japan");
    assert_eq!(storage.recall(REGION_KEY).as_deref(), Some("japan"));
    let words = said.take();
    assert!(words.len() == 1 && words[0].contains("Japan"), "{words:?}");
    crate::sync::hub::reset();
}

mod edits_cleared {
    use std::rc::Rc;

    use crate::shared::i18n::{Args, Locale};
    use crate::shared::ports::{MemoryStorage, RecordingAnnouncer, Storage, test_ports_with_page};
    use crate::shell::vm::ShellVm;
    use crate::shell::{Bare, view::EditsClearedNotice};

    const MESSAGE: &str = "The map was rebuilt with one street per block, so your earlier street and junction edits were cleared.";

    fn edits_document(import: Option<u32>) -> String {
        let record = match import {
            Some(n) => serde_json::json!({"ways": [1], "import": n}),
            None => serde_json::json!({"ways": [1]}),
        };
        serde_json::json!({"version": crate::city::edits::IMPORT_STAMP_VERSION, "streets": {"w1~n1~n2": record}, "junctions": {}}).to_string()
    }

    fn open(storage: &Rc<MemoryStorage>, project: Option<&str>) -> (Rc<ShellVm>, Rc<RecordingAnnouncer>) {
        let (mut ports, _, _) = test_ports_with_page();
        let said = Rc::new(RecordingAnnouncer::default());
        ports.announcer = said.clone();
        ports.storage = storage.clone();
        crate::sync::hub::reset();
        let target = Rc::new(Bare::in_project(project.map(String::from)));
        (ShellVm::new_with(ports, crate::i18n_for(Locale::En), target, "details-details", vec![], false, true), said)
    }

    fn storage_with(key: &str, doc: &str) -> Rc<MemoryStorage> {
        let (_, _, storage) = test_ports_with_page();
        storage.remember(key, doc);
        storage
    }

    fn streets_in(storage: &MemoryStorage, key: &str) -> usize {
        let v: serde_json::Value = serde_json::from_str(&storage.recall(key).unwrap()).unwrap();
        v["streets"].as_object().unwrap().len()
    }

    #[test]
    fn a_page_of_a_project_with_stale_edits_clears_them_and_says_so_once() {
        let storage = storage_with("cityloom-edits@p1", &edits_document(None));
        let (vm, said) = open(&storage, Some("p1"));
        assert!(vm.edits_cleared());
        assert_eq!(said.take(), vec![MESSAGE.to_string()]);
        assert_eq!(streets_in(&storage, "cityloom-edits@p1"), 0);
        let (again, said) = open(&storage, Some("p1"));
        assert!(!again.edits_cleared());
        assert!(said.take().is_empty());
    }

    #[test]
    fn a_project_without_stale_edits_shows_nothing() {
        let storage = storage_with("cityloom-edits@p1", &edits_document(Some(crate::city::edits::STALE_BEFORE_IMPORT)));
        let (vm, said) = open(&storage, Some("p1"));
        assert!(!vm.edits_cleared());
        assert!(said.take().is_empty());
        assert_eq!(streets_in(&storage, "cityloom-edits@p1"), 1);
    }

    /// A design opened read-only (`view=1`) is not changed, so its stale edits wait, never applied nor sent, until the
    /// project is next opened to edit, which clears them and says so.
    #[test]
    fn a_page_shown_read_only_discards_nothing_and_the_next_page_to_edit_does() {
        let storage = storage_with("cityloom-edits@p1", &edits_document(None));
        let (mut ports, page, _) = test_ports_with_page();
        let said = Rc::new(RecordingAnnouncer::default());
        ports.announcer = said.clone();
        ports.storage = storage.clone();
        *page.path.borrow_mut() = "/map.html?project=p1&view=1".into();
        crate::sync::hub::reset();
        let target = Rc::new(Bare::in_project(Some("p1".into())));
        let vm = ShellVm::new_with(ports, crate::i18n_for(Locale::En), target, "details-details", vec![], false, true);
        assert!(vm.read_only());
        assert!(!vm.edits_cleared());
        assert!(said.take().is_empty());
        assert_eq!(streets_in(&storage, "cityloom-edits@p1"), 1, "the read-only page keeps the document as it is");
        let (vm, said) = open(&storage, Some("p1"));
        assert!(vm.edits_cleared());
        assert_eq!(said.take(), vec![MESSAGE.to_string()]);
        assert_eq!(streets_in(&storage, "cityloom-edits@p1"), 0);
    }

    #[test]
    fn a_page_in_no_project_discards_nothing() {
        let storage = storage_with("cityloom-edits", &edits_document(None));
        let (vm, _) = open(&storage, None);
        assert!(!vm.edits_cleared());
        assert_eq!(streets_in(&storage, "cityloom-edits"), 1);
    }

    #[test]
    fn dismissing_hides_the_notice() {
        let storage = storage_with("cityloom-edits@p1", &edits_document(None));
        let (vm, _) = open(&storage, Some("p1"));
        vm.dismiss_edits_cleared();
        assert!(!vm.edits_cleared());
    }

    #[test]
    fn the_notice_is_a_status_with_the_message_and_a_dismiss_button_in_both_languages() {
        use crate::shared::testing::html;
        use leptos::prelude::*;
        for (locale, button) in [(Locale::En, "Dismiss"), (Locale::FrCa, "Fermer")] {
            let storage = storage_with("cityloom-edits@p1", &edits_document(None));
            let (mut ports, _, _) = test_ports_with_page();
            ports.storage = storage;
            crate::sync::hub::reset();
            let i18n = crate::i18n_for(locale);
            let vm = ShellVm::new_with(ports, i18n.clone(), Rc::new(Bare::in_project(Some("p1".into()))), "details-details", vec![], false, true);
            let out = html({
                let vm = vm.clone();
                move || view! { <EditsClearedNotice vm=vm/> }.into_any()
            });
            let message = i18n.tr_now("shell-edits-cleared", &Args::new());
            assert!(out.contains(r#"role="status""#) && out.contains(r#"class="shell-notice""#), "{locale:?}: {out}");
            assert!(out.contains(&message) && out.contains(&format!(">{button}<")), "{locale:?}: {out}");
            assert!(out.contains(r#"type="button""#), "{out}");
            if locale == Locale::En {
                assert_eq!(message, MESSAGE);
            }
            vm.dismiss_edits_cleared();
            assert!(
                !html({
                    let vm = vm.clone();
                    move || view! { <EditsClearedNotice vm=vm/> }.into_any()
                })
                .contains("shell-notice")
            );
        }
    }
}
