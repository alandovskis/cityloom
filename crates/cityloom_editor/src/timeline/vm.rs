//! The timeline page: a project's milestones, oldest first, to save, compare with the work now and restore. The view
//! binds to this and decides nothing. It works on the PROJECT's storage (`project::context::enter`).

use std::rc::{Rc, Weak};

use leptos::prelude::*;

use crate::shared::i18n::{Args, I18n};
use crate::shared::ports::Ports;
use crate::sync::hub;
use crate::timeline::model::{Draft, Milestone};
use crate::timeline::store::MilestoneStore;
use crate::timeline::text::{aspect_key, author_text, how_key, place_label, refused_key, summary_text, when_text};
use crate::timeline::work::{self, capture, records_of, summary_of};

/// What the page is opened in, which this slice does not look up itself (the composition root does).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    /// The name of the project the page is opened in.
    pub project_name: String,
    /// The build's version, kept with each milestone.
    pub app_version: String,
}

/// A milestone as the timeline shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub name: String,
    /// Saved by the app before a restore.
    pub auto: bool,
    /// When it was saved, in the page's language; empty where that is not known.
    pub when: String,
    /// "By Ada"; empty where nobody was signed in.
    pub author: String,
    /// "CityLoom 0.70.0".
    pub version: String,
    /// "2 streets, 0 junctions, 0 rules".
    pub summary: String,
}

/// One place that differs from a milestone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffRow {
    pub label: String,
    /// "Changed", "Added since" or "Removed since".
    pub how: String,
    /// What about it changed (a changed place only): "width", "lanes".
    pub aspects: Vec<String>,
}

/// A milestone set beside the work now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comparison {
    pub id: String,
    pub heading: String,
    /// How many places differ, or that none does.
    pub summary: String,
    pub rows: Vec<DiffRow>,
}

/// Where focus should go after what the person was using changed. The view moves it when `focus_request` changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Focus {
    None,
    Heading,
    CreateField,
    CompareButton(String),
    RestoreButton(String),
    DeleteButton(String),
    /// "Cancel", the safe button of the question before restoring.
    RestoreSafe,
    /// "Cancel", the safe button of the question before deleting.
    DeleteSafe,
}

pub struct TimelineVm {
    ports: Ports,
    i18n: Rc<I18n>,
    context: Context,
    store: MilestoneStore,
    /// Changed whenever the milestones or the work may have: read by the getters so the view follows.
    version: ArcRwSignal<u32>,
    /// The signed-in person's name, once known.
    author: ArcRwSignal<Option<String>>,
    create_error: ArcRwSignal<Option<&'static str>>,
    comparing: ArcRwSignal<Option<String>>,
    restoring: ArcRwSignal<Option<String>>,
    deleting: ArcRwSignal<Option<String>>,
    focus: ArcRwSignal<(u32, Focus)>,
}

impl TimelineVm {
    /// The timeline of the project whose storage `ports` has.
    pub fn new(ports: Ports, i18n: Rc<I18n>, context: Context) -> Rc<TimelineVm> {
        let vm = Rc::new(TimelineVm {
            store: MilestoneStore::new(ports.storage.clone()),
            ports,
            i18n,
            context,
            version: ArcRwSignal::new(0),
            author: ArcRwSignal::new(None),
            create_error: ArcRwSignal::new(None),
            comparing: ArcRwSignal::new(None),
            restoring: ArcRwSignal::new(None),
            deleting: ArcRwSignal::new(None),
            focus: ArcRwSignal::new((0, Focus::None)),
        });
        // What a sync brings or takes (a milestone made or deleted elsewhere, a place changed) shows at once.
        let me: Weak<TimelineVm> = Rc::downgrade(&vm);
        hub::register(
            || {},
            move |_| {
                if let Some(vm) = me.upgrade() {
                    vm.refresh();
                }
            },
        );
        vm
    }

    pub fn i18n(&self) -> Rc<I18n> {
        self.i18n.clone()
    }

    /// The message `key`, for a view: drawn again when the language is switched.
    pub fn word(&self, key: &str) -> String {
        self.i18n.tr(key, &Args::new())
    }

    /// The message `key` naming the milestone `name`, for a view.
    pub fn named(&self, key: &str, name: &str) -> String {
        self.i18n.tr(key, &Args::new().str("name", name.to_string()))
    }

    /// The page's heading, naming the project (tracked).
    pub fn heading(&self) -> String {
        self.i18n.tr("timeline-heading", &Args::new().str("project", self.context.project_name.clone()))
    }

    /// The address of the project's map, which the page leads back to.
    pub fn back_href(&self) -> String {
        crate::shared::project_id::link("map.html", self.ports.storage.project().as_deref())
    }

    /// What marks a milestone the app saved before a restore (tracked).
    pub fn auto_label(&self) -> String {
        self.word("timeline-auto")
    }

    /// Reads the milestones and the work again: another tab or the sync changed them.
    pub fn refresh(&self) {
        self.version.update(|v| *v = v.wrapping_add(1));
    }

