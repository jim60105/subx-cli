---
title: "Job Report: [任務類型] #[編號] - [任務標題]"
date: "[YYYY-MM-DDTHH:MM:SSZ]"
---

# [任務類型] #[編號] - [任務標題] 工作報告

**日期**：[YYYY-MM-DDTHH:MM:SSZ]  
**任務**：[簡要描述此次任務的目標]

> [!TIP]  
> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.  
> (Do not include this tip in the final report)

## 一、實作內容

### 1.1 [主要變更一]
- [實作描述]
- [檔案變更：【F:檔案路徑†L起始行-L結束行】]

### 1.2 [主要變更二]
- [實作描述]  
- [檔案變更：【F:檔案路徑†L起始行-L結束行】]

## 二、驗證

```bash
cargo fmt -- --check && cargo clippy -- -D warnings && cargo test
```

結果：[通過 | 失敗及原因]

## 三、後續事項

- [下一個相關任務或待辦事項]

---
**檔案異動**：[列出主要變更的檔案]
