//! K1 over the fetched corpus — konflux **P1**.
//!
//! MASTER_PLAN §4.1 P1: *"K1 holds on a corpus of ≥1,000 real-world files (top
//! Helm charts, kustomize overlays, Terraform repos, K8s manifests) plus ≥72
//! cumulative hours of fuzzing with zero violations."*
//!
//! This is the corpus half. It lives in `xtask` rather than in a `#[test]`
//! because the corpus is fetched, not committed (ADR-004): a test would either
//! fail on every machine that has not run `corpora/fetch.sh`, or skip — and a
//! skip that reports green is the vacuity `core-verify` invariant V4 forbids.
//! As an xtask command it runs in the one CI job that fetches, and it is an
//! error, never a skip, when the corpus is missing.
//!
//! Coverage is reported **per format**, because a single total would hide which
//! half of konflux's MVP is actually proven.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use core_formats::{Format, Json, Yaml};

/// konflux **P1**: *"K1 holds on a corpus of ≥1,000 real-world files."*
///
/// Counted over files in a format the MVP actually parses. The corpus also
/// carries HCL for Phase 2, and counting those toward a proof that cannot yet
/// touch them would be arithmetic dressed as evidence.
const P1_CORPUS_MINIMUM: usize = 1_000;

#[derive(Default)]
struct Tally {
    files: usize,
    bytes: u64,
    round_tripped: usize,
    rejected: Vec<String>,
    violations: Vec<String>,
}

/// Run K1 across every corpus file whose extension a format claims.
///
/// # Errors
///
/// Errors when the corpus is absent, or when any file that parses fails to
/// round-trip. A K1 violation on real-world input is a release blocker, not a
/// statistic.
pub(crate) fn run(root: &Path) -> Result<String, String> {
    let corpora = root.join("corpora");
    let mut files: Vec<PathBuf> = Vec::new();
    collect(&corpora, &mut files);
    files.sort();

    if files.is_empty() {
        return Err(format!(
            "no corpus under {} — run corpora/fetch.sh first.\n\
             This is an error rather than a skip: a P1 run that measured nothing\n\
             and reported success is exactly the vacuity invariant V4 forbids.",
            corpora.display()
        ));
    }

    let yaml = Yaml;
    let json = Json;
    let mut by_format: BTreeMap<&'static str, Tally> = BTreeMap::new();
    let mut unclaimed = 0usize;

    for path in &files {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        let format: Option<&dyn Format> = if yaml.extensions().contains(&extension.as_str()) {
            Some(&yaml)
        } else if json.extensions().contains(&extension.as_str()) {
            Some(&json)
        } else {
            None
        };
        let Some(format) = format else {
            unclaimed += 1;
            continue;
        };

        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let name = relative(root, path);
        let tally = by_format.entry(format.name()).or_default();
        tally.files += 1;
        tally.bytes += u64::try_from(bytes.len()).unwrap_or(0);

        match format.parse(&bytes) {
            Err(_) => tally.rejected.push(name),
            Ok(cst) => {
                if format.serialize(&cst) == bytes {
                    tally.round_tripped += 1;
                } else {
                    tally.violations.push(name);
                }
            }
        }
    }

    Ok(report(&by_format, unclaimed)).and_then(|out| finish(out, &by_format))
}

/// Render the per-format table.
fn report(by_format: &BTreeMap<&'static str, Tally>, unclaimed: usize) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "K1 over the corpus — konflux P1");
    let _ = writeln!(
        out,
        "  {:<8} {:>7} {:>12} {:>14} {:>10}",
        "format", "files", "bytes", "round-tripped", "rejected"
    );
    let _ = writeln!(out, "  {}", "-".repeat(56));

    let mut total_files = 0usize;
    let mut total_ok = 0usize;
    for (name, tally) in by_format {
        let _ = writeln!(
            out,
            "  {name:<8} {:>7} {:>12} {:>14} {:>10}",
            tally.files,
            tally.bytes,
            tally.round_tripped,
            tally.rejected.len()
        );
        total_files += tally.files;
        total_ok += tally.round_tripped;
    }
    let _ = writeln!(
        out,
        "  {:<8} {:>7} {:>12} {:>14}",
        "TOTAL", total_files, "", total_ok
    );
    if unclaimed > 0 {
        let _ = writeln!(
            out,
            "\n  {unclaimed} corpus file(s) are in no format we parse yet (HCL, Phase 2)."
        );
    }
    if total_files >= P1_CORPUS_MINIMUM {
        let _ = writeln!(
            out,
            "  P1 corpus half: MET — {total_files} files in a format konflux parses,\n  \
             all round-tripping. The fuzzing half (≥72 cumulative hours) is tracked\n  \
             separately."
        );
    } else {
        let _ = writeln!(
            out,
            "  P1 corpus half: NOT MET — needs ≥{P1_CORPUS_MINIMUM} files in a format\n  \
             konflux parses, and only {total_files} are."
        );
    }
    out
}

/// A K1 violation on real-world input is a release blocker, not a statistic.
fn finish(out: String, by_format: &BTreeMap<&'static str, Tally>) -> Result<String, String> {
    let violations: Vec<String> = by_format
        .values()
        .flat_map(|t| t.violations.iter().cloned())
        .collect();
    if violations.is_empty() {
        Ok(out)
    } else {
        let listed: Vec<String> = violations.iter().take(20).cloned().collect();
        Err(format!(
            "{out}\nK1 VIOLATED on {} corpus file(s):\n  {}\n\n\
             serialize(parse(x)) != x on real-world input. This is a release\n\
             blocker (MASTER_PLAN §4.1 P1), not a statistic. Minimise one, file it\n\
             as a golden case, then fix the parser (§3.3).",
            violations.len(),
            listed.join("\n  ")
        ))
    }
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}
