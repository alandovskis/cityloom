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
mod tests {
    use super::*;

    fn key(n: u8) -> cookie::Key {
        cookie::Key::from(&[n; 64])
    }

    fn flow(at: i64) -> FlowState {
        FlowState {
            provider: "google".into(),
            state: "state-abc".into(),
            nonce: "nonce-abc".into(),
            pkce_verifier: "verifier-secret-xyz".into(),
            return_path: "/map.html".into(),
            link_user: None,
            started_at: at,
        }
    }

    #[test]
    fn a_flow_round_trips_through_its_cookie_and_the_value_hides_its_contents() {
        let c = cookie(&flow(1000), &key(1));
        assert_eq!(read(Some(c.value()), &key(1), 1000).unwrap(), flow(1000));
        for secret in ["verifier-secret-xyz", "state-abc", "nonce-abc", "google"] {
            assert!(!c.value().contains(secret));
        }
    }

    #[test]
    fn the_cookie_is_host_prefixed_http_only_secure_same_site_none_for_ten_minutes() {
        let s = cookie(&flow(0), &key(1)).to_string();
        assert!(s.starts_with("__Host-cityloom-flow="), "{s}");
        for part in ["HttpOnly", "Secure", "SameSite=None", "Path=/", "Max-Age=600"] {
            assert!(s.contains(part), "{s} lacks {part}");
        }
        assert!(!s.contains("Domain"));
        let gone = removal().to_string();
        assert!(gone.contains("Max-Age=0") && gone.contains("SameSite=None") && gone.contains("Secure"), "{gone}");
    }

    #[test]
    fn a_missing_tampered_or_foreign_cookie_is_refused() {
        assert_eq!(read(None, &key(1), 0).unwrap_err(), FlowError::Missing);
        let c = cookie(&flow(0), &key(1));
        assert_eq!(read(Some(c.value()), &key(2), 0).unwrap_err(), FlowError::Invalid, "another cookie key");
        let mut bad = c.value().to_string();
        bad.replace_range(5..6, if &bad[5..6] == "A" { "B" } else { "A" });
        assert_eq!(read(Some(&bad), &key(1), 0).unwrap_err(), FlowError::Invalid);
        assert_eq!(read(Some("plain-json"), &key(1), 0).unwrap_err(), FlowError::Invalid);
    }

    #[test]
    fn a_flow_older_than_ten_minutes_is_expired_even_if_the_cookie_is_still_there() {
        let c = cookie(&flow(1000), &key(1));
        assert!(read(Some(c.value()), &key(1), 1600).is_ok(), "exactly ten minutes is still good");
        assert_eq!(read(Some(c.value()), &key(1), 1601).unwrap_err(), FlowError::Expired);
        assert_eq!(read(Some(c.value()), &key(1), 1000 - 61).unwrap_err(), FlowError::Invalid, "dated in the future");
    }

    #[test]
    fn state_must_match_exactly() {
        let f = flow(0);
        assert_eq!(check_state(&f, Some("state-abc")), Ok(()));
        assert_eq!(check_state(&f, Some("state-abd")), Err(FlowError::StateMismatch));
        assert_eq!(check_state(&f, Some("")), Err(FlowError::StateMismatch));
        assert_eq!(check_state(&f, None), Err(FlowError::StateMismatch));
    }

    #[test]
    fn the_return_path_is_a_relative_path_on_this_site_or_the_home_page() {
        for ok in ["/", "/map.html", "/street.html?street=7", "/account.html", "/a/b/c?x=1&y=2"] {
            assert_eq!(safe_return(Some(ok)), ok);
        }
        assert_eq!(safe_return(Some("/map.html#frag")), "/map.html", "a fragment would fight the #account= outcome");
        for bad in [
            "",
            "map.html",
            "//evil.com",
            "///evil.com",
            "/\\evil.com",
            "\\\\evil.com",
            "https://evil.com",
            "javascript:alert(1)",
            "/%2f%2fevil.com",
            "/%2F/evil.com",
            "/%5cevil.com",
            "/%5Cevil.com",
            "/ok\nSet-Cookie: x=y",
            "/ok%0d%0a",
            "/\t/evil.com",
            "http:/evil.com",
            "/%00",
            "/path\\..\\x",
        ] {
            assert_eq!(safe_return(Some(bad)), "/", "{bad:?}");
        }
        assert_eq!(safe_return(None), "/");
        assert_eq!(safe_return(Some(&format!("/{}", "a".repeat(600)))), "/", "too long");
    }
}
