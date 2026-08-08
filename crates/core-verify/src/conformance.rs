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
    /// The suite's own description of the case, when it ships one.
    /// yaml-test-suite does; JSONTestSuite puts everything in the filename.
    pub title: Option<String>,
    /// The exact bytes to feed the implementation.
    pub bytes: Vec<u8>,
    /// What the suite requires.
    pub expectation: Expectation,
}

/// One mismatch: what the suite required, and what we answered instead.
#[derive(Debug, Clone)]
pub struct Failure {
    /// Case identifier.
    pub name: String,
    /// The suite's description of the case, when it ships one.
    pub title: Option<String>,
    /// What the suite required.
    pub expected: Expectation,
    /// What this implementation answered.
    pub got: Verdict,
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
    pub failures: Vec<Failure>,
    /// How we answered each implementation-defined case, in case order. The
    /// spec permits either answer, so none of these is a failure; publishing
    /// which way we went is the checkable half of a bare `22/35`.
    pub either_answers: Vec<(String, Verdict)>,
}

impl Report {
    /// Render the failure list for a console summary.
    ///
    /// Truncating is right for a terminal and wrong for evidence, so the
    /// complete list goes to [`publish`] and this one says where it lives.
    #[must_use]
    pub fn failure_list(&self, limit: usize) -> String {
        let mut out = String::new();
        for failure in self.failures.iter().take(limit) {
            let _ = writeln!(
                out,
                "    {}: expected {:?}, got {:?}",
                failure.name, failure.expected, failure.got
            );
        }
        if self.failures.len() > limit {
            let _ = writeln!(
                out,
                "    ... and {} more — the complete list is published in \
                 conformance/REPORT.md",
                self.failures.len() - limit
            );
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
    let mut either_answers = Vec::new();

    for case in &cases {
        let got = verdict(&case.bytes);
        let mut record = |expected| {
            failures.push(Failure {
                name: case.name.clone(),
                title: case.title.clone(),
                expected,
                got,
            });
        };
        match case.expectation {
            Expectation::MustAccept => {
                accept.total += 1;
                if got == Verdict::Accepted {
                    accept.correct += 1;
                } else {
                    record(Expectation::MustAccept);
                }
            }
            Expectation::MustReject => {
                reject.total += 1;
                if got == Verdict::Rejected {
                    reject.correct += 1;
                } else {
                    record(Expectation::MustReject);
                }
            }
            Expectation::ImplementationDefined => {
                // Counted for publication, never a failure: the spec permits
                // either answer, and gating on one would invent a requirement.
                either.total += 1;
                if got == Verdict::Accepted {
                    either.correct += 1;
                }
                either_answers.push((case.name.clone(), got));
            }
        }
    }

    Report {
        suite: suite.to_string(),
        accept,
        reject,
        either,
        failures,
        either_answers,
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

// --- Suite adapters (MASTER_PLAN §3.3) --------------------------------------
//
// These live here rather than in the test that first needed them because §3.3
// makes conformance adapters part of the harness. Two callers now read the same
// suites — the gate and the report generator — and a second copy of "how a
// suite is laid out" is a second place for the case count to drift.

/// Load JSONTestSuite's `test_parsing/` directory.
///
/// The expectation is encoded in the filename: `y_` must parse, `n_` must be
/// rejected, `i_` is implementation-defined.
///
/// # Errors
///
/// Returns the first I/O error from reading the directory or a case file. A
/// missing suite is an error and never an empty result: a conformance run that
/// silently measured nothing is exactly the vacuity invariant V4 forbids.
pub fn json_test_suite(root: &Path) -> std::io::Result<Vec<Case>> {
    let dir = root.join("test_parsing");
    let mut cases = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let expectation = if name.starts_with("y_") {
            Expectation::MustAccept
        } else if name.starts_with("n_") {
            Expectation::MustReject
        } else if name.starts_with("i_") {
            Expectation::ImplementationDefined
        } else {
            continue;
        };
        cases.push(Case {
            name: name.to_string(),
            title: None,
            bytes: std::fs::read(&path)?,
            expectation,
        });
    }
    Ok(cases)
}

/// Load yaml-test-suite's `cases/` tree.
///
/// One directory per case holding `in.yaml`, plus an `error` marker when the
/// case must be rejected and a `name` file carrying the suite's own one-line
/// description. Nested `<ID>/<NN>/` directories are multi-document cases and
/// count as separate cases.
///
/// # Errors
///
/// Returns an error when `cases/` itself is unreadable, and the first I/O error
/// from reading a case file.
pub fn yaml_test_suite(root: &Path) -> std::io::Result<Vec<Case>> {
    let cases_dir = root.join("cases");
    // Probe the root explicitly. The walk below tolerates one unreadable
    // subdirectory, so without this an absent suite would return zero cases and
    // report a clean 0/0 — the vacuity V4 forbids.
    std::fs::read_dir(&cases_dir)?;

    let mut cases = Vec::new();
    let mut stack = vec![cases_dir.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            }
        }
        let input = dir.join("in.yaml");
        if !input.is_file() {
            continue;
        }
        cases.push(Case {
            name: dir
                .strip_prefix(&cases_dir)
                .unwrap_or(&dir)
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect::<Vec<_>>()
                .join("/"),
            title: std::fs::read_to_string(dir.join("name"))
                .ok()
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty()),
            bytes: std::fs::read(&input)?,
            expectation: if dir.join("error").is_file() {
                Expectation::MustReject
            } else {
                Expectation::MustAccept
            },
        });
    }
    Ok(cases)
}

