//! K1 and F1 for YAML: `serialize(parse(x)) == x`, and `parse` never panics.
//!
//! MASTER_PLAN §3.1: *"This is the credibility of the whole platform; it is the
//! first CI gate ever written and it never comes out."*
//!
//! # The two properties
//!
//! **F1 — `parse` never panics.** Checked on every single input, including the
//! ones that fail to parse. This is the property the fuzzer is uniquely good at:
//! it will find the truncated multi-byte sequence, the 4 MB of nested brackets,
//! and the byte that makes an index calculation wrap, and none of them may
//! abort. F1 holds today, trivially, because there is no parser.
//!
//! **K1 — round-trip.** Checked only when `parse` succeeds, because there is no
//! tree to serialise otherwise.
//!
//! # Why this target cannot report success yet
//!
//! With `parse` stubbed to always fail, the K1 assertion below is **never
//! reached**. The target would run for 72 hours, assert nothing, and report
//! green — and a vacuous fuzz run is indistinguishable from a productive one
//! from the outside. That is the failure mode `core-verify` invariant V4 names,
//! and it is worse here than in a golden suite because there is no case count to
//! notice shrinking.
//!
//! The guard is not in this file — a `fuzz_target!` sees one input at a time and
//! cannot know whether it ever reached the interesting branch. It lives in
//! `core-formats`' `fuzz_seeds_are_not_vacuous` test, which asserts that the
//! seed corpus contains inputs this format can actually parse. That test is red
//! until a parser exists, and it is what makes this target's eventual green
//! mean something.
//!
//! # When a violation is found
//!
//! Per §3.3: minimise it, file it as a case under
//! `crates/core-formats/tests/golden/yaml-roundtrip/`, then fix the parser. The
//! case stays forever — that is what turns a one-off crash into a regression
//! gate. Never weaken the assertion (§8).

#![no_main]

use core_formats::{Format, Yaml};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // F1: this call must not panic for any `data` whatsoever. Reaching the next
    // line at all is the assertion.
    let Ok(cst) = Yaml.parse(data) else {
        return;
    };

    // K1: every input byte reached a token, and serialisation put them back in
    // order. Serialisation has no format knowledge, so a failure here is always
    // a parse bug.
    let serialised = Yaml.serialize(&cst);
    assert_eq!(
        serialised,
        data,
        "K1 VIOLATED for a {}-byte yaml input: serialize(parse(x)) != x",
        data.len()
    );
});
