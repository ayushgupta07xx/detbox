//! Conformance runner: official suites, honest rates, a one-way ratchet.
//!
//! MASTER_PLAN §3.3 requires conformance adapters; §4.1 **P4** requires pass
//! rates *"published as badges, including honest failure lists"*.
//!
//! # Why there is no single "pass rate"
//!
//! The obvious metric is `passed / total`. It is worse than useless here, and
//! the numbers say so plainly. JSONTestSuite is 95 must-accept cases and 188
//! must-reject. A parser that does nothing at all — rejects every input,
//! unconditionally — scores:
//!
//! ```text
//!   0 of  95 must-accept   correct
//! 188 of 188 must-reject   correct
//! -------------------------------
//! 188 of 283               = 66% "conformance"
//! ```
//!
//! Two-thirds, from a function that is `return Err`. A blended rate rewards
//! rejecting things, which is the opposite of what a format implementation is
//! for, and it would let a badge climb while the parser rots.
//!
//! So this module reports **accept-rate and reject-rate separately, always**,
//! and there is deliberately no method that combines them. Both are ratcheted
//! independently: a parser that starts accepting more cannot pay for it by
//! rejecting less.
//!
//! # The ratchet
//!
//! `conformance/thresholds.tsv` records what we currently claim. A measured
//! rate below its recorded threshold fails. Raising a threshold is a normal
//! reviewed change; **lowering one requires `[NEEDS-AYUSH-APPROVAL]`** (§8),
//! and `golden-guard` covers the file.
//!
//! A threshold of `unrecorded` is an **error**, not a free pass. Unlike a
//! benchmark baseline — which may be `uncalibrated` because timings are
//! machine-dependent (ADR-006) — a conformance rate is deterministic, so there
//! is no honest reason not to have recorded one.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// What the suite says should happen to a case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expectation {
    /// The input is valid and must be accepted.
    MustAccept,
    /// The input is invalid and must be rejected.
    MustReject,
    /// The spec permits either. Scored and reported, never gated on.
    ImplementationDefined,
}

/// What our implementation actually did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Produced a tree.
    Accepted,
    /// Refused, with diagnostics.
    Rejected,
}

/// One suite case.
#[derive(Debug)]
pub struct Case {
    /// Stable identifier, used in the published failure list.
    pub name: String,
    /// The exact bytes to feed the implementation.
    pub bytes: Vec<u8>,
    /// What the suite requires.
    pub expectation: Expectation,
}

/// Counts for one expectation class.
#[derive(Debug, Default, Clone, Copy)]
pub struct Tally {
    /// Cases in this class.
    pub total: usize,
    /// Cases whose verdict matched.
    pub correct: usize,
}

impl Tally {
    /// Correct fraction in `0.0..=1.0`. An empty class scores zero, never one:
    /// a class with no cases has proven nothing.
    #[must_use]
    pub fn rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        // Case counts are in the hundreds; f64 has ample mantissa here.
        #[allow(clippy::cast_precision_loss)]
        {
            self.correct as f64 / self.total as f64
        }
    }
}

/// The outcome of running a suite.
#[derive(Debug)]
pub struct Report {
    /// Suite identifier, matching the thresholds file.
    pub suite: String,
    /// Valid inputs we were required to accept.
    pub accept: Tally,
    /// Invalid inputs we were required to reject.
    pub reject: Tally,
    /// Cases the spec leaves open. Reported, never gated.
    pub either: Tally,
    /// Every mismatch, in case order — §4.1 P4's "honest failure list".
    pub failures: Vec<(String, Expectation, Verdict)>,
}

impl Report {
    /// Render the failure list for publication.
    #[must_use]
    pub fn failure_list(&self, limit: usize) -> String {
        let mut out = String::new();
        for (name, expected, got) in self.failures.iter().take(limit) {
            let _ = writeln!(out, "    {name}: expected {expected:?}, got {got:?}");
        }
        if self.failures.len() > limit {
            let _ = writeln!(out, "    ... and {} more", self.failures.len() - limit);
        }
        out
    }

    /// Human summary. Deliberately prints two rates and never their average.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "{}: accept {}/{} ({:.1}%)  reject {}/{} ({:.1}%)  \
             implementation-defined accepted {}/{}",
            self.suite,
            self.accept.correct,
            self.accept.total,
            self.accept.rate() * 100.0,
            self.reject.correct,
            self.reject.total,
            self.reject.rate() * 100.0,
            self.either.correct,
            self.either.total,
        )
    }
}

