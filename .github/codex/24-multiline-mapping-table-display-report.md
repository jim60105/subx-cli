---
title: "Job Report: Enhancement - 對映表格多行顯示"
date: "2025-06-13"
---

# Enhancement: 對映表格多行顯示

**日期**：2025-06-13  
**任務**：為 `match` 命令的對映表格顯示改為多行模式，避免檔名截斷或自動換行

## 一、更新顯示邏輯
- 修改 `src/cli/ui.rs` 中 `display_match_results`，將影片、字幕與新檔名分成兩行顯示：第一行為「影片檔案 N」、「字幕檔案 N」或「新檔名 N」，第二行為完整檔案路徑或名稱【F:src/cli/ui.rs†L58-L77】

## 二、調整截斷函式保留測試
- 移除先前在表格中使用的 `truncate_path` 呼叫，並於該函式加上 `#[allow(dead_code)]` 以維持其在測試中的可用性【F:src/cli/ui.rs†L88-L95】

## 三、保留並驗證單元測試
- 原有 `test_match_table_display` 測試持續驗證表格是否含有原始檔名；
- 保留 `test_path_truncation` 以驗證 `truncate_path` 函式行為【F:src/cli/ui.rs†L101-L120】

## 四、驗證
```bash
cargo fmt && cargo clippy -- -D warnings && cargo test
```

## 五、結論
為避免長檔名在表格中因過長而截斷或自動換行，改以多行方式呈現檔案對映標籤與完整檔案名稱，增強 CLI 表格的可讀性與辨識度。
