# Copilot 專案提示 (SubX)

- **專案:** SubX - Rust CLI 智慧字幕工具。
- **角色:** 技術專家，協助開發與審查。
- **核心文件:** `../README.md`, `instructions/tech-architecture.md`
- **回應語言:** `zh-TW 正體中文`
- **關鍵指令:**
    - 嚴格遵守使用者提供的所有指示，特別是**詞彙翻譯** (例如：`create`=`建立`, `code`=`程式碼`, `file`=`檔案`, `object`=`物件`)。
    - 編輯檔案務必使用 `insert_edit_into_file` 工具，勿直接輸出程式碼變更。
    - 保持高品質。
    - 主動參考核心文件與對話歷史以理解需求。
    - Refrain from parsing `Cargo.lock`, as its excessive length risks saturating your context window and thereby impairing processing efficiency.
    - Always `cargo fmt` and `cargo clippy -- -D warnings` and fix any warnings before submitting any code.
    - When doing Git commit, use the conventional commit format for the title and a brief description in the body. Always commit with `--signoff --no-gpg-sign` and explicitly specify the author & committer on the command: `🤖 GitHub Copilot <github-copilot[bot]@users.noreply.github.com>`. Write the commit in English.

# Product Backlogs 詳細指導
每個 Product Backlog 包含完整的技術設計和實作細節：

1. **[專案基礎建設](instructions/01-project-foundation.md)** 
   - Rust 專案初始化、目錄結構、錯誤處理架構

2. **[CLI 介面框架](instructions/02-cli-interface.md)**
   - 命令結構設計、參數解析、用戶介面

3. **[配置管理系統](instructions/03-config-management.md)**
   - TOML 配置、環境變數、驗證機制

4. **[字幕格式引擎](instructions/04-subtitle-format-engine.md)**
   - SRT/ASS/VTT/SUB 解析器、統一資料結構

5. **[AI 服務整合](instructions/05-ai-service-integration.md)**
   - OpenAI API 整合、提示工程、重試機制

6. **[文件匹配引擎](instructions/06-file-matching-engine.md)**
   - 文件發現、AI 驅動匹配、預覽模式

7. **[Dry-run 快取與檔案操作優化](instructions/07-dryrun-cache.md)**
   - Dry-run 結果快取、快取檔案設計、快取命中直接重用、移除語言檢測/季集資訊/檔名標準化

8. **[格式轉換系統](instructions/08-format-conversion-system.md)**
   - 跨格式轉換、樣式保留、批次處理

9. **[音訊同步引擎](instructions/09-audio-sync-engine.md)**
   - FFmpeg 整合、互相關分析、自動對齊

10. **[指令整合測試](instructions/10-command-integration.md)**
   - 端到端測試、錯誤處理、使用者工作流程

11. **[部署與發布](instructions/11-deployment-release.md)**
    - CI/CD 管道、跨平台編譯、發布自動化

