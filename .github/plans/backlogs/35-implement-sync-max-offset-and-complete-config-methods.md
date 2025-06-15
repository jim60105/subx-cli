# Backlog 35: 實作 sync.max_offset_seconds 配置使用與完善 get_config_value/set_config_value 邏輯

## 概覽

根據配置使用分析報告，目前存在兩個關鍵問題需要解決：

1. **`sync.max_offset_seconds` 配置已定義但未實際使用** - 雖然配置可以設定和驗證，但在同步引擎的業務邏輯中並未實際使用此限制
2. **`get_config_value` 和 `set_config_value` 方法支援不一致** - get 方法僅支援 15 項配置，而 set 方法支援 31 項，導致配置管理功能不完整

## 背景

### 當前問題分析

#### 問題 1: sync.max_offset_seconds 未實際使用

**配置現狀：**
```toml
[sync]
max_offset_seconds = 60.0  # 預設值：60 秒
```

**當前實作狀況：**
- ✅ 配置結構中已定義 (`src/config/mod.rs:199`)
- ✅ 配置驗證中已包含 (`src/config/validator.rs:98`)
- ✅ `get_config_value` 和 `set_config_value` 均支援
- ❌ **業務邏輯中未使用** - 同步引擎不會根據此配置限制偏移量

**影響範圍：**
- 使用者設定 `max_offset_seconds` 期望限制偏移量，但實際上無效
- 可能導致意外的大偏移量結果，不符合使用者預期
- 配置文檔承諾的功能未實際提供

#### 問題 2: get_config_value 支援不完整

**配置支援差異：**

| 配置類別 | set_config_value 支援 | get_config_value 支援 | 缺失項目 |
|---------|---------------------|---------------------|---------|
| AI 配置 | 8 項 | 5 項 | max_sample_length, retry_attempts, retry_delay_ms |
| 格式配置 | 4 項 | 3 項 | encoding_detection_confidence |
| 同步配置 | 8 項 + VAD (7 項) | 3 項 | default_method, 所有 VAD 配置 |
| 一般配置 | 5 項 | 2 項 | task_timeout_seconds, enable_progress_bar, worker_idle_timeout_seconds |
| 並行配置 | 5 項 | 1 項 | task_queue_size, enable_task_priorities, auto_balance_workers, overflow_strategy |

**嚴重缺失：**
1. **所有 VAD 相關配置** (7 項)：enabled, sensitivity, chunk_size, sample_rate, padding_chunks, min_speech_duration_ms, speech_merge_gap_ms
2. **sync.default_method** 配置
3. **一般配置中的 3 項重要設定**
4. **並行配置中的 4 項進階設定**

## 目標

### 主要目標

1. **實作 sync.max_offset_seconds 使用邏輯**
   - 在同步引擎中實際使用此配置
   - 限制檢測到的偏移量不超過設定值
   - 提供適當的警告和錯誤處理

2. **完善 get_config_value 方法支援**
   - 擴展 `ProductionConfigService::get_config_value()` 方法
   - 添加所有缺失的配置鍵支援
   - 確保與 `set_config_value()` 方法的一致性

3. **完善 TestConfigService 支援**
   - 同步更新測試配置服務
   - 確保測試環境與生產環境的一致性

### 技術目標

1. **配置一致性** - 確保 get 和 set 方法支援相同的配置項目
2. **功能完整性** - 所有已定義的配置都應該在業務邏輯中發揮作用
3. **使用者體驗** - 提供清晰的配置行為和適當的錯誤信息
4. **測試覆蓋** - 確保新功能有完整的測試覆蓋

## 技術規格

### 階段 1: 實作 sync.max_offset_seconds 使用邏輯

#### 1.1 更新同步引擎核心邏輯

**檔案：** `src/core/sync/engine.rs`

需要在以下方法中添加偏移量限制邏輯：

```rust
impl SyncEngine {
    /// Apply manual offset with max_offset_seconds validation
    pub fn apply_manual_offset(
        &self,
        subtitle: &mut Subtitle,
        offset_seconds: f32,
    ) -> Result<SyncResult> {
        // 添加最大偏移量驗證
        if offset_seconds.abs() > self.config.max_offset_seconds {
            return Err(SubXError::sync(format!(
                "偏移量 {:.2}s 超過最大允許值 {:.2}s。請檢查配置 sync.max_offset_seconds 或使用更小的偏移量。",
                offset_seconds, self.config.max_offset_seconds
            )));
        }

        // 現有的偏移應用邏輯
        // ...
    }

    /// VAD-based sync detection with offset limit validation
    async fn vad_detect_sync_offset(
        &self,
        audio_path: &Path,
        subtitle: &Subtitle,
    ) -> Result<SyncResult> {
        // 現有的 VAD 檢測邏輯
        let mut result = self.perform_vad_detection(audio_path, subtitle).await?;
        
        // 添加偏移量限制檢查
        if result.offset_seconds.abs() > self.config.max_offset_seconds {
            // 提供警告但不完全失敗，允許使用者決定
            result.warnings.push(format!(
                "檢測到的偏移量 {:.2}s 超過配置的最大值 {:.2}s。建議檢查音訊品質或調整 sync.max_offset_seconds 配置。",
                result.offset_seconds, self.config.max_offset_seconds
            ));
            
            // 可選：截斷到最大值（保留符號）
            let sign = if result.offset_seconds >= 0.0 { 1.0 } else { -1.0 };
            result.offset_seconds = sign * self.config.max_offset_seconds;
            
            result.additional_info = Some(json!({
                "original_offset": result.offset_seconds,
                "capped_at_max": self.config.max_offset_seconds,
                "reason": "Exceeded max_offset_seconds configuration"
            }));
        }

        Ok(result)
    }
}
```

