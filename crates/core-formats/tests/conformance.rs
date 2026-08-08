//! Official conformance suites: JSONTestSuite and yaml-test-suite.
//!
//! MASTER_PLAN §3.3 and §4.1 **P4**: pass rates published as badges, *"including
//! honest failure lists"*.
//!
//! **RED, deliberately.** konflux M1 is at the oracle stage. `parse` is stubbed,
//! so `conformance/thresholds.tsv` records `unrecorded` for every rate and the
//! ratchet refuses to pass. Once a parser lands, the measured rates get recorded
//! as a reviewed change and the ratchet starts holding them.
//!
//! # Requires the suites
//!
//! ```bash
//! conformance/fetch.sh
//! ```
//!
//! A missing suite is a **failure**, not a skip. Skipping would let the
//! conformance gate quietly report nothing on a machine where the fetch never
//! ran, which is the same vacuity `core-verify` invariant V4 exists to forbid.

// Entirely test code; clippy's allow-*-in-tests covers only `#[test]` fns.
#![allow(clippy::expect_used, clippy::panic)]

use core_formats::{Format, Json, Yaml};
use core_verify::conformance::{self, Case, Expectation, Verdict};
use std::path::{Path, PathBuf};

fn conformance_dir() -> PathBuf {
    conformance::dir_from(env!("CARGO_MANIFEST_DIR"), 2)
}

fn require(dir: &Path, suite: &str) {
    assert!(
        dir.is_dir(),
        "conformance suite `{suite}` is not fetched ({} missing).\n\
         Run `conformance/fetch.sh`. This is a failure rather than a skip: a\n\
         conformance gate that silently measures nothing is worse than no gate.",
        dir.display()
    );
}

/// Verdict adapter. Today `parse` always fails, so this is always `Rejected` —
/// which is precisely why accept-rate and reject-rate are never blended.
fn verdict_of<F: Format>(format: &F) -> impl Fn(&[u8]) -> Verdict + '_ {
    move |bytes: &[u8]| {
        if format.parse(bytes).is_ok() {
            Verdict::Accepted
        } else {
            Verdict::Rejected
        }
    }
}

/// JSONTestSuite encodes the expectation in the filename:
/// `y_` must parse, `n_` must be rejected, `i_` is implementation-defined.
fn json_cases(root: &Path) -> Vec<Case> {
    let dir = root.join("test_parsing");
    let mut cases = Vec::new();
    let entries =
        std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
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
            bytes: std::fs::read(&path).expect("case is readable"),
            expectation,
        });
    }
    cases
}

/// yaml-test-suite's `data` layout: one directory per case holding `in.yaml`,
/// plus an `error` marker file when the case must be rejected. Nested
/// `<ID>/<NN>/` directories are multi-document cases and count separately.
fn yaml_cases(root: &Path) -> Vec<Case> {
    let cases_dir = root.join("cases");
    let mut cases = Vec::new();
    let mut stack = vec![cases_dir.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            }
        }
        let input = dir.join("in.yaml");
        if !input.is_file() {
            continue;
        }
        let name = dir
            .strip_prefix(&cases_dir)
            .unwrap_or(&dir)
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect::<Vec<_>>()
            .join("/");
        cases.push(Case {
            name,
            bytes: std::fs::read(&input).expect("case is readable"),
            expectation: if dir.join("error").is_file() {
                Expectation::MustReject
            } else {
                Expectation::MustAccept
            },
        });
    }
    cases
}

fn run_suite(
    suite: &str,
    cases: Vec<Case>,
    expect_cases: usize,
    verdict: impl Fn(&[u8]) -> Verdict,
) {
    assert_eq!(
        cases.len(),
        expect_cases,
        "{suite} has {} cases, expected {expect_cases}. The suite is pinned to a \
         commit, so this count is fixed — a change means the adapter is reading \
         it wrong, or the pin moved.",
        cases.len()
    );

    let report = conformance::run(suite, cases, verdict);
    println!("{}", report.summary());
    if !report.failures.is_empty() {
        println!(
            "  honest failure list (first 15 of {}):",
            report.failures.len()
        );
        print!("{}", report.failure_list(15));
    }

    match conformance::check_ratchet(&report, &conformance_dir().join("thresholds.tsv")) {
        Ok(summary) => print!("{summary}"),
        Err(e) => panic!("\n{e}\n\nMeasured: {}\n", report.summary()),
    }
}

#[test]
fn json_test_suite() {
    let root = conformance_dir().join("json-test-suite");
    require(&root, "json-test-suite");
    // 95 must-accept + 188 must-reject + 35 implementation-defined, at the
    // pinned commit. Asserted by conformance/fetch.sh as well.
    run_suite("json-test-suite", json_cases(&root), 318, verdict_of(&Json));
}

#[test]
fn yaml_test_suite() {
    let root = conformance_dir().join("yaml-test-suite");
    require(&root, "yaml-test-suite");
    // 308 must-accept + 94 must-reject, at the pinned commit.
    run_suite("yaml-test-suite", yaml_cases(&root), 402, verdict_of(&Yaml));
}
