//! Parse cache: Moka LRU in-memory + optional SQLite persistence.
//! Keyed by content hash — same content always produces same parse result.

use moka::sync::Cache;

use super::types::ParseResult;

/// In-memory parse cache using Moka (TinyLFU admission).
pub struct ParseCache {
    inner: Cache<u64, ParseResult>,
}

impl ParseCache {
    /// Create a new parse cache with the given capacity.
    pub fn new(capacity: u64) -> Self {
        Self {
            inner: Cache::new(capacity),
        }
    }

    /// Get a cached parse result by content hash.
    pub fn get(&self, content_hash: u64) -> Option<ParseResult> {
        self.inner.get(&content_hash)
    }

    /// Insert a parse result into the cache.
    pub fn insert(&self, content_hash: u64, result: ParseResult) {
        self.inner.insert(content_hash, result);
    }

    /// Returns the number of entries in the cache.
    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    /// Invalidate a cache entry.
    pub fn invalidate(&self, content_hash: u64) {
        self.inner.invalidate(&content_hash);
    }
}

impl Default for ParseCache {
    fn default() -> Self {
        // Default: cache up to 10,000 parse results
        Self::new(10_000)
    }
}
