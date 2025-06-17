# 37 - 統一服務工廠架構重構：整合 ComponentFactory 與 SyncServiceFactory

## 概述

本計劃旨在重構核心服務工廠架構，通過將 `SyncServiceFactory` 的職責整合到 `ComponentFactory` 中，消除重複的工廠邏輯，並建立統一的組件建立入口點。目前 `SyncServiceFactory` 位於 `src/core/services.rs` 中，與 `ServiceContainer` 混合在一起，這違反了單一職責原則並導致架構混亂。

## 問題描述

### 當前狀況
- `ComponentFactory` (`src/core/factory.rs`) 負責建立 `MatchEngine`、`FileManager` 和 `AIProvider`
- `SyncServiceFactory` (`src/core/services.rs`) 負責建立同步相關的偵測器
- 兩個工廠都需要 `ConfigService` 進行相依性注入
- `SyncServiceFactory` 的位置不合適，應該與其他工廠邏輯統一

### 影響評估
- 架構職責分散，違反 DRY 原則
- 開發者需要記住兩個不同的工廠入口點
- 測試複雜度增加，需要模擬多個工廠
- 未來擴展新服務時可能導致更多工廠類別

## 技術需求

### 主要目標
1. 將 `SyncServiceFactory` 的功能整合到 `ComponentFactory` 中
2. 移除 `src/core/services.rs` 中的 `SyncServiceFactory` 結構
3. 在 `ComponentFactory` 中新增同步相關組件的建立方法
4. 更新所有使用 `SyncServiceFactory` 的程式碼
5. 確保測試覆蓋率不降低
6. 維持 API 的向後相容性

### 技術規格
- 使用 Rust 的 trait 系統確保類型安全
- 遵循 SOLID 原則，特別是單一職責原則
- 使用相依性注入模式
- 確保執行緒安全性

## 實作計劃

### 階段 1：分析現有 SyncServiceFactory 功能
**預估時間：1 小時**

1. **檢視 SyncServiceFactory 實作**：
   ```bash
   # Review the current implementation
   grep -r "SyncServiceFactory" src/ --include="*.rs"
   ```

2. **識別所有使用點**：
   - 檢查 `src/core/services.rs` 中的 `SyncServiceFactory` 結構
   - 找出所有呼叫 `SyncServiceFactory` 的程式碼位置
   - 分析其建立的組件類型和配置需求

3. **建立遷移清單**：
   - 列出需要遷移的方法
   - 識別相依的 VAD 和音訊處理組件
   - 記錄當前的配置參數

### 階段 2：擴展 ComponentFactory
**預估時間：2 小時**

1. **新增同步相關建立方法**：
   ```rust
   // Add to src/core/factory.rs
   impl ComponentFactory {
       /// Create a VAD sync detector with sync configuration.
       ///
       /// Returns a properly configured VadSyncDetector instance using
       /// the sync configuration section.
       ///
       /// # Errors
       ///
       /// Returns an error if VAD detector creation fails.
       pub fn create_vad_sync_detector(&self) -> Result<Box<dyn crate::services::vad::VadSyncDetector>> {
           // Implementation will use self.config.sync settings
           create_vad_sync_detector(&self.config.sync)
       }

       /// Create a VAD detector with audio processing configuration.
       ///
       /// Returns a properly configured VadDetector instance.
       ///
       /// # Errors
       ///
       /// Returns an error if detector initialization fails.
       pub fn create_vad_detector(&self) -> Result<Box<dyn crate::services::vad::VadDetector>> {
           create_vad_detector(&self.config.sync)
       }

       /// Create an audio processor for VAD operations.
       ///
       /// Returns a properly configured AudioProcessor instance.
       pub fn create_audio_processor(&self) -> crate::services::vad::AudioProcessor {
           crate::services::vad::AudioProcessor::new()
       }
   }
   ```

2. **實作輔助建立函式**：
   ```rust
   // Add helper functions at the end of factory.rs
   
   /// Create a VAD sync detector from sync configuration.
   fn create_vad_sync_detector(sync_config: &crate::config::SyncConfig) -> Result<Box<dyn crate::services::vad::VadSyncDetector>> {
       // Implementation based on current SyncServiceFactory logic
       use crate::services::vad::sync_detector::VadSyncDetector as ConcreteVadSyncDetector;
       
       match sync_config.method {
           crate::core::sync::SyncMethod::LocalVad => {
               Ok(Box::new(ConcreteVadSyncDetector::new()?))
           }
           _ => Err(SubXError::config("Unsupported sync method for VAD detector"))
       }
   }

   /// Create a VAD detector from sync configuration.
   fn create_vad_detector(sync_config: &crate::config::SyncConfig) -> Result<Box<dyn crate::services::vad::VadDetector>> {
       use crate::services::vad::detector::VadDetector as ConcreteVadDetector;
       Ok(Box::new(ConcreteVadDetector::new()?))
   }
   ```

