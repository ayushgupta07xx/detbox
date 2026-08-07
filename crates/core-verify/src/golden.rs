//! Golden-file runner.
//!
//! A golden suite is a directory of case directories. Each case directory holds
//! an `input` file and an `expected` file, both read as raw bytes:
//!
//! ```text
//! tests/golden/<suite>/
//!   ├── 001-empty/          input  expected
//!   ├── 002-crlf/           input  expected
//!   └── 003-non-utf8/       input  expected
//! ```
//!
//! Cases are discovered in byte-wise sorted order (invariant V3), never
//! filesystem order. An empty suite is an error (invariant V4).
//!
//! **There is no blessing mode.** See [`crate`] invariant V1.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Byte window shown on either side of the first difference, in [`render_diff`].
const DIFF_CONTEXT: usize = 24;

/// Why a golden suite could not be run at all (distinct from a case failing).
#[derive(Debug)]
pub enum SuiteError {
    /// The suite directory does not exist or could not be read.
    Unreadable {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error, rendered.
        cause: String,
    },
    /// A case directory is missing `input` or `expected`.
    MalformedCase {
        /// The case directory.
        path: PathBuf,
        /// Which file was missing.
        missing: &'static str,
    },
    /// The suite contains no cases. Invariant V4: vacuous is not passing.
    Empty {
        /// The suite directory.
        path: PathBuf,
    },
}

impl std::fmt::Display for SuiteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable { path, cause } => {
                write!(f, "golden suite unreadable: {} ({cause})", path.display())
            }
            Self::MalformedCase { path, missing } => write!(
                f,
                "golden case {} is missing its `{missing}` file",
                path.display()
            ),
            Self::Empty { path } => write!(
                f,
                "golden suite {} contains no cases — a vacuous suite is a failing suite \
                 (core-verify invariant V4)",
                path.display()
            ),
        }
    }
}

impl std::error::Error for SuiteError {}

/// One case that did not match its golden.
#[derive(Debug)]
pub struct Failure {
    /// The case directory.
    pub case: PathBuf,
    /// Rendered, actionable diff (invariant V2).
    pub diff: String,
}

/// The outcome of running a suite.
#[derive(Debug)]
pub struct Report {
    /// Suite directory that was run.
    pub suite: PathBuf,
    /// Number of cases that matched.
    pub passed: usize,
    /// Cases that did not match, in discovery (sorted) order.
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
    /// Intended to be the last line of a `#[test]`. This is the only place in
    /// the crate that panics, and it panics only in test context.
    ///
    /// # Panics
    ///
    /// Panics if [`Self::failures`] is non-empty.
    // Panicking IS this function's contract: it is the bridge between the
    // harness and `#[test]`, and a test reports failure by unwinding.
    #[allow(clippy::panic)]
    pub fn assert_ok(&self) {
        if self.failures.is_empty() {
            return;
        }
        let mut msg = format!(
            "\n{} of {} golden cases failed in {}\n\n\
             Reminder (MASTER_PLAN §8): goldens are evidence. Do not edit them to go green.\n\
             If a golden is genuinely wrong, stop and propose the change under a\n\
             [NEEDS-AYUSH-APPROVAL] header.\n",
            self.failures.len(),
            self.total(),
            self.suite.display()
        );
        for failure in &self.failures {
            let _ = write!(
                msg,
                "\n--- {} ---\n{}",
                failure.case.display(),
                failure.diff
            );
        }
        panic!("{msg}");
    }
}

/// Run every case in `suite`, applying `transform` to each `input` and
/// comparing the result byte-for-byte against `expected`.
///
/// # Errors
///
/// Returns [`SuiteError`] if the suite cannot be read, a case is malformed, or
/// the suite is empty.
pub fn run_dir<F>(suite: &Path, transform: F) -> Result<Report, SuiteError>
where
    F: Fn(&[u8]) -> Vec<u8>,
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
        let input = read(&case, "input")?;
        let expected = read(&case, "expected")?;
        let actual = transform(&input);
        if actual == expected {
            passed += 1;
        } else {
            failures.push(Failure {
                case,
                diff: render_diff(&expected, &actual),
            });
        }
    }
    Ok(Report {
        suite: suite.to_path_buf(),
        passed,
        failures,
    })
}

/// Case directories under `suite`, byte-wise sorted (invariant V3).
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
        if path.is_dir() {
            cases.push(path);
        }
    }
    // Sort on the OS-string bytes: identical ordering on every platform and in
    // every locale. `sort` (not `sort_unstable`) per MASTER_PLAN §9.5.
    cases.sort_by(|a, b| a.as_os_str().cmp(b.as_os_str()));
    Ok(cases)
}

fn read(case: &Path, name: &'static str) -> Result<Vec<u8>, SuiteError> {
    std::fs::read(case.join(name)).map_err(|_| SuiteError::MalformedCase {
        path: case.to_path_buf(),
        missing: name,
    })
}

/// Render a byte-level diff anchored at the first difference (invariant V2).
fn render_diff(expected: &[u8], actual: &[u8]) -> String {
    let at = expected
        .iter()
        .zip(actual.iter())
        .position(|(e, a)| e != a)
        .unwrap_or_else(|| expected.len().min(actual.len()));

    let start = at.saturating_sub(DIFF_CONTEXT);
    let mut out = format!(
        "first difference at byte {at} (expected {} bytes, got {} bytes)\n",
        expected.len(),
        actual.len()
    );
    let _ = write!(
        out,
        "  expected: {}\n  actual:   {}\n",
        window(expected, start),
        window(actual, start)
    );
    out
}

/// A printable window of `bytes` beginning at `start`, escaping non-printables.
fn window(bytes: &[u8], start: usize) -> String {
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
    use super::{SuiteError, run_dir};
    use std::path::Path;

    fn fixtures() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn identity_suite_passes() {
        let suite = fixtures().join("tests/golden/roundtrip-identity");
        let report = run_dir(&suite, <[u8]>::to_vec).expect("suite runs");
        assert!(
            report.total() >= 4,
            "suite shrank: {} cases",
            report.total()
        );
        report.assert_ok();
    }

    #[test]
    fn a_wrong_transform_is_caught() {
        // The runner must be able to fail. A gate that cannot go red is not a gate.
        let suite = fixtures().join("tests/golden/roundtrip-identity");
        let report = run_dir(&suite, |_| b"wrong".to_vec()).expect("suite runs");
        assert_eq!(report.passed, 0);
        assert_eq!(report.failures.len(), report.total());
        assert!(report.failures[0].diff.contains("first difference at byte"));
    }

    #[test]
    fn an_empty_suite_is_an_error_not_a_pass() {
        // Created here rather than checked in: git cannot track an empty directory,
        // and a `.gitkeep` inside would make the suite non-empty.
        let suite = std::env::temp_dir().join("core-verify-empty-suite-fixture");
        std::fs::create_dir_all(&suite).expect("create fixture dir");
        match run_dir(&suite, <[u8]>::to_vec) {
            Err(SuiteError::Empty { .. }) => {}
            other => panic!("invariant V4 not enforced: {other:?}"),
        }
    }

    #[test]
    fn discovery_order_is_sorted_not_filesystem_order() {
        let suite = fixtures().join("tests/golden/roundtrip-identity");
        let report = run_dir(&suite, |_| b"wrong".to_vec()).expect("suite runs");
        let names: Vec<_> = report.failures.iter().map(|f| f.case.clone()).collect();
        let mut sorted = names.clone();
        sorted.sort_by(|a, b| a.as_os_str().cmp(b.as_os_str()));
        assert_eq!(names, sorted, "invariant V3: discovery must be sorted");
    }
}
