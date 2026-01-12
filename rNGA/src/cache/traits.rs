//! Cache storage trait definitions.

use async_trait::async_trait;
use std::time::Duration;

use crate::error::Result;

/// Trait for cache storage backends.
#[async_trait]
pub trait CacheStorage: Send + Sync + std::fmt::Debug {
    /// Get a value by key.
    async fn get(&self, key: &str) -> Option<Vec<u8>>;

    /// Set a value with optional TTL.
    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>);

    /// Remove a value by key.
    async fn remove(&self, key: &str);

    /// Clear all cached values.
    async fn clear(&self);

    /// Scan keys with a prefix.
    async fn scan_prefix(&self, prefix: &str) -> Vec<String>;
}

/// Extension trait for cache storage with typed operations.
#[async_trait]
pub trait CacheStorageExt: CacheStorage {
    /// Get a JSON-deserialized value.
    async fn get_json<T: serde::de::DeserializeOwned + Send>(&self, key: &str) -> Option<T> {
        let data = self.get(key).await?;
        serde_json::from_slice(&data).ok()
    }

    /// Set a JSON-serialized value.
    async fn set_json<T: serde::Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let data = serde_json::to_vec(value).map_err(crate::error::Error::Json)?;
        self.set(key, &data, ttl).await;
        Ok(())
    }
}

// Blanket implementation
impl<T: CacheStorage> CacheStorageExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::MemoryCache;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestData {
        value: String,
    }

    #[tokio::test]
    async fn test_cache_ext() {
        let cache = MemoryCache::new();
        let key = "test";
        let value = TestData {
            value: "hello".into(),
        };

        cache.set_json(key, &value, None).await.unwrap();
        let result: Option<TestData> = cache.get_json(key).await;
        assert_eq!(result, Some(value));
    }
}
