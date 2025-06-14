# Backlog 33: 完成 Sync 架構驗證與修復

## 概覽

基於 [程式碼審查報告 #138](../reports/138-backlog-32-sync-architecture-redesign-code-review.md) 的發現，Backlog #32 的核心架構已完成 75-80%，但仍有關鍵問題需要修復以確保功能完整性。

本 backlog 專注於修復已識別的問題，完成 #32 原始需求，不新增額外功能。每個任務都提供詳細的執行步驟和技術指引，確保工作能夠順利完成。

## 核心問題與解決策略

根據程式碼審查，需要修復以下問題：

1. **測試問題**：大量整合測試標記為 `#[ignore]`，導致 CI 無法驗證功能正確性
2. **回退機制未完全驗證**：Whisper-VAD 回退邏輯實作完成但缺乏測試確認
3. **VAD 測試環境依賴**：現有測試依賴外部音訊檔案，不適合 CI 環境
4. **基本文檔缺失**：使用者無法根據現有文檔成功配置和使用新功能

## 技術背景說明

### 專案結構概述
```
src/
├── services/
│   ├── whisper/          # Whisper API 整合模組
│   │   ├── client.rs     # API 客戶端
│   │   ├── detection.rs  # 同步檢測邏輯
│   │   └── mod.rs        # 模組定義
│   └── vad/              # 本地 VAD 模組
│       ├── detector.rs   # VAD 檢測器
│       └── mod.rs        # 模組定義
├── core/sync/
│   └── engine.rs         # 統一同步引擎
└── config/mod.rs         # 配置結構定義

tests/
├── whisper_integration_tests.rs  # Whisper 整合測試
├── vad_integration_tests.rs      # VAD 整合測試
└── sync_engine_integration_tests.rs  # 同步引擎測試
```

### 關鍵配置結構
```rust
// src/config/mod.rs 中的關鍵結構
pub struct SyncConfig {
    pub default_method: String,           // "whisper" 或 "vad"
    pub analysis_window_seconds: u32,     // 預設 30 秒
    pub max_offset_seconds: f64,          // 預設 60.0 秒
    pub whisper: WhisperConfig,
    pub vad: VadConfig,
}

pub struct WhisperConfig {
    pub enabled: bool,
    pub fallback_to_vad: bool,           // 關鍵：回退機制開關
    pub min_confidence_threshold: f64,   // 預設 0.7
    // ... 其他配置
}
```

## 詳細修復任務

### 任務 1：修復整合測試 
**預估工時**：4-5 小時  
**目標**：讓所有標記為 `#[ignore]` 的整合測試能夠正常執行並通過

#### 1.1 檢查現有測試狀況

**步驟 1**：檢查當前被忽略的測試
```bash
# 在專案根目錄執行
grep -r "#\[ignore\]" tests/ --include="*.rs"
```

**預期結果**：應該會顯示類似以下的檔案和行號
```
tests/whisper_integration_tests.rs:15:#[ignore]
tests/vad_integration_tests.rs:23:#[ignore]
```

**步驟 2**：執行測試以了解失敗原因
```bash
# 執行被忽略的測試
cargo test -- --ignored
```

**常見失敗原因分析**：
- 缺少 `OPENAI_API_KEY` 環境變數
- 測試依賴外部音訊檔案不存在
- 網路連線問題導致 API 呼叫失敗
- 測試設定的路徑或參數不正確

#### 1.2 修復 Whisper 整合測試

**檔案位置**：`tests/whisper_integration_tests.rs`

**重要**：我們將使用 wiremock 來模擬 Whisper API，不進行實際的 API 呼叫。

**步驟 1**：檢查測試結構
```bash
# 查看測試檔案內容
cat tests/whisper_integration_tests.rs
```

**步驟 2**：使用 wiremock 模擬 API 呼叫
在 `tests/whisper_integration_tests.rs` 中實作：

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header};
use serde_json::json;
use subx_cli::services::whisper::WhisperApiClient;
use subx_cli::config::WhisperConfig;
use std::path::Path;

