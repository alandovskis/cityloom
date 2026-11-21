//! The share links made on this device of a project, ended before the project goes (#221): once a project is deleted,
//! its list of links (`cityloom-shares@<id>`, `store.rs`) is gone with it, and with it the only way to delete them.
//! The home page asks this through `project::model::ProjectLinks`, which the composition root (`lib.rs`) hands it.

use std::rc::Rc;

use crate::project::model::{Ending, ProjectLinks};
use crate::share::model::ShareError;
use crate::share::store::ShareStore;
use crate::shared::ports::{Ports, ShareFailure};
use crate::shared::said::Said;
use crate::shared::scoped::ScopedStorage;
use crate::sync::model::stamp;

/// The links of the projects of the browser whose ROOT ports these are.
pub struct ProjectShares {
    ports: Ports,
}

impl ProjectShares {
    pub fn new(ports: Ports) -> ProjectShares {
        ProjectShares { ports }
    }
}

fn store_of(ports: &Ports, id: &str) -> ShareStore {
    ShareStore::new(Rc::new(ScopedStorage::new(ports.storage.clone(), id)))
}

/// The tokens of project `id`'s links that still work by this device's clock: what the question counts.
fn live(ports: &Ports, id: &str) -> Vec<String> {
    let now = stamp(ports.storage.now_ms());
    store_of(ports, id).list().into_iter().filter(|m| m.expires_at > now).map(|m| m.token).collect()
}

/// How many times one link is asked for before a run of 429s (the rate limit, `ShareFailure::Busy`) gives up: after
/// waits of 1, 2, 4, 8 and 16 seconds. The server's limit lets a burst through and then one request a second, so a
/// project with many links ends a link about each second once the burst is spent.
pub const MAX_TRIES: u32 = 6;

/// Why a link could not be deleted, as the home page says it.
fn why(failure: ShareFailure) -> Said {
    match failure {
        // Not "sign in to share": the person is deleting.
        ShareFailure::SignedOut => Said::new("share-error-end-signed-out"),
        other => ShareError::from(other).said(),
    }
}

/// A run of ending a project's links: the ports, the project, and who hears each step.
struct Run {
    ports: Ports,
    id: String,
    progress: Rc<dyn Fn(Ending)>,
}

/// Deletes `tokens` of the run's project one after the other from the first, forgetting each the server no longer has.
/// One refused by the rate limit is asked again after a wait that doubles (`MAX_TRIES` in all); any other failure, or
/// the last of those, stops it. `tried` is how many times the first has been asked already.
fn end_from(run: Run, mut tokens: Vec<String>, tried: u32, done: Box<dyn FnOnce(Result<(), Said>)>) {
    if tokens.is_empty() {
        return done(Ok(()));
    }
    let (token, share) = (tokens[0].clone(), run.ports.share.clone());
    share.delete(
        &tokens[0].clone(),
        Box::new(move |answer| match answer {
            Ok(()) | Err(ShareFailure::NotFound | ShareFailure::Expired | ShareFailure::Deleted) => {
                store_of(&run.ports, &run.id).remove(&token);
                tokens.remove(0);
                (run.progress)(Ending::Going);
                end_from(run, tokens, 0, done);
            }
            Err(ShareFailure::Busy) if tried + 1 < MAX_TRIES => {
                (run.progress)(Ending::Waiting);
                let scheduler = run.ports.scheduler.clone();
                scheduler.after(1000 << tried, Box::new(move || end_from(run, tokens, tried + 1, done)));
            }
            Err(failure) => done(Err(why(failure))),
        }),
    );
}

impl ProjectLinks for ProjectShares {
    fn live(&self, id: &str) -> usize {
        live(&self.ports, id).len()
    }

    fn listed(&self, id: &str) -> usize {
        store_of(&self.ports, id).worth_asking().len()
    }

    /// Asks for every link listed, not only those this device's clock thinks still work: the server decides, and one it
    /// has let expire (or deleted, or never had) counts as done. Only one expired more than `CLOCK_SKEW_MS` ago is left
    /// out: no clock is that wrong.
    fn end(&self, id: &str, progress: Box<dyn Fn(Ending)>, done: Box<dyn FnOnce(Result<(), Said>)>) {
        let tokens = store_of(&self.ports, id).worth_asking().into_iter().map(|m| m.token).collect();
        let run = Run { ports: self.ports.clone(), id: id.to_string(), progress: Rc::from(progress) };
        end_from(run, tokens, 0, done);
    }
}
