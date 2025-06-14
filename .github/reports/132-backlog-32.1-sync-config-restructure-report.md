---
title: "工作報告: Backlog #32.1 - 新同步配置結構設計"
date: "2025-06-14T15:16:39Z"
---

# Backlog #32.1 - 新同步配置結構設計 工作報告

**日期**：2025-06-14T15:16:39Z  
**任務**：重構 SubX 同步配置結構，移除舊設定並新增 Whisper/VAD 子結構  
**類型**：Backlog  
**狀態**：已完成

> [!TIP]  
> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.

## 一、任務概述
本次任務旨在重構 SubX 的音訊同步配置結構，移除基於包絡頻譜的舊欄位，建立支援 OpenAI Whisper API 及本地 VAD 的嵌套設定，並實作對應的預設值與驗證邏輯，同時保留舊欄位做過渡相容。

## 二、實作內容

### 2.1 重構 SyncConfig 結構
- 新增 `default_method`、`analysis_window_seconds`、`max_offset_seconds` 欄位
- 建立 `WhisperConfig` 及 `VadConfig` 子結構
- 保留舊欄位並以 `#[deprecated]` 標記以維持相容性
- 實作【F:src/config/mod.rs†L192-L290】

```rust
pub struct SyncConfig {
    pub default_method: String,
    pub analysis_window_seconds: u32,
    pub max_offset_seconds: f32,
    pub whisper: WhisperConfig,
    pub vad: VadConfig,
    #[deprecated]
    #[serde(skip)]
    pub correlation_threshold: f32,
    // ...其他過渡欄位
}
```

### 2.2 實作 Default 與 validate
- 為三結構實作 `Default`，並於 `SyncConfig` 加入過渡欄位的 `#[allow(deprecated)]`
- 實作 `SyncConfig::validate`、`WhisperConfig::validate`、`VadConfig::validate` 驗證邊界
- 由 `SyncValidator` 委派調用
- 實作【F:src/config/validator.rs†L1-L158】【F:src/config/validator.rs†L218-L260】

### 2.3 更新 TestConfigBuilder API
- 移除舊的 `with_sync_threshold`、`with_max_offset` 等方法
- 新增 `with_sync_method`、`with_analysis_window` 及 Whisper/VAD 相關設定方法
- 實作【F:src/config/builder.rs†L111-L215】

### 2.4 更新使用者文件
- 修改 `docs/configuration-guide.md` 中 `[sync]` 範本，新增 `[sync.whisper]`、`[sync.vad]` 區段
- 增加環境變數範例：`SUBX_SYNC_DEFAULT_METHOD`、`SUBX_SYNC_ANALYSIS_WINDOW_SECONDS`…等
- 實作【F:docs/configuration-guide.md†L73-L83】【F:docs/configuration-guide.md†L157-L167】【F:docs/configuration-guide.md†L213-L236】

### 2.5 撰寫整合測試與相容性測試
- `tests/config_new_sync_structure_tests.rs`：驗證 TOML 序列化/反序列化
- `tests/config_migration_tests.rs`：舊式配置應驗證失敗
- 實作【F:tests/config_new_sync_structure_tests.rs†whole】【F:tests/config_migration_tests.rs†whole】

## 三、技術細節

### 3.1 架構變更
- 同步配置改為「頂層 + Whisper/VAD 子結構」模型，拆分原本單一扁平設定

### 3.2 API 變更
- `TestConfigBuilder` API 調整，不再有 `with_sync_threshold` 視為內部細節，改採更語意化的 `with_sync_method` 等

### 3.3 配置變更
- 新增環境變數 `SUBX_SYNC_DEFAULT_METHOD`、`SUBX_SYNC_ANALYSIS_WINDOW_SECONDS`、… 等供覆蓋使用

## 四、測試與驗證

### 4.1 程式碼品質檢查
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

### 4.2 功能測試
- 自動化測試全面通過，包含預設值、驗證邏輯、序列化、相容性測試

## 五、影響評估

### 5.1 向後相容性
- 保留舊欄位並標註 deprecated，確保升級後仍能編譯直至移除

### 5.2 使用者體驗
- 新增 Whisper/VAD 配置彈性，讓使用者可選擇不同同步方法

## 六、問題與解決方案

無重大阻礙，若遇到 deprecated 欄位警告，已統一於 `#![allow(deprecated)]` 關閉

## 七、後續事項

### 7.1 待完成項目
- Backlog 32.2：Whisper API 整合
- Backlog 32.3：本地 VAD 實作
- Backlog 32.4：同步引擎與 CLI 重構

## 八、檔案異動清單
| 檔案路徑 | 異動類型 | 描述 |
|----------|----------|------|
| src/config/mod.rs | 修改 | 重構 SyncConfig 結構，新增子結構與 deprecated 欄位 |
| src/config/validator.rs | 修改 | 實作預設值與驗證函式 |
| src/config/builder.rs | 修改 | 更新 TestConfigBuilder API |
| docs/configuration-guide.md | 修改 | 更新 sync 範本與環境變數說明 |
| tests/config_new_sync_structure_tests.rs | 新增 | 整合測試：新配置結構解析 |
| tests/config_migration_tests.rs | 新增 | 相容性測試：舊配置應失敗 |
