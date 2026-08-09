//! # `konflux` — structural diff & 3-way merge for configs
//!
//! > **FLAGSHIP** — Decision D3, made 2026-08-09 (ADR-010). It was decided
//! > *without* the §11 validation read, so "flagship" here means "the product
//! > being built first" and never "the product whose demand was measured".
//! > Nothing public may imply otherwise.
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
//! **M1 complete** — `core-cst` per ADR-001, YAML and JSON parse/serialize, K1
//! green on 1,000 corpus files, conformance published (ADR-009).
//!
//! **M2 in progress — structural diff.** [`diff()`] is the contract and returns
//! no changes: the golden suite is written and merged before the implementation
//! (§8), so it is red on purpose. See [`mod@diff`] and ADR-011 for why the
//! golden `expected` files are the tool's `--json` output, not its rendering.

pub mod diff;

pub use diff::{Change, ChangeKind, DiffError, DiffReport, Significance, diff};
