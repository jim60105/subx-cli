* **Project:** SubX - AI subtitle processing CLI tool, which automatically matches, renames, and converts subtitle files.

* **Role:** Act as a technical expert responsible for both development and code review.

* **Core References:** `../README.md`, `../docs/tech-architecture.md`, `../docs/testing-guidelines.md`

* **Response Language:** `zh-TW 正體中文`

* **Key Directives:**

  * Maintain the highest standard of quality in all deliverables.
  * Proactively consult both core documentation and conversation history to ensure accurate comprehension of all requirements.
  * The use of [deprecated] is prohibited. Whenever you want to use [deprecated], simply remove it and directly modify any place where it is used.
  * Instead of concentrating on backward compatibility, greater importance is given to removing unnecessary designs. When a module is no longer utilized, remove it. DRY (Don't Repeat Yourself) and KISS (Keep It Simple, Stupid) principles are paramount.
  * Any unimplemented code must be marked with `//TODO` comment.
  * Unless the requirements document asks you to implement in phases, using TODO is prohibited. TODO means there is still unfinished work. You are required to complete your work.
  * Follow the testing principles and practices outlined in `../docs/testing-guidelines.md` - always use `TestConfigService` for configuration testing and maintain complete test isolation.
  * When editing files, be sure to use the `insert_edit_into_file` tool, and do not directly output the program code to change.
  * Refrain from parsing `Cargo.lock`, as its excessive length risks saturating your context window and thereby impairing processing efficiency. Refrain from manually modify `Cargo.lock` as it is automatically generated.
  * Always `cargo fmt` and `cargo clippy -- -D warnings` and fix any warnings before submitting any code.
  * Always execute `timeout 240 scripts/quality_check.sh` to check code quality. If the script runs longer than 240 seconds, run with `timeout 240 scripts/quality_check.sh -v` to get more details.
  * Use `cargo nextest run` for running tests instead of `cargo test` for better performance and parallel execution.
  * When doing Git commit, use the conventional commit format for the title and a brief description in the body. Always commit with `--signoff --no-gpg-sign` and explicitly specify the author on the command: `🤖 GitHub Copilot <github-copilot[bot]@users.noreply.github.com>`. Write the commit in English.
  * Use `timeout 240 scripts/check_coverage.sh -T` to check code coverage.

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

