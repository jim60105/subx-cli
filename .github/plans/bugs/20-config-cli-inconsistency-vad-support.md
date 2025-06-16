# Bug 20: 配置 CLI 不一致性 - VAD 配置支援缺失

## 問題描述

### 核心問題
在配置稽核過程中發現嚴重的配置 CLI 一致性問題：`ProductionConfigService` 的 `get_config_value()` 和 `set_config_value()` 方法支援的配置項目數量存在顯著差異，特別是所有 VAD 相關配置完全無法透過 CLI 命令進行操作。

### 問題嚴重性分析
1. **VAD 配置完全缺失**：7 個 VAD 相關配置項目無法透過 `subx config get/set` 命令操作
2. **支援不對稱**：`get_config_value` 僅支援 15 項配置，而 `set_config_value` 支援 31 項
3. **使用者體驗問題**：使用者無法透過 CLI 查看或修改關鍵的 VAD 配置，必須手動編輯配置檔案
4. **功能完整性影響**：VAD 是 SubX 的核心功能，其配置不可存取嚴重影響系統可用性

### 具體缺失的配置項目

#### VAD 相關配置（完全缺失）
```toml
[sync.vad]
enabled = true                       # 是否啟用 VAD 檢測
sensitivity = 0.75                   # 語音檢測敏感度 (0.0-1.0)
chunk_size = 512                     # 音訊塊大小
sample_rate = 16000                  # 處理採樣率
padding_chunks = 3                   # 填充塊數量
min_speech_duration_ms = 100         # 最小語音持續時間
speech_merge_gap_ms = 200            # 語音段合併間隔
```

#### 其他缺失的配置項目
- `sync.default_method`：同步方法選擇
- `ai.max_sample_length`：AI 內容長度限制
- `ai.retry_attempts`：API 重試次數
- `ai.retry_delay_ms`：重試延遲時間
- `formats.encoding_detection_confidence`：編碼檢測信心度
- `general.task_timeout_seconds`：任務逾時設定
- `general.enable_progress_bar`：進度條顯示控制
- `general.worker_idle_timeout_seconds`：工作執行緒逾時
- `parallel.task_queue_size`：任務佇列大小
- `parallel.enable_task_priorities`：優先級排程
- `parallel.auto_balance_workers`：自動負載平衡
- `parallel.overflow_strategy`：佇列溢出策略

## 根本原因分析

### 程式碼位置
- **檔案**：`src/config/service.rs`
- **get_config_value 方法**：第 522-542 行
- **set_config_value 方法**：第 281-416 行

### 架構設計問題
1. **手動維護配置清單**：兩個方法都使用硬編碼的 match 分支，容易遺漏新增配置
2. **缺乏自動化同步機制**：新增配置項目時沒有自動檢查確保兩個方法都支援
3. **測試覆蓋不足**：沒有測試來驗證 get/set 方法支援的配置項目一致性
4. **文檔更新滯後**：配置變更時沒有同步更新相關文檔

### 實作不一致性
```rust
// get_config_value 僅支援基本配置
match parts.as_slice() {
    ["ai", "provider"] => Ok(config.ai.provider.clone()),
    ["ai", "model"] => Ok(config.ai.model.clone()),
    // ... 僅 15 項配置
    ["parallel", "max_workers"] => Ok(config.parallel.max_workers.to_string()),
    _ => Err(SubXError::config(format!("Unknown configuration key: {}", key))),
}

// set_config_value 支援更多配置，包括已棄用項目
match parts.as_slice() {
    ["ai", "provider"] => { /* ... */ },
    ["ai", "max_sample_length"] => { /* ... */ },
    ["ai", "retry_attempts"] => { /* ... */ },
    // ... 31 項配置，但缺少所有 VAD 配置
    ["parallel", "overflow_strategy"] => { /* ... */ },
    // 但仍然缺少 sync.vad.* 配置
}
```

## 影響範圍