/// Read a `#! <field> = <value>` header from a fetched suite's `MANIFEST.tsv`.
///
/// The pin belongs in the published report: a pass rate is a number about the
/// suite *at some commit*, and one that does not say which commit is a number
/// about nothing.
#[must_use]
pub fn manifest_field(suite_dir: &Path, field: &str) -> Option<String> {
    let text = std::fs::read_to_string(suite_dir.join("MANIFEST.tsv")).ok()?;
    let prefix = format!("#! {field} = ");
    text.lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .map(|value| value.trim().to_string())
}

// --- The published report (MASTER_PLAN §4.1 P4) -----------------------------

/// A measured suite, with the pin the measurement describes.
#[derive(Debug)]
pub struct Publication {
    /// What the run measured.
    pub report: Report,
    /// Upstream repository URL, from the fetch receipt.
    pub repo: Option<String>,
    /// Commit the suite was pinned at, from the fetch receipt.
    pub rev: Option<String>,
}

/// Render the published conformance report.
///
/// Deterministic by construction (§9.5): suites are sorted by name, cases
/// arrive already sorted from [`run`], and no field carries a wall-clock — the
/// output is a pure function of the pinned suites and the implementation.
#[must_use]
pub fn publish(mut publications: Vec<Publication>) -> String {
    publications.sort_by(|a, b| a.report.suite.as_bytes().cmp(b.report.suite.as_bytes()));

    let mut out = String::from(
        "# Conformance — published pass rates\n\
         \n\
         <!--\n\
         GENERATED — `cargo xtask conformance-report --write`.\n\
         \n\
         Do not edit this file by hand. `gate/conformance` regenerates it and\n\
         byte-compares the result against the committed copy, so an edited number\n\
         fails the build instead of becoming a claim.\n\
         \n\
         The claim lives in `thresholds.tsv`, which ratchets one way and is covered\n\
         by `golden-guard`. This file is the measurement that has to keep meeting it.\n\
         -->\n\
         \n\
         MASTER_PLAN §4.1 **P4**: conformance pass rates published *\"including honest\n\
         failure lists\"*.\n\
         \n\
         Two rates per suite, never a blended one. ADR-008 measured what a single\n\
         figure would publish for a parser whose entire body is `return Err`: **66.4%**\n\
         on JSONTestSuite, earned by rejecting all 188 must-reject cases and parsing\n\
         nothing at all.\n\
         \n",
    );

    render_rates(&mut out, &publications);
    render_badge_values(&mut out, &publications);
    for publication in &publications {
        render_suite(&mut out, publication);
    }
    // Exactly one trailing newline. The file is byte-compared, so its shape is
    // part of the contract and not something the last section happens to decide.
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

/// The summary table: every rate this report publishes, in one place.
fn render_rates(out: &mut String, publications: &[Publication]) {
    out.push_str("## Rates\n\n");
    out.push_str("| Suite | Pinned rev | Must-accept | Must-reject | Implementation-defined |\n");
    out.push_str("|---|---|---|---|---|\n");
    for publication in publications {
        let report = &publication.report;
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            report.suite,
            publication
                .rev
                .as_deref()
                .map_or_else(|| "unpinned".to_string(), |r| format!("`{}`", short(r))),
            rate_cell(report.accept),
            rate_cell(report.reject),
            either_cell(report.either),
        );
    }
    out.push('\n');
}

