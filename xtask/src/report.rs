//! The determinism oracle for Phase 0.
//!
//! MASTER_PLAN §8 requires a "double-build + double-run output-hash compare"
//! gate. Before any tool produces output there is nothing to hash, so the gate
//! would be vacuous — and a vacuous gate rots (`core-verify` invariant V4).
//!
//! This module produces a deterministic JSON report over the repository tree.
//! It is a real exercise of every failure mode MASTER_PLAN §9.5 names:
//!
//! | §9.5 rule | How this module is subject to it |
//! |---|---|
//! | no `HashMap` iteration in an output path | entries are a `Vec` sorted byte-wise, and `HashMap` is banned workspace-wide |
//! | no wall-clock in outputs | no field carries mtime, ctime or run time — asserted by a unit test |
//! | stable sorts | `sort_by`, never `sort_unstable_by` |
//! | path handling identical across OS | separators normalised to `/` before they reach the output |
//! | locale-independent formatting | byte-wise comparison, no `to_lowercase`, no locale-aware collation |
//!
//! At konflux M1 the determinism gate additionally runs the real tool binaries.
//! This report stays: it is the gate's canary for the repo itself. See ADR-005.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::sha256::{Sha256, to_hex};

/// Bumped whenever the report's shape changes. Consumers pin it.
const SCHEMA_VERSION: u32 = 1;

/// Directory names never descended into. Build output and VCS internals are not
/// part of the repository's deterministic content.
const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", ".venv", "__pycache__"];

/// A directory holding this file is fetched third-party corpus content, not
/// repository content (`corpora/fetch.sh` writes it as the fetch receipt).
/// Skipping it keeps the report the same whether or not the corpus happens to be
/// fetched on this machine — the corpus has its own integrity receipt.
const FETCH_RECEIPT: &str = "MANIFEST.tsv";

/// One file in the report.
struct Entry {
    /// Repo-relative path, `/`-separated on every platform.
    path: String,
    /// Size in bytes.
    len: u64,
    /// SHA-256 of the file's bytes, lowercase hex.
    digest: String,
}

/// Render the deterministic scaffold report for the tree rooted at `root`.
///
/// # Errors
///
/// Returns the first I/O error encountered while walking or reading.
pub(crate) fn render(root: &Path) -> std::io::Result<String> {
    let mut entries = Vec::new();
    walk(root, root, &mut entries)?;
    // Stable sort on the normalised path bytes: identical on every platform and
    // in every locale (MASTER_PLAN §9.5).
    entries.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));

    let mut out = String::new();
    out.push_str("{\n");
    let _ = writeln!(out, "  \"schema_version\": {SCHEMA_VERSION},");
    let _ = writeln!(out, "  \"file_count\": {},", entries.len());
    out.push_str("  \"files\": [\n");
    for (i, entry) in entries.iter().enumerate() {
        let comma = if i + 1 == entries.len() { "" } else { "," };
        let _ = writeln!(
            out,
            "    {{\"path\": \"{}\", \"len\": {}, \"sha256\": \"{}\"}}{comma}",
            escape(&entry.path),
            entry.len,
            entry.digest
        );
    }
    out.push_str("  ]\n}\n");
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<Entry>) -> std::io::Result<()> {
    if dir.join(FETCH_RECEIPT).is_file() {
        return Ok(());
    }
    // Collect and sort before descending so that recursion order is also
    // deterministic, not just the final report. Filesystem order is never trusted.
    let mut children: Vec<PathBuf> = std::fs::read_dir(dir)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|e| e.path())
        .collect();
    children.sort_by(|a, b| a.as_os_str().cmp(b.as_os_str()));

    for child in children {
        let name = child.file_name().map(std::ffi::OsStr::to_os_string);
        let is_skipped = name
            .as_ref()
            .and_then(|n| n.to_str())
            .is_some_and(|n| SKIP_DIRS.contains(&n));
        if is_skipped {
            continue;
        }
        // `symlink_metadata`: never follow a symlink, so a link loop cannot hang
        // the walk and a link's target cannot leak in twice.
        let meta = std::fs::symlink_metadata(&child)?;
        if meta.is_dir() {
            walk(root, &child, out)?;
        } else if meta.is_file() {
            let bytes = std::fs::read(&child)?;
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            out.push(Entry {
                path: relative(root, &child),
                len: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                digest: to_hex(&hasher.finalize()),
            });
        }
        // Symlinks are neither followed nor recorded: their target is recorded
        // on its own if it is inside the tree.
    }
    Ok(())
}

/// Repo-relative, `/`-separated path (MASTER_PLAN §9.5: "path handling
/// identical across OS — normalize separators in output").
fn relative(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{SKIP_DIRS, escape, relative, render};
    use std::path::Path;

    fn workspace_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has a parent")
            .to_path_buf()
    }

    #[test]
    fn two_runs_produce_identical_bytes() {
        let root = workspace_root();
        let first = render(&root).expect("render");
        let second = render(&root).expect("render");
        assert_eq!(first, second, "the determinism oracle is not deterministic");
    }

    #[test]
    fn the_report_contains_no_wall_clock_field() {
        // MASTER_PLAN §9.5: no wall-clock in outputs. Guard the field names, so
        // that adding `"mtime"` later trips this test rather than CI's hash compare
        // in six months.
        let out = render(&workspace_root()).expect("render");
        for banned in ["mtime", "ctime", "modified", "timestamp", "generated_at"] {
            assert!(
                !out.contains(banned),
                "wall-clock-derived field `{banned}` leaked into the report"
            );
        }
    }

    #[test]
    fn paths_are_slash_separated_everywhere() {
        let out = render(&workspace_root()).expect("render");
        assert!(!out.contains('\\'), "a platform path separator leaked out");
        assert!(out.contains("crates/core-cst/src/lib.rs"));
    }

    #[test]
    fn file_entries_are_sorted_byte_wise() {
        let out = render(&workspace_root()).expect("render");
        let paths: Vec<&str> = out
            .lines()
            .filter_map(|l| l.split("\"path\": \"").nth(1))
            .filter_map(|l| l.split('"').next())
            .collect();
        let mut sorted = paths.clone();
        sorted.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        assert_eq!(paths, sorted, "report ordering is not byte-wise sorted");
        assert!(
            paths.len() > 20,
            "report looks empty: {} files",
            paths.len()
        );
    }

    #[test]
    fn build_output_is_not_reported() {
        let out = render(&workspace_root()).expect("render");
        for skipped in SKIP_DIRS {
            assert!(
                !out.contains(&format!("\"{skipped}/")),
                "`{skipped}` leaked into the report; the hash would change on every build"
            );
        }
    }

    #[test]
    fn fetched_corpus_content_is_not_reported() {
        // The report must read the same whether or not `corpora/fetch.sh` has run
        // on this machine. Otherwise the determinism gate's hash would depend on
        // local state, which is exactly the class of bug it exists to catch.
        let out = render(&workspace_root()).expect("render");
        assert!(
            !out.contains("/files/"),
            "fetched corpus content leaked into the report"
        );
        assert!(!out.contains("MANIFEST.tsv"), "fetch receipts leaked in");
        // The manifests themselves ARE repo content and must stay covered.
        assert!(out.contains("corpora/sources/helm-charts.sources"));
    }

    #[test]
    fn helpers_behave() {
        assert_eq!(escape(r#"a"b\c"#), r#"a\"b\\c"#);
        assert_eq!(
            relative(Path::new("/repo"), Path::new("/repo/a/b.rs")),
            "a/b.rs"
        );
    }
}
