//! Offline verification of the corpus source manifests.
//!
//! MASTER_PLAN §10: *"corpus files respect upstream licenses (fetch scripts,
//! not vendored copies, where required)."* That is a promise, and a promise
//! that is not checked is a wish. This module is the check, and it runs in CI
//! with **no network access** — it validates the manifests, not the fetch.
//!
//! The rules enforced here (all blocking):
//!
//! 1. Every source declares `name`, `repo`, `rev`, `license`, `license_file`,
//!    at least one `include`, and `max_files`.
//! 2. `rev` is a full 40-character lowercase hex commit SHA. **Never a branch
//!    or a tag** — those move, and a moving corpus means a proof that was green
//!    yesterday can go red today for reasons unrelated to the code.
//! 3. `license` is on the permissive allow-list.
//! 4. `repo` is an `https://` git URL.
//! 5. Names are unique within a category.
//! 6. Per-category `max_files` sums to the category's declared `cap`.
//! 7. The sum of all caps does not exceed [`GLOBAL_FILE_CAP`].

use std::fmt::Write as _;
use std::path::Path;

/// Phase 0 corpus ceiling. konflux P1 requires ≥1,000 real-world files; this is
/// that number, and it is the *initial* cap — raising it is a deliberate act.
pub(crate) const GLOBAL_FILE_CAP: u32 = 1_000;

/// Licences under which we are willing to fetch third-party files into a test
/// corpus. Anything else needs a decision, not a default.
const ALLOWED_LICENSES: &[&str] = &[
    "Apache-2.0",
    "MIT",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "MPL-2.0",
];

/// A parsed `[source]` record.
struct Source {
    name: String,
    repo: String,
    rev: String,
    license: String,
    license_file: String,
    includes: Vec<String>,
    max_files: u32,
    line: usize,
}

/// A parsed category manifest.
struct Manifest {
    file: String,
    category: String,
    cap: u32,
    sources: Vec<Source>,
}

/// Verify every manifest under `sources_dir`. Returns the human-readable
/// summary on success.
///
/// # Errors
///
/// Returns a rendered list of every violation found. All manifests are checked
/// before returning, so one run reports every problem rather than the first.
pub(crate) fn verify(sources_dir: &Path) -> Result<String, String> {
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(sources_dir)
        .map_err(|e| format!("cannot read {}: {e}", sources_dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "sources"))
        .collect();
    files.sort_by(|a, b| a.as_os_str().cmp(b.as_os_str()));

    if files.is_empty() {
        return Err(format!(
            "no `*.sources` manifests in {} — a corpus gate with no sources is vacuous",
            sources_dir.display()
        ));
    }

    let mut problems = Vec::new();
    let mut manifests = Vec::new();
    for file in &files {
        match parse(file) {
            Ok(manifest) => manifests.push(manifest),
            Err(e) => problems.push(e),
        }
    }

    let mut total_cap = 0u32;
    let mut summary = String::from("corpus manifests\n");
    for manifest in &manifests {
        problems.extend(check(manifest));
        total_cap = total_cap.saturating_add(manifest.cap);
        let _ = writeln!(
            summary,
            "  {:<16} cap {:>5}   {} source(s)",
            manifest.category,
            manifest.cap,
            manifest.sources.len()
        );
    }
    let _ = writeln!(
        summary,
        "  {:<16} cap {:>5}   (ceiling {GLOBAL_FILE_CAP})",
        "TOTAL", total_cap
    );

    if total_cap > GLOBAL_FILE_CAP {
        problems.push(format!(
            "total cap {total_cap} exceeds the {GLOBAL_FILE_CAP}-file ceiling"
        ));
    }

    if problems.is_empty() {
        Ok(summary)
    } else {
        problems.sort();
        Err(problems.join("\n"))
    }
}

