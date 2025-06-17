# 40 - 核心服務容器重新命名與職責釐清：解決 services.rs 命名衝突

## 概述

本計劃旨在解決 `src/core/services.rs` 檔案命名與頂層 `src/services/` 目錄的衝突問題，並釐清 `ServiceContainer` 的職責範圍。目前的命名容易造成混淆，`ServiceContainer` 的職責也與其名稱暗示的功器模式不完全一致。重構後將建立更清晰的架構層次，避免命名衝突，並確保每個組件都有明確的職責。

## 問題描述

### 當前狀況
- `src/core/services.rs` 包含 `ServiceContainer` 結構
- `src/services/` 是包含外部服務整合的目錄
- 檔案名稱與目錄名稱衝突，容易造成 import 混淆
- `ServiceContainer` 實際上更像是一個依賴注入容器而非服務提供者

### 命名衝突示例
```rust
// 容易產生混淆的 import 路徑
use subx_cli::core::services::ServiceContainer;    // 這是容器
use subx_cli::services::ai::OpenAIClient;           // 這是實際服務
```

### 架構問題
- `ServiceContainer` 名稱暗示它是服務提供者，但實際上是依賴注入容器
- 檔案名稱 `services.rs` 與服務層目錄 `services/` 衝突
- 開發者需要記住兩個不同的 "services" 概念

## 技術需求

### 主要目標
1. 將 `src/core/services.rs` 重新命名為更具描述性的名稱
2. 重新命名 `ServiceContainer` 以更準確反映其職責
3. 更新所有相關的 import 和引用
4. 確保模組文件和架構描述準確
5. 保持 API 的向後相容性（透過重新匯出）
6. 清理不必要的程式碼和註解

### 技術規格
- 遵循 Rust 命名慣例
- 保持依賴注入模式
- 確保執行緒安全性
- 維持現有的 API 簽章

## 實作計劃

### 階段 1：分析當前使用狀況
**預估時間：45 分鐘**

1. **查找所有 services.rs 的引用**：
   ```bash
   # Find all references to the services module
   grep -r "services::" src/ --include="*.rs"
   grep -r "use.*services" src/ --include="*.rs"
   grep -r "ServiceContainer" src/ --include="*.rs"
   ```

2. **檢查模組宣告**：
   ```bash
   # Check how services is declared in mod.rs
   grep -r "mod services" src/ --include="*.rs"
   grep -r "pub mod services" src/ --include="*.rs"
   ```

3. **分析 ServiceContainer 的實際用途**：
   ```bash
   # Find all usages of ServiceContainer
   grep -rn "ServiceContainer" src/ --include="*.rs"
   ```

4. **建立重構清單**：
   - 列出所有需要更新的檔案
   - 識別公開 API 的使用點
   - 確認是否有外部依賴

### 階段 2：選擇新的命名方案
**預估時間：30 分鐘**

1. **檔案重新命名方案**：
   ```
   現有：src/core/services.rs
   選項 A：src/core/container.rs
   選項 B：src/core/di_container.rs  
   選項 C：src/core/service_container.rs
   推薦：src/core/container.rs (簡潔且明確)
   ```

2. **結構重新命名方案**：
   ```rust
   現有：ServiceContainer
   選項 A：DIContainer (Dependency Injection Container)
   選項 B：AppContainer
   選項 C：DependencyContainer
   推薦：DIContainer (業界標準術語)
   ```

3. **確認命名一致性**：
   - 檔案名稱：`container.rs`
   - 主要結構：`DIContainer`
   - 模組名稱：`container`

### 階段 3：執行檔案重新命名
**預估時間：30 分鐘**

1. **重新命名檔案**：
   ```bash
   # Rename the file
   mv src/core/services.rs src/core/container.rs
   ```

2. **更新 core/mod.rs**：
   ```rust
   // Update src/core/mod.rs
   // Replace:
   pub mod services;
   // With:
   pub mod container;
   
   // Add re-export for backward compatibility
   pub use container::{DIContainer, SyncServiceFactory};
   // Deprecated re-export
   #[deprecated(since = "0.3.0", note = "Use DIContainer instead")]
   pub use container::DIContainer as ServiceContainer;
   ```

### 階段 4：重構 container.rs 內容
**預估時間：1.5 小時**

