# Product Backlog #12: 單元測試與程式碼覆蓋率

## 領域範圍
單元測試實作、模擬測試、程式碼覆蓋率分析、測試基礎設施

## 完成項目

### 1. 測試基礎設施建立
- [ ] 設定測試覆蓋率工具 (cargo-tarpaulin)
- [ ] 新增 mockall 框架用於模擬測試
- [ ] 建立測試資料產生器
- [ ] 設定 CI/CD 測試覆蓋率檢查
- [ ] 建立測試工具輔助函式

### 2. 錯誤處理模組測試
- [ ] `SubXError` 各種錯誤類型建立測試
- [ ] 錯誤訊息格式化測試
- [ ] 錯誤退出碼驗證
- [ ] 錯誤鏈追蹤測試
- [ ] 用戶友善錯誤訊息測試

### 3. 配置管理系統測試
- [ ] 配置檔案讀寫功能測試
- [ ] 環境變數優先權測試
- [ ] 配置驗證機制測試
- [ ] 預設值載入測試
- [ ] 配置合併邏輯測試
- [ ] 跨平台路徑處理測試

### 4. 字幕格式解析引擎測試
- [ ] SRT 格式解析與序列化測試
- [ ] ASS 格式解析與序列化測試
- [ ] VTT 格式解析與序列化測試
- [ ] SUB 格式解析與序列化測試
- [ ] 格式自動檢測測試
- [ ] 編碼檢測與轉換測試
- [ ] 錯誤格式處理測試

### 5. AI 服務整合測試
- [ ] OpenAI 客戶端模擬測試
- [ ] 重試機制測試
- [ ] 快取系統測試
- [ ] Prompt 建構測試
- [ ] 回應解析測試
- [ ] 錯誤處理測試

### 6. 檔案匹配引擎測試
- [ ] 檔案發現系統測試
- [ ] 檔名分析器測試
- [ ] 內容採樣器測試
- [ ] 匹配結果生成測試
- [ ] Dry-run 快取測試
- [ ] 檔案操作安全性測試

### 7. 格式轉換系統測試
- [ ] 格式間轉換邏輯測試
- [ ] 樣式保留機制測試
- [ ] 批量轉換測試
- [ ] 轉換品質驗證測試
- [ ] 錯誤恢復測試

### 8. 音訊處理與同步測試
- [ ] 音訊解碼測試
- [ ] 對話檢測算法測試
- [ ] 交叉相關分析測試
- [ ] 時間軸同步測試
- [ ] 偏移計算測試

### 9. CLI 介面測試
- [ ] 命令解析測試
- [ ] 參數驗證測試
- [ ] 錯誤處理測試
- [ ] 幫助資訊測試
- [ ] 用戶介面輸出測試

### 10. 覆蓋率目標與監控
- [ ] 達成 50% 以上整體測試覆蓋率
- [ ] 核心模組達成 70% 以上覆蓋率
- [ ] 建立覆蓋率監控機制
- [ ] 設定覆蓋率回歸檢查

## 技術設計

### 測試基礎設施配置

**Cargo.toml 測試相依套件更新：**
```toml
[dev-dependencies]
tokio-test = "0.4"
assert_cmd = "2.0"
predicates = "3.0"
tempfile = "3.8"
criterion = { version = "0.5", features = ["html_reports"] }
# 新增測試相依套件
mockall = "0.11"
serial_test = "3.0"
rstest = "0.18"
test-case = "3.0"
tarpaulin = "0.27"
wiremock = "0.5"
```

**測試覆蓋率工具設定：**
```toml
# tarpaulin.toml
[tarpaulin]
out = ["Html", "Lcov"]
target-dir = "target/tarpaulin"
timeout = 120
fail-under = 50
ignore-panics = true
count = true
all-features = true
exclude = [
    "benches/*",
    "tests/*"
]
```

