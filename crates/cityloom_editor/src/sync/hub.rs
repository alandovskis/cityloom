//! The meeting point of the sync view-model and whatever is open on the page: the editors register a way to flush
//! what they hold and a way to hear what a sync changed; the stores say that a record was marked. A page is one
//! thread, so a thread-local is enough (tests run each on a thread of their own, and `reset` clears it).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::sync::model::{Change, Report};
use crate::sync::vm::SyncVm;

type Flush = Rc<dyn Fn()>;
type Applied = Rc<dyn Fn(&[Change])>;

#[derive(Default)]
struct Hub {
    listeners: Vec<(Flush, Applied)>,
    /// Who hears of a mark, by the storage the mark was made in.
    marked: Vec<(usize, Rc<dyn Fn()>)>,
    /// The page's sync view-model.
    sync: Option<Weak<SyncVm>>,
}

thread_local! {
    static HUB: RefCell<Hub> = RefCell::new(Hub::default());
}

/// An editor: `flush` writes what it holds to the stores (so it is marked before a sync reads it), `applied` hears
/// the records a sync changed. It lives as long as the page.
pub fn register(flush: impl Fn() + 'static, applied: impl Fn(&[Change]) + 'static) {
    HUB.with(|h| h.borrow_mut().listeners.push((Rc::new(flush), Rc::new(applied))));
}

/// An editor's whole hookup: `flush` writes what it holds, `applied` hears what a sync changed. The editor is held
/// weakly, so one that is gone is simply not called. The street and junction editors share this.
pub fn watch<T: 'static>(editor: &Rc<T>, flush: fn(&T), applied: fn(&T, &[Change])) {
    let (to_flush, to_hear) = (Rc::downgrade(editor), Rc::downgrade(editor));
    register(
        move || {
            if let Some(editor) = to_flush.upgrade() {
                flush(&editor);
            }
        },
        move |changes: &[Change]| {
            if let Some(editor) = to_hear.upgrade() {
                applied(&editor, changes);
            }
        },
    );
}

/// Asks every editor to write what it holds.
pub fn flush() {
    let all: Vec<Flush> = HUB.with(|h| h.borrow().listeners.iter().map(|(f, _)| f.clone()).collect());
    for f in all {
        f();
    }
}

/// Tells every editor what a sync changed. Nothing changed, nobody is told.
pub fn publish(changes: &[Change]) {
    if changes.is_empty() {
        return;
    }
    let all: Vec<Applied> = HUB.with(|h| h.borrow().listeners.iter().map(|(_, a)| a.clone()).collect());
    for a in all {
        a(changes);
    }
}

/// The sync view-model's ear for marks made in the storage `Storage::identity` names (every `BrowserStorage` is the
/// page's one; each fake is a device of its own, so two browsers on one thread, as in the tests, do not hear each
/// other's marks).
pub fn on_marked(key: usize, f: impl Fn() + 'static) {
    HUB.with(|h| h.borrow_mut().marked.push((key, Rc::new(f))));
}

/// A store marked a record in the storage `Storage::identity` names.
pub fn marked(key: usize) {
    let all: Vec<Rc<dyn Fn()>> = HUB.with(|h| h.borrow().marked.iter().filter(|(k, _)| *k == key).map(|(_, f)| f.clone()).collect());
    for f in all {
        f();
    }
}

/// The page's sync view-model.
pub fn install(vm: &Rc<SyncVm>) {
    HUB.with(|h| h.borrow_mut().sync = Some(Rc::downgrade(vm)));
}

pub fn current() -> Option<Rc<SyncVm>> {
    HUB.with(|h| h.borrow().sync.as_ref().and_then(Weak::upgrade))
}

/// The person just chose to store their work on the server: merge it with the account's, then tell `done` what
/// happened. The Account page calls this instead of reaching into the view-model.
pub fn enable(user: &str, done: Box<dyn FnOnce(Report)>) {
    match current() {
        Some(vm) => vm.enable(user, done),
        None => done(Report::default()),
    }
}

/// Storage on the server was turned off: stop, and clear the sync state (the browser's work stays).
pub fn disable() {
    if let Some(vm) = current() {
        vm.disable();
    }
}

/// The person signed out or the account is gone: stop; what waits stays marked.
pub fn stop() {
    if let Some(vm) = current() {
        vm.stop();
    }
}

/// Forgets every listener (tests).
#[cfg(test)]
pub fn reset() {
    HUB.with(|h| *h.borrow_mut() = Hub::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::model::{Id, Kind, Origin};
    use std::cell::Cell;

    #[test]
    fn editors_are_flushed_told_of_changes_and_the_sync_is_told_of_marks() {
        reset();
        let flushed = Rc::new(Cell::new(0));
        let told = Rc::new(RefCell::new(Vec::new()));
        for _ in 0..2 {
            let (f, t) = (flushed.clone(), told.clone());
            register(move || f.set(f.get() + 1), move |changes: &[Change]| t.borrow_mut().push(changes.len()));
        }
        flush();
        assert_eq!(flushed.get(), 2);
        publish(&[Change { id: Id::new(Kind::Street, "k"), origin: Origin::Remote }]);
        assert_eq!(*told.borrow(), [1, 1]);
        publish(&[]);
        assert_eq!(told.borrow().len(), 2, "nothing changed: nobody is told");
        marked(1); // nobody listens yet
        let marks = Rc::new(Cell::new(0));
        let m = marks.clone();
        on_marked(1, move || m.set(m.get() + 1));
        marked(1);
        marked(1);
        marked(2); // a mark in another storage
        assert_eq!(marks.get(), 2);
    }

    #[test]
    fn a_mark_is_heard_by_the_sync_of_the_storage_it_was_made_in_by_its_identity() {
        use crate::shared::ports::{MemoryStorage, Storage};
        reset();
        let (mine, theirs) = (MemoryStorage::default(), MemoryStorage::default());
        let heard = Rc::new(Cell::new(0));
        let h = heard.clone();
        on_marked(mine.identity(), move || h.set(h.get() + 1));
        crate::sync::model::mark(&theirs, Kind::Street, "k", false);
        assert_eq!(heard.get(), 0, "another device's mark");
        crate::sync::model::mark(&mine, Kind::Street, "k", false);
        assert_eq!(heard.get(), 1);
    }

    #[test]
    fn watch_reaches_a_live_editor_and_does_not_keep_a_dropped_one_alive() {
        reset();
        struct Editor {
            flushed: Cell<u32>,
            heard: Cell<usize>,
        }
        let editor = Rc::new(Editor { flushed: Cell::new(0), heard: Cell::new(0) });
        watch(&editor, |e| e.flushed.set(e.flushed.get() + 1), |e, changes| e.heard.set(e.heard.get() + changes.len()));
        flush();
        publish(&[Change { id: Id::new(Kind::Street, "k"), origin: Origin::Remote }]);
        assert_eq!((editor.flushed.get(), editor.heard.get()), (1, 1));
        let gone = Rc::downgrade(&editor);
        drop(editor);
        flush();
        publish(&[Change { id: Id::new(Kind::Street, "k"), origin: Origin::Remote }]);
        assert!(gone.upgrade().is_none(), "the hub holds an editor weakly");
    }

    #[test]
    fn a_listener_may_register_another_while_it_is_called() {
        reset();
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        register(
            move || {
                let c = c.clone();
                register(move || c.set(c.get() + 1), |_: &[Change]| {});
            },
            |_: &[Change]| {},
        );
        flush();
        flush();
        assert_eq!(count.get(), 1, "the one registered during the first flush runs in the second");
    }
}