/// Badge label/message pairs. Values only: the badge itself is visual design.
fn render_badge_values(out: &mut String, publications: &[Publication]) {
    out.push_str(
        "## Badge values\n\
         \n\
         §12 puts proof badges at the top of a launch README. Each badge's label and\n\
         message are generated here, so a badge can only ever show the number this run\n\
         measured. Their visual form is Ayush's (§16) and lands with the launch README.\n\
         \n\
         | Label | Message |\n\
         |---|---|\n",
    );
    for publication in publications {
        let report = &publication.report;
        for (class, tally) in [("accept", report.accept), ("reject", report.reject)] {
            let _ = writeln!(
                out,
                "| {} {class} | {:.1}% ({}/{}) |",
                report.suite,
                tally.rate() * 100.0,
                tally.correct,
                tally.total,
            );
        }
    }
    out.push('\n');
}

/// One suite's section, including its complete failure list.
fn render_suite(out: &mut String, publication: &Publication) {
    let report = &publication.report;
    let _ = writeln!(out, "## {}\n", report.suite);
    match (publication.rev.as_deref(), publication.repo.as_deref()) {
        (Some(rev), Some(repo)) => {
            let _ = writeln!(out, "Pinned at `{rev}` · {repo}\n");
        }
        (Some(rev), None) => {
            let _ = writeln!(out, "Pinned at `{rev}`\n");
        }
        _ => out.push_str("No fetch receipt: this suite's pin is unrecorded.\n\n"),
    }

    out.push_str("| Class | Correct | Rate |\n|---|---|---|\n");
    let _ = writeln!(
        out,
        "| must-accept | {}/{} | {:.1}% |",
        report.accept.correct,
        report.accept.total,
        report.accept.rate() * 100.0
    );
    let _ = writeln!(
        out,
        "| must-reject | {}/{} | {:.1}% |",
        report.reject.correct,
        report.reject.total,
        report.reject.rate() * 100.0
    );
    if report.either.total > 0 {
        let _ = writeln!(
            out,
            "| implementation-defined | {}/{} accepted | not gated |",
            report.either.correct, report.either.total
        );
    }
    out.push('\n');

    render_failures(out, report);
    render_either(out, report);
}

/// §4.1 P4's honest failure list — complete, never truncated.
fn render_failures(out: &mut String, report: &Report) {
    if report.failures.is_empty() {
        out.push_str(
            "**No failures.** Every must-accept case is accepted and every must-reject\n\
             case is rejected.\n\n",
        );
        return;
    }

    let total = report.accept.total + report.reject.total + report.either.total;
    let _ = writeln!(
        out,
        "### Honest failure list — {} of {total} cases\n",
        report.failures.len()
    );

    let refused = report
        .failures
        .iter()
        .filter(|f| f.expected == Expectation::MustAccept)
        .count();
    if refused == 0 {
        out.push_str(
            "All of them are must-reject cases this implementation accepts: input the\n\
             suite calls invalid that our parser does not yet recognise as invalid. None\n\
             is a valid document we refuse, which would be the worse failure — a refused\n\
             file is one konflux cannot help with at all.\n\n",
        );
    } else {
        let _ = writeln!(
            out,
            "**{refused} of these are valid documents this implementation refuses.** That\n\
             is the worse direction to fail in: a refused file is one konflux cannot help\n\
             with at all, where a wrongly-accepted one is at least still readable.\n"
        );
    }

    out.push_str("```text\n");
    for failure in &report.failures {
        let verdict = match failure.got {
            Verdict::Accepted => "accepted",
            Verdict::Rejected => "refused",
        };
        let expectation = match failure.expected {
            Expectation::MustAccept => "must-accept",
            Expectation::MustReject => "must-reject",
            Expectation::ImplementationDefined => "implementation-defined",
        };
        match &failure.title {
            Some(title) => {
                let _ = writeln!(out, "{} [{expectation}, {verdict}]  {title}", failure.name);
            }
            None => {
                let _ = writeln!(out, "{} [{expectation}, {verdict}]", failure.name);
            }
        }
    }
    out.push_str("```\n\n");
}

