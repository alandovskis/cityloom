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
mod tests;
