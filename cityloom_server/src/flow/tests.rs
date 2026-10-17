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
