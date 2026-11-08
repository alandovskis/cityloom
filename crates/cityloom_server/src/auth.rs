//! `/auth/*`: start a sign-in, finish it, sign out.
use std::sync::Arc;

use axum::Form;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use serde::Deserialize;

use crate::accounts::{self, Outcome};
use crate::app::{Account, json_error};
use crate::flow::{self, FlowState};
use crate::{providers, session};

#[derive(Deserialize)]
pub struct StartQuery {
    #[serde(rename = "return")]
    return_path: Option<String>,
    link: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    user: Option<String>,
}

/// A 303 to `to`. A header value must be visible ASCII, and a return path may hold other characters (the check on it
/// only wants a relative path on this site), so those are percent-encoded; a value that is still refused goes home.
fn redirect(to: &str) -> Response {
    let to = percent_encoding::utf8_percent_encode(to, percent_encoding::CONTROLS).to_string();
    let mut res = StatusCode::SEE_OTHER.into_response();
    res.headers_mut().insert(header::LOCATION, HeaderValue::from_str(&to).unwrap_or_else(|_| HeaderValue::from_static("/")));
    res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}

fn back(jar: CookieJar, return_path: &str, outcome: &str) -> Response {
    (jar, redirect(&format!("{return_path}#account={outcome}"))).into_response()
}

pub async fn start(State(a): State<Arc<Account>>, Path(slug): Path<String>, Query(q): Query<StartQuery>, jar: CookieJar) -> Response {
    let Some(spec) = a.providers.get(&slug) else { return StatusCode::NOT_FOUND.into_response() };
    let return_path = flow::safe_return(q.return_path.as_deref());
    let link_user = if q.link.as_deref() == Some("1") {
        match a.current_user(&jar).await {
            Some(u) => Some(u),
            None => return json_error(StatusCode::UNAUTHORIZED, "signed-out"),
        }
    } else {
        None
    };
    let started = match providers::begin(spec, &a.callback_url(&slug), &*a.http).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(provider = %slug, reason = %e, "sign-in could not start");
            return back(jar, &return_path, "failed");
        }
    };
    let state = FlowState {
        provider: slug,
        state: started.state,
        nonce: started.nonce,
        pkce_verifier: started.pkce_verifier,
        return_path,
        link_user,
        started_at: a.clock.now().unix_timestamp(),
    };
    (jar.add(flow::cookie(&state, &a.cookie_key)), redirect(started.url.as_str())).into_response()
}

pub async fn callback_get(state: State<Arc<Account>>, path: Path<String>, Query(p): Query<CallbackParams>, jar: CookieJar) -> Response {
    callback(state.0, path.0, jar, p).await
}

pub async fn callback_post(state: State<Arc<Account>>, path: Path<String>, jar: CookieJar, Form(p): Form<CallbackParams>) -> Response {
    callback(state.0, path.0, jar, p).await
}

async fn callback(a: Arc<Account>, slug: String, jar: CookieJar, p: CallbackParams) -> Response {
    let Some(spec) = a.providers.get(&slug) else { return StatusCode::NOT_FOUND.into_response() };
    let now = a.clock.now();
    let raw = jar.get(flow::COOKIE).map(|c| c.value().to_string());
    let flow = match flow::read(raw.as_deref(), &a.cookie_key, now.unix_timestamp()) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(provider = %slug, reason = ?e, "sign-in refused");
            // A cookie that cannot be matched to any state (expired, damaged, from another key) is unusable, so it
            // goes; with no cookie there is nothing to clear.
            let jar = if raw.is_some() { jar.add(flow::removal()) } else { jar };
            return back(jar, "/", "failed");
        }
    };
    // A flow of another provider, or another tab's (the state differs): not this callback's, and possibly still live.
    if flow.provider != slug || flow::check_state(&flow, p.state.as_deref()).is_err() {
        tracing::warn!(provider = %slug, reason = "state", "sign-in refused");
        return back(jar, "/", "failed");
    }
    let jar = jar.add(flow::removal());
    let fail = |jar: CookieJar, reason: &str| {
        tracing::warn!(provider = %slug, reason, "sign-in failed");
        back(jar, &flow.return_path, "failed")
    };
    if let Some(error) = &p.error {
        // Apple reports a cancel as `user_cancelled_authorize`; the others as `access_denied`.
        if matches!(error.as_str(), "access_denied" | "user_cancelled_authorize") {
            tracing::info!(provider = %slug, outcome = "cancelled", "sign-in");
            return back(jar, &flow.return_path, "cancelled");
        }
        return fail(jar, "provider-error");
    }
    let Some(code) = p.code.as_deref() else { return fail(jar, "no-code") };
    let claims = match providers::finish(spec, &a.callback_url(&slug), &*a.http, &flow, code, p.user.as_deref(), now).await {
        Ok(c) => c,
        Err(e) => return fail(jar, &e.to_string()),
    };
    if let Some(user) = flow.link_user {
        return match accounts::link(&*a.repo, user, &claims).await {
            Ok(outcome) => {
                tracing::info!(provider = %slug, outcome = outcome.slug(), "link");
                // The conflict message names the provider ("This Google account is already linked..."), so the fragment carries its slug.
                let fragment = if outcome == Outcome::Conflict { format!("{}&provider={slug}", outcome.slug()) } else { outcome.slug().to_string() };
                back(jar, &flow.return_path, &fragment)
            }
            Err(e) => fail(jar, &e.to_string()),
        };
    }
    let (user, outcome) = match accounts::sign_in(&*a.repo, &claims).await {
        Ok(r) => r,
        Err(e) => return fail(jar, &e.to_string()),
    };
    let token = match session::start(&*a.repo, user, now).await {
        Ok(t) => t,
        Err(e) => return fail(jar, &e.to_string()),
    };
    tracing::info!(provider = %slug, outcome = outcome.slug(), "sign-in");
    let jar = jar.add(session::cookie(token));
    let to = if outcome == Outcome::New { "/account.html" } else { flow.return_path.as_str() };
    back(jar, to, outcome.slug())
}

pub async fn signout(State(a): State<Arc<Account>>, jar: CookieJar) -> Response {
    if let Some(c) = jar.get(session::COOKIE) {
        if let Err(e) = session::end(&*a.repo, c.value()).await {
            // The cookie goes whatever happens, and the person is told they are out; the row that is left is useful
            // only to someone who already holds the token. The log has the reason, never the token.
            // The cookie goes whatever happens, and the person is told they are out; the row that is left is useful only to
            // someone who already holds the token. The log has the reason, never the token.
            tracing::error!(reason = %e, "a session could not be ended");
        }
    }
    // A link flow begun before signing out must not finish for the next person on this browser.
    (jar.add(session::removal()).add(flow::removal()), StatusCode::NO_CONTENT).into_response()
}