#[tokio::test]
async fn test_whisper_sync_detection_with_mock() -> Result<(), Box<dyn std::error::Error>> {
    // 建立 mock 伺服器
    let mock_server = MockServer::start().await;
    
    // 設定 mock 回應
    Mock::given(method("POST"))
        .and(path("/audio/transcriptions"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "text": "Hello world this is a test",
                "segments": [{
                    "start": 0.5,
                    "end": 2.0,
                    "text": "Hello world this is a test"
                }],
                "words": [
                    {"word": "Hello", "start": 0.5, "end": 1.0},
                    {"word": "world", "start": 1.0, "end": 1.5}
                ]
            })))
        .mount(&mock_server)
        .await;
    
    // 建立測試配置
    let config = WhisperConfig {
        enabled: true,
        model: "whisper-1".to_string(),
        language: "auto".to_string(),
        temperature: 0.0,
        timeout_seconds: 30,
        fallback_to_vad: true,
        min_confidence_threshold: 0.7,
        max_retries: 1,
        retry_delay_ms: 100,
    };
    
    // 建立客戶端（使用 mock 伺服器的 URL）
    let client = WhisperApiClient::new(
        "test-key".to_string(),
        mock_server.uri(),
        config
    )?;
    
    // 使用準備好的測試音訊檔案
    let test_audio_path = Path::new("tests/data/test_speech.wav");
    
    // 執行測試
    let result = client.transcribe(&test_audio_path).await?;
    
    // 驗證結果
    assert_eq!(result.text, "Hello world this is a test");
    assert!(!result.segments.is_empty());
    assert!(result.words.is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_whisper_api_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    // 測試 API 錯誤處理
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(429)
            .set_body_json(json!({
                "error": {
                    "message": "Rate limit exceeded",
                    "type": "rate_limit_error"
                }
            })))
        .mount(&mock_server)
        .await;
    
    let config = WhisperConfig::default();
    let client = WhisperApiClient::new(
        "test-key".to_string(),
        mock_server.uri(),
        config
    )?;
    
    let test_audio_path = Path::new("tests/data/test_speech.wav");
    let result = client.transcribe(&test_audio_path).await;
    
    // 驗證錯誤處理
    assert!(result.is_err());
    
    Ok(())
}
```

**步驟 3**：評估 Whisper 客戶端重構需求
檢查現有的 `WhisperApiClient` 是否需要修改以支援 mock URL：

```bash
# 檢查現有的客戶端建構函數
grep -A 10 "pub fn new" src/services/whisper/client.rs
```

**重要發現**：現有的 `WhisperApiClient::new` 已經接受 `base_url` 參數，因此**不需要額外重構**就可以使用 wiremock。

**步驟 4**：驗證現有 wiremock 整合
```bash
# 確認現有的 mock 測試正常運作
cargo test whisper_mock --test whisper_mock_tests
```

#### 1.3 修復 VAD 整合測試

**檔案位置**：`tests/vad_integration_tests.rs`

**步驟 1**：移除外部檔案依賴，使用手動準備的音訊檔案
將原本依賴外部音訊檔案的測試改為使用預先準備的測試音訊：

```rust
#[tokio::test]
async fn test_vad_detection_with_prepared_audio() -> Result<(), Box<dyn std::error::Error>> {
    // 建立 VAD 配置
    let config = VadConfig {
        enabled: true,
        sensitivity: 0.75,
        chunk_size: 512,
        sample_rate: 16000,
        padding_chunks: 3,
    };
    
    // 使用手動準備的測試音訊檔案
    let test_audio_path = Path::new("tests/data/vad_test_speech.wav");
    
    // 確保測試檔案存在
    if !test_audio_path.exists() {
        panic!("Test audio file not found: {:?}. Please prepare the required test audio files.", test_audio_path);
    }
    
    // 載入音訊檔案
    let audio_data = load_audio_file(test_audio_path)?;
    
    // 執行 VAD 檢測
    let detector = VadDetector::new(&config)?;
    let detection_result = detector.detect_voice_activity(&audio_data)?;
    
    // 驗證結果
    assert!(!detection_result.voice_segments.is_empty());
    assert!(detection_result.voice_segments[0].start_time >= 0.0);
    
    Ok(())
}

#[tokio::test]
async fn test_vad_with_silence() -> Result<(), Box<dyn std::error::Error>> {
    let config = VadConfig {
        enabled: true,
        sensitivity: 0.75,
        chunk_size: 512,
        sample_rate: 16000,
        padding_chunks: 3,
    };
    
    // 使用手動準備的靜音檔案
    let silence_audio_path = Path::new("tests/data/vad_test_silence.wav");
    
    if !silence_audio_path.exists() {
        panic!("Test silence file not found: {:?}. Please prepare the required test audio files.", silence_audio_path);
    }
    
    let audio_data = load_audio_file(silence_audio_path)?;
    
    let detector = VadDetector::new(&config)?;
    let result = detector.detect_voice_activity(&audio_data)?;
    
    // 驗證：靜音應該不被檢測為語音
    assert!(result.voice_segments.is_empty(), "Silence should not be detected as speech");
    
    Ok(())
}

#[tokio::test]
async fn test_vad_basic_integration() -> Result<(), Box<dyn std::error::Error>> {
    // 測試 VAD 模組的基本整合功能，不關注性能細節
    let config = VadConfig {
        enabled: true,
        sensitivity: 0.75,
        chunk_size: 512,
        sample_rate: 16000,
        padding_chunks: 3,
    };
    
    // 使用包含語音和靜音的混合檔案
    let mixed_audio_path = Path::new("tests/data/vad_test_mixed.wav");
    
    if !mixed_audio_path.exists() {
        println!("Mixed audio test file not found, skipping test");
        return Ok(());
    }
    
    let audio_data = load_audio_file(mixed_audio_path)?;
    let detector = VadDetector::new(&config)?;
    let result = detector.detect_voice_activity(&audio_data)?;
    
    // 基本驗證：確保 VAD 能夠運作並返回合理結果
    // 不驗證性能指標，只驗證功能正確性
    assert!(result.voice_segments.len() <= audio_data.len() / config.chunk_size);
    
    Ok(())
}

fn load_audio_file(path: &Path) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // 簡化的音訊檔案載入函數
    // 實際實作會根據音訊格式進行解碼
    use std::fs;
    
    // 這裡只是示例，實際需要使用音訊解碼庫
    let _file_content = fs::read(path)?;
    
    // 暫時返回測試用的假資料
    // 實際實作需要解碼音訊檔案
    Ok(vec![0.0; 16000]) // 1 秒的靜音
}
```

#### 1.4 執行驗證

**步驟 1**：移除 `#[ignore]` 標記
```bash
# 在修復測試後，移除 ignore 標記
sed -i 's/#\[ignore\]//g' tests/whisper_integration_tests.rs
sed -i 's/#\[ignore\]//g' tests/vad_integration_tests.rs
```

