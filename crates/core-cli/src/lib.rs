//! # `core-cli` — one feel across every tool
//!
//! **Mission (MASTER_PLAN §3.4).** The conventions every binary in this
//! ecosystem obeys, implemented once. A user who learns one tool has learned
//! the argument surface of all nine.
//!
//! ## The law every tool obeys
//!
//! - **`--json`** — stable, machine-readable output. The schema is *versioned*;
//!   a field is never removed or repurposed without a version bump and a
//!   migration note. Breaking it is a public API change (§9.3: human review).
//! - **`--check`** — exit-code-only mode for CI. No stdout noise, no colour,
//!   meaningful exit codes.
//! - **Deterministic output ordering** — always. Two runs on the same input
//!   produce the same bytes, in the same order, on every platform.
//! - **`NO_COLOR` respected** — and colour is off whenever stdout is not a TTY.
//! - **No network access, ever** — unless the tool has an explicit `--online`
//!   flag *and* its README explains why. Offline by default is a brand promise
//!   (§10), not a default setting. Telemetry and phone-home are permanently
//!   banned (Appendix C).
//! - **Span-rich diagnostics** — miette-style: show the bytes, point at the
//!   problem, suggest the fix. A byte offset with no context is not a
//!   diagnostic.
//!
//! ## Invariants
//!
//! - **C1 `--json` is append-only within a schema version.** Enforced by a
//!   golden suite over serialised output.
//! - **C2 Exit codes are a contract.** `0` success, `1` a finding the user
//!   asked about (conflict, divergence, lossy conversion), `2` usage error,
//!   `>2` internal failure. `--check` distinguishes "clean" from "found
//!   something" without parsing stdout.
//! - **C3 Nothing here reads the clock, the locale, the environment's random
//!   state, or the network.** Determinism is a property of the shared layer, so
//!   no tool has to re-earn it.
//! - **C4 Errors carry spans, not strings.** Diagnostics are structured values
//!   that render to human, `--json`, and markdown.
//!
//! ## Status
//!
//! Phase 0 scaffold. First real content lands at konflux M2 (side-by-side diff
//! output) and M4 (`--check` CI mode).
