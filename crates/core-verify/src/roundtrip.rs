//! K1 round-trip runner: `serialize(parse(x)) == x`, byte-identical.
//!
//! # Why these cases cannot be doctored
//!
//! A [`golden`][crate::golden] case is an `(input, expected)` pair, and an
//! `expected` file is editable — which is why MASTER_PLAN §8 has to forbid
//! editing it. A round-trip case has no `expected`: **the expectation is the
//! input**. The suite is a directory of raw files, each of which must reproduce
//! itself exactly.
//!
//! So the anti-reward-hacking law needs no enforcement here. There is nothing
//! to loosen. The only way to make a failing K1 case pass is to delete it —
//! which `golden-guard` catches, and which a shrinking case count makes
//! obvious.
//!
//! # What a failure means
//!
//! Serialisation is a fixed in-order walk with no format knowledge
//! (`core-cst`'s `Cst::serialize`), so a K1 failure is always a
//! `parse` failure: some input byte did not make it into a token. The runner
//! reports which byte, and whether the parse failed outright or produced a tree
//! that serialised to something else.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Bytes shown either side of the first difference.
const DIFF_CONTEXT: usize = 32;

/// Why a round-trip suite could not run at all.
#[derive(Debug)]
pub enum SuiteError {
    /// The suite directory could not be read.
    Unreadable {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error, rendered.
        cause: String,
    },
    /// The suite contains no cases. A vacuous suite is a failing suite.
    Empty {
        /// The suite directory.
        path: PathBuf,
    },
}

impl std::fmt::Display for SuiteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable { path, cause } => {
                write!(
                    f,
                    "round-trip suite unreadable: {} ({cause})",
                    path.display()
                )
            }
            Self::Empty { path } => write!(
                f,
                "round-trip suite {} contains no cases — a vacuous suite is a failing \
                 suite (core-verify invariant V4)",
                path.display()
            ),
        }
    }
}

impl std::error::Error for SuiteError {}

/// How a single case failed.
#[derive(Debug)]
pub enum Cause {
    /// `parse` returned an error: no tree was produced.
    ParseFailed(String),
    /// A tree was produced, but it did not serialise back to the input.
    NotByteIdentical {
        /// Offset of the first differing byte.
        at: usize,
        /// Input length.
        input_len: usize,
        /// Serialised length.
        output_len: usize,
        /// Windowed view of both sides at the difference.
        window: String,
    },
}

/// One case that did not round-trip.
#[derive(Debug)]
pub struct Failure {
    /// The case file.
    pub case: PathBuf,
    /// Why it failed.
    pub cause: Cause,
}

/// The outcome of running a suite.
#[derive(Debug)]
pub struct Report {
    /// Suite directory.
    pub suite: PathBuf,
    /// Cases that round-tripped.
    pub passed: usize,
    /// Cases that did not, in discovery (sorted) order.
    pub failures: Vec<Failure>,
}

impl Report {
    /// Total cases considered.
    #[must_use]
    pub fn total(&self) -> usize {
        self.passed + self.failures.len()
    }

    /// Panic with every failure rendered, if any case failed.
    ///
    /// # Panics
    ///
    /// Panics if [`Self::failures`] is non-empty.
    // Panicking IS this function's contract: it is the bridge to `#[test]`.
    #[allow(clippy::panic)]
    pub fn assert_ok(&self) {
        if self.failures.is_empty() {
            return;
        }
        let mut msg = format!(
            "\nK1 VIOLATED: {} of {} cases in {} did not round-trip.\n\n\
             serialize(parse(x)) must equal x, byte for byte (MASTER_PLAN §3.1).\n\
             Serialisation has no format knowledge, so every failure here is a parse\n\
             failure: some input byte did not reach a token.\n\n\
             These cases have no `expected` file — the input IS the expectation.\n\
             There is nothing to edit. Fix the parser (§8).\n",
            self.failures.len(),
            self.total(),
            self.suite.display()
        );
        for failure in &self.failures {
            let _ = write!(msg, "\n--- {} ---\n", failure.case.display());
            match &failure.cause {
                Cause::ParseFailed(report) => {
                    let _ = write!(msg, "parse failed:\n{report}\n");
                }
                Cause::NotByteIdentical {
                    at,
                    input_len,
                    output_len,
                    window,
                } => {
                    let _ = write!(
                        msg,
                        "first difference at byte {at} (input {input_len} bytes, \
                         serialised {output_len} bytes)\n{window}"
                    );
                }
            }
        }
        panic!("{msg}");
    }
}

/// Run every file in `suite` through `roundtrip`, requiring byte-identical output.
///
/// `roundtrip` is a closure rather than a `Format` so this crate stays free of
/// format knowledge: the harness proves things, it does not know what YAML is.
///
/// # Errors
///
/// Returns [`SuiteError`] if the suite cannot be read or contains no cases.
pub fn run_dir<F>(suite: &Path, roundtrip: F) -> Result<Report, SuiteError>
where
    F: Fn(&[u8]) -> Result<Vec<u8>, String>,
{
    let cases = discover(suite)?;
    if cases.is_empty() {
        return Err(SuiteError::Empty {
            path: suite.to_path_buf(),
        });
    }

    let mut passed = 0;
    let mut failures = Vec::new();
    for case in cases {
        let Ok(input) = std::fs::read(&case) else {
            failures.push(Failure {
                case,
                cause: Cause::ParseFailed("case file could not be read".into()),
            });
            continue;
        };
        match roundtrip(&input) {
            Err(report) => failures.push(Failure {
                case,
                cause: Cause::ParseFailed(report),
            }),
            Ok(output) if output == input => passed += 1,
            Ok(output) => {
                let at = first_difference(&input, &output);
                failures.push(Failure {
                    case,
                    cause: Cause::NotByteIdentical {
                        at,
                        input_len: input.len(),
                        output_len: output.len(),
                        window: render_window(&input, &output, at),
                    },
                });
            }
        }
    }

    Ok(Report {
        suite: suite.to_path_buf(),
        passed,
        failures,
    })
}