### 錯誤處理模組測試實作
```rust
// src/error.rs (測試模組)
#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_config_error_creation() {
        let error = SubXError::config("測試配置錯誤");
        assert!(matches!(error, SubXError::Config { .. }));
        assert_eq!(error.to_string(), "配置錯誤: 測試配置錯誤");
    }

    #[test]
    fn test_subtitle_format_error_creation() {
        let error = SubXError::subtitle_format("SRT", "無效格式");
        assert!(matches!(error, SubXError::SubtitleFormat { .. }));
        assert!(error.to_string().contains("SRT"));
        assert!(error.to_string().contains("無效格式"));
    }

    #[test]
    fn test_audio_processing_error_creation() {
        let error = SubXError::audio_processing("音訊解碼失敗");
        assert!(matches!(error, SubXError::AudioProcessing { .. }));
        assert_eq!(error.to_string(), "音訊處理錯誤: 音訊解碼失敗");
    }

    #[test]
    fn test_file_matching_error_creation() {
        let error = SubXError::file_matching("匹配失敗");
        assert!(matches!(error, SubXError::FileMatching { .. }));
        assert_eq!(error.to_string(), "文件匹配錯誤: 匹配失敗");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "檔案不存在");
        let subx_error: SubXError = io_error.into();
        assert!(matches!(subx_error, SubXError::Io(_)));
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(SubXError::config("test").exit_code(), 2);
        assert_eq!(SubXError::subtitle_format("SRT", "test").exit_code(), 4);
        assert_eq!(SubXError::audio_processing("test").exit_code(), 5);
        assert_eq!(SubXError::file_matching("test").exit_code(), 6);
    }

    #[test]
    fn test_user_friendly_messages() {
        let config_error = SubXError::config("API 金鑰未設定");
        let message = config_error.user_friendly_message();
        assert!(message.contains("配置錯誤"));
        assert!(message.contains("subx config --help"));

        let ai_error = SubXError::ai_service("網路連接失敗");
        let message = ai_error.user_friendly_message();
        assert!(message.contains("AI 服務錯誤"));
        assert!(message.contains("檢查網路連接"));
    }
}
```

### 配置管理系統測試實作
```rust
// src/config.rs (測試模組)
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::env;

    #[test]
    fn test_default_config_creation() {
        let config = Config::default();
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4o-mini");
        assert_eq!(config.formats.default_output, "srt");
        assert_eq!(config.general.default_confidence, 80);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("[ai]"));
        assert!(toml_str.contains("[formats]"));
        assert!(toml_str.contains("[sync]"));
        assert!(toml_str.contains("[general]"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_content = r#"
[ai]
provider = "openai"
model = "gpt-4"
max_sample_length = 1500

[formats]
default_output = "vtt"
preserve_styling = false

[sync]
max_offset_seconds = 60.0

[general]
backup_enabled = true
default_confidence = 90
"#;
        let config: Config = toml::from_str(toml_content).unwrap();
        assert_eq!(config.ai.model, "gpt-4");
        assert_eq!(config.formats.default_output, "vtt");
        assert!(!config.formats.preserve_styling);
        assert_eq!(config.sync.max_offset_seconds, 60.0);
        assert!(config.general.backup_enabled);
    }

    #[test]
    fn test_env_var_override() {
        env::set_var("OPENAI_API_KEY", "test-key-123");
        env::set_var("SUBX_AI_MODEL", "gpt-3.5-turbo");

        let mut config = Config::default();
        config.apply_env_vars();

        assert_eq!(config.ai.api_key, Some("test-key-123".to_string()));
        assert_eq!(config.ai.model, "gpt-3.5-turbo");

        env::remove_var("OPENAI_API_KEY");
        env::remove_var("SUBX_AI_MODEL");
    }

    #[test]
    fn test_config_validation_missing_api_key() {
        env::remove_var("OPENAI_API_KEY");
        let config = Config::default();
        // API Key 驗證應該在特定命令執行時進行，不在載入時
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_provider() {
        let mut config = Config::default();
        config.ai.provider = "invalid-provider".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_file_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let original_config = Config::default();
        
        // 模擬儲存配置檔案
        let toml_content = toml::to_string_pretty(&original_config).unwrap();
        std::fs::write(&config_path, toml_content).unwrap();

        // 測試載入
        let file_content = std::fs::read_to_string(&config_path).unwrap();
        let loaded_config: Config = toml::from_str(&file_content).unwrap();

        assert_eq!(original_config.ai.model, loaded_config.ai.model);
        assert_eq!(original_config.formats.default_output, loaded_config.formats.default_output);
    }

    #[test]
    fn test_config_merge() {
        let mut base_config = Config::default();
        let mut override_config = Config::default();
        override_config.ai.model = "gpt-4".to_string();
        override_config.general.backup_enabled = true;

        base_config.merge(override_config);

        assert_eq!(base_config.ai.model, "gpt-4");
        assert!(base_config.general.backup_enabled);
    }
}
```

