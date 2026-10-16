//! `cityloom_server rewrap` and `cityloom_server rehash`.
use crate::config::AccountConfig;
use crate::crypto::{LookupKey, MasterKeys};
use crate::db::{self, PgRepo};

/// The old lookup key a command uses: only `rehash` does, and it needs it. A `rewrap` run with `OLD_LOOKUP_KEY` still
/// in the environment must not rehash.
fn old_key_for(name: &str, old: Option<zeroize::Zeroizing<[u8; 32]>>) -> Result<Option<zeroize::Zeroizing<[u8; 32]>>, String> {
    match (name, old) {
        ("rehash", None) => Err("OLD_LOOKUP_KEY is not set".into()),
        ("rehash", old) => Ok(old),
        _ => Ok(None),
    }
}

/// The line `rewrap` prints; an operator runs it until it says 0.
fn rewrapped(n: u64) -> String {
    format!("resealed {n} data keys")
}

pub async fn run(name: &str, config: &AccountConfig, old_lookup_key: Option<zeroize::Zeroizing<[u8; 32]>>) -> Result<String, String> {
    if !matches!(name, "rewrap" | "rehash") {
        return Err(format!("unknown command {name}: use rewrap or rehash"));
    }
    let old = old_key_for(name, old_lookup_key)?;
    let pool = db::connect(&config.database_url).await.map_err(|_| "the database could not be reached or migrated".to_string())?;
    let masters = MasterKeys::new(&config.data_keys, config.data_key_current).map_err(|e| e.to_string())?;
    let repo = PgRepo::new(pool, masters, LookupKey::new(config.lookup_key.clone()));
    match old {
        None => repo.rewrap().await.map(|n| rewrapped(n)).map_err(|e| e.to_string()),
        Some(old) => repo.rehash(&LookupKey::new(old)).await.map(|counts| counts.to_string()).map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
mod tests {
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
}
