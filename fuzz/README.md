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

| Target | Invariants | Status |
|---|---|---|
| `roundtrip_identity` | K1 on the empty grammar | Phase 0 leftover; deleted when parse lands (ADR-003) |
| `yaml_roundtrip` | F1 (never panics) + K1 | M1, wired, **vacuous until parse exists** |
| `json_roundtrip` | F1 (never panics) + K1 | M1, wired, **vacuous until parse exists** |
| `edit_locality` | K2 | strukt |
| `merge_algebra` | P2 laws | konflux M3 |

## The vacuity problem, and the guard

A K1 target is shaped like this:

```rust
let Ok(cst) = format.parse(data) else { return };   // parse failed: nothing to check
assert_eq!(format.serialize(&cst), data);           // the actual assertion
```

If `parse` never succeeds, the assertion is **never reached**. The target runs,
finds no crash, and reports success — having verified nothing. Measured, not
asserted:

```text
$ cargo +nightly fuzz run yaml_roundtrip fuzz/corpus/yaml_roundtrip -- -runs=200000
#200000 DONE   cov: 39 ft: 40 corp: 1/1b
Done 200000 runs in 0 second(s)
exit=0
```

200,000 inputs, zero crashes, exit 0 — and not one K1 assertion evaluated. This
is worse than a vacuous golden suite, which at least has a case count that
visibly shrinks: a fuzz run that asserts nothing prints exactly what a
productive one prints.

A `fuzz_target!` cannot detect this itself; it sees one input at a time and has
no memory across a run. So the guard lives outside the fuzzer, in
`crates/core-formats/tests/fuzz_seeds.rs`: **every seed in a target's corpus
must parse.** Seeds are copied from the K1 golden cases — inputs that by
definition must round-trip — so a seed that does not parse is a parser bug or a
case that does not belong.

It also asserts the seed count has not fallen behind the golden suite, so adding
a case without re-seeding is caught rather than silently narrowing what the
fuzzer starts from.