#### 1.2 更新 CLI 參數處理

**檔案：** `src/commands/sync_command.rs`

在手動偏移模式下添加配置檢查：

```rust
pub async fn execute_with_config(
    args: SyncArgs,
    config_service: std::sync::Arc<dyn ConfigService>,
) -> Result<()> {
    let config = config_service.get_config()?;
    let sync_config = &config.sync;

    // 驗證手動偏移量
    if let Some(manual_offset) = args.offset {
        if manual_offset.abs() > sync_config.max_offset_seconds {
            return Err(SubXError::config(format!(
                "指定的偏移量 {:.2}s 超過配置的最大允許值 {:.2}s。\n\n\
                解決方案：\n\
                • 使用較小的偏移量\n\
                • 修改配置：subx-cli config set sync.max_offset_seconds {:.1}\n\
                • 檢查偏移量是否正確",
                manual_offset, 
                sync_config.max_offset_seconds,
                manual_offset.abs().max(sync_config.max_offset_seconds * 1.5)
            )));
        }
    }
    
    // 現有邏輯繼續
    // ...
}
```

#### 1.3 添加配置驗證增強

**檔案：** `src/config/validator.rs`

確保 max_offset_seconds 的驗證邏輯更加完善：

```rust
impl SyncConfig {
    pub fn validate(&self) -> Result<()> {
        // 增強 max_offset_seconds 驗證
        if self.max_offset_seconds <= 0.0 {
            return Err(SubXError::config_validation(
                "sync.max_offset_seconds 必須大於 0。建議值：30.0 到 300.0 秒之間。"
            ));
        }
        
        if self.max_offset_seconds > 3600.0 {
            return Err(SubXError::config_validation(
                "sync.max_offset_seconds 不應超過 3600 秒（1 小時）。如需更大值，請檢查同步需求是否合理。"
            ));
        }

        // 提供建議範圍的警告
        if self.max_offset_seconds < 5.0 || self.max_offset_seconds > 600.0 {
            // 可考慮添加 warning 系統，暫時通過驗證但記錄建議
        }

        // 現有的其他驗證邏輯
        // ...
    }
}
```

### 階段 2: 完善 get_config_value 方法支援

#### 2.1 擴展 ProductionConfigService::get_config_value()

**檔案：** `src/config/service.rs`

完整支援所有配置項目：

```rust
impl ConfigService for ProductionConfigService {
    fn get_config_value(&self, key: &str) -> Result<String> {
        let config = self.get_config()?;
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            // === AI 配置 ===
            ["ai", "provider"] => Ok(config.ai.provider.clone()),
            ["ai", "model"] => Ok(config.ai.model.clone()),
            ["ai", "api_key"] => Ok(config.ai.api_key.clone().unwrap_or_default()),
            ["ai", "base_url"] => Ok(config.ai.base_url.clone()),
            ["ai", "temperature"] => Ok(config.ai.temperature.to_string()),
            // 新增：缺失的 AI 配置
            ["ai", "max_sample_length"] => Ok(config.ai.max_sample_length.to_string()),
            ["ai", "retry_attempts"] => Ok(config.ai.retry_attempts.to_string()),
            ["ai", "retry_delay_ms"] => Ok(config.ai.retry_delay_ms.to_string()),

            // === 格式配置 ===
            ["formats", "default_output"] => Ok(config.formats.default_output.clone()),
            ["formats", "default_encoding"] => Ok(config.formats.default_encoding.clone()),
            ["formats", "preserve_styling"] => Ok(config.formats.preserve_styling.to_string()),
            // 新增：缺失的格式配置
            ["formats", "encoding_detection_confidence"] => {
                Ok(config.formats.encoding_detection_confidence.to_string())
            }

            // === 同步配置 ===
            ["sync", "max_offset_seconds"] => Ok(config.sync.max_offset_seconds.to_string()),
            ["sync", "correlation_threshold"] => Ok(config.sync.correlation_threshold.to_string()),
            ["sync", "audio_sample_rate"] => Ok(config.sync.audio_sample_rate.to_string()),
            // 新增：缺失的同步配置
            ["sync", "default_method"] => Ok(config.sync.default_method.clone()),
            // 新增：VAD 配置支援
            ["sync", "vad", "enabled"] => Ok(config.sync.vad.enabled.to_string()),
            ["sync", "vad", "sensitivity"] => Ok(config.sync.vad.sensitivity.to_string()),
            ["sync", "vad", "chunk_size"] => Ok(config.sync.vad.chunk_size.to_string()),
            ["sync", "vad", "sample_rate"] => Ok(config.sync.vad.sample_rate.to_string()),
            ["sync", "vad", "padding_chunks"] => Ok(config.sync.vad.padding_chunks.to_string()),
            ["sync", "vad", "min_speech_duration_ms"] => {
                Ok(config.sync.vad.min_speech_duration_ms.to_string())
            }
            ["sync", "vad", "speech_merge_gap_ms"] => {
                Ok(config.sync.vad.speech_merge_gap_ms.to_string())
            }

            // === 一般配置 ===
            ["general", "backup_enabled"] => Ok(config.general.backup_enabled.to_string()),
            ["general", "max_concurrent_jobs"] => {
                Ok(config.general.max_concurrent_jobs.to_string())
            }
            // 新增：缺失的一般配置
            ["general", "task_timeout_seconds"] => {
                Ok(config.general.task_timeout_seconds.to_string())
            }
            ["general", "enable_progress_bar"] => {
                Ok(config.general.enable_progress_bar.to_string())
            }
            ["general", "worker_idle_timeout_seconds"] => {
                Ok(config.general.worker_idle_timeout_seconds.to_string())
            }

            // === 並行配置 ===
            ["parallel", "max_workers"] => Ok(config.parallel.max_workers.to_string()),
            // 新增：缺失的並行配置
            ["parallel", "task_queue_size"] => Ok(config.parallel.task_queue_size.to_string()),
            ["parallel", "enable_task_priorities"] => {
                Ok(config.parallel.enable_task_priorities.to_string())
            }
            ["parallel", "auto_balance_workers"] => {
                Ok(config.parallel.auto_balance_workers.to_string())
            }
            ["parallel", "overflow_strategy"] => {
                Ok(match config.parallel.overflow_strategy {
                    OverflowStrategy::Block => "Block",
                    OverflowStrategy::Drop => "Drop", 
                    OverflowStrategy::Expand => "Expand",
                }.to_string())
            }

            _ => Err(SubXError::config(format!(
                "Unknown configuration key: {}. Use 'subx-cli config list' to see all available keys.",
                key
            ))),
        }
    }
}
```