### 使用者體驗影響
- **配置困難**：使用者無法透過 CLI 快速調整 VAD 參數來優化語音檢測效果
- **調試障礙**：無法快速查看當前的 VAD 配置值進行問題排查
- **學習曲線**：使用者必須學習 TOML 格式和配置檔案位置才能修改 VAD 設定
- **一致性困惑**：部分配置可以透過 CLI 設定但無法讀取，造成使用體驗不一致

### 功能完整性影響
- **核心功能受限**：VAD 是 SubX 的主要同步方法，其配置不可存取影響核心功能使用
- **自動化腳本限制**：無法編寫自動化腳本來動態調整 VAD 參數
- **CI/CD 整合問題**：在自動化環境中難以配置和驗證 VAD 設定

### 維護成本影響
- **支援負擔**：需要更多文檔說明手動配置檔案編輯流程
- **錯誤處理複雜**：使用者可能在手動編輯配置檔案時引入語法錯誤
- **版本升級風險**：配置檔案格式變更時可能破壞現有設定

## 技術債務分析

### 當前債務清單
1. **配置項目手動維護**：新增配置時容易遺漏 CLI 支援
2. **測試覆蓋不足**：缺乏配置一致性測試
3. **文檔不同步**：配置變更時文檔更新滯後
4. **錯誤處理不一致**：不同配置項目的錯誤訊息和驗證邏輯不統一

### 風險評估
- **高風險**：VAD 配置缺失影響核心功能使用
- **中風險**：配置不一致性可能導致使用者困惑和支援問題
- **低風險**：部分非核心配置項目的缺失影響相對較小

## 解決方案設計

### 設計原則
1. **完整性**：確保所有定義的配置項目都可透過 CLI 操作
2. **一致性**：`get_config_value` 和 `set_config_value` 必須支援相同的配置項目
3. **可維護性**：使用更系統化的方法來管理配置項目清單
4. **向後相容性**：保持現有 CLI 介面的相容性

### 技術架構改進
1. **統一配置項目定義**：建立集中的配置項目註冊機制
2. **自動化測試**：新增測試來確保 get/set 方法的一致性
3. **型別安全改進**：使用更型別安全的方法來處理配置值轉換
4. **錯誤處理統一**：標準化配置錯誤訊息和驗證邏輯

## 實作計劃

### 階段一：分析和準備（估計時間：1-2 小時）
1. **完整配置項目清單**
   - 分析 `Config` 結構體的所有欄位
   - 識別所有需要支援的配置項目（不含已棄用項目）
   - 建立配置項目對應的 CLI 鍵值清單

2. **當前實作分析**
   - 詳細分析 `get_config_value` 和 `set_config_value` 的實作
   - 識別所有缺失的配置項目
   - 分析每個配置項目的型別和驗證需求

### 階段二：get_config_value 方法擴展（估計時間：2-3 小時）
1. **新增 VAD 配置支援**
   ```rust
   // 新增 VAD 配置讀取
   ["sync", "vad", "enabled"] => Ok(config.sync.vad.enabled.to_string()),
   ["sync", "vad", "sensitivity"] => Ok(config.sync.vad.sensitivity.to_string()),
   ["sync", "vad", "chunk_size"] => Ok(config.sync.vad.chunk_size.to_string()),
   ["sync", "vad", "sample_rate"] => Ok(config.sync.vad.sample_rate.to_string()),
   ["sync", "vad", "padding_chunks"] => Ok(config.sync.vad.padding_chunks.to_string()),
   ["sync", "vad", "min_speech_duration_ms"] => Ok(config.sync.vad.min_speech_duration_ms.to_string()),
   ["sync", "vad", "speech_merge_gap_ms"] => Ok(config.sync.vad.speech_merge_gap_ms.to_string()),
   ```

