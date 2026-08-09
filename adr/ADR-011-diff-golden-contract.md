# ADR-011: The diff golden is the `--json` output, and it lands red-but-recorded

**Date:** 2026-08-09 · **Status:** proposed

## Context

konflux M2's first item is *"oracle first: diff golden suite — hand-built cases
where line-based diff is wrong and structural diff is right. Confirm red."*
Writing it forces four questions the K1 oracle never asked, because K1 needed no
`expected` file at all — the input was the expectation, so a K1 case could not
be doctored, only deleted. A diff case must state an answer, and stating it
means choosing a shape for the answer before any code produces one.

A fifth question is procedural and turned out to be the sharpest: **M1's oracle
PR could merge green because the parser landed in the same PR.** A diff
implementation cannot — it is semantic-tree matching plus format-preserving
output, which is the majority of Phase 1's time budget by design (§15). So the
oracle must merge without it, and a suite that is simply red on `main` is the
one thing worse than no gate, because a permanently-red gate trains everyone to
ignore the colour. That is not hypothetical here: MILESTONES already records the
determinism false positive as *"a gate that cries wolf trains everyone to ignore
red, which is worse than no gate."*

## Options

- **A — Golden is the rendered CLI output.** Tests what users see; every
  cosmetic change to rendering rewrites every golden, and colour, width and TTY
  behaviour leak into evidence.
- **B — Golden is the `--json` output.** Tests the machine contract `core-cli`
  C1 already requires. Does not test the human rendering, which needs its own
  (smaller) suite later.
- **C — Golden is an internal edit-script dump.** Most direct test of the
  algorithm; invents a second serialisation that no user ever sees and that
  nothing else keeps honest.

## Decision

**B, with four sub-decisions that the cases depend on.**

**1. The golden is `--json`.** §3.4 requires every tool to emit stable,
schema-versioned `--json`. Making it the oracle means the public contract is
under test from before it has an implementation, rather than being retrofitted
once the shape has already leaked into someone's script. It also means the §9.2
self-review question *"is `--json` stable?"* is answered mechanically instead of
by memory.

**2. Paths are RFC 6901 JSON Pointers, not dotted paths.** A path is an
identity, and an identity may not be ambiguous. Real Helm charts contain keys
like `kubernetes.io/os`; `.nodeSelector.kubernetes.io/os` cannot be parsed back
to the key it came from, and a merge tool that resolves a path to the wrong node
is silently wrong, which §0 ranks as the worst thing we can be. Case
`120-key-containing-a-slash` exists to hold this decision in place. The pretty
dotted form belongs in the human rendering, where being wrong is cosmetic.

**3. `before` and `after` carry source text, never parsed values.** They are the
node's exact bytes as text. Emitting a *value* would require deciding whether
`yes` is a boolean, which is YAML-version-dependent — the same line ADR-008 drew
when it put conformance at *structure* rather than at interpretation. A diff
that reports `true → yes` has already lost the argument.

**4. `kind` and `significance` are separate fields.** `kind` says what happened
to the tree (added, removed, changed, moved); `significance` says whether it
means anything (semantic, formatting). Neither determines the other, and the
pair is the whole product: a reordered **mapping** is `moved` + `formatting`, a
reordered **sequence** is `moved` + `semantic`. A line diff renders those two
identically, and cases `010` and `130` are that pair, deliberately adjacent.

### And it lands red-but-recorded, not red

`UNIMPLEMENTED_CASES = 9` is checked exactly. This is **ADR-003's idiom, reused
rather than invented**: wire the gate blocking today against the weakest *true*
statement available, and publish the schedule on which it arms. The weakest true
statement is "nine of these ten cases are unproven." M2's implementation drives
it to zero, at which point the constant and its branch are deleted and
`assert_ok()` becomes the whole test.

It is checked with `==` rather than `<=`, which is stricter than the conformance
ratchet (ADR-008) and deliberate: an improvement should not be able to land
without the recorded number moving in the same PR. Both directions are red and
say opposite things — a regression prints the standard goldens-are-evidence
warning; an improvement prints *"good news, and a constant to lower."*

**This is not a skip and not a loosened threshold.** Every case still runs, and
every one is still compared byte-for-byte. What is recorded is how many cannot
yet be produced, and that number may only fall. Tightening is free (§8).

### The vacuity guard

`the_suite_is_not_vacuous` runs a null diff over the suite and requires exactly
nine of ten cases to reject it. M1 taught this the hard way: a K1 fuzz target
reported success having evaluated no assertion at all, and the fix was a
separate test that made the vacuity visible. A diff suite fails the same way
more quietly — if formatting-only differences were reported as *no* change, the
three formatting cases would be satisfied by returning `[]` and a third of the
suite would prove nothing. That is the real reason formatting changes are
reported rather than left implicit.

`900-identical` is the one case a null diff passes, and it is here on purpose:
an oracle no output can satisfy is as broken as one everything satisfies.

## Consequences

- **The `--json` shape is now public before it is implemented**, and changing it
  is a reviewed golden diff. `SCHEMA_VERSION` exists so it can change honestly.
- **Two constructs are deliberately absent**: comments and re-indentation. Their
  right answer depends on trivia attachment in the CST walk, and a guessed
  `expected` is a guess wearing evidence's clothes. They land with the
  implementation. konflux's pitch is *"comments and key order preserved"*, and
  today the comment half is proven by nothing — that is recorded, not hidden.
- **`core-verify` grew a two-input runner.** A diff takes two documents and
  `run_dir` takes one. Everything else about it is identical on purpose;
  a second harness with second-hand discipline is how the two drift apart.
- **What is bet:** that the hand-written `expected` files are right. They are a
  specification, and if the algorithm later shows one is wrong, the fix is
  `[NEEDS-AYUSH-APPROVAL]` and not a quiet edit.

## Proof impact

Discharges the oracle half of konflux M2 and is the substrate for **P4**'s
differential runner, which arms at M2 against `diff3`. No invariant changes: K1,
K2 and K3 are untouched, and `Format::parse`'s contract is unchanged.

## Reproduce

```bash
cargo test -p konflux
```
