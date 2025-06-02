# Product Backlog #05: AI 服務整合

## 領域範圍
OpenAI API 整合、AI 分析服務、錯誤處理和重試機制

## 完成項目

### 1. AI Provider Trait 設計
- [ ] 定義 `AIProvider` trait 介面
- [ ] 實作分析請求和回應結構
- [ ] 支援多種 AI 提供商擴展
- [ ] 定義信心度評分機制

### 2. OpenAI 客戶端實作
- [ ] HTTP 客戶端設定和配置
- [ ] API 金鑰認證處理
- [ ] Chat Completion API 整合
- [ ] 請求和回應序列化

### 3. 內容分析功能
- [ ] 檔名模式分析
- [ ] 字幕內容採樣
- [ ] 語義相似度計算
- [ ] 匹配信心度評估

### 4. Prompt 工程
- [ ] 設計檔案匹配 Prompt 模板
- [ ] 優化 AI 回應格式
- [ ] 多語言內容處理
- [ ] 上下文窗口管理

### 5. 錯誤處理和重試
- [ ] API 限流處理
- [ ] 網路錯誤重試機制
- [ ] 成本控制和監控
- [ ] 降級策略設計

### 6. 快取機制
- [ ] 分析結果快取
- [ ] 快取過期策略
- [ ] 本地快取儲存
- [ ] 快取鍵設計

## 技術設計

### AI Provider Trait
```rust
// src/services/ai/mod.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait AIProvider: Send + Sync {
    async fn analyze_content(&self, request: AnalysisRequest) -> crate::Result<MatchResult>;
    async fn verify_match(&self, verification: VerificationRequest) -> crate::Result<ConfidenceScore>;
}

#[derive(Debug, Serialize)]
pub struct AnalysisRequest {
    pub video_files: Vec<String>,
    pub subtitle_files: Vec<String>,
    pub content_samples: Vec<ContentSample>,
}

#[derive(Debug, Serialize)]
pub struct ContentSample {
    pub filename: String,
    pub content_preview: String,
    pub file_size: u64,
    pub language_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MatchResult {
    pub matches: Vec<FileMatch>,
    pub confidence: f32,
    pub reasoning: String,
}

#[derive(Debug, Deserialize)]
pub struct FileMatch {
    pub video_file: String,
    pub subtitle_file: String,
    pub confidence: f32,
    pub match_factors: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConfidenceScore {
    pub score: f32,
    pub factors: Vec<String>,
}
```

### OpenAI 客戶端實作
```rust
// src/services/ai/openai.rs
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

pub struct OpenAIClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIClient {
    pub fn new(api_key: String, model: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            api_key,
            model,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }
    
    async fn chat_completion(&self, messages: Vec<serde_json::Value>) -> crate::Result<String> {
        let request_body = json!({
            "model": self.model,
            "messages": messages,
            "temperature": 0.3,
            "max_tokens": 1000
        });
        
        let response = self.client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(crate::SubXError::AIService(
                format!("OpenAI API 錯誤 {}: {}", status, error_text)
            ));
        }
        
        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| crate::SubXError::AIService("無效的 API 回應格式".to_string()))?;
        
        Ok(content.to_string())
    }
}

#[async_trait]
impl AIProvider for OpenAIClient {
    async fn analyze_content(&self, request: AnalysisRequest) -> crate::Result<MatchResult> {
        let prompt = self.build_analysis_prompt(&request);
        let messages = vec![
            json!({"role": "system", "content": "你是一個專業的字幕匹配助手，能夠分析影片和字幕檔案的對應關係。"}),
            json!({"role": "user", "content": prompt})
        ];
        
        let response = self.chat_completion(messages).await?;
        self.parse_match_result(&response)
    }
    
    async fn verify_match(&self, verification: VerificationRequest) -> crate::Result<ConfidenceScore> {
        let prompt = self.build_verification_prompt(&verification);
        let messages = vec![
            json!({"role": "system", "content": "請評估字幕匹配的信心度，提供 0-1 之間的分數。"}),
            json!({"role": "user", "content": prompt})
        ];
        
        let response = self.chat_completion(messages).await?;
        self.parse_confidence_score(&response)
    }
}
```

