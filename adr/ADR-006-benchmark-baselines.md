# ADR-006: Benchmark baselines are named-and-uncalibrated until recorded on CI

**Date:** 2026-08-07 · **Status:** accepted (Phase 0 accepted 2026-08-07)

## Context

MASTER_PLAN §8 requires a blocking benchmark-regression gate: criterion vs saved
baselines, ±threshold. At Phase 0 there is no saved baseline, and the only
machine available to record one is a WSL2 laptop whose timings have no
relationship to a shared GitHub-hosted runner. Checking in laptop numbers as if
they were runner numbers would produce false regressions immediately — and
publishing them would be a cherry-picked benchmark, which Appendix C bans
permanently.

## Options

- **A — Skip the gate until numbers exist.** Honest, and it means the §8 table
  has a hole from day one and nobody finds the plumbing bugs until later.
- **B — Check in laptop numbers with a wide tolerance.** Gives a green check
  that measures nothing, and bakes in a number we would have to defend.
- **C — Make the baseline carry benchmark *names* plus either a calibrated
  number or the literal `uncalibrated`, and gate on the structural properties
  now.** Slightly unusual; every part of it is true today.

## Decision

**C.** `benches/baselines/<runner-class>.tsv` lists every benchmark name with a
value that is a number or `uncalibrated`. `xtask bench-compare` enforces four
things, all blocking, from commit one:

1. the criterion run happened and its output parses;
2. **every benchmark named in the baseline still exists** — a benchmark that
   silently disappears takes its regression gate with it;
3. **every benchmark that ran is named in the baseline** — new benchmarks cannot
   land unrecorded;
4. for calibrated entries, the measurement is within `tolerance_pct`.

Checks 2 and 3 are the anti-rot properties, and they are meaningful with zero
calibrated numbers. Check 4 arms per-benchmark the moment a number is recorded
from a CI run on `main` and lands as a reviewed change.

Baselines are **evidence**: `golden-guard` and `CODEOWNERS` cover
`benches/baselines/**`. Tightening a tolerance or a number is free. Loosening
either requires `[NEEDS-AYUSH-APPROVAL]`.

The §12 honesty rules bind everything published from this data: hardware,
versions, exact commands, raw data in-repo, a reproduction one-liner, and the
cells where incumbents win. bigsheet's P4 is a performance *budget*, not a
proof; that wording does not drift.

## Consequences

- The badge cannot say "no performance regressions" until numbers are
  calibrated. It can say "benchmark set intact," which is what is true.
- Shared runners are noisy. A ±10% default will need revisiting with real
  variance data — upward only with sign-off, and preferably by reducing noise
  instead.
- Every new benchmark forces a baseline edit. That is the intended friction.

## Proof impact

Implements the §8 benchmark-regression gate structurally now and numerically on
calibration. Governs bigsheet **P4**, konflux's published benchmark table, and
cage **P3**.
