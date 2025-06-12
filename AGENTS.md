# Codex Project Instructions (SubX)

* **Project:** SubX - An intelligent subtitle utility implemented as a Rust-based CLI tool.

* **Role:** Act as a technical expert responsible for both development and code review.

* **Core References:** `README.md`, `docs/tech-architecture.md`, `docs/testing-guidelines.md`

* **Response Language:** `zh-TW 正體中文`

* **Key Directives:**

  * Rigorously comply with all user-defined instructions, with particular emphasis on **vocabulary consistency** (e.g., `create` = `建立`, `code` = `程式碼`, `file` = `檔案`, `object` = `物件`).
  * Maintain the highest standard of quality in all deliverables.
  * Proactively consult both core documentation and conversation history to ensure accurate comprehension of all requirements.
  * Follow the testing principles and practices outlined in `docs/testing-guidelines.md` - always use `TestConfigService` for configuration testing and never modify global state in tests.
  * Refrain from parsing `Cargo.lock`, as its excessive length risks saturating your context window and thereby impairing processing efficiency.
  * Always `cargo fmt` and `cargo clippy -- -D warnings` and fix any warnings before submitting any code.
  * Always execute `timeout 30 scripts/check_docs.sh` to check documentation quality. If the script runs longer than 20 seconds, run with `timeout 30 scripts/check_docs.sh -v` to get more details.
  * Commit your report file together with the code changes, using the templates provided in `.github/codex/`.
  * Git commit after completing your work, using the conventional commit format for the title and a brief description in the body. Always commit with `--signoff` and `--no-gpg-sign`. Write the commit in English.
  * Use `scripts/check_coverage.sh -T` to check code coverage.

---

# Project Planning Structure

The project development planning is organized in the `.github/plans` directory with the following structure:

## Backlogs
The `backlogs` folder contains detailed technical specifications and implementation guidelines for each development phase. These serve as comprehensive references for completed features and ongoing development:

* **[Backlogs Directory](.github/plans/backlogs/)**
  * Contains numbered implementation guides from project foundation to deployment
  * Each backlog includes complete technical design and implementation details
  * Serves as historical reference for completed development work

## Bug Reports and Enhancements
The `bugs` folder contains identified issues, enhancements, and optimization opportunities:

* **[Bugs Directory](.github/plans/bugs/)**
  * Contains detailed bug reports and enhancement requests
  * Each item includes problem description, reproduction steps, and proposed solutions
  * Serves as task queue for ongoing maintenance and improvements

# Work Report Protocol

Development progress for this project is systematically tracked within the `.github/codex` directory. Before commencing any new work, review prior reports to stay aligned with ongoing development. Treat all past reports as immutable references—do not edit or revise them under any circumstance. Upon the completion of each task, you are required to generate a new comprehensive work report. Refer to the naming conventions of existing files to determine an appropriate filename. 

Your report must include a detailed account of the work performed, encompassing all relevant code modifications and corresponding test outcomes.
