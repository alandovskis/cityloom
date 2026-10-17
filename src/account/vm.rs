//! The Account page: who is signed in, the sign-ins linked, where the work is kept, and deleting the account.
//! The view binds to this and decides nothing.

use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::{Args, I18n};
use crate::shared::ports::{AccountError, Me, Ports, ProviderInfo};
use crate::sync::hub;
use crate::sync::model::Report;

/// Where the page comes back to from a provider.
const PAGE: &str = "%2Faccount.html";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Loading,
    SignedOut,
    Unavailable,
    Ready,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityRow {
    pub id: String,
    pub provider: String,
    pub label: Option<String>,
    pub can_unlink: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkOption {
    pub slug: String,
    pub name: String,
}

/// Where focus should go after what the person was using was removed or moved. The view-model cannot touch the page,
/// so the view moves focus when `AccountVm::focus_request` changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    None,
    /// The page's heading, which is always there.
    Page,
    /// The heading of the storage choice.
    Storage,
    /// The heading of the linked sign-ins, whose rows were just changed.
    Linked,
    /// "Delete account...", which shows when the confirmation is not open.
    DeleteButton,
    /// "Keep my account", the safe button of the confirmation.
    DeleteSafe,
    /// "Keep storing my work", the safe button of the confirmation of stopping.
    StopSafe,
}

pub struct AccountVm {
    ports: Ports,
    i18n: Rc<I18n>,
    state: ArcRwSignal<State>,
    me: ArcRwSignal<Option<Me>>,
    providers: ArcRwSignal<Vec<ProviderInfo>>,
    confirming: ArcRwSignal<bool>,
    stopping: ArcRwSignal<bool>,
    merged: ArcRwSignal<Option<Report>>,
    busy: Cell<bool>,
    /// Whether the storage choice was still to be made when the account was first read.
    choice_first: Cell<Option<bool>>,
    /// How many times focus was asked for, and where to.
    focus: ArcRwSignal<(u32, Focus)>,
}

impl AccountVm {
    pub fn new(ports: Ports, i18n: Rc<I18n>) -> Rc<AccountVm> {
        Rc::new(AccountVm {
            ports,
            i18n,
            state: ArcRwSignal::new(State::Loading),
            me: ArcRwSignal::new(None),
            providers: ArcRwSignal::new(Vec::new()),
            confirming: ArcRwSignal::new(false),
            stopping: ArcRwSignal::new(false),
            merged: ArcRwSignal::new(None),
            busy: Cell::new(false),
            choice_first: Cell::new(None),
            focus: ArcRwSignal::new((0, Focus::None)),
        })
    }

    pub fn i18n(&self) -> Rc<I18n> {
        self.i18n.clone()
    }

    fn say(&self, key: &str) {
        self.ports.announcer.say(&self.i18n.tr_now(key, &Args::new()));
    }

    fn ask_focus(&self, to: Focus) {
        self.focus.update(|f| *f = (f.0 + 1, to));
    }

    /// Where focus is asked to go, with a count so that asking twice for the same place is seen; tracked.
    pub fn focus_request(&self) -> (u32, Focus) {
        self.focus.get()
    }

    pub fn load(self: &Rc<Self>) {
        self.refresh(None);
        let vm = self.clone();
        self.ports.account.providers(Box::new(move |list| {
            if let Ok(list) = list {
                vm.providers.set(list);
            }
        }));
    }

    /// Reads who is signed in again; once it is shown, focus goes to `then`.
    fn refresh(self: &Rc<Self>, then: Option<Focus>) {
        let vm = self.clone();
        self.ports.account.me(Box::new(move |me| match me {
            Ok(me) => {
                let first = vm.choice_first.get().is_none();
                if first {
                    vm.choice_first.set(Some(me.store_on_server.is_none()));
                }
                vm.me.set(Some(me));
                vm.state.set(State::Ready);
                // A choice still to be made is what a new account is for: it is where focus starts.
                match then {
                    Some(to) => vm.ask_focus(to),
                    None if first && vm.choice_first() => vm.ask_focus(Focus::Storage),
                    None => {}
                }
            }
            Err(AccountError::SignedOut) => vm.signed_out(),
            // A failed re-read after a change leaves what is shown as it is (the change already said it failed) and
            // focus still goes where the person was; a page that was never read is unavailable.
            Err(_) if then.is_some() && vm.state.get_untracked() == State::Ready => vm.ask_focus(then.unwrap_or(Focus::Page)),
            Err(_) => vm.state.set(State::Unavailable),
        }));
    }

