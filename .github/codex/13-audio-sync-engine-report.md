# 13. 音訊同步引擎 (Audio Synchronization Engine) 實作報告

## 完成項目
- 在 `src/services/audio/mod.rs` 實作 `AudioAnalyzer`，採用 Symphonia 解碼音訊並提取能量包絡與對話段偵測。
- 在 `src/core/sync/engine.rs` 實作 `SyncEngine`，包含交叉相關 (cross-correlation) 分析、自動與手動偏移校正。
- 新增 `src/commands/sync_command.rs`，實作 CLI `sync` 命令，支援手動偏移、單檔與批量處理模式。
- 更新 `src/core/sync/mod.rs`、`src/commands/mod.rs` 與 `src/cli/mod.rs`，整合 `sync` 子命令執行流程。
- 更新 `src/error.rs` 增加 Symphonia 錯誤轉換，統一為 `SubXError::audio_processing`。
- 更新 `Cargo.toml` 新增 `symphonia = { version = "0.5", features = ["all"] }` 相依。

## 測試驗證
- 已通過原有字幕格式解析相關 15 個單元測試。
- 執行 `cargo fmt`、`cargo clippy -- -D warnings` 均無格式或警告錯誤。
- 執行 `cargo test`，所有測試通過。