    /// Where the milestones' author comes from: whoever is signed in. Asked once; the answer arrives later.
    pub fn load_author(self: &Rc<Self>) {
        let me = Rc::downgrade(self);
        self.ports.account.me(Box::new(move |answer| {
            if let (Some(vm), Ok(me)) = (me.upgrade(), answer) {
                vm.set_author(Some(me.display_name).filter(|n| !n.trim().is_empty()));
            }
        }));
    }

    pub fn set_author(&self, author: Option<String>) {
        self.author.set(author);
    }

    fn say(&self, key: &str, args: &Args) {
        self.ports.announcer.say(&self.i18n.tr_now(key, args));
    }

    fn ask_focus(&self, to: Focus) {
        self.focus.update(|f| *f = (f.0.wrapping_add(1), to));
    }

    /// Where focus is asked to go, with a count so that asking twice for the same place is seen; tracked.
    pub fn focus_request(&self) -> (u32, Focus) {
        self.focus.get()
    }

    // ---- the list ----

    /// The milestones, oldest first; tracked.
    pub fn rows(&self) -> Vec<Row> {
        self.version.track();
        self.store.list().into_iter().map(|m| self.row_of(&m)).collect()
    }

    fn row_of(&self, m: &Milestone) -> Row {
        Row {
            id: m.id.clone(),
            name: m.name.clone(),
            auto: m.auto,
            when: when_text(&self.i18n, &m.created_at),
            author: author_text(&self.i18n, m),
            version: self.i18n.tr("timeline-version", &Args::new().str("version", m.app_version.clone())),
            summary: summary_text(&self.i18n, &m.summary),
        }
    }

    fn name_of(&self, id: &str) -> Option<String> {
        self.store.get(id).map(|m| m.name)
    }

    // ---- save ----

    /// Keeps the work now as a milestone named `name`; says why where it is refused. True where it was kept.
    pub fn create(&self, name: &str) -> bool {
        let made = capture(&self.ports.storage).and_then(|snapshot| {
            let summary = summary_of(&records_of(&snapshot));
            self.store.create(Draft {
                name: name.to_string(),
                auto: false,
                app_version: self.context.app_version.clone(),
                author: self.author.get_untracked(),
                summary,
                snapshot,
            })
        });
        match made {
            Ok(milestone) => {
                self.create_error.set(None);
                self.refresh();
                self.say("timeline-created", &Args::new().str("name", milestone.name));
                true
            }
            Err(refused) => {
                self.create_error.set(Some(refused_key(refused)));
                self.say(refused_key(refused), &Args::new());
                self.ask_focus(Focus::CreateField);
                false
            }
        }
    }

    /// Why the last save was refused; tracked.
    pub fn create_error(&self) -> Option<String> {
        self.create_error.get().map(|key| self.word(key))
    }

    // ---- compare ----

    /// Sets milestone `id` beside the work now. False where there is no such milestone.
    pub fn compare(&self, id: &str) -> bool {
        if self.store.get(id).is_none() {
            return false;
        }
        self.comparing.set(Some(id.to_string()));
        // The comparison opens below the button, where focus stays: say what it found.
        if let Some(name) = self.name_of(id) {
            let count = capture(&self.ports.storage).ok().and_then(|now| self.store.snapshot(id).map(|then| work::compare(&then, &now).len()));
            self.ports.announcer.say(&self.compare_summary(&name, count, true));
        }
        true
    }

    pub fn close_compare(&self) {
        if let Some(id) = self.comparing.get_untracked() {
            self.comparing.set(None);
            self.ask_focus(Focus::CompareButton(id));
        }
    }

    /// The milestone being compared; tracked.
    pub fn comparing(&self) -> Option<String> {
        self.comparing.get()
    }

    /// The milestone set beside the work now, while one is; tracked.
    pub fn comparison(&self) -> Option<Comparison> {
        self.version.track();
        let id = self.comparing.get()?;
        let milestone = self.store.get(&id)?;
        let then = self.store.snapshot(&id)?;
        let name = Args::new().str("name", milestone.name.clone());
        let heading = self.i18n.tr("timeline-compare-heading", &name);
        let Ok(now) = capture(&self.ports.storage) else {
            return Some(Comparison { id, heading, summary: self.word("timeline-compare-unusable"), rows: Vec::new() });
        };
        let (before, after) = (records_of(&then), records_of(&now));
        let differences = work::compare(&then, &now);
        let rows: Vec<DiffRow> = differences
            .iter()
            .map(|d| {
                let key = (d.kind, d.key.clone());
                // A place gone is told from the milestone's record, any other from the work now.
                let record = if d.how == work::How::Removed { before.get(&key) } else { after.get(&key) };
                DiffRow {
                    label: place_label(&self.i18n, d.kind, &d.key, record),
                    how: self.word(how_key(d.how)),
                    aspects: d.aspects.iter().map(|a| self.word(aspect_key(*a))).collect(),
                }
            })
            .collect();
        let summary = self.compare_summary(&milestone.name, Some(rows.len()), false);
        Some(Comparison { id, heading, summary, rows })
    }

