//! Non-vacuity guard for the K1 fuzz targets.
//!
//! # The problem this exists to solve
//!
//! A K1 fuzz target is shaped like this:
//!
//! ```ignore
//! let Ok(cst) = format.parse(data) else { return };   // parse failed: nothing to check
//! assert_eq!(format.serialize(&cst), data);           // the actual assertion
//! ```
//!
//! If `parse` never succeeds, the assertion is never reached. The target runs,
//! finds no crash, and reports **success** — having verified nothing about K1.
//!
//! That is `core-verify` invariant V4 (a vacuous suite is a failing suite), and
//! it is sharper for fuzzing than for a golden suite. A golden suite has a case
//! count that visibly shrinks. A fuzz run that asserts nothing produces exactly
//! the same output as one that asserts everything: `Done. 0 crashes.`
//!
//! A `fuzz_target!` cannot detect this itself — it sees one input at a time and
//! has no memory across the run. So the guard lives here, outside the fuzzer:
//! **every seed in a target's corpus must parse.** Seeds are drawn from the K1
//! golden cases, which are by definition inputs that must round-trip, so a seed
//! that does not parse is either a broken parser or a case that does not belong.
//!
//! # Status
//!
//! **RED.** `parse` is stubbed, so 0 of 47 seeds parse and both assertions
//! fail. When a parser lands these go green, and from then on they are what
//! stops the fuzz gate rotting into a very expensive no-op.

// Entirely test code; clippy's allow-*-in-tests covers only `#[test]` fns, and
// the helpers below are shared by both tests.
#![allow(clippy::expect_used, clippy::panic)]

use core_formats::{Format, Json, Yaml};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // crates/core-formats -> crates -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate lives two levels below the repo root")
        .to_path_buf()
}

fn files_in(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.file_name().is_some_and(|n| n != "README.md"))
        .collect();
    out.sort();
    out
}

/// Assert a fuzz target's seed corpus is populated from the golden suite, and
/// that every seed reaches the K1 assertion.
fn assert_seeds_exercise_k1<F: Format>(format: &F, target: &str, golden_suite: &str) {
    let root = repo_root();
    let seeds = files_in(&root.join("fuzz/corpus").join(target));
    let golden = files_in(
        &root
            .join("crates/core-formats/tests/golden")
            .join(golden_suite),
    );

    assert!(
        !seeds.is_empty(),
        "fuzz/corpus/{target} is empty. An unseeded K1 target explores from \
         nothing and will spend its budget rediscovering that `{{` is a byte."
    );

    // Seeds are copied from the golden cases. If a golden case is added and the
    // seed corpus is not refreshed, the fuzzer starts from a stale picture of
    // what this format has to handle.
    assert!(
        seeds.len() >= golden.len(),
        "fuzz/corpus/{target} has {} seeds but {golden_suite} has {} cases. \
         Re-seed the corpus so the fuzzer starts from every known-interesting input.",
        seeds.len(),
        golden.len()
    );

    // The actual non-vacuity check.
    let mut parsed = 0usize;
    let mut unparsed: Vec<String> = Vec::new();
    for seed in &seeds {
        let bytes = std::fs::read(seed).expect("seed is readable");
        if format.parse(&bytes).is_ok() {
            parsed += 1;
        } else {
            unparsed.push(
                seed.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string(),
            );
        }
    }

    assert_eq!(
        parsed,
        seeds.len(),
        "\nVACUOUS FUZZ TARGET: {} of {} seeds in fuzz/corpus/{target} do not parse,\n\
         so `{}_roundtrip` never reaches its K1 assertion for them and would report\n\
         success while verifying nothing.\n\n\
         Seeds come from the K1 golden cases — inputs that must round-trip, which\n\
         requires parse to succeed. A seed that does not parse is a parser bug, or\n\
         a case that does not belong in a round-trip suite.\n\n\
         Not parsing:\n  {}\n",
        unparsed.len(),
        seeds.len(),
        format.name(),
        unparsed.join("\n  ")
    );
}

#[test]
fn yaml_fuzz_seeds_exercise_k1() {
    assert_seeds_exercise_k1(&Yaml, "yaml_roundtrip", "yaml-roundtrip");
}

#[test]
fn json_fuzz_seeds_exercise_k1() {
    assert_seeds_exercise_k1(&Json, "json_roundtrip", "json-roundtrip");
}
