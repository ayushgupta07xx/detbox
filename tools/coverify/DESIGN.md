# DESIGN — `coverify`

## Scope
`core-verify` productized: a deterministic simulation testing harness —
deterministic async executor, simulated clock/network/disk fault injection,
seed-based reproduction, time-travel trace replay, and (stretch) a
linearizability checker.

## Path (MASTER_PLAN §5)
Internal harness (Phase 1) → published crate others can adopt (Phase 3) → full
product (Phase 5).

## Proof obligations — the product proves itself
| ID | Statement |
|----|-----------|
| P1 | Same seed → byte-identical execution trace, enforced by trace-hash in CI across platforms |
| P2 | Demo suite: deterministically reproduce ≥3 known historical concurrency bugs from public OSS issues, each with a one-command repro |
| P3 | The entire monorepo runs under coverify in CI — we are user zero, publicly |

## Current milestone
**Phase 0 — scaffold.** Phase 5, weeks 38–50. Every design decision made in
`crates/core-verify` before then is a product decision here later — write the ADR.