1. **更新結構定義和文件**：
   ```rust
   // Update src/core/container.rs
   //! Dependency injection container for service management.
   //!
   //! This module provides a centralized dependency injection container that manages
   //! the lifecycle of services and components, enabling clean dependency
   //! injection throughout the application.
   //!
   //! # Design Principles
   //!
   //! - **Dependency Injection**: Components receive dependencies explicitly
   //! - **Configuration Isolation**: Services are decoupled from global configuration
   //! - **Single Responsibility**: Container manages dependencies, not business logic
   //! - **Test Friendliness**: Easy to mock and test individual components
   //!
   //! # Examples
   //!
   //! ```rust
   //! use subx_cli::core::DIContainer;
   //! use subx_cli::config::ProductionConfigService;
   //! use std::sync::Arc;
   //!
   //! # async fn example() -> subx_cli::Result<()> {
   //! let config_service = Arc::new(ProductionConfigService::new()?);
   //! let container = DIContainer::new(config_service)?;
   //!
   //! // Access services through container
   //! let config_service = container.config_service();
   //! let factory = container.component_factory();
   //! # Ok(())
   //! # }
   //! ```

   use crate::{Result, config::ConfigService, core::ComponentFactory};
   use std::sync::Arc;

   /// Dependency injection container for service and component management.
   ///
   /// The DI container holds references to core services and provides
   /// a centralized way to access them throughout the application. It manages
   /// the lifecycle of services and ensures proper dependency injection.
   pub struct DIContainer {
       config_service: Arc<dyn ConfigService>,
       component_factory: ComponentFactory,
   }

   impl DIContainer {
       /// Create a new DI container with the given configuration service.
       ///
       /// # Arguments
       ///
       /// * `config_service` - Configuration service implementation
       ///
       /// # Errors
       ///
       /// Returns an error if component factory creation fails.
       pub fn new(config_service: Arc<dyn ConfigService>) -> Result<Self> {
           let component_factory = ComponentFactory::new(config_service.as_ref())?;

           Ok(Self {
               config_service,
               component_factory,
           })
       }

       /// Get a reference to the configuration service.
       ///
       /// Returns a reference to the configuration service managed by this container.
       pub fn config_service(&self) -> &Arc<dyn ConfigService> {
           &self.config_service
       }

       /// Get a reference to the component factory.
       ///
       /// Returns a reference to the component factory that can create
       /// configured instances of core components.
       pub fn component_factory(&self) -> &ComponentFactory {
           &self.component_factory
       }

       /// Reload all services and components.
       ///
       /// This method triggers a reload of the configuration service and
       /// recreates the component factory with the updated configuration.
       /// This is useful for dynamic configuration updates.
       ///
       /// # Errors
       ///
       /// Returns an error if configuration reloading or factory recreation fails.
       pub fn reload(&mut self) -> Result<()> {
           // Reload configuration service
           self.config_service.reload()?;

           // Recreate component factory with updated configuration
           self.component_factory = ComponentFactory::new(self.config_service.as_ref())?;

           Ok(())
       }

       /// Create a new DI container for testing with custom configuration.
       ///
       /// This method is useful for testing scenarios where you need to provide
       /// specific configuration values.
       ///
       /// # Arguments
       ///
       /// * `config_service` - Test configuration service
       ///
       /// # Errors
       ///
       /// Returns an error if container creation fails.
       #[cfg(test)]
       pub fn new_for_testing(config_service: Arc<dyn ConfigService>) -> Result<Self> {
           Self::new(config_service)
       }
   }

   // Keep SyncServiceFactory here for now (to be moved in backlog 37)
   /// Sync detection service factory for creating different sync detectors.
   ///
   /// This factory creates various sync detection services with the provided
   /// configuration service for dependency injection.
   /// 
   /// # Deprecation Notice
   /// 
   /// This factory will be integrated into ComponentFactory in a future update.
   /// See backlog 37 for the unified service factory architecture.
   pub struct SyncServiceFactory {
       config_service: Box<dyn ConfigService>,
   }

   impl SyncServiceFactory {
       /// Create a new sync service factory.
       ///
       /// # Arguments
       ///
       /// * `config_service` - Configuration service for dependency injection
       pub fn new(config_service: Box<dyn ConfigService>) -> Self {
           Self { config_service }
       }
   }

   // Backward compatibility type alias
   #[deprecated(since = "0.3.0", note = "Use DIContainer instead of ServiceContainer")]
   pub type ServiceContainer = DIContainer;
   ```

2. **更新測試**：
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use crate::config::test_service::TestConfigService;

       #[test]
       fn test_di_container_creation() {
           let config_service = Arc::new(TestConfigService::default());
           let container = DIContainer::new(config_service);
           assert!(container.is_ok());
       }

       #[test]
       fn test_di_container_access() {
           let config_service = Arc::new(TestConfigService::default());
           let container = DIContainer::new(config_service.clone()).unwrap();

           // Test that we can access services
           let _retrieved_config_service = container.config_service();
           let factory = container.component_factory();
           assert!(factory.config().ai.provider == "openai");
       }

       #[test]
       fn test_di_container_reload() {
           let config_service = Arc::new(TestConfigService::default());
           let mut container = DIContainer::new(config_service).unwrap();

           // Test reload operation
           let result = container.reload();
           assert!(result.is_ok());
       }

       #[test]
       fn test_new_for_testing() {
           let config_service = Arc::new(TestConfigService::default());
           let container = DIContainer::new_for_testing(config_service);
           assert!(container.is_ok());
       }

       // Backward compatibility test
       #[test]
       #[allow(deprecated)]
       fn test_service_container_alias() {
           let config_service = Arc::new(TestConfigService::default());
           let container: ServiceContainer = DIContainer::new(config_service).unwrap();
           
           // Should work exactly like DIContainer
           let _factory = container.component_factory();
       }
   }
   ```