    /// The sign-in is over: what the person was using is gone, so focus goes to the page's heading.
    fn signed_out(&self) {
        self.me.set(None);
        self.confirming.set(false);
        self.state.set(State::SignedOut);
        self.ask_focus(Focus::Page);
    }

    pub fn state(&self) -> State {
        self.state.get()
    }

    pub fn name(&self) -> String {
        self.me.get().map(|m| m.display_name).unwrap_or_default()
    }

    pub fn providers(&self) -> Vec<ProviderInfo> {
        self.providers.get()
    }

    pub fn identities(&self) -> Vec<IdentityRow> {
        let Some(me) = self.me.get() else { return Vec::new() };
        let providers = self.providers.get();
        let several = me.identities.len() > 1;
        me.identities
            .iter()
            .map(|i| IdentityRow {
                id: i.id.clone(),
                provider: providers.iter().find(|p| p.slug == i.provider).map(|p| p.name.clone()).unwrap_or_else(|| i.provider.clone()),
                label: i.label.clone(),
                can_unlink: several,
            })
            .collect()
    }

    /// Every provider the server offers, those already linked too: a second account of one provider can be linked, and
    /// the labels tell them apart.
    pub fn link_options(&self) -> Vec<LinkOption> {
        self.providers.get().into_iter().map(|p| LinkOption { slug: p.slug, name: p.name }).collect()
    }

    pub fn storage(&self) -> Option<bool> {
        self.me.get().and_then(|m| m.store_on_server)
    }

    pub fn needs_choice(&self) -> bool {
        self.state.get() == State::Ready && self.storage().is_none()
    }

    /// Whether the choice was still to be made when the page loaded: it is shown first, and stays first.
    pub fn choice_first(&self) -> bool {
        self.choice_first.get().unwrap_or(false)
    }

    pub fn sign_in(&self, slug: &str) {
        self.ports.navigator.go(&format!("/auth/{slug}/start?return={PAGE}"));
    }

    pub fn link(&self, slug: &str) {
        self.ports.navigator.go(&format!("/auth/{slug}/start?link=1&return={PAGE}"));
    }

    /// Takes the answer of a change: the page falls to signed out when the sign-in is over, says it when the change
    /// failed (and reads the page again, so it shows what is true, and puts focus on `after_failure`), and otherwise
    /// runs `ok`.
    fn settled(self: &Rc<Self>, result: Result<(), AccountError>, after_failure: Focus, ok: impl FnOnce(&Rc<Self>)) {
        self.busy.set(false);
        match result {
            Ok(()) => ok(self),
            Err(AccountError::SignedOut) => self.signed_out(),
            Err(_) => {
                self.say("account-failed");
                self.confirming.set(false);
                self.refresh(Some(after_failure));
            }
        }
    }

    pub fn unlink(self: &Rc<Self>, identity_id: &str) {
        if self.busy.get() || !self.identities().iter().any(|i| i.id == identity_id && i.can_unlink) {
            return;
        }
        self.busy.set(true);
        let vm = self.clone();
        self.ports.account.unlink(
            identity_id,
            Box::new(move |r| {
                // The row that held focus is gone: it goes to the list's heading, once the list is read again.
                vm.clone().settled(r, Focus::Linked, |vm| {
                    vm.say("account-unlinked");
                    vm.refresh(Some(Focus::Linked));
                })
            }),
        );
    }