### 字幕格式解析引擎測試實作
```rust
// src/core/formats/srt.rs (擴展測試模組)
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::formats::{SubtitleFormat, SubtitleFormatType};

    const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:03,000\nHello, World!\n\n2\n00:00:05,000 --> 00:00:08,000\nThis is a test subtitle.\n多行測試\n\n";

    #[test]
    fn test_srt_parsing_basic() {
        let format = SrtFormat;
        let subtitle = format.parse(SAMPLE_SRT).unwrap();

        assert_eq!(subtitle.entries.len(), 2);
        assert_eq!(subtitle.format, SubtitleFormatType::Srt);

        let first_entry = &subtitle.entries[0];
        assert_eq!(first_entry.index, 1);
        assert_eq!(first_entry.start_time, Duration::from_millis(1000));
        assert_eq!(first_entry.end_time, Duration::from_millis(3000));
        assert_eq!(first_entry.text, "Hello, World!");

        let second_entry = &subtitle.entries[1];
        assert_eq!(second_entry.index, 2);
        assert_eq!(second_entry.start_time, Duration::from_millis(5000));
        assert_eq!(second_entry.end_time, Duration::from_millis(8000));
        assert_eq!(second_entry.text, "This is a test subtitle.\n多行測試");
    }

    #[test]
    fn test_srt_serialization() {
        let format = SrtFormat;
        let subtitle = format.parse(SAMPLE_SRT).unwrap();
        let serialized = format.serialize(&subtitle).unwrap();

        // 重新解析序列化結果
        let reparsed = format.parse(&serialized).unwrap();
        assert_eq!(subtitle.entries.len(), reparsed.entries.len());

        for (original, reparsed) in subtitle.entries.iter().zip(reparsed.entries.iter()) {
            assert_eq!(original.start_time, reparsed.start_time);
            assert_eq!(original.end_time, reparsed.end_time);
            assert_eq!(original.text, reparsed.text);
        }
    }

    #[test]
    fn test_srt_detection() {
        let format = SrtFormat;
        assert!(format.detect(SAMPLE_SRT));
        assert!(!format.detect("This is not SRT content"));
        assert!(!format.detect("WEBVTT\n\n00:00:01.000 --> 00:00:03.000\nHello"));
    }

    #[test]
    fn test_srt_invalid_format() {
        let format = SrtFormat;
        
        // 測試無效時間格式
        let invalid_time = "1\n00:00:01 --> 00:00:03\nText\n\n";
        assert!(format.parse(invalid_time).is_err());

        // 測試無效序列號
        let invalid_index = "invalid\n00:00:01,000 --> 00:00:03,000\nText\n\n";
        assert!(format.parse(invalid_index).is_err());
    }

    #[test]
    fn test_srt_empty_content() {
        let format = SrtFormat;
        let subtitle = format.parse("").unwrap();
        assert_eq!(subtitle.entries.len(), 0);

        let subtitle = format.parse("\n\n\n").unwrap();
        assert_eq!(subtitle.entries.len(), 0);
    }

    #[test]
    fn test_srt_malformed_blocks() {
        let format = SrtFormat;
        
        // 測試缺少文字內容的塊
        let malformed = "1\n00:00:01,000 --> 00:00:03,000\n\n";
        let subtitle = format.parse(malformed).unwrap();
        assert_eq!(subtitle.entries.len(), 0); // 應該跳過格式錯誤的塊
    }

    #[test]
    fn test_time_parsing_edge_cases() {
        let format = SrtFormat;
        
        // 測試邊界時間值
        let edge_case = "1\n23:59:59,999 --> 23:59:59,999\nEnd of day\n\n";
        let subtitle = format.parse(edge_case).unwrap();
        assert_eq!(subtitle.entries.len(), 1);
        
        let entry = &subtitle.entries[0];
        let expected_duration = Duration::from_millis(23 * 3600000 + 59 * 60000 + 59 * 1000 + 999);
        assert_eq!(entry.start_time, expected_duration);
        assert_eq!(entry.end_time, expected_duration);
    }

    #[test]
    fn test_file_extensions() {
        let format = SrtFormat;
        assert_eq!(format.file_extensions(), &["srt"]);
    }

    #[test]
    fn test_format_name() {
        let format = SrtFormat;
        assert_eq!(format.format_name(), "SRT");
    }
}
```

