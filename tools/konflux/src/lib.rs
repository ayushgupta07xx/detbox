//! # `konflux` — structural diff & 3-way merge for configs
//!
//! > **FLAGSHIP, pending Decision D3** (MASTER_PLAN §16). The flagship is
//! > confirmed by Ayush after the Phase 0 validation read, not by this crate.
//! > If the signal is weak, bigsheet takes the flagship slot and konflux slides
//! > to Phase 4 (§11).
//!
//! **Mission.** End line-based merge for structured config.
//!
//! **Pitch.** *"Git merges that finally understand your YAML. Structural 3-way
//! merge and diff for Kubernetes, Terraform, and Helm configs — comments and
//! key order preserved, zero false conflicts."*
//!
//! ## Scope
//!
//! **MVP:** YAML + JSON structural diff and 3-way merge, shipped as a git
//! merge-driver + `git mergetool` integration, with the byte-identical
//! round-trip guarantee.
//! **Full:** TOML + HCL; Kubernetes semantic merging (list-by-key);
//! Helm/kustomize awareness; TUI conflict resolver; `--check` CI mode.
//!
//! **Non-goals:** general text merge (delegate to git); auto-resolving semantic
//! conflicts it cannot prove safe; any AI-assisted resolution (banned,
//! Appendix C).
//!
//! ## Proof obligations — these ARE the definition of done
//!
//! - **P1 Round-trip** — K1 holds on a corpus of ≥1,000 real-world files plus
//!   ≥72 cumulative hours of fuzzing, zero violations.
//! - **P2 Merge algebra** (property tests) — `merge(A,A,A)=A` ·
//!   `merge(Base,X,Base)=X` · `merge(Base,Base,X)=X` · stability (same inputs →
//!   same output bytes) · conflict symmetry (swapping ours/theirs swaps
//!   conflict sides, never changes *what* conflicts).
//! - **P3 Soundness gate** — **zero incorrect merges** on a golden suite of
//!   ≥2,000 triples. Anything uncertain must surface as a conflict.
//!   Auto-resolution rate vs `git merge-file`/diff3 and vs Mergiraf is measured
//!   and published, but never bought with soundness.
//! - **P4 Conformance** — yaml-test-suite, JSONTestSuite, toml-test pass rates
//!   published as badges, including honest failure lists.
//!
//! ## Status
//!
//! Phase 0 scaffold — no code. Next: **M1**, CST + K1 for YAML/JSON, which is
//! blocked on ADR-001 (CST representation) and does not start until Ayush
//! accepts Phase 0.
