# `fuzz/` — cargo-fuzz targets

MASTER_PLAN §2: *"one per format per operation."* §3.3: *"corpus-seeded; any K1
violation is minimized and auto-filed as a failing golden case."*

## Run

```bash
cargo +nightly fuzz run roundtrip_identity -- -max_total_time=300
```

`fuzz/` is a **detached workspace**: cargo-fuzz builds on nightly with sanitizer
and libfuzzer link flags that must not leak into the main build.

## The loop

1. Fuzz smoke runs on every PR, ~5 min per target (blocking, §8).
2. Long fuzz runs nightly in CI within the free tier, and for extended sessions
   on Ayush's machine via cron (§8, §10).
3. A crash is **minimised**, then filed as a golden case under
   `crates/core-verify/tests/golden/`, then fixed. The golden case stays
   forever — that is what makes it a regression gate.
4. Fuzz-hours are counted and published as a README badge (§12). konflux P1
   requires ≥72 cumulative hours with zero violations.

## Never

Do not weaken an assertion, shrink the corpus, or delete a seed to make a target
stop finding things. That is the anti-reward-hacking law (§8). If a target's
assertion is genuinely wrong, propose the change under `[NEEDS-AYUSH-APPROVAL]`.

## Targets

| Target | Invariant | Status |
|---|---|---|
| `roundtrip_identity` | K1 on the empty grammar | Phase 0, wired |
| `yaml_roundtrip` | K1 for YAML | konflux M1 |
| `json_roundtrip` | K1 for JSON | konflux M1 |
| `edit_locality` | K2 | strukt |
| `merge_algebra` | P2 laws | konflux M3 |