### 階段 5：更新所有引用
**預估時間：2 小時**

1. **更新命令模組中的引用**：
   ```bash
   # Find and update all references
   find src/commands/ -name "*.rs" -exec grep -l "ServiceContainer\|services::" {} \;
   ```

   ```rust
   // Update imports in command files
   // Replace:
   use crate::core::services::ServiceContainer;
   // With:
   use crate::core::container::DIContainer;
   
   // Update usage:
   // Replace:
   let container = ServiceContainer::new(config_service)?;
   // With:
   let container = DIContainer::new(config_service)?;
   ```

2. **檢查 lib.rs 中的重新匯出**：
   ```rust
   // Update src/lib.rs if needed
   // Add re-export for public API
   pub use core::container::DIContainer;
   
   // Deprecated re-export for backward compatibility
   #[deprecated(since = "0.3.0", note = "Use DIContainer instead")]
   pub use core::container::DIContainer as ServiceContainer;
   ```

3. **更新測試檔案**：
   ```bash
   # Find and update test files
   find tests/ -name "*.rs" -exec grep -l "ServiceContainer\|services::" {} \;
   ```

### 階段 6：更新文件和註解
**預估時間：1 小時**

1. **更新 tech-architecture.md**：
   ```markdown
   ## Core Engine (`src/core/`)
   
   ### Dependency Injection Container (`src/core/container.rs`)
   
   - **DIContainer**: 管理服務生命週期和相依性注入的容器
   - **用途**: 提供統一的依賴注入入口點，管理 ConfigService 和 ComponentFactory 的實例
   - **重要方法**: `new()`, `reload()`, `config_service()`, `component_factory()`
   
   #### 向後相容性
   
   `ServiceContainer` 已棄用，請使用 `DIContainer`。舊的名稱將在 v1.0 中移除。
   ```

2. **更新模組文件**：
   ```rust
   // Update src/core/mod.rs documentation
   //! Core processing engines and dependency injection infrastructure.
   //!
   //! This module contains the core business logic engines and the dependency
   //! injection container that manages their lifecycle.
   //!
   //! # Key Components
   //!
   //! - [`container`] - Dependency injection container for service management
   //! - [`factory`] - Component factory for creating configured instances
   //! - [`matcher`] - AI-powered file matching engine
   //! - [`sync`] - Audio-subtitle synchronization engine
   //! - [`formats`] - Subtitle format processing
   //! - [`parallel`] - Parallel processing coordination
   ```

3. **更新範例程式碼**：
   - 確保所有文件中的範例都使用新的命名
   - 更新 README.md 中的程式碼範例（如果有）

### 階段 7：處理遷移和棄用警告
**預估時間：45 分鐘**

