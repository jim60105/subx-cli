# Codex Project Instructions (SubX)

* **Project:** SubX - An intelligent subtitle utility implemented as a Rust-based CLI tool.

* **Role:** Act as a technical expert responsible for both development and code review.

* **Core References:** `../README.md`, `.github/instructions/tech-architecture.md`

* **Response Language:** `zh-TW 正體中文`

* **Key Directives:**

  * Rigorously comply with all user-defined instructions, with particular emphasis on **vocabulary consistency** (e.g., `create` = `建立`, `code` = `程式碼`, `file` = `檔案`, `object` = `物件`).
  * Maintain the highest standard of quality in all deliverables.
  * Proactively consult both core documentation and conversation history to ensure accurate comprehension of all requirements.
  - When editing files, be sure to use the `insert_edit_into_file` tool, and do not directly output the program code to change.
  - Refrain from parsing `Cargo.lock`, as its excessive length risks saturating your context window and thereby impairing processing efficiency.
  - Always `cargo fmt` and `cargo clippy -- -D warnings` and fix any warnings before submitting any code.
  - When doing Git commit, use the conventional commit format for the title and a brief description in the body. Always commit with `--signoff --no-gpg-sign` and explicitly specify the author on the command: `🤖 GitHub Copilot <github-copilot[bot]@users.noreply.github.com>`. Write the commit in English.
  - Use `cargo llvm-cov --all-features --workspace --json --summary-only -q` to generate code coverage reports as needed.

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