**步驟 2**：執行測試驗證
```bash
# 執行所有測試
cargo test

# 只執行整合測試
cargo test --test whisper_integration_tests
cargo test --test vad_integration_tests
```

**預期結果**：
- 有 API 金鑰時：Whisper 測試通過
- 無 API 金鑰時：Whisper 測試跳過但不失敗
- VAD 測試始終能夠通過（使用合成音訊）

#### 1.5 驗收標準檢查清單

- [ ] `cargo test` 執行無 `#[ignore]` 相關警告
- [ ] Whisper 測試在有 API 金鑰時能通過
- [ ] Whisper 測試在無 API 金鑰時能優雅跳過
- [ ] VAD 測試能穩定通過，不依賴外部檔案
- [ ] 所有測試能在 CI 環境中執行
- [ ] 測試覆蓋主要的成功和失敗場景

### 任務 2：驗證 Whisper-VAD 回退機制
**預估工時**：2-3 小時  
**目標**：確認 Whisper API 失敗時能正確回退到 VAD（使用 wiremock 模擬）

#### 2.1 理解回退機制的設計

**步驟 1**：檢查回退邏輯實作
```bash
# 查看同步引擎的回退實作
grep -A 20 -B 5 "fallback_to_vad" src/core/sync/engine.rs
```

**步驟 2**：確認配置結構
```bash
# 檢查配置中的回退設定
grep -A 10 -B 5 "fallback_to_vad" src/config/mod.rs
```

**預期發現**：應該能找到類似以下的邏輯
```rust
// 在 src/core/sync/engine.rs 中
if config.whisper.enabled && config.whisper.fallback_to_vad {
    match whisper_detection_result {
        Ok(result) if result.confidence >= config.whisper.min_confidence_threshold => {
            return Ok(result);
        }
        _ => {
            // 回退到 VAD
            return self.detect_with_vad(audio_data, config).await;
        }
    }
}
```

#### 2.2 建立回退機制測試（使用 wiremock 模擬失敗）

**檔案位置**：建立新檔案 `tests/sync_fallback_integration_tests.rs`

**步驟 1**：建立測試檔案結構
```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};
use serde_json::json;
use subx::config::{SyncConfig, WhisperConfig, VadConfig};
use subx::core::sync::SyncEngine;
use std::path::Path;

#[tokio::test]
async fn test_whisper_api_failure_fallback() -> Result<(), Box<dyn std::error::Error>> {
    // 建立 mock 伺服器模擬 API 失敗
    let mock_server = MockServer::start().await;
    
    // 設定 API 失敗回應
    Mock::given(method("POST"))
        .and(path("/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(500)
            .set_body_json(json!({
                "error": {
                    "message": "Internal server error",
                    "type": "server_error"
                }
            })))
        .mount(&mock_server)
        .await;
    
    // 建立配置，啟用回退機制
    let config = create_fallback_test_config(mock_server.uri());
    
    // 準備測試音訊（手動準備的檔案）
    let test_audio_path = Path::new("tests/data/fallback_test.wav");
    
    if !test_audio_path.exists() {
        panic!("Fallback test audio file not found. Please prepare: {:?}", test_audio_path);
    }
    
    // 執行同步檢測
    let engine = SyncEngine::new();
    let result = engine.detect_sync_point(&test_audio_path, &config).await;
    
    // 驗證：即使 Whisper 失敗，仍應通過 VAD 得到結果
    assert!(result.is_ok());
    let sync_result = result.unwrap();
    assert_eq!(sync_result.method_used, "vad"); // 確認使用了 VAD
    assert!(sync_result.offset_seconds.is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_whisper_low_confidence_fallback() -> Result<(), Box<dyn std::error::Error>> {
    // 建立 mock 伺服器回傳低品質轉錄結果
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({
                "text": "uh um er",  // 低品質轉錄結果
                "segments": [{
                    "start": 0.1,
                    "end": 0.5,
                    "text": "uh um er"
                }],
                "words": []  // 沒有詳細的詞彙時間戳
            })))
        .mount(&mock_server)
        .await;
    
    // 設定很高的信心度閾值
    let mut config = create_fallback_test_config(mock_server.uri());
    config.whisper.min_confidence_threshold = 0.9; // 設定很高的閾值
    
    let test_audio_path = Path::new("tests/data/fallback_test.wav");
    if !test_audio_path.exists() {
        panic!("Fallback test audio file not found. Please prepare: {:?}", test_audio_path);
    }
    
    let engine = SyncEngine::new();
    let result = engine.detect_sync_point(&test_audio_path, &config).await;
    
    // 驗證回退行為
    match result {
        Ok(sync_result) => {
            // 如果成功，應該是 VAD 提供的結果
            assert_eq!(sync_result.method_used, "vad");
        }
        Err(_) => {
            // 如果失敗，確認是合理的失敗原因
            // 這裡可以檢查錯誤訊息是否包含回退相關資訊
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_fallback_disabled() -> Result<(), Box<dyn std::error::Error>> {
    // 測試當回退機制關閉時的行為
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;
    
    let mut config = create_fallback_test_config(mock_server.uri());
    config.whisper.fallback_to_vad = false; // 關閉回退
    
    let test_audio_path = Path::new("tests/data/fallback_test.wav");
    if !test_audio_path.exists() {
        println!("Fallback test audio file not found, skipping test");
        return Ok(());
    }
    
    let engine = SyncEngine::new();
    let result = engine.detect_sync_point(&test_audio_path, &config).await;
    
    // 驗證：應該失敗，不會回退到 VAD
    assert!(result.is_err());
    
    Ok(())
}

fn create_fallback_test_config(mock_url: String) -> SyncConfig {
    SyncConfig {
        default_method: "whisper".to_string(),
        analysis_window_seconds: 30,
        max_offset_seconds: 60.0,
        whisper: WhisperConfig {
            enabled: true,
            model: "whisper-1".to_string(),
            language: "auto".to_string(),
            temperature: 0.0,
            timeout_seconds: 30,
            fallback_to_vad: true, // 關鍵：啟用回退
            min_confidence_threshold: 0.7,
            max_retries: 1,
            retry_delay_ms: 100,
            // 使用 mock 伺服器 URL
            api_base_url: mock_url,
        },
        vad: VadConfig {
            enabled: true,
            sensitivity: 0.75,
            chunk_size: 512,
            sample_rate: 16000,
            padding_chunks: 3,
        },
    }
}
```