    /// How a comparison came out: `count` places differ (none, or the work could not be read). Said in the page's
    /// language: `now` for a command (nothing watched), else tracked.
    fn compare_summary(&self, name: &str, count: Option<usize>, now: bool) -> String {
        let say = |key: &str, args: &Args| if now { self.i18n.tr_now(key, args) } else { self.i18n.tr(key, args) };
        let args = Args::new().str("name", name.to_string());
        match count {
            None => say("timeline-compare-unusable", &Args::new()),
            Some(0) => say("timeline-compare-none", &args),
            Some(n) => say("timeline-compare-count", &args.num("n", n as i64)),
        }
    }

    // ---- the questions asked before restoring and before deleting ----

    /// Asks about milestone `id` in `asked`, closing the other question, and sends focus to the safe button. Nothing
    /// for a milestone that is not here.
    fn ask(&self, id: &str, (asked, other): (&ArcRwSignal<Option<String>>, &ArcRwSignal<Option<String>>), safe: Focus) {
        if self.store.get(id).is_none() {
            return;
        }
        other.set(None);
        asked.set(Some(id.to_string()));
        self.ask_focus(safe);
    }

    /// The question `key` about the milestone `asked` holds, while one is asked; tracked.
    fn question_of(&self, asked: &ArcRwSignal<Option<String>>, key: &str) -> Option<String> {
        self.version.track();
        let id = asked.get()?;
        let name = self.name_of(&id)?;
        Some(self.named(key, &name))
    }

    /// Answers no to the question `asked` holds: it closes and focus goes back to the button that asked.
    fn withdraw(&self, asked: &ArcRwSignal<Option<String>>, back_to: fn(String) -> Focus) {
        if let Some(id) = asked.get_untracked() {
            asked.set(None);
            self.ask_focus(back_to(id));
        }
    }

    // ---- restore ----

    /// Asks before putting the work back as milestone `id` held it. Nothing for a milestone that is not here.
    pub fn ask_restore(&self, id: &str) {
        self.ask(id, (&self.restoring, &self.deleting), Focus::RestoreSafe);
    }

    /// The milestone the question is about, while it is asked; tracked.
    pub fn restoring(&self) -> Option<String> {
        self.restoring.get()
    }

    /// The question asked before restoring; tracked.
    pub fn restore_question(&self) -> Option<String> {
        self.question_of(&self.restoring, "timeline-restore-confirm")
    }

    pub fn cancel_restore(&self) {
        self.withdraw(&self.restoring, Focus::RestoreButton);
    }

    /// Puts the work back as the milestone asked about held it, after keeping the work it replaces as a milestone
    /// of its own. False where nothing was asked or the restore was refused (nothing changed then).
    pub fn confirm_restore(&self) -> bool {
        let Some(id) = self.restoring.get_untracked() else { return false };
        self.restoring.set(None);
        let Some(name) = self.name_of(&id) else {
            self.say("timeline-gone", &Args::new());
            return false;
        };
        let backup_name = self.i18n.tr_now("timeline-backup-name", &Args::new().str("name", name.clone()));
        let author = self.author.get_untracked();
        let done = work::restore(&self.ports.storage, &id, &backup_name, &self.context.app_version, author.as_deref());
        self.refresh();
        self.ask_focus(Focus::RestoreButton(id));
        match done {
            Ok(restored) => {
                match restored.backup {
                    Some(backup) => self.say("timeline-restored", &Args::new().str("name", name).str("backup", backup.name)),
                    None => self.say("timeline-restored-same", &Args::new().str("name", name)),
                }
                true
            }
            Err(refused) => {
                self.say(refused_key(refused), &Args::new());
                false
            }
        }
    }

    // ---- delete ----

    /// Asks before deleting milestone `id`. Nothing for a milestone that is not here.
    pub fn ask_delete(&self, id: &str) {
        self.ask(id, (&self.deleting, &self.restoring), Focus::DeleteSafe);
    }

    /// The milestone the question is about, while it is asked; tracked.
    pub fn deleting(&self) -> Option<String> {
        self.deleting.get()
    }

    /// The question asked before deleting; tracked.
    pub fn delete_question(&self) -> Option<String> {
        self.question_of(&self.deleting, "timeline-delete-confirm")
    }

    pub fn cancel_delete(&self) {
        self.withdraw(&self.deleting, Focus::DeleteButton);
    }

    /// Deletes the milestone the question is about; the work is not touched. False where nothing was asked or it
    /// could not be deleted.
    pub fn confirm_delete(&self) -> bool {
        let Some(id) = self.deleting.get_untracked() else { return false };
        self.deleting.set(None);
        if self.comparing.get_untracked().as_deref() == Some(id.as_str()) {
            self.comparing.set(None);
        }
        let name = self.name_of(&id).unwrap_or_default();
        let done = self.store.delete(&id);
        self.refresh();
        match done {
            Ok(()) => self.say("timeline-deleted", &Args::new().str("name", name)),
            Err(refused) => self.say(refused_key(refused), &Args::new()),
        }
        self.ask_focus(Focus::Heading);
        done.is_ok()
    }
}
