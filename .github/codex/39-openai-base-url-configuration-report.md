---
title: "工作報告: Backlog #15 - OpenAI Base URL 設定功能"
date: "2025-06-08T15:31:23Z"
---

# Backlog #15 - OpenAI Base URL 設定功能 工作報告

**日期**: 2025-06-08T15:31:23Z  
**任務目標**: 擴展統一配置系統，支援自訂 OpenAI API Base URL，以相容 Azure OpenAI、私有部署及其他相容 API。

> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.

## 一、實作內容

### 1. 配置結構擴展
- 在 `PartialAIConfig` 中新增 `base_url` 欄位，並更新 merge/to_complete_config 邏輯
- [檔案變更：【F:src/config/partial.rs†L42-L52】【F:src/config/partial.rs†L91-L100】【F:src/config/partial.rs†L151-L156】]

### 2. 完整配置與 CLI 整合
- 在 `AIConfig` 結構與 `Config::default()` 中新增 `base_url` 欄位及預設值
- 更新 `get_value()` 支援 `ai.base_url`
- [檔案變更：【F:src/config.rs†L194-L203】【F:src/config.rs†L237-L242】【F:src/config.rs†L315-L319】]

### 3. 環境變數來源支援
- 在 `EnvSource::load()` 新增讀取 `OPENAI_BASE_URL`
- [檔案變更：【F:src/config/source.rs†L76-L82】]

### 4. 客戶端重構與驗證
- 將 `OpenAIClient::new()` 改為調用 `new_with_base_url()`，新增 `new_with_base_url()`、`from_config()` 與 `validate_base_url()`
- 移除對硬編碼端點的依賴，所有 API 請求使用可配置的 `base_url`
- [檔案變更：【F:src/services/ai/openai.rs†L140-L160】【F:src/services/ai/openai.rs†L172-L212】]

### 5. 配置驗證器擴展
- 在 `AIConfigValidator` 中新增 `base_url` 格式驗證，使用 `url` crate 解析、檢查 scheme 與 host
- [檔案變更：【F:src/config/validator.rs†L55-L63】【F:src/config/validator.rs†L70-L80】]

### 6. 配置合併優先權修正
- 調整 `ConfigManager::load()` 合併順序，確保高優先級來源覆蓋低優先級來源
- [檔案變更：【F:src/config/manager.rs†L98-L103】【F:src/config/manager.rs†L11-L12】]

## 二、測試與驗證

```bash
cargo fmt -- --check && cargo clippy -- -D warnings && cargo test
```

所有單元與整合測試皆已通過，涵蓋：
- PartialConfig 合併與轉換測試
- 環境變數覆蓋流程整合測試
- URL 驗證器有效/無效案例
- OpenAIClient 不同 base_url 情境測試

## 三、後續事項

- 更新 README.md 文件，新增 `OPENAI_BASE_URL` 與 `ai.base_url` 範例說明
- 進行 `cargo llvm-cov` 覆蓋率報告生成與審查

---
**檔案異動**:
- `src/config/partial.rs`
- `src/config.rs`
- `src/config/source.rs`
- `src/services/ai/openai.rs`
- `src/config/validator.rs`
- `src/config/manager.rs`