#### 2.2 更新 TestConfigService::get_config_value()

**檔案：** `src/config/test_service.rs`

確保測試配置服務與生產服務保持一致：

```rust
impl ConfigService for TestConfigService {
    fn get_config_value(&self, key: &str) -> Result<String> {
        let config = self.config.lock().unwrap();
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            // 完全複製 ProductionConfigService 的邏輯
            // === AI 配置 === (完整 8 項)
            ["ai", "provider"] => Ok(config.ai.provider.clone()),
            ["ai", "model"] => Ok(config.ai.model.clone()),
            ["ai", "api_key"] => Ok(config.ai.api_key.clone().unwrap_or_default()),
            ["ai", "base_url"] => Ok(config.ai.base_url.clone()),
            ["ai", "temperature"] => Ok(config.ai.temperature.to_string()),
            ["ai", "max_sample_length"] => Ok(config.ai.max_sample_length.to_string()),
            ["ai", "retry_attempts"] => Ok(config.ai.retry_attempts.to_string()),
            ["ai", "retry_delay_ms"] => Ok(config.ai.retry_delay_ms.to_string()),

            // === 格式配置 === (完整 4 項)
            ["formats", "default_output"] => Ok(config.formats.default_output.clone()),
            ["formats", "default_encoding"] => Ok(config.formats.default_encoding.clone()),
            ["formats", "preserve_styling"] => Ok(config.formats.preserve_styling.to_string()),
            ["formats", "encoding_detection_confidence"] => {
                Ok(config.formats.encoding_detection_confidence.to_string())
            }

            // === 同步配置 === (完整 9 項 + VAD 7 項)
            ["sync", "max_offset_seconds"] => Ok(config.sync.max_offset_seconds.to_string()),
            ["sync", "default_method"] => Ok(config.sync.default_method.clone()),
            ["sync", "correlation_threshold"] => Ok(config.sync.correlation_threshold.to_string()),
            ["sync", "audio_sample_rate"] => Ok(config.sync.audio_sample_rate.to_string()),
            // VAD 配置
            ["sync", "vad", "enabled"] => Ok(config.sync.vad.enabled.to_string()),
            ["sync", "vad", "sensitivity"] => Ok(config.sync.vad.sensitivity.to_string()),
            ["sync", "vad", "chunk_size"] => Ok(config.sync.vad.chunk_size.to_string()),
            ["sync", "vad", "sample_rate"] => Ok(config.sync.vad.sample_rate.to_string()),
            ["sync", "vad", "padding_chunks"] => Ok(config.sync.vad.padding_chunks.to_string()),
            ["sync", "vad", "min_speech_duration_ms"] => {
                Ok(config.sync.vad.min_speech_duration_ms.to_string())
            }
            ["sync", "vad", "speech_merge_gap_ms"] => {
                Ok(config.sync.vad.speech_merge_gap_ms.to_string())
            }

            // === 一般配置 === (完整 5 項)
            ["general", "backup_enabled"] => Ok(config.general.backup_enabled.to_string()),
            ["general", "max_concurrent_jobs"] => {
                Ok(config.general.max_concurrent_jobs.to_string())
            }
            ["general", "task_timeout_seconds"] => {
                Ok(config.general.task_timeout_seconds.to_string())
            }
            ["general", "enable_progress_bar"] => {
                Ok(config.general.enable_progress_bar.to_string())
            }
            ["general", "worker_idle_timeout_seconds"] => {
                Ok(config.general.worker_idle_timeout_seconds.to_string())
            }

            // === 並行配置 === (完整 5 項)
            ["parallel", "max_workers"] => Ok(config.parallel.max_workers.to_string()),
            ["parallel", "task_queue_size"] => Ok(config.parallel.task_queue_size.to_string()),
            ["parallel", "enable_task_priorities"] => {
                Ok(config.parallel.enable_task_priorities.to_string())
            }
            ["parallel", "auto_balance_workers"] => {
                Ok(config.parallel.auto_balance_workers.to_string())
            }
            ["parallel", "overflow_strategy"] => {
                Ok(match config.parallel.overflow_strategy {
                    OverflowStrategy::Block => "Block",
                    OverflowStrategy::Drop => "Drop",
                    OverflowStrategy::Expand => "Expand",
                }.to_string())
            }

            _ => Err(SubXError::config(format!(
                "Unknown configuration key: {}",
                key
            ))),
        }
    }
}
```

