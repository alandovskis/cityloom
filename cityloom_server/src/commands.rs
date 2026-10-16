//! `cityloom_server rewrap` and `cityloom_server rehash`.
use crate::config::AccountConfig;
use crate::crypto::{LookupKey, MasterKeys};
use crate::db::{self, PgRepo};

pub async fn run(name: &str, config: &AccountConfig, old_lookup_key: Option<zeroize::Zeroizing<[u8; 32]>>) -> Result<String, String> {
    if !matches!(name, "rewrap" | "rehash") {
        return Err(format!("unknown command {name}: use rewrap or rehash"));
    }
    let old = match (name, old_lookup_key) {
        ("rehash", None) => return Err("OLD_LOOKUP_KEY is not set".into()),
        (_, old) => old,
    };
    let pool = db::connect(&config.database_url).await.map_err(|_| "the database could not be reached or migrated".to_string())?;
    let masters = MasterKeys::new(&config.data_keys, config.data_key_current).map_err(|e| e.to_string())?;
    let repo = PgRepo::new(pool, masters, LookupKey::new(config.lookup_key.clone()));
    match old {
        None => repo.rewrap().await.map(|n| format!("resealed {n} data keys")).map_err(|e| e.to_string()),
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