### AI 服務整合模擬測試實作
```rust
// src/services/ai/openai.rs (模擬測試模組)
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::{predicate::*, mock};
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, header};
    use serde_json::json;

    // 建立 AI Provider 的模擬版本
    mock! {
        AIClient {}

        #[async_trait]
        impl AIProvider for AIClient {
            async fn analyze_content(&self, request: AnalysisRequest) -> crate::Result<MatchResult>;
            async fn verify_match(&self, verification: VerificationRequest) -> crate::Result<ConfidenceScore>;
        }
    }

    #[tokio::test]
    async fn test_openai_client_creation() {
        let client = OpenAIClient::new(
            "test-api-key".to_string(),
            "gpt-4o-mini".to_string(),
        );
        assert_eq!(client.api_key, "test-api-key");
        assert_eq!(client.model, "gpt-4o-mini");
    }

    #[tokio::test]
    async fn test_chat_completion_success() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{
                    "message": {
                        "content": "測試回應內容"
                    }
                }]
            })))
            .mount(&mock_server)
            .await;

        let mut client = OpenAIClient::new(
            "test-api-key".to_string(),
            "gpt-4o-mini".to_string(),
        );
        client.base_url = mock_server.uri();

        let messages = vec![
            json!({"role": "user", "content": "測試訊息"})
        ];

        let response = client.chat_completion(messages).await.unwrap();
        assert_eq!(response, "測試回應內容");
    }

    #[tokio::test]
    async fn test_chat_completion_error() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": {
                    "message": "Invalid API key"
                }
            })))
            .mount(&mock_server)
            .await;

        let mut client = OpenAIClient::new(
            "invalid-key".to_string(),
            "gpt-4o-mini".to_string(),
        );
        client.base_url = mock_server.uri();

        let messages = vec![
            json!({"role": "user", "content": "測試訊息"})
        ];

        let result = client.chat_completion(messages).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_analyze_content() {
        let mut mock_client = MockAIClient::new();
        
        let expected_request = AnalysisRequest {
            video_files: vec!["video1.mp4".to_string()],
            subtitle_files: vec!["subtitle1.srt".to_string()],
            content_samples: vec![],
        };

        let expected_result = MatchResult {
            matches: vec![FileMatch {
                video_file: "video1.mp4".to_string(),
                subtitle_file: "subtitle1.srt".to_string(),
                confidence: 0.95,
                match_factors: vec!["檔名相似".to_string()],
            }],
            confidence: 0.95,
            reasoning: "檔名模式匹配".to_string(),
        };

        mock_client
            .expect_analyze_content()
            .with(eq(expected_request))
            .times(1)
            .returning(move |_| Ok(expected_result.clone()));

        let request = AnalysisRequest {
            video_files: vec!["video1.mp4".to_string()],
            subtitle_files: vec!["subtitle1.srt".to_string()],
            content_samples: vec![],
        };

        let result = mock_client.analyze_content(request).await.unwrap();
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.confidence, 0.95);
    }

    #[test]
    fn test_prompt_building() {
        let client = OpenAIClient::new(
            "test-key".to_string(),
            "gpt-4o-mini".to_string(),
        );

        let request = AnalysisRequest {
            video_files: vec!["Season 1 Episode 1.mp4".to_string()],
            subtitle_files: vec!["S01E01.srt".to_string()],
            content_samples: vec![ContentSample {
                filename: "S01E01.srt".to_string(),
                content_preview: "Hello, world!".to_string(),
                file_size: 1024,
                language_hint: Some("en".to_string()),
            }],
        };

        let prompt = client.build_analysis_prompt(&request);
        assert!(prompt.contains("Season 1 Episode 1.mp4"));
        assert!(prompt.contains("S01E01.srt"));
        assert!(prompt.contains("Hello, world!"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_match_result_parsing() {
        let client = OpenAIClient::new(
            "test-key".to_string(),
            "gpt-4o-mini".to_string(),
        );

        let json_response = r#"
        {
            "matches": [
                {
                    "video_file": "video.mp4",
                    "subtitle_file": "subtitle.srt",
                    "confidence": 0.9,
                    "match_factors": ["檔名相似", "內容匹配"]
                }
            ],
            "confidence": 0.9,
            "reasoning": "高度匹配"
        }
        "#;

        let result = client.parse_match_result(json_response).unwrap();
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.confidence, 0.9);
        assert_eq!(result.reasoning, "高度匹配");
        
        let file_match = &result.matches[0];
        assert_eq!(file_match.video_file, "video.mp4");
        assert_eq!(file_match.subtitle_file, "subtitle.srt");
        assert_eq!(file_match.confidence, 0.9);
        assert_eq!(file_match.match_factors.len(), 2);
    }
}
```