1. **新增遷移指南**：
   ```rust
   // Add to src/core/container.rs
   //! # Migration Guide
   //!
   //! ## From ServiceContainer to DIContainer
   //!
   //! The `ServiceContainer` has been renamed to `DIContainer` to better reflect
   //! its role as a dependency injection container.
   //!
   //! ### Before
   //! ```rust
   //! use subx_cli::core::services::ServiceContainer;
   //! 
   //! let container = ServiceContainer::new(config_service)?;
   //! ```
   //!
   //! ### After
   //! ```rust
   //! use subx_cli::core::container::DIContainer;
   //! 
   //! let container = DIContainer::new(config_service)?;
   //! ```
   //!
   //! ### Compatibility
   //!
   //! The old `ServiceContainer` is still available as a deprecated type alias:
   //! ```rust
   //! #[allow(deprecated)]
   //! use subx_cli::core::container::ServiceContainer; // Works but deprecated
   //! ```
   ```

2. **建立 clippy 允許規則**：
   ```rust
   // Temporarily allow deprecated usage in backward compatibility code
   #[allow(deprecated)]
   pub type ServiceContainer = DIContainer;
   ```

### 階段 8：測試與驗證
**預估時間：1.5 小時**

1. **執行完整測試套件**：
   ```bash
   cargo test
   cargo test --release
   ```

2. **檢查棄用警告**：
   ```bash
   # Should show deprecation warnings for old usage
   cargo check 2>&1 | grep -i deprecat
   ```

3. **測試向後相容性**：
   ```rust
   // Create a specific test for backward compatibility
   #[test]
   #[allow(deprecated)]
   fn test_backward_compatibility() {
       use crate::core::services::ServiceContainer;
       
       let config_service = Arc::new(TestConfigService::default());
       let container = ServiceContainer::new(config_service).unwrap();
       
       // Should work exactly like DIContainer
       let _factory = container.component_factory();
   }
   ```

4. **執行品質檢查**：
   ```bash
   cargo clippy -- -D warnings
   cargo fmt --check
   timeout 30 scripts/quality_check.sh
   ```

5. **驗證文件生成**：
   ```bash
   cargo doc --no-deps
   ```

### 階段 9：清理和最終化
**預估時間：30 分鐘**

1. **檢查未使用的 import**：
   ```bash
   cargo check 2>&1 | grep "unused import"
   ```

2. **確認所有測試通過**：
   ```bash
   cargo test -- --nocapture
   ```

3. **更新 CHANGELOG.md**：
   ```markdown
   ### Changed
   - Renamed `ServiceContainer` to `DIContainer` for better clarity
   - Moved `src/core/services.rs` to `src/core/container.rs` to avoid naming conflicts
   - Improved dependency injection container documentation and examples
   
   ### Deprecated
   - `ServiceContainer` type alias (use `DIContainer` instead)
   - `src/core/services` module path (use `src/core::container` instead)
   
   ### Migration
   - Replace `ServiceContainer` with `DIContainer` in your code
   - Update imports from `core::services` to `core::container`
   - See module documentation for detailed migration guide
   ```

## 驗收標準

### 功能性需求
- [ ] `src/core/services.rs` 成功重新命名為 `src/core/container.rs`
- [ ] `ServiceContainer` 重新命名為 `DIContainer`
- [ ] 所有現有功能保持不變
- [ ] 向後相容性透過棄用別名維持

### 非功能性需求
- [ ] 清除命名衝突和混淆
- [ ] 改善程式碼可讀性和維護性
- [ ] 保持執行緒安全性
- [ ] 文件清晰且準確

### 品質保證
- [ ] 所有測試繼續通過
- [ ] 棄用警告適當顯示
- [ ] 程式碼符合專案風格指南
- [ ] 文件生成無錯誤

## 風險評估

### 高風險項目
- **遺漏引用更新**：可能遺漏某些檔案中的引用，導致編譯錯誤
- **外部依賴中斷**：如果有外部程式碼依賴舊名稱，可能會中斷

### 中風險項目
- **測試中斷**：重新命名可能導致某些測試失敗
- **文件不一致**：可能遺漏某些文件的更新

### 緩解策略
- 使用全專案搜尋確保所有引用都已更新
- 保持向後相容性別名直到下一個主版本
- 詳細的程式碼審查確保沒有遺漏

## 後續工作

### 立即後續
- 在下一個主版本（v1.0）中移除棄用的別名
- 考慮是否需要為其他模組建立類似的命名標準

### 長期改進
- 評估是否需要更進階的依賴注入功能
- 考慮實作服務註冊和發現機制
- 評估是否需要生命週期管理功能

## 實作注意事項

### 向後相容性
- 保持所有公開 API 的簽章不變
- 使用適當的棄用屬性和訊息
- 提供清晰的遷移路徑

### 文件品質
- 確保所有新名稱都有完整的文件
- 提供實用的範例程式碼
- 包含遷移指南

### 測試策略
- 測試新舊兩種 API 的功能
- 確保棄用警告正確顯示
- 驗證向後相容性

這次重構將解決命名衝突問題，提高程式碼的清晰度，並為未來的架構改進奠定基礎。透過保持向後相容性，現有的程式碼可以平滑遷移到新的命名方案。