2. **新增其他缺失配置**
   ```rust
   // AI 配置補完
   ["ai", "max_sample_length"] => Ok(config.ai.max_sample_length.to_string()),
   ["ai", "retry_attempts"] => Ok(config.ai.retry_attempts.to_string()),
   ["ai", "retry_delay_ms"] => Ok(config.ai.retry_delay_ms.to_string()),
   
   // 同步配置補完
   ["sync", "default_method"] => Ok(config.sync.default_method.clone()),
   
   // 格式配置補完
   ["formats", "encoding_detection_confidence"] => Ok(config.formats.encoding_detection_confidence.to_string()),
   
   // 一般配置補完
   ["general", "task_timeout_seconds"] => Ok(config.general.task_timeout_seconds.to_string()),
   ["general", "enable_progress_bar"] => Ok(config.general.enable_progress_bar.to_string()),
   ["general", "worker_idle_timeout_seconds"] => Ok(config.general.worker_idle_timeout_seconds.to_string()),
   
   // 並行配置補完
   ["parallel", "task_queue_size"] => Ok(config.parallel.task_queue_size.to_string()),
   ["parallel", "enable_task_priorities"] => Ok(config.parallel.enable_task_priorities.to_string()),
   ["parallel", "auto_balance_workers"] => Ok(config.parallel.auto_balance_workers.to_string()),
   ["parallel", "overflow_strategy"] => Ok(format!("{:?}", config.parallel.overflow_strategy)),
   ```

### 階段三：set_config_value 方法擴展（估計時間：2-3 小時）
1. **新增 VAD 配置設定支援**
   ```rust
   // VAD 配置設定
   ["sync", "vad", "enabled"] => {
       let v = parse_bool(value)?;
       config.sync.vad.enabled = v;
   }
   ["sync", "vad", "sensitivity"] => {
       let v = validate_float_range(value, 0.0, 1.0)?;
       config.sync.vad.sensitivity = v;
   }
   ["sync", "vad", "chunk_size"] => {
       let v = validate_power_of_two(value)?;
       config.sync.vad.chunk_size = v;
   }
   ["sync", "vad", "sample_rate"] => {
       validate_enum(value, &["8000", "16000", "22050", "44100", "48000"])?;
       config.sync.vad.sample_rate = value.parse().unwrap();
   }
   // ... 其他 VAD 配置
   ```

2. **新增 sync.default_method 支援**
   ```rust
   ["sync", "default_method"] => {
       validate_enum(value, &["auto", "vad"])?;
       config.sync.default_method = value.to_string();
   }
   ```

3. **移除已棄用配置支援**
   - 移除對 `correlation_threshold`, `dialogue_detection_threshold` 等已棄用配置的支援
   - 確保這些配置項目不會被意外設定

### 階段四：測試實作（估計時間：3-4 小時）
1. **配置一致性測試**
   ```rust
   #[test]
   fn test_config_get_set_consistency() {
       // 確保所有可設定的配置項目都可以讀取
       let supported_keys = get_all_supported_config_keys();
       
       for key in supported_keys {
           // 測試可以設定
           assert!(can_set_config_value(key), "Cannot set config key: {}", key);
           // 測試可以讀取
           assert!(can_get_config_value(key), "Cannot get config key: {}", key);
       }
   }
   ```

2. **VAD 配置專項測試**
   ```rust
   #[test]
   fn test_vad_config_cli_support() {
       test_with_config!(
           TestConfigBuilder::new(),
           |config_service: &dyn ConfigService| {
               // 測試 VAD 配置的完整 get/set 循環
               config_service.set_config_value("sync.vad.enabled", "false").unwrap();
               assert_eq!(config_service.get_config_value("sync.vad.enabled").unwrap(), "false");
               
               config_service.set_config_value("sync.vad.sensitivity", "0.8").unwrap();
               assert_eq!(config_service.get_config_value("sync.vad.sensitivity").unwrap(), "0.8");
               
               // ... 測試所有 VAD 配置項目
           }
       );
   }
   ```

3. **錯誤處理測試**
   ```rust
   #[test]
   fn test_vad_config_validation() {
       test_with_config!(
           TestConfigBuilder::new(),
           |config_service: &dyn ConfigService| {
               // 測試無效值的錯誤處理
               assert!(config_service.set_config_value("sync.vad.sensitivity", "1.5").is_err());
               assert!(config_service.set_config_value("sync.vad.chunk_size", "100").is_err()); // 不是 2 的冪次
               assert!(config_service.set_config_value("sync.vad.sample_rate", "12000").is_err()); // 不支援的採樣率
           }
       );
   }
   ```