### Prompt 模板
```rust
// src/services/ai/prompts.rs
impl OpenAIClient {
    fn build_analysis_prompt(&self, request: &AnalysisRequest) -> String {
        let mut prompt = String::new();
        
        prompt.push_str("請分析以下影片和字幕檔案的匹配關係：\n\n");
        
        prompt.push_str("影片檔案：\n");
        for video in &request.video_files {
            prompt.push_str(&format!("- {}\n", video));
        }
        
        prompt.push_str("\n字幕檔案：\n");
        for subtitle in &request.subtitle_files {
            prompt.push_str(&format!("- {}\n", subtitle));
        }
        
        if !request.content_samples.is_empty() {
            prompt.push_str("\n字幕內容預覽：\n");
            for sample in &request.content_samples {
                prompt.push_str(&format!("檔案: {}\n", sample.filename));
                prompt.push_str(&format!("內容: {}\n\n", sample.content_preview));
            }
        }
        
        prompt.push_str(
            "請根據檔名模式、內容語言、季集資訊等因素，提供匹配建議。\n\
            回應格式為 JSON：\n\
            {\n\
              \"matches\": [\n\
                {\n\
                  \"video_file\": \"影片檔名\",\n\
                  \"subtitle_file\": \"字幕檔名\",\n\
                  \"confidence\": 0.95,\n\
                  \"match_factors\": [\"檔名相似\", \"內容語言匹配\"]\n\
                }\n\
              ],\n\
              \"confidence\": 0.9,\n\
              \"reasoning\": \"匹配原因說明\"\n\
            }"
        );
        
        prompt
    }
    
    fn parse_match_result(&self, response: &str) -> crate::Result<MatchResult> {
        // 嘗試從回應中提取 JSON
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];
        
        serde_json::from_str(json_str)
            .map_err(|e| crate::SubXError::AIService(
                format!("AI 回應解析失敗: {}", e)
            ))
    }
}
```

### 重試機制
```rust
// src/services/ai/retry.rs
use tokio::time::{sleep, Duration};

pub struct RetryConfig {
    pub max_attempts: usize,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

pub async fn retry_with_backoff<F, Fut, T>(
    operation: F,
    config: &RetryConfig,
) -> crate::Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = crate::Result<T>>,
{
    let mut last_error = None;
    
    for attempt in 0..config.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                
                if attempt < config.max_attempts - 1 {
                    let delay = std::cmp::min(
                        Duration::from_millis(
                            (config.base_delay.as_millis() as f64 
                             * config.backoff_multiplier.powi(attempt as i32)) as u64
                        ),
                        config.max_delay,
                    );
                    
                    sleep(delay).await;
                }
            }
        }
    }
    
    Err(last_error.unwrap())
}
```

### 快取實作
```rust
// src/services/ai/cache.rs
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

pub struct AICache {
    cache: RwLock<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

struct CacheEntry {
    data: MatchResult,
    created_at: SystemTime,
}

impl AICache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            ttl,
        }
    }
    
    pub async fn get(&self, key: &str) -> Option<MatchResult> {
        let cache = self.cache.read().await;
        
        if let Some(entry) = cache.get(key) {
            if entry.created_at.elapsed().unwrap_or(Duration::MAX) < self.ttl {
                return Some(entry.data.clone());
            }
        }
        
        None
    }
    
    pub async fn set(&self, key: String, data: MatchResult) {
        let mut cache = self.cache.write().await;
        cache.insert(key, CacheEntry {
            data,
            created_at: SystemTime::now(),
        });
    }
    
    fn generate_key(request: &AnalysisRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        request.video_files.hash(&mut hasher);
        request.subtitle_files.hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }
}
```

## 驗收標準
1. OpenAI API 整合正常運作
2. 錯誤處理和重試機制有效
3. 快取機制正確運作
4. AI 分析結果格式正確
5. 成本控制在合理範圍內

## 估計工時
4-5 天

## 相依性
- 依賴 Backlog #03 (配置管理系統)

## 風險評估
- 中風險：外部 API 依賴
- 注意事項：API 限流、成本控制、網路穩定性
