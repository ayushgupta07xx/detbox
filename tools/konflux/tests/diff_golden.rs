//! Structural diff golden suite — konflux **M2**, first item.
//!
//! *"Oracle first: diff golden suite — hand-built cases where line-based diff is
//! wrong and structural diff is right. Confirm red."*
//!
//! **RED by design.** `konflux::diff` is a contract with no implementation
//! behind it (§8: the oracle is written and merged first), so every case that
//! asserts a change fails. The suite README says what each case is for and
//! ADR-011 says why `expected` is the `--json` output.

// Entirely test code; clippy's allow-*-in-tests covers only `#[test]` fns.
#![allow(clippy::expect_used, clippy::panic)]

use core_formats::{Json, Yaml};
use core_verify::golden::{Pair, run_pairs_dir};
use std::path::PathBuf;

/// Every case in the suite. A shrinking suite is a weakening oracle (§8), so
/// this is asserted rather than trusted.
const EXPECTED_CASES: usize = 15;

/// Cases a diff that reports nothing must still fail. Everything except the
/// `900-identical` control — see the suite README.
const CASES_A_NULL_DIFF_FAILS: usize = 14;

fn suite() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/diff")
}

/// Run a pair through the format its extension names.
///
/// Selecting by extension, exactly as `xtask corpus-k1` does, so a case cannot
/// be silently run through the wrong parser and score a diff about nothing.
fn diff_json(pair: &Pair) -> Vec<u8> {
    let report = match pair.extension.as_str() {
        "yaml" | "yml" => konflux::diff(&Yaml, &pair.a, &pair.b),
        "json" => konflux::diff(&Json, &pair.a, &pair.b),
        other => panic!(
            "golden case uses `.{other}`, which konflux does not parse yet. \
             TOML and HCL arrive in Phase 2; a case in a format we cannot read \
             is an unrunnable case, not a pending one."
        ),
    };
    match report {
        Ok(diff) => diff.to_json().into_bytes(),
        // A refusal is a *result*, not a broken case, so it flows into the
        // byte-compare and shows up as an ordinary failure with the reason
        // printed. Panicking here would abort the suite on the first YAML case
        // and hide the eight behind it (ADR-012).
        Err(refusal) => format!("{refusal}\n").into_bytes(),
    }
}

#[test]
fn structural_diff_matches_the_golden_suite() {
    let report = run_pairs_dir(&suite(), diff_json).expect("suite runs");
    assert_eq!(
        report.total(),
        EXPECTED_CASES,
        "the diff suite has {} cases, expected {EXPECTED_CASES}. Adding cases is \
         free and this constant moves up with them; a suite that shrank is an \
         oracle that was weakened (§8).",
        report.total()
    );

    // ADR-011 recorded `UNIMPLEMENTED_CASES` as the weakest true statement
    // available while the diff did not exist, and said the constant comes out
    // when it reaches zero. It reached zero, so this is now an ordinary golden
    // gate: every case must pass, and any regression is simply red.
    report.assert_ok();
}

/// The suite must not be satisfiable by doing nothing.
///
/// This is the guard M1 learned to write. A K1 fuzz target could report success
/// having evaluated no assertion at all, and the fix was a separate test that
/// made the vacuity itself visible. A diff suite has the same failure mode in a
/// quieter form: if formatting-only differences were reported as *no* change,
/// then `010`, `020` and `030` would all be satisfied by returning `[]`, and a
/// third of the suite would be proving nothing.
///
/// So the constant below is load-bearing. If it ever drops, either a case was
/// weakened or formatting changes stopped being reported — and both are things
/// to notice deliberately rather than discover at M3.
#[test]
fn the_suite_is_not_vacuous() {
    let null_diff = |_: &Pair| konflux::DiffReport::default().to_json().into_bytes();
    let report = run_pairs_dir(&suite(), null_diff).expect("suite runs");

    assert_eq!(
        report.failures.len(),
        CASES_A_NULL_DIFF_FAILS,
        "a diff that reports nothing failed {} of {} cases, expected \
         {CASES_A_NULL_DIFF_FAILS}.\n\
         Every case except the `900-identical` control must reject the empty \
         answer. If this dropped, a case stopped asserting anything.",
        report.failures.len(),
        report.total()
    );
    assert_eq!(
        report.passed, 1,
        "exactly one case — the control — may pass a null diff"
    );
}
