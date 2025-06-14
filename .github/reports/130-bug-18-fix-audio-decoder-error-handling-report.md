---
title: "Job Report: Bug Fix #18 - Fix audio decoder error handling"
date: "2025-06-14T14:17:53Z"
---

# Bug Fix #18 - Fix audio decoder error handling 工作報告

**日期**：2025-06-14T14:17:53Z  
**任務**：修復 `transcode_to_wav` 方法中音訊解碼器錯誤處理邏輯，提升對 Symphonia API 的容錯能力  
**類型**：Bug Fix  
**狀態**：已完成

> [!TIP]
> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.

## 一、任務概述
原 `transcode_to_wav` 方法中，所有 `decoder.decode()` 的錯誤均直接中斷流程，未區分可恢復與不可恢復錯誤，違反 Symphonia API 規範，導致單一損壞封包就使整個轉碼過程失敗。

## 二、實作內容

### 2.1 引入日誌記錄宏
- 使用 `log::warn` 記錄可恢復錯誤與警告。
- 導入位置：【F:src/services/audio/transcoder.rs†L5】

### 2.2 新增 `TranscodeStats` 統計結構
- 收集封包總數、成功解碼數、不同錯誤跳過計數等統計資訊。
- 實作 `new()` 與 `success_rate()` 方法。
- 相關程式碼：【F:src/services/audio/transcoder.rs†L24-L58】

### 2.3 重構 `transcode_to_wav` 實作，新增可配置方法
- 新增 `transcode_to_wav_with_config`，支援傳入最小解碼成功率參數，依錯誤類型跳過封包或中斷。
- 更新原 `transcode_to_wav` 為向後相容呼叫，並對低成功率發出警告。
- 相關程式碼：【F:src/services/audio/transcoder.rs†L158-L284】【F:src/services/audio/transcoder.rs†L286-L297】

## 三、技術細節

### 3.1 錯誤分類與處理
- 對 `DecodeError`、`IoError`、`ResetRequired` 屬可恢復或需重置情況進行跳過與警告。
- 其他錯誤視為不可恢復並中斷。

### 3.2 向後相容性
- 原 `transcode_to_wav` 方法改為呼叫新版本，並保留舊有 API 簽名。

## 四、測試與驗證

### 4.1 單元測試
- 驗證 `TranscodeStats.success_rate()` 正確度。
- 測試程式碼位於：【F:src/services/audio/transcoder.rs†L105-L117】

## 五、影響評估

### 5.1 向後相容性
- 保留原有 `transcode_to_wav` 簽名，呼叫新實作。不影響外部呼用。

### 5.2 使用者體驗
- 增加對部分損壞音訊檔容錯能力，避免因單一錯誤包導致整體失敗。

## 六、問題與解決方案

### 6.1 遇到的問題
- 原本未區分錯誤類型，導致可恢復錯誤也會中斷。

**解決方案**：根據 Symphonia API 規範分類處理錯誤，並新增成功率檢查與跳過機制。

## 七、後續事項

### 7.1 建議的後續改進
- 增加錯誤日誌結構化輸出，並納入更多統計與度量。