#### 2.3 測試回退日誌記錄

**步驟 1**：驗證回退過程有適當的日誌
```rust
#[tokio::test]
async fn test_fallback_logging() -> Result<(), Box<dyn std::error::Error>> {
    // 設定日誌捕獲
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();
    
    // 執行會觸發回退的測試
    let config = create_fallback_test_config();
    let engine = SyncEngine::new();
    let test_audio = create_test_audio_for_fallback()?;
    
    let result = engine.detect_sync_point(&test_audio, &config).await;
    
    // 這裡可以檢查日誌輸出是否包含回退相關訊息
    // 例如 "Whisper detection failed, falling back to VAD"
    
    Ok(())
}
```

#### 2.4 驗收標準檢查清單

- [ ] API 失敗時能正確回退到 VAD
- [ ] 低信心度時能正確回退到 VAD  
- [ ] 回退過程有適當的日誌記錄
- [ ] 回退後的結果格式與正常結果一致
- [ ] 配置 `fallback_to_vad = false` 時不會回退
- [ ] 回退機制的測試能穩定通過

### 任務 3：建立基本 VAD 測試
**預估工時**：1-2 小時  
**目標**：驗證 VAD 功能整合的正確性（不測試性能）

#### 3.1 分析現有 VAD 實作

**步驟 1**：檢查 VAD 模組結構
```bash
# 查看 VAD 模組檔案
find src/ -name "*vad*" -type f
ls -la src/services/vad/
```

**步驟 2**：理解 VAD 配置參數
```bash
# 檢查 VAD 配置結構
grep -A 15 "struct VadConfig" src/config/mod.rs
```

**預期發現**：
```rust
pub struct VadConfig {
    pub enabled: bool,
    pub sensitivity: f64,        // 0.0-1.0，影響檢測敏感度
    pub chunk_size: usize,       // 音訊處理塊大小
    pub sample_rate: usize,      // 取樣率
    pub padding_chunks: usize,   // 語音段前後的填充
}
```

#### 3.2 實作 VAD 基本整合測試

**檔案位置**：更新 `tests/vad_integration_tests.rs`

**重點**：我們只測試整合功能的正確性，不測試 VAD 本身的性能，因為那是外部 crate 的責任。

**步驟 1**：移除 `#[ignore]` 並實作基本測試
```rust
// tests/vad_integration_tests.rs
use subx::config::VadConfig;
use subx::services::vad::VadDetector;
use std::path::Path;

#[tokio::test]
async fn test_vad_basic_functionality() -> Result<(), Box<dyn std::error::Error>> {
    // 建立標準配置
    let config = VadConfig {
        enabled: true,
        sensitivity: 0.75,
        chunk_size: 512,
        sample_rate: 16000,
        padding_chunks: 3,
    };
    
    // 使用手動準備的測試音訊檔案
    let test_audio_path = Path::new("tests/data/vad_test_speech.wav");
    
    if !test_audio_path.exists() {
        panic!("VAD test audio file not found: {:?}. Please prepare the required test audio files.", test_audio_path);
    }
    
    // 載入音訊資料
    let audio_data = load_audio_data(test_audio_path)?;
    
    // 執行 VAD 檢測
    let detector = VadDetector::new(&config)?;
    let result = detector.detect_voice_activity(&audio_data)?;
    
    // 基本驗證：確保 VAD 整合功能正常
    // 不驗證檢測精度，只確保功能運作
    assert!(result.voice_segments.len() <= audio_data.len() / config.chunk_size);
    
    // 如果檢測到語音段，驗證基本結構
    if !result.voice_segments.is_empty() {
        let first_segment = &result.voice_segments[0];
        assert!(first_segment.start_time >= 0.0);
        assert!(first_segment.end_time > first_segment.start_time);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_vad_configuration_integration() -> Result<(), Box<dyn std::error::Error>> {
    // 測試不同配置參數能正常傳遞到 VAD 檢測器
    let configs = vec![
        VadConfig {
            enabled: true,
            sensitivity: 0.3,  // 低敏感度
            chunk_size: 256,
            sample_rate: 16000,
            padding_chunks: 1,
        },
        VadConfig {
            enabled: true,
            sensitivity: 0.9,  // 高敏感度
            chunk_size: 1024,
            sample_rate: 16000,
            padding_chunks: 5,
        },
    ];
    
    let test_audio_path = Path::new("tests/data/vad_test_speech.wav");
    if !test_audio_path.exists() {
        println!("VAD test audio file not found, skipping configuration test");
        return Ok(());
    }
    
    let audio_data = load_audio_data(test_audio_path)?;
    
    for config in configs {
        // 驗證每個配置都能成功建立檢測器
        let detector = VadDetector::new(&config)?;
        let result = detector.detect_voice_activity(&audio_data);
        
        // 確保配置不會導致錯誤
        assert!(result.is_ok(), "VAD should work with valid configuration");
    }
    
    Ok(())
}

#[tokio::test]
async fn test_vad_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    // 測試 VAD 的基本錯誤處理
    let config = VadConfig {
        enabled: true,
        sensitivity: 0.75,
        chunk_size: 512,
        sample_rate: 16000,
        padding_chunks: 3,
    };
    
    let detector = VadDetector::new(&config)?;
    
    // 測試空音訊資料
    let empty_audio: Vec<f32> = vec![];
    let result = detector.detect_voice_activity(&empty_audio);
    
    // VAD 應該能處理空輸入而不崩潰
    // 結果可能是錯誤或空的語音段列表，都是可接受的
    match result {
        Ok(detection_result) => {
            assert!(detection_result.voice_segments.is_empty());
        }
        Err(_) => {
            // 返回錯誤也是可接受的行為
        }
    }
    
    Ok(())
}

fn load_audio_data(path: &Path) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // 簡化的音訊載入函數
    // 實際實作需要根據具體的音訊處理庫來實作
    use std::fs;
    
    if !path.exists() {
        return Err(format!("Audio file not found: {:?}", path).into());
    }
    
    let _file_content = fs::read(path)?;
    
    // 這裡應該實作實際的音訊解碼
    // 目前返回測試用的假資料
    // 實際使用時需要替換為真正的音訊解碼邏輯
    Ok(vec![0.1; 16000]) // 1 秒的低音量測試資料
}
```

