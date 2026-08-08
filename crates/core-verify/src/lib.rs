//! # `core-verify` — the proof harness (Wing 0's seed)
//!
//! **Mission (MASTER_PLAN §3.3).** A library plus internal CLI providing,
//! uniformly to every crate: golden runner, round-trip fuzzer, property kit,
//! differential runner, conformance adapters, report emitter. This crate is the
//! spine: it proves everything else. It grows into `coverify`, a product
//! (Phase 5), and `veritas` is its report emitter productized (§4.5).
//!
//! ## Invariants
//!
//! - **V1 Golden files are evidence, not code.** The runner reads them; nothing
//!   in this workspace writes them. They change only by human decision, under a
//!   `[NEEDS-AYUSH-APPROVAL]` header (MASTER_PLAN §8, the anti-reward-hacking
//!   law). There is deliberately no `--bless` / `--update-goldens` flag, and
//!   adding one requires an ADR.
//! - **V2 Failure output is actionable.** A failure prints the case path, the
//!   first differing byte offset, and a windowed hex/ASCII view of both sides.
//!   "assertion failed: left == right" is not a diagnostic.
//! - **V3 The harness is itself deterministic.** Case discovery is sorted
//!   byte-wise (never filesystem order), reports are ordered, and no wall-clock
//!   value reaches any report field.
//! - **V4 A vacuous suite is a failing suite.** Running a golden directory that
//!   contains zero cases is an error, not a pass. Silent zero-coverage is how a
//!   proof gate rots.
//!
//! ## Surfaces (MASTER_PLAN §3.3 — built out across Phase 1+)
//!
//! | Surface | Status |
//! |---|---|
//! | Golden runner | **Phase 0: implemented (minimal)** — this module |
//! | Round-trip fuzzer (`fuzz/`) | Phase 0: one target wired, K1 identity |
//! | Property kit (proptest strategies, merge algebra §4) | konflux M3 |
//! | Differential runner (`git merge-file`, Mergiraf, jq, DuckDB, cosign, qpdf) | konflux M2 |
//! | Conformance adapters (yaml-test-suite, JSONTestSuite, toml-test, veraPDF) | konflux M1 |
//! | Report emitter (human / JSON / markdown badge) | konflux M4 |
//!
//! ## Status
//!
//! Phase 0 scaffold. Only the golden runner exists, minimally, so that the
//! §8 golden gate is wired and non-vacuous from commit one (ADR-003).

pub mod golden;
pub mod roundtrip;
