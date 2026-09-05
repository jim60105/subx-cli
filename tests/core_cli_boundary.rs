//! Layering guard: the `subx-core` crate may never reference `subx-cli`.
//!
//! `subx-core` is the library half of the SubX two-crate split (the git
//! submodule mounted at `subx-core/`). The dependency between the crates
//! points downward only — `subx-cli` depends on `subx-core`, never the other
//! way — so any reference from core to the CLI is unfixable rather than
//! merely stale: there is no dependency edge for rustc or rustdoc to resolve
//! it through, and `broken_intra_doc_links = "deny"` turns such a rustdoc
//! link into a hard build failure.
//!
//! The guard therefore rejects, anywhere in a line of any `.rs` file under
//! `subx-core/src/` (code, comments, doc links and doctests alike):
//!
//! - `crate::cli` and `crate::commands` — the paths core code used before
//!   the crate split, when the CLI modules shared the crate root;
//! - `subx_cli` — the cross-crate spelling of the same references.
//!
//! Human-oriented output from core goes through the transport-agnostic
//! `subx_core::core::report::Reporter` seam instead.
//!
//! The mirror assertion checks the other direction of the test tree: after
//! the test-suite split, every test and bench under `subx-cli` drives either
//! the CLI surface (`subx_cli::cli`, `subx_cli::commands`, `subx_cli::App`)
//! or the core library *through* `subx_core` directly. No test may still
//! reach a library segment through `subx_cli`'s D11 back-compatibility
//! re-exports (its `config`, `core`, `error`, `services` and `Result`
//! segments, or any of the twelve `#[macro_export]` macros re-exported at
//! the crate root) — those re-exports exist only for the out-of-tree GUI and
//! must have no in-repository consumer. The forbidden tokens are assembled
//! from a prefix constant so this file never contains one as a literal.
//!
//! Paths resolve from `CARGO_MANIFEST_DIR`, never the working directory, so
//! the guard works no matter where the test runner is invoked from.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN: [&str; 3] = ["crate::cli", "crate::commands", "subx_cli"];

/// The `subx_cli::` prefix, split so this guard file does not itself contain
/// any forbidden token as a contiguous literal.
const CLI_PREFIX: &str = concat!("subx_", "cli::");

/// Library segments and crate-root macro re-exports that `subx-cli`'s tests
/// and benches must name as `subx_core::…`, never through the CLI crate's
/// back-compatibility re-exports.
const FORBIDDEN_SUFFIXES: [&str; 17] = [
    "config",
    "core",
    "error",
    "services",
    "Result",
    "test_with_config",
    "test_production_config_with_env",
    "test_production_config_with_openai_env",
    "create_production_config_service_with_env",
    "create_production_config_service_with_empty_env",
    "test_with_default_config",
    "test_with_ai_config_and_key",
    "test_with_ai_config",
    "test_with_sync_config",
    "test_with_parallel_config",
    "create_test_config_service",
    "create_default_test_config_service",
];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("readable dirent").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn subx_core_never_references_the_cli_crate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let core_src = root.join("subx-core").join("src");
    let mut offenders: Vec<String> = Vec::new();

    let mut files = Vec::new();
    collect_rs_files(&core_src, &mut files);
    assert!(
        !files.is_empty(),
        "guard walked no files under {} — the submodule is not initialised \
         (run `git submodule update --init --recursive`) or path resolution \
         is broken",
        core_src.display()
    );
    for file in files {
        let text = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", file.display()));
        for (lineno, line) in text.lines().enumerate() {
            if let Some(token) = FORBIDDEN.iter().find(|token| line.contains(**token)) {
                offenders.push(format!(
                    "{}:{}: contains `{token}`",
                    file.strip_prefix(&root).unwrap_or(&file).display(),
                    lineno + 1
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "subx-core/src/ must never reference the subx-cli crate — there is \
         no dependency edge in that direction, so code, doc links and \
         doctests naming crate::cli, crate::commands or subx_cli cannot \
         resolve (route human-oriented output through \
         subx_core::core::report::Reporter instead). \nOffending file:line:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn subx_cli_tests_never_reach_core_through_the_re_exports() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut offenders: Vec<String> = Vec::new();

    let mut files = Vec::new();
    for dir in [root.join("tests"), root.join("benches")] {
        if dir.is_dir() {
            collect_rs_files(&dir, &mut files);
        }
    }
    assert!(
        !files.is_empty(),
        "guard walked no files under subx-cli/tests or subx-cli/benches — \
         path resolution is broken"
    );
    for file in files {
        // This guard file necessarily names the forbidden tokens (in its own
        // docs and error strings); it is the one file exempt from its textual
        // scan.
        if file
            .file_name()
            .is_some_and(|n| n == "core_cli_boundary.rs")
        {
            continue;
        }
        let text = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", file.display()));
        for (lineno, line) in text.lines().enumerate() {
            let prefix_len = CLI_PREFIX.len();
            if let Some(at) = line.find(CLI_PREFIX) {
                let rest = &line[at + prefix_len..];
                // Contiguous form: `subx_cli::config`, `subx_cli::test_with_config`, …
                if let Some(suffix) = FORBIDDEN_SUFFIXES.iter().find(|s| rest.starts_with(**s)) {
                    offenders.push(format!(
                        "{}:{}: contains `{}{}`",
                        file.strip_prefix(&root).unwrap_or(&file).display(),
                        lineno + 1,
                        CLI_PREFIX,
                        suffix
                    ));
                    continue;
                }
                // Grouped form: `use subx_cli::{config, App};` — tokenize the
                // brace list so no forbidden segment hides behind a sibling.
                if let Some(list) = rest.strip_prefix('{') {
                    let list = &list[..list.find('}').unwrap_or(list.len())];
                    for seg in list.split(',') {
                        let seg = seg.trim().split(" as ").next().unwrap_or("").trim();
                        if FORBIDDEN_SUFFIXES.contains(&seg) {
                            offenders.push(format!(
                                "{}:{}: grouped import names forbidden segment `{seg}`",
                                file.strip_prefix(&root).unwrap_or(&file).display(),
                                lineno + 1,
                            ));
                        }
                    }
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "subx-cli's own tests and benches must name library internals as \
         subx_core::…, not through subx_cli's D11 back-compatibility \
         re-exports (kept only for the out-of-tree GUI; they must have no \
         in-repository consumer). \nOffending file:line:\n{}",
        offenders.join("\n")
    );
}
