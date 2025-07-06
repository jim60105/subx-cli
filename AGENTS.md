# SubX - AI subtitle processing CLI tool, which automatically matches, renames, and converts subtitle files.

* **Role:** Act as a technical expert responsible for development.

* **Response Language:** `zh-TW 正體中文`

# Key Directives:

* Maintain the highest standard of quality in all deliverables.
* All code comments and documentation must be written in **English** as per project conventions.
* Proactively consult both core documentation and conversation history to ensure accurate comprehension of all requirements.
* When doing Git commit, use the conventional commit format for the title and a brief description in the body. Always commit with `--signoff` and explicitly specify the author on the command: `Codex-CLI <bot@ChenJ.im>`. Write the commit in English.

---

# Project DevOps

This project uses GitHub for DevOps management.

Use `gh` CLI commands to perform DevOps tasks.

***Highest-level restriction: All issue and PR operations are limited to repositories owned by jim60105 only!***

* **GitHub repo**: https://github.com/jim60105/subx-cli

* **Backlog & Bugs**: All backlogs and bugs must be managed on GitHub Issues.

  * Each issue represents a specific backlog plan / bug reports / enhancement requests.
  * Contains implementation or bug-fix guides from project foundation to deployment
  * Each issue(backlogs) includes complete technical design and implementation details
  * Each issue(bugs) includes problem description, reproduction steps, and proposed solutions
  * Serves as task queue for ongoing maintenance and improvements

## DevOps Flow

### Planning Stage

**If we are at planning stage you shouldn't start to implement anything!**
**Planning Stage is to create a detailed development plan and create issue on GitHub using `gh issue create`**

1. **Issue Creation**: Use `gh issue create --title "Issue Title" --body "Issue Description"` to create a new issue for each backlog item or bug report. Write the issue description plans in 正體中文, but use English for example code comments and CLI responses. The plan should be very detailed (try your best!). Please write that enables anyone to complete the work successfully.
2. **Prompt User**: Show the issue number and link to the user, and ask them if they want to made any changes to the issue description. If they do, you can edit the issue description using `gh issue edit [number] --body "New Description"`.

### Implementation Stage

**Only start to implement stage when user prompt you to do so!**
**Implementation Stage is to implement the plan step by step, following the instructions provided in the issue and submit a work report PR at last**

1. **Check Current Situation**: Run `git status` to check the current status of the Git repository to ensure you are aware of any uncommitted changes or issues before proceeding with any operations. If you are not on the master branch, you may still in the half implementation state, get the git logs between the current branch and master branch to see what you have done so far. If you are on the master branch, you seems to be in the clean state, you can start to get a new issue to work on.
2. **Get Issue Lists**: Use `gh issue list` to get the list of issues to see all backlogs and bugs. Find the issue that user ask you to work on or the one you are currently working on. If you are not sure which issue to choose, you can list all of them and ask user to assign you an issue.
3. **Get Issue Details**: Use `gh issue view [number]` to get the details of the issue to understand the requirements and implementation plan. Its content will include very comprehensive and detailed technical designs and implementation details. Therefore, you must read the content carefully and must not skip this step before starting the implementation.
4. **Get Issue Comments**: Use `gh issue view [number] --comments` to read the comments in the issue to understand the context and any additional requirements or discussions that have taken place. Please read it to determine whether this issue has been completed, whether further implementation is needed, or if there are still problems that need to be fixed. This step must not be skipped before starting implementation.
5. **Get Pull Requests**: Use `gh pr list`, `gh pr view [number]`, and `gh pr view [number] --comments` to list the existing pull requests and details to check if there are any related to the issue you are working on. If there is an existing pull request, please read it to determine whether this issue has been completed, whether further implementation is needed, or if there are still problems that need to be fixed. This step must not be skipped before starting implementation.
6. **Git Checkout**: Run `git checkout -b [branch-name]` to checkout the issue branch to start working on the code changes. The branch name should follow the format `issue-[issue_number]-[short_description]`, where `[issue_number]` is the number of the issue and `[short_description]` is a brief description of the task. Skip this step if you are already on the correct branch.
7. **Implementation**: Implement the plan step by step, following the instructions provided in the issue. Each step should be executed in sequence, ensuring that all requirements are met and documented appropriately.
8. **Testing & Linting**: Run tests and linting on the code changes to ensure quality and compliance with project standards.
9. **Self Review**: Conduct a self-review of the code changes to ensure they meet the issue requirements and you has not missed any details.
10. **Git Commit & Git Push**: Run `git commit` using the conventional commit format for the title and a brief description in the body. Always commit with `--signoff` and explicitly specify the author on the command: `Codex-CLI <bot@ChenJ.im>`. Write the commit in English. Link the issue number in the commit message body. Run `git push` to push the changes to the remote repository.
11. **Create Pull Request**: Use `gh pr list` and `gh pr create` commands. ALWAYS SUBMIT PR TO `origin`, NEVER SUBMIT PR TO `upstream`. Create a pull request if there isn't already has one related to your issue using `gh pr create --title "PR Title" --body "PR Description"`. Create a comprehensive work report and use it as pull request details, detailing the work performed, code changes, and test results for the project. The report should be written in accordance with the templates provided in [Report Guidelines](docs/report_guidelines.md) and [REPORT_TEMPLATE](docs/REPORT_TEMPLATE.md). Follow the template exactly. Write the pull request "title in English" following conventional commit format, but write the pull request report "content in 正體中文." Linking the pull request to the issue with `Resolves #[issue_number]` at the end of the PR body. ALWAYS SUBMIT PR TO `origin`, NEVER SUBMIT PR TO `upstream`. ALWAYS SUBMIT PR TO `origin`, NEVER SUBMIT PR TO `upstream`. ALWAYS SUBMIT PR TO `origin`, NEVER SUBMIT PR to `upstream`.