#### 3.4 驗收標準檢查清單

- [ ] VAD 能正確檢測合成語音音訊中的語音段
- [ ] VAD 不會將靜音誤判為語音
- [ ] 敏感度參數調整能產生預期的檢測行為差異
- [ ] VAD 在有背景噪音時仍能正常運作
- [ ] 所有測試不依賴外部音訊檔案
- [ ] 測試執行時間合理（每個測試 < 5 秒）

### 任務 4：補充基本使用文檔
**預估工時**：2-3 小時  
**目標**：提供使用者所需的基本配置和使用範例

#### 4.1 更新配置指南

**檔案位置**：`docs/configuration-guide.md`

**步驟 1**：檢查現有文檔結構
```bash
# 查看現有配置文檔
head -50 docs/configuration-guide.md
```

**步驟 2**：在適當位置加入 sync 配置說明

找到配置文檔中的適當位置（通常是在 `[sync]` 節段），加入以下詳細說明：

```markdown
## Sync 配置

SubX 提供兩種音訊同步方法：OpenAI Whisper API 和本地語音活動檢測 (VAD)。

### 基本配置

```toml
[sync]
# 預設同步方法："whisper" 或 "vad"
default_method = "whisper"

# 分析時間窗口：檢查第一句字幕前後的秒數
analysis_window_seconds = 30

# 最大允許偏移量（秒）
max_offset_seconds = 60.0
```

### Whisper API 方法配置

Whisper 方法使用 OpenAI 的語音轉錄 API 來精確檢測語音時間點。

```toml
[sync.whisper]
# 啟用 Whisper API 方法
enabled = true

# Whisper 模型選擇（推薦使用 "whisper-1"）
model = "whisper-1"

# 語言設定："auto" 為自動檢測，或指定語言代碼如 "en", "zh"
language = "auto"

# API 溫度參數：0.0-1.0，較低值產生更一致的結果
temperature = 0.0

# API 超時時間（秒）
timeout_seconds = 30

# 智能回退：當 Whisper 失敗時是否自動使用 VAD
fallback_to_vad = true

# 最低信心度閾值：低於此值時觸發回退
min_confidence_threshold = 0.7
```

**使用 Whisper 的前置需求：**
1. 設定環境變數 `OPENAI_API_KEY`
2. 確保網路連線正常
3. 有足夠的 OpenAI API 配額

### 本地 VAD 方法配置

VAD 方法完全在本地執行，不需要網路連線或 API 金鑰。

```toml
[sync.vad]
# 啟用本地 VAD 方法
enabled = true

# 語音檢測敏感度：0.0-1.0，較高值檢測更多語音
sensitivity = 0.75

# 音訊處理塊大小：影響處理精度和速度
chunk_size = 512

# 處理採樣率：通常使用 16000 Hz
sample_rate = 16000

# 語音段前後填充塊數：避免語音被截斷
padding_chunks = 3
```

### 配置範例

#### 範例 1：優先使用 Whisper，失敗時回退到 VAD
```toml
[sync]
default_method = "whisper"
analysis_window_seconds = 30
max_offset_seconds = 60.0

[sync.whisper]
enabled = true
model = "whisper-1"
language = "auto"
temperature = 0.0
timeout_seconds = 30
fallback_to_vad = true
min_confidence_threshold = 0.7

[sync.vad]
enabled = true
sensitivity = 0.75
chunk_size = 512
sample_rate = 16000
padding_chunks = 3
```

#### 範例 2：純本地 VAD 模式（無網路需求）
```toml
[sync]
default_method = "vad"
analysis_window_seconds = 30
max_offset_seconds = 60.0

[sync.whisper]
enabled = false

