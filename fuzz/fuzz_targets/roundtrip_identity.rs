//! K1: `serialize(parse(x)) == x`, byte-identical.
//!
//! MASTER_PLAN §3.1: *"This is the credibility of the whole platform; it is the
//! first CI gate ever written and it never comes out."* This target is that
//! gate, wired at Phase 0 against the empty grammar so the fuzz job is real
//! before a parser exists.
//!
//! At konflux M1 the body is re-pointed at the YAML and JSON `parse`/`serialize`
//! pair and this file splits into one target per format per operation. The
//! assertion does not change.
//!
//! Per MASTER_PLAN §3.3, any K1 violation found here is minimised and filed as
//! a failing golden case under `crates/core-verify/tests/golden/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let out = core_cst::roundtrip_identity(data);
    assert_eq!(
        out, data,
        "K1 VIOLATED: serialize(parse(x)) != x for a {}-byte input",
        data.len()
    );
});
