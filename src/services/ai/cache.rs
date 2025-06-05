use crate::services::ai::{AnalysisRequest, MatchResult};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// AI 分析結果快取
pub struct AICache {
    cache: RwLock<HashMap<String, CacheEntry>>,
    ttl: Duration,
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