/// The cases the spec leaves open, and which way we went on each.
fn render_either(out: &mut String, report: &Report) {
    let rejected: Vec<&String> = report
        .either_answers
        .iter()
        .filter(|(_, got)| *got == Verdict::Rejected)
        .map(|(name, _)| name)
        .collect();
    if rejected.is_empty() {
        return;
    }
    let _ = writeln!(
        out,
        "### Implementation-defined cases we reject — {} of {}\n",
        rejected.len(),
        report.either.total
    );
    out.push_str(
        "The spec permits either answer, so none of these is a failure. They are listed\n\
         because an accepted-count without the list is a number nobody can check.\n\
         \n\
         ```text\n",
    );
    for name in rejected {
        let _ = writeln!(out, "{name}");
    }
    out.push_str("```\n\n");
}

/// `95/95 · 100.0%`
fn rate_cell(tally: Tally) -> String {
    format!(
        "{}/{} · {:.1}%",
        tally.correct,
        tally.total,
        tally.rate() * 100.0
    )
}

/// `22/35 accepted`, or a note that the suite has no such class.
fn either_cell(tally: Tally) -> String {
    if tally.total == 0 {
        "none in suite".to_string()
    } else {
        format!("{}/{} accepted", tally.correct, tally.total)
    }
}

/// First 12 hex characters of a commit, the length git itself abbreviates to.
fn short(rev: &str) -> &str {
    rev.get(..12).unwrap_or(rev)
}

/// Measure both fetched suites and render the published report.
///
/// One code path produces these bytes, so the generator and the gate that
/// byte-compares its output cannot drift apart.
///
/// # Errors
///
/// Returns a diagnostic when a suite is not fetched or a case cannot be read.
/// Never a skip: a report that silently measured one suite would publish a
/// smaller failure list as though the parser had improved.
pub fn publish_report<J, Y>(dir: &Path, json: J, yaml: Y) -> Result<String, String>
where
    J: Fn(&[u8]) -> Verdict,
    Y: Fn(&[u8]) -> Verdict,
{
    let json_root = dir.join("json-test-suite");
    let yaml_root = dir.join("yaml-test-suite");
    let json_cases = load("json-test-suite", &json_root, json_test_suite)?;
    let yaml_cases = load("yaml-test-suite", &yaml_root, yaml_test_suite)?;

    Ok(publish(vec![
        Publication {
            report: run("json-test-suite", json_cases, json),
            repo: manifest_field(&json_root, "repo"),
            rev: manifest_field(&json_root, "rev"),
        },
        Publication {
            report: run("yaml-test-suite", yaml_cases, yaml),
            repo: manifest_field(&yaml_root, "repo"),
            rev: manifest_field(&yaml_root, "rev"),
        },
    ]))
}

