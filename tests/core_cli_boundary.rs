//! Layering guard: no module under `src/core/` or `src/services/` may
//! reference `crate::cli`.
//!
//! The crate split (`expose-core-orchestration-apis`, eventually the
//! `subx-core` crate) requires `src/core/` and `src/services/` to stop
//! depending on the CLI layer: core code must not read the process-global
//! output mode or quiet flag, and must not call terminal helpers such as
//! `display_ai_usage`. All human-oriented output goes through the
//! `crate::core::report::Reporter` seam instead.
//!
//! Paths resolve from `CARGO_MANIFEST_DIR`, never the working directory, so
//! the guard works no matter where the test runner is invoked from.

use std::fs;
use std::path::{Path, PathBuf};

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
fn core_and_services_never_reference_the_cli_layer() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut offenders: Vec<String> = Vec::new();

    for subtree in ["src/core", "src/services"] {
        let mut files = Vec::new();
        collect_rs_files(&root.join(subtree), &mut files);
        assert!(
            !files.is_empty(),
            "guard walked no files under {subtree} — path resolution is broken"
        );
        for file in files {
            let text = fs::read_to_string(&file)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", file.display()));
            for (lineno, line) in text.lines().enumerate() {
                if line.contains("crate::cli") {
                    offenders.push(format!(
                        "{}:{}",
                        file.strip_prefix(&root).unwrap_or(&file).display(),
                        lineno + 1
                    ));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "src/core/ and src/services/ must not reference crate::cli \
         (route human-oriented output through crate::core::report::Reporter). \nOffending file:line:\n{}",
        offenders.join("\n")
    );
}
