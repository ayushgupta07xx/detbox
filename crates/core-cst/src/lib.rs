//! # `core-cst` — lossless concrete syntax trees
//!
//! **Mission (MASTER_PLAN §3.1).** The load-bearing crate of the entire
//! ecosystem. A CST here owns *every byte* of the input: keys, values,
//! comments, whitespace, ordering, YAML anchors/aliases, quoting style, line
//! endings, trailing garbage. Every other crate in this workspace is downstream
//! of this promise.
//!
//! ## Invariants (machine-checked, permanent)
//!
//! - **K1 Round-trip** — `serialize(parse(x)) == x`, byte-identical, for every
//!   `x` in the corpus and every fuzz input that parses. This is the
//!   credibility of the whole platform; it is the first CI gate ever written
//!   and it never comes out.
//! - **K2 Edit locality** — after an edit operation, all bytes outside the
//!   edited span(s) are unchanged.
//! - **K3 Determinism** — identical input + identical operation sequence →
//!   identical output bytes, on every platform. No iteration-order leaks
//!   (`BTreeMap`/`IndexMap` only in output paths), no timestamps, stable sorts
//!   everywhere.
//!
//! ## Escape hatch for hostile input
//!
//! Anything the modelled grammar cannot represent (exotic YAML tags, weird
//! encodings) is preserved as an opaque verbatim node rather than normalised.
//! **Preserving beats understanding; K1 outranks elegance.**
//!
//! ## Status
//!
//! Phase 0 scaffold. The green/red-tree vs owned-token-tree choice is
//! **ADR-001**, reserved and unwritten — it is made after the 2-day spike at
//! the start of konflux M1 (MASTER_PLAN §3.1). Nothing in this crate may
//! presume the outcome.
//!
//! The single function below exists so that the K1 gate — golden runner, fuzz
//! target, determinism check, and miri job — is wired and **non-vacuous from
//! commit one**, before any parser exists. It is replaced, not extended, by the
//! real `parse`/`serialize` pair at M1. See ADR-003.

/// The K1 identity: a byte sequence carried through the CST boundary unchanged.
///
/// This is the degenerate case of `serialize(parse(x)) == x` for the empty
/// grammar — the grammar that models nothing and therefore preserves
/// everything as one verbatim node. It is the weakest true statement of K1 and
/// the strongest one available before a parser exists.
///
/// # Phase 0 only
///
/// At konflux M1 this is deleted and its callers (golden runner cases, fuzz
/// target `roundtrip_identity`, determinism harness) are re-pointed at the real
/// `Format::parse` / `Format::serialize` pair. If this function still exists
/// when a parser ships, that is a bug in the milestone, not a feature.
#[must_use]
pub fn roundtrip_identity(input: &[u8]) -> Vec<u8> {
    input.to_vec()
}

#[cfg(test)]
mod tests {
    use super::roundtrip_identity;

    #[test]
    fn k1_holds_on_the_empty_grammar() {
        for case in [
            &b""[..],
            b"a",
            b"key: value  # comment\n",
            b"\r\n\r\n",
            b"\xff\xfe\x00invalid utf-8",
        ] {
            assert_eq!(roundtrip_identity(case), case, "K1 violated for {case:?}");
        }
    }
}
