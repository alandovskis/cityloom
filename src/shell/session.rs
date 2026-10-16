//! The header's sign-in state: who is signed in, what the Sign in button offers, and what the address says happened
//! on the way back from a provider (`#account=<outcome>`). The view binds to this and decides nothing.

use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::{Args, I18n};
use crate::shared::ports::{AccountError, Ports, ProviderInfo};
use crate::shell::popover::PopoverViewModel;

#[derive(Clone, Debug, PartialEq)]
pub enum Session {
    Loading,
    SignedOut,
    Unavailable,
    SignedIn { name: String },
}

/// The outcomes the server puts in the address, and the message that says each.
const OUTCOMES: [(&str, &str); 7] = [
    ("new", "session-outcome-new"),
    ("signed-in", "session-outcome-signed-in"),
    ("linked", "session-outcome-linked"),
    ("already-linked", "session-outcome-already-linked"),
    ("conflict", "session-outcome-conflict"),
    ("cancelled", "session-outcome-cancelled"),
    ("failed", "session-outcome-failed"),
];

/// The message and the provider (as the server's slug) an address fragment such as `account=conflict&provider=google` means.
fn outcome_of(fragment: &str) -> Option<(&'static str, Option<String>)> {
    let mut parts = fragment.split('&');
    let outcome = parts.next()?.strip_prefix("account=")?;
    let key = OUTCOMES.iter().find(|(o, _)| *o == outcome)?.1;
    let provider = parts.find_map(|p| p.strip_prefix("provider=")).filter(|p| !p.is_empty()).map(str::to_string);
    // The sentence names the provider; without one it would read "This  account", so the generic sentence is said.
    let key = if key == "session-outcome-conflict" && provider.is_none() { "session-outcome-conflict-any" } else { key };
    Some((key, provider))
}

/// The brand name of a provider, which is not translated; a slug nobody knows is shown as it is.
fn provider_name(slug: &str) -> String {
    match slug {
        "osm" => "OpenStreetMap",
        "google" => "Google",
        "microsoft" => "Microsoft",
        "apple" => "Apple",
        other => other,
    }
    .to_string()
}

