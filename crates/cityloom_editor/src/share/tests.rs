use std::rc::Rc;

use serde_json::{Value, json};

use super::model::{self, ShareError, compose, parse_and_validate};
use super::vm::{Account, Context, Focus, OpenVm, Opening, ShareVm};
use crate::city::edits::EDITS_KEY;
use crate::place::area::default_area;
use crate::restriction::store::RESTRICTIONS_KEY;
use crate::shared::i18n::Locale;
use crate::shared::ports::{FAKE_ORIGIN, MemoryStorage, ShareCall, ShareFailure, ShareMade, ShareReply, Storage, TestPorts, test_ports_full};
use crate::shared::testing::{en, fr};

const TOKEN: &str = "AbCdEfGhIjKlMnOpQrStUvWxYz0123456789-_abcde";

fn ring() -> Value {
    json!([[-73.6, 45.52], [-73.59, 45.52], [-73.59, 45.53], [-73.6, 45.52]])
}

fn edits() -> Value {
    // Stamped by a current importer (`city::edits::STALE_BEFORE_IMPORT`): an unstamped record is stale, never applied.
    let import = crate::city::edits::STALE_BEFORE_IMPORT;
    json!({"version": 5, "streets": {"w1~n2~n3": {"ways": [1], "import": import, "street": {"name": "Rue Rachel", "row_mm": 20000}, "future": [1]}}, "junctions": {}})
}

fn rules() -> Value {
    json!({"version": 1, "restrictions": {"r-1": {"name": "Car-free", "rule": {"kind": "car_free"}, "ring": ring()}}})
}

/// A project's storage holding a street edit and an area rule.
fn project_storage() -> (Rc<MemoryStorage>, Rc<dyn Storage>) {
    let memory = Rc::new(MemoryStorage::default());
    memory.remember(EDITS_KEY, &edits().to_string());
    memory.remember(RESTRICTIONS_KEY, &rules().to_string());
    memory.set_now(1_800_000_000_000);
    (memory.clone(), memory)
}

fn share_text() -> String {
    let (_, storage) = project_storage();
    compose(&storage, "Plateau", &default_area().to_query(), "0.96.0", "2027-01-15T08:00:00.000Z").unwrap()
}

fn with(change: &dyn Fn(&mut Value)) -> String {
    let mut v: Value = serde_json::from_str(&share_text()).unwrap();
    change(&mut v);
    v.to_string()
}

// ---- the format ----

#[test]
fn a_share_holds_the_project_as_it_is_and_reads_back_whole() {
    let text = share_text();
    let v: Value = serde_json::from_str(&text).unwrap();
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(keys, ["app_version", "created_at", "edits", "format_version", "kind", "project", "restrictions"], "no milestones, nothing of the account");
    assert_eq!((v["kind"].as_str(), v["format_version"].as_u64()), (Some("cityloom-share"), Some(1)));
    assert_eq!(v["edits"], edits(), "the store's own document, a newer build's field too");
    assert_eq!(v["restrictions"], rules(), "the area rules");
    let shared = parse_and_validate(&text).unwrap();
    assert_eq!((shared.name.as_str(), shared.area.clone(), shared.streets, shared.junctions, shared.rules), ("Plateau", default_area().to_query(), 1, 0, 1));
    let docs = shared.documents();
    assert_eq!(docs.iter().map(|(k, _)| *k).collect::<Vec<_>>(), [EDITS_KEY, RESTRICTIONS_KEY]);
    assert_eq!(serde_json::from_str::<Value>(&docs[0].1).unwrap(), edits());
}

#[test]
fn a_project_with_no_work_shares_its_area_alone_and_unknown_fields_are_ignored() {
    let empty: Rc<dyn Storage> = Rc::new(MemoryStorage::default());
    let text = compose(&empty, "Empty", &default_area().to_query(), "0.96.0", "x").unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    assert!(v.get("edits").is_none() && v.get("restrictions").is_none());
    assert!(parse_and_validate(&text).unwrap().documents().is_empty());
    assert!(parse_and_validate(&with(&|v| v["notes"] = json!({"later": true}))).is_ok());
}

