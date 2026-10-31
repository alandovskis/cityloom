//! The Export and Import commands of the Account page's design archive section. The view binds to this and decides
//! nothing; the files come and go through the `ArchiveFiles` port and the work through the `Storage` port.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::archive::model::{self, Archive, Choice, Status};
use crate::archive::text::{Differences, Outcome, differences_detail, differences_summary, outcome_text, outcome_text_now};
use crate::shared::i18n::I18n;
use crate::shared::ports::Ports;

/// Where focus is asked to go, so that the view can move it once the change is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    None,
    /// The result line, which is a focus target (`tabindex=-1`) so that a keyboard user lands on what happened.
    Result,
    /// "Keep mine", the safe button of the confirmation.
    Safe,
    /// "Import design…", where the person was when they cancelled.
    Import,
}

pub struct ArchiveVm {
    ports: Ports,
    i18n: Rc<I18n>,
    outcome: ArcRwSignal<Option<Outcome>>,
    waiting: ArcRwSignal<Option<Differences>>,
    held: RefCell<Option<Archive>>,
    focus: ArcRwSignal<(u32, Focus)>,
}

impl ArchiveVm {
    pub fn new(ports: Ports, i18n: Rc<I18n>) -> Rc<ArchiveVm> {
        Rc::new(ArchiveVm {
            ports,
            i18n,
            outcome: ArcRwSignal::new(None),
            waiting: ArcRwSignal::new(None),
            held: RefCell::new(None),
            focus: ArcRwSignal::new((0, Focus::None)),
        })
    }

    pub fn i18n(&self) -> Rc<I18n> {
        self.i18n.clone()
    }

    /// Where focus is asked to go, with a count so that asking twice for one place is seen; tracked.
    pub fn focus_request(&self) -> (u32, Focus) {
        self.focus.get()
    }

    fn ask_focus(&self, to: Focus) {
        self.focus.update(|f| *f = (f.0 + 1, to));
    }

    fn finish(&self, outcome: Outcome, focus: Focus) {
        self.ports.announcer.say(&outcome_text_now(&self.i18n, &outcome));
        self.outcome.set(Some(outcome));
        self.ask_focus(focus);
    }

    /// The last result in words; tracked.
    pub fn result_text(&self) -> Option<String> {
        self.outcome.get().map(|o| outcome_text(&self.i18n, &o))
    }

    /// Whether the last result is a refusal; tracked.
    pub fn result_is_error(&self) -> bool {
        matches!(self.outcome.get(), Some(Outcome::Failed(_)))
    }

    /// Whether an import waits for a choice; tracked.
    pub fn confirming(&self) -> bool {
        self.waiting.get().is_some()
    }

    pub fn summary_text(&self) -> Option<String> {
        self.waiting.get().map(|d| differences_summary(&self.i18n, &d, true))
    }

    pub fn detail_text(&self) -> Option<String> {
        self.waiting.get().map(|d| differences_detail(&self.i18n, &d))
    }

    /// Offers this device's design work as a file.
    pub fn export(&self) {
        let text = match model::export(&self.ports.storage) {
            Ok(text) => text,
            Err(e) => return self.finish(Outcome::Failed(e), Focus::Result),
        };
        // The text just made is read back by the rules an import uses: what is offered is what an import accepts.
        let count = match model::parse_and_validate(&text) {
            Ok(a) => a.streets() + a.junctions() + a.rules(),
            Err(e) => return self.finish(Outcome::Failed(e), Focus::Result),
        };
        if count == 0 {
            return self.finish(Outcome::ExportEmpty, Focus::Result);
        }
        let name = model::file_name(self.ports.storage.now_ms());
        if self.ports.files.save(&name, &text) {
            self.finish(Outcome::Exported { name, count }, Focus::Result);
        } else {
            self.finish(Outcome::ExportFailed, Focus::Result);
        }
    }

    /// Lets the person choose an archive and reads it.
    pub fn import(self: &Rc<Self>) {
        let vm = self.clone();
        self.ports.files.pick(Box::new(move |picked| {
            if let Some(picked) = picked {
                vm.import_text(&picked.text);
            }
        }));
    }

    /// Reads `text` as an archive. Nothing is written unless it is whole and the work it would change is readable;
    /// where places differ, nothing is written until the person chooses.
    pub fn import_text(&self, text: &str) {
        self.held.borrow_mut().take();
        self.waiting.set(None);
        let archive = match model::parse_and_validate(text) {
            Ok(a) => a,
            Err(e) => return self.finish(Outcome::Failed(e), Focus::Result),
        };
        let plan = match model::plan_import(&archive, &self.ports.storage) {
            Ok(p) => p,
            Err(e) => return self.finish(Outcome::Failed(e), Focus::Result),
        };
        if plan.count(Status::Conflict) == 0 {
            return self.run(&archive, Choice::KeepMine);
        }
        let differences = Differences::of(&plan);
        self.outcome.set(None);
        self.ports.announcer.say(&differences_summary(&self.i18n, &differences, false));
        self.waiting.set(Some(differences));
        *self.held.borrow_mut() = Some(archive);
        self.ask_focus(Focus::Safe);
    }

    fn run(&self, archive: &Archive, choice: Choice) {
        match model::apply(archive, &self.ports.storage, choice) {
            Ok(done) if done.added + done.replaced == 0 && done.kept == 0 => self.finish(Outcome::NothingNew, Focus::Result),
            Ok(done) => self.finish(Outcome::Imported(done), Focus::Result),
            Err(e) => self.finish(Outcome::Failed(e), Focus::Result),
        }
    }

    /// The person's answer to the confirmation. The plan is made again from what is stored now.
    pub fn choose(&self, choice: Choice) {
        let Some(archive) = self.held.borrow_mut().take() else { return };
        self.waiting.set(None);
        self.run(&archive, choice);
    }

    /// Leaves the import; nothing was written.
    pub fn cancel(&self) {
        if self.held.borrow_mut().take().is_none() {
            return;
        }
        self.waiting.set(None);
        self.finish(Outcome::Cancelled, Focus::Import);
    }
}