### 階段五：整合測試和驗證（估計時間：2-3 小時）
1. **CLI 整合測試**
   ```bash
   # 測試 VAD 配置的 CLI 操作
   subx config set sync.vad.enabled true
   subx config get sync.vad.enabled  # 應該返回 "true"
   
   subx config set sync.vad.sensitivity 0.8
   subx config get sync.vad.sensitivity  # 應該返回 "0.8"
   
   # 測試錯誤處理
   subx config set sync.vad.sensitivity 1.5  # 應該返回錯誤
   ```

2. **回歸測試**
   - 執行完整的測試套件確保沒有破壞現有功能
   - 驗證所有現有的配置項目仍然正常工作
   - 測試配置檔案載入和保存功能

### 階段六：文檔更新（估計時間：1-2 小時）
1. **更新配置指南**
   - 在 `docs/configuration-guide.md` 中新增 VAD 配置的 CLI 操作範例
   - 更新配置項目清單和說明

2. **更新 CLI 幫助文檔**
   - 確保 `subx config --help` 顯示最新的配置項目資訊
   - 新增 VAD 配置的使用範例

3. **更新配置使用分析文檔**
   - 在 `docs/config-usage-analysis.md` 中更新配置一致性問題的狀態
   - 標記問題已解決並更新相關統計數據

## 實作檢查清單

### 程式碼修改
- [ ] 分析 `Config` 結構體，建立完整的配置項目清單
- [ ] 擴展 `get_config_value` 方法支援所有缺失的配置項目
- [ ] 擴展 `set_config_value` 方法支援 VAD 和其他缺失配置
- [ ] 新增 `sync.default_method` 的 get/set 支援
- [ ] 移除已棄用配置項目的 set 支援
- [ ] 實作 VAD 配置的專用驗證邏輯
- [ ] 統一錯誤訊息格式和處理邏輯

### 驗證邏輯實作
- [ ] `sync.vad.sensitivity`：範圍驗證 (0.0-1.0)
- [ ] `sync.vad.chunk_size`：2 的冪次驗證
- [ ] `sync.vad.sample_rate`：支援採樣率清單驗證
- [ ] `sync.vad.padding_chunks`：正整數範圍驗證
- [ ] `sync.vad.min_speech_duration_ms`：正整數範圍驗證
- [ ] `sync.vad.speech_merge_gap_ms`：正整數範圍驗證
- [ ] `sync.default_method`：枚舉值驗證 ("auto", "vad")

### 測試實作
- [ ] 新增配置一致性測試，確保 get/set 方法支援相同配置項目
- [ ] 新增 VAD 配置專項測試，測試所有 VAD 配置的 get/set 循環
- [ ] 新增配置驗證測試，測試各種無效值的錯誤處理
- [ ] 新增 CLI 整合測試，測試透過命令列操作 VAD 配置
- [ ] 更新現有測試以涵蓋新增的配置項目
- [ ] 執行完整回歸測試確保沒有破壞現有功能

### 文檔更新
- [ ] 更新 `docs/configuration-guide.md` 新增 VAD CLI 操作範例
- [ ] 更新 `src/cli/config_args.rs` 中的 rustdoc 註解
- [ ] 更新 `README.md` 中的配置相關範例
- [ ] 更新 `docs/config-usage-analysis.md` 標記問題已解決
- [ ] 新增 VAD 配置調整的最佳實踐指南
- [ ] 更新錯誤訊息說明和疑難排解指南

### 品質確保
- [ ] 執行 `cargo test` 確保所有測試通過
- [ ] 執行 `cargo clippy -- -D warnings` 確保沒有警告
- [ ] 執行 `cargo fmt` 確保程式碼格式正確
- [ ] 執行 `scripts/quality_check.sh` 進行完整品質檢查
- [ ] 執行 `scripts/check_coverage.sh -T` 確保測試覆蓋率達標
- [ ] 手動測試各種 VAD 配置情境確保功能正確