#[test]
fn what_is_not_a_whole_share_is_refused_as_damaged_or_newer() {
    let cases: Vec<(&str, String, ShareError)> = vec![
        ("not JSON", "{".into(), ShareError::Damaged),
        ("not an object", "[1]".into(), ShareError::Damaged),
        ("another kind", with(&|v| v["kind"] = json!("cityloom-design")), ShareError::Damaged),
        ("no version", with(&|v| drop(v.as_object_mut().unwrap().remove("format_version"))), ShareError::Damaged),
        ("version 0", with(&|v| v["format_version"] = json!(0)), ShareError::Damaged),
        ("a newer format", with(&|v| v["format_version"] = json!(2)), ShareError::Newer),
        ("newer edits", with(&|v| v["edits"]["version"] = json!(99)), ShareError::Newer),
        ("newer rules", with(&|v| v["restrictions"]["version"] = json!(2)), ShareError::Newer),
        ("no project", with(&|v| drop(v.as_object_mut().unwrap().remove("project"))), ShareError::Damaged),
        ("no name", with(&|v| v["project"]["name"] = json!("  ")), ShareError::Damaged),
        ("an area that is no area", with(&|v| v["project"]["area"] = json!("nowhere")), ShareError::Damaged),
        ("edits that are not a document", with(&|v| v["edits"] = json!("x")), ShareError::Damaged),
        ("a rule that is not an object", with(&|v| v["restrictions"]["restrictions"] = json!({"r1": 1})), ShareError::Damaged),
        ("a header field of the wrong type", with(&|v| v["app_version"] = json!(7)), ShareError::Damaged),
        ("over a megabyte", format!("{{\"pad\":\"{}\"}}", " ".repeat(model::MAX_BYTES)), ShareError::Damaged),
    ];
    for (name, text, want) in cases {
        assert_eq!(parse_and_validate(&text), Err(want), "{name}");
    }
}

#[test]
fn a_project_whose_work_cannot_be_read_or_is_too_large_is_not_shared() {
    let (memory, storage) = project_storage();
    memory.remember(EDITS_KEY, "{not json");
    assert_eq!(compose(&storage, "P", "q", "v", "t"), Err(ShareError::Unusable));
    memory.remember(EDITS_KEY, "{\"version\":99,\"streets\":{}}");
    assert_eq!(compose(&storage, "P", "q", "v", "t"), Err(ShareError::Unusable), "a newer build's");
    memory.remember(EDITS_KEY, &edits().to_string());
    let big = json!({"version": 5, "streets": {"w1": {"pad": " ".repeat(model::MAX_BYTES)}}, "junctions": {}});
    memory.remember(EDITS_KEY, &big.to_string());
    assert_eq!(compose(&storage, "P", "q", "v", "t"), Err(ShareError::TooLarge));
}

#[test]
fn a_token_is_43_characters_of_base64url_and_its_link_carries_it_in_the_fragment() {
    assert!(model::valid_token(TOKEN));
    for bad in ["", "short", &TOKEN[..42], &format!("{TOKEN}a"), &format!("{}+", &TOKEN[..42]), "account=signed-in"] {
        assert!(!model::valid_token(bad), "{bad}");
    }
    assert_eq!(model::link("https://cityloom.example", TOKEN), format!("https://cityloom.example/share.html#{TOKEN}"));
}

/// The share page opens a link only for a whole token, or for a fragment where no project is named; a project's page
/// keeps its own anchors (the skip link's `#sharing-heading`, after a reload) and the account's sign-in fragment.
#[test]
fn the_page_opens_a_link_only_for_a_whole_token_or_where_it_names_no_project() {
    let opens = |hash: &str, search: &str| model::opens_a_link(hash, search);
    assert!(opens(TOKEN, "") && opens(TOKEN, "?project=p1"), "a whole token is a link, whatever else the address says");
    assert!(opens("short", "") && opens("sharing-heading", ""), "no project named: a fragment is a link, said not whole");
    assert!(!opens("sharing-heading", "?project=p1") && !opens("short", "project=p1&view=1"), "a project's page keeps its anchors");
    assert!(!opens("account=signed-in", "") && !opens("account=signed-in", "?project=p1"), "the sign-in's fragment");
    assert!(!opens("", "") && !opens("", "?project=p1"), "no fragment: the project's page (the one opened last where none is named)");
}

