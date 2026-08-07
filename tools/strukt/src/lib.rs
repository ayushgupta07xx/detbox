//! # `strukt` — deterministic query & edit for every config format
//!
//! **Mission.** The yq/jq category, rebuilt on a lossless kernel.
//!
//! **Pitch.** *"Query and edit YAML/JSON/TOML/HCL from the command line —
//! without destroying comments, key order, or formatting."* Today's tools
//! normalise the whole file to change one value; strukt touches only what you
//! asked.
//!
//! ## Scope
//!
//! **MVP:** path query language (jq-inspired, small and boring on purpose),
//! get/set/delete/insert, in-place edit with K2 edit-locality.
//! **Full:** structural grep across repos, format-aware bulk refactors ("bump
//! image tag across 40 charts"), shell completions, editor integration.
//!
//! **Non-goals:** a Turing-complete query language; jq's full feature surface.
//!
//! ## Proof obligations
//!
//! - **P1** K2 edit-locality — bytes outside the edited span byte-identical,
//!   fuzz-verified.
//! - **P2** Differential vs jq on JSON: semantically identical query results on
//!   a 10k-query corpus.
//! - **P3** Idempotence: applying the same edit twice == applying it once.
//! - **P4** Determinism across platforms.
//!
//! ## Why it ships second
//!
//! ~90% kernel reuse. It is the public proof of the platform thesis — "second
//! product in four weeks *because* of the kernel" — and the single most
//! daily-useful tool in the set for working DevOps engineers.
//!
//! ## Status
//!
//! Phase 0 scaffold — no code. Phase 2 (weeks 10–14).
