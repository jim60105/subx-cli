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