### 檔案匹配引擎測試實作
```rust
// src/core/matcher/discovery.rs (測試模組)
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_files(dir: &Path) -> std::io::Result<()> {
        fs::write(dir.join("video1.mp4"), b"")?;
        fs::write(dir.join("video2.mkv"), b"")?;
        fs::write(dir.join("video3.avi"), b"")?;
        fs::write(dir.join("subtitle1.srt"), b"")?;
        fs::write(dir.join("subtitle2.ass"), b"")?;
        fs::write(dir.join("subtitle3.vtt"), b"")?;
        fs::write(dir.join("document.txt"), b"")?; // 非媒體檔案
        fs::write(dir.join("image.jpg"), b"")?; // 非媒體檔案
        
        // 建立子目錄
        let subdir = dir.join("season1");
        fs::create_dir(&subdir)?;
        fs::write(subdir.join("episode1.mp4"), b"")?;
        fs::write(subdir.join("episode1.srt"), b"")?;
        
        Ok(())
    }

    #[test]
    fn test_file_discovery_non_recursive() {
        let temp_dir = TempDir::new().unwrap();
        create_test_files(temp_dir.path()).unwrap();

        let discovery = FileDiscovery::new();
        let files = discovery.scan_directory(temp_dir.path(), false).unwrap();

        let video_files: Vec<_> = files.iter()
            .filter(|f| matches!(f.file_type, MediaFileType::Video))
            .collect();
        let subtitle_files: Vec<_> = files.iter()
            .filter(|f| matches!(f.file_type, MediaFileType::Subtitle))
            .collect();

        assert_eq!(video_files.len(), 3); // mp4, mkv, avi
        assert_eq!(subtitle_files.len(), 3); // srt, ass, vtt
        
        // 確認沒有包含子目錄的檔案
        assert!(!files.iter().any(|f| f.name.contains("episode")));
    }

    #[test]
    fn test_file_discovery_recursive() {
        let temp_dir = TempDir::new().unwrap();
        create_test_files(temp_dir.path()).unwrap();

        let discovery = FileDiscovery::new();
        let files = discovery.scan_directory(temp_dir.path(), true).unwrap();

        let video_files: Vec<_> = files.iter()
            .filter(|f| matches!(f.file_type, MediaFileType::Video))
            .collect();
        let subtitle_files: Vec<_> = files.iter()
            .filter(|f| matches!(f.file_type, MediaFileType::Subtitle))
            .collect();

        assert_eq!(video_files.len(), 4); // 包含子目錄的 episode1.mp4
        assert_eq!(subtitle_files.len(), 4); // 包含子目錄的 episode1.srt
        
        // 確認包含子目錄的檔案
        assert!(files.iter().any(|f| f.name == "episode1"));
    }

    #[test]
    fn test_file_classification() {
        let temp_dir = TempDir::new().unwrap();
        let video_path = temp_dir.path().join("test.mp4");
        let subtitle_path = temp_dir.path().join("test.srt");
        let unknown_path = temp_dir.path().join("test.txt");

        fs::write(&video_path, b"").unwrap();
        fs::write(&subtitle_path, b"").unwrap();
        fs::write(&unknown_path, b"").unwrap();

        let discovery = FileDiscovery::new();

        let video_file = discovery.classify_file(&video_path).unwrap().unwrap();
        assert!(matches!(video_file.file_type, MediaFileType::Video));
        assert_eq!(video_file.name, "test");
        assert_eq!(video_file.extension, "mp4");

        let subtitle_file = discovery.classify_file(&subtitle_path).unwrap().unwrap();
        assert!(matches!(subtitle_file.file_type, MediaFileType::Subtitle));
        assert_eq!(subtitle_file.name, "test");
        assert_eq!(subtitle_file.extension, "srt");

        let unknown_file = discovery.classify_file(&unknown_path).unwrap();
        assert!(unknown_file.is_none());
    }

    #[test]
    fn test_supported_extensions() {
        let discovery = FileDiscovery::new();
        
        // 測試影片副檔名
        assert!(discovery.video_extensions.contains(&"mp4".to_string()));
        assert!(discovery.video_extensions.contains(&"mkv".to_string()));
        assert!(discovery.video_extensions.contains(&"avi".to_string()));
        
        // 測試字幕副檔名
        assert!(discovery.subtitle_extensions.contains(&"srt".to_string()));
        assert!(discovery.subtitle_extensions.contains(&"ass".to_string()));
        assert!(discovery.subtitle_extensions.contains(&"vtt".to_string()));
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let discovery = FileDiscovery::new();
        let files = discovery.scan_directory(temp_dir.path(), false).unwrap();
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_nonexistent_directory() {
        let discovery = FileDiscovery::new();
        let nonexistent = Path::new("/nonexistent/path");
        let result = discovery.scan_directory(nonexistent, false);
        assert!(result.is_err());
    }
}
```

