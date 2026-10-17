//! The state of one sign-in in progress, kept in an encrypted cookie, and the check of where to return to.
use cookie::{Cookie, CookieJar, Key, SameSite};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use uuid::Uuid;

pub const COOKIE: &str = "__Host-cityloom-flow";
pub const MAX_AGE_SECS: i64 = 600;
const FUTURE_LEEWAY_SECS: i64 = 60;
const RETURN_MAX: usize = 512;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowState {
    pub provider: String,
    pub state: String,
    pub nonce: String,
    pub pkce_verifier: String,
    pub return_path: String,
    pub link_user: Option<Uuid>,
    pub started_at: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FlowError {
    Missing,
    Invalid,
    Expired,
    StateMismatch,
}

fn attributes(value: String, max_age: i64) -> Cookie<'static> {
    Cookie::build((COOKIE, value)).http_only(true).secure(true).same_site(SameSite::None).path("/").max_age(cookie::time::Duration::seconds(max_age)).build()
}

pub fn cookie(flow: &FlowState, key: &Key) -> Cookie<'static> {
    let mut jar = CookieJar::new();
    jar.private_mut(key).add(attributes(serde_json::to_string(flow).expect("a flow serialises"), MAX_AGE_SECS));
    jar.get(COOKIE).expect("just added").clone()
}

pub fn removal() -> Cookie<'static> {
    attributes(String::new(), 0)
}

pub fn read(raw_value: Option<&str>, key: &Key, now_unix: i64) -> Result<FlowState, FlowError> {
    let raw = raw_value.ok_or(FlowError::Missing)?;
    let mut jar = CookieJar::new();
    jar.add_original(Cookie::new(COOKIE, raw.to_string()));
    let opened = jar.private(key).get(COOKIE).ok_or(FlowError::Invalid)?;
    let flow: FlowState = serde_json::from_str(opened.value()).map_err(|_| FlowError::Invalid)?;
    if flow.started_at - now_unix > FUTURE_LEEWAY_SECS {
        return Err(FlowError::Invalid);
    }
    if now_unix - flow.started_at > MAX_AGE_SECS {
        return Err(FlowError::Expired);
    }
    Ok(flow)
}

pub fn check_state(flow: &FlowState, got: Option<&str>) -> Result<(), FlowError> {
    match got {
        Some(g) if bool::from(g.as_bytes().ct_eq(flow.state.as_bytes())) => Ok(()),
        _ => Err(FlowError::StateMismatch),
    }
}

fn plain(path: &str) -> bool {
    path.starts_with('/') && !path.starts_with("//") && !path.contains('\\') && !path.chars().any(char::is_control)
}

/// Where to go back to after the provider: a relative path on this site, else `/`. A fragment is dropped (the
/// outcome is put there). The path is also judged after one round of percent-decoding, so `/%2f%2fevil` and `/%5c`
/// do not get through either.
pub fn safe_return(raw: Option<&str>) -> String {
    let Some(raw) = raw else { return "/".into() };
    let path = raw.split('#').next().unwrap_or("");
    if path.len() > RETURN_MAX || !plain(path) {
        return "/".into();
    }
    match percent_encoding::percent_decode_str(path).decode_utf8() {
        Ok(decoded) if plain(&decoded) => path.to_string(),
        _ => "/".into(),
    }
}

#[cfg(test)]
mod tests;
