//! The share page's two view-models (#221). `ShareVm` makes, lists, copies and deletes the share links of the project
//! the page is opened in (`share.html?project=<id>`, the project's ports). `OpenVm` opens the link the page's address
//! carries (`share.html#<token>`, the root ports): it fetches the share with no account, checks it whole, keeps it as a
//! copy (`project::copies`, never synced) and goes to its map, read-only. The views bind to these and decide nothing.

use std::rc::{Rc, Weak};

use leptos::prelude::*;

use crate::share::model::{self, ShareError, valid_token};
use crate::share::store::{Made, ShareStore};
use crate::share::text::{date_text, expiry_text};
use crate::shared::i18n::{Args, I18n};
use crate::shared::ports::Ports;
use crate::shared::said::{say, say_now};
use crate::shared::units::Units;
use crate::sync::model::stamp;

/// What the page is opened in, which this slice does not look up itself (the composition root does).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub project_name: String,
    /// The project's area, as its record names it (`Area::to_query`).
    pub project_area: String,
    /// The build's version, written into each share.
    pub app_version: String,
}

/// Whether someone is signed in, as far as the page knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Account {
    Unknown,
    SignedOut,
    SignedIn,
}

/// What the last command did, as data: the words follow a language switch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Created { expires_at: String },
    Copied,
    CopyFailed,
    Deleted,
    Failed(ShareError),
}

/// A link made here, as the page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkRow {
    pub token: String,
    /// The whole link, to copy and give.
    pub link: String,
    /// "Works until April 15, 2027" or "Expired on …".
    pub expiry: String,
    pub expired: bool,
}

/// Where focus should go after a command. The view moves it when `focus_request` changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Focus {
    None,
    /// The field holding the link just made.
    Link(String),
    /// The button that makes a link (after one was deleted).
    Create,
}

pub struct ShareVm {
    ports: Ports,
    i18n: Rc<I18n>,
    context: Context,
    store: ShareStore,
    account: ArcRwSignal<Account>,
    busy: ArcRwSignal<bool>,
    outcome: ArcRwSignal<Option<Outcome>>,
    version: ArcRwSignal<u32>,
    focus: ArcRwSignal<(u32, Focus)>,
}

impl ShareVm {
    /// The sharing of the project whose storage `ports` has.
    pub fn new(ports: Ports, i18n: Rc<I18n>, context: Context) -> Rc<ShareVm> {
        Rc::new(ShareVm {
            store: ShareStore::new(ports.storage.clone()),
            ports,
            i18n,
            context,
            account: ArcRwSignal::new(Account::Unknown),
            busy: ArcRwSignal::new(false),
            outcome: ArcRwSignal::new(None),
            version: ArcRwSignal::new(0),
            focus: ArcRwSignal::new((0, Focus::None)),
        })
    }

    /// The message `key`, for a view (tracked).
    pub fn word(&self, key: &str) -> String {
        self.i18n.tr(key, &Args::new())
    }

    /// The page's heading, naming the project (tracked).
    pub fn heading(&self) -> String {
        self.i18n.tr("share-heading", &Args::new().str("project", self.context.project_name.clone()))
    }

    /// What a share holds and who can see it (tracked).
    pub fn retention(&self) -> String {
        self.i18n.tr("share-retention", &Args::new().num("days", model::RETENTION_DAYS))
    }

    /// The address of the project's map, which the page leads back to: read-only where this page is.
    pub fn back_href(&self) -> String {
        crate::shared::project_id::link_for("map.html", &*self.ports.storage)
    }

    /// Asks the server who is signed in, once; the answer arrives later.
    pub fn load_account(self: &Rc<Self>) {
        let me = Rc::downgrade(self);
        self.ports.account.me(Box::new(move |answer| {
            if let Some(vm) = me.upgrade() {
                vm.account.set(if answer.is_ok() { Account::SignedIn } else { Account::SignedOut });
            }
        }));
    }

    /// Who is signed in, as far as the page knows (tracked).
    pub fn account(&self) -> Account {
        self.account.get()
    }