### 階段 3: 完善 set_config_value 方法（補充缺失項目）

#### 3.1 添加對 VAD 配置的 set 支援

**檔案：** `src/config/service.rs` 和 `src/config/test_service.rs`

在 `validate_and_set_value` 方法中添加 VAD 配置支援：

```rust
fn validate_and_set_value(&self, config: &mut Config, key: &str, value: &str) -> Result<()> {
    use crate::config::validation::*;
    
    let parts: Vec<&str> = key.split('.').collect();
    match parts.as_slice() {
        // 現有配置項目...
        
        // === 新增：sync.default_method 支援 ===
        ["sync", "default_method"] => {
            validate_enum(value, &["auto", "vad", "local_vad"])?;
            config.sync.default_method = value.to_string();
        }
        
        // === 新增：VAD 配置支援 ===
        ["sync", "vad", "enabled"] => {
            let v = parse_bool(value)?;
            config.sync.vad.enabled = v;
        }
        ["sync", "vad", "sensitivity"] => {
            let v = validate_float_range(value, 0.0, 1.0)?;
            config.sync.vad.sensitivity = v;
        }
        ["sync", "vad", "chunk_size"] => {
            let v = validate_usize_range(value, 128, 4096)?;
            // 確保是 2 的幂
            if !v.is_power_of_two() {
                return Err(SubXError::config_validation(
                    "sync.vad.chunk_size 必須是 2 的幂 (128, 256, 512, 1024, 2048, 4096)"
                ));
            }
            config.sync.vad.chunk_size = v;
        }
        ["sync", "vad", "sample_rate"] => {
            let v = validate_uint_range(value, 8000, 48000)?;
            // 驗證常見採樣率
            match v {
                8000 | 16000 | 22050 | 32000 | 44100 | 48000 => {
                    config.sync.vad.sample_rate = v;
                }
                _ => return Err(SubXError::config_validation(
                    "sync.vad.sample_rate 建議使用標準採樣率: 8000, 16000, 22050, 32000, 44100, 48000"
                )),
            }
        }
        ["sync", "vad", "padding_chunks"] => {
            let v = validate_uint_range(value, 0, 10)?;
            config.sync.vad.padding_chunks = v;
        }
        ["sync", "vad", "min_speech_duration_ms"] => {
            let v = validate_uint_range(value, 50, 5000)?;
            config.sync.vad.min_speech_duration_ms = v;
        }
        ["sync", "vad", "speech_merge_gap_ms"] => {
            let v = validate_uint_range(value, 50, 2000)?;
            config.sync.vad.speech_merge_gap_ms = v;
        }

        // 現有的其他配置...
        _ => {
            return Err(SubXError::config(format!(
                "Unknown configuration key: {}",
                key
            )));
        }
    }
    Ok(())
}
```

### 階段 4: 測試實作

#### 4.1 max_offset_seconds 功能測試

**檔案：** `tests/sync_max_offset_integration_tests.rs`

```rust
use tempfile::TempDir;
use std::fs;
use std::sync::Arc;
use subx_cli::config::{TestConfigService, TestConfigBuilder};
use subx_cli::core::sync::SyncEngine;
use subx_cli::core::subtitle::Subtitle;
use subx_cli::cli::SyncArgs;
use subx_cli::commands::sync_command;

#[tokio::test]
async fn test_manual_offset_exceeds_max_limit() {
    // 建立配置：max_offset_seconds = 30.0
    let config = TestConfigBuilder::new()
        .with_sync_max_offset_seconds(30.0)
        .build_config();
    let config_service = Arc::new(TestConfigService::new(config));
    
    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest").unwrap();

    // 嘗試使用超過限制的偏移量 (45.0 > 30.0)
    let args = SyncArgs {
        video: None,
        subtitle: subtitle_path,
        offset: Some(45.0),
        // ... 其他預設值
    };

    let result = sync_command::execute_with_config(args, config_service).await;
    
    // 應該返回錯誤
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("超過配置的最大允許值"));
    assert!(error_msg.contains("30.0"));
    assert!(error_msg.contains("45.0"));
}

#[tokio::test]
async fn test_manual_offset_within_limit() {
    // 建立配置：max_offset_seconds = 60.0
    let config = TestConfigBuilder::new()
        .with_sync_max_offset_seconds(60.0)
        .build_config();
    let config_service = Arc::new(TestConfigService::new(config));
    
    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest").unwrap();

    // 使用在限制內的偏移量 (25.0 < 60.0)
    let args = SyncArgs {
        video: None,
        subtitle: subtitle_path.clone(),
        offset: Some(25.0),
        // ... 其他預設值
    };

    let result = sync_command::execute_with_config(args, config_service).await;
    
    // 應該成功執行
    assert!(result.is_ok());
    
    // 驗證字幕時間已調整
    let content = fs::read_to_string(&subtitle_path).unwrap();
    assert!(content.contains("00:00:26,000")); // 1s + 25s
}

#[tokio::test] 
async fn test_vad_detection_offset_capping() {
    // 建立配置：max_offset_seconds = 15.0 (較小值以便測試)
    let config = TestConfigBuilder::new()
        .with_sync_max_offset_seconds(15.0)
        .with_vad_enabled(true)
        .build_config();
    let config_service = Arc::new(TestConfigService::new(config.clone()));
    
    let engine = SyncEngine::new(config.sync).unwrap();
    
    // 模擬檢測到大偏移量的情況
    // 注意：這需要創建實際的音訊和字幕檔案進行測試
    // 或者創建可控的測試案例
    
    // 這裡可以測試當 VAD 檢測結果超過 max_offset_seconds 時
    // 是否正確截斷並添加警告
}
```

