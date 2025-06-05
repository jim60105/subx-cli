# Product Backlog #07: Dry-run 快取與檔案操作優化

## 領域範圍
Dry-run 結果快取、快取檔案設計、快取命中直接重用、移除過度複雜的分析功能

## 完成項目

### 1. Dry-run 結果快取系統
- [ ] 實作 Dry-run 結果快取機制
- [ ] 快取以檔案型式儲存於設定檔同一目錄下
- [ ] 快取命中時，若檔案結構完全一致則直接重用結果
- [ ] 快取失效條件：目錄內檔案有異動（新增/刪除/修改時間/大小）
- [ ] 快取內容包含：影片/字幕檔案清單、AI 匹配結果、操作建議
- [ ] 快取檔案格式設計與序列化/反序列化
- [ ] 快取過期和清理機制

### 2. 移除語言檢測功能
**語言檢測功能的詳細說明：**
原先設計的語言檢測功能是為了自動識別字幕內容的語言類型，以提供給 AI 額外的語言提示。此功能包含：
- 基於正則表達式的字符模式檢測（中文字符範圍、日文平假名片假名、韓文字符、ASCII 英文等）
- 統計不同語言字符的出現頻率來判斷主要語言
- 語言檢測信心度計算
- 多語言混合內容的處理邏輯

**移除任務：**
- [ ] 移除語言檢測相關的正則表達式模式：
- [ ] 從 `ContentSample` 結構中移除 `language_hint: Option<String>` 欄位
- [ ] 移除語言檢測相關的測試程式碼和測試資料
- [ ] 更新 AI 提示模板，不再在 JSON 請求中包含語言提示欄位
- [ ] 更新文件說明，移除語言檢測功能的描述

### 3. 移除季集資訊提取功能
**季集資訊提取功能的詳細說明：**
原先設計的季集資訊提取功能是為了從檔名中自動識別影集的季數和集數資訊，支援多種常見的命名格式。此功能包含：
- 季集格式識別：`S01E01`、`s01e01`、`1x01`、`Season 1 Episode 1`、`第1季第1集`
- 數字範圍驗證（季數1-99，集數1-999）
- 特殊格式處理：`S01E01-02`（多集合併）、`S01E01E02`（連續集數）
- 零填充格式處理：`S01E001`、`S1E1`
- 與其他檔名元素的分離邏輯

**移除任務：**
- [ ] 從 `FilenameAnalyzer` 中移除季集正則表達式模式：
  - `[Ss](\d{1,2})[Ee](\d{1,3})` - 標準 SxxExx 格式
  - `(\d{1,2})x(\d{1,3})` - 數字x數字格式
  - `Season\s*(\d{1,2}).*Episode\s*(\d{1,3})` - 英文全稱格式
  - `第(\d{1,2})季.*第(\d{1,3})集` - 中文格式
- [ ] 從 `ParsedFilename` 結構中移除欄位：
  - `season: Option<u32>`
  - `episode: Option<u32>`
- [ ] 刪除相關測試程式碼和測試資料檔案
- [ ] 簡化檔名分析流程，僅保留基本檔名清理

### 4. 移除檔名標準化處理
**檔名標準化處理功能的詳細說明：**
原先設計的檔名標準化功能是為了統一不同的檔名格式，消除分隔符差異，提高 AI 匹配的準確率。此功能包含：
- 特殊字符替換：將 `.`、`_`、`-` 等分隔符統一轉換為空格
- 多重空格清理：將連續空格合併為單一空格
- 前後空白修剪：移除檔名開頭和結尾的空白字符
- 大小寫標準化：統一轉換為小寫或保持原始大小寫
- 括號內容處理：移除或保留括號內的額外資訊
- 特殊符號過濾：移除不必要的標點符號和特殊字符

**移除任務：**
- [ ] 刪除 `extract_title` 方法及其所有邏輯：
  - 檔名分隔符標準化邏輯
  - 空格清理和修剪邏輯
  - 括號內容移除邏輯
- [ ] 更新匹配邏輯改為完全基於原始檔名字符串：
  - 檔名比對直接使用 `file.name` 而非處理後的標題
  - AI 請求中直接傳遞原始檔名，不經過任何預處理
- [ ] 刪除標準化相關的測試程式碼與配置選項條目，因為 src 目錄下已無相關內容

### 5. 簡化匹配引擎架構
- [ ] 更新 `MatchEngine` 以移除複雜的檔名分析依賴：
  - 移除對 `FilenameAnalyzer` 的複雜呼叫
  - 簡化檔案掃描邏輯，僅保留基本檔案類型識別
- [ ] 簡化 AI 請求結構，減少不必要的 metadata：
  - 移除季集資訊、年份、品質等結構化資料
  - 移除語言提示欄位
  - 請求中僅包含檔名列表和基本檔案資訊
- [ ] 更新匹配邏輯以專注於檔名字符串相似性：
  - 依賴 AI 的自然語言理解能力進行檔名匹配
  - 移除本地的複雜檔名解析和比對邏輯
- [ ] 優化效能，減少不必要的計算開銷：
  - 移除正則表達式編譯和執行
  - 減少字符串處理和轉換操作
  - 簡化檔案資訊提取流程

## 技術設計

### Dry-run 快取檔案設計
**快取檔案路徑：**
- Linux/macOS: `~/.config/subx/match_cache.json`
- Windows: `%APPDATA%\subx\match_cache.json`