fn parse(path: &Path) -> Result<Manifest, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("{}: unreadable ({e})", path.display()))?;
    let file = path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("?")
        .to_string();

    let mut category = String::new();
    let mut cap: Option<u32> = None;
    let mut sources: Vec<Source> = Vec::new();

    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(directive) = line.strip_prefix("#!") {
            let (key, value) = split_kv(directive, &file, line_no)?;
            match key.as_str() {
                "category" => category = value,
                "cap" => {
                    cap =
                        Some(value.parse().map_err(|_| {
                            format!("{file}:{line_no}: cap `{value}` is not a number")
                        })?);
                }
                other => return Err(format!("{file}:{line_no}: unknown directive `{other}`")),
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if line == "[source]" {
            sources.push(Source {
                name: String::new(),
                repo: String::new(),
                rev: String::new(),
                license: String::new(),
                license_file: String::new(),
                includes: Vec::new(),
                max_files: 0,
                line: line_no,
            });
            continue;
        }
        let Some(current) = sources.last_mut() else {
            return Err(format!(
                "{file}:{line_no}: `{line}` appears before any [source]"
            ));
        };
        let (key, value) = split_kv(line, &file, line_no)?;
        match key.as_str() {
            "name" => current.name = value,
            "repo" => current.repo = value,
            "rev" => current.rev = value,
            "license" => current.license = value,
            "license_file" => current.license_file = value,
            "include" => current.includes.push(value),
            "max_files" => {
                current.max_files = value.parse().map_err(|_| {
                    format!("{file}:{line_no}: max_files `{value}` is not a number")
                })?;
            }
            other => return Err(format!("{file}:{line_no}: unknown key `{other}`")),
        }
    }

    if category.is_empty() {
        return Err(format!("{file}: missing `#! category = ...`"));
    }
    let Some(cap) = cap else {
        return Err(format!("{file}: missing `#! cap = ...`"));
    };
    Ok(Manifest {
        file,
        category,
        cap,
        sources,
    })
}

fn split_kv(line: &str, file: &str, line_no: usize) -> Result<(String, String), String> {
    let Some((key, value)) = line.split_once('=') else {
        return Err(format!("{file}:{line_no}: `{line}` is not `key = value`"));
    };
    Ok((key.trim().to_string(), value.trim().to_string()))
}

fn check(manifest: &Manifest) -> Vec<String> {
    let mut problems = Vec::new();
    let file = &manifest.file;

    if manifest.sources.is_empty() {
        problems.push(format!("{file}: declares a cap but lists no sources"));
    }

    let mut names: Vec<&str> = Vec::new();
    let mut declared = 0u32;
    for source in &manifest.sources {
        let at = format!("{file}:{}", source.line);
        for (field, value) in [
            ("name", &source.name),
            ("repo", &source.repo),
            ("rev", &source.rev),
            ("license", &source.license),
            ("license_file", &source.license_file),
        ] {
            if value.is_empty() {
                problems.push(format!("{at}: source is missing `{field}`"));
            }
        }
        if source.includes.is_empty() {
            problems.push(format!("{at}: source has no `include` pattern"));
        }
        if source.max_files == 0 {
            problems.push(format!("{at}: `max_files` must be > 0"));
        }
        if !is_pinned_sha(&source.rev) {
            problems.push(format!(
                "{at}: rev `{}` is not a 40-char lowercase hex commit SHA. \
                 Branches and tags move; a moving corpus makes a green proof meaningless.",
                source.rev
            ));
        }
        if !source.repo.starts_with("https://") {
            problems.push(format!(
                "{at}: repo `{}` is not an https:// URL",
                source.repo
            ));
        }
        if !ALLOWED_LICENSES.contains(&source.license.as_str()) {
            problems.push(format!(
                "{at}: license `{}` is not on the allow-list ({})",
                source.license,
                ALLOWED_LICENSES.join(", ")
            ));
        }
        if names.contains(&source.name.as_str()) {
            problems.push(format!("{at}: duplicate source name `{}`", source.name));
        }
        names.push(&source.name);
        declared = declared.saturating_add(source.max_files);
    }

    if declared != manifest.cap {
        problems.push(format!(
            "{file}: sources declare {declared} files but the category cap is {}. \
             They must match exactly, so the cap is a fact and not a hope.",
            manifest.cap
        ));
    }
    problems
}

fn is_pinned_sha(rev: &str) -> bool {
    rev.len() == 40
        && rev
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::{GLOBAL_FILE_CAP, is_pinned_sha, verify};
    use std::path::Path;

    fn sources_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("corpora/sources")
    }

    #[test]
    fn the_checked_in_manifests_are_valid() {
        match verify(&sources_dir()) {
            Ok(summary) => assert!(summary.contains("TOTAL"), "{summary}"),
            Err(problems) => panic!("corpus manifests are invalid:\n{problems}"),
        }
    }

    #[test]
    fn pinned_sha_rejects_moving_refs() {
        assert!(is_pinned_sha("7da1f0d6e15bb28f3ad2446b7e6c560cc164098f"));
        assert!(!is_pinned_sha("main"), "branches move");
        assert!(!is_pinned_sha("v1.2.3"), "tags move");
        assert!(
            !is_pinned_sha("7DA1F0D6E15BB28F3AD2446B7E6C560CC164098F"),
            "case must be canonical"
        );
        assert!(!is_pinned_sha("7da1f0d"), "short SHAs are ambiguous");
    }

    #[test]
    fn the_global_cap_is_the_documented_one() {
        // Phase 0 instruction: cap the initial corpus at ~1,000 files.
        assert_eq!(GLOBAL_FILE_CAP, 1_000);
    }
}
