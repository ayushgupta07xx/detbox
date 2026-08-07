# ADR-003: Phase-0 gates are wired non-vacuously, and arm on a published schedule

**Date:** 2026-08-07 · **Status:** proposed

## Context

Phase 0 requires "every §8 gate present and green against hello-world code."
But four gates have nothing to measure before a parser exists: golden, fuzz,
conformance and differential. The tempting move is to stub them green. A stubbed
green gate is worse than a missing one — it reports success, so nobody notices
it never armed, and six months later the badge is a lie.

## Options

- **A — Stub the four gates to `exit 0` with a TODO.** Fast, and structurally
  dishonest: the check is green and proves nothing.
- **B — Omit them until their milestone.** Honest, but then M1 both writes a
  parser and discovers the harness, and the CI law starts as aspiration.
- **C — Wire each gate against the weakest *true* statement available now, and
  publish the milestone at which it arms.** More Phase-0 work; every gate is
  real from commit one.

## Decision

**C.** `core_cst::roundtrip_identity` is K1 for the empty grammar — the grammar
that models nothing and therefore preserves everything as one verbatim node. It
is the weakest true statement of `serialize(parse(x)) == x` and the strongest
one available before a parser exists. The golden suite, the fuzz target, the
determinism gate and the miri job all point at it, so all four are non-vacuous
today. At M1 it is **deleted** and its callers re-point at the real
`parse`/`serialize` pair; the golden cases (CRLF, BOM, NUL, invalid UTF-8, YAML
anchors) survive unchanged, because an input that round-trips today must still
round-trip once there is a parser.

For conformance and differential, where no true statement about *our* code
exists yet, the gate asserts the surrounding machinery instead: the harness
builds and runs; `git merge-file` — konflux's first differential oracle — is
present and behaves as expected on a fixture. Each job prints the milestone at
which it arms, and `MILESTONES.md` carries the same schedule.

The corollary, encoded in `core-verify` as **invariant V4**: a suite with zero
cases is an **error**, not a pass. That is the specific way this decision could
rot, so it is a test.

## Consequences

- More Phase-0 work than stubbing, and a small amount of code lives in
  `core-cst` and `core-verify` that would otherwise be empty. Both are marked
  Phase-0-only and are deleted or replaced at M1.
- If `roundtrip_identity` still exists when a parser ships, that is a bug in the
  milestone. Its doc comment says so.
- The arming schedule is a public promise. Missing one is visible.

## Proof impact

Makes the §8 golden, fuzz-smoke, determinism and miri gates real from commit
one. K1 is live — as the plan requires, it is the first gate written and it
never comes out.
