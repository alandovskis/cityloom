//! `cache` — the in-memory extract cache: an LRU by byte weight under a
//! ceiling (BR6.2, NFR3.1.8), single-flight for concurrent misses on the
//! same key (BR6.3), and a hard rule that a loader failure is never stored
//! (BR6.2). Built on `moka::future::Cache`, whose default eviction policy is
//! not LRU, so [`moka::policy::EvictionPolicy::lru`] is set explicitly
//! (PD-5).

use std::sync::Arc;

use moka::future::Cache as MokaCache;
use moka::policy::EvictionPolicy;

use crate::extract_key::ExtractKey;

/// One cached clip's encoded bytes (BR5.5's PBF output). Cheap to clone: the
/// bytes are behind an `Arc`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedExtract {
    pub bytes: Arc<Vec<u8>>,
}

impl CachedExtract {
    pub fn new(bytes: Vec<u8>) -> CachedExtract {
        CachedExtract {
            bytes: Arc::new(bytes),
        }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

/// The extract cache: keyed on the raw [`ExtractKey`] bytes, weighed in
/// bytes, evicted least-recently-used first under `max_bytes` (BR6.2,
/// NFR3.1.8).
pub struct Cache {
    inner: MokaCache<[u8; 32], CachedExtract>,
    max_bytes: u64,
}

impl Cache {
    pub fn new(max_bytes: u64) -> Cache {
        let inner = MokaCache::builder()
            .max_capacity(max_bytes)
            .weigher(|_key: &[u8; 32], value: &CachedExtract| {
                u32::try_from(value.len()).unwrap_or(u32::MAX)
            })
            .eviction_policy(EvictionPolicy::lru())
            .build();
        Cache { inner, max_bytes }
    }

    /// Fetch the cached clip for `key`, or run `init` if absent. Concurrent
    /// calls for the same key share the one `init` run (BR6.3, moka's
    /// native single-flight); an `Err` from `init` is never stored (BR6.2).
    /// A result bigger than the whole cache ceiling is returned to the
    /// caller but evicted immediately rather than retained (NFR3.1.8).
    pub async fn try_get_with<Fut, E>(
        &self,
        key: ExtractKey,
        init: Fut,
    ) -> Result<CachedExtract, Arc<E>>
    where
        Fut: Future<Output = Result<CachedExtract, E>>,
        E: Send + Sync + 'static,
    {
        let key_bytes = *key.as_bytes();
        let result = self.inner.try_get_with(key_bytes, init).await?;
        if result.len() as u64 > self.max_bytes {
            self.inner.invalidate(&key_bytes).await;
        }
        Ok(result)
    }

    /// Force pending eviction/insertion housekeeping to run synchronously,
    /// so tests can observe the cache's state deterministically.
    pub async fn run_pending_tasks(&self) {
        self.inner.run_pending_tasks().await;
    }

    /// Whether `key` is cached right now (used only to distinguish a hit
    /// from a miss for the counters; BR2.2 already counts both against the
    /// requester before this is ever consulted).
    pub fn contains(&self, key: ExtractKey) -> bool {
        self.inner.contains_key(key.as_bytes())
    }

    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    pub fn weighted_size(&self) -> u64 {
        self.inner.weighted_size()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::geo::CanonicalBox;
    use crate::manifest::BuildId;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct LoadFailed;

    fn key(n: u8) -> ExtractKey {
        let build = BuildId::parse(&"ab".repeat(32)).unwrap();
        let b = CanonicalBox::parse(&format!("0,0,{n}.001,{n}.001")).unwrap();
        ExtractKey::derive(&b, &build)
    }

    fn extract(bytes: usize) -> CachedExtract {
        CachedExtract::new(vec![7u8; bytes])
    }

    // NFR3.1.8 — filling past the ceiling evicts the oldest key first and
    // keeps the total under the ceiling.
    #[tokio::test]
    async fn fills_past_the_ceiling_and_evicts_oldest_first() {
        let cache = Cache::new(100);
        for n in 0..5u8 {
            cache
                .try_get_with::<_, LoadFailed>(key(n), async { Ok(extract(40)) })
                .await
                .unwrap();
            cache.run_pending_tasks().await;
        }
        cache.run_pending_tasks().await;
        assert!(cache.weighted_size() <= 100, "{}", cache.weighted_size());
        // The first key inserted (key(0)) should have been evicted; a fresh
        // fetch for it runs the loader again rather than hitting a stale entry.
        let ran_again = AtomicUsize::new(0);
        cache
            .try_get_with::<_, LoadFailed>(key(0), async {
                ran_again.fetch_add(1, Ordering::SeqCst);
                Ok(extract(40))
            })
            .await
            .unwrap();
        assert_eq!(ran_again.load(Ordering::SeqCst), 1, "key(0) was evicted");
    }

    // NFR3.1.8 / NFR5.1.1 — an oversize clip is returned but not retained.
    #[tokio::test]
    async fn an_oversize_clip_is_returned_and_not_retained() {
        let cache = Cache::new(100);
        let result = cache
            .try_get_with::<_, LoadFailed>(key(9), async { Ok(extract(500)) })
            .await
            .unwrap();
        assert_eq!(result.len(), 500, "the caller still gets the bytes");
        cache.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 0, "but nothing is retained");
    }

    // BR6.3 — concurrent misses for one key share one loader run.
    #[tokio::test]
    async fn concurrent_misses_for_one_key_share_one_loader_run() {
        let cache = Arc::new(Cache::new(1_000));
        let runs = Arc::new(AtomicUsize::new(0));
        let k = key(1);

        let mut handles = Vec::new();
        for _ in 0..8 {
            let cache = Arc::clone(&cache);
            let runs = Arc::clone(&runs);
            handles.push(tokio::spawn(async move {
                cache
                    .try_get_with::<_, LoadFailed>(k, async {
                        runs.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                        Ok(extract(10))
                    })
                    .await
                    .unwrap()
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(runs.load(Ordering::SeqCst), 1);
    }

    // BR6.2 — a loader error is stored nowhere.
    #[tokio::test]
    async fn a_loader_error_is_stored_nowhere() {
        let cache = Cache::new(1_000);
        let k = key(2);
        let err = cache
            .try_get_with::<_, LoadFailed>(k, async { Err(LoadFailed) })
            .await;
        assert!(err.is_err());
        cache.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 0);

        // A retry after the failure runs the loader again and can succeed.
        let ok = cache
            .try_get_with::<_, LoadFailed>(k, async { Ok(extract(10)) })
            .await;
        assert!(ok.is_ok());
    }
}
