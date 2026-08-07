//! # `bigsheet` — the offline large-data workbench
//!
//! **Mission.** Kill the 1,048,576-row wall.
//!
//! **Pitch.** *"Open a 5 GB CSV instantly and query it like a spreadsheet —
//! offline, and it never silently drops a row."*
//!
//! > **Designated pivot flagship.** If the Phase 0 validation signal for
//! > konflux is weak, bigsheet takes the flagship slot (§11). That call is
//! > Decision D3 and belongs to Ayush.
//!
//! ## Scope
//!
//! **MVP:** open CSV/Parquet/XLSX/JSONL far past Excel's limits; virtualised
//! grid (Tauri, Decision D2) with instant scroll; SQL pane (embedded DuckDB);
//! exact row/column counts always visible — the **never-silently-truncate
//! guarantee** is the anti-Excel brand promise.
//! **Full:** logfmt + structured-log ingestion (this *absorbs* the "log query
//! engine" idea as input formats — we do not build a second query engine),
//! lossless CSV↔Parquet↔XLSX conversion with typed fidelity reports (veritas
//! integration), saved views, joins across files, `bigsheet-cli`.
//!
//! **Non-goals:** charting suites, collaboration/cloud, competing with Excel on
//! formula breadth.
//!
//! ## Proof obligations
//!
//! - **P1 Exactness** — SQL results differentially tested against DuckDB CLI
//!   and golden hand-computed queries; any divergence is a release blocker.
//! - **P2 Fidelity** — CSV↔Parquet round-trip preserves values + types, or says
//!   exactly what changed (the report is golden-tested); multi-dialect CSV
//!   parsing validated on a nasty-real-world corpus.
//! - **P3 No-truncation** — property tests: rows in == rows reported, always.
//! - **P4 Performance *budgets*** — honest framing: budgets, not proofs.
//!   Open-time and scroll-latency targets on named public datasets (NYC Taxi,
//!   Stack Overflow dump), published with methodology.
//!
//! ## Status
//!
//! Phase 0 scaffold — no code. Phase 3 (weeks 14–26).
