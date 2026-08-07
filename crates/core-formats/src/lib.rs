//! # `core-formats` — one trait, many formats
//!
//! **Mission (MASTER_PLAN §3.2).** Every format the ecosystem speaks implements
//! one trait. Products are heads on this trait; adding a format adds it to
//! every product at once. This is the mechanical form of the platform thesis.
//!
//! ```ignore
//! trait Format {
//!     fn parse(&self, input: &[u8]) -> Result<Cst, ParseReport>;   // never panics; report has spans
//!     fn serialize(&self, cst: &Cst) -> Vec<u8>;                   // total function
//!     fn semantic_view(&self, cst: &Cst) -> SemanticTree;          // typed layer for diff/merge/query
//!     fn merge_hints(&self) -> MergeHints;                         // e.g. K8s list-identity keys
//!     fn conformance_suite(&self) -> Option<SuiteAdapter>;         // official test suite hookup
//! }
//! ```
//!
//! ## Invariants
//!
//! - **F1 `parse` never panics.** Not on hostile input, not on non-UTF-8, not
//!   on truncated input. Failure is a `ParseReport` carrying spans, never an
//!   abort. Enforced by a per-format fuzz target.
//! - **F2 `serialize` is total.** Every `Cst` this crate can construct
//!   serialises. There is no "unserialisable" state.
//! - **F3 K1 composition.** For every format, `serialize(parse(x)) == x`
//!   byte-identically whenever `parse` succeeds. This crate inherits
//!   [`core-cst`]'s K1 and must not weaken it — a format that cannot round-trip
//!   some construct models that construct as a verbatim node instead.
//! - **F4 The semantic view is derived, never authoritative.** Output bytes
//!   come from the CST. A semantic view may be lossy; the CST may not.
//!
//! ## Where domain intelligence lives
//!
//! `semantic_view` and `merge_hints` are the *only* places format-specific
//! knowledge is allowed: K8s-aware list merging (match containers by `name`,
//! env vars by key — not by list position), Helm/kustomize awareness, Terraform
//! block identity. This is konflux's moat over generic tools.
//!
//! ## Rollout order (MASTER_PLAN §3.2 — do not reorder without an ADR)
//!
//! 1. **yaml, json** — Phase 1 (konflux)
//! 2. **toml, hcl** — Phase 2 (strukt)
//! 3. **csv, jsonl, logfmt** — Phase 3 (bigsheet)
//! 4. **lockfiles** — Phase 4 (lockproof): `package-lock.json`, `Cargo.lock`,
//!    `uv.lock`, `yarn.lock`, `pnpm-lock.yaml`, `go.sum`
//!
//! PDF is architecturally different (object graph + xref, not a text CST) and
//! lives in its own crate under `tools/pdfsurgeon`. It still answers to
//! [`core-verify`].
//!
//! ## Status
//!
//! Phase 0 scaffold — no trait, no implementations. The trait shape above is
//! quoted from the master plan and is not yet code, because its associated
//! types depend on **ADR-001** (the CST representation).
//!
//! [`core-cst`]: https://github.com/ayushgupta07xx/detbox/tree/main/crates/core-cst
//! [`core-verify`]: https://github.com/ayushgupta07xx/detbox/tree/main/crates/core-verify