#### 4.2 get_config_value 完整性測試

**檔案：** `tests/config_get_value_completeness_tests.rs`

```rust
use subx_cli::config::{TestConfigService, TestConfigBuilder, ConfigService};

#[test]
fn test_get_config_value_ai_configurations() {
    let service = TestConfigBuilder::new()
        .with_ai_provider("test_provider")
        .with_ai_model("test_model")
        .with_ai_api_key("test_key")
        .with_ai_base_url("https://test.api")
        .with_ai_temperature(0.8)
        .with_ai_max_sample_length(5000)
        .with_ai_retry_attempts(5)
        .with_ai_retry_delay_ms(2000)
        .build_service();

    // 測試所有 AI 配置項目
    assert_eq!(service.get_config_value("ai.provider").unwrap(), "test_provider");
    assert_eq!(service.get_config_value("ai.model").unwrap(), "test_model");
    assert_eq!(service.get_config_value("ai.api_key").unwrap(), "test_key");
    assert_eq!(service.get_config_value("ai.base_url").unwrap(), "https://test.api");
    assert_eq!(service.get_config_value("ai.temperature").unwrap(), "0.8");
    assert_eq!(service.get_config_value("ai.max_sample_length").unwrap(), "5000");
    assert_eq!(service.get_config_value("ai.retry_attempts").unwrap(), "5");
    assert_eq!(service.get_config_value("ai.retry_delay_ms").unwrap(), "2000");
}

#[test]
fn test_get_config_value_vad_configurations() {
    let service = TestConfigBuilder::new()
        .with_vad_enabled(true)
        .with_vad_sensitivity(0.9)
        .with_vad_chunk_size(1024)
        .with_vad_sample_rate(22050)
        .with_vad_padding_chunks(5)
        .with_vad_min_speech_duration_ms(150)
        .with_vad_speech_merge_gap_ms(250)
        .build_service();

    // 測試所有 VAD 配置項目
    assert_eq!(service.get_config_value("sync.vad.enabled").unwrap(), "true");
    assert_eq!(service.get_config_value("sync.vad.sensitivity").unwrap(), "0.9");
    assert_eq!(service.get_config_value("sync.vad.chunk_size").unwrap(), "1024");
    assert_eq!(service.get_config_value("sync.vad.sample_rate").unwrap(), "22050");
    assert_eq!(service.get_config_value("sync.vad.padding_chunks").unwrap(), "5");
    assert_eq!(service.get_config_value("sync.vad.min_speech_duration_ms").unwrap(), "150");
    assert_eq!(service.get_config_value("sync.vad.speech_merge_gap_ms").unwrap(), "250");
}

#[test]
fn test_get_config_value_parallel_configurations() {
    let service = TestConfigBuilder::new()
        .with_parallel_max_workers(12)
        .with_parallel_task_queue_size(2000)
        .with_parallel_enable_task_priorities(true)
        .with_parallel_auto_balance_workers(false)
        .with_parallel_overflow_strategy("Drop")
        .build_service();

    // 測試所有並行配置項目
    assert_eq!(service.get_config_value("parallel.max_workers").unwrap(), "12");
    assert_eq!(service.get_config_value("parallel.task_queue_size").unwrap(), "2000");
    assert_eq!(service.get_config_value("parallel.enable_task_priorities").unwrap(), "true");
    assert_eq!(service.get_config_value("parallel.auto_balance_workers").unwrap(), "false");
    assert_eq!(service.get_config_value("parallel.overflow_strategy").unwrap(), "Drop");
}

#[test]
fn test_get_set_config_value_consistency() {
    let service = TestConfigBuilder::new().build_service();
    
    // 測試所有支援的配置鍵
    let test_cases = vec![
        // AI 配置 (8 項)
        ("ai.provider", "openai"),
        ("ai.model", "gpt-4"),
        ("ai.api_key", "test-key"),
        ("ai.base_url", "https://api.test.com"),
        ("ai.temperature", "0.7"),
        ("ai.max_sample_length", "8000"),
        ("ai.retry_attempts", "3"),
        ("ai.retry_delay_ms", "1500"),
        
        // 格式配置 (4 項)
        ("formats.default_output", "vtt"),
        ("formats.preserve_styling", "true"),
        ("formats.default_encoding", "utf-8"),
        ("formats.encoding_detection_confidence", "0.9"),
        
        // 同步配置 (9 項)
        ("sync.max_offset_seconds", "90.0"),
        ("sync.default_method", "vad"),
        ("sync.correlation_threshold", "0.7"),
        ("sync.audio_sample_rate", "48000"),
        
        // VAD 配置 (7 項)
        ("sync.vad.enabled", "true"),
        ("sync.vad.sensitivity", "0.8"),
        ("sync.vad.chunk_size", "512"),
        ("sync.vad.sample_rate", "16000"),
        ("sync.vad.padding_chunks", "4"),
        ("sync.vad.min_speech_duration_ms", "120"),
        ("sync.vad.speech_merge_gap_ms", "180"),
        
        // 一般配置 (5 項)
        ("general.backup_enabled", "true"),
        ("general.max_concurrent_jobs", "8"),
        ("general.task_timeout_seconds", "600"),
        ("general.enable_progress_bar", "false"),
        ("general.worker_idle_timeout_seconds", "120"),
        
        // 並行配置 (5 項)
        ("parallel.max_workers", "6"),
        ("parallel.task_queue_size", "1500"),
        ("parallel.enable_task_priorities", "true"),
        ("parallel.auto_balance_workers", "false"),
        ("parallel.overflow_strategy", "Expand"),
    ];

    for (key, value) in test_cases {
        // 測試 set 然後 get
        assert!(service.set_config_value(key, value).is_ok(), 
                "Failed to set {}", key);
        
        let retrieved = service.get_config_value(key);
        assert!(retrieved.is_ok(), "Failed to get {}", key);
        assert_eq!(retrieved.unwrap(), value, "Value mismatch for {}", key);
    }
}
```

