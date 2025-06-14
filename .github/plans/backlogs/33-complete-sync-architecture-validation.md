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

**步驟 1**：檢查測試結構
```bash
# 查看測試檔案內容
cat tests/whisper_integration_tests.rs
```

**步驟 2**：實作條件式測試執行
在 `tests/whisper_integration_tests.rs` 中加入以下模式：

```rust
use std::env;

fn skip_if_no_api_key() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("OPENAI_API_KEY").is_err() {
        println!("Skipping test: OPENAI_API_KEY not set");
        return Err("API key not available".into());
    }
    Ok(())
}

#[tokio::test]
async fn test_whisper_sync_detection() -> Result<(), Box<dyn std::error::Error>> {
    // 檢查 API 金鑰，如果沒有則跳過測試
    skip_if_no_api_key()?;
    
    // 建立測試配置
    let config = WhisperConfig {
        enabled: true,
        model: "whisper-1".to_string(),
        language: "auto".to_string(),
        temperature: 0.0,
        timeout_seconds: 30,
        fallback_to_vad: true,
        min_confidence_threshold: 0.7,
    };
    
    // 建立合成音訊測試資料或使用測試檔案
    let test_audio_path = create_test_audio_file()?;
    
    // 執行實際測試
    let result = detect_sync_with_whisper(&config, &test_audio_path).await?;
    
    // 驗證結果
    assert!(result.is_some());
    assert!(result.unwrap().confidence > 0.5);
    
    Ok(())
}
```

**步驟 3**：建立測試音訊檔案產生函數
```rust
fn create_test_audio_file() -> Result<PathBuf, Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::PathBuf;
    
    // 建立臨時測試目錄
    let test_dir = PathBuf::from("target/test_audio");
    fs::create_dir_all(&test_dir)?;
    
    // 這裡可以：
    // 1. 複製現有的測試音訊檔案
    // 2. 產生簡單的合成音訊
    // 3. 使用預先準備的測試資源
    
    let test_file = test_dir.join("test.wav");
    
    // 簡單的方式：檢查是否有測試音訊檔案
    if !test_file.exists() {
        // 創建一個簡單的靜音 WAV 檔案作為測試
        create_silent_wav_file(&test_file)?;
    }
    
    Ok(test_file)
}
```

#### 1.3 修復 VAD 整合測試

**檔案位置**：`tests/vad_integration_tests.rs`

**步驟 1**：移除外部檔案依賴
將原本依賴外部音訊檔案的測試改為使用合成音訊：

```rust
#[tokio::test]
async fn test_vad_detection_with_synthetic_audio() -> Result<(), Box<dyn std::error::Error>> {
    // 建立 VAD 配置
    let config = VadConfig {
        enabled: true,
        sensitivity: 0.75,
        chunk_size: 512,
        sample_rate: 16000,
        padding_chunks: 3,
    };
    
    // 產生合成音訊資料
    let synthetic_audio = generate_synthetic_speech_audio()?;
    
    // 執行 VAD 檢測
    let detector = VadDetector::new(&config)?;
    let detection_result = detector.detect_voice_activity(&synthetic_audio)?;
    
    // 驗證結果
    assert!(!detection_result.voice_segments.is_empty());
    assert!(detection_result.voice_segments[0].start_time >= 0.0);
    
    Ok(())
}

fn generate_synthetic_speech_audio() -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // 產生包含語音特徵的合成音訊
    let sample_rate = 16000;
    let duration_seconds = 2.0;
    let total_samples = (sample_rate as f32 * duration_seconds) as usize;
    
    let mut audio_data = Vec::with_capacity(total_samples);
    
    // 產生具有語音特徵的音訊波形
    for i in 0..total_samples {
        let t = i as f32 / sample_rate as f32;
        // 模擬語音的復合波形
        let speech_like = 0.3 * (2.0 * std::f32::consts::PI * 200.0 * t).sin()
                        + 0.2 * (2.0 * std::f32::consts::PI * 400.0 * t).sin()
                        + 0.1 * (2.0 * std::f32::consts::PI * 800.0 * t).sin();
        audio_data.push(speech_like);
    }
    
    Ok(audio_data)
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
**目標**：確認 Whisper API 失敗時能正確回退到 VAD

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

#### 2.2 建立回退機制測試

**檔案位置**：建立新檔案 `tests/sync_fallback_integration_tests.rs`

**步驟 1**：建立測試檔案結構
```rust
use subx::config::{SyncConfig, WhisperConfig, VadConfig};
use subx::core::sync::SyncEngine;
use std::path::PathBuf;