fn load(
    suite: &str,
    root: &Path,
    adapter: fn(&Path) -> std::io::Result<Vec<Case>>,
) -> Result<Vec<Case>, String> {
    adapter(root).map_err(|e| {
        format!(
            "conformance suite `{suite}` is not readable ({}): {e}\n\
             Run `conformance/fetch.sh`. This is an error rather than a skip: a\n\
             published report that quietly measured nothing is worse than none.",
            root.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{
        Case, Expectation, Publication, RatchetError, Tally, Verdict, check_ratchet, publish, run,
    };

    fn case(name: &str, expectation: Expectation) -> Case {
        Case {
            name: name.to_string(),
            title: None,
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
                title: c.title.clone(),
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
        let names: Vec<&str> = report.failures.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["y_a", "y_b", "y_c"], "failure list is not sorted");
        assert!(report.failure_list(2).contains("and 1 more"));
    }

    // --- The published report (§4.1 P4) ------------------------------------

    fn publication(suite: &str, cases: Vec<Case>, verdict: Verdict) -> Publication {
        Publication {
            report: run(suite, cases, |_| verdict),
            repo: Some(format!("https://example.invalid/{suite}")),
            rev: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
        }
    }

    #[test]
    fn the_published_list_names_every_failure() {
        // The console summary truncates at 15 and that is right for a terminal.
        // Evidence may not: "... and 62 more" is 62 failures nobody can read,
        // and P4 asks for the honest failure list, not its first page.
        let cases: Vec<Case> = (0..40)
            .map(|i| case(&format!("n_{i:03}"), Expectation::MustReject))
            .collect();
        let text = publish(vec![publication("demo", cases, Verdict::Accepted)]);
        for i in 0..40 {
            let name = format!("n_{i:03}");
            assert!(text.contains(&name), "published report omits `{name}`");
        }
        assert!(
            !text.contains("more"),
            "the published report truncated its failure list"
        );
    }

    #[test]
    fn publishing_is_deterministic_and_independent_of_suite_order() {
        let build = |suite: &str| {
            publication(
                suite,
                vec![
                    case("y_a", Expectation::MustAccept),
                    case("n_a", Expectation::MustReject),
                ],
                Verdict::Accepted,
            )
        };
        let forwards = publish(vec![build("alpha"), build("beta")]);
        let backwards = publish(vec![build("beta"), build("alpha")]);
        assert_eq!(
            forwards, backwards,
            "the published report depends on the order suites were measured in"
        );
        assert_eq!(forwards, publish(vec![build("alpha"), build("beta")]));
    }

    #[test]
    fn the_published_report_carries_no_wall_clock() {
        // §9.5: no wall-clock in outputs. This file is committed and
        // byte-compared, so a date field would make the gate fail every day.
        let text = publish(vec![publication(
            "demo",
            vec![case("y_a", Expectation::MustAccept)],
            Verdict::Accepted,
        )]);
        for banned in ["generated at", "timestamp", "modified", "UTC"] {
            assert!(
                !text.contains(banned),
                "wall-clock-derived text `{banned}` leaked into the published report"
            );
        }
        let bytes = text.as_bytes();
        let dated = bytes
            .windows(5)
            .any(|w| w.iter().take(4).all(u8::is_ascii_digit) && w.last() == Some(&b'-'));
        assert!(
            !dated,
            "something shaped like a YYYY- date reached the report"
        );
    }

    #[test]
    fn refusing_a_valid_document_is_called_out_as_the_worse_failure() {
        // Both directions are failures; they are not equally bad, and a list
        // that renders them identically hides which one we are in.
        let wrongly_accepted = publish(vec![publication(
            "demo",
            vec![case("n_a", Expectation::MustReject)],
            Verdict::Accepted,
        )]);
        assert!(wrongly_accepted.contains("must-reject cases this implementation accepts"));

        let wrongly_refused = publish(vec![publication(
            "demo",
            vec![case("y_a", Expectation::MustAccept)],
            Verdict::Rejected,
        )]);
        assert!(wrongly_refused.contains("**1 of these are valid documents"));
    }

    #[test]
    fn implementation_defined_answers_are_published_not_just_counted() {
        let text = publish(vec![publication(
            "demo",
            vec![case("i_a", Expectation::ImplementationDefined)],
            Verdict::Rejected,
        )]);
        assert!(text.contains("Implementation-defined cases we reject — 1 of 1"));
        assert!(text.contains("i_a"));
    }
}
