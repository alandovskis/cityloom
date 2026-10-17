use super::*;

fn config() -> AccountConfig {
    let get = |k: &str| {
        use base64::Engine;
        let b = |n: usize| base64::engine::general_purpose::STANDARD.encode(vec![5u8; n]);
        Some(match k {
            "DATABASE_URL" => "postgres://invalid.invalid/none".to_string(),
            "BASE_URL" => "http://127.0.0.1:8137".to_string(),
            "COOKIE_KEY" => b(64),
            "DATA_KEYS" => format!("1:{}", b(32)),
            "DATA_KEY_CURRENT" => "1".to_string(),
            "LOOKUP_KEY" => b(32),
            _ => return None,
        })
    };
    match crate::config::Config::from_lookup(&get).unwrap().account {
        crate::config::AccountSide::On(a) => *a,
        other => panic!("{other:?}"),
    }
}

#[test]
fn only_rehash_uses_the_old_lookup_key() {
    let key = || Some(zeroize::Zeroizing::new([1u8; 32]));
    assert!(old_key_for("rewrap", key()).unwrap().is_none(), "a rewrap with OLD_LOOKUP_KEY left set stays a rewrap");
    assert!(old_key_for("rehash", key()).unwrap().is_some());
    assert_eq!(old_key_for("rehash", None).unwrap_err(), "OLD_LOOKUP_KEY is not set");
}

#[test]
fn the_rewrap_line_says_how_many_data_keys_it_resealed() {
    assert_eq!(rewrapped(0), "resealed 0 data keys");
}

#[test]
fn the_rehash_line_says_what_the_run_did() {
    let counts = crate::db::rotate::Rehashed { checked: 12, rehashed: 0, already_new: 12 };
    assert_eq!(counts.to_string(), "checked 12 identities: 0 rehashed, 12 already on the new key");
}

#[tokio::test]
async fn an_unknown_command_is_refused_and_rehash_needs_the_old_key() {
    assert_eq!(run("shrug", &config(), None).await.unwrap_err(), "unknown command shrug: use rewrap or rehash");
    assert_eq!(run("rehash", &config(), None).await.unwrap_err(), "OLD_LOOKUP_KEY is not set");
}