    /// Why a link cannot be made here, before one is asked for (tracked): signed out, or the design is read-only.
    pub fn blocked(&self) -> Option<String> {
        if self.ports.storage.read_only() {
            return Some(say(&self.i18n, Units::Metres, &ShareError::ReadOnly.said()));
        }
        (self.account.get() == Account::SignedOut).then(|| say(&self.i18n, Units::Metres, &ShareError::SignedOut.said()))
    }

    /// A request is on its way (tracked): the buttons wait.
    pub fn busy(&self) -> bool {
        self.busy.get()
    }

    /// What the last command did, in words (tracked), and whether it is an error.
    pub fn outcome(&self) -> Option<(String, bool)> {
        let outcome = self.outcome.get()?;
        Some(match outcome {
            Outcome::Failed(e) => (say(&self.i18n, Units::Metres, &e.said()), true),
            other => (self.outcome_words(&other, false), false),
        })
    }

    fn outcome_words(&self, outcome: &Outcome, now: bool) -> String {
        let tr = |key: &str, args: &Args| if now { self.i18n.tr_now(key, args) } else { self.i18n.tr(key, args) };
        match outcome {
            Outcome::Created { expires_at } => tr("share-created", &Args::new().str("date", date_text(&self.i18n, expires_at))),
            Outcome::Copied => tr("share-copied", &Args::new()),
            Outcome::CopyFailed => tr("share-copy-failed", &Args::new()),
            Outcome::Deleted => tr("share-deleted", &Args::new()),
            Outcome::Failed(e) => say_now(&self.i18n, Units::Metres, &e.said()),
        }
    }

    /// Keeps what a command did and says it.
    fn done(&self, outcome: Outcome) {
        self.ports.announcer.say(&self.outcome_words(&outcome, true));
        self.outcome.set(Some(outcome));
    }

    fn ask_focus(&self, to: Focus) {
        self.focus.update(|f| *f = (f.0.wrapping_add(1), to));
    }

    /// Where focus is asked to go, with a count so that asking twice for the same place is seen (tracked).
    pub fn focus_request(&self) -> (u32, Focus) {
        self.focus.get()
    }

    /// The links made here for this project, newest first (tracked).
    pub fn links(&self) -> Vec<LinkRow> {
        self.version.track();
        let now = stamp(self.ports.storage.now_ms());
        let origin = self.ports.page.origin();
        self.store
            .list()
            .into_iter()
            .rev()
            .map(|m| LinkRow {
                link: model::link(&origin, &m.token),
                expiry: expiry_text(&self.i18n, &m.expires_at, &now),
                expired: m.expires_at <= now,
                token: m.token,
            })
            .collect()
    }

    /// Makes a share of the project as it is now, if someone is signed in; the link is kept here, shown, and said.
    pub fn create(self: &Rc<Self>) {
        if self.busy.get_untracked() {
            return;
        }
        if self.ports.storage.read_only() {
            return self.done(Outcome::Failed(ShareError::ReadOnly));
        }
        if self.account.get_untracked() == Account::SignedOut {
            return self.done(Outcome::Failed(ShareError::SignedOut));
        }
        let now = stamp(self.ports.storage.now_ms());
        let body = match model::compose(&self.ports.storage, &self.context.project_name, &self.context.project_area, &self.context.app_version, &now) {
            Ok(body) => body,
            Err(e) => return self.done(Outcome::Failed(e)),
        };
        self.busy.set(true);
        let me: Weak<ShareVm> = Rc::downgrade(self);
        self.ports.share.create(
            body,
            Box::new(move |answer| {
                let Some(vm) = me.upgrade() else { return };
                vm.busy.set(false);
                match answer {
                    Ok(made) => {
                        vm.store.add(Made { token: made.token.clone(), created_at: now, expires_at: made.expires_at.clone() });
                        vm.version.update(|v| *v = v.wrapping_add(1));
                        vm.done(Outcome::Created { expires_at: made.expires_at });
                        vm.ask_focus(Focus::Link(made.token));
                    }
                    Err(failure) => {
                        if failure == crate::shared::ports::ShareFailure::SignedOut {
                            vm.account.set(Account::SignedOut);
                        }
                        vm.done(Outcome::Failed(failure.into()));
                    }
                }
            }),
        );
    }

