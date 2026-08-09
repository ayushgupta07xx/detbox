# ADR-012: A format with no semantic view is refused, never answered

**Date:** 2026-08-09 · **Status:** accepted (2026-08-09)

## Context

M2's diff needs a typed layer. §3.2 sketches one:

```rust
fn semantic_view(&self, cst: &Cst) -> SemanticTree;
```

Total. It cannot be. M1 built YAML's concrete tree as `STREAM → DOCUMENT →
LINE*` — a flat list of lines carrying indentation tokens, deliberately lossless
and structural rather than semantic. Producing a view of it means *inferring*
block structure from indentation, and that inference is the largest single piece
of work left in M2. JSON's tree is already nested, so its view is a walk.

So there is a window — and there will be one again for TOML and HCL in Phase 2,
and for PDF later — in which konflux can parse a format perfectly and understand
it not at all. The question this forces is small to write and large to get
wrong: **what does `diff` return when it cannot model the input?**

## Options

- **A — Return an empty diff.** The signature stays total and every caller stays
  simple. It also means konflux answers *"no changes"* for a file it never
  understood.
- **B — Refuse, with a reason.** `diff` returns `Result`, and a format without a
  view produces a refusal naming itself. Every caller must handle it, and the
  golden harness has to treat a refusal as a comparable result rather than a
  crash.
- **C — Return a diff annotated "low confidence".** Expressive; it is a
  confidence score, which Appendix C bans outright, and it would make the
  caller decide how much to trust us. Rejected on sight.

## Decision

**B. And the reasoning is a one-liner, which is why it is worth an ADR at all:**
*an empty diff and an agreement are the same bytes.*

`{"changes": []}` is what konflux emits for two identical files. If it also
emits that for two files it cannot read, then the output has one spelling for
two opposite meanings, and the caller — a merge driver, a CI `--check`, a human
skimming a PR — cannot tell them apart. §0 ranks *never silently wrong* first
and defines the alternative precisely: *"when uncertain, emit a conflict, a
refusal, or a structured report."* This is the refusal.

It matters most exactly where konflux is aimed. A git merge-driver that reports
no changes gets its silence believed and takes one side wholesale. A driver that
refuses hands the file back to git's line-based merge, which is worse output and
a correct answer. **Being unhelpful is recoverable; being wrong is not.**

The variant list is therefore deliberately short: `Parse` and `Unmodelled`.
There is no variant meaning "I could not tell", because that is A wearing a
different hat.

### What this costs the oracle, and why that is right

A refusal is a *result*, so the golden runner compares it like any other. The
harness renders it as the case's actual output:

```
expected: {\n  "schema_version": 1,\n  "changes": [\n    {\n
actual:   konflux: refused — yaml has no semantic view: ...
```

The case fails, readably, and the eight cases behind it still run. Panicking
instead would abort the suite at the first YAML case and hide the rest — the
harness would be reporting on the harness, not on konflux.

This is also why `900-identical` — the suite's deliberate control — is currently
**red**. konflux cannot say even *"no changes"* about a YAML file today, and
saying it anyway is the thing this ADR forbids. The control goes green with
YAML's view, not before, and its redness is honest rather than embarrassing.

## Consequences

- **§3.2's sketched signature changes**, and this is the second time a §3.2
  signature has met reality (ADR-008 fixed `parse`'s `Err` case). The pattern is
  the same both times: the plan's shape was right and its totality was
  optimistic.
- **Every future format gets this for free.** TOML, HCL and PDF each land parse
  before they land meaning, and in that window each refuses rather than
  agreeing. The default trait implementation is the refusal, so a new format
  cannot get this wrong by omission — only by deliberately overriding it.
- **Callers must handle a refusal**, including M4's merge driver, where the
  correct handling is to exit non-zero and let git fall back.
- **The window is visible in the proof surface**, not hidden by it: the count of
  cases konflux cannot answer is `UNIMPLEMENTED_CASES` in the golden test, and
  it ratchets down (ADR-011).
- **What we are betting:** that refusing is rare and shrinking. If a format sat
  in the refusing state indefinitely, users would learn konflux does not work on
  their files — which is the honest outcome of it not working on their files.

## Proof impact

No invariant changes. K1, K2 and K3 are untouched: this is about the typed layer
above the CST, and `parse`/`serialize` keep their contracts. It fixes the
meaning of `Format::semantic_view`'s `Err` case, which every format from Phase 2
onward inherits, exactly as ADR-008 fixed `parse`'s.

## Reproduce

```bash
cargo test -p konflux && cargo test -p core-formats --lib semantic
```
