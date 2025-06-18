# Bug #24: Sync 指令參數彈性與直覺性問題

## 問題描述

目前 `subx-cli sync` 指令在處理參數時，與 README 文件描述及使用者直覺不符：

- 執行 `subx-cli sync -b test` 會出現 `unexpected argument 'test' found`，無法像文件所述直接指定目錄或檔案。
- 執行 `subx-cli sync -v "影片.mkv" -b`，即使同目錄下有對應字幕，卻出現 `no matching subtitle`。
- 只有同時指定 `-v` 與 `-s` 參數時才可正常執行，與 README 所述「可直接指定目錄或單一檔案」不符。

## 預期行為

- 使用者可依 README 範例，直接以 `subx-cli sync <目錄或檔案>`、`subx-cli sync -b <目錄>`、`subx-cli sync -v <影片> -b` 等方式執行，且能自動配對字幕。
- 批次模式下，應能自動偵測目錄下所有影片與字幕檔案，並正確配對。
- 單一檔案模式下，若僅給定影片或字幕，應能自動推論另一檔案（如同目錄下同名副檔名）。

## 問題根因分析

- 目前 `SyncArgs` 與 `get_sync_mode`、`validate` 等參數組合驗證邏輯過於嚴格，導致 CLI 解析時無法接受單一檔案或目錄作為參數。
- 批次模式下，`input_paths` 與 `batch` 參數未能正確處理 positional argument，且未自動推論影片/字幕配對。
- 單一檔案模式下，未實作自動推論同名字幕或影片的邏輯。
- CLI 層級的參數 required_unless_present_xxx 設定過於嚴格，導致部分組合被 clap 拒絕。

## 受影響檔案

- `src/cli/sync_args.rs`：參數定義與驗證邏輯
- `src/commands/sync_command.rs`：執行邏輯與自動配對
- `tests/cli/sync_cli_integration_tests.rs`、`tests/sync_command_comprehensive_tests.rs`：整合測試
- `README.md`、`README.zh-TW.md`：文件說明

## 修正方案

1. **CLI 參數彈性調整**
   - 調整 `SyncArgs` 參數 required_unless_present_xxx 設定，允許 positional argument 作為 input_paths 或自動推論。
   - 支援 `subx-cli sync <目錄或檔案>` 直覺用法，將 positional argument 自動填入 input_paths。
   - 批次模式下，允許 `-b <目錄>`、`-b -i <目錄>`、`-b`（搭配 positional argument）等多種組合。

2. **自動配對邏輯強化**
   - 單一檔案模式：若僅給定影片或字幕，嘗試於同目錄下自動尋找同名副檔名檔案。
   - 批次模式：遍歷 input_paths 目錄，對每個影片自動尋找同名字幕。

3. **驗證與錯誤訊息優化**
   - 調整 `validate` 與 `get_sync_mode`，允許更彈性的參數組合。
   - 若自動推論失敗，給予明確且友善的錯誤訊息與建議。

4. **測試強化**
   - 新增 CLI integration 測試，覆蓋所有參數組合（單一檔案、目錄、batch、positional argument、-v/-s/-i 等）。
   - 強化自動配對失敗、參數缺漏等異常情境測試。
   - 測試需使用 `TestConfigService` 並完全隔離。

5. **文件同步更新**
   - 更新 `README.md`、`README.zh-TW.md` 之 sync 指令範例，明確說明各種用法。
   - 增加 troubleshooting 區段，說明自動配對失敗時的排查建議。

## 實作檢查清單

- [ ] 調整 `SyncArgs` 參數定義與 clap 設定，允許 positional argument 彈性
- [ ] 強化 `get_sync_mode` 與自動配對邏輯，支援單一檔案與目錄推論
- [ ] 優化錯誤訊息，明確提示缺少檔案或配對失敗原因
- [ ] 新增/強化 CLI integration 測試，覆蓋所有參數組合
- [ ] 文件同步更新，確保範例與實作一致
- [ ] 通過 `cargo fmt`、`cargo clippy -- -D warnings`、`timeout 240 scripts/quality_check.sh`、`cargo nextest run`、`timeout 240 scripts/check_coverage.sh -T`

## 驗收標準

1. ✅ 使用者可依 README 範例，彈性指定檔案或目錄進行同步，無需強制指定 -v/-s
2. ✅ 批次模式下可自動配對所有影片與字幕，並正確處理缺漏情境
3. ✅ 單一檔案模式下可自動推論同名字幕或影片
4. ✅ 所有 CLI integration 測試通過，並覆蓋所有參數組合
5. ✅ 文件說明與實際行為完全一致
6. ✅ 程式碼品質與測試覆蓋率皆達標

## 範例

```shell
# 單一目錄批次同步
subx-cli sync -b test

# 單一影片自動配對字幕
subx-cli sync "影片.mkv"

# 單一字幕自動配對影片
subx-cli sync "字幕.ass"

# 指定影片與字幕
subx-cli sync -v "影片.mkv" -s "字幕.ass"

# 多目錄批次同步
subx-cli sync -b -i dir1 -i dir2
```

## 相關 Issues 與 Dependencies

- 需同步檢查 #17、#23 相關 sync 參數與自動配對行為
- 需參考 `docs/testing-guidelines.md` 測試原則

---

> 若需進一步範例程式碼片段，請參考 `tests/cli/sync_cli_integration_tests.rs` 及 `tests/sync_command_comprehensive_tests.rs` 之測試案例。