### 測試覆蓋率 CI/CD 整合
```yaml
# .github/workflows/test-coverage.yml
name: Test Coverage

on:
  push:
    branches: [ master ]
  pull_request:
    branches: [ master ]

env:
  CARGO_TERM_COLOR: always

jobs:
  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust stable
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true
        components: llvm-tools-preview
    
    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Install cargo-tarpaulin
      run: cargo install cargo-tarpaulin
    
    - name: Run tests with coverage
      run: |
        cargo tarpaulin \
          --verbose \
          --all-features \
          --workspace \
          --timeout 120 \
          --out Html \
          --out Lcov \
          --output-dir coverage/ \
          --fail-under 50
    
    - name: Upload coverage to Codecov
      uses: codecov/codecov-action@v3
      with:
        files: coverage/lcov.info
        fail_ci_if_error: true
        verbose: true
    
    - name: Archive coverage results
      uses: actions/upload-artifact@v3
      with:
        name: coverage-report
        path: coverage/
    
    - name: Comment coverage on PR
      if: github.event_name == 'pull_request'
      uses: actions/github-script@v6
      with:
        script: |
          const fs = require('fs');
          try {
            const coverage = fs.readFileSync('coverage/tarpaulin-report.html', 'utf8');
            // 解析覆蓋率百分比並留言到 PR
            const coverageMatch = coverage.match(/(\d+\.?\d*)%/);
            if (coverageMatch) {
              const percentage = parseFloat(coverageMatch[1]);
              const message = `## 🧪 測試覆蓋率報告\n\n當前覆蓋率: **${percentage}%**\n\n${percentage >= 50 ? '✅ 達成目標覆蓋率 (≥50%)' : '❌ 未達成目標覆蓋率 (≥50%)'}`;
              
              github.rest.issues.createComment({
                issue_number: context.issue.number,
                owner: context.repo.owner,
                repo: context.repo.repo,
                body: message
              });
            }
          } catch (error) {
            console.log('無法讀取覆蓋率報告:', error);
          }
