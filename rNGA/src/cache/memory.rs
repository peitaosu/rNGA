//! In-memory cache implementation.

use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::RwLock,
    time::{Duration, Instant},
};

use super::traits::CacheStorage;

/// In-memory cache with optional TTL support.
#[derive(Debug, Default)]
pub struct MemoryCache {
    data: RwLock<HashMap<String, CacheEntry>>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    data: Vec<u8>,
    expires_at: Option<Instant>,
}

impl CacheEntry {
    fn new(data: Vec<u8>, ttl: Option<Duration>) -> Self {
        Self {
            data,
            expires_at: ttl.map(|d| Instant::now() + d),
        }
    }

    fn is_expired(&self) -> bool {
        self.expires_at.map_or(false, |e| Instant::now() > e)
    }
}

impl MemoryCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Remove expired entries.
    pub fn cleanup(&self) {
        let mut data = self.data.write().unwrap();
        data.retain(|_, v| !v.is_expired());
    }
}

#[async_trait]
impl CacheStorage for MemoryCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let data = self.data.read().unwrap();
        data.get(key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.data.clone())
            }
        })
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) {
        let mut data = self.data.write().unwrap();
        data.insert(key.to_owned(), CacheEntry::new(value.to_vec(), ttl));
    }

    async fn remove(&self, key: &str) {
        let mut data = self.data.write().unwrap();
        data.remove(key);
    }

    async fn clear(&self) {
        let mut data = self.data.write().unwrap();
        data.clear();
    }

    async fn scan_prefix(&self, prefix: &str) -> Vec<String> {
        let data = self.data.read().unwrap();
        data.keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_operations() {
        let cache = MemoryCache::new();

        cache.set("key1", b"value1", None).await;
        assert_eq!(cache.get("key1").await, Some(b"value1".to_vec()));

        cache.remove("key1").await;
        assert_eq!(cache.get("key1").await, None);
    }

    #[tokio::test]
    async fn test_scan_prefix() {
        let cache = MemoryCache::new();

        cache.set("prefix/a", b"1", None).await;
        cache.set("prefix/b", b"2", None).await;
        cache.set("other/c", b"3", None).await;

        let keys = cache.scan_prefix("prefix/").await;
        assert_eq!(keys.len(), 2);
        assert!(keys.iter().all(|k| k.starts_with("prefix/")));
    }

    #[tokio::test]
    async fn test_ttl_expiry() {
        let cache = MemoryCache::new();

        cache
            .set("key", b"value", Some(Duration::from_millis(60)))
            .await;

        assert!(cache.get("key").await.is_some());

        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(cache.get("key").await.is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = MemoryCache::new();

        cache.set("a", b"1", None).await;
        cache.set("b", b"2", None).await;

        cache.clear().await;

        assert!(cache.get("a").await.is_none());
        assert!(cache.get("b").await.is_none());
    }
}