## 驗證標準

### 功能驗證
1. **完整性驗證**
   ```bash
   # 所有 VAD 配置都可以透過 CLI 操作
   subx config get sync.vad.enabled
   subx config get sync.vad.sensitivity
   subx config get sync.vad.chunk_size
   subx config get sync.vad.sample_rate
   subx config get sync.vad.padding_chunks
   subx config get sync.vad.min_speech_duration_ms
   subx config get sync.vad.speech_merge_gap_ms
   ```

2. **一致性驗證**
   ```bash
   # get/set 循環測試
   original_value=$(subx config get sync.vad.sensitivity)
   subx config set sync.vad.sensitivity 0.8
   new_value=$(subx config get sync.vad.sensitivity)
   test "$new_value" = "0.8"
   subx config set sync.vad.sensitivity "$original_value"
   ```

3. **錯誤處理驗證**
   ```bash
   # 無效值應該返回錯誤
   subx config set sync.vad.sensitivity 1.5 && exit 1  # 應該失敗
   subx config set sync.vad.chunk_size 100 && exit 1   # 應該失敗
   echo "Error handling works correctly"
   ```

### 效能驗證
- 確保新增的配置項目不會顯著影響配置載入/保存效能
- 驗證大量配置操作的效能表現
- 確保記憶體使用量沒有異常增加

### 相容性驗證
- 確保現有的配置檔案可以正常載入
- 驗證環境變數覆蓋功能仍然正常
- 確保配置檔案格式沒有破壞性變更

## 風險評估和緩解

### 高風險項目
1. **配置檔案相容性**
   - **風險**：新增配置項目可能影響現有配置檔案載入
   - **緩解**：確保所有新增項目都有適當的預設值，向後相容

2. **效能影響**
   - **風險**：新增大量配置項目可能影響配置操作效能
   - **緩解**：使用高效的 match 分支，避免複雜的巢狀邏輯

### 中風險項目
1. **測試覆蓋不足**
   - **風險**：新增功能的測試覆蓋可能不完整
   - **緩解**：實作全面的測試套件，包括單元測試和整合測試

2. **文檔同步**
   - **風險**：文檔更新可能不及時或不完整
   - **緩解**：將文檔更新作為實作檢查清單的必要項目

### 低風險項目
1. **使用者適應**
   - **風險**：使用者可能需要時間適應新的 CLI 功能
   - **緩解**：提供清楚的文檔和範例，保持向後相容性

## 成功標準

### 量化指標
- 所有 38 個配置項目（排除已棄用項目）都可透過 CLI 進行 get/set 操作
- `get_config_value` 和 `set_config_value` 支援的配置項目數量完全一致
- 所有新增功能的測試覆蓋率達到 90% 以上
- 配置操作效能沒有顯著降低（< 5% 效能損失）

### 質化指標
- 使用者可以透過 CLI 完成所有 VAD 配置調整需求
- 錯誤訊息清楚且具有指導性
- 文檔完整且容易理解
- 程式碼結構清晰且易於維護

## 結論

這個 bug 修正將徹底解決 SubX 配置系統的一致性問題，特別是 VAD 配置的 CLI 支援缺失。通過系統性的方法來擴展 `get_config_value` 和 `set_config_value` 方法，我們將：

1. **提升使用者體驗**：使用者可以透過 CLI 輕鬆調整所有配置項目，特別是關鍵的 VAD 參數
2. **改善功能完整性**：確保所有定義的配置項目都可透過標準介面存取
3. **增強系統一致性**：消除 get/set 方法之間的不一致性，提供統一的配置體驗
4. **降低維護成本**：建立完整的測試覆蓋和文檔，減少未來的支援負擔

此修正不僅解決了當前的技術債務，也為未來的配置系統擴展建立了更堅實的基礎。透過系統化的實作和全面的測試，我們可以確保 SubX 提供一致、可靠且使用者友善的配置管理體驗。