/// Run `cases` through `verdict`.
///
/// Cases are sorted by name first, so the failure list is stable across runs
/// and across platforms (§9.5).
pub fn run<F>(suite: &str, mut cases: Vec<Case>, verdict: F) -> Report
where
    F: Fn(&[u8]) -> Verdict,
{
    cases.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));

    let mut accept = Tally::default();
    let mut reject = Tally::default();
    let mut either = Tally::default();
    let mut failures = Vec::new();

    for case in &cases {
        let got = verdict(&case.bytes);
        match case.expectation {
            Expectation::MustAccept => {
                accept.total += 1;
                if got == Verdict::Accepted {
                    accept.correct += 1;
                } else {
                    failures.push((case.name.clone(), case.expectation, got));
                }
            }
            Expectation::MustReject => {
                reject.total += 1;
                if got == Verdict::Rejected {
                    reject.correct += 1;
                } else {
                    failures.push((case.name.clone(), case.expectation, got));
                }
            }
            Expectation::ImplementationDefined => {
                // Counted for publication, never a failure: the spec permits
                // either answer, and gating on one would invent a requirement.
                either.total += 1;
                if got == Verdict::Accepted {
                    either.correct += 1;
                }
            }
        }
    }

    Report {
        suite: suite.to_string(),
        accept,
        reject,
        either,
        failures,
    }
}

/// A recorded claim: `unrecorded`, or a rate in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Threshold {
    Unrecorded,
    Rate(f64),
}

/// Why a ratchet check failed.
#[derive(Debug)]
pub enum RatchetError {
    /// The thresholds file could not be read or parsed.
    Unreadable(String),
    /// No threshold has been recorded for a suite/class.
    Unrecorded {
        /// Suite identifier.
        suite: String,
        /// `accept` or `reject`.
        class: String,
        /// What the run measured.
        measured: f64,
    },
    /// A measured rate fell below its recorded threshold.
    Regressed {
        /// Suite identifier.
        suite: String,
        /// `accept` or `reject`.
        class: String,
        /// What was recorded.
        threshold: f64,
        /// What the run measured.
        measured: f64,
    },
}

impl std::fmt::Display for RatchetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable(why) => write!(f, "conformance thresholds unreadable: {why}"),
            Self::Unrecorded {
                suite,
                class,
                measured,
            } => write!(
                f,
                "no conformance threshold recorded for `{suite}` `{class}` \
                 (this run measured {:.1}%).\n\
                 A conformance rate is deterministic, so `unrecorded` is not a\n\
                 calibration problem — it means no claim has been made yet. Record\n\
                 the measured rate in conformance/thresholds.tsv as a reviewed change.",
                measured * 100.0
            ),
            Self::Regressed {
                suite,
                class,
                threshold,
                measured,
            } => write!(
                f,
                "CONFORMANCE REGRESSION: `{suite}` `{class}` measured {:.1}%, \
                 below the recorded {:.1}%.\n\
                 Thresholds ratchet one way. Lowering one requires a \
                 [NEEDS-AYUSH-APPROVAL] header (§8).",
                measured * 100.0,
                threshold * 100.0
            ),
        }
    }
}

impl std::error::Error for RatchetError {}

/// Check a report against `conformance/thresholds.tsv`.
///
/// # Errors
///
/// Returns the first violation: an unreadable file, an unrecorded claim, or a
/// rate below its threshold.
pub fn check_ratchet(report: &Report, thresholds: &Path) -> Result<String, RatchetError> {
    let text = std::fs::read_to_string(thresholds)
        .map_err(|e| RatchetError::Unreadable(format!("{}: {e}", thresholds.display())))?;

    let mut recorded: Vec<(String, String, Threshold)> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(suite), Some(class), Some(value)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(RatchetError::Unreadable(format!(
                "{}: expected `<suite> <accept|reject> <rate|unrecorded>`, got `{line}`",
                thresholds.display()
            )));
        };
        let threshold = if value == "unrecorded" {
            Threshold::Unrecorded
        } else {
            Threshold::Rate(value.parse::<f64>().map_err(|_| {
                RatchetError::Unreadable(format!(
                    "{}: `{value}` is neither a rate nor `unrecorded`",
                    thresholds.display()
                ))
            })?)
        };
        recorded.push((suite.to_string(), class.to_string(), threshold));
    }

    let mut summary = String::new();
    for (class, tally) in [("accept", report.accept), ("reject", report.reject)] {
        let measured = tally.rate();
        let found = recorded
            .iter()
            .find(|(s, c, _)| s == &report.suite && c == class);
        let Some((_, _, threshold)) = found else {
            return Err(RatchetError::Unrecorded {
                suite: report.suite.clone(),
                class: class.to_string(),
                measured,
            });
        };
        match threshold {
            Threshold::Unrecorded => {
                return Err(RatchetError::Unrecorded {
                    suite: report.suite.clone(),
                    class: class.to_string(),
                    measured,
                });
            }
            Threshold::Rate(rate) => {
                if measured + f64::EPSILON < *rate {
                    return Err(RatchetError::Regressed {
                        suite: report.suite.clone(),
                        class: class.to_string(),
                        threshold: *rate,
                        measured,
                    });
                }
                let _ = writeln!(
                    summary,
                    "  {} {class}: {:.1}% (threshold {:.1}%)",
                    report.suite,
                    measured * 100.0,
                    rate * 100.0
                );
            }
        }
    }
    Ok(summary)
}

/// Locate `conformance/` from a crate manifest directory.
#[must_use]
pub fn dir_from(manifest_dir: &str, levels_up: usize) -> PathBuf {
    Path::new(manifest_dir)
        .ancestors()
        .nth(levels_up)
        .map_or_else(
            || PathBuf::from("conformance"),
            |root| root.join("conformance"),
        )
}