pub fn encode_component(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

pub struct SessionVm {
    ports: Ports,
    i18n: Rc<I18n>,
    state: ArcRwSignal<Session>,
    providers: ArcRwSignal<Vec<ProviderInfo>>,
    popover: Rc<PopoverViewModel>,
    started: Cell<bool>,
}

impl SessionVm {
    pub fn new(ports: Ports, i18n: Rc<I18n>) -> Rc<SessionVm> {
        let popover = PopoverViewModel::new(i18n.clone(), "session-sign-in", "session-sign-in");
        Rc::new(SessionVm {
            ports,
            i18n,
            state: ArcRwSignal::new(Session::Loading),
            providers: ArcRwSignal::new(Vec::new()),
            popover,
            started: Cell::new(false),
        })
    }

    fn say(&self, key: &str) {
        self.ports.announcer.say(&self.i18n.tr_now(key, &Args::new()));
    }

    pub fn start(self: &Rc<Self>) {
        if self.started.replace(true) {
            return;
        }
        if let Some((key, provider)) = self.ports.page.take_fragment().as_deref().and_then(outcome_of) {
            let args = Args::new().str("provider", provider.as_deref().map(provider_name).unwrap_or_default());
            self.ports.announcer.say(&self.i18n.tr_now(key, &args));
        }
        let vm = self.clone();
        self.ports.account.me(Box::new(move |me| match me {
            Ok(me) => vm.state.set(Session::SignedIn { name: me.display_name }),
            Err(AccountError::SignedOut) => vm.load_providers(),
            Err(_) => vm.state.set(Session::Unavailable),
        }));
    }

    fn load_providers(self: &Rc<Self>) {
        let vm = self.clone();
        self.ports.account.providers(Box::new(move |list| match list {
            Ok(list) => {
                vm.providers.set(list);
                vm.state.set(Session::SignedOut);
            }
            Err(_) => vm.state.set(Session::Unavailable),
        }));
    }

    pub fn state(&self) -> Session {
        self.state.get()
    }

    pub fn providers(&self) -> Vec<ProviderInfo> {
        self.providers.get()
    }

    pub fn visible(&self) -> bool {
        match self.state.get() {
            Session::Loading => false,
            Session::SignedOut => !self.providers.get().is_empty(),
            Session::Unavailable | Session::SignedIn { .. } => true,
        }
    }

    pub fn label(&self) -> String {
        match self.state.get() {
            Session::SignedIn { name } => name,
            _ => self.i18n.tr("session-sign-in", &Args::new()),
        }
    }

    pub fn title(&self) -> String {
        match self.state.get() {
            Session::Unavailable => self.i18n.tr("session-unavailable", &Args::new()),
            _ => String::new(),
        }
    }

    pub fn i18n(&self) -> Rc<I18n> {
        self.i18n.clone()
    }

    pub fn popover(&self) -> Rc<PopoverViewModel> {
        self.popover.clone()
    }

    pub fn sign_in(&self, slug: &str) {
        self.ports.navigator.go(&format!("/auth/{slug}/start?return={}", encode_component(&self.ports.page.path())));
    }

    pub fn sign_out(self: &Rc<Self>) {
        self.popover.close();
        let vm = self.clone();
        self.ports.account.sign_out(Box::new(move |done| match done {
            Ok(()) => {
                vm.state.set(Session::SignedOut);
                vm.say("session-signed-out");
                // The Account page has nothing to show a signed-out person: leave it for the home page.
                if vm.ports.page.path().starts_with("/account.html") {
                    vm.ports.navigator.go("/index.html");
                } else {
                    vm.load_providers();
                }
            }
            Err(_) => vm.say("session-sign-out-failed"),
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;
    use crate::shared::ports::{AccountCall, AccountError, AccountReply, LinkedIdentity, Me, TestPorts, test_ports_full};

    fn vm(locale: Locale) -> (Rc<SessionVm>, TestPorts) {
        let t = test_ports_full();
        (SessionVm::new(t.ports.clone(), crate::i18n_for(locale)), t)
    }

    fn me(name: &str) -> Me {
        Me {
            id: "u".into(),
            display_name: name.into(),
            store_on_server: None,
            identities: vec![LinkedIdentity { id: "ab".into(), provider: "google".into(), label: None, created_at: String::new() }],
        }
    }

    fn providers(slugs: &[&str]) -> Vec<ProviderInfo> {
        slugs.iter().map(|s| ProviderInfo { slug: s.to_string(), name: s.to_uppercase() }).collect()
    }

    #[test]
    fn it_asks_who_is_signed_in_once_and_is_hidden_until_it_knows() {
        let (vm, t) = vm(Locale::En);
        assert_eq!((vm.state(), vm.visible()), (Session::Loading, false));
        vm.start();
        vm.start();
        assert_eq!(t.account.calls(), [AccountCall::Me], "once, however often it is started");
        assert!(!vm.visible());
    }

    #[test]
    fn a_signed_in_person_sees_their_name_and_no_provider_list_is_asked() {
        let (vm, t) = vm(Locale::En);
        vm.start();
        t.account.answer(AccountReply::Me(Ok(me("Ada"))));
        assert_eq!((vm.state(), vm.label(), vm.visible(), vm.title()), (Session::SignedIn { name: "Ada".into() }, "Ada".into(), true, String::new()));
        assert!(t.account.calls().is_empty());
    }

    #[test]
    fn a_signed_out_person_is_offered_the_providers_the_server_lists() {
        let (vm, t) = vm(Locale::En);
        vm.start();
        t.account.answer(AccountReply::Me(Err(AccountError::SignedOut)));
        assert_eq!(t.account.calls(), [AccountCall::Providers]);
        assert!(!vm.visible(), "not until the providers are known");
        t.account.answer(AccountReply::Providers(Ok(providers(&["google", "osm"]))));
        assert_eq!((vm.state(), vm.visible(), vm.label()), (Session::SignedOut, true, "Sign in".into()));
        assert_eq!(vm.providers().iter().map(|p| p.slug.as_str()).collect::<Vec<_>>(), ["google", "osm"]);
    }

    #[test]
    fn with_no_provider_to_offer_there_is_no_button() {
        let (vm, t) = vm(Locale::En);
        vm.start();
        t.account.answer(AccountReply::Me(Err(AccountError::SignedOut)));
        t.account.answer(AccountReply::Providers(Ok(vec![])));
        assert_eq!((vm.state(), vm.visible()), (Session::SignedOut, false));
    }

    #[test]
    fn a_server_that_cannot_answer_marks_the_account_unavailable_without_a_word_said() {
        for failure in [AccountError::Unavailable, AccountError::Refused, AccountError::NotFound] {
            let (vm, t) = vm(Locale::En);
            vm.start();
            t.account.answer(AccountReply::Me(Err(failure)));
            assert_eq!((vm.state(), vm.visible()), (Session::Unavailable, true), "{failure:?}");
            assert_eq!(vm.title(), "Sign-in is not available right now.");
            assert!(t.announcer.take().is_empty(), "nothing interrupts the person");
            assert!(t.account.calls().is_empty());
        }
        let (vm, t) = vm(Locale::En);
        vm.start();
        t.account.answer(AccountReply::Me(Err(AccountError::SignedOut)));
        t.account.answer(AccountReply::Providers(Err(AccountError::Unavailable)));
        assert_eq!(vm.state(), Session::Unavailable);
    }

    #[test]
    fn choosing_a_provider_goes_to_its_start_with_the_page_to_come_back_to() {
        let (vm, t) = vm(Locale::En);
        *t.page.path.borrow_mut() = "/street.html?street=7".into();
        vm.sign_in("google");
        assert_eq!(t.navigator.take(), ["/auth/google/start?return=%2Fstreet.html%3Fstreet%3D7"]);
    }

    #[test]
    fn encoding_leaves_only_the_unreserved_characters() {
        assert_eq!(encode_component("/a b?c=d&e#é"), "%2Fa%20b%3Fc%3Dd%26e%23%C3%A9");
        assert_eq!(encode_component("AZaz09-_.~"), "AZaz09-_.~");
    }

    #[test]
    fn every_outcome_in_the_address_is_announced_once_in_the_language_of_the_page_and_removed() {
        let cases = [
            ("account=new", "Signed in. Choose where your work is kept.", "Vous êtes connecté. Choisissez où votre travail est conservé."),
            ("account=signed-in", "Signed in.", "Vous êtes connecté."),
            ("account=linked", "Account linked.", "Compte associé."),
            ("account=already-linked", "That account is already linked to yours.", "Ce compte est déjà associé au vôtre."),
            (
                "account=conflict&provider=google",
                "This Google account is already linked to another CityLoom account.",
                "Ce compte Google est déjà associé à un autre compte CityLoom.",
            ),
            (
                "account=conflict&provider=fake",
                "This fake account is already linked to another CityLoom account.",
                "Ce compte fake est déjà associé à un autre compte CityLoom.",
            ),
            // An address with no provider in it (an older server, a hand-edited link) gets the generic sentence, not "This  account".
            ("account=conflict", "This account is already linked to another CityLoom account.", "Ce compte est déjà associé à un autre compte CityLoom."),
            (
                "account=conflict&provider=",
                "This account is already linked to another CityLoom account.",
                "Ce compte est déjà associé à un autre compte CityLoom.",
            ),
            ("account=cancelled", "Sign-in cancelled.", "Connexion annulée."),
            ("account=failed", "Sign-in failed. Try again.", "La connexion a échoué. Réessayez."),
        ];
        for (fragment, en, fr) in cases {
            for (locale, expected) in [(Locale::En, en), (Locale::FrCa, fr)] {
                let (vm, t) = vm(locale);
                *t.page.fragment.borrow_mut() = Some(fragment.into());
                vm.start();
                assert_eq!(t.announcer.take(), [expected], "{fragment} in {locale:?}");
                assert_eq!(t.page.fragment.borrow().clone(), None, "the fragment is removed");
            }
        }
    }

    #[test]
    fn a_fragment_that_is_not_an_outcome_is_left_unsaid() {
        for fragment in ["account=surprise", "street=7", "account", "", "account=&provider=x"] {
            let (vm, t) = vm(Locale::En);
            *t.page.fragment.borrow_mut() = Some(fragment.into());
            vm.start();
            assert!(t.announcer.take().is_empty(), "{fragment:?}");
        }
    }

    #[test]
    fn the_outcome_is_announced_even_when_the_server_does_not_answer() {
        let (vm, t) = vm(Locale::En);
        *t.page.fragment.borrow_mut() = Some("account=cancelled".into());
        vm.start();
        t.account.answer(AccountReply::Me(Err(AccountError::Unavailable)));
        assert_eq!(t.announcer.take(), ["Sign-in cancelled."]);
    }

    #[test]
    fn signing_out_ends_the_session_and_says_so_and_a_failure_keeps_the_person_signed_in() {
        let (vm, t) = vm(Locale::En);
        vm.start();
        t.account.answer(AccountReply::Me(Ok(me("Ada"))));
        vm.popover().toggle();
        vm.sign_out();
        assert_eq!(t.account.calls(), [AccountCall::SignOut]);
        assert!(!vm.popover().is_open_now(), "the menu closes");
        t.account.answer(AccountReply::Done(Err(AccountError::Unavailable)));
        assert_eq!(vm.state(), Session::SignedIn { name: "Ada".into() });
        assert_eq!(t.announcer.take(), ["Sign-out failed. Try again."]);
        vm.sign_out();
        t.account.answer(AccountReply::Done(Ok(())));
        assert_eq!(vm.state(), Session::SignedOut);
        assert_eq!(t.announcer.take(), ["Signed out."]);
        assert_eq!(t.account.calls(), [AccountCall::Providers], "the providers to sign in with are asked for");
    }

    #[test]
    fn signing_out_on_the_account_page_leaves_it_for_the_home_page() {
        let (vm, t) = vm(Locale::En);
        *t.page.path.borrow_mut() = "/account.html".into();
        vm.start();
        t.account.answer(AccountReply::Me(Ok(me("Ada"))));
        vm.sign_out();
        t.account.answer(AccountReply::Done(Ok(())));
        assert_eq!(t.navigator.take(), ["/index.html"]);
        assert!(t.account.calls().is_empty(), "no providers are asked for on the way out");
    }

    #[test]
    fn the_words_follow_the_language() {
        let (vm, _) = vm(Locale::FrCa);
        assert_eq!(vm.label(), "Se connecter");
    }
}