#[test]
fn every_failure_is_said_in_english_and_in_french() {
    use ShareError::*;
    let all =
        [BadLink, NotFound, Expired, Deleted, Newer, Damaged, Network, Unavailable, Storage, SignedOut, ReadOnly, Unusable, TooLarge, Full, NotYours, Refused];
    let mut seen = std::collections::BTreeSet::new();
    for e in all {
        let (english, french) = (en(&e.said()), fr(&e.said()));
        assert!(!english.is_empty() && !french.is_empty() && english != french, "{e:?}: {english} / {french}");
        assert!(!english.contains('{') && !french.contains('{'), "{e:?} left a placeable: {english} / {french}");
        assert!(seen.insert(english.clone()), "{e:?} says what another says: {english}");
    }
    assert_eq!(
        en(&Expired.said()),
        "This shared design has expired: a share link works for 90 days. Ask the person who shared it for a new link. Nothing was kept on this device."
    );
    assert!(fr(&Expired.said()).starts_with("Cet aménagement partagé a expiré"), "{}", fr(&Expired.said()));
    assert!(en(&NotFound.said()).starts_with("No shared design has this link."));
    assert!(fr(&NotFound.said()).starts_with("Aucun aménagement partagé n’a ce lien."));
    assert!(en(&Full.said()).contains("50 share links"));
    assert!(en(&TooLarge.said()).contains("1 MB"));
}

/// Whatever stops a link from opening, the page says that nothing was kept on this device; a failure to make or delete
/// a link does not say so (a link it could not delete is still kept).
#[test]
fn every_failure_to_open_a_link_ends_saying_nothing_was_kept() {
    use ShareError::*;
    for e in [BadLink, NotFound, Expired, Deleted, Newer, Damaged, Network, Unavailable, Storage, Refused] {
        let (english, french) = (en(&e.said_opening()), fr(&e.said_opening()));
        assert!(english.ends_with(" Nothing was kept on this device."), "{e:?}: {english}");
        assert!(french.ends_with(" Rien n’a été conservé sur cet appareil."), "{e:?}: {french}");
    }
    for e in [Network, Unavailable, Refused] {
        assert!(!en(&e.said()).contains("Nothing was kept"), "{e:?} when making or deleting a link: {}", en(&e.said()));
    }
    assert_eq!(en(&Network.said_opening()), "The server could not be reached. Check your connection and try again. Nothing was kept on this device.");
}

#[test]
fn the_server_s_refusals_become_the_slice_s_errors() {
    let pairs = [
        (ShareFailure::SignedOut, ShareError::SignedOut),
        (ShareFailure::NotFound, ShareError::NotFound),
        (ShareFailure::Expired, ShareError::Expired),
        (ShareFailure::Deleted, ShareError::Deleted),
        (ShareFailure::NotYours, ShareError::NotYours),
        (ShareFailure::Full, ShareError::Full),
        (ShareFailure::TooLarge, ShareError::TooLarge),
        (ShareFailure::Newer, ShareError::Newer),
        (ShareFailure::Network, ShareError::Network),
        (ShareFailure::Unavailable, ShareError::Unavailable),
        (ShareFailure::Refused, ShareError::Refused),
    ];
    for (failure, error) in pairs {
        assert_eq!(ShareError::from(failure), error);
    }
}

#[test]
fn the_messages_are_the_same_in_both_languages() {
    let problems = crate::shared::i18n::tests::parity_problems(include_str!("i18n/en.ftl"), include_str!("i18n/fr.ftl"));
    assert!(problems.is_empty(), "{problems:?}");
}

// ---- opening a link ----

fn opening(locale: Locale, hash: &str) -> (Rc<OpenVm>, TestPorts) {
    let t = test_ports_full();
    *t.page.fragment.borrow_mut() = Some(hash.to_string());
    t.storage.set_now(1_800_000_000_000);
    (OpenVm::new(t.ports.clone(), crate::i18n_for(locale)), t)
}

fn everything(storage: &MemoryStorage) -> Vec<(String, Option<String>)> {
    let mut keys = storage.keys();
    keys.sort();
    keys.into_iter().map(|k| (k.clone(), storage.recall(&k))).collect()
}

