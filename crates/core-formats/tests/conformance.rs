//! Official conformance suites: JSONTestSuite and yaml-test-suite.
//!
//! MASTER_PLAN §3.3 and §4.1 **P4**: pass rates published as badges, *"including
//! honest failure lists"*.
//!
//! **Armed at konflux M1.** Both parsers exist, all four rates are recorded in
//! `conformance/thresholds.tsv`, and the ratchet now holds them: a rate that
//! drops fails here, and lowering the recorded claim needs sign-off (§8).
//! `conformance/REPORT.md` is the publication, regenerated and byte-compared by
//! the last test in this file.
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
use core_verify::conformance::{self, Case, Verdict};
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

/// Verdict adapter: a tree means accepted, an error means refused. ADR-008 puts
/// the line at structure — a spec violation is refused, an unmodelled construct
/// is kept verbatim.
fn verdict_of<F: Format>(format: &F) -> impl Fn(&[u8]) -> Verdict + '_ {
    move |bytes: &[u8]| {
        if format.parse(bytes).is_ok() {
            Verdict::Accepted
        } else {
            Verdict::Rejected
        }
    }
}

/// Everything we accept must round-trip.
///
/// K1 and conformance are not independent properties. A document this
/// implementation *accepts* is one it claims to understand, and a lossless
/// kernel that accepts a document it cannot reproduce byte-for-byte has broken
/// its central promise on real, third-party input rather than on our own
/// hand-written cases.
///
/// This runs over every accepted case in the suite, which for JSONTestSuite is
/// 95 valid documents plus whatever implementation-defined cases we take — a
/// broader K1 corpus than our 18 golden cases, and one we did not write.
fn assert_accepted_cases_round_trip<F: Format>(format: &F, cases: &[Case]) -> usize {
    let mut checked = 0usize;
    for case in cases {
        let Ok(cst) = format.parse(&case.bytes) else {
            continue;
        };
        let serialised = format.serialize(&cst);
        assert_eq!(
            serialised, case.bytes,
            "K1 VIOLATED on accepted conformance case `{}`: \
             this implementation accepted a document it cannot reproduce",
            case.name
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "no conformance case was accepted, so K1 was never exercised here"
    );
    checked
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
    let cases = conformance::json_test_suite(&root).expect("suite is readable");
    let checked = assert_accepted_cases_round_trip(&Json, &cases);
    println!("K1 verified on {checked} accepted JSONTestSuite documents");
    run_suite("json-test-suite", cases, 318, verdict_of(&Json));
}

#[test]
fn yaml_test_suite() {
    let root = conformance_dir().join("yaml-test-suite");
    require(&root, "yaml-test-suite");
    // 308 must-accept + 94 must-reject, at the pinned commit.
    let cases = conformance::yaml_test_suite(&root).expect("suite is readable");
    run_suite("yaml-test-suite", cases, 402, verdict_of(&Yaml));
}

/// The published pass rates must be the ones this run measures — **P4**.
///
/// `conformance/REPORT.md` is the only place konflux's conformance numbers are
/// written down where they outlive a CI log, and the only place the failure
/// list appears in full rather than as "the first 15 of 77". A published number
/// nothing re-derives is a number that drifts, so this regenerates the file
/// from the pinned suites and byte-compares.
///
/// It closes both directions at once. Improve the parser without regenerating
/// and the published rate is stale; hand-edit the published rate and it stops
/// matching the parser. Either way the gate is red, and neither can be fixed by
/// editing this file — the fix is `cargo xtask conformance-report --write`.
#[test]
fn the_published_report_is_what_this_run_measures() {
    let dir = conformance_dir();
    require(&dir.join("json-test-suite"), "json-test-suite");
    require(&dir.join("yaml-test-suite"), "yaml-test-suite");

    let measured = conformance::publish_report(&dir, verdict_of(&Json), verdict_of(&Yaml))
        .unwrap_or_else(|e| panic!("{e}"));

    let path = dir.join("REPORT.md");
    let published = std::fs::read_to_string(&path).unwrap_or_default();

    if published == measured {
        println!(
            "published report matches this run ({} bytes, {})",
            measured.len(),
            path.display()
        );
        return;
    }

    let at = published
        .lines()
        .zip(measured.lines())
        .position(|(a, b)| a != b);
    let detail = at.map_or_else(
        || {
            format!(
                "identical for the first {} line(s), then one side ends: \
                 published {} lines, measured {} lines",
                published.lines().count().min(measured.lines().count()),
                published.lines().count(),
                measured.lines().count()
            )
        },
        |line| {
            format!(
                "first difference at line {}:\n    published: {}\n    measured:  {}",
                line + 1,
                published.lines().nth(line).unwrap_or("<absent>"),
                measured.lines().nth(line).unwrap_or("<absent>"),
            )
        },
    );

    panic!(
        "\nPUBLISHED CONFORMANCE REPORT IS STALE\n\
         \n\
         {}\n\
         {detail}\n\
         \n\
         konflux P4 publishes conformance pass rates \"including honest failure\n\
         lists\". This file is that publication, and it no longer describes what\n\
         the parser does.\n\
         \n\
         Regenerate it — never hand-edit it:\n\
         \n\
         \x20   conformance/fetch.sh && cargo xtask conformance-report --write\n\
         \n\
         Then read the diff. A rate that moved down is a regression the ratchet in\n\
         thresholds.tsv should also have caught; a rate that moved up is a\n\
         threshold worth raising in the same PR.\n",
        path.display()
    );
}