    /// Puts the link of `token` on the clipboard and says whether it went.
    pub fn copy(self: &Rc<Self>, token: &str) {
        let link = model::link(&self.ports.page.origin(), token);
        let me: Weak<ShareVm> = Rc::downgrade(self);
        self.ports.page.copy_text(
            &link,
            Box::new(move |taken| {
                if let Some(vm) = me.upgrade() {
                    vm.done(if taken { Outcome::Copied } else { Outcome::CopyFailed });
                }
            }),
        );
    }

    /// Deletes the share behind `token` on the server and forgets it here; a share the server no longer has (not found,
    /// expired, deleted) is forgotten too.
    pub fn delete(self: &Rc<Self>, token: &str) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        let (me, token_kept): (Weak<ShareVm>, String) = (Rc::downgrade(self), token.to_string());
        self.ports.share.delete(
            token,
            Box::new(move |answer| {
                let Some(vm) = me.upgrade() else { return };
                vm.busy.set(false);
                use crate::shared::ports::ShareFailure as F;
                match answer {
                    Ok(()) | Err(F::NotFound | F::Expired | F::Deleted) => {
                        vm.store.remove(&token_kept);
                        vm.version.update(|v| *v = v.wrapping_add(1));
                        vm.done(Outcome::Deleted);
                        vm.ask_focus(Focus::Create);
                    }
                    Err(failure) => vm.done(Outcome::Failed(failure.into())),
                }
            }),
        );
    }
}

/// How opening a link stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opening {
    Working,
    Failed(ShareError),
}

pub struct OpenVm {
    ports: Ports,
    i18n: Rc<I18n>,
    state: ArcRwSignal<Opening>,
}

impl OpenVm {
    /// Opening the link of a page whose ports are the root's (the page is in no project).
    pub fn new(ports: Ports, i18n: Rc<I18n>) -> Rc<OpenVm> {
        Rc::new(OpenVm { ports, i18n, state: ArcRwSignal::new(Opening::Working) })
    }

    /// The message `key`, for a view (tracked).
    pub fn word(&self, key: &str) -> String {
        self.i18n.tr(key, &Args::new())
    }

    pub fn state(&self) -> Opening {
        self.state.get()
    }

    /// What the page says while it works, or why it could not open the link (tracked).
    pub fn message(&self) -> String {
        match self.state.get() {
            Opening::Working => self.word("share-opening"),
            Opening::Failed(e) => say(&self.i18n, Units::Metres, &e.said()),
        }
    }

    fn fail(&self, e: ShareError) {
        self.state.set(Opening::Failed(e));
    }

    /// Fetches the share the address names, checks it whole, keeps it as a copy and goes to its map, read-only. Any
    /// failure is said and nothing is kept.
    pub fn start(self: &Rc<Self>) {
        let token = self.ports.page.hash();
        if !valid_token(&token) {
            return self.fail(ShareError::BadLink);
        }
        let me: Weak<OpenVm> = Rc::downgrade(self);
        let kept_token = token.clone();
        self.ports.share.open(
            &token,
            Box::new(move |answer| {
                let Some(vm) = me.upgrade() else { return };
                match answer.map_err(ShareError::from).and_then(|text| model::parse_and_validate(&text)) {
                    Ok(shared) => vm.keep(&kept_token, &shared),
                    Err(e) => vm.fail(e),
                }
            }),
        );
    }

    fn keep(&self, token: &str, shared: &model::Shared) {
        let documents = shared.documents();
        let documents: Vec<(&str, &str)> = documents.iter().map(|(key, text)| (*key, text.as_str())).collect();
        match crate::project::copies::keep(&self.ports.storage, token, &shared.name, &shared.area, &documents) {
            Ok(id) => self.ports.navigator.replace(&format!("map.html?project={id}&{}", crate::shared::project_id::VIEW)),
            Err(_) => self.fail(ShareError::Storage),
        }
    }
}