***Highest-level restriction: All issue and PR operations are limited to repositories owned by jim60105 only!***
***Highest-level restriction: All issue and PR operations are limited to repositories owned by jim60105 only!***
***Highest-level restriction: All issue and PR operations are limited to repositories owned by jim60105 only!***

---

## Rust Code Guidelines

* All code comments must be written in **English**.
* Documentation and user interface text are authored in **English**.
* The use of [deprecated] is prohibited. Whenever you want to use [deprecated], simply remove it and directly modify any place where it is used.
* Instead of concentrating on backward compatibility, greater importance is given to removing unnecessary designs. When a module is no longer utilized, remove it. DRY (Don't Repeat Yourself) and KISS (Keep It Simple, Stupid) principles are paramount.
* Any unimplemented code must be marked with `//TODO` comment.
* Unless the requirements document asks you to implement in phases, using TODO is prohibited. TODO means there is still unfinished work. You are required to complete your work.
* Follow the testing principles and practices outlined in [Test Guidelines](docs/testing-guidelines.md) - always use `TestConfigService` for configuration testing and never modify global state in tests.
* Refrain from parsing `Cargo.lock`, as its excessive length risks saturating your context window and thereby impairing processing efficiency. Refrain from manually modify `Cargo.lock` as it is automatically generated.
* Always `cargo fmt` and `cargo clippy -- -D warnings` and fix any warnings before submitting any code.
* Always execute `timeout 240 scripts/quality_check.sh` to check code quality. If the script runs longer than 240 seconds, run with `timeout 240 scripts/quality_check.sh -v` to get more details.
* Use `cargo nextest run 2>&1 | tee test.log || true` for running tests instead of `cargo test` for better performance and parallel execution. Always run `cargo nextest run` with ` 2>&1 | tee test.log || true` since there's technical issue with `cargo nextest run` in the current project setup.
* Use `timeout 240 scripts/check_coverage.sh -T` to check code coverage.

## Project Overview

SubX-CLI is an advanced AI-powered command-line tool designed for automated subtitle processing. It provides intelligent matching, renaming, and conversion of subtitle files for video collections. Key features include AI-based subtitle-video matching, batch processing, format conversion (SRT, ASS, VTT, SUB), timeline correction using VAD (Voice Activity Detection), and robust file organization capabilities. The tool is highly configurable, supports dry-run previews, and offers comprehensive cache management for efficient repeated operations.

## File Organization

```
.github/           # GitHub configuration, workflows, and project instructions
assets/            # Project logo, media samples, and test assets
benches/           # Benchmark tests for performance evaluation

scripts/           # Shell scripts for installation, quality checks, and CI tasks
  ├── check_coverage.sh           # Code coverage report script
  ├── install.sh                  # Project dependency installation script
  ├── quality_check.sh            # Lint, format, and quality check script
  ├── test_parallel_stability.sh  # Parallel test stability check script
  └── test_unified_paths.sh       # Unified path handling test script

docs/              # Documentation, guidelines, and technical architecture
  ├── REPORT_TEMPLATE.md         # PR/issue report template
  ├── config-usage-analysis.md   # Configuration usage analysis
  ├── configuration-guide.md     # Configuration usage guide
  ├── report-guidelines.md       # Work report writing guidelines
  ├── rustdoc-guidelines.md      # Rust documentation guidelines
  ├── tech-architecture.md       # Technical architecture overview
  └── testing-guidelines.md      # Testing principles and practices

src/               # Main Rust source code
  ├── cli/         # CLI argument parsing and user interface modules
  ├── commands/    # Command implementations (match, convert, sync, etc.)
  ├── config/      # Configuration management and validation
  ├── core/        # Core logic: file management, matching, formats, parallelism, sync
  ├── services/    # AI, audio, and VAD service modules
  ├── error.rs     # Error handling definitions
  ├── lib.rs       # Library entry point
  └── main.rs      # CLI application entry point
tests/             # Integration and unit tests, organized by feature
target/            # Build output directory (generated by Cargo and never read it/modify it)
CHANGELOG.md       # Project changelog
Cargo.toml         # Rust package manifest
README.md          # Project overview and usage instructions
README.zh-TW.md    # Traditional Chinese project overview
```

---

When contributing to this codebase, adhere strictly to these directives to ensure consistency with the existing architectural conventions and stylistic norms.