#[test]
fn a_link_opens_its_design_as_a_copy_and_goes_to_its_map_read_only() {
    let (vm, t) = opening(Locale::En, TOKEN);
    vm.start();
    assert_eq!(t.share.calls(), [ShareCall::Open(TOKEN.into())], "no account needed: only the link is asked for");
    assert_eq!((vm.state(), vm.message()), (Opening::Working, "Opening the shared design…".to_string()));
    assert!(t.share.answer(ShareReply::Opened(Ok(share_text()))));
    assert_eq!(t.navigator.take_replaced(), ["map.html?project=s1&view=1"]);
    assert_eq!(t.storage.recall("cityloom-edits@s1").map(|t| serde_json::from_str::<Value>(&t).unwrap()), Some(edits()));
    assert_eq!(t.storage.recall("cityloom-restrictions@s1").map(|t| serde_json::from_str::<Value>(&t).unwrap()), Some(rules()));
    let project = crate::project::context::find(&*t.storage, "s1").unwrap();
    assert_eq!((project.name.as_str(), project.area), ("Plateau", default_area().to_query()));
    assert!(crate::project::ProjectStore::new(t.ports.storage.clone()).list().is_empty(), "not one of the person's projects");
    assert_eq!(t.storage.recall(crate::sync::model::SYNC_KEY), None, "nothing marked for the sync");
    // The same link again replaces its copy.
    let again = OpenVm::new(t.ports.clone(), crate::i18n_for(Locale::En));
    again.start();
    assert!(t.share.answer(ShareReply::Opened(Ok(share_text()))));
    assert_eq!(t.navigator.take_replaced(), ["map.html?project=s1&view=1"]);
    assert_eq!(crate::project::copies::list(&*t.storage).len(), 1);
}

#[test]
fn every_failure_to_open_a_link_is_said_and_keeps_nothing() {
    let newer = with(&|v| v["format_version"] = json!(2));
    let cases: Vec<(&str, Option<Result<String, ShareFailure>>, &str, &str)> = vec![
        ("a made-up link", Some(Err(ShareFailure::NotFound)), "No shared design has this link.", "Aucun aménagement partagé n’a ce lien."),
        ("an expired link", Some(Err(ShareFailure::Expired)), "This shared design has expired", "Cet aménagement partagé a expiré"),
        (
            "a deleted link",
            Some(Err(ShareFailure::Deleted)),
            "The person who shared this design deleted it.",
            "La personne qui a partagé cet aménagement l’a supprimé.",
        ),
        ("no network", Some(Err(ShareFailure::Network)), "The server could not be reached.", "Le serveur n’a pas pu être joint."),
        (
            "a server that cannot answer",
            Some(Err(ShareFailure::Unavailable)),
            "The server that keeps shared designs cannot answer now.",
            "Le serveur qui conserve",
        ),
        (
            "a newer format",
            Some(Ok(newer)),
            "This shared design was made with a newer CityLoom",
            "Cet aménagement partagé a été créé avec une version de CityLoom plus récente",
        ),
        ("a damaged body", Some(Ok("{\"kind\":\"cityloom-share\"".into())), "This shared design is damaged", "Cet aménagement partagé est endommagé"),
    ];
    for (name, answer, english, french) in cases {
        for (locale, words) in [(Locale::En, english), (Locale::FrCa, french)] {
            let (vm, t) = opening(locale, TOKEN);
            let before = everything(&t.storage);
            vm.start();
            assert!(t.share.answer(ShareReply::Opened(answer.clone().unwrap())), "{name}");
            assert!(matches!(vm.state(), Opening::Failed(_)), "{name}");
            assert!(vm.message().starts_with(words), "{name} ({locale:?}): {}", vm.message());
            assert_eq!(everything(&t.storage), before, "{name}: nothing kept");
            assert!(t.navigator.take_replaced().is_empty() && t.navigator.take().is_empty(), "{name}: the page stays");
        }
    }
}

#[test]
fn a_link_that_holds_no_whole_token_asks_the_server_nothing() {
    for hash in ["", "abc", &TOKEN[..40]] {
        let (vm, t) = opening(Locale::FrCa, hash);
        vm.start();
        assert!(t.share.calls().is_empty(), "{hash:?}");
        assert_eq!(vm.state(), Opening::Failed(ShareError::BadLink));
        assert!(vm.message().starts_with("Ce n’est pas un lien de partage complet."), "{}", vm.message());
    }
}

