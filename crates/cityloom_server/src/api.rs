//! `/api/*`: who I am, the storage choice, unlinking, deleting.
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{any, delete, get, put};
use axum::{Json, Router};
use axum_extra::extract::CookieJar;
use serde::Serialize;
use uuid::Uuid;

use crate::accounts::{self, RepoError, UnlinkError};
use crate::app::{Account, json_error, require_origin, require_ready};
use crate::session::{self, Read};

mod work;

pub(crate) fn store_error(e: RepoError) -> Response {
    match e {
        RepoError::Unavailable => json_error(StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
        RepoError::Gone => json_error(StatusCode::UNAUTHORIZED, "signed-out"),
        _ => json_error(StatusCode::INTERNAL_SERVER_ERROR, "corrupt"),
    }
}

/// The signed-in user, and the jar to answer with: the session cookie set again when it was refreshed, cleared when it was stale.
pub(crate) async fn authed(a: &Account, jar: CookieJar) -> Result<(Uuid, CookieJar), Response> {
    let Some(token) = jar.get(session::COOKIE).map(|c| c.value().to_string()) else {
        return Err(json_error(StatusCode::UNAUTHORIZED, "signed-out"));
    };
    match session::read(&*a.repo, &token, a.clock.now()).await {
        Ok(Read::Valid(user)) => Ok((user, jar)),
        Ok(Read::Refreshed(user)) => Ok((user, jar.add(session::cookie(token)))),
        Ok(Read::Unknown) => Err((jar.add(session::removal()), json_error(StatusCode::UNAUTHORIZED, "signed-out")).into_response()),
        Err(e) => Err(store_error(e)),
    }
}

pub(crate) fn no_content(jar: CookieJar) -> Response {
    (jar, StatusCode::NO_CONTENT).into_response()
}

pub(crate) fn json<T: Serialize>(jar: CookieJar, value: &T) -> Response {
    (jar, Json(value)).into_response()
}

#[derive(Serialize)]
struct ProviderOut {
    slug: String,
    name: &'static str,
}

async fn providers(State(a): State<Arc<Account>>) -> Response {
    let list: Vec<ProviderOut> = a.providers.list().into_iter().map(|(slug, name)| ProviderOut { slug, name }).collect();
    Json(serde_json::json!({ "providers": list })).into_response()
}

#[derive(Serialize)]
struct IdentityOut {
    id: String,
    provider: String,
    label: Option<String>,
    created_at: String,
}

#[derive(Serialize)]
struct MeOut {
    id: Uuid,
    display_name: String,
    store_on_server: Option<bool>,
    identities: Vec<IdentityOut>,
}

async fn me(State(a): State<Arc<Account>>, jar: CookieJar) -> Response {
    let (user, jar) = match authed(&a, jar).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let (view, identities) = match tokio::try_join!(a.repo.user(user), a.repo.identities(user)) {
        Ok((Some(view), identities)) => (view, identities),
        Ok((None, _)) => return (jar.add(session::removal()), json_error(StatusCode::UNAUTHORIZED, "signed-out")).into_response(),
        Err(e) => return store_error(e),
    };
    let identities = identities
        .into_iter()
        .map(|i| IdentityOut {
            id: i.id,
            provider: i.provider,
            label: i.label,
            created_at: i.created_at.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
        })
        .collect();
    json(jar, &MeOut { id: view.id, display_name: view.display_name, store_on_server: view.store_on_server, identities })
}

async fn set_storage(
    State(a): State<Arc<Account>>,
    jar: CookieJar,
    body: Result<Json<serde_json::Value>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let (user, jar) = match authed(&a, jar).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    // Read by hand: a derived `Deserialize` would also take the array `[true]` for the one field.
    let Ok(Json(body)) = body else { return json_error(StatusCode::BAD_REQUEST, "bad-request") };
    let Some(store) = body.as_object().and_then(|o| o.get("store_on_server")).and_then(serde_json::Value::as_bool) else {
        return json_error(StatusCode::BAD_REQUEST, "bad-request");
    };
    match accounts::set_storage(&*a.repo, user, store).await {
        Ok(()) => no_content(jar),
        Err(e) => store_error(e),
    }
}

async fn unlink(State(a): State<Arc<Account>>, Path(id): Path<String>, jar: CookieJar) -> Response {
    let (user, jar) = match authed(&a, jar).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    match accounts::unlink(&*a.repo, user, &id).await {
        Ok(()) => no_content(jar),
        Err(UnlinkError::LastIdentity) => json_error(StatusCode::CONFLICT, "last-identity"),
        Err(UnlinkError::NotFound) => json_error(StatusCode::NOT_FOUND, "not-found"),
        Err(UnlinkError::Repo(e)) => store_error(e),
    }
}

async fn delete_me(State(a): State<Arc<Account>>, jar: CookieJar) -> Response {
    let (user, jar) = match authed(&a, jar).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    match accounts::delete(&*a.repo, user).await {
        Ok(()) => {
            tracing::info!(outcome = "deleted", "account");
            no_content(jar.add(session::removal()))
        }
        Err(e) => store_error(e),
    }
}

async fn not_found() -> Response {
    json_error(StatusCode::NOT_FOUND, "not-found")
}

pub fn routes(account: Arc<Account>) -> Router {
    let changing = Router::new()
        .route("/api/me/storage", put(set_storage))
        .route("/api/me/identities/{id}", delete(unlink))
        .route("/api/me", delete(delete_me))
        .route_layer(middleware::from_fn_with_state(account.clone(), require_origin));
    Router::new()
        .route("/api/providers", get(providers))
        .route("/api/me", get(me))
        .merge(changing)
        .merge(work::routes(account.clone()))
        .route("/api/{*rest}", any(not_found))
        .layer(middleware::map_response(|mut res: Response| async move {
            res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            res
        }))
        // Outermost, as for `/auth/*` in Task 10: 503 while the database is not ready (`use crate::app::require_ready`).
        .layer(middleware::from_fn_with_state(account.clone(), require_ready))
        .with_state(account)
}
