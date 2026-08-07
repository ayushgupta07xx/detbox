//! # `coverify` — verification as a product (Wing 0)
//!
//! **Mission.** The `core-verify` harness, productized into a **deterministic
//! simulation testing** harness for other people's code.
//!
//! **Pitch.** *"The harness we used to prove every one of our tools lossless —
//! now pointed at your code. Find the 1-in-a-million bug, get a seed, replay it
//! forever."*
//!
//! ## Path (MASTER_PLAN §5)
//!
//! Internal harness (Phase 1) → published crate other Rust projects can adopt
//! (Phase 3) → full product (Phase 5): deterministic async executor, simulated
//! clock/network/disk fault injection, seed-based reproduction, time-travel
//! trace replay, and (stretch) a linearizability checker.
//!
//! ## Proof obligations — the product proves itself
//!
//! - **P1** Same seed → byte-identical execution trace, enforced by trace-hash
//!   in CI across platforms.
//! - **P2** Demo suite: deterministically reproduce ≥3 known historical
//!   concurrency bugs from public OSS issues, each with a one-command repro.
//! - **P3** The entire monorepo runs under coverify in CI — **we are user zero,
//!   publicly.**
//!
//! ## Why this is the most senior artifact in the plan
//!
//! "My internal test infrastructure was good enough to productize" is a
//! staff-engineer sentence. It is also the connective tissue that makes this
//! ecosystem a *doctrine* rather than a pile of tools.
//!
//! ## Status
//!
//! Phase 0 scaffold — no code. Phase 5 (weeks 38–50).