    pub fn choose_storage(self: &Rc<Self>, store: bool) {
        if self.busy.get() {
            return;
        }
        self.busy.set(true);
        let vm = self.clone();
        self.ports.account.set_storage(
            store,
            Box::new(move |r| {
                // The choice stays where it is and keeps focus; a failure reads the page again and returns to it.
                vm.clone().settled(r, Focus::Storage, move |vm| {
                    vm.me.update(|m| {
                        if let Some(m) = m {
                            m.store_on_server = Some(store);
                        }
                    });
                    if store {
                        vm.say("account-storage-set-server");
                        // Merge what this browser holds with the account's, then say how many places went each way.
                        let user = vm.me.get_untracked().map(|m| m.id).unwrap_or_default();
                        let after = vm.clone();
                        hub::enable(&user, Box::new(move |report| after.merged_with(report)));
                    } else {
                        vm.merged.set(None);
                        hub::disable();
                        vm.say("account-storage-set-device");
                    }
                })
            }),
        );
    }

    /// A press on a storage choice: stopping server storage deletes the server copy, so it is asked about first. True
    /// when the press is held back (the view stops the radio from changing) and the confirmation is shown.
    pub fn guard_stop(&self, store: bool) -> bool {
        if store || self.storage() != Some(true) || self.busy.get() {
            return false;
        }
        self.stopping.set(true);
        self.ask_focus(Focus::StopSafe);
        true
    }

    pub fn confirming_stop(&self) -> bool {
        self.stopping.get()
    }

    /// Closes the confirmation; focus goes back to the choice.
    pub fn cancel_stop(&self) {
        self.stopping.set(false);
        self.ask_focus(Focus::Storage);
    }

    pub fn confirm_stop(self: &Rc<Self>) {
        self.stopping.set(false);
        self.choose_storage(false);
    }

    /// What turning storage on did, said in the page's language: how many places went each way.
    pub fn merge_text(&self) -> Option<String> {
        self.merged.get().map(|r| self.merge_words(r))
    }

    fn merge_words(&self, r: Report) -> String {
        self.i18n.tr("sync-merged", &Args::new().num("sent", r.sent as i64).num("received", r.received as i64))
    }

    /// The merge ended. If it got to the end the counts are said and kept on the page; if it did not (no network) the
    /// work goes when the network returns and there is nothing more to say.
    fn merged_with(&self, report: Report) {
        if !report.ok {
            return;
        }
        self.merged.set(Some(report));
        self.ports.announcer.say(&self.i18n.tr_now("sync-merged", &Args::new().num("sent", report.sent as i64).num("received", report.received as i64)));
    }

    pub fn confirming_delete(&self) -> bool {
        self.confirming.get()
    }

    /// Opens the confirmation; focus goes to its safe button.
    pub fn ask_delete(&self) {
        self.confirming.set(true);
        self.ask_focus(Focus::DeleteSafe);
    }

    /// Closes it; focus goes back to "Delete account...".
    pub fn cancel_delete(&self) {
        self.confirming.set(false);
        self.ask_focus(Focus::DeleteButton);
    }

    pub fn confirm_delete(self: &Rc<Self>) {
        if self.busy.get() {
            return;
        }
        self.busy.set(true);
        let vm = self.clone();
        self.ports.account.delete(Box::new(move |r| {
            vm.clone().settled(r, Focus::DeleteButton, |vm| {
                hub::disable();
                vm.say("account-deleted");
                vm.ports.navigator.go("/index.html");
            })
        }));
    }

