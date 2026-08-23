//! A small bounded cache with per-entry expiry.
//!
//! Library calls tachyon-api on the request path, and the slowest of
//! those calls is token verification: every authenticated request
//! re-verifies the same bearer against `/v1/me`. Production traces show
//! that call dominating the latency tail, so the results are held here
//! for a short window instead.
//!
//! The cache is deliberately minimal — a mutex around a map, with
//! capacity enforced by evicting expired entries first and the oldest
//! entry after that. Nothing awaits while the lock is held.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A cached value together with the instant it stops being valid.
#[derive(Debug, Clone)]
struct Entry<V> {
    value: V,
    expires_at: Instant,
    stored_at: Instant,
}

/// A bounded, time-limited cache.
///
/// A `ttl` of zero disables the cache: nothing is ever stored and every
/// lookup misses. That is the switch operators reach for when a stale
/// entry has to be ruled out.
#[derive(Debug)]
pub struct TtlCache<K, V> {
    entries: Mutex<HashMap<K, Entry<V>>>,
    ttl: Duration,
    capacity: usize,
}

impl<K, V> TtlCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Create a cache holding at most `capacity` entries for `ttl`.
    pub fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl,
            capacity,
        }
    }

    /// Whether this cache stores anything at all.
    pub fn is_enabled(&self) -> bool {
        !self.ttl.is_zero() && self.capacity > 0
    }

    /// The cached value for `key`, if one is still valid.
    pub fn get(&self, key: &K) -> Option<V> {
        self.get_at(key, Instant::now())
    }

    /// Cache `value` under `key` until the configured TTL elapses.
    pub fn insert(&self, key: K, value: V) {
        self.insert_until(key, value, self.ttl);
    }

    /// Cache `value` under `key` for `ttl`, capped at the configured
    /// TTL.
    ///
    /// Callers use this when the value carries its own deadline — a
    /// verified token must never outlive its `exp`, however long the
    /// cache would otherwise keep it.
    pub fn insert_until(&self, key: K, value: V, ttl: Duration) {
        self.insert_at(key, value, ttl, Instant::now());
    }

    fn get_at(&self, key: &K, now: Instant) -> Option<V> {
        if !self.is_enabled() {
            return None;
        }

        let mut entries = self.lock();
        match entries.get(key) {
            Some(entry) if entry.expires_at > now => {
                Some(entry.value.clone())
            }
            Some(_) => {
                entries.remove(key);
                None
            }
            None => None,
        }
    }

    fn insert_at(&self, key: K, value: V, ttl: Duration, now: Instant) {
        if !self.is_enabled() {
            return;
        }

        let ttl = ttl.min(self.ttl);
        if ttl.is_zero() {
            return;
        }

        let mut entries = self.lock();
        entries.insert(
            key,
            Entry {
                value,
                expires_at: now + ttl,
                stored_at: now,
            },
        );

        if entries.len() <= self.capacity {
            return;
        }

        entries.retain(|_, entry| entry.expires_at > now);
        while entries.len() > self.capacity {
            let Some(oldest) = entries
                .iter()
                .min_by_key(|(_, entry)| entry.stored_at)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            entries.remove(&oldest);
        }
    }

    /// Number of entries currently held, expired ones included.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.lock().len()
    }

    /// Take the lock, recovering from a panic in another holder.
    ///
    /// A poisoned lock here means some caller panicked mid-update. The
    /// map is still structurally sound — this is a cache, and refusing
    /// to serve one would turn an unrelated panic into an outage.
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<K, Entry<V>>> {
        self.entries.lock().unwrap_or_else(|err| err.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache() -> TtlCache<String, u32> {
        TtlCache::new(Duration::from_secs(30), 4)
    }

    #[test]
    fn returns_a_stored_value_before_it_expires() {
        let cache = cache();
        let now = Instant::now();

        cache.insert_at("a".into(), 1, Duration::from_secs(30), now);

        assert_eq!(
            cache.get_at(&"a".into(), now + Duration::from_secs(29)),
            Some(1)
        );
    }

    #[test]
    fn drops_a_value_once_it_expires() {
        let cache = cache();
        let now = Instant::now();

        cache.insert_at("a".into(), 1, Duration::from_secs(30), now);

        assert_eq!(
            cache.get_at(&"a".into(), now + Duration::from_secs(31)),
            None
        );
        assert_eq!(cache.len(), 0, "the expired entry is not retained");
    }

    #[test]
    fn caps_an_entry_ttl_at_the_configured_ttl() {
        let cache = cache();
        let now = Instant::now();

        cache.insert_at("a".into(), 1, Duration::from_secs(600), now);

        assert_eq!(
            cache.get_at(&"a".into(), now + Duration::from_secs(31)),
            None,
            "a long-lived value must not outlive the cache TTL"
        );
    }

    #[test]
    fn honours_a_shorter_per_entry_ttl() {
        let cache = cache();
        let now = Instant::now();

        cache.insert_at("a".into(), 1, Duration::from_secs(5), now);

        assert_eq!(
            cache.get_at(&"a".into(), now + Duration::from_secs(6)),
            None,
            "an entry with its own deadline expires at that deadline"
        );
    }

    #[test]
    fn evicts_the_oldest_entry_when_over_capacity() {
        let cache = cache();
        let now = Instant::now();

        for (index, key) in ["a", "b", "c", "d", "e"].iter().enumerate() {
            cache.insert_at(
                (*key).into(),
                index as u32,
                Duration::from_secs(30),
                now + Duration::from_millis(index as u64),
            );
        }

        assert_eq!(cache.len(), 4);
        assert_eq!(cache.get_at(&"a".into(), now), None, "oldest evicted");
        assert_eq!(cache.get_at(&"e".into(), now), Some(4));
    }

    #[test]
    fn stores_nothing_when_the_ttl_is_zero() {
        let cache: TtlCache<String, u32> = TtlCache::new(Duration::ZERO, 4);
        let now = Instant::now();

        cache.insert_at("a".into(), 1, Duration::from_secs(30), now);

        assert!(!cache.is_enabled());
        assert_eq!(cache.get_at(&"a".into(), now), None);
        assert_eq!(cache.len(), 0);
    }
}