3. **更新測試**：
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[test]
       fn test_create_vad_sync_detector() {
           let config_service = TestConfigService::default();
           let factory = ComponentFactory::new(&config_service).unwrap();
           
           let result = factory.create_vad_sync_detector();
           // Test based on configuration method
           // This might fail initially if VAD components aren't ready
       }
   }
   ```

### 階段 3：更新使用點
**預估時間：1.5 小時**

1. **找出所有 SyncServiceFactory 使用點**：
   ```bash
   # Search for usage patterns
   grep -r "SyncServiceFactory" src/ --include="*.rs" -n
   ```

2. **更新命令模組**：
   - 檢查 `src/commands/sync_command.rs` 是否使用 `SyncServiceFactory`
   - 如果有使用，將其改為使用 `ComponentFactory`

3. **更新核心模組**：
   - 檢查 `src/core/sync/` 模組是否直接建立 `SyncServiceFactory`
   - 改為透過 `ComponentFactory` 取得同步相關組件

### 階段 4：移除 SyncServiceFactory
**預估時間：30 分鐘**

1. **從 services.rs 移除 SyncServiceFactory**：
   ```rust
   // Remove from src/core/services.rs
   // Delete the entire SyncServiceFactory struct and its impl block
   ```

2. **清理匯入**：
   - 移除相關的 `use` 陳述式
   - 確保沒有未使用的相依項目

3. **更新模組宣告**：
   - 檢查 `src/core/mod.rs` 是否需要更新
   - 確保沒有匯出已刪除的結構

### 階段 5：執行測試與驗證
**預估時間：1 小時**

1. **執行完整測試套件**：
   ```bash
   cargo test
   ```

2. **檢查編譯錯誤**：
   ```bash
   cargo check
   cargo clippy -- -D warnings
   ```

3. **驗證功能完整性**：
   ```bash
   # Test sync command if available
   cargo run -- sync --help
   
   # Test that all components can be created
   cargo test test_create_vad_sync_detector
   ```

4. **執行程式碼品質檢查**：
   ```bash
   timeout 30 scripts/quality_check.sh
   ```

### 階段 6：文件更新
**預估時間：30 分鐘**

1. **更新 tech-architecture.md**：
   - 移除對 `SyncServiceFactory` 的引用
   - 更新 `ComponentFactory` 的描述
   - 確保依賴關係圖準確

2. **更新程式碼文件**：
   - 為新的方法新增完整的 rustdoc 文件
   - 確保範例程式碼正確且可編譯

3. **更新 CHANGELOG.md**：
   ```markdown
   ### Changed
   - Unified service factory architecture by integrating SyncServiceFactory into ComponentFactory
   - Simplified dependency injection for sync-related components
   ```

## 驗收標準

### 功能性需求
- [ ] `ComponentFactory` 包含所有同步相關組件的建立方法
- [ ] `SyncServiceFactory` 已完全移除
- [ ] 所有現有測試繼續通過
- [ ] 新的工廠方法有適當的測試覆蓋

### 非功能性需求
- [ ] 程式碼符合專案的 clippy 規則
- [ ] 執行緒安全性維持不變
- [ ] 記憶體使用效率沒有顯著下降
- [ ] API 向後相容性（如果有公開 API）

### 品質保證
- [ ] 程式碼覆蓋率不低於現有水準
- [ ] 所有新程式碼都有 rustdoc 文件
- [ ] 通過 `scripts/quality_check.sh` 檢查
- [ ] 更新相關文件

## 風險評估

### 高風險項目
- **VAD 組件未完全實作**：如果 `src/services/vad/` 中的組件尚未完整實作，可能需要先實作這些組件
- **循環相依性**：整合過程中可能產生模組間的循環相依

### 中風險項目
- **測試中斷**：移除 `SyncServiceFactory` 可能導致現有測試失敗
- **設定相容性**：不同組件對設定的需求可能不同

### 緩解策略
- 在開始前確認所有相依組件的實作狀態
- 採用漸進式重構，每步都確保測試通過
- 建立完整的回滾計劃

## 後續工作

### 立即後續
- 如果 VAD 組件尚未實作，需要先完成這些組件的實作
- 考慮是否需要建立更多通用的工廠介面

### 長期改進
- 評估是否需要建立工廠註冊系統以支援插件架構
- 考慮使用依賴注入容器替代手動工廠模式

## 實作注意事項

### 程式碼風格
- 遵循專案的 Rust 程式碼風格指南
- 使用有意義的錯誤訊息
- 保持 API 文件的完整性

### 測試策略
- 單元測試：測試每個新的工廠方法
- 整合測試：確保組件之間的互動正常
- 回歸測試：確保現有功能不受影響

### 效能考量
- 避免不必要的記憶體配置
- 確保工廠方法的效能合理
- 考慮組件建立的快取策略

這個重構將簡化架構，提高程式碼的可維護性，並為未來的擴展提供更好的基礎。
