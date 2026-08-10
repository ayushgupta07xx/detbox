# ADR-017: The differential runs against ground truth, not against diff3

**Date:** 2026-08-09 · **Status:** proposed

## Context

`gate/differential` has been wired-but-inert since Phase 0, arming at M2 by
ADR-003's schedule. MILESTONES specifies it as *"ours vs `diff3`/`git diff` on
the corpus; divergences triaged into golden cases, never ignored."*

Taken literally, that comparison is **ill-posed**. A line differ and a
structural differ do not compute the same function. `git diff` says *these bytes
differ*; konflux says *this key changed, and here is whether it means anything*.
Neither answer refutes the other, and there is no agreement set to measure —
a runner that "compared" them would be inventing a scoring rule and then citing
its own rule as evidence. §8's differential row says "yes on agreement set", and
here the agreement set is empty.

## Options

- **A — Compare rendered output to `git diff`.** Requires deciding what
  agreement means between two different functions. Whatever rule we picked would
  be ours, so the oracle would not be independent.
- **B — Skip the gate until an oracle exists.** Mergiraf is the real structural
  peer and arrives as a comparison at M3, when there is a merge to compare. It
  leaves a §8 gate inert for a milestone it was scheduled to arm at.
- **C — Differential against ground truth we construct.** Take real corpus
  files, apply an edit whose meaning we know, and require konflux to notice.
  Not a second implementation, but *stronger* than one: we know exactly what
  changed, so silence is unambiguously a lost edit rather than a difference of
  opinion between two tools.

## Decision

**C**, with `git`'s byte-level verdict as the one place it *is* a sound oracle.

Two properties over every corpus file konflux can model:

1. **A file diffed against itself must report nothing.** Bytes identical, so the
   answer is not a matter of interpretation. If konflux reports a change here,
   its diff is not a function of its inputs.
2. **A known semantic edit must be noticed.** One top-level key appended,
   `zzz_konflux_probe: 1` — deterministic, valid on any mapping-rooted document,
   impossible to confuse with existing content. Konflux must report a *semantic*
   change at that path.

Property 2 is where the value is. **A lost edit is the worst failure this tool
has**: a merge driver that does not notice a changed key will drop it, silently,
in someone's cluster config. §0 ranks "never silently wrong" first, and this is
the mechanical check for it on 640 real files rather than on cases we invented.

### It found a real bug on its first run

**Duplicate mapping keys.** A Kubernetes secret in the corpus has `type:` twice,
with different quoting. A diff matches entries by key, so the second occurrence
pairs against the *first* — and the file reported a change **against itself**.
In JSON it was worse: `{"a":"x","a":"y"}` compared to itself reported a
**semantic** change and exited `1`, so a CI job would fail on a file diffed
against a copy of itself.

The fix is to refuse. Both specs already do — YAML 1.2 calls duplicate keys an
error, RFC 8259 calls repeated names unpredictable — so this is reading the spec
rather than inventing a rule. Our parsers still accept them, which is K1 doing
its job: preserving bytes it does not endorse. The refusal belongs one layer up,
where we claim to know what a document *means* (ADR-012).

Cost: 3 corpus files, coverage 64.7% → 64.3%. Refusing three files is cheaper
than being wrong about them.

### It also found a bug in itself

Eight files reported as lost edits were not. They are istio manifests ending in
a trailing `---`, so appending a key created a **second document** rather than
adding a key — a shape change, which konflux correctly reported as a
replacement. The runner was blaming the tool for the runner's own mutation.

The mutation now requires the mutated document to still be mapping-rooted, and
those eight are counted as skipped. **Every skip class is printed**, because a
runner that quietly narrows its own corpus is reporting on a sample it chose.

## Consequences

- **`gate/differential` is armed** and blocking, on its ADR-003 schedule.
- **640 of 1,000 corpus files are in scope.** The rest are skipped for reasons
  that are counted and named: unmodelled, not mapping-rooted, unreadable once
  mutated, reshaped by the mutation.
- **One probe is one property.** This catches a lost *addition*; it does not
  catch a lost *modification* or *deletion*. Those are further mutations and
  they are cheap to add — the runner's shape is the contribution, not its
  current single mutation.
- **Mergiraf remains the real peer comparison**, and it arrives at M3 where §4.1
  P3 asks for auto-resolution rates measured against it. This ADR does not
  replace that; it fills the M2 slot with something checkable instead of
  something ceremonial.

## Proof impact

Arms `gate/differential` (§8). Discharges the differential half of M2. Does not
touch K1, K2 or K3, and does not change `--json`. It found and fixed a soundness
bug in `semantic_view`, which is the whole reason the gate exists.

## Reproduce

```bash
corpora/fetch.sh && cargo xtask diff-differential
```
