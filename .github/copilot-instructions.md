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
    - Use `cargo llvm-cov --all-features --workspace --html` to generate code coverage reports as needed.

# Project Planning Structure

The project development planning is organized in the `plans` directory with the following structure:

## Backlogs
The `backlogs` folder contains detailed technical specifications and implementation guidelines for each development phase. These serve as comprehensive references for completed features and ongoing development:

* **[Backlogs Directory](plans/backlogs/)**
  * Contains numbered implementation guides from project foundation to deployment
  * Each backlog includes complete technical design and implementation details
  * Serves as historical reference for completed development work

## Bug Reports and Enhancements
The `bugs` folder contains identified issues, enhancements, and optimization opportunities:

* **[Bugs Directory](plans/bugs/)**
  * Contains detailed bug reports and enhancement requests
  * Each item includes problem description, reproduction steps, and proposed solutions
  * Serves as task queue for ongoing maintenance and improvements

