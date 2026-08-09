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

ADR-001 was reserved for the `core-cst` representation choice and is now
written, so numbering is contiguous from here. Phase 0's ADRs start at 002
because 001 was held open for it.

## Index

| ADR | Decision | Status |
|---|---|---|
| [001](ADR-001-cst-representation.md) | `core-cst` uses a green/red tree | accepted |
| [002](ADR-002-toolchain-and-msrv.md) | Pinned toolchain and MSRV 1.90 | accepted |
| [003](ADR-003-phase-0-gate-arming.md) | Phase-0 gates are wired non-vacuously, and arm on a published schedule | accepted |
| [004](ADR-004-corpus-by-pinned-fetch.md) | Corpora are fetched at pinned SHAs with exact yields, never vendored | accepted |
| [005](ADR-005-determinism-gate-mechanism.md) | Determinism = double-build + double-run output-hash compare, with an in-repo SHA-256 | accepted |
| [006](ADR-006-benchmark-baselines.md) | Benchmark baselines are named-and-uncalibrated until recorded on CI | accepted |
| [007](ADR-007-no-brand-named-artifacts.md) | No brand-named crate or binary exists until D1 | accepted · discharged by 015 |
| [008](ADR-008-conformance-semantics.md) | Conformance: reject invalid *structure*, and two rates instead of one | accepted |
| [009](ADR-009-publishing-conformance-rates.md) | Conformance rates are published as a generated file CI regenerates and byte-compares | accepted |
| [010](ADR-010-flagship-is-konflux.md) | konflux is the flagship (D3), decided without the validation signal | accepted |
| [011](ADR-011-diff-golden-contract.md) | The diff golden is the `--json` output, and it lands red-but-recorded | accepted |
| [012](ADR-012-refusal-is-a-result.md) | A format with no semantic view is refused, never answered | accepted |
| [013](ADR-013-semantic-coverage-is-measured.md) | YAML's semantic view, and publishing how much of the corpus it models | accepted |
| [015](ADR-015-brand-is-inviolate.md) | The umbrella brand is `inviolate`, the multicall binary is `invio` | proposed |