/// Case files under `suite`, byte-wise sorted. Sub-directories are ignored, and
/// `README.md` is documentation rather than a case.
fn discover(suite: &Path) -> Result<Vec<PathBuf>, SuiteError> {
    let entries = std::fs::read_dir(suite).map_err(|e| SuiteError::Unreadable {
        path: suite.to_path_buf(),
        cause: e.to_string(),
    })?;

    let mut cases = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| SuiteError::Unreadable {
            path: suite.to_path_buf(),
            cause: e.to_string(),
        })?;
        let path = entry.path();
        if path.is_file() && path.file_name().is_some_and(|n| n != "README.md") {
            cases.push(path);
        }
    }
    cases.sort_by(|a, b| a.as_os_str().cmp(b.as_os_str()));
    Ok(cases)
}

fn first_difference(input: &[u8], output: &[u8]) -> usize {
    input
        .iter()
        .zip(output.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| input.len().min(output.len()))
}

fn render_window(input: &[u8], output: &[u8], at: usize) -> String {
    let start = at.saturating_sub(DIFF_CONTEXT);
    format!(
        "  input:      {}\n  serialised: {}\n",
        escape(input, start),
        escape(output, start)
    )
}

fn escape(bytes: &[u8], start: usize) -> String {
    let slice = bytes.get(start..).unwrap_or(&[]);
    let mut out = String::new();
    for byte in slice.iter().take(DIFF_CONTEXT * 2) {
        match byte {
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7e => out.push(char::from(*byte)),
            other => {
                let _ = write!(out, "\\x{other:02x}");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Cause, SuiteError, run_dir};
    use std::path::{Path, PathBuf};

    fn fixture_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("core-verify-roundtrip-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn write(dir: &Path, name: &str, bytes: &[u8]) {
        std::fs::write(dir.join(name), bytes).expect("write fixture");
    }

    #[test]
    fn identity_round_trips_every_case() {
        let dir = fixture_dir("identity");
        write(&dir, "a.yaml", b"key: value\n");
        write(&dir, "b.yaml", b"\xff\xfe\x00");
        let report = run_dir(&dir, |input| Ok(input.to_vec())).expect("suite runs");
        assert_eq!(report.passed, 2);
        report.assert_ok();
    }

    #[test]
    fn a_normalising_roundtrip_is_caught() {
        // The failure this whole suite exists to catch: a "helpful" serializer
        // that rewrites CRLF, or strips a comment, or reflows indentation.
        let dir = fixture_dir("normalising");
        write(&dir, "crlf.yaml", b"a: 1\r\nb: 2\r\n");
        let report = run_dir(&dir, |input| {
            Ok(String::from_utf8_lossy(input)
                .replace("\r\n", "\n")
                .into_bytes())
        })
        .expect("suite runs");
        assert_eq!(report.passed, 0);
        match &report.failures[0].cause {
            Cause::NotByteIdentical {
                at,
                input_len,
                output_len,
                ..
            } => {
                assert_eq!(*at, 4);
                assert_eq!(*input_len, 12);
                assert_eq!(*output_len, 10);
            }
            other @ Cause::ParseFailed(_) => panic!("expected a byte mismatch, got {other:?}"),
        }
    }

    #[test]
    fn a_parse_failure_is_reported_as_such() {
        // Distinguishing "no tree" from "wrong bytes" is the difference between
        // an actionable message and a puzzle.
        let dir = fixture_dir("parse-fail");
        write(&dir, "x.yaml", b"anything");
        let report = run_dir(&dir, |_| Err("no parser yet".into())).expect("suite runs");
        match &report.failures[0].cause {
            Cause::ParseFailed(message) => assert!(message.contains("no parser yet")),
            other @ Cause::NotByteIdentical { .. } => {
                panic!("expected a parse failure, got {other:?}")
            }
        }
    }

    #[test]
    fn an_empty_suite_is_an_error_not_a_pass() {
        let dir = fixture_dir("empty");
        match run_dir(&dir, |input| Ok(input.to_vec())) {
            Err(SuiteError::Empty { .. }) => {}
            other => panic!("invariant V4 not enforced: {other:?}"),
        }
    }

    #[test]
    fn discovery_is_sorted_and_skips_the_readme() {
        let dir = fixture_dir("ordering");
        for name in ["c.yaml", "a.yaml", "b.yaml"] {
            write(&dir, name, b"x");
        }
        write(&dir, "README.md", b"not a case");
        let report = run_dir(&dir, |_| Err("fail".into())).expect("suite runs");
        assert_eq!(report.total(), 3, "README.md was counted as a case");
        let names: Vec<_> = report
            .failures
            .iter()
            .filter_map(|f| f.case.file_name().and_then(|n| n.to_str()))
            .collect();
        assert_eq!(names, ["a.yaml", "b.yaml", "c.yaml"]);
    }
}
