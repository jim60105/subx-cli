//! Configuration cache manager module.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json;

use crate::config::Config;
#[cfg(test)]
use crate::config::{AIConfig, FormatsConfig, GeneralConfig, SyncConfig};

/// Cache entry storing serialized configuration value and its expiration.
struct CacheEntry {
    value: serde_json::Value,
    created_at: Instant,
    ttl: Duration,
}

/// Manager for caching configuration segments to avoid repetitive loads.
pub struct ConfigCache {
    entries: HashMap<String, CacheEntry>,
    default_ttl: Duration,
}

impl ConfigCache {
    /// Create a new configuration cache with default TTL of 5 minutes.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl: Duration::from_secs(300),
        }
    }

    /// Attempt to retrieve a cached value, returning None if missing or expired.
    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let entry = self.entries.get(key)?;
        if entry.created_at.elapsed() > entry.ttl {
            return None;
        }
        serde_json::from_value(entry.value.clone()).ok()
    }

    /// Insert or update a cache entry with an optional TTL override.
    pub fn set<T>(&mut self, key: String, value: T, ttl: Option<Duration>)
    where
        T: serde::Serialize,
    {
        let json_value = serde_json::to_value(value).unwrap();
        let entry = CacheEntry {
            value: json_value,
            created_at: Instant::now(),
            ttl: ttl.unwrap_or(self.default_ttl),
        };
        self.entries.insert(key, entry);
    }

    /// Update the cache with the full configuration and its segments.
    pub fn update(&mut self, config: &Config) {
        self.set("full_config".to_string(), config, None);
        self.set("ai_config".to_string(), &config.ai, None);
        self.set("formats_config".to_string(), &config.formats, None);
        self.set("sync_config".to_string(), &config.sync, None);
        self.set("general_config".to_string(), &config.general, None);
    }

    /// Clear all cache entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Remove expired entries from the cache.
    pub fn cleanup_expired(&mut self) {
        self.entries
            .retain(|_, entry| entry.created_at.elapsed() <= entry.ttl);
    }
}

impl Default for ConfigCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Dummy {
        x: u32,
    }

    #[test]
    fn test_set_get() {
        let mut cache = ConfigCache::new();
        cache.set("dummy".to_string(), Dummy { x: 42 }, None);
        let value: Option<Dummy> = cache.get("dummy");
        assert_eq!(value, Some(Dummy { x: 42 }));
    }

    #[test]
    fn test_ttl_expiration() {
        let mut cache = ConfigCache::new();
        cache.set(
            "dummy".to_string(),
            Dummy { x: 42 },
            Some(Duration::from_millis(10)),
        );
        sleep(Duration::from_millis(20));
        let value: Option<Dummy> = cache.get("dummy");
        assert!(value.is_none());
    }

    #[test]
    fn test_clear_and_cleanup() {
        let mut cache = ConfigCache::new();
        cache.set("a".to_string(), Dummy { x: 1 }, None);
        cache.set(
            "b".to_string(),
            Dummy { x: 2 },
            Some(Duration::from_secs(0)),
        );
        cache.cleanup_expired();
        assert!(cache.get::<Dummy>("b").is_none());
        assert!(cache.get::<Dummy>("a").is_some());
        cache.clear();
        assert!(cache.get::<Dummy>("a").is_none());
    }

    #[test]
    fn test_update() {
        let mut cache = ConfigCache::new();
        let config = Config::default();
        cache.update(&config);
        assert!(cache.get::<Config>("full_config").is_some());
        assert!(cache.get::<AIConfig>("ai_config").is_some());
        assert!(cache.get::<FormatsConfig>("formats_config").is_some());
        assert!(cache.get::<SyncConfig>("sync_config").is_some());
        assert!(cache.get::<GeneralConfig>("general_config").is_some());
    }
}
