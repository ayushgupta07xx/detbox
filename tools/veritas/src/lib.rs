//! # `veritas` — the converter that proves what it kept
//!
//! **Mission.** Convert files and *tell the truth about it*.
//!
//! **Pitch.** *"Convert files locally — with a receipt proving exactly what
//! survived and what, if anything, was lost."*
//!
//! ## The product IS the report
//!
//! veritas re-parses its own output, structurally diffs it against the input's
//! semantic tree, and emits a structured receipt: `perfect` |
//! `lossy-with-itemized-losses` | `refused`. A conversion pair is admitted only
//! when a **decidable** fidelity report is possible — depth over breadth,
//! permanently.
//!
//! This is `core-verify`'s report emitter productized (§3.3).
//!
//! ## Scope
//!
//! **MVP:** the format pairs the kernel already speaks — yaml↔json↔toml,
//! csv↔parquet↔xlsx via bigsheet's engine, markdown→pdf via pdfsurgeon.
//! **Full:** more pairs strictly as kernel coverage grows; a public "fidelity
//! matrix" page that doubles as marketing.
//!
//! **Non-goals:** "convert anything" breadth; media/codec formats (patent trap
//! — banned, Appendix C).
//!
//! ## Proof obligations
//!
//! - **P1** Round-trip on every pair claiming lossless.
//! - **P2** Report-completeness golden suite: seeded known-loss inputs must
//!   produce exactly the expected loss items — **the report itself is under
//!   test**.
//! - **P3** Determinism: same input → same output bytes + same receipt.
//!
//! ## Status
//!
//! Phase 0 scaffold — no code. Phase 4, ordered against pdfsurgeon by
//! **Decision D5** (Ayush's).
