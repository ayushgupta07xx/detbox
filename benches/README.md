# `benches/` — criterion benchmarks and the published benchmark pages

MASTER_PLAN §12: *"Benchmark methodology page: hardware, versions, commands, raw
data in-repo, reproduction one-liner. Anything less gets shredded, correctly."*

## Run

```bash
cargo bench -p benchsuite
cargo xtask bench-compare target/criterion benches/baselines/linux-x86_64.tsv
```

## The honesty rules (MASTER_PLAN §0, §12, Appendix C)

- Benchmark tables **include the cells where incumbents win**. Calibration is
  the credibility.
- Every published number carries hardware, tool versions, the exact command, and
  a reproduction one-liner.
- Raw data lives in this repo.
- Cherry-picked or irreproducible benchmarks are a permanent ban.
- bigsheet's P4 is a *performance budget*, not a proof. Do not let that wording
  drift.

## Status

Phase 0: two benchmarks over the K1 identity boundary, both `uncalibrated`.
They exist so the regression gate is wired and non-vacuous. Real
parse/serialize benchmarks over corpus files land at konflux M1.
