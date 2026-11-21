//! The share links made on this device of a project, ended before the project goes (#221): once a project is deleted,
//! its list of links (`cityloom-shares@<id>`, `store.rs`) is gone with it, and with it the only way to delete them.
//! The home page asks this through `project::model::ProjectLinks`, which the composition root (`lib.rs`) hands it.

use std::rc::Rc;

use crate::project::model::ProjectLinks;
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

/// The tokens of project `id`'s links that still work.
fn live(ports: &Ports, id: &str) -> Vec<String> {
    let now = stamp(ports.storage.now_ms());
    store_of(ports, id).list().into_iter().filter(|m| m.expires_at > now).map(|m| m.token).collect()
}

/// Why a link could not be deleted, as the home page says it.
fn why(failure: ShareFailure) -> Said {
    match failure {
        // Not "sign in to share": the person is deleting.
        ShareFailure::SignedOut => Said::new("share-error-end-signed-out"),
        other => ShareError::from(other).said(),
    }
}

/// Deletes `tokens` of project `id` one after the other from the first, forgetting each the server no longer has; the
/// first failure stops it.
fn end_from(ports: Ports, id: String, mut tokens: Vec<String>, done: Box<dyn FnOnce(Result<(), Said>)>) {
    if tokens.is_empty() {
        return done(Ok(()));
    }
    let token = tokens.remove(0);
    let (share, asked) = (ports.share.clone(), token.clone());
    share.delete(
        &asked,
        Box::new(move |answer| match answer {
            Ok(()) | Err(ShareFailure::NotFound | ShareFailure::Expired | ShareFailure::Deleted) => {
                store_of(&ports, &id).remove(&token);
                end_from(ports, id, tokens, done);
            }
            Err(failure) => done(Err(why(failure))),
        }),
    );
}

impl ProjectLinks for ProjectShares {
    fn live(&self, id: &str) -> usize {
        live(&self.ports, id).len()
    }

    fn end(&self, id: &str, done: Box<dyn FnOnce(Result<(), Said>)>) {
        end_from(self.ports.clone(), id.to_string(), live(&self.ports, id), done);
    }
}