#[test]
fn a_copy_the_browser_cannot_keep_is_said_and_leaves_nothing() {
    let (vm, t) = opening(Locale::En, TOKEN);
    t.storage.refuse("cityloom-shared-copies");
    vm.start();
    assert!(t.share.answer(ShareReply::Opened(Ok(share_text()))));
    assert_eq!(vm.state(), Opening::Failed(ShareError::Storage));
    assert!(crate::project::copies::list(&*t.storage).is_empty());
    assert_eq!(t.storage.recall("cityloom-edits@s1").as_deref(), Some(""));
    assert!(t.navigator.take_replaced().is_empty());
}

// ---- making a project's links ----

fn sharing(locale: Locale) -> (Rc<ShareVm>, TestPorts) {
    let t = test_ports_full();
    t.storage.remember(EDITS_KEY, &edits().to_string());
    t.storage.remember(RESTRICTIONS_KEY, &rules().to_string());
    t.storage.set_now(1_800_000_000_000); // 2027-01-15T08:00:00Z
    let context = Context { project_name: "Plateau".into(), project_area: default_area().to_query(), app_version: "0.96.0".into() };
    (ShareVm::new(t.ports.clone(), crate::i18n_for(locale), context), t)
}

fn signed_in(vm: &Rc<ShareVm>, t: &TestPorts, yes: bool) {
    vm.load_account();
    let me = crate::shared::ports::Me { id: "u1".into(), display_name: "Ada".into(), store_on_server: None, identities: vec![] };
    let answer = if yes { Ok(me) } else { Err(crate::shared::ports::AccountError::SignedOut) };
    assert!(t.account.answer(crate::shared::ports::AccountReply::Me(answer)));
}

#[test]
fn signed_out_a_link_is_not_made_and_the_page_says_why() {
    let (vm, t) = sharing(Locale::En);
    assert_eq!(vm.account(), Account::Unknown);
    signed_in(&vm, &t, false);
    assert_eq!(vm.account(), Account::SignedOut);
    assert_eq!(vm.blocked().as_deref(), Some("Sign in to share this project: the server keeps shared copies for your account."));
    vm.create();
    assert!(t.share.calls().is_empty());
    assert_eq!(t.announcer.take(), ["Sign in to share this project: the server keeps shared copies for your account."]);
    let (vm, t) = sharing(Locale::FrCa);
    signed_in(&vm, &t, false);
    assert!(vm.blocked().unwrap().starts_with("Connectez-vous pour partager ce projet"));
}

/// Only a server that says nobody is signed in makes the page say to sign in; one that could not answer (down, no
/// network) lets a link be asked for, and the server's answer to that is said.
#[test]
fn a_server_that_cannot_say_who_is_signed_in_does_not_make_the_page_say_signed_out() {
    for failure in [crate::shared::ports::AccountError::Unavailable, crate::shared::ports::AccountError::Refused] {
        let (vm, t) = sharing(Locale::En);
        vm.load_account();
        assert!(t.account.answer(crate::shared::ports::AccountReply::Me(Err(failure))));
        assert_eq!(vm.account(), Account::Unchecked, "{failure:?}");
        assert_eq!(vm.blocked(), None, "{failure:?}");
        vm.create();
        assert!(matches!(t.share.calls().as_slice(), [ShareCall::Create(_)]), "{failure:?}: the server decides");
        t.share.answer(ShareReply::Made(Err(ShareFailure::SignedOut)));
        assert_eq!(vm.account(), Account::SignedOut);
        assert!(vm.blocked().is_some());
    }
}

#[test]
fn signed_in_a_link_is_made_of_the_project_kept_here_said_and_focused() {
    let (vm, t) = sharing(Locale::En);
    signed_in(&vm, &t, true);
    assert_eq!(vm.heading(), "Share Plateau");
    assert!(vm.blocked().is_none() && vm.links().is_empty());
    vm.create();
    assert!(vm.busy());
    let calls = t.share.calls();
    let [ShareCall::Create(body)] = calls.as_slice() else { panic!("{calls:?}") };
    let shared = parse_and_validate(body).expect("the server is sent a whole share");
    assert_eq!((shared.name.as_str(), shared.streets, shared.rules), ("Plateau", 1, 1));
    assert!(t.share.answer(ShareReply::Made(Ok(ShareMade { token: TOKEN.into(), expires_at: "2027-04-15T08:00:00.000Z".into() }))));
    assert!(!vm.busy());
    assert_eq!(t.announcer.take(), ["Share link created. It works until April 15, 2027."]);
    assert_eq!(vm.outcome(), Some(("Share link created. It works until April 15, 2027.".to_string(), false)));
    let links = vm.links();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].link, format!("{FAKE_ORIGIN}/share.html#{TOKEN}"));
    assert_eq!((links[0].expiry.as_str(), links[0].expired), ("Works until April 15, 2027", false));
    assert_eq!(vm.focus_request().1, Focus::Link(TOKEN.into()));
    // A later page of the project lists it, from this device's storage.
    let again = ShareVm::new(
        t.ports.clone(),
        crate::i18n_for(Locale::FrCa),
        Context { project_name: "P".into(), project_area: String::new(), app_version: String::new() },
    );
    assert_eq!(again.links()[0].expiry, "Fonctionne jusqu’au 15 avril 2027");
    t.storage.set_now(1_800_000_000_000 + 91 * 86_400_000);
    assert_eq!((again.links()[0].expiry.as_str(), again.links()[0].expired), ("A expiré le 15 avril 2027", true));
}