**快取內容格式（JSON）：**
```json
{
  "cache_version": "1.0",
  "directory": "/path/to/media/folder",
  "file_snapshot": [
    {
      "name": "S01E01.mkv",
      "size": 123456789,
      "mtime": 1710000000,
      "file_type": "video"
    },
    {
      "name": "subtitle1.ass",
      "size": 12345,
      "mtime": 1710000001,
      "file_type": "subtitle"
    }
  ],
  "match_operations": [
    {
      "video_file": "S01E01.mkv",
      "subtitle_file": "subtitle1.ass",
      "new_subtitle_name": "S01E01.ass",
      "confidence": 0.98,
      "reasoning": ["檔名模式匹配", "內容相關性高"]
    }
  ],
  "created_at": 1710000002,
  "ai_model_used": "gpt-4o-mini",
  "config_hash": "abc123def456"
}
```

**快取命中條件：**
- 目錄路徑相同
- 檔案清單、大小、修改時間完全一致
- AI 模型版本相同
- 配置檔案雜湊值相同

### 快取流程設計

**1. Dry-run 執行前快取檢查：**
```rust
// 偽程式碼示例
async fn check_cache(&self, directory: &Path) -> Option<Vec<MatchOperation>> {
    // 1. 計算目錄檔案快照
    let current_snapshot = self.calculate_file_snapshot(directory)?;
    
    // 2. 讀取快取檔案
    let cache_data = self.load_cache_file().ok()?;
    
    // 3. 比對快取條件
    if cache_data.directory == directory.to_string_lossy() &&
       cache_data.file_snapshot == current_snapshot &&
       cache_data.ai_model_used == self.config.ai.model &&
       cache_data.config_hash == self.calculate_config_hash() {
        return Some(cache_data.match_operations);
    }
    
    None
}
```

**2. Dry-run 結果快取儲存：**
```rust
async fn save_cache(&self, directory: &Path, operations: &[MatchOperation]) -> Result<()> {
    let cache_data = CacheData {
        cache_version: "1.0".to_string(),
        directory: directory.to_string_lossy().to_string(),
        file_snapshot: self.calculate_file_snapshot(directory)?,
        match_operations: operations.to_vec(),
        created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        ai_model_used: self.config.ai.model.clone(),
        config_hash: self.calculate_config_hash(),
    };
    
    let cache_path = self.get_cache_file_path()?;
    let cache_content = serde_json::to_string_pretty(&cache_data)?;
    std::fs::write(cache_path, cache_content)?;
    
    Ok(())
}
```

**3. 正式執行時快取重用：**
- 執行正式 match 命令時，若檔案結構未變動且快取有效
- 直接重用 dry-run 的匹配結果，跳過 AI 分析步驟
- 僅執行檔案重命名操作，大幅節省時間和 API 成本

### 移除功能的具體影響

**語言檢測功能移除的技術影響：**
- AI 請求的 `ContentSample` 結構簡化，移除 `language_hint` 欄位
- Prompt 模板不再包含語言提示相關的 JSON 欄位
- 減少字幕檔案內容的預處理步驟
- 降低正則表達式編譯和執行的 CPU 開銷

**季集資訊提取功能移除的技術影響：**
- `ParsedFilename` 結構大幅簡化，僅保留基本檔名資訊
- AI 請求中不再包含結構化的季集 metadata
- 移除複雜的正則表達式模式匹配邏輯
- 檔名分析效能顯著提升

**檔名標準化處理移除的技術影響：**
- AI 請求直接使用原始檔名，不經過任何預處理
- 移除字符串轉換和清理的計算開銷
- 簡化匹配邏輯，完全依賴 AI 的自然語言理解
- 減少記憶體分配和字符串操作

### 快取管理機制

**快取清理策略：**
- 定期清理超過 30 天的過期快取
- 當快取檔案超過 10MB 時自動清理最舊的條目
- 提供手動清理命令：`subx cache clear`

**快取失效機制：**
- 檔案異動檢測（新增、刪除、修改時間、大小變更）
- AI 模型版本變更時自動失效
- 配置檔案變更時重新計算雜湊值

**錯誤處理：**
- 快取檔案損壞時自動重建
- 快取讀取失敗時回退到正常 AI 分析流程
- 快取寫入失敗時不影響主要功能執行

## 驗收標準
1. **快取功能正常運作：**
   - Dry-run 結果正確儲存到快取檔案
   - 快取命中時不重複呼叫 AI API
   - 檔案變動時快取正確失效

2. **功能移除完整：**
   - 所有語言檢測相關程式碼完全移除
   - 所有季集資訊提取相關程式碼完全移除
   - 所有檔名標準化相關程式碼完全移除
   - 不再有相關的測試程式碼和測試資料

3. **效能改善明顯：**
   - 快取命中時 match 命令執行時間 < 1 秒
   - 記憶體使用量相比原設計減少 > 30%
   - CPU 使用率降低，特別是正則表達式處理部分

4. **相容性保持：**
   - AI 匹配準確度不因功能簡化而顯著下降
   - 原有的檔案操作功能保持正常
   - 錯誤處理機制完善且用戶友善

## 估計工時
2-3 天

## 相依性
- 依賴 Backlog #06 (檔案匹配引擎)

## 風險評估
- **低風險：** 快取機制為常見優化模式
- **注意事項：** 
  - 快取失效判斷需嚴謹，避免誤用過期結果
  - 功能移除需確保不影響核心匹配邏輯
  - 需要充分測試 AI 匹配在簡化輸入下的表現
