//! Keeps what a page makes: a moment after the last change, and at once if the
//! page is about to be left before then. If it cannot be kept, the person is
//! told once.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::shared::ports::Scheduler;

/// How long after the last change it is kept, in milliseconds.
const DELAY_MS: u32 = 250;

pub struct Keeper {
    scheduler: Rc<dyn Scheduler>,
    keep: Rc<dyn Fn() -> bool>,
    on_fail: Rc<dyn Fn()>,
    pending: Cell<Option<u32>>,
    told: Cell<bool>,
    me: RefCell<Option<std::rc::Weak<Keeper>>>,
}

impl Keeper {
    pub fn new(scheduler: Rc<dyn Scheduler>, keep: impl Fn() -> bool + 'static, on_fail: impl Fn() + 'static) -> Rc<Keeper> {
        let k = Rc::new(Keeper {
            scheduler,
            keep: Rc::new(keep),
            on_fail: Rc::new(on_fail),
            pending: Cell::new(None),
            told: Cell::new(false),
            me: RefCell::new(None),
        });
        *k.me.borrow_mut() = Some(Rc::downgrade(&k));
        k
    }

    /// Something changed: keep it soon.
    pub fn touch(&self) {
        if let Some(id) = self.pending.take() {
            self.scheduler.cancel(id);
        }
        let me = self.me.borrow().clone();
        let id = self.scheduler.after(
            DELAY_MS,
            Box::new(move || {
                if let Some(k) = me.and_then(|w| w.upgrade()) {
                    k.pending.set(None);
                    k.run();
                }
            }),
        );
        self.pending.set(Some(id));
    }

    /// The page is being left: keep what is waiting to be kept.
    pub fn flush(&self) {
        if let Some(id) = self.pending.take() {
            self.scheduler.cancel(id);
            self.run();
        }
    }

    fn run(&self) {
        if !(self.keep)() && !self.told.replace(true) {
            (self.on_fail)();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::ManualScheduler;

    fn keeper(ok: bool) -> (Rc<Keeper>, Rc<ManualScheduler>, Rc<Cell<u32>>, Rc<Cell<u32>>) {
        let s = Rc::new(ManualScheduler::default());
        let (kept, failed) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let (k, f) = (kept.clone(), failed.clone());
        let keeper = Keeper::new(
            s.clone(),
            move || {
                k.set(k.get() + 1);
                ok
            },
            move || f.set(f.get() + 1),
        );
        (keeper, s, kept, failed)
    }

    #[test]
    fn a_change_is_kept_a_moment_later_and_a_burst_of_changes_is_kept_once() {
        let (k, s, kept, _) = keeper(true);
        k.touch();
        s.advance(200);
        assert_eq!(kept.get(), 0);
        k.touch();
        s.advance(200);
        assert_eq!(kept.get(), 0, "each change puts it off");
        s.advance(100);
        assert_eq!(kept.get(), 1);
        s.advance(1_000);
        assert_eq!(kept.get(), 1);
    }

    #[test]
    fn leaving_the_page_keeps_what_is_waiting_at_once_and_only_that() {
        let (k, s, kept, _) = keeper(true);
        k.flush();
        assert_eq!(kept.get(), 0, "nothing was waiting");
        k.touch();
        k.flush();
        assert_eq!(kept.get(), 1);
        s.advance(1_000);
        assert_eq!(kept.get(), 1, "and it is not kept again");
    }

    #[test]
    fn the_person_is_told_once_that_it_could_not_be_kept() {
        let (k, s, _, failed) = keeper(false);
        k.touch();
        s.advance(300);
        k.touch();
        s.advance(300);
        k.touch();
        k.flush();
        assert_eq!(failed.get(), 1);
    }
}