#[test]
fn a_link_is_copied_and_deleted_and_the_server_s_refusals_are_said() {
    let (vm, t) = sharing(Locale::En);
    signed_in(&vm, &t, true);
    vm.create();
    t.share.answer(ShareReply::Made(Ok(ShareMade { token: TOKEN.into(), expires_at: "2027-04-15T08:00:00.000Z".into() })));
    t.announcer.take();
    vm.copy(TOKEN);
    assert_eq!(*t.page.copied.borrow(), [format!("{FAKE_ORIGIN}/share.html#{TOKEN}")]);
    assert_eq!(t.announcer.take(), ["Link copied."]);
    t.page.clipboard_refuses.set(true);
    vm.copy(TOKEN);
    assert_eq!(t.announcer.take(), ["The link could not be copied. Select it and copy it yourself."]);
    vm.delete(TOKEN);
    assert_eq!(t.share.calls(), [ShareCall::Delete(TOKEN.into())]);
    assert!(t.share.answer(ShareReply::Deleted(Err(ShareFailure::NotYours))));
    assert_eq!(vm.links().len(), 1, "kept: the server did not delete it");
    assert_eq!(vm.outcome().map(|(_, error)| error), Some(true));
    vm.delete(TOKEN);
    assert!(t.share.answer(ShareReply::Deleted(Err(ShareFailure::Network))));
    assert_eq!(vm.outcome().map(|(text, _)| text), Some("The server could not be reached. Check your connection and try again.".into()));
    vm.delete(TOKEN);
    assert!(t.share.answer(ShareReply::Deleted(Ok(()))));
    assert!(vm.links().is_empty());
    assert_eq!(t.announcer.take().last().map(String::as_str), Some("Share link deleted. Anyone who opens it is told it was deleted."));
    assert_eq!(vm.focus_request().1, Focus::Create);
}

#[test]
fn a_link_the_server_no_longer_has_is_forgotten_when_deleted() {
    for gone in [ShareFailure::NotFound, ShareFailure::Expired, ShareFailure::Deleted] {
        let (vm, t) = sharing(Locale::En);
        signed_in(&vm, &t, true);
        vm.create();
        t.share.answer(ShareReply::Made(Ok(ShareMade { token: TOKEN.into(), expires_at: "2027-04-15T08:00:00.000Z".into() })));
        vm.delete(TOKEN);
        t.share.answer(ShareReply::Deleted(Err(gone)));
        assert!(vm.links().is_empty(), "{gone:?}");
    }
}

#[test]
fn the_server_s_refusals_of_a_new_link_are_said_and_nothing_is_kept() {
    for (failure, words) in [
        (ShareFailure::Full, "You have 50 share links already. Delete one before making another."),
        (ShareFailure::TooLarge, "This project is too large to share: a share holds at most 1 MB."),
        (ShareFailure::Unavailable, "The server that keeps shared designs cannot answer now. Try again later."),
        (ShareFailure::SignedOut, "Sign in to share this project: the server keeps shared copies for your account."),
    ] {
        let (vm, t) = sharing(Locale::En);
        signed_in(&vm, &t, true);
        vm.create();
        t.share.answer(ShareReply::Made(Err(failure)));
        assert_eq!(vm.outcome(), Some((words.to_string(), true)), "{failure:?}");
        assert!(vm.links().is_empty());
        assert_eq!(t.storage.recall(super::store::SHARES_KEY), None);
    }
}

