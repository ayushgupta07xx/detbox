//! Benchmark baseline comparison (MASTER_PLAN §8: "criterion vs saved
//! baselines, ±threshold — blocking on regression").
//!
//! # What this gate proves at Phase 0, and what it does not
//!
//! Timings recorded on Ayush's WSL2 laptop are not comparable to timings on a
//! shared GitHub-hosted runner, so a checked-in *number* would produce false
//! regressions from day one. Claiming otherwise would be a cherry-picked
//! benchmark, which is a permanent ban (Appendix C).
//!
//! So the baseline file carries every benchmark's **name**, and a value that is
//! either a calibrated number or the literal `uncalibrated`. This gate then
//! enforces, blocking, from commit one:
//!
//! - the criterion run happened and its output parses;
//! - **every benchmark named in the baseline still exists** — a benchmark that
//!   silently disappears is how a perf suite rots;
//! - **every benchmark that ran is named in the baseline** — new benchmarks
//!   cannot land unrecorded;
//! - for calibrated entries, the measurement is within `tolerance_pct`.
//!
//! Calibrated numbers are recorded from a CI run on `main` and land as a
//! reviewed change, like any other threshold. Tightening is free; loosening
//! requires `[NEEDS-AYUSH-APPROVAL]`. See ADR-006.

use std::fmt::Write as _;
use std::path::Path;

/// A baseline entry: a benchmark name and its recorded nanoseconds, if any.
struct Baseline {
    name: String,
    nanos: Option<f64>,
}

/// Parsed baseline file.
struct BaselineFile {
    tolerance_pct: f64,
    entries: Vec<Baseline>,
}

/// Compare a criterion output directory against a baseline file.
///
/// # Errors
///
/// Returns a rendered report of every violation.
pub(crate) fn compare(criterion_dir: &Path, baseline_path: &Path) -> Result<String, String> {
    let baseline = parse_baseline(baseline_path)?;
    let measured = collect_measurements(criterion_dir)?;

    let mut problems = Vec::new();
    let mut summary = String::from("benchmark vs baseline\n");

    for entry in &baseline.entries {
        let Some((_, measured_nanos)) = measured.iter().find(|(name, _)| name == &entry.name)
        else {
            problems.push(format!(
                "benchmark `{}` is in the baseline but did not run. A benchmark that \
                 silently disappears takes its regression gate with it.",
                entry.name
            ));
            continue;
        };
        match entry.nanos {
            None => {
                let _ = writeln!(
                    summary,
                    "  {:<32} {:>12.1} ns   (uncalibrated — recording only)",
                    entry.name, measured_nanos
                );
            }
            Some(expected) => {
                let delta_pct = (measured_nanos - expected) / expected * 100.0;
                let verdict = if delta_pct > baseline.tolerance_pct {
                    problems.push(format!(
                        "benchmark `{}` regressed {delta_pct:+.1}% ({expected:.1} ns -> \
                         {measured_nanos:.1} ns), tolerance ±{:.1}%",
                        entry.name, baseline.tolerance_pct
                    ));
                    "REGRESSION"
                } else {
                    "ok"
                };
                let _ = writeln!(
                    summary,
                    "  {:<32} {:>12.1} ns   {delta_pct:+.1}%  {verdict}",
                    entry.name, measured_nanos
                );
            }
        }
    }

    for (name, _) in &measured {
        if !baseline.entries.iter().any(|e| &e.name == name) {
            problems.push(format!(
                "benchmark `{name}` ran but is not in {}. Add it, so it is covered by \
                 the regression gate.",
                baseline_path.display()
            ));
        }
    }

    if problems.is_empty() {
        Ok(summary)
    } else {
        problems.sort();
        Err(format!("{summary}\n{}", problems.join("\n")))
    }
}