#[tokio::test]
async fn test_whisper_api_failure_fallback() -> Result<(), Box<dyn std::error::Error>> {
    // 建立配置，啟用回退機制
    let config = create_fallback_test_config();
    
    // 建立故意會失敗的 Whisper 配置（錯誤的 API 金鑰）
    let mut whisper_config = config.whisper.clone();
    whisper_config.api_key = Some("invalid_key".to_string());
    
    let test_config = SyncConfig {
        whisper: whisper_config,
        ..config
    };
    
    // 準備測試音訊
    let test_audio = create_test_audio_for_fallback()?;
    
    // 執行同步檢測
    let engine = SyncEngine::new();
    let result = engine.detect_sync_point(&test_audio, &test_config).await;
    
    // 驗證：即使 Whisper 失敗，仍應通過 VAD 得到結果
    assert!(result.is_ok());
    let sync_result = result.unwrap();
    assert!(sync_result.method_used == "vad"); // 確認使用了 VAD
    assert!(sync_result.offset_seconds.is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_whisper_low_confidence_fallback() -> Result<(), Box<dyn std::error::Error>> {
    // 測試低信心度場景的回退
    let mut config = create_fallback_test_config();
    config.whisper.min_confidence_threshold = 0.9; // 設定很高的閾值
    
    // 建立會產生低信心度結果的測試音訊
    let low_quality_audio = create_low_quality_test_audio()?;
    
    let engine = SyncEngine::new();
    let result = engine.detect_sync_point(&low_quality_audio, &config).await;
    
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

fn create_fallback_test_config() -> SyncConfig {
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
            api_key: None, // 將由測試設定
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
**預估工時**：2-3 小時  
**目標**：提供不依賴外部檔案的 VAD 功能驗證

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

#### 3.2 建立合成音訊工具函數

**檔案位置**：建立 `tests/common/audio_generation.rs`

**步驟 1**：建立 common 模組
```bash
mkdir -p tests/common
```

**步驟 2**：實作音訊產生工具
```rust
// tests/common/audio_generation.rs
use std::f32::consts::PI;

pub struct AudioGenerator {
    sample_rate: usize,
}

impl AudioGenerator {
    pub fn new(sample_rate: usize) -> Self {
        Self { sample_rate }
    }
    
    /// 產生靜音音訊
    pub fn generate_silence(&self, duration_seconds: f32) -> Vec<f32> {
        let total_samples = (self.sample_rate as f32 * duration_seconds) as usize;
        vec![0.0; total_samples]
    }
    
    /// 產生具有語音特徵的音訊
    pub fn generate_speech_like_audio(&self, duration_seconds: f32) -> Vec<f32> {
        let total_samples = (self.sample_rate as f32 * duration_seconds) as usize;
        let mut audio_data = Vec::with_capacity(total_samples);
        
        for i in 0..total_samples {
            let t = i as f32 / self.sample_rate as f32;
            
            // 模擬語音的基頻和共振峰
            let fundamental = 0.3 * (2.0 * PI * 150.0 * t).sin(); // 基頻 150Hz
            let formant1 = 0.2 * (2.0 * PI * 800.0 * t).sin();    // 第一共振峰
            let formant2 = 0.1 * (2.0 * PI * 1200.0 * t).sin();   // 第二共振峰
            
            // 加入振幅調制模擬語音包絡
            let envelope = (2.0 * PI * 5.0 * t).sin().abs(); // 5Hz 的包絡變化
            
            let sample = (fundamental + formant1 + formant2) * envelope;
            audio_data.push(sample);
        }
        
        audio_data
    }
    
    /// 產生帶有語音段的測試音訊（靜音 + 語音 + 靜音）
    pub fn generate_speech_with_silence(&self, 
        silence_before: f32, 
        speech_duration: f32, 
        silence_after: f32
    ) -> Vec<f32> {
        let mut audio = Vec::new();
        
        // 前段靜音
        audio.extend(self.generate_silence(silence_before));
        
        // 語音段
        audio.extend(self.generate_speech_like_audio(speech_duration));
        
        // 後段靜音
        audio.extend(self.generate_silence(silence_after));
        
        audio
    }
    
    /// 產生白噪音
    pub fn generate_white_noise(&self, duration_seconds: f32, amplitude: f32) -> Vec<f32> {
        use rand::Rng;
        let total_samples = (self.sample_rate as f32 * duration_seconds) as usize;
        let mut rng = rand::thread_rng();
        
        (0..total_samples)
            .map(|_| amplitude * (rng.gen::<f32>() * 2.0 - 1.0))
            .collect()
    }
}

// tests/common/mod.rs
pub mod audio_generation;
```

#### 3.3 實作 VAD 基本功能測試

**檔案位置**：更新 `tests/vad_integration_tests.rs`

**步驟 1**：移除 `#[ignore]` 並實作新測試
```rust
// tests/vad_integration_tests.rs
mod common;

use common::audio_generation::AudioGenerator;
use subx::config::VadConfig;
use subx::services::vad::VadDetector;

#[tokio::test]
async fn test_vad_detects_speech_in_synthetic_audio() -> Result<(), Box<dyn std::error::Error>> {
    // 建立標準配置
    let config = VadConfig {
        enabled: true,
        sensitivity: 0.75,
        chunk_size: 512,
        sample_rate: 16000,
        padding_chunks: 3,
    };
    
    // 產生測試音訊：1秒靜音 + 2秒語音 + 1秒靜音
    let generator = AudioGenerator::new(config.sample_rate);
    let test_audio = generator.generate_speech_with_silence(1.0, 2.0, 1.0);
    
    // 執行 VAD 檢測
    let detector = VadDetector::new(&config)?;
    let result = detector.detect_voice_activity(&test_audio)?;
    
    // 驗證結果
    assert!(!result.voice_segments.is_empty(), "Should detect at least one voice segment");
    
    let first_segment = &result.voice_segments[0];
    assert!(first_segment.start_time >= 0.8, "Speech should start around 1 second");
    assert!(first_segment.start_time <= 1.2, "Speech start should be detected accurately");
    assert!(first_segment.end_time >= 2.8, "Speech should end around 3 seconds");
    
    Ok(())
}

#[tokio::test]
async fn test_vad_ignores_silence() -> Result<(), Box<dyn std::error::Error>> {
    let config = VadConfig {
        enabled: true,
        sensitivity: 0.75,
        chunk_size: 512,
        sample_rate: 16000,
        padding_chunks: 3,
    };
    
    // 產生純靜音音訊
    let generator = AudioGenerator::new(config.sample_rate);
    let silence_audio = generator.generate_silence(3.0);
    
    let detector = VadDetector::new(&config)?;
    let result = detector.detect_voice_activity(&silence_audio)?;
    
    // 驗證：靜音應該不被檢測為語音
    assert!(result.voice_segments.is_empty(), "Silence should not be detected as speech");
    
    Ok(())
}

#[tokio::test]
async fn test_vad_sensitivity_adjustment() -> Result<(), Box<dyn std::error::Error>> {
    // 產生低音量的語音音訊
    let generator = AudioGenerator::new(16000);
    let mut low_volume_speech = generator.generate_speech_like_audio(2.0);
    
    // 降低音量到 10%
    for sample in &mut low_volume_speech {
        *sample *= 0.1;
    }
    
    // 測試高敏感度配置
    let high_sensitivity_config = VadConfig {
        enabled: true,
        sensitivity: 0.9, // 高敏感度
        chunk_size: 512,
        sample_rate: 16000,
        padding_chunks: 3,
    };
    
    // 測試低敏感度配置
    let low_sensitivity_config = VadConfig {
        sensitivity: 0.3, // 低敏感度
        ..high_sensitivity_config
    };
    
    let detector_high = VadDetector::new(&high_sensitivity_config)?;
    let detector_low = VadDetector::new(&low_sensitivity_config)?;
    
    let result_high = detector_high.detect_voice_activity(&low_volume_speech)?;
    let result_low = detector_low.detect_voice_activity(&low_volume_speech)?;
    
    // 驗證：高敏感度應該檢測到更多語音段
    assert!(result_high.voice_segments.len() >= result_low.voice_segments.len(),
            "High sensitivity should detect more or equal voice segments");
    
    Ok(())
}

#[tokio::test]
async fn test_vad_noise_robustness() -> Result<(), Box<dyn std::error::Error>> {
    let config = VadConfig {
        enabled: true,
        sensitivity: 0.75,
        chunk_size: 512,
        sample_rate: 16000,
        padding_chunks: 3,
    };
    
    let generator = AudioGenerator::new(config.sample_rate);
    
    // 產生語音 + 噪音的混合音訊
    let speech = generator.generate_speech_like_audio(2.0);
    let noise = generator.generate_white_noise(2.0, 0.1); // 低強度白噪音
    
    let mut mixed_audio: Vec<f32> = speech.iter()
        .zip(noise.iter())
        .map(|(s, n)| s + n)
        .collect();
    
    let detector = VadDetector::new(&config)?;
    let result = detector.detect_voice_activity(&mixed_audio)?;
    
    // 驗證：即使有噪音，仍應能檢測到語音
    assert!(!result.voice_segments.is_empty(), "Should detect speech even with background noise");
    
    Ok(())
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
- 可選：OpenAI API 金鑰（用於 Whisper 測試）

### 工具和指令參考
```bash
# 執行特定測試
cargo test test_name

# 執行被忽略的測試
cargo test -- --ignored

# 執行整合測試
cargo test --test integration_test_name

# 產生測試覆蓋率報告
cargo llvm-cov --html

# 檢查程式碼品質
cargo clippy -- -D warnings
cargo fmt --check
```

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
