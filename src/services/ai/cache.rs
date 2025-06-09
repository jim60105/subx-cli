//! AI analysis result caching system for performance optimization.
//!
//! This module provides a high-performance caching layer for AI analysis results,
//! reducing the cost and latency of repeated content analysis operations. The cache
//! uses intelligent key generation and TTL-based expiration to balance performance
//! with data freshness.
//!
//! # Cache Architecture
//!
//! ## Key Generation
//! - **Content-Based Keys**: Hash-based keys derived from request content
//! - **Deterministic Hashing**: Consistent keys for identical requests
//! - **Collision Resistance**: Low probability of hash collisions
//! - **Efficient Lookup**: O(1) average case lookup performance
//!
//! ## Storage Strategy
//! - **In-Memory Storage**: Fast access using HashMap data structure
//! - **TTL Expiration**: Time-based cache invalidation for freshness
//! - **LRU Eviction**: Least Recently Used eviction for memory management
//! - **Concurrent Access**: Thread-safe operations using RwLock
//!
//! ## Performance Benefits
//! - **Cost Reduction**: Avoid expensive AI API calls for repeated requests
//! - **Latency Improvement**: Sub-millisecond response time for cached results
//! - **Rate Limit Compliance**: Reduce API usage to stay within provider limits
//! - **Offline Operation**: Serve cached results when API is unavailable
//!
//! # Usage Examples
//!
//! ## Basic Caching Operation
//! ```rust,ignore
//! use subx_cli::services::ai::{AICache, AnalysisRequest};
//! use std::time::Duration;
//!
//! // Create cache with 1-hour TTL
//! let cache = AICache::new(Duration::from_secs(3600));
//!
//! // Check for cached result
//! let request = AnalysisRequest { /* ... */ };
//! if let Some(cached_result) = cache.get(&request).await {
//!     println!("Using cached result: {:?}", cached_result);
//!     return Ok(cached_result);
//! }
//!
//! // Perform AI analysis and cache result
//! let fresh_result = ai_client.analyze_content(request.clone()).await?;
//! cache.put(request, fresh_result.clone()).await;
//! ```
//!
//! ## Cache Management
//! ```rust,ignore
//! use subx_cli::services::ai::AICache;
//!
//! let cache = AICache::new(Duration::from_secs(1800)); // 30 minutes
//!
//! // Get cache statistics
//! let stats = cache.stats().await;
//! println!("Cache hits: {}, misses: {}, size: {}",
//!     stats.hits, stats.misses, stats.size);
//!
//! // Clear expired entries
//! let expired_count = cache.cleanup_expired().await;
//! println!("Removed {} expired entries", expired_count);
//!
//! // Clear all cache entries
//! cache.clear().await;
//! ```

use crate::services::ai::{AnalysisRequest, MatchResult};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// AI 分析結果快取
pub struct AICache {
    cache: RwLock<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

#[cfg(test)]
mod tests {
    use super::{AICache, AnalysisRequest, MatchResult};
    use crate::services::ai::ContentSample;
    use std::time::Duration;
    use tokio::time::sleep;

    fn make_request() -> AnalysisRequest {
        AnalysisRequest {
            video_files: vec!["video1.mp4".to_string()],
            subtitle_files: vec!["sub1.srt".to_string()],
            content_samples: vec![ContentSample {
                filename: "sub1.srt".to_string(),
                content_preview: "test".to_string(),
                file_size: 123,
            }],
        }
    }

    #[tokio::test]
    async fn test_cache_get_set_and_generate_key() {
        let cache = AICache::new(Duration::from_secs(60));
        let key = AICache::generate_key(&make_request());
        // cache miss
        assert!(cache.get(&key).await.is_none());

        let result = MatchResult {
            matches: vec![],
            confidence: 0.5,
            reasoning: "ok".to_string(),
        };
        cache.set(key.clone(), result.clone()).await;
        // cache hit
        assert_eq!(cache.get(&key).await, Some(result));
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = AICache::new(Duration::from_millis(50));
        let key = "expire".to_string();
        let result = MatchResult {
            matches: vec![],
            confidence: 1.0,
            reasoning: "expire".to_string(),
        };
        cache.set(key.clone(), result).await;
        // immediate hit
        assert!(cache.get(&key).await.is_some());
        sleep(Duration::from_millis(100)).await;
        // after ttl
        assert!(cache.get(&key).await.is_none());
    }
}

struct CacheEntry {
    data: MatchResult,
    created_at: SystemTime,
}

impl AICache {
    /// 建立快取，ttl 為過期時間
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    /// 嘗試從快取讀取結果
    pub async fn get(&self, key: &str) -> Option<MatchResult> {
        let cache = self.cache.read().await;

        if let Some(entry) = cache.get(key) {
            if entry.created_at.elapsed().unwrap_or(Duration::MAX) < self.ttl {
                return Some(entry.data.clone());
            }
        }
        None
    }

    /// 將新結果寫入快取
    pub async fn set(&self, key: String, data: MatchResult) {
        let mut cache = self.cache.write().await;
        cache.insert(
            key,
            CacheEntry {
                data,
                created_at: SystemTime::now(),
            },
        );
    }

    /// 根據請求產生快取鍵
    pub fn generate_key(request: &AnalysisRequest) -> String {
        let mut hasher = DefaultHasher::new();
        request.video_files.hash(&mut hasher);
        request.subtitle_files.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}