fn parse_baseline(path: &Path) -> Result<BaselineFile, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read baseline {}: {e}", path.display()))?;
    let mut tolerance_pct: Option<f64> = None;
    let mut entries = Vec::new();

    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(directive) = line.strip_prefix("#!") {
            if let Some((key, value)) = directive.split_once('=')
                && key.trim() == "tolerance_pct"
            {
                tolerance_pct = value.trim().parse().ok();
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
            return Err(format!(
                "{}:{line_no}: expected `<name> <ns|uncalibrated>`",
                path.display()
            ));
        };
        let nanos = if value == "uncalibrated" {
            None
        } else {
            Some(value.parse::<f64>().map_err(|_| {
                format!(
                    "{}:{line_no}: `{value}` is neither a number nor `uncalibrated`",
                    path.display()
                )
            })?)
        };
        entries.push(Baseline {
            name: name.to_string(),
            nanos,
        });
    }

    let Some(tolerance_pct) = tolerance_pct else {
        return Err(format!(
            "{}: missing `#! tolerance_pct = ...`",
            path.display()
        ));
    };
    entries.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    Ok(BaselineFile {
        tolerance_pct,
        entries,
    })
}

/// Walk `<criterion_dir>/<bench>/new/estimates.json` and pull the mean point
/// estimate (nanoseconds) out of each.
fn collect_measurements(criterion_dir: &Path) -> Result<Vec<(String, f64)>, String> {
    if !criterion_dir.is_dir() {
        return Err(format!(
            "no criterion output at {} — did the bench run?",
            criterion_dir.display()
        ));
    }
    let mut found = Vec::new();
    let mut stack = vec![criterion_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut children: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
            .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .collect();
        children.sort_by(|a, b| a.as_os_str().cmp(b.as_os_str()));
        for child in children {
            if child.is_dir() {
                stack.push(child);
            } else if child.file_name().is_some_and(|n| n == "estimates.json")
                && child.parent().is_some_and(|p| p.ends_with("new"))
            {
                let name = child
                    .parent()
                    .and_then(Path::parent)
                    .and_then(Path::file_name)
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or("?")
                    .to_string();
                let text = std::fs::read_to_string(&child)
                    .map_err(|e| format!("cannot read {}: {e}", child.display()))?;
                if let Some(nanos) = mean_point_estimate(&text) {
                    found.push((name, nanos));
                }
            }
        }
    }
    if found.is_empty() {
        return Err(format!(
            "criterion produced no estimates under {} — a benchmark gate with no \
             measurements is vacuous",
            criterion_dir.display()
        ));
    }
    found.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    Ok(found)
}

/// Extract `mean.point_estimate` from criterion's `estimates.json`.
///
/// Deliberately a scanner rather than a JSON dependency: the shape is fixed by
/// criterion and pulling in a parser for one float is not a justified
/// dependency (MASTER_PLAN §2). Covered by a fixture test below.
fn mean_point_estimate(json: &str) -> Option<f64> {
    let after_mean = json.split("\"mean\"").nth(1)?;
    let after_key = after_mean.split("\"point_estimate\"").nth(1)?;
    let value: String = after_key
        .trim_start()
        .trim_start_matches(':')
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == 'e' || *c == '-' || *c == '+')
        .collect();
    value.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::mean_point_estimate;

    const CRITERION_SHAPE: &str = r#"{
      "mean": {"confidence_interval": {"confidence_level": 0.95,
        "lower_bound": 1234.5, "upper_bound": 1240.1},
        "point_estimate": 1237.25, "standard_error": 1.4},
      "median": {"point_estimate": 1236.0},
      "slope": {"point_estimate": 1237.9}
    }"#;

    #[test]
    fn reads_the_mean_not_the_median_or_slope() {
        assert_eq!(mean_point_estimate(CRITERION_SHAPE), Some(1237.25));
    }

    #[test]
    fn malformed_input_is_none_not_a_panic() {
        assert_eq!(mean_point_estimate("{}"), None);
        assert_eq!(mean_point_estimate(r#"{"mean": {}}"#), None);
        assert_eq!(mean_point_estimate(""), None);
    }

    #[test]
    fn scientific_notation_parses() {
        assert_eq!(
            mean_point_estimate(r#"{"mean":{"point_estimate":1.23e5}}"#),
            Some(123_000.0)
        );
    }
}