[sync.vad]
enabled = true
sensitivity = 0.8
chunk_size = 512
sample_rate = 16000
padding_chunks = 3
```

#### 範例 3：高精度模式（較長分析窗口）
```toml
[sync]
default_method = "whisper"
analysis_window_seconds = 60  # 更長的分析窗口
max_offset_seconds = 120.0

[sync.whisper]
enabled = true
model = "whisper-1"
language = "en"  # 指定語言以提高精度
temperature = 0.0
timeout_seconds = 45  # 較長的超時時間
fallback_to_vad = true
min_confidence_threshold = 0.8  # 較高的信心度要求

[sync.vad]
enabled = true
sensitivity = 0.9  # 高敏感度
chunk_size = 256   # 較小塊大小以提高精度
sample_rate = 16000
padding_chunks = 5  # 更多填充
```

### 參數調校建議

#### Whisper API 參數
- **language**: 如果知道音訊語言，指定具體語言代碼可提高準確度
- **temperature**: 通常保持 0.0，除非需要更多變化
- **min_confidence_threshold**: 根據使用場景調整，較高值提供更可靠結果但可能觸發更多回退

#### VAD 參數
- **sensitivity**: 
  - 0.3-0.5: 適合清晰、高品質音訊
  - 0.6-0.8: 適合一般品質音訊
  - 0.8-1.0: 適合低品質或有背景噪音的音訊
- **chunk_size**: 較小值提供更精確檢測但消耗更多資源
- **padding_chunks**: 根據語音特性調整，較快語速需要更多填充

### 故障排解

#### Whisper API 問題
- **API 金鑰錯誤**: 檢查 `OPENAI_API_KEY` 環境變數設定
- **配額不足**: 檢查 OpenAI 帳戶餘額和使用限制
- **網路逾時**: 增加 `timeout_seconds` 值
- **語言檢測失敗**: 嘗試指定具體的語言代碼

#### VAD 檢測問題
- **檢測不到語音**: 降低 `sensitivity` 值
- **誤檢測過多**: 提高 `sensitivity` 值
- **語音被截斷**: 增加 `padding_chunks` 值
- **處理速度慢**: 增加 `chunk_size` 值

#### 同步精度問題
- **偏移量過大**: 增加 `analysis_window_seconds`
- **檢測不穩定**: 啟用 `fallback_to_vad` 並調整 `min_confidence_threshold`
```

#### 4.2 建立快速開始指南

**檔案位置**：在 `docs/` 目錄建立 `sync-quick-start.md`

```markdown
# Sync 功能快速開始指南

本指南幫助您快速開始使用 SubX 的新同步功能。

## 最小配置

如果您只想使用基本功能，在配置檔案中加入：

```toml
[sync]
default_method = "vad"

[sync.vad]
enabled = true
```

## 使用步驟

### 1. 準備檔案
確保您有：
- 音訊檔案（支援 MP3, WAV, M4A 等格式）
- 對應的字幕檔案（SRT, VTT 等格式）

### 2. 執行同步
```bash
# 基本用法
subx sync audio.mp3 subtitle.srt

# 指定輸出檔案
subx sync audio.mp3 subtitle.srt -o synced_subtitle.srt

# 使用特定方法
subx sync audio.mp3 subtitle.srt --method vad
subx sync audio.mp3 subtitle.srt --method whisper
```

### 3. 驗證結果
檢查輸出的字幕檔案，確認時間軸是否正確對齊。

## 進階使用

### 使用 Whisper API（需要 API 金鑰）
```bash
# 設定 API 金鑰
export OPENAI_API_KEY="your-api-key-here"

# 使用 Whisper 方法
subx sync audio.mp3 subtitle.srt --method whisper
```

### 自訂分析窗口
```bash
# 使用 60 秒分析窗口（適合較難檢測的音訊）
subx sync audio.mp3 subtitle.srt --analysis-window 60
```

### 調整 VAD 敏感度
```bash
# 高敏感度（適合低音量或有背景噪音）
subx sync audio.mp3 subtitle.srt --vad-sensitivity 0.9

# 低敏感度（適合清晰音訊）
subx sync audio.mp3 subtitle.srt --vad-sensitivity 0.5
```

## 常見問題

**Q: 同步結果不準確怎麼辦？**
A: 嘗試以下方法：
1. 增加分析窗口時間 `--analysis-window 60`
2. 如果使用 VAD，調整敏感度參數
3. 如果音訊品質較好，嘗試使用 Whisper 方法

**Q: Whisper API 呼叫失敗怎麼辦？**
A: 檢查：
1. API 金鑰是否正確設定
2. 網路連線是否正常
3. OpenAI 帳戶是否有足夠配額

**Q: 處理大檔案時很慢怎麼辦？**
A: 建議：
1. 使用 VAD 方法（完全本地處理）
2. 確保音訊檔案品質適中（不需過高品質）
3. 檢查系統資源使用情況
```

#### 4.3 更新主要文檔的索引

**步驟 1**：檢查是否需要更新 `README.md`
```bash
# 檢查 README 中是否有 sync 相關說明
grep -i sync README.md
```

**步驟 2**：如果需要，在 README.md 的適當位置加入新功能說明
```markdown
## 音訊同步功能

SubX 提供智能音訊同步功能，支援兩種檢測方法：

- **OpenAI Whisper API**: 高精度雲端語音識別
- **本地 VAD**: 快速的本地語音活動檢測

```bash
# 基本同步
subx sync audio.mp3 subtitle.srt

# 使用 Whisper API（需要 API 金鑰）
export OPENAI_API_KEY="your-key"
subx sync audio.mp3 subtitle.srt --method whisper
```

詳細配置請參考 [配置指南](docs/configuration-guide.md#sync-配置)。
```

