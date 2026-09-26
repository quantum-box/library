//! Built indexes, kept per Database until the Database changes.
//!
//! An entry is valid for exactly one fingerprint: the Database's
//! `DataRevision` plus its Property definitions. Every search reads the
//! current fingerprint first (one aggregate query) and rebuilds on a
//! mismatch, so a write is visible to the next search on every instance
//! without any invalidation message having to reach it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::index::SearchIndex;

/// Databases whose index one process keeps at a time. Each entry holds a
/// whole Database, so this bounds memory rather than tuning hit rate.
pub const DEFAULT_CAPACITY: usize = 32;

#[derive(Debug)]
pub struct SearchIndexCache {
    capacity: usize,
    entries: Mutex<HashMap<String, Entry>>,
}

#[derive(Debug)]
struct Entry {
    fingerprint: String,
    index: Arc<SearchIndex>,
    last_used: Instant,
}

impl Default for SearchIndexCache {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

impl SearchIndexCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// The index for `key`, if one was built at `fingerprint`.
    pub fn get(
        &self,
        key: &str,
        fingerprint: &str,
    ) -> Option<Arc<SearchIndex>> {
        let mut entries = self.entries.lock().ok()?;
        let entry = entries.get_mut(key)?;
        if entry.fingerprint != fingerprint {
            return None;
        }
        entry.last_used = Instant::now();
        Some(entry.index.clone())
    }

    /// Keep `index` as the current index for `key`, evicting the least
    /// recently used Database when full.
    pub fn insert(
        &self,
        key: &str,
        fingerprint: &str,
        index: Arc<SearchIndex>,
    ) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        if !entries.contains_key(key) && entries.len() >= self.capacity {
            if let Some(oldest) = entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone())
            {
                entries.remove(&oldest);
            }
        }
        entries.insert(
            key.to_string(),
            Entry {
                fingerprint: fingerprint.to_string(),
                index,
                last_used: Instant::now(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> Arc<SearchIndex> {
        Arc::new(SearchIndex::build(vec![], vec![]))
    }

    #[test]
    fn hit_only_at_the_same_fingerprint() {
        let cache = SearchIndexCache::new(2);
        cache.insert("db", "rev-1", index());
        assert!(cache.get("db", "rev-1").is_some());
        assert!(cache.get("db", "rev-2").is_none());
        assert!(cache.get("other", "rev-1").is_none());
    }

    #[test]
    fn a_new_revision_replaces_the_old_entry() {
        let cache = SearchIndexCache::new(2);
        cache.insert("db", "rev-1", index());
        cache.insert("db", "rev-2", index());
        assert!(cache.get("db", "rev-1").is_none());
        assert!(cache.get("db", "rev-2").is_some());
    }

    #[test]
    fn evicts_the_least_recently_used_database() {
        let cache = SearchIndexCache::new(2);
        cache.insert("a", "1", index());
        cache.insert("b", "1", index());
        assert!(cache.get("a", "1").is_some());
        cache.insert("c", "1", index());
        assert!(cache.get("a", "1").is_some());
        assert!(cache.get("b", "1").is_none());
        assert!(cache.get("c", "1").is_some());
    }
}
