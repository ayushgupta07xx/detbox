//! Fuzz-hour ledger — the other half of konflux **P1**.
//!
//! MASTER_PLAN §4.1 P1: *"K1 holds on a corpus of ≥1,000 real-world files plus
//! **≥72 cumulative hours of fuzzing with zero violations**."* §12 wants that
//! number on a badge.
//!
//! # Where the number comes from
//!
//! GitHub's workflow-run history, not a file in this repo. A committed ledger
//! would be a number we maintain about ourselves, editable in the same commit
//! that needed it to be larger. The run history is written by the thing doing
//! the work and cannot be edited from a PR.
//!
//! `gh` fetches it; this parses and totals it. That keeps `xtask` free of both
//! third-party dependencies and network access — it is handed a file.
//!
//! # Why most recorded hours do not count
//!
//! **Fuzzing proves the code that was fuzzed.** Seventy-two hours against
//! last week's parser says nothing about today's. So runs are counted only when
//! their commit descends from the last change to the fuzzed code — `core-cst`,
//! `core-formats`, or the targets themselves.
//!
//! That makes the headline number small and occasionally resets it to zero,
//! which is the honest behaviour: a parser change invalidates the evidence
//! gathered about its predecessor. Reporting the larger cumulative figure and
//! calling it P1 would be exactly the kind of true-arithmetic-false-claim this
//! project exists to avoid.

use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

/// konflux P1's fuzzing threshold, in hours.
const P1_HOURS: f64 = 72.0;

/// Paths whose change invalidates accumulated fuzz-hours.
const FUZZED_PATHS: &[&str] = &[
    "crates/core-cst",
    "crates/core-formats",
    "fuzz/fuzz_targets",
    "fuzz/Cargo.toml",
];

/// One workflow run, as reported by `gh run list --json`.
#[derive(Debug, PartialEq, Eq)]
struct Run {
    sha: String,
    conclusion: String,
}

/// Pull `headSha` and `conclusion` out of `gh`'s JSON array.
///
/// A hand-rolled scanner rather than a JSON dependency: the shape is fixed by
/// the `--json` flag we pass, and one field pair does not justify a parser
/// (MASTER_PLAN §2). Covered by a fixture test below.
fn parse_runs(json: &str) -> Vec<Run> {
    let field = |chunk: &str, name: &str| -> Option<String> {
        let after = chunk.split(&format!("\"{name}\":")).nth(1)?;
        let value = after.trim_start().strip_prefix('"')?;
        value.split('"').next().map(str::to_string)
    };
    json.split('{')
        .filter_map(|chunk| {
            Some(Run {
                sha: field(chunk, "headSha")?,
                conclusion: field(chunk, "conclusion").unwrap_or_default(),
            })
        })
        .collect()
}

