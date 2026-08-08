# ADR-008: What conformance means for a lossless parser, and two rates instead of one

**Date:** 2026-08-08 · **Status:** accepted (2026-08-08)

## Context

MASTER_PLAN §3.3 requires conformance adapters and §4.1 **P4** requires
published pass rates. Wiring them up forces two questions that the K1 oracle
alone never asked.

**First: what is our parser allowed to reject?** §3.1 says input the modelled
grammar cannot represent is *"preserved as an opaque verbatim node rather than
normalized — preserving beats understanding."* Read at its widest, that means
accepting everything, and a parser that accepts everything scores **0%** on the
94 must-reject cases in yaml-test-suite and the 188 in JSONTestSuite. Read at
its narrowest, konflux refuses files it could have safely round-tripped.

**Second: what number do we publish?** The obvious one is `passed / total`, and
it is a trap. Measured, with today's stubbed parser that does nothing but return
`Err`:

| suite | accept | reject | **blended** |
|---|---:|---:|---:|
| json-test-suite | 0/95 = 0.0% | 188/188 = 100.0% | **66.4%** |
| yaml-test-suite | 0/308 = 0.0% | 94/94 = 100.0% | **23.4%** |

A function whose entire body is `return Err` would publish **66.4% JSON
conformance**. That badge would be a lie told with true arithmetic.

## Options

- **A — Strict.** `parse` rejects anything the spec calls invalid; verbatim
  covers only unmodelled-but-valid constructs. Conformance is meaningful; some
  files konflux could have preserved get refused.
- **B — Permissive.** `parse` accepts everything as verbatim. K1 holds
  universally and the reject-rate is pinned at 0% forever, making half of every
  conformance suite unscoreable.
- **C — Two-level.** `parse` always produces a tree (K1 universal) and a
  separate validity verdict answers conformance. Most expressive; changes
  §3.2's `parse` signature and adds a concept to every downstream product.

## Decision

**A, with the line drawn at *structure* rather than at *bytes*.**

`parse` **rejects** input that violates the spec's grammar — bad indentation, a
malformed anchor, a duplicate merge key, an unterminated flow collection. It
**accepts, as `SyntaxKind::VERBATIM`**, input that is tokenizable but not
modelled: exotic tags, Helm's `{{ }}`, unusual encodings, trailing bytes after a
document. §3.1's own examples — *"exotic YAML tags, weird encodings"* — are all
on the accept side of that line, so this is a reading of §3.1 rather than a
change to it.

For konflux the consequence is the safe one: a file we cannot structurally
understand is refused, and git falls back to its line-based merge. Refusing is
never silently wrong; merging a document we misparsed is exactly the failure
§0 ranks first.

**C is rejected as unnecessary, and that conclusion is measured rather than
assumed.** The worry was that A makes the two oracles mutually unsatisfiable:
our K1 golden suite asserts round-trip on `250-non-utf8`, `280-control-bytes`
and `290-trailing-garbage`, which are *not* valid YAML, while conformance
demands invalid input be rejected. If those two sets overlapped, one contract
would have to give and C would be the only way out.

They do not overlap. Of yaml-test-suite's 94 must-reject cases, **0 involve
invalid UTF-8 and 0 involve C0 control bytes** — all 94 are structural. Our
three awkward K1 cases are encoding-level, which is the accept side of the line.
The two oracles are compatible exactly as written, so C's extra concept buys
nothing and §3.2's `parse` signature stands unchanged.

**And there is no blended pass rate.** `core-verify::conformance` reports
accept-rate and reject-rate separately and offers no method that combines them.
Both ratchet independently, so a parser cannot pay for accepting more by
rejecting less. Implementation-defined cases (JSONTestSuite's 35 `i_*`) are
measured and published but never gated: the spec permits either answer, and
gating one would invent a requirement.

`unrecorded` in `conformance/thresholds.tsv` is an **error**, not a free pass.
A benchmark baseline may honestly be `uncalibrated` because timings are
machine-dependent (ADR-006); a conformance rate is deterministic, so
`unrecorded` means only that no claim has been made yet.

## Consequences

- **konflux will refuse some files it could have preserved.** That is the
  intended trade and it should be visible: the `--json` output must say
  *"refused: invalid YAML at line N"*, not fail silently, so a user can see why
  git took over.
- **The badge cannot be a single number**, which is slightly worse marketing and
  considerably better honesty. §12 already requires publishing the cells where
  incumbents win; this is the same principle applied to our own metric.
- **The line will be tested at M1.** "Structural violation" versus "unmodelled
  construct" is crisp in the cases examined here, and there will be inputs where
  it is not. Each one is a golden case and, if the reasoning is non-obvious, an
  amendment here.
- Both thresholds start `unrecorded`, so the conformance gate is red until a
  parser exists and a first rate is recorded under review.

## Proof impact

Implements konflux **P4** and the §8 conformance gate. Fixes the meaning of
`Format::parse`'s `Err` case, which every format from Phase 2 onward inherits.
Does **not** touch K1: the golden round-trip suites are unaffected, as measured
above.

## Reproduce

```bash
conformance/fetch.sh && cargo test -p core-formats --test conformance -- --nocapture
```
