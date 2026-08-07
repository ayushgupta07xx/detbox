//! # `replaylab` — record/replay proxy for LLM workloads (Wing 2)
//!
//! > **GATED.** Wing 2 opens only when ≥2 Wing-1 launches are complete **and**
//! > at least one shows real adoption signal (MASTER_PLAN §6/§14). Until that
//! > gate passes, this crate stays empty. The doctrine transfers perfectly to
//! > AI-infra, but it must stand on an established brand, not launch one.
//!
//! **Pitch.** *"Make any agent run 100% reproducible. Record once, replay
//! byte-identically, forever — and know exactly what every run cost."*
//!
//! ## Deep core
//!
//! OpenAI/Anthropic-compatible local proxy; **cassette** recording of full
//! request/response streams; hash-verified byte-identical replay for tests, CI
//! and debugging; **exact-prefix caching only**; deterministic token/cost ledger
//! (arithmetic, not estimates); streaming with backpressure.
//!
//! ## The permanent ban that defines this tool
//!
//! The semantic/similarity cache is banned (Appendix C). An
//! embedding-similarity cache hit that returns a subtly wrong answer is
//! precisely the probabilistic failure this brand exists to reject. Exact
//! prefix match, or a miss.
//!
//! ## Proof obligations
//!
//! - **P1** Replay determinism — the replayed run's transcript hash equals the
//!   recorded hash, always.
//! - **P2** Passthrough transparency — proxied vs direct responses
//!   byte-identical modulo an allow-listed header set, differentially tested.
//! - **P3** Cassette schema versioned, with migration tests.
//! - **P4** Ledger exactness against provider-reported usage on golden traces.
//!
//! ## Status
//!
//! Phase 0 scaffold — no code, gate not open. Launches jointly with `cage`.
