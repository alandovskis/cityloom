//! The Account page's words and markup.

use std::rc::Rc;

use leptos::prelude::*;

use crate::account::RESOURCES;
use crate::account::view::AccountView;
use crate::account::vm::AccountVm;
use crate::shared::i18n::tests::parity_problems;
use crate::shared::i18n::{Args, Locale};
use crate::shared::ports::{AccountError, AccountReply, LinkedIdentity, Me, ProviderInfo, TestPorts, test_ports_full};
use crate::shared::testing::html;
use crate::shell::view::i18n_keys;

fn page(locale: Locale, replies: Vec<AccountReply>, then: impl FnOnce(&Rc<AccountVm>, &TestPorts)) -> String {
    let t = test_ports_full();
    let vm = AccountVm::new(t.ports.clone(), crate::i18n_for(locale));
    vm.load();
    for r in replies {
        t.account.answer(r);
    }
    then(&vm, &t);
    html(move || view! { <AccountView vm=vm/> }.into_any())
}

fn identity(id: &str, provider: &str, label: Option<&str>) -> LinkedIdentity {
    LinkedIdentity { id: id.into(), provider: provider.into(), label: label.map(str::to_string), created_at: String::new() }
}

fn providers() -> AccountReply {
    AccountReply::Providers(Ok(["osm", "google", "microsoft", "apple"].iter().map(|s| ProviderInfo { slug: s.to_string(), name: s.to_uppercase() }).collect()))
}

fn signed_in(store: Option<bool>, identities: Vec<LinkedIdentity>) -> Vec<AccountReply> {
    vec![AccountReply::Me(Ok(Me { id: "u".into(), display_name: "Ada".into(), store_on_server: store, identities })), providers()]
}

fn no_op(_: &Rc<AccountVm>, _: &TestPorts) {}

#[test]
fn the_ftl_files_have_the_same_messages_and_variables() {
    assert_eq!(parity_problems(RESOURCES.en, RESOURCES.fr), Vec::<String>::new());
}

