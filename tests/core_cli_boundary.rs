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
//! Paths resolve from `CARGO_MANIFEST_DIR`, never the working directory, so
//! the guard works no matter where the test runner is invoked from.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN: [&str; 3] = ["crate::cli", "crate::commands", "subx_cli"];

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