#### 4.4 驗收標準檢查清單

- [ ] `docs/configuration-guide.md` 包含完整的 sync 配置說明
- [ ] 提供了不同使用場景的配置範例
- [ ] 建立了快速開始指南 `docs/sync-quick-start.md`
- [ ] 包含常見問題的故障排解說明
- [ ] 參數調校建議清晰易懂
- [ ] 主要文檔（如 README）已更新相關連結

## 執行前準備

### 開發環境要求
- Rust 1.70+ 
- 已安裝專案相關依賴：`cargo build`
- Wiremock 已可用於模擬測試（已在 Cargo.toml 中）

### 需要手動準備的測試音訊檔案

在 `tests/data/` 目錄下需要準備以下音訊檔案：

**目錄結構：**
```
tests/data/
├── test_speech.wav              # Whisper 測試用：包含清晰語音的音訊
├── fallback_test.wav            # 回退測試用：包含語音的音訊
├── vad_test_speech.wav          # VAD 測試用：包含語音的音訊
├── vad_test_silence.wav         # VAD 測試用：純靜音音訊
└── vad_test_mixed.wav           # VAD 測試用：語音與靜音混合
```

**檔案規格要求：**
1. **格式**：WAV 格式
2. **採樣率**：16000 Hz（16 kHz）
3. **聲道**：單聲道（mono）
4. **時長**：3-5 秒即可
5. **音量**：適中，避免過於微弱或過於響亮

**具體內容要求：**

1. **test_speech.wav** 和 **fallback_test.wav**：
   - 包含清晰的語音內容（可以是任何語言）
   - 建議內容：簡單的句子，如「Hello world, this is a test」
   - 語音應該在音訊開始後 0.5-1 秒開始

2. **vad_test_speech.wav**：
   - 包含清晰的語音
   - 可以與上述檔案相同或類似

3. **vad_test_silence.wav**：
   - 純靜音或極低的背景噪音
   - 時長 3-5 秒

4. **vad_test_mixed.wav**：
   - 開始 1 秒靜音
   - 中間 2-3 秒語音
   - 結束 1 秒靜音

**建立測試檔案的建議方法：**
- 使用 Audacity 等音訊編輯軟體
- 錄製簡單的語音片段
- 確保檔案符合上述規格
- 可以使用合成語音（TTS）產生測試內容

## 評估 Whisper 客戶端重構需求

根據現有的實作檢查，需要評估是否需要重構 Whisper 客戶端以支援依賴注入：

**步驟 1**：檢查現有客戶端建構
```bash
# 檢查 WhisperApiClient 的 new 方法
grep -A 10 "pub fn new" src/services/whisper/client.rs
```

**步驟 2**：確認 base_url 參數支援
現有的 `WhisperApiClient::new` 已經接受 `base_url` 參數，這意味著我們可以直接傳入 wiremock 伺服器的 URL，無需額外重構。

**步驟 3**：驗證 wiremock 整合
確認現有的 wiremock 測試（如 `tests/whisper_mock_tests.rs`）已經正常運作：

```bash
# 執行現有的 mock 測試
cargo test whisper_mock --test whisper_mock_tests
```

**結論**：根據現有實作，Whisper 客戶端已經支援自訂 base_url，因此**不需要額外的重構**就可以使用 wiremock 進行測試。

### 除錯技巧
- 使用 `RUST_LOG=debug cargo test` 查看詳細日誌
- 測試失敗時，檢查 `target/` 目錄下的測試輸出
- 使用 `println!` 或 `dbg!` 巨集進行除錯
- 善用 IDE 的除錯功能進行逐步執行

## 不包含的工作

為保持焦點，以下工作**不**在此 backlog 中：
- 新功能開發或架構變更
- 性能優化或進階配置選項
- 詳細的 API 文檔補充
- 複雜的測試基礎設施建設
- UI/UX 改進
- 國際化或本地化工作

## 品質標準

### 程式碼品質要求
- 所有新增程式碼必須通過 `cargo clippy` 檢查
- 程式碼格式必須符合 `cargo fmt` 標準
- 新增的測試必須有清晰的文檔註釋
- 錯誤處理必須完整且有意義的錯誤訊息

### 測試品質要求
- 測試名稱必須清楚描述測試目的
- 每個測試必須獨立執行，不依賴其他測試
- 測試資料必須在測試內部產生，不依賴外部檔案
- 失敗的測試必須提供清晰的失敗原因

### 文檔品質要求
- 使用範例必須可以直接執行
- 配置範例必須經過實際測試
- 故障排解說明必須涵蓋常見情況
- 文檔語言要簡潔明確，避免技術術語過多

## 最終驗收與交付

### 完整驗收檢查清單

#### 測試相關
- [ ] 執行 `cargo test` 無任何 `#[ignore]` 相關警告
- [ ] 所有原先被忽略的測試現在能正常執行或優雅跳過
- [ ] 新增的測試覆蓋 Whisper-VAD 回退機制的主要場景
- [ ] VAD 測試完全使用合成音訊，不依賴外部檔案
- [ ] 測試執行時間合理（總測試時間 < 2 分鐘）

#### 功能相關  
- [ ] Whisper API 失敗時能正確回退到 VAD
- [ ] 低信心度時能正確回退到 VAD
- [ ] 配置 `fallback_to_vad = false` 時不會回退
- [ ] VAD 能檢測合成語音中的語音段
- [ ] VAD 不會將靜音誤判為語音