#[test]
fn a_design_shown_read_only_is_not_shared_from_its_page() {
    let t = test_ports_full();
    let storage: Rc<dyn Storage> = Rc::new(crate::shared::scoped::ScopedStorage::new(t.ports.storage.clone(), "p1").only_to_read());
    let ports = crate::shared::ports::Ports { storage, ..t.ports.clone() };
    let context = Context { project_name: "P".into(), project_area: default_area().to_query(), app_version: "v".into() };
    let vm = ShareVm::new(ports, crate::i18n_for(Locale::En), context);
    signed_in(&vm, &t, true);
    assert_eq!(vm.blocked().as_deref(), Some("This design is open read-only: it cannot be shared from here."));
    vm.create();
    assert!(t.share.calls().is_empty());
}

#[test]
fn a_project_whose_work_cannot_be_read_is_not_sent() {
    let (vm, t) = sharing(Locale::En);
    signed_in(&vm, &t, true);
    t.storage.remember(EDITS_KEY, "{damaged");
    vm.create();
    assert!(t.share.calls().is_empty());
    assert_eq!(vm.outcome(), Some(("This project's work cannot be read, so it cannot be shared.".to_string(), true)));
}

// ---- the views ----

mod page {
    use super::*;
    use crate::share::view::{OpenView, ShareView};
    use crate::shared::testing::html;
    use leptos::prelude::*;

    #[test]
    fn the_sharing_page_says_what_a_share_is_and_lists_the_links_made_here() {
        let (vm, t) = sharing(Locale::En);
        signed_in(&vm, &t, true);
        let shown = vm.clone();
        let out = html(move || view! { <ShareView vm=shown/> }.into_any());
        assert!(out.contains(r#"id="sharing-heading""#) && out.contains("Share Plateau"), "{out}");
        assert!(out.contains("read-only and with no account") && out.contains("for 90 days"), "{out}");
        assert!(out.contains(r#"id="sharing-create""#) && out.contains("Create a share link"));
        assert!(out.contains("No share link has been made on this device for this project yet."));
        vm.create();
        t.share.answer(ShareReply::Made(Ok(ShareMade { token: TOKEN.into(), expires_at: "2027-04-15T08:00:00.000Z".into() })));
        let shown = vm.clone();
        let out = html(move || view! { <ShareView vm=shown/> }.into_any());
        assert!(out.contains(&format!(r#"id="sharing-link-{TOKEN}""#)) && out.contains(&format!("{FAKE_ORIGIN}/share.html#{TOKEN}")), "{out}");
        assert!(out.contains(&format!(r#"for="sharing-link-{TOKEN}""#)), "the field is labelled");
        assert!(out.contains("Works until April 15, 2027") && out.contains("Copy link") && out.contains("Delete link"));
        assert!(out.contains("Share link created."), "{out}");
    }

    #[test]
    fn signed_out_the_button_waits_and_the_reason_is_shown() {
        let (vm, t) = sharing(Locale::FrCa);
        signed_in(&vm, &t, false);
        let shown = vm.clone();
        let out = html(move || view! { <ShareView vm=shown/> }.into_any());
        let button = crate::shared::testing::button_tag(&out, "sharing-create");
        assert!(button.contains("disabled"), "{button}");
        assert!(out.contains("Connectez-vous pour partager ce projet"), "{out}");
    }

    #[test]
    fn the_opening_page_has_a_status_while_it_works_and_an_alert_when_it_cannot() {
        let (vm, t) = opening(Locale::FrCa, TOKEN);
        vm.start();
        let shown = vm.clone();
        let out = html(move || view! { <OpenView vm=shown/> }.into_any());
        assert!(out.contains(r#"role="status""#) && out.contains("Ouverture de l’aménagement partagé…"), "{out}");
        t.share.answer(ShareReply::Opened(Err(ShareFailure::NotFound)));
        let shown = vm.clone();
        let out = html(move || view! { <OpenView vm=shown/> }.into_any());
        let alert = out.split(r#"role="alert""#).nth(1).expect("an alert");
        assert!(alert.contains("Aucun aménagement partagé n’a ce lien."), "{out}");
        assert!(out.contains(r#"href="index.html""#) && out.contains("Aller à vos projets"));
    }
}