#### 4.3 CLI 整合測試

**檔案：** `tests/cli_config_complete_integration_tests.rs`

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_cli_config_get_all_supported_keys() {
    let temp = TempDir::new().unwrap();
    let config_file = temp.path().join("config.toml");
    
    // 建立完整的配置檔案
    let config_content = r#"
[ai]
provider = "openai"
model = "gpt-4"
api_key = "test-key"
base_url = "https://api.test.com"
temperature = 0.7
max_sample_length = 8000
retry_attempts = 3
retry_delay_ms = 1500

[formats]
default_output = "vtt"
preserve_styling = true
default_encoding = "utf-8"
encoding_detection_confidence = 0.9

[sync]
max_offset_seconds = 90.0
default_method = "vad"

[sync.vad]
enabled = true
sensitivity = 0.8
chunk_size = 512
sample_rate = 16000
padding_chunks = 4
min_speech_duration_ms = 120
speech_merge_gap_ms = 180

[general]
backup_enabled = true
max_concurrent_jobs = 8
task_timeout_seconds = 600
enable_progress_bar = false
worker_idle_timeout_seconds = 120

[parallel]
max_workers = 6
task_queue_size = 1500
enable_task_priorities = true
auto_balance_workers = false
overflow_strategy = "Expand"
"#;
    
    fs::write(&config_file, config_content).unwrap();
    
    // 測試 CLI 命令
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.env("SUBX_CONFIG_PATH", config_file.to_str().unwrap());
    
    // 測試獲取各種配置值
    let test_keys = [
        "ai.provider",
        "ai.max_sample_length",
        "sync.vad.enabled",
        "sync.vad.sensitivity",
        "parallel.overflow_strategy",
        "general.enable_progress_bar",
    ];
    
    for key in &test_keys {
        cmd.args(&["config", "get", key])
           .assert()
           .success();
    }
}