/// Seconds of fuzzing one nightly run performs, read from the workflow rather
/// than hardcoded, so the two cannot drift apart.
fn nightly_seconds_and_targets(root: &Path) -> Result<(f64, usize), String> {
    let path = root.join(".github/workflows/fuzz-nightly.yml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;

    let seconds: f64 = text
        .split("-max_total_time=")
        .nth(1)
        .and_then(|rest| {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .ok_or_else(|| format!("no -max_total_time= in {}", path.display()))?;

    let targets = text
        .split("target: [")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .map(|list| list.split(',').filter(|t| !t.trim().is_empty()).count())
        .ok_or_else(|| format!("no target matrix in {}", path.display()))?;

    Ok((seconds, targets))
}

/// The last commit that touched code the fuzzers exercise.
fn last_fuzzed_change(root: &Path) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(root)
        .args(["log", "-1", "--format=%H", "--"]);
    for path in FUZZED_PATHS {
        cmd.arg(path);
    }
    let out = cmd.output().ok()?;
    let sha = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!sha.is_empty()).then_some(sha)
}

/// Whether `descendant` has `ancestor` in its history.
fn descends_from(root: &Path, ancestor: &str, descendant: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .status()
        .is_ok_and(|s| s.success())
}

/// Total the fuzz-hours in `runs_json` and check them against P1.
///
/// # Errors
///
/// Errors when the workflow cannot be read, or when the qualifying hours fall
/// short of [`P1_HOURS`]. Falling short is the expected state for a long time,
/// and saying so is the point.
pub(crate) fn run(root: &Path, runs_json: &str) -> Result<String, String> {
    let (seconds, targets) = nightly_seconds_and_targets(root)?;
    #[allow(clippy::cast_precision_loss)]
    let hours_per_run = seconds * targets as f64 / 3600.0;

    let runs = parse_runs(runs_json);
    let successful: Vec<&Run> = runs.iter().filter(|r| r.conclusion == "success").collect();

    let baseline = last_fuzzed_change(root);
    let qualifying: Vec<&&Run> = match &baseline {
        None => successful.iter().collect(),
        Some(base) => successful
            .iter()
            .filter(|r| descends_from(root, base, &r.sha))
            .collect(),
    };

    #[allow(clippy::cast_precision_loss)]
    let all_hours = successful.len() as f64 * hours_per_run;
    #[allow(clippy::cast_precision_loss)]
    let live_hours = qualifying.len() as f64 * hours_per_run;

    let mut out = String::new();
    let _ = writeln!(out, "fuzz-hours — konflux P1");
    let _ = writeln!(
        out,
        "  one nightly run = {targets} target(s) x {seconds:.0}s = {hours_per_run:.2} hours"
    );
    let _ = writeln!(
        out,
        "  runs recorded: {} total, {} successful",
        runs.len(),
        successful.len()
    );
    let _ = writeln!(out, "  cumulative, all history: {all_hours:.2} hours");
    if let Some(base) = &baseline {
        let _ = writeln!(
            out,
            "  fuzzed code last changed at {}",
            base.get(..12).unwrap_or(base)
        );
    }
    let _ = writeln!(
        out,
        "  ON THE CURRENT PARSER:   {live_hours:.2} of {P1_HOURS:.0} hours"
    );

    if live_hours + f64::EPSILON >= P1_HOURS {
        let _ = writeln!(out, "  P1 fuzzing half: MET");
        Ok(out)
    } else {
        Err(format!(
            "{out}\n\
             P1 fuzzing half: NOT MET — {live_hours:.2} of {P1_HOURS:.0} hours.\n\n\
             Only runs descending from the last change to the fuzzed code count.\n\
             Fuzzing proves the code that was fuzzed, so a parser change resets\n\
             this, and reporting the {all_hours:.2}-hour cumulative figure as P1\n\
             would be true arithmetic making a false claim."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{Run, nightly_seconds_and_targets, parse_runs};
    use std::path::Path;

    const SAMPLE: &str = r#"[{"conclusion":"success","databaseId":1,"headSha":"aaa111"},
      {"conclusion":"failure","databaseId":2,"headSha":"bbb222"},
      {"conclusion":"","databaseId":3,"headSha":"ccc333"}]"#;

    #[test]
    fn parses_the_shape_gh_emits() {
        let runs = parse_runs(SAMPLE);
        assert_eq!(
            runs,
            vec![
                Run {
                    sha: "aaa111".into(),
                    conclusion: "success".into()
                },
                Run {
                    sha: "bbb222".into(),
                    conclusion: "failure".into()
                },
                Run {
                    sha: "ccc333".into(),
                    conclusion: String::new()
                },
            ]
        );
    }

    #[test]
    fn malformed_input_yields_nothing_rather_than_panicking() {
        for junk in ["", "[]", "not json", "{\"conclusion\":\"success\"}"] {
            assert!(parse_runs(junk).is_empty(), "{junk:?}");
        }
    }

    #[test]
    fn the_run_length_is_read_from_the_workflow_not_hardcoded() {
        // If the workflow's -max_total_time or its target matrix changes, the
        // ledger must follow. Hardcoding would let the two drift and quietly
        // inflate the hours.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        let (seconds, targets) = nightly_seconds_and_targets(root).expect("workflow is readable");
        assert!(seconds > 0.0, "no fuzzing time parsed");
        assert!(targets > 0, "no targets parsed");
    }
}
