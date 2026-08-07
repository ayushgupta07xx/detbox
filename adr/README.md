# Architecture Decision Records

MASTER_PLAN §9.1: every non-obvious decision gets an ADR, written in the **same
PR** as the decision. §13: the public ADR corpus is *"the 'why' trail a
rushed repo never has"* — it is a credibility artifact as much as an engineering
one.

Write each ADR as **the sentence Ayush says in an interview**. Not a changelog
entry: the reasoning, the options rejected, and the cost accepted. If an ADR
does not survive being read aloud to an interviewer, rewrite it.

Every ADR is a non-delegable human review point (§9.3). ADRs land as
`proposed` and become `accepted` only on Ayush's sign-off.

## Template (§9.4)

See [TEMPLATE.md](TEMPLATE.md).

## Numbering

**ADR-001 is RESERVED** for the `core-cst` representation choice — green/red
tree (rowan-style) vs owned token tree — made after a 2-day spike comparing edit
ergonomics and memory footprint at konflux M1 (MASTER_PLAN §3.1). Do not use
that number for anything else.

Phase 0 ADRs therefore start at 002.

## Index

| ADR | Decision | Status |
|---|---|---|
| [001](.) | `core-cst` representation: green/red tree vs owned token tree | **reserved — konflux M1** |
| [002](ADR-002-toolchain-and-msrv.md) | Pinned toolchain and MSRV 1.90 | proposed |
| [003](ADR-003-phase-0-gate-arming.md) | Phase-0 gates are wired non-vacuously, and arm on a published schedule | proposed |
| [004](ADR-004-corpus-by-pinned-fetch.md) | Corpora are fetched at pinned SHAs with exact yields, never vendored | proposed |
| [005](ADR-005-determinism-gate-mechanism.md) | Determinism = double-build + double-run output-hash compare, with an in-repo SHA-256 | proposed |
| [006](ADR-006-benchmark-baselines.md) | Benchmark baselines are named-and-uncalibrated until recorded on CI | proposed |
| [007](ADR-007-no-brand-named-artifacts.md) | No brand-named crate or binary exists until D1 | proposed |