#[test]
fn test_cli_sync_max_offset_validation() {
    let temp = TempDir::new().unwrap();
    let config_file = temp.path().join("config.toml");
    let subtitle_file = temp.path().join("test.srt");
    
    // 建立配置：max_offset_seconds = 30.0
    fs::write(&config_file, r#"
[sync]
max_offset_seconds = 30.0
"#).unwrap();
    
    // 建立測試字幕
    fs::write(&subtitle_file, "1\n00:00:01,000 --> 00:00:03,000\nTest").unwrap();
    
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.env("SUBX_CONFIG_PATH", config_file.to_str().unwrap());
    
    // 測試超過限制的偏移量 (應該失敗)
    cmd.args(&["sync", "--offset", "45.0", subtitle_file.to_str().unwrap()])
       .assert()
       .failure()
       .stderr(predicate::str::contains("超過配置的最大允許值"));
    
    // 測試在限制內的偏移量 (應該成功)
    let mut cmd2 = Command::cargo_bin("subx-cli").unwrap();
    cmd2.env("SUBX_CONFIG_PATH", config_file.to_str().unwrap());
    cmd2.args(&["sync", "--offset", "15.0", subtitle_file.to_str().unwrap()])
        .assert()
        .success();
}
```

### 階段 5: 文件和 CLI 幫助更新

#### 5.1 更新配置文檔

**檔案：** `docs/configuration-guide.md`

需要更新同步配置部分，強調 max_offset_seconds 的實際作用：

```markdown
### Sync Configuration (`[sync]`)

#### Basic Configuration

```toml
[sync]
max_offset_seconds = 60.0            # 最大允許時間偏移量（秒），用於限制檢測和手動偏移結果
default_method = "vad"               # 預設同步方法（目前僅支援 "vad"）

# Local VAD configuration
[sync.vad]
enabled = true                       # 啟用本地 VAD 方法（布林值）
sensitivity = 0.75                   # 語音檢測敏感度（0.0-1.0）（浮點數）
chunk_size = 512                     # 音訊塊大小（樣本數）（無符號整數）
sample_rate = 16000                  # 處理採樣率（赫茲）（無符號 32 位整數）
padding_chunks = 3                   # 語音檢測前後的填充塊數（無符號 32 位整數）
min_speech_duration_ms = 100         # 最小語音持續時間（毫秒）（無符號 32 位整數）
speech_merge_gap_ms = 200            # 語音段合併間隔（毫秒）（無符號 32 位整數）
```

#### max_offset_seconds 配置詳解

`max_offset_seconds` 配置項目控制同步操作中允許的最大時間偏移量：

**影響範圍：**
- **手動偏移模式 (`--offset`)**: 如果指定的偏移量超過此限制，命令將拒絕執行
- **自動檢測模式**: 如果 VAD 檢測到的偏移量超過此限制，將截斷到最大值並提供警告

**建議值：**
- **一般使用**: 60.0 秒（預設值）
- **高品質影片**: 30.0-45.0 秒
- **低品質/複雜音訊**: 90.0-120.0 秒
- **特殊需求**: 最高可設定 3600.0 秒（1 小時）

**使用範例：**
```bash
# 設定最大偏移量為 90 秒
subx-cli config set sync.max_offset_seconds 90.0

# 驗證設定
subx-cli config get sync.max_offset_seconds

# 使用手動偏移（會檢查限制）
subx-cli sync --offset 75.0 movie.srt  # 在限制內，成功執行
subx-cli sync --offset 120.0 movie.srt # 超過限制，返回錯誤
```
```

#### 5.2 更新 CLI 說明文字

**檔案：** `src/cli/sync_args.rs`

更新 offset 參數的說明：

```rust
#[arg(
    long,
    value_name = "SECONDS",
    help = "Manual offset in seconds (positive delays subtitles, negative advances them). \
           Must not exceed sync.max_offset_seconds configuration (default: 60.0s)",
    conflicts_with_all = ["method", "window", "vad_sensitivity"]
)]
pub offset: Option<f32>,
```

**檔案：** `src/cli/config_args.rs`

添加對新配置鍵的說明：

```rust
/// # Supported Configuration Keys
///
/// ## AI Configuration (8 keys)
/// - `ai.provider` - AI service provider selection
/// - `ai.model` - AI model selection  
/// - `ai.api_key` - API authentication key
/// - `ai.base_url` - Custom API endpoint URL
/// - `ai.temperature` - AI response randomness (0.0-1.0)
/// - `ai.max_sample_length` - Maximum text sample length
/// - `ai.retry_attempts` - Number of retry attempts for failed requests
/// - `ai.retry_delay_ms` - Delay between retry attempts in milliseconds
///
/// ## Sync Configuration (16 keys)
/// - `sync.max_offset_seconds` - Maximum allowed time offset for sync operations
/// - `sync.default_method` - Default synchronization method
/// - `sync.vad.enabled` - Enable Voice Activity Detection
/// - `sync.vad.sensitivity` - VAD detection sensitivity (0.0-1.0)  
/// - `sync.vad.chunk_size` - Audio processing chunk size
/// - `sync.vad.sample_rate` - Audio processing sample rate
/// - `sync.vad.padding_chunks` - Padding chunks for speech detection
/// - `sync.vad.min_speech_duration_ms` - Minimum speech segment duration
/// - `sync.vad.speech_merge_gap_ms` - Gap for merging speech segments
/// - And 7 legacy deprecated sync settings (maintained for compatibility)
///
/// ## General Configuration (5 keys)
/// - `general.backup_enabled` - Enable automatic file backups
/// - `general.max_concurrent_jobs` - Maximum concurrent processing jobs
/// - `general.task_timeout_seconds` - Task execution timeout
/// - `general.enable_progress_bar` - Show progress indicators
/// - `general.worker_idle_timeout_seconds` - Worker thread idle timeout
///
/// ## Parallel Processing Configuration (5 keys)  
/// - `parallel.max_workers` - Maximum worker threads
/// - `parallel.task_queue_size` - Task queue capacity
/// - `parallel.enable_task_priorities` - Enable task priority scheduling
/// - `parallel.auto_balance_workers` - Enable automatic load balancing
/// - `parallel.overflow_strategy` - Queue overflow handling strategy
///
/// ## Format Configuration (4 keys)
/// - `formats.default_output` - Default output subtitle format
/// - `formats.preserve_styling` - Preserve subtitle styling information
/// - `formats.default_encoding` - Default file encoding
/// - `formats.encoding_detection_confidence` - Encoding detection threshold
```

## 實作順序

### 第 1 週：max_offset_seconds 功能實作

**Day 1-2**: 更新同步引擎核心邏輯
- 修改 `SyncEngine::apply_manual_offset()` 添加偏移量限制檢查
- 修改 `SyncEngine::vad_detect_sync_offset()` 添加檢測結果限制

**Day 3-4**: 更新 CLI 命令處理邏輯  
- 修改 `sync_command::execute_with_config()` 添加手動偏移驗證
- 增強錯誤信息和使用者指引

**Day 5**: 增強配置驗證
- 更新 `SyncConfig::validate()` 方法
- 添加建議值範圍檢查

### 第 2 週：get_config_value 完整性實作

**Day 1-2**: 擴展 ProductionConfigService
- 完整實作 `get_config_value()` 方法
- 添加所有缺失的配置鍵支援

**Day 3-4**: 更新 TestConfigService
- 同步 `get_config_value()` 實作
- 確保測試與生產環境一致性

**Day 5**: set_config_value 補充
- 添加 VAD 配置和其他缺失項目的 set 支援
- 完善驗證邏輯

### 第 3 週：測試實作與驗證

**Day 1-2**: 功能測試
- 實作 max_offset_seconds 相關測試
- 測試手動偏移和自動檢測的限制邏輯

**Day 3-4**: 配置管理測試
- 實作 get/set 一致性測試
- 驗證所有支援的配置鍵

**Day 5**: CLI 整合測試
- 實作命令列整合測試
- 驗證錯誤處理和使用者體驗

### 第 4 週：文檔更新與最終驗證

**Day 1-2**: 更新技術文檔
- 更新配置指南
- 更新 CLI 說明文字

**Day 3-4**: 全面測試驗證
- 執行完整測試套件
- 驗證向後相容性

**Day 5**: 品質檢查與部署準備
- 程式碼審查和最佳化
- 準備部署和發布

## 測試策略

### 單元測試

1. **配置驗證測試**
   - max_offset_seconds 範圍驗證
   - VAD 配置參數驗證
   - 錯誤情況處理

2. **同步引擎測試**
   - 偏移量限制邏輯
   - 警告和錯誤生成
   - 結果截斷行為

3. **配置服務測試**
   - get/set 方法一致性
   - 所有支援配置鍵的完整性
   - 錯誤處理

### 整合測試

1. **命令列整合**
   - CLI 參數驗證
   - 配置檔案讀取
   - 錯誤信息顯示

2. **端到端工作流程**
   - 完整同步流程測試
   - 配置管理操作測試
   - 錯誤復原測試

### 效能測試

1. **配置服務效能**
   - get/set 操作響應時間
   - 大量配置項目處理
   - 記憶體使用情況

2. **同步引擎效能**
   - 偏移量驗證開銷
   - 大檔案處理性能
   - 錯誤處理效能

## 相容性考量

### 向後相容性

1. **配置檔案相容性**
   - 現有配置檔案繼續有效
   - 新增配置項目使用合理預設值
   - 棄用配置保持可用但標記為過時

2. **CLI 介面相容性**  
   - 現有命令和參數繼續工作
   - 新增驗證不破壞現有工作流程
   - 錯誤信息提供清晰的解決方案

3. **API 相容性**
   - ConfigService 介面保持穩定
   - 新增方法不影響現有實作
   - 測試介面與生產介面保持一致

### 升級路徑

1. **配置遷移**
   - 自動檢測並轉換舊配置格式
   - 提供配置驗證和修復建議
   - 支援批量配置更新

2. **文檔遷移**
   - 更新所有相關文檔
   - 提供配置升級指南
   - 添加最佳實踐建議

## 風險評估

### 高風險項目

1. **配置驗證邏輯複雜性**
   - **風險**: 新的驗證邏輯可能引入邊界情況錯誤
   - **緩解**: 全面的單元測試和邊界情況測試

2. **向後相容性問題**
   - **風險**: 新的限制可能破壞現有工作流程
   - **緩解**: 提供清晰的錯誤信息和解決方案

### 中風險項目

1. **效能影響**
   - **風險**: 額外的配置檢查可能影響效能
   - **緩解**: 效能測試和最佳化

2. **測試覆蓋不足**
   - **風險**: 複雜的配置邏輯可能有未測試的路徑
   - **緩解**: 全面的測試策略和程式碼審查

### 低風險項目

1. **文檔更新延遲**
   - **風險**: 文檔可能滯後於實作
   - **緩解**: 並行進行文檔更新

## 成功標準

### 功能標準

- [ ] **max_offset_seconds 功能完整實作**
  - 手動偏移模式驗證偏移量限制
  - 自動檢測模式截斷超限結果並提供警告
  - 配置驗證邏輯完善

- [ ] **get_config_value 完整支援**
  - 支援所有 31 項配置鍵（與 set_config_value 一致）
  - 包含所有 VAD 配置項目
  - 錯誤處理完善

- [ ] **配置管理一致性**
  - ProductionConfigService 和 TestConfigService 行為一致
  - get 和 set 方法支援相同的配置項目
  - 驗證邏輯統一

### 品質標準

- [ ] **測試覆蓋完整**
  - 單元測試覆蓋率 > 95%
  - 整合測試覆蓋所有主要功能路徑
  - 邊界情況測試完整

- [ ] **效能標準**
  - 配置操作響應時間 < 10ms
  - 同步操作額外開銷 < 5%
  - 記憶體使用無明顯增加

- [ ] **使用者體驗標準**
  - 錯誤信息清晰具體
  - 提供具體的解決方案建議
  - CLI 回饋信息完整

### 文檔標準

- [ ] **技術文檔完整**
  - 配置指南包含所有新功能
  - CLI 說明文字準確完整
  - 程式碼註解清晰

- [ ] **使用範例充實**
  - 提供常見使用情境範例
  - 包含錯誤處理範例
  - 配置最佳實踐指南

## 後續維護

### 監控和告警

1. **配置使用情況監控**
   - 追蹤常用的配置設定
   - 識別可能的設定問題
   - 監控效能影響

2. **錯誤情況分析**
   - 收集偏移量限制觸發情況
   - 分析配置錯誤模式
   - 改進錯誤信息和使用者指引

### 持續改進

1. **配置系統最佳化**
   - 根據使用情況調整預設值
   - 簡化複雜配置項目
   - 改進驗證邏輯

2. **使用者體驗改進**
   - 收集使用者回饋
   - 簡化配置管理工作流程
   - 提供更直觀的配置介面

---

**預估完成時間**: 4 週  
**優先級**: 高  
**複雜度**: 中高  
**影響範圍**: 配置系統、同步引擎、CLI 介面、測試系統

**關鍵利益相關者**: 
- 核心開發團隊（實作）
- QA 團隊（測試）
- 技術文檔團隊（文檔）
- 使用者支援團隊（回饋收集）
