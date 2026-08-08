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

## Amendment, 2026-08-08: calibrated, with per-benchmark tolerances

The baselines are now calibrated from the bench job on `main` at `9ef31af`, as
this ADR required. Doing so produced the variance data this decision asked for,
and it changed one thing.

Measured across three CI runs on `ubuntu-latest`:

| benchmark | main run (ns) | run-to-run spread |
|---|---:|---:|
| `json_parse_real` | 11,355.2 | 1.3% |
| `yaml_parse_64kib` | 1,352,928.0 | 2.9% |
| `json_parse_64kib` | 61,243.4 | 4.2% |
| `yaml_parse_real` | 8,742.2 | 4.6% |
| `yaml_parse_small` | 3,323.2 | **7.6%** |

A single `tolerance_pct` would have held here, but only just: 7.6% against a 10%
threshold, on three samples. The cause is not a badly written benchmark — the
same absolute jitter is a very different fraction of a 3.5 µs measurement than of
a 1.3 ms one. One global figure must therefore either flake on the small
benchmarks or go blind on the large ones.

So the baseline format gained an optional third column, a per-benchmark
tolerance, defaulting to the file-level figure. Each is set at roughly **3× its
observed spread, floored at 10%**. `n = 3` is thin, so these start loose enough
not to cry wolf; tightening them as samples accumulate is free and expected.
Loosening one still requires `[NEEDS-AYUSH-APPROVAL]`.

Incidentally confirming the original decision not to calibrate locally: the same
benchmarks run **30–38% faster on the WSL2 dev machine** than on the shared
runner. Checked-in laptop numbers would have made every CI run a regression.

## Amendment 2, 2026-08-08: timings cannot gate; allocations can

Amendment 1 calibrated the baselines from a CI run on `main`. **The very next CI
run flagged a 43% regression on a parser that had not changed by a byte.**

Six runs of data explain it, and refute the obvious excuse:

| | measurement |
|---|---|
| criterion's own 95% CI, within a run | **±0.6% – ±1.4%** |
| the same benchmark, between runs | **+38% / −20%** |

Criterion is measuring precisely; the thing being measured is not stable. And
"the host was slow" does not fit: the failing run produced the **fastest** result
of six for three benchmarks and the **slowest** for a fourth, in one job.

No tolerance survives that. Wide enough not to cry wolf (±45%) is wide enough to
miss any regression worth catching — and a gate that cries wolf is worse than no
gate, because it teaches everyone to ignore red. That failure mode has already
cost this repo two false alarms.

**So the numeric gate moves to allocation counts.** `xtask alloc-profile` counts
allocations for parse+serialize over the committed golden cases; `alloc-check`
compares the result to `benches/baselines/allocations.tsv` **for equality**, with
no tolerance, because there is no noise to tolerate. Verified identical across
five consecutive runs and across debug and release builds.

This is a *stricter* gate than the one it replaces, not a looser one. A single
allocation appearing or disappearing fails it. And it follows the plan's own
priority order: §0 puts deterministic, machine-verifiable measures above
probabilistic ones, which is exactly the choice between a count and a stopwatch.

Timings keep their structural gate — the run happened, the output parses, no
benchmark vanished, none landed unrecorded — and the raw criterion data is still
published, because §12 requires a benchmark table with methodology and raw data.
They simply no longer decide whether CI is red. `linux-x86_64.tsv` returns to
`uncalibrated`, which is now the honest state and not a placeholder.

**What this costs.** Allocation count is a proxy. A change that makes the parser
slower without changing its allocation behaviour — a worse inner loop, a bad
branch — passes this gate. That is a real hole, and the honest mitigation is the
published timing table rather than a threshold nobody can trust. If timing ever
needs to gate, it needs a dedicated runner, not a wider tolerance.

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
