# Benchmark baselines

One file per runner class. A baseline is a **reviewed artifact**: it is recorded
from a CI run on `main` and lands as a normal, human-approved change.

```
#! tolerance_pct = 10
<benchmark-name>   <nanoseconds | uncalibrated>
```

`xtask bench-compare` enforces, blocking, from commit one:

1. the criterion run happened and its output parses;
2. every benchmark named here still exists — a benchmark that silently
   disappears takes its regression gate with it;
3. every benchmark that ran is named here — new benchmarks cannot land
   unrecorded;
4. for calibrated entries, the measurement is within `tolerance_pct`.

`uncalibrated` means "we have not recorded a trustworthy number on this runner
class yet." Checks 1–3 still apply to it. Numbers taken on a WSL2 laptop are not
comparable to a shared GitHub-hosted runner, and publishing them as if they were
would be a cherry-picked benchmark — a permanent ban (Appendix C).

**Tightening a tolerance or a number is free. Loosening either requires a
`[NEEDS-AYUSH-APPROVAL]` header and sign-off** (MASTER_PLAN §8).

See ADR-006.