```

### 測試工具輔助函式
```rust
// tests/common/mod.rs
use tempfile::TempDir;
use std::fs;
use std::path::Path;

/// 測試用的媒體檔案生成器
pub struct TestMediaGenerator {
    pub temp_dir: TempDir,
}

impl TestMediaGenerator {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().unwrap(),
        }
    }

    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// 建立測試用的 SRT 字幕檔案
    pub fn create_srt_file(&self, name: &str, entries: &[(&str, &str, &str)]) -> PathBuf {
        let mut content = String::new();
        for (i, (start, end, text)) in entries.iter().enumerate() {
            content.push_str(&format!("{}\n{} --> {}\n{}\n\n", i + 1, start, end, text));
        }
        
        let path = self.path().join(format!("{}.srt", name));
        fs::write(&path, content).unwrap();
        path
    }

    /// 建立測試用的影片檔案（空檔案）
    pub fn create_video_file(&self, name: &str, extension: &str) -> PathBuf {
        let path = self.path().join(format!("{}.{}", name, extension));
        fs::write(&path, b"").unwrap();
        path
    }

    /// 建立測試用的配置檔案
    pub fn create_config_file(&self, config: &subx_cli::config::Config) -> PathBuf {
        let content = toml::to_string_pretty(config).unwrap();
        let path = self.path().join("config.toml");
        fs::write(&path, content).unwrap();
        path
    }
}

/// 測試用的 AI 回應模擬器
pub struct MockAIResponses;

impl MockAIResponses {
    pub fn successful_match_response() -> serde_json::Value {
        serde_json::json!({
            "matches": [
                {
                    "video_file": "video.mp4",
                    "subtitle_file": "subtitle.srt",
                    "confidence": 0.95,
                    "match_factors": ["檔名相似", "內容匹配"]
                }
            ],
            "confidence": 0.95,
            "reasoning": "檔名模式高度相似"
        })
    }

    pub fn low_confidence_response() -> serde_json::Value {
        serde_json::json!({
            "matches": [
                {
                    "video_file": "video.mp4",
                    "subtitle_file": "subtitle.srt",
                    "confidence": 0.3,
                    "match_factors": ["部分檔名相似"]
                }
            ],
            "confidence": 0.3,
            "reasoning": "匹配度較低"
        })
    }

    pub fn no_match_response() -> serde_json::Value {
        serde_json::json!({
            "matches": [],
            "confidence": 0.0,
            "reasoning": "找不到合適的匹配"
        })
    }
}

/// 斷言輔助巨集
#[macro_export]
macro_rules! assert_subtitle_entry {
    ($entry:expr, $index:expr, $start:expr, $end:expr, $text:expr) => {
        assert_eq!($entry.index, $index);
        assert_eq!($entry.start_time, std::time::Duration::from_millis($start));
        assert_eq!($entry.end_time, std::time::Duration::from_millis($end));
        assert_eq!($entry.text, $text);
    };
}
```

### 整合測試擴展
```rust
// tests/integration_tests.rs (擴展)
mod common;

use common::TestMediaGenerator;
use assert_cmd::Command;
use predicates::prelude::*;
use std::env;

#[test]
fn test_config_command_integration() {
    let test_gen = TestMediaGenerator::new();
    
    // 測試設定配置值
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.env("SUBX_CONFIG_PATH", test_gen.path().join("config.toml"))
        .arg("config")
        .arg("set")
        .arg("ai.model")
        .arg("gpt-4")
        .assert()
        .success()
        .stdout(predicate::str::contains("gpt-4"));

    // 測試讀取配置值
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.env("SUBX_CONFIG_PATH", test_gen.path().join("config.toml"))
        .arg("config")
        .arg("get")
        .arg("ai.model")
        .assert()
        .success()
        .stdout(predicate::str::contains("gpt-4"));
}