#[test]
fn account_html_asks_only_for_messages_that_exist_and_says_them_in_english() {
    let html = include_str!("../../web/account.html");
    let i = crate::i18n_for(Locale::En);
    let f = crate::i18n_for(Locale::FrCa);
    let keys = i18n_keys(html);
    assert!(keys.len() >= 5, "the page should be translated: {}", keys.len());
    for (key, fallback) in keys {
        assert_eq!(fallback, i.tr_now(&key, &Args::new()), "{key}");
        f.tr_now(&key, &Args::new());
    }
    assert!(i.missing().is_empty() && f.missing().is_empty(), "{:?} {:?}", i.missing(), f.missing());
    assert!(html.contains(r#"id="session-slot""#) && html.contains(r#"id="account-slot""#) && html.contains(r#"id="live""#));
    assert!(html.contains(r#"data-lang="fr-CA""#) && html.contains(r#"localStorage.getItem("cityloom-lang")"#));
    assert_eq!(html.matches("data-version").count(), 1);
}

#[test]
fn a_signed_out_page_offers_the_providers_and_a_server_that_is_down_says_so() {
    let out = page(Locale::En, vec![AccountReply::Me(Err(AccountError::SignedOut)), providers()], no_op);
    assert!(out.contains("You are signed out.") && out.contains(r#"data-provider="google""#) && out.contains(r#"data-provider="osm""#), "{out}");
    assert!(!out.contains("Delete account"), "{out}");
    let out = page(Locale::En, vec![AccountReply::Me(Err(AccountError::Unavailable))], no_op);
    assert!(out.contains("Accounts are not available right now.") && !out.contains("data-provider"), "{out}");
}

#[test]
fn a_new_account_sees_the_storage_choice_first_with_two_options_and_nothing_selected() {
    let out = page(Locale::En, signed_in(None, vec![identity("a", "google", Some("ada@example.org"))]), no_op);
    let storage = out.find("Where is your work kept?").expect("the choice is shown");
    let linked = out.find("Linked sign-ins").expect("the linked list is shown");
    assert!(storage < linked, "the choice comes first while nothing is chosen: {out}");
    assert!(out.contains("Keep my work on this device only") && out.contains("Store my work on the server"), "{out}");
    assert_eq!(out.matches(r#"type="radio""#).count(), 2, "{out}");
    assert!(!out.contains("checked"), "nothing is preselected: {out}");
    assert!(out.contains("cannot share it except by printing") || out.contains("You cannot share it except by printing it."), "{out}");
}

#[test]
fn once_chosen_the_choice_is_marked_and_moves_below_the_linked_sign_ins() {
    for (store, checked) in [(Some(true), "server"), (Some(false), "device")] {
        let out = page(Locale::En, signed_in(store, vec![identity("a", "google", None)]), no_op);
        assert_eq!(out.matches("checked").count(), 1, "{out}");
        assert!(out.contains(&format!(r#"value="{checked}" checked"#)), "{checked}: {out}");
        assert!(out.find("Linked sign-ins").unwrap() < out.find("Where is your work kept?").unwrap(), "{out}");
    }
}

#[test]
fn the_linked_sign_ins_show_their_labels_and_unlink_is_not_offered_on_the_last() {
    let out = page(Locale::En, signed_in(Some(true), vec![identity("a", "google", Some("ada@example.org")), identity("b", "osm", None)]), no_op);
    assert!(out.contains("GOOGLE (ada@example.org)") && out.contains(">OSM<"), "{out}");
    assert_eq!(out.matches(">Unlink<").count(), 2, "{out}");
    let out = page(Locale::En, signed_in(Some(true), vec![identity("a", "google", None)]), no_op);
    assert!(!out.contains(">Unlink<") && out.contains("This is your only sign-in, so it cannot be unlinked."), "{out}");
}

#[test]
fn the_link_buttons_are_for_every_enabled_provider_so_a_second_account_of_one_can_be_linked() {
    let out = page(Locale::En, signed_in(Some(true), vec![identity("a", "google", None), identity("b", "osm", None)]), no_op);
    for provider in ["OSM", "GOOGLE", "MICROSOFT", "APPLE"] {
        assert!(out.contains(&format!(">Link {provider}<")), "{provider}: {out}");
    }
    assert!(out.contains("the labels tell them apart"), "{out}");
}

#[test]
fn deleting_is_confirmed_in_the_page_with_the_backup_retention_said() {
    let before = page(Locale::En, signed_in(Some(true), vec![identity("a", "google", None)]), no_op);
    assert!(before.contains("Delete account…") && !before.contains("Keep my account"), "{before}");
    let after = page(Locale::En, signed_in(Some(true), vec![identity("a", "google", None)]), |vm, _| vm.ask_delete());
    assert!(after.contains("Delete my account") && after.contains("Keep my account"), "{after}");
    assert!(after.contains("Railway") && after.contains("backups"), "the retention is said: {after}");
    assert!(after.contains("Work kept on this device is not affected."), "{after}");
    assert!(after.contains(r#"aria-describedby="delete-warning""#) && after.contains(r#"id="delete-warning""#), "{after}");
}

#[test]
fn the_page_speaks_french() {
    let out = page(Locale::FrCa, signed_in(None, vec![identity("a", "google", None)]), no_op);
    assert!(
        out.contains("Où votre travail est-il conservé")
            && out.contains("Conserver mon travail sur cet appareil seulement")
            && out.contains("Stocker mon travail sur le serveur"),
        "{out}"
    );
    let out = page(Locale::FrCa, vec![AccountReply::Me(Err(AccountError::SignedOut)), providers()], no_op);
    assert!(out.contains("Aucune session n’est ouverte."), "{out}");
}

#[test]
fn a_name_with_markup_is_escaped() {
    let t = test_ports_full();
    let vm = AccountVm::new(t.ports.clone(), crate::i18n_for(Locale::En));
    vm.load();
    t.account.answer(AccountReply::Me(Ok(Me {
        id: "u".into(),
        display_name: "<script>x</script>".into(),
        store_on_server: Some(true),
        identities: vec![identity("a", "google", None)],
    })));
    let out = html(move || view! { <AccountView vm=vm/> }.into_any());
    assert!(!out.contains("<script>x") && out.contains("&lt;script&gt;"), "{out}");
}
