//! K1 round-trip suites for YAML and JSON.
//!
//! `serialize(parse(x)) == x`, byte-identical, for every case in
//! `tests/golden/<format>-roundtrip/`.
//!
//! **These are expected to be RED until a parser exists.** konflux M1 is at the
//! oracle stage: the cases, the runner and the contract are written first and
//! observed failing, then the parser is written until they pass. A test never
//! seen failing is not known to test anything (MASTER_PLAN §8).
//!
//! A case here has no `expected` file. **The input is the expectation.** There
//! is nothing to loosen and nothing to edit — the only way to make a failing
//! case pass is to delete it, which `golden-guard` catches and a shrinking case
//! count makes obvious.

// This file is entirely test code. clippy's `allow-expect-in-tests` only covers
// `#[test]`-annotated functions, and the shared helper below is not one.
#![allow(clippy::expect_used)]

use core_formats::{Format, Json, Yaml};
use core_verify::roundtrip;
use std::path::{Path, PathBuf};

fn suite(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name)
}

/// Run one format against its suite, asserting a floor on the case count so a
/// suite cannot quietly shrink its way to green.
fn run<F: Format>(format: &F, dir: &str, minimum_cases: usize) {
    let path = suite(dir);
    let report = roundtrip::run_dir(&path, |input| {
        format
            .parse(input)
            .map(|cst| format.serialize(&cst))
            .map_err(|report| report.to_string())
    })
    .expect("suite is readable and non-empty");

    assert!(
        report.total() >= minimum_cases,
        "{dir} has {} cases, expected at least {minimum_cases}. \
         Cases are evidence: removing one needs [NEEDS-AYUSH-APPROVAL] (§8).",
        report.total()
    );
    report.assert_ok();
}

#[test]
fn yaml_k1_round_trip() {
    // 31 cases: 17 covering constructs the corpus survey found in real files,
    // 12 covering constructs the YAML spec allows but our corpus happens not to
    // contain, and 2 minimised from K1 violations the corpus itself produced.
    // K1 is a claim about YAML, not about our corpus.
    run(&Yaml, "yaml-roundtrip", 31);
}

#[test]
fn json_k1_round_trip() {
    // 18 cases. JSON has no corpus yet — the fetched corpus is YAML and HCL —
    // so these are derived from the grammar and from what normalising JSON
    // libraries destroy: key order, duplicate keys, number spelling, escape
    // form, whitespace.
    run(&Json, "json-roundtrip", 18);
}
