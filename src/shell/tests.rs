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
fn the_share_menu_is_a_button_and_a_hidden_menu_holding_print_and_export_in_the_language_of_the_page() {
    use crate::shared::testing::html;
    use crate::shell::share_view::ShareMenu;
    use leptos::prelude::*;

    for (locale, print, export) in [(Locale::En, "Print this sheet", "Export design"), (Locale::FrCa, "Imprimer cette fiche", "Exporter la conception")] {
        let i18n = crate::i18n_for(locale);
        let page = html(move || view! { <ShareMenu i18n=i18n/> }.into_any());
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
fn every_app_page_has_one_slot_for_the_sign_in_menu() {
    for (name, html) in [
        ("index", include_str!("../../web/index.html")),
        ("map", include_str!("../../web/map.html")),
        ("street", include_str!("../../web/street.html")),
        ("intersection", include_str!("../../web/intersection.html")),
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
    for (page, html) in [("street", include_str!("../../web/street.html")), ("intersection", include_str!("../../web/intersection.html"))] {
        let body = &html[html.find("<body>").unwrap() + "<body>".len()..];
        assert!(
            body.trim_start().starts_with(r##"<a class="skip-link" href="#wrap" data-i18n="ui-skip-drawing">Skip to the drawing</a>"##),
            "{page}: the first thing in the body"
        );
    }
    let css = include_str!("../../web/app.css");
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
        ("street", include_str!("../../web/street.html"), "st-piece-details"),
        ("intersection", include_str!("../../web/intersection.html"), "jn-page-details-panel"),
    ] {
        let aside = &html[html.find("id=\"details-panel\"").unwrap()..];
        let first = aside[aside.find('>').unwrap() + 1..].trim_start();
        assert!(first.starts_with(&format!("<h2 class=\"sr-only\" data-i18n=\"{key}\">")), "{page}: {}", &first[..80]);
    }
}

/// The pages that run the module say, in their own markup, that it could not start: the module holds every other
/// message, and it is what did not load. `web/boot.js` shows the block of the page's language.
#[test]
fn every_page_that_loads_the_module_has_its_start_failure_in_both_languages() {
    let pages = [
        ("index", include_str!("../../web/index.html")),
        ("map", include_str!("../../web/map.html")),
        ("street", include_str!("../../web/street.html")),
        ("intersection", include_str!("../../web/intersection.html")),
        ("account", include_str!("../../web/account.html")),
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

#[test]
fn the_pages_that_read_the_city_preload_the_tile_index() {
    for (name, html) in [
        ("index", include_str!("../../web/index.html")),
        ("map", include_str!("../../web/map.html")),
        ("street", include_str!("../../web/street.html")),
        ("intersection", include_str!("../../web/intersection.html")),
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
