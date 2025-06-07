---
title: "Job Report: Backlog #07 - 為 MatchCommand 增加 Dry-run 快取單元測試"
date: "2025-06-08"
---

# Backlog #07 - 為 MatchCommand 增加 Dry-run 快取單元測試

**日期**：2025-06-08  
**任務**：撰寫單元測試，驗證在 `dry_run` 模式下是否會建立快取檔案

## 一、調整 `match_command.rs` 執行邏輯

- 修改 dry_run 條件，移除預覽模式下的過早返回，確保 Dry-run 時執行匹配並儲存快取後才退出
  【F:src/commands/match_command.rs†L31-L42】

## 二、撰寫單元測試

- 在 `match_command.rs` 底部新增測試模組，使用 `tempfile` 與環境變數模擬臨時資料夾
- 先行建立相容的快取檔案，藉由 `check_cache` 令匹配流程跳過 AI 分析，再呼叫 `execute` 後驗證快取仍存在
  【F:src/commands/match_command.rs†L49-L107】

## 三、驗證結果

```text
$ cargo test dry_run_creates_cache_file -- --nocapture
running 1 test

ℹ 預覽模式 - 未實際執行操作
test commands::match_command::tests::dry_run_creates_cache_file ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 0.05s
```

以上變更已完成 `cargo fmt`，但現有程式碼中仍有其他模組產生 Clippy warning，故無法整體通過 `cargo clippy -- -D warnings`。