#[cfg(test)]
mod tests {
    use super::{Case, Expectation, RatchetError, Tally, Verdict, check_ratchet, run};

    fn case(name: &str, expectation: Expectation) -> Case {
        Case {
            name: name.to_string(),
            bytes: name.as_bytes().to_vec(),
            expectation,
        }
    }

    fn thresholds(body: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "core-verify-thresholds-{}.tsv",
            body.len() // distinct per fixture, and stable across runs
        ));
        std::fs::write(&path, body).expect("write fixture");
        path
    }

    #[test]
    fn a_do_nothing_parser_scores_zero_on_accept() {
        // The whole reason accept and reject are never blended: a parser that
        // rejects everything is perfect on one class and useless overall.
        let cases = vec![
            case("y_a", Expectation::MustAccept),
            case("y_b", Expectation::MustAccept),
            case("n_a", Expectation::MustReject),
            case("n_b", Expectation::MustReject),
            case("n_c", Expectation::MustReject),
        ];
        let report = run("demo", cases, |_| Verdict::Rejected);
        assert_eq!(report.accept.correct, 0);
        assert_eq!(report.reject.correct, 3);
        assert!((report.accept.rate() - 0.0).abs() < f64::EPSILON);
        assert!((report.reject.rate() - 1.0).abs() < f64::EPSILON);
        // A blended figure would have been 3/5 = 60%. There is no method to
        // compute one, deliberately.
        assert!(report.summary().contains("accept 0/2"));
        assert!(report.summary().contains("reject 3/3"));
    }

    #[test]
    fn implementation_defined_cases_never_fail() {
        let cases = vec![case("i_a", Expectation::ImplementationDefined)];
        for verdict in [Verdict::Accepted, Verdict::Rejected] {
            let report = run("demo", cases_clone(&cases), |_| verdict);
            assert!(
                report.failures.is_empty(),
                "{verdict:?} was treated as a failure"
            );
            assert_eq!(report.either.total, 1);
        }
    }

    fn cases_clone(cases: &[Case]) -> Vec<Case> {
        cases
            .iter()
            .map(|c| Case {
                name: c.name.clone(),
                bytes: c.bytes.clone(),
                expectation: c.expectation,
            })
            .collect()
    }

    #[test]
    fn an_empty_class_scores_zero_not_one() {
        // 0/0 is not 100%. A class with no cases has proven nothing, and
        // scoring it perfect would let a suite that failed to load look green.
        assert!((Tally::default().rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn unrecorded_thresholds_are_an_error() {
        let path = thresholds("#! schema = 1\ndemo accept unrecorded\ndemo reject unrecorded\n");
        let report = run("demo", vec![case("y", Expectation::MustAccept)], |_| {
            Verdict::Accepted
        });
        match check_ratchet(&report, &path) {
            Err(RatchetError::Unrecorded { class, .. }) => assert_eq!(class, "accept"),
            other => panic!("expected Unrecorded, got {other:?}"),
        }
    }

    #[test]
    fn a_dropped_rate_is_a_regression() {
        let path = thresholds("demo accept 1.0\ndemo reject 1.0\n");
        let report = run(
            "demo",
            vec![
                case("y_a", Expectation::MustAccept),
                case("y_b", Expectation::MustAccept),
            ],
            |bytes| {
                if bytes == b"y_a" {
                    Verdict::Accepted
                } else {
                    Verdict::Rejected
                }
            },
        );
        match check_ratchet(&report, &path) {
            Err(RatchetError::Regressed {
                threshold,
                measured,
                ..
            }) => {
                assert!((threshold - 1.0).abs() < f64::EPSILON);
                assert!((measured - 0.5).abs() < f64::EPSILON);
            }
            other => panic!("expected Regressed, got {other:?}"),
        }
    }

    #[test]
    fn meeting_the_threshold_passes_and_exceeding_it_passes() {
        let path = thresholds("demo accept 0.5\ndemo reject 0.0\n\n");
        let report = run(
            "demo",
            vec![
                case("y_a", Expectation::MustAccept),
                case("y_b", Expectation::MustAccept),
            ],
            |bytes| {
                if bytes == b"y_a" {
                    Verdict::Accepted
                } else {
                    Verdict::Rejected
                }
            },
        );
        let summary = check_ratchet(&report, &path).expect("0.5 measured meets a 0.5 threshold");
        assert!(summary.contains("50.0%"));
    }

    #[test]
    fn the_failure_list_is_stable_and_truncates() {
        let cases: Vec<Case> = ["y_c", "y_a", "y_b"]
            .iter()
            .map(|n| case(n, Expectation::MustAccept))
            .collect();
        let report = run("demo", cases, |_| Verdict::Rejected);
        let names: Vec<&str> = report.failures.iter().map(|(n, _, _)| n.as_str()).collect();
        assert_eq!(names, ["y_a", "y_b", "y_c"], "failure list is not sorted");
        assert!(report.failure_list(2).contains("and 1 more"));
    }
}
