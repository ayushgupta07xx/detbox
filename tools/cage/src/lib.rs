//! # `cage` — default-deny sandbox for agent-executed code (Wing 2)
//!
//! > **GATED.** Same gate as `replaylab` (MASTER_PLAN §6). Empty until it opens.
//!
//! **Pitch.** *"Let AI agents run code on your machine without trusting them.
//! Deny by default; every allowed capability is an explicit, logged decision."*
//!
//! ## Deep core
//!
//! Capability-based sandbox — WASI preview-2 for portable workloads plus
//! Landlock/seccomp/namespaces on Linux for native processes; path-jailed
//! filesystem; egress allowlists with SSRF-safe resolution; CPU/mem/time limits;
//! MCP tool-call gating; tool-call record/replay sharing `replaylab`'s cassette
//! substrate (the Wing-2 integration).
//!
//! ## Marketing law — non-negotiable
//!
//! Claim **"provable default-deny policy enforcement."** Never
//! "unescapable" or "unbreakable" (Appendix C). No honest security tool claims
//! the latter; pretending otherwise would burn the exact credibility this brand
//! is built on. Linux-first, with an explicit platform-support matrix in the
//! README — never fake cross-platform security claims.
//!
//! ## Proof obligations
//!
//! - **P1** Adversarial escape suite — a versioned battery of escape attempts
//!   (secret reads, exfil, process spawn, path traversal, symlink games, DNS
//!   rebinding) all denied, running as a permanent CI regression gate that only
//!   grows.
//! - **P2** Policy determinism — same policy + same request → same allow/deny +
//!   same audit log line.
//! - **P3** Overhead benchmarks vs raw execution, honest.
//!
//! ## Status
//!
//! Phase 0 scaffold — no code, gate not open. Launches jointly with
//! `replaylab`: *"run agents reproducibly and safely"* — one story, two tools,
//! shared cassettes.