    pub fn sign_out(self: &Rc<Self>) {
        if self.busy.get() {
            return;
        }
        self.busy.set(true);
        let vm = self.clone();
        self.ports.account.sign_out(Box::new(move |r| {
            // A session already over (another tab signed out) is the outcome asked for.
            // It shows the signed-out page on the way to the home page.
            if r == Err(AccountError::SignedOut) {
                vm.busy.set(false);
                vm.signed_out();
                hub::stop();
                vm.ports.navigator.go("/index.html");
                return;
            }
            vm.clone().settled(r, Focus::Page, |vm| {
                hub::stop();
                vm.ports.navigator.go("/index.html");
            })
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;
    use crate::shared::ports::{AccountCall, AccountError, AccountReply, LinkedIdentity, Me, TestPorts, test_ports_full};

    fn new_vm(locale: Locale) -> (Rc<AccountVm>, TestPorts) {
        let t = test_ports_full();
        (AccountVm::new(t.ports.clone(), crate::i18n_for(locale)), t)
    }

    fn identity(id: &str, provider: &str, label: Option<&str>) -> LinkedIdentity {
        LinkedIdentity { id: id.into(), provider: provider.into(), label: label.map(str::to_string), created_at: String::new() }
    }

    fn me(store: Option<bool>, identities: Vec<LinkedIdentity>) -> Me {
        Me { id: "u".into(), display_name: "Ada".into(), store_on_server: store, identities }
    }

    fn providers() -> Vec<ProviderInfo> {
        ["osm", "google", "microsoft", "apple"]
            .iter()
            .map(|s| ProviderInfo {
                slug: s.to_string(),
                name: match *s {
                    "osm" => "OpenStreetMap",
                    "google" => "Google",
                    "microsoft" => "Microsoft",
                    _ => "Apple",
                }
                .into(),
            })
            .collect()
    }

    /// A page that has loaded: signed in with `identities`.
    fn ready(locale: Locale, store: Option<bool>, identities: Vec<LinkedIdentity>) -> (Rc<AccountVm>, TestPorts) {
        let (vm, t) = new_vm(locale);
        vm.load();
        t.account.answer(AccountReply::Me(Ok(me(store, identities))));
        t.account.answer(AccountReply::Providers(Ok(providers())));
        (vm, t)
    }

    #[test]
    fn loading_asks_who_and_which_providers_and_a_signed_in_person_is_ready() {
        let (vm, t) = new_vm(Locale::En);
        assert_eq!(vm.state(), State::Loading);
        vm.load();
        assert_eq!(t.account.calls(), [AccountCall::Me, AccountCall::Providers]);
        t.account.answer(AccountReply::Me(Ok(me(None, vec![identity("a", "google", Some("ada@example.org"))]))));
        t.account.answer(AccountReply::Providers(Ok(providers())));
        assert_eq!((vm.state(), vm.name()), (State::Ready, "Ada".to_string()));
    }

    #[test]
    fn signed_out_and_unavailable_are_told_apart() {
        let (vm, t) = new_vm(Locale::En);
        vm.load();
        t.account.answer(AccountReply::Me(Err(AccountError::SignedOut)));
        t.account.answer(AccountReply::Providers(Ok(providers())));
        assert_eq!((vm.state(), vm.providers().len()), (State::SignedOut, 4));
        let (vm, t) = vm_unavailable();
        assert_eq!(vm.state(), State::Unavailable);
        assert!(t.announcer.take().is_empty());
    }

    fn vm_unavailable() -> (Rc<AccountVm>, TestPorts) {
        let (vm, t) = new_vm(Locale::En);
        vm.load();
        t.account.answer(AccountReply::Me(Err(AccountError::Unavailable)));
        t.account.answer(AccountReply::Providers(Err(AccountError::Unavailable)));
        (vm, t)
    }

    #[test]
    fn the_identities_carry_their_provider_names_and_unlink_is_only_for_more_than_one() {
        let (vm, _) =
            ready(Locale::En, Some(true), vec![identity("a", "google", Some("ada@example.org")), identity("b", "osm", None), identity("c", "mystery", None)]);
        let rows = vm.identities();
        assert_eq!(rows.iter().map(|r| r.provider.as_str()).collect::<Vec<_>>(), ["Google", "OpenStreetMap", "mystery"]);
        assert!(rows.iter().all(|r| r.can_unlink));
        let (vm, _) = ready(Locale::En, Some(true), vec![identity("a", "google", None)]);
        assert!(!vm.identities()[0].can_unlink);
    }

    #[test]
    fn every_enabled_provider_is_offered_for_linking_including_those_already_linked() {
        // A second Google account can be linked to a person who already has one: the spec lets labels tell them apart.
        let (vm, _) = ready(Locale::En, Some(true), vec![identity("a", "google", None), identity("b", "osm", None)]);
        assert_eq!(vm.link_options().iter().map(|o| o.slug.as_str()).collect::<Vec<_>>(), ["osm", "google", "microsoft", "apple"]);
        assert_eq!(vm.link_options()[2].name, "Microsoft");
        // Only a provider the server does not offer is missing.
        let (vm, t) = new_vm(Locale::En);
        vm.load();
        t.account.answer(AccountReply::Me(Ok(me(Some(true), vec![identity("a", "google", None)]))));
        t.account.answer(AccountReply::Providers(Ok(providers().into_iter().filter(|p| p.slug == "google").collect())));
        assert_eq!(vm.link_options().iter().map(|o| o.slug.as_str()).collect::<Vec<_>>(), ["google"]);
    }

    #[test]
    fn linking_and_signing_in_go_to_the_servers_start() {
        let (vm, t) = ready(Locale::En, Some(true), vec![identity("a", "google", None)]);
        vm.link("apple");
        vm.sign_in("osm");
        assert_eq!(t.navigator.take(), ["/auth/apple/start?link=1&return=%2Faccount.html", "/auth/osm/start?return=%2Faccount.html"]);
    }

    #[test]
    fn unlinking_the_last_sign_in_is_ignored_and_another_is_asked_then_the_page_is_read_again() {
        let (vm, t) = ready(Locale::En, Some(true), vec![identity("a", "google", None)]);
        vm.unlink("a");
        assert!(t.account.calls().is_empty(), "the last one is never asked for");
        let (vm, t) = ready(Locale::En, Some(true), vec![identity("a", "google", None), identity("b", "osm", None)]);
        vm.unlink("a");
        vm.unlink("b");
        assert_eq!(t.account.calls(), [AccountCall::Unlink("a".into())], "one at a time");
        t.account.answer(AccountReply::Done(Ok(())));
        assert_eq!(t.announcer.take(), ["Sign-in unlinked."]);
        assert_eq!(t.account.calls(), [AccountCall::Me], "read again");
        t.account.answer(AccountReply::Me(Ok(me(Some(true), vec![identity("b", "osm", None)]))));
        assert_eq!(vm.identities().len(), 1);
    }

    #[test]
    fn a_refused_unlink_says_so_and_reads_the_page_again() {
        let (vm, t) = ready(Locale::En, Some(true), vec![identity("a", "google", None), identity("b", "osm", None)]);
        vm.unlink("a");
        t.account.answer(AccountReply::Done(Err(AccountError::LastIdentity)));
        assert_eq!(t.announcer.take(), ["That did not work. Try again."]);
        assert_eq!(t.account.calls(), [AccountCall::Me]);
    }

    #[test]
    fn a_sign_in_that_ended_meanwhile_shows_the_signed_out_page() {
        let (vm, t) = ready(Locale::En, Some(true), vec![identity("a", "google", None), identity("b", "osm", None)]);
        vm.unlink("a");
        t.account.answer(AccountReply::Done(Err(AccountError::SignedOut)));
        assert_eq!(vm.state(), State::SignedOut);
    }

    #[test]
    fn nothing_is_chosen_for_the_person_and_the_choice_is_kept_and_said() {
        let (vm, t) = ready(Locale::En, None, vec![identity("a", "google", None)]);
        assert_eq!((vm.storage(), vm.needs_choice()), (None, true));
        vm.choose_storage(true);
        assert_eq!(t.account.calls(), [AccountCall::SetStorage(true)]);
        assert_eq!(vm.storage(), None, "not until the server says so");
        t.account.answer(AccountReply::Done(Ok(())));
        assert_eq!((vm.storage(), vm.needs_choice()), (Some(true), false));
        assert_eq!(t.announcer.take(), ["Your choice is recorded: store your work on the server."]);
        vm.choose_storage(false);
        t.account.answer(AccountReply::Done(Ok(())));
        assert_eq!(vm.storage(), Some(false));
        assert_eq!(t.announcer.take(), ["Your choice is recorded: keep your work on this device only. The copy on the server was deleted."]);
    }

    #[test]
    fn the_choice_stays_where_it_was_first_shown_until_the_page_is_loaded_again() {
        let (vm, t) = ready(Locale::En, None, vec![identity("a", "google", None)]);
        assert!(vm.choice_first());
        vm.choose_storage(true);
        t.account.answer(AccountReply::Done(Ok(())));
        assert!(vm.choice_first(), "making the choice does not move it from under the person");
        let (vm, _) = ready(Locale::En, Some(false), vec![identity("a", "google", None)]);
        assert!(!vm.choice_first());
    }

    #[test]
    fn a_storage_choice_that_fails_changes_nothing_and_says_so() {
        let (vm, t) = ready(Locale::En, None, vec![identity("a", "google", None)]);
        vm.choose_storage(true);
        t.account.answer(AccountReply::Done(Err(AccountError::Unavailable)));
        assert_eq!(vm.storage(), None);
        assert_eq!(t.announcer.take(), ["That did not work. Try again."]);
    }

    #[test]
    fn deleting_needs_the_confirmation_in_the_page_and_then_leaves_for_the_home_page() {
        let (vm, t) = ready(Locale::En, Some(true), vec![identity("a", "google", None)]);
        assert!(!vm.confirming_delete());
        vm.ask_delete();
        assert!(vm.confirming_delete());
        assert!(t.account.calls().is_empty(), "asking is not deleting");
        vm.cancel_delete();
        assert!(!vm.confirming_delete());
        vm.ask_delete();
        vm.confirm_delete();
        vm.confirm_delete();
        assert_eq!(t.account.calls(), [AccountCall::Delete], "once");
        t.account.answer(AccountReply::Done(Ok(())));
        assert_eq!(t.announcer.take(), ["Your account was deleted."]);
        assert_eq!(t.navigator.take(), ["/index.html"]);
    }

    #[test]
    fn a_deletion_that_fails_goes_nowhere_and_closes_the_confirmation() {
        let (vm, t) = ready(Locale::En, Some(true), vec![identity("a", "google", None)]);
        vm.ask_delete();
        vm.confirm_delete();
        t.account.answer(AccountReply::Done(Err(AccountError::Unavailable)));
        assert!(!vm.confirming_delete());
        assert!(t.navigator.take().is_empty());
        assert_eq!(t.announcer.take(), ["That did not work. Try again."]);
    }

    #[test]
    fn signing_out_goes_to_the_home_page() {
        let (vm, t) = ready(Locale::En, Some(true), vec![identity("a", "google", None)]);
        vm.sign_out();
        t.account.answer(AccountReply::Done(Ok(())));
        assert_eq!(t.navigator.take(), ["/index.html"]);
    }

    #[test]
    fn the_announcements_follow_the_language() {
        let (vm, t) = ready(Locale::FrCa, None, vec![identity("a", "google", None)]);
        vm.choose_storage(false);
        t.account.answer(AccountReply::Done(Ok(())));
        assert_eq!(
            t.announcer.take(),
            ["Votre choix est enregistré\u{a0}: conserver votre travail sur cet appareil seulement. La copie sur le serveur a été supprimée."]
        );
    }

    fn focus(vm: &Rc<AccountVm>) -> Focus {
        vm.focus_request().1
    }

    fn reread(t: &TestPorts, store: Option<bool>, identities: Vec<LinkedIdentity>) {
        assert!(t.account.answer(AccountReply::Me(Ok(me(store, identities)))), "the page is read again");
    }

    #[test]
    fn a_first_load_with_nothing_chosen_asks_for_the_storage_choice_and_one_made_does_not() {
        let (vm, _) = ready(Locale::En, None, vec![identity("a", "google", None)]);
        assert_eq!(vm.focus_request(), (1, Focus::Storage));
        let (vm, _) = ready(Locale::En, Some(true), vec![identity("a", "google", None)]);
        assert_eq!(vm.focus_request(), (0, Focus::None));
    }

    #[test]
    fn a_storage_choice_that_fails_puts_focus_back_on_the_choice_even_when_the_page_cannot_be_read_again() {
        let (vm, t) = ready(Locale::En, None, vec![identity("a", "google", None)]);
        vm.choose_storage(true);
        t.account.answer(AccountReply::Done(Err(AccountError::Unavailable)));
        reread(&t, None, vec![identity("a", "google", None)]);
        assert_eq!(vm.focus_request(), (2, Focus::Storage));
        vm.choose_storage(true);
        t.account.answer(AccountReply::Done(Err(AccountError::Unavailable)));
        t.account.answer(AccountReply::Me(Err(AccountError::Unavailable)));
        assert_eq!((vm.focus_request(), vm.state()), ((3, Focus::Storage), State::Ready), "what is shown stays");
    }

    #[test]
    fn unlinking_puts_focus_on_the_list_whether_it_worked_or_not_and_whether_or_not_the_page_is_read_again() {
        let two = || vec![identity("a", "google", None), identity("b", "osm", None)];
        let (vm, t) = ready(Locale::En, Some(true), two());
        vm.unlink("a");
        t.account.answer(AccountReply::Done(Ok(())));
        reread(&t, Some(true), vec![identity("b", "osm", None)]);
        assert_eq!(vm.focus_request().1, Focus::Linked);
        let (vm, t) = ready(Locale::En, Some(true), two());
        vm.unlink("a");
        t.account.answer(AccountReply::Done(Err(AccountError::Unavailable)));
        reread(&t, Some(true), two());
        assert_eq!(vm.focus_request().1, Focus::Linked);
        let (vm, t) = ready(Locale::En, Some(true), two());
        vm.unlink("a");
        t.account.answer(AccountReply::Done(Ok(())));
        t.account.answer(AccountReply::Me(Err(AccountError::Unavailable)));
        assert_eq!((focus(&vm), vm.state()), (Focus::Linked, State::Ready));
    }

    #[test]
    fn the_deletion_confirmation_takes_focus_to_its_safe_button_and_gives_it_back() {
        let (vm, t) = ready(Locale::En, Some(true), vec![identity("a", "google", None)]);
        vm.ask_delete();
        assert_eq!(focus(&vm), Focus::DeleteSafe);
        vm.cancel_delete();
        assert_eq!(focus(&vm), Focus::DeleteButton);
        vm.ask_delete();
        vm.confirm_delete();
        t.account.answer(AccountReply::Done(Err(AccountError::Unavailable)));
        assert!(!vm.confirming_delete());
        reread(&t, Some(true), vec![identity("a", "google", None)]);
        assert_eq!(focus(&vm), Focus::DeleteButton);
    }

    #[test]
    fn a_sign_in_that_ended_during_a_change_shows_the_signed_out_page_with_focus_on_the_heading() {
        let one = || vec![identity("a", "google", None)];
        let (vm, t) = ready(Locale::En, None, one());
        vm.choose_storage(true);
        t.account.answer(AccountReply::Done(Err(AccountError::SignedOut)));
        assert_eq!((vm.state(), focus(&vm)), (State::SignedOut, Focus::Page));
        let (vm, t) = ready(Locale::En, Some(true), one());
        vm.ask_delete();
        vm.confirm_delete();
        t.account.answer(AccountReply::Done(Err(AccountError::SignedOut)));
        assert_eq!((vm.state(), focus(&vm), vm.confirming_delete()), (State::SignedOut, Focus::Page, false));
        let (vm, t) = ready(Locale::En, Some(true), one());
        vm.sign_out();
        t.account.answer(AccountReply::Done(Err(AccountError::SignedOut)));
        assert_eq!((vm.state(), focus(&vm)), (State::SignedOut, Focus::Page));
        assert_eq!(t.navigator.take(), ["/index.html"]);
    }

    fn sync_for(t: &TestPorts) -> Rc<crate::sync::SyncVm> {
        crate::sync::hub::reset();
        crate::sync::SyncVm::new(t.ports.clone(), crate::i18n_for(Locale::En))
    }

    #[test]
    fn choosing_the_server_merges_the_work_and_says_how_many_places_went_each_way() {
        use crate::shared::ports::{WorkAsk, WorkCall, WorkPage, WorkPushed, WorkReply, WorkResult, WorkStatus};
        let (vm, t) = new_vm(Locale::En);
        let _sync = sync_for(&t);
        vm.load();
        t.account.answer(AccountReply::Me(Ok(me(None, vec![identity("a", "google", None)]))));
        // a street edited here before storage was turned on
        let store = crate::city::store::CityStore::sample(t.ports.storage.clone());
        let mut s = store.open().street_editor(1, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(store.write_street(1, s));
        t.announcer.take();
        vm.choose_storage(true);
        t.account.answer(AccountReply::Done(Ok(())));
        assert_eq!(t.announcer.take(), ["Your choice is recorded: store your work on the server."]);
        assert_eq!(t.work.calls(), [WorkCall::Pull(WorkAsk::first(0))]);
        t.work.answer(WorkReply::Page(Ok(WorkPage { epoch: "e1".into(), rev: 0, more: false, records: vec![] })));
        let Some(WorkCall::Push { batch, .. }) = t.work.calls().into_iter().next() else { panic!("a push was expected") };
        let result = WorkResult { kind: batch[0].kind.clone(), key: batch[0].key.clone(), status: WorkStatus::Accepted, rev: 1, record: None };
        t.work.answer(WorkReply::Pushed(Ok(WorkPushed { rev: 1, results: vec![result] })));
        let merged = "Your work was merged with your account: 1 place sent, 0 places received.";
        assert_eq!(t.announcer.take(), [merged]);
        assert_eq!(vm.merge_text().as_deref(), Some(merged));
    }

    #[test]
    fn a_merge_that_could_not_finish_says_no_counts() {
        use crate::shared::ports::{WorkError, WorkReply};
        let (vm, t) = new_vm(Locale::En);
        let _sync = sync_for(&t);
        vm.load();
        t.account.answer(AccountReply::Me(Ok(me(None, vec![identity("a", "google", None)]))));
        vm.choose_storage(true);
        t.account.answer(AccountReply::Done(Ok(())));
        t.announcer.take();
        t.work.answer(WorkReply::Page(Err(WorkError::Unavailable))); // offline: it will go when the network returns
        assert_eq!((t.announcer.take(), vm.merge_text()), (vec![], None));
        assert_eq!(vm.storage(), Some(true), "the choice itself stands");
    }

    #[test]
    fn stopping_server_storage_is_held_back_until_confirmed_then_deletes_the_copy_and_clears_the_sync() {
        use crate::sync::model::{self, SyncDoc};
        let (vm, t) = new_vm(Locale::En);
        let _sync = sync_for(&t);
        vm.load();
        t.account.answer(AccountReply::Me(Ok(me(Some(true), vec![identity("a", "google", None)]))));
        t.account.answer(AccountReply::Providers(Ok(providers())));
        model::save(&*t.storage, &SyncDoc { user: Some("u".into()), rev: 5, ..SyncDoc::new() });
        assert!(vm.guard_stop(false) && vm.confirming_stop());
        assert!(t.account.calls().is_empty(), "nothing is asked of the server until the person confirms");
        vm.cancel_stop();
        assert_eq!((vm.confirming_stop(), vm.storage()), (false, Some(true)));
        assert!(vm.guard_stop(false));
        vm.confirm_stop();
        assert_eq!(t.account.calls(), [AccountCall::SetStorage(false)]);
        t.account.answer(AccountReply::Done(Ok(())));
        assert_eq!(vm.storage(), Some(false));
        assert_eq!(model::open(&*t.storage).unwrap(), SyncDoc::new(), "the device clears its sync state");
        assert_eq!(
            t.announcer.take().last().map(String::as_str),
            Some("Your choice is recorded: keep your work on this device only. The copy on the server was deleted.")
        );
    }

    #[test]
    fn only_stopping_a_stored_copy_is_held_back() {
        let (vm, t) = new_vm(Locale::En);
        vm.load();
        t.account.answer(AccountReply::Me(Ok(me(None, vec![identity("a", "google", None)]))));
        assert!(!vm.guard_stop(false), "nothing chosen yet: there is no server copy");
        assert!(!vm.guard_stop(true));
        vm.choose_storage(false);
        t.account.answer(AccountReply::Done(Ok(())));
        assert!(!vm.guard_stop(false), "already on this device only");
    }

    #[test]
    fn deleting_the_account_clears_the_sync_state_and_signing_out_stops_the_sync() {
        use crate::sync::model::{self, SyncDoc};
        let (vm, t) = new_vm(Locale::En);
        let sync = sync_for(&t);
        vm.load();
        t.account.answer(AccountReply::Me(Ok(me(Some(true), vec![identity("a", "google", None)]))));
        model::save(&*t.storage, &SyncDoc { user: Some("u".into()), rev: 5, ..SyncDoc::new() });
        vm.confirm_delete();
        t.account.answer(AccountReply::Done(Ok(())));
        assert_eq!(model::open(&*t.storage).unwrap(), SyncDoc::new());
        assert_eq!(sync.status(), crate::sync::Status::Off);
    }
}
