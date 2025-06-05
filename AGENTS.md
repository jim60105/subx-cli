# Codex Project Instructions (SubX)

* **Project:** SubX – An intelligent subtitle utility implemented as a Rust-based CLI tool.

* **Role:** Act as a technical expert responsible for both development and code review.

* **Core References:** `../README.md`, `.github/instructions/tech-architecture.md`

* **Response Language:** `zh-TW 正體中文`

* **Key Directives:**

  * Rigorously comply with all user-defined instructions, with particular emphasis on **vocabulary consistency** (e.g., `create` = `建立`, `code` = `程式碼`, `file` = `檔案`, `object` = `物件`).
  * Maintain the highest standard of quality in all deliverables.
  * Proactively consult both core documentation and conversation history to ensure accurate comprehension of all requirements.
  * Always `cargo fmt` and `cargo clippy -- -D warnings` and fix any warnings before submitting any code.

---

# Detailed Guidelines for Product Backlogs

Each product backlog entry encapsulates the complete technical design and implementation blueprint for a discrete module:

1. **[Project Foundation](.github/instructions/01-project-foundation.md)**

   * Initialization of the Rust project, directory layout, and error-handling architecture

2. **[CLI Interface Framework](.github/instructions/02-cli-interface.md)**

   * Command architecture, parameter parsing, and user interface specification

3. **[Configuration Management System](.github/instructions/03-config-management.md)**

   * TOML-based configuration, environment variable support, and validation mechanisms

4. **[Subtitle Format Engine](.github/instructions/04-subtitle-format-engine.md)**

   * Parsers for SRT/ASS/VTT/SUB formats and a unified data model

5. **[AI Service Integration](.github/instructions/05-ai-service-integration.md)**

   * Integration with the OpenAI API, prompt engineering strategies, and retry logic

6. **[File Matching Engine](.github/instructions/06-file-matching-engine.md)**

   * File discovery mechanisms, AI-assisted matching, and preview mode implementation

7. **[Format Conversion System](.github/instructions/07-format-conversion-system.md)**

   * Cross-format subtitle transformation, style preservation, and batch processing

8. **[Audio Synchronization Engine](.github/instructions/08-audio-sync-engine.md)**

   * FFmpeg integration, cross-correlation analysis, and automatic alignment routines

9. **[Command Integration Testing](.github/instructions/09-command-integration.md)**

   * End-to-end validation, fault tolerance, and user workflow simulations

10. **[Deployment and Release](.github/instructions/10-deployment-release.md)**

    * CI/CD pipeline orchestration, cross-platform compilation, and automated release management

# Work Report Protocol

Development progress for this project is systematically tracked within the `.github/codex` directory. Before commencing any new work, review prior reports to stay aligned with ongoing development. Treat all past reports as immutable references—do not edit or revise them under any circumstance. Upon the completion of each task, you are required to generate a new comprehensive work report. Refer to the naming conventions of existing files to determine an appropriate filename. 

Your report must include a detailed account of the work performed, encompassing all relevant code modifications and corresponding test outcomes.

Following the completion of each task, it is **imperative** that you execute a Git commit with sign-off. Write the commit message in English. Use conventional commit format, ensuring it is concise yet descriptive.