#[test]
fn test_convert_command_integration() {
    let test_gen = TestMediaGenerator::new();
    let srt_file = test_gen.create_srt_file("test", &[
        ("00:00:01,000", "00:00:03,000", "Hello"),
        ("00:00:05,000", "00:00:07,000", "World"),
    ]);

    // 測試轉換為 VTT
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.arg("convert")
        .arg(&srt_file)
        .arg("--format")
        .arg("vtt")
        .assert()
        .success();

    // 檢查輸出檔案
    let vtt_file = srt_file.with_extension("vtt");
    assert!(vtt_file.exists());
    
    let content = std::fs::read_to_string(vtt_file).unwrap();
    assert!(content.contains("WEBVTT"));
}

#[test]
fn test_error_handling() {
    // 測試不存在的路徑
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.arg("match")
        .arg("/nonexistent/path")
        .assert()
        .failure()
        .stderr(predicate::str::contains("指定路徑不存在"));

    // 測試無效的格式
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.arg("convert")
        .arg("nonexistent.srt")
        .arg("--format")
        .arg("invalid")
        .assert()
        .failure();
}

#[test] 
fn test_help_messages() {
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("智慧字幕處理 CLI 工具"));

    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.arg("match")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI 匹配重命名字幕文件"));
}
```

## 驗收標準

### 1. 測試覆蓋率目標
- [ ] **整體測試覆蓋率達成 50% 以上**
- [ ] **核心模組測試覆蓋率達成 70% 以上：**
  - 錯誤處理模組 (error.rs)
  - 配置管理模組 (config.rs)  
  - 字幕格式解析引擎 (core/formats/)
  - 檔案匹配引擎 (core/matcher/)
- [ ] **服務模組測試覆蓋率達成 60% 以上：**
  - AI 服務整合 (services/ai/)
  - 音訊處理服務 (services/audio/)

### 2. 測試品質要求
- [ ] **所有單元測試必須獨立執行**
- [ ] **測試必須具有確定性（不依賴外部服務）**
- [ ] **模擬測試正確使用 mockall 框架**
- [ ] **測試資料使用 tempfile 進行隔離**
- [ ] **每個測試都有清楚的斷言和錯誤訊息**

### 3. 測試執行要求
- [ ] **cargo test 執行所有測試無錯誤**
- [ ] **cargo clippy -- -D warnings 無警告**
- [ ] **測試執行時間 < 60 秒**
- [ ] **並行測試執行無衝突**

### 4. CI/CD 整合要求
- [ ] **GitHub Actions 自動執行覆蓋率檢查**
- [ ] **PR 自動留言覆蓋率報告**
- [ ] **覆蓋率低於 50% 時 CI 失敗**
- [ ] **覆蓋率報告上傳至 Codecov**

### 5. 文件和維護性
- [ ] **測試程式碼有適當的註釋說明**
- [ ] **測試輔助函式文件完整**
- [ ] **測試資料生成器易於使用**
- [ ] **模擬物件設定清楚明確**

## 估計工時
**6-8 天**

### 工時分配：
- 測試基礎設施建立：1 天
- 核心模組測試實作：2-3 天
- 服務模組模擬測試：2 天
- CI/CD 整合和優化：1 天
- 測試覆蓋率達標調整：1-2 天

## 相依性
- 依賴 Backlog #01-11 (所有功能模組已實作完成)

## 風險評估
- **中風險：** 測試覆蓋率目標可能需要多次調整
- **注意事項：**
  - 模擬外部服務（OpenAI API）的複雜度
  - 音訊處理測試的資源需求
  - 並行測試的同步問題
  - 測試資料的管理和清理

## 成功指標
1. **整體測試覆蓋率 ≥ 50%**
2. **核心模組覆蓋率 ≥ 70%**
3. **所有測試穩定通過**
4. **CI/CD 自動化覆蓋率檢查正常運作**
5. **測試維護成本合理**