#### 文檔相關
- [ ] `docs/configuration-guide.md` 包含完整 sync 配置說明
- [ ] 提供了至少 3 個不同使用場景的配置範例
- [ ] 建立了快速開始指南 `docs/sync-quick-start.md`
- [ ] 故障排解說明涵蓋常見問題
- [ ] 所有配置範例都經過實際測試驗證

#### 程式碼品質
- [ ] `cargo clippy -- -D warnings` 通過
- [ ] `cargo fmt --check` 通過
- [ ] 所有新增程式碼有適當的註釋
- [ ] 沒有遺留的除錯程式碼或 TODO 註釋

### 交付物清單

1. **修復的測試檔案**
   - `tests/whisper_integration_tests.rs`（移除 #[ignore]）
   - `tests/vad_integration_tests.rs`（移除 #[ignore]）
   - `tests/sync_fallback_integration_tests.rs`（新建）

2. **測試支援檔案**
   - `tests/common/audio_generation.rs`（新建）
   - `tests/common/mod.rs`（新建）

3. **文檔檔案**
   - `docs/configuration-guide.md`（更新）
   - `docs/sync-quick-start.md`（新建）

4. **可能的程式碼修復**
   - 任何在測試過程中發現的 bug 修復
   - 改進的錯誤處理或日誌記錄

### 驗收測試腳本

執行以下腳本確認所有功能正常：

```bash
#!/bin/bash
# acceptance_test.sh

echo "=== Backlog #33 驗收測試 ==="

# 1. 基本編譯測試
echo "1. 檢查編譯..."
cargo build || exit 1

# 2. 格式和 linting
echo "2. 檢查程式碼格式..."
cargo fmt --check || exit 1
cargo clippy -- -D warnings || exit 1

# 3. 執行所有測試
echo "3. 執行測試套件..."
cargo test || exit 1

# 4. 檢查是否還有被忽略的測試
echo "4. 檢查忽略的測試..."
IGNORED_TESTS=$(cargo test 2>&1 | grep "test result:" | grep -o "[0-9]* ignored")
if [[ "$IGNORED_TESTS" != "0 ignored" && -n "$IGNORED_TESTS" ]]; then
    echo "警告：仍有被忽略的測試: $IGNORED_TESTS"
    cargo test -- --ignored --list
fi

# 5. 檢查文檔檔案存在
echo "5. 檢查文檔完整性..."
if [[ ! -f "docs/sync-quick-start.md" ]]; then
    echo "錯誤：缺少 docs/sync-quick-start.md"
    exit 1
fi

# 6. 驗證配置範例語法
echo "6. 驗證配置範例..."
# 這裡可以加入配置檔案語法檢查

echo "✅ 所有驗收測試通過！"
```

## 風險評估與緩解

### 識別的風險

| 風險類型 | 描述 | 影響程度 | 緩解措施 |
|---------|------|----------|----------|
| 技術風險 | 合成音訊無法完全模擬真實場景 | 中 | 提供多種合成音訊類型，涵蓋不同場景 |
| 時程風險 | 音訊處理邏輯比預期複雜 | 低 | 重點修復現有功能，避免重新實作 |
| 品質風險 | 測試環境與實際使用環境差異 | 中 | 確保測試涵蓋邊界情況和錯誤處理 |
| 依賴風險 | OpenAI API 變更影響測試 | 低 | 主要依賴回退機制，減少對外部 API 依賴 |

### 緩解策略詳細說明

**技術風險緩解**：
- 建立多種類型的合成音訊（不同音量、頻率、有/無背景噪音）
- 保留一些基本的真實音訊測試作為參考（但不依賴於 CI）
- 確保 VAD 參數可以透過配置調整

**品質風險緩解**：
- 測試不僅驗證正常情況，也要測試邊界條件
- 加入錯誤處理和異常情況的測試
- 確保日誌記錄足夠詳細，便於問題診斷

## 後續改進建議

完成此 backlog 後，建議考慮以下優化（但不在當前範圍內）：

1. **性能測試**：加入大檔案處理的性能測試
2. **更多音訊格式支援**：測試不同音訊格式的相容性
3. **進階 VAD 參數**：研究更精細的 VAD 調校選項
4. **使用者介面改進**：提供更友善的錯誤訊息和進度顯示
5. **批次處理**：支援多個檔案的批次同步功能

## 預估工時分布

| 任務 | 預估時間 | 主要工作內容 |
|------|----------|-------------|
| 任務 1：修復整合測試 | 4-5 小時 | 分析失敗原因、實作條件測試、建立合成音訊 |
| 任務 2：驗證回退機制 | 2-3 小時 | 建立回退測試、驗證邏輯、測試日誌記錄 |
| 任務 3：建立 VAD 測試 | 2-3 小時 | 實作音訊產生工具、建立各種測試場景 |
| 任務 4：補充使用文檔 | 2-3 小時 | 撰寫配置指南、快速開始指南、故障排解 |
| 整合測試和除錯 | 1-2 小時 | 執行完整測試套件、修復發現的問題 |
| **總計** | **11-16 小時** | 包含測試、除錯和文檔撰寫的完整工作 |

---

**負責人**: 待指派  
**建立日期**: 2025-06-14  
**更新日期**: 2025-06-14  
**狀態**: 待開始  
**優先級**: 高  
**相關**: Backlog #32, 程式碼審查報告 #138

**標籤**: `testing`, `documentation`, `sync`, `validation`, `integration`
