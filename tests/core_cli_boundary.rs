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

/// Normalise a path for comparison without requiring it to exist (Cargo
/// resolves `#[path]` targets relative to the declaring file's directory —
/// here always the test-tree root). Drops `.` components and collapses
/// `..` against the preceding normal component; the paths compared never
/// traverse symlinks, so lexical folding is sufficient.
fn normalise(path: &Path) -> PathBuf {
    let mut out = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out.iter().collect()
}

/// Collect `.rs` files under `dir` recursively, skipping nested module
/// directories (any directory with its own `mod.rs` is reached through
/// `mod <dir>;` chains, not shims) and never following symlinks.
fn collect_orphan_candidates(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()));
    for entry in entries {
        let entry = entry.expect("readable dirent");
        let path = entry.path();
        let meta = entry
            .file_type()
            .unwrap_or_else(|e| panic!("failed to stat {}: {e}", path.display()));
        if meta.is_symlink() {
            continue;
        }
        if meta.is_dir() {
            if path.join("mod.rs").is_file() {
                continue;
            }
            collect_orphan_candidates(&path, out);
        } else if meta.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Extract the targets of every genuine `#[path = "…"] mod <name>;` shim in a
/// harness file (empty when it declares none). Line-oriented and
/// deliberately strict: the `#[path = "<target>"]` must sit on its own
/// trimmed line, the next non-blank line must be `mod <ident>;`, and any
/// `cfg`/`cfg_attr` stacked on the same declaration is rejected outright —
/// a conditionally compiled shim's target is not reliably compiled and must
/// not be counted as covering the file. Doc/line comments can never satisfy
/// the grammar because their `///`/`//`/`/*`/`*` prefixes make the trimmed
/// line differ.
fn shim_targets(text: &str) -> Vec<Result<String, String>> {
    let mut found = Vec::new();
    let mut lines = text.lines().map(str::trim);
    while let Some(line) = lines.next() {
        let Some(rest) = line.strip_prefix("#[path = \"") else {
            continue;
        };
        let Some(target) = rest.strip_suffix("\"]") else {
            continue;
        };
        // Gather the attribute stack directly above the #[path] line.
        let mut stack: Vec<&str> = Vec::new();
        {
            let mut seen = text.lines().map(str::trim);
            let mut prev: Vec<&str> = Vec::new();
            for l in seen.by_ref() {
                if l == line {
                    break;
                }
                prev.push(l);
            }
            for l in prev.iter().rev() {
                if l.starts_with("#[") {
                    stack.push(l);
                } else {
                    break;
                }
            }
        }
        if stack.iter().any(|a| a.starts_with("#[cfg")) {
            found.push(Err(format!(
                "declares a cfg-bearing #[path] shim (`{line}` under {}) — \
                 conditionally compiled shims cannot be verified as always-on \
                 coverage; give the target an unconditional shim or remove the cfg",
                stack.len()
            )));
            continue;
        }
        // The next non-blank line must be the mod declaration.
        let Some(next) = lines.by_ref().find(|l| !l.is_empty()) else {
            break;
        };
        let Some(mod_part) = next.strip_prefix("mod ") else {
            continue;
        };
        let Some(name) = mod_part.strip_suffix(';') else {
            continue;
        };
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        found.push(Ok(target.to_string()));
    }
    found
}

/// Cargo auto-discovers only `tests/*.rs`. Every `.rs` file under a
/// subdirectory of either crate's test tree is therefore compiled only if a
/// top-level harness declares `#[path = "<subdir>/<file>.rs"] mod <name>;`
/// for it — exactly once, because a second shim would run the same tests
/// twice. Module directories (containing their own `mod.rs`, like
/// `tests/common/`) are reached by plain `mod <dir>;` declarations from the
/// crate root and are exempt; data directories (`fixtures/`) hold no `.rs`
/// files and pass vacuously. This guard walks both test trees from
/// `CARGO_MANIFEST_DIR` — never the working directory — and fails naming
/// every file with zero or more than one declaring shim, so an orphan can
/// never again hide uncompiled in a subdirectory.
#[test]
fn every_subdirectory_test_file_has_exactly_one_harness_shim() {
    use std::collections::BTreeMap;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut offenders: Vec<String> = Vec::new();

    for tree in [root.join("tests"), root.join("subx-core").join("tests")] {
        assert!(
            tree.is_dir(),
            "test tree {} missing — path resolution is broken",
            tree.display()
        );

        // Map each shim target (normalised absolute path) to the top-level
        // harness files declaring it.
        let mut declared: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
        let entries = fs::read_dir(&tree)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", tree.display()));
        for entry in entries {
            let path = entry.expect("readable dirent").path();
            if !path.is_file() || path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
            for result in shim_targets(&text) {
                let target = result.unwrap_or_else(|reason| {
                    panic!(
                        "{}: {reason}",
                        path.strip_prefix(&root).unwrap_or(&path).display()
                    )
                });
                let resolved = normalise(&path.join("..").join(target));
                declared.entry(resolved).or_default().push(path.clone());
            }
        }

        // Every .rs under a non-module subdirectory of the tree must be
        // declared exactly once.
        let entries = fs::read_dir(&tree)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", tree.display()));
        for entry in entries {
            let entry = entry.expect("readable dirent");
            let dir = entry.path();
            let meta = entry
                .file_type()
                .unwrap_or_else(|e| panic!("failed to stat {}: {e}", dir.display()));
            if !meta.is_dir() || meta.is_symlink() || dir.join("mod.rs").is_file() {
                continue;
            }
            let mut nested = Vec::new();
            collect_orphan_candidates(&dir, &mut nested);
            for file in nested {
                let file = normalise(&file);
                let relative = file.strip_prefix(&root).unwrap_or(&file).display();
                match declared.get(&file).map(Vec::len) {
                    None => offenders.push(format!(
                        "{relative}: no #[path] shim in {} reaches this file",
                        tree.strip_prefix(&root).unwrap_or(&tree).display(),
                    )),
                    Some(1) => {}
                    Some(count) => offenders.push(format!(
                        "{relative}: declared by {count} shims ({}) — exactly one is required",
                        declared[&file]
                            .iter()
                            .map(|s| s.strip_prefix(&root).unwrap_or(s).display().to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                    )),
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "every .rs under a subdirectory of either crate's tests/ tree must be \
         reached by exactly one top-level #[path] shim (module directories \
         with a mod.rs are reached by `mod <dir>;` and exempt). Orphan test \
         files are never compiled and silently inflate the impression of \
         coverage; wire each up, relocate it to the top level, or delete it. \
         \nOffenders:\n{}",
        offenders.join("\n")
    );
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
