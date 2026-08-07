//! # `lockproof` — lockfile intelligence + provenance verification
//!
//! **Mission.** Make dependency updates legible and verifiable.
//!
//! **Pitch.** *"Know exactly what changed when your lockfile did: every new
//! transitive dep, loosened pin, install script, and unsigned package — before
//! you merge."*
//!
//! ## The strongest crossover in the ecosystem
//!
//! Lockfiles are just JSON/TOML/YAML. **lockproof is a konflux head plus a
//! verification layer** — the clearest demonstration that the kernel is real.
//!
//! ## Scope
//!
//! **MVP:** structural, semantic lockfile diff for npm (`package-lock.json`) +
//! `Cargo.lock` + `uv.lock`, as a CLI and a GitHub Action that comments on PRs:
//! *"+3 transitive deps (list), 1 version pin loosened (^ → \*), +1 preinstall
//! script, 2 packages without provenance."*
//! **Full:** yarn/pnpm/`go.sum`; Sigstore signature + SLSA provenance + hash
//! verification (pure pass/fail — cryptographic, binary, provable); policy
//! gates ("fail CI if any new install scripts").
//!
//! ## Non-goals (permanent)
//!
//! Typosquat heuristics, "suspiciousness" scores, behavioural ML — **anything
//! that can cry wolf** (Appendix C). lockproof states facts and verifies
//! signatures; it never guesses.
//!
//! ## Proof obligations
//!
//! - **P1** Parse fidelity on the top-1,000 real lockfiles per ecosystem — zero
//!   parse failures, K1 round-trip.
//! - **P2** Verification differential vs reference implementations (cosign,
//!   npm/cargo native checks) — full agreement or triaged divergence.
//! - **P3** Diff completeness golden suite: seeded lockfile changes must each
//!   surface exactly once, correctly classified.
//! - **P4** Deterministic reports.
//!
//! ## Status
//!
//! Phase 0 scaffold — no code. Phase 4. Distribution advantage: it lives inside
//! CI, where the free tier is the natural habitat.
