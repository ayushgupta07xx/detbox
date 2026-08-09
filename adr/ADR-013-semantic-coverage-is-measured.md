# ADR-013: YAML's semantic view, and publishing how much of the corpus it models

**Date:** 2026-08-09 · **Status:** proposed

## Context

ADR-012 established that a format with no semantic view refuses rather than
answers. That makes refusal *safe*. It does not make it *useful*, and it hides a
question the diff golden suite cannot ask: ten hand-built cases all pass, so is
konflux ready for real config, or does it decline every file a user owns?

Building YAML's view forced two readings of the concrete tree, and then answered
that question with a number that was worse than expected in a way worth writing
down.

## The two readings

**1. Comment-only lines are dropped from the view.** The lexer gives a line with
no indentation of its own to the innermost open line, so in

```yaml
global:
  imageRegistry: ""
  ## E.g. imagePullSecrets:
```

the comment is a **child of `imageRegistry`**, not a sibling. Read literally,
that makes a scalar look like a container, and the document is refused. Helm
values files are more comment than value, so this one shape accounted for
**235 of 750** corpus files. Dropping the lines is sound rather than convenient:
they carry no semantic content, so a view without them is complete, and their
bytes remain the CST's business where K1 already proves they survive.

**2. A key adopts the zero-indented sequence beneath it.** YAML permits

```yaml
items:
- a
- b
```

and Kubernetes manifests are written that way more often than not. Indentation
makes those dashes *siblings* of the key, so a literal reading produces a
mapping and a sequence sharing one level — which is not a document anyone wrote.
The key therefore adopts the run of items immediately following it. Another
**139 files**.

Both are cases where the concrete tree is right and a naive reading of it is
wrong. That is the whole job of this layer, and it is worth noting that **both
were found by measurement, not by review** — the golden suite passed at 10/10
while the parser modelled 14.5% of real YAML.

## Decision

**Measure and publish semantic coverage over the corpus, with the refusal
reasons ranked and each carrying an example file.** `cargo xtask
semantic-coverage`.

The reasoning is the one ADR-008 used for conformance: a capability nobody has
measured is a claim. "konflux diffs YAML" is true of ten golden cases and was
true of 14.5% of the corpus at the moment those cases went green. Only one of
those numbers tells you whether to ship.

Ranking the refusals by count turns "not modelled yet" from an excuse into a
**work queue ordered by how much real config each item unblocks**, and the
example path makes each entry reproducible. Today it reads:

```
yaml   237/750 modelled (31.6%)
        204  flow collections
        145  a line that is neither a mapping entry nor a sequence item
         68  multi-document streams
         32  block scalars
json   250/250 modelled (100.0%)
TOTAL  487/1000 (48.7%)
```

**Reported, not gated.** §8's gate list is for statements that should never
regress; this number will move on most M2 commits, and a threshold that changes
every PR trains everyone to edit thresholds — the same reasoning that kept the
fuzz-hours ledger out of the blocking set. It becomes a ratchet when M2 ends and
the number stops moving weekly.

## Consequences

- **konflux declines the majority of real YAML today**, and that is now a
  published number rather than a surprise waiting for a user. It is the honest
  reading of ADR-012: refusing is safe, not free.
- **The next four pieces of M2 are chosen by data**: flow collections, then
  whatever the 145 unclassifiable lines turn out to be (Helm templates, most
  likely — the survey already found `{{ }}` in 41.2% of corpus YAML), then
  multi-document streams, then block scalars.
- **Coverage is not conformance and must never be blended with it.** One says
  what we can read, the other what we correctly accept or reject. ADR-008
  already refused to blend two numbers into one badge; this is a third number
  and it stays separate.
- **A view that drops comments cannot answer comment questions.** konflux's
  pitch is *"comments and key order preserved"* — preservation is K1's job and
  is proven, but *diffing* a comment change is not something this layer can
  express, and the deferred comment goldens (ADR-011) will need trivia
  attachment in the CST walk rather than in the view.

## Proof impact

No invariant changes. Adds a measured, published figure to the proof surface
alongside the conformance rates and the fuzz-hour counter. It does **not**
discharge the yaml reject-rate gate: that gate is about `parse` refusing invalid
documents, and this ADR touches only the layer above `parse`. Wiring the view's
block/flow knowledge back into validation is separate work, and MILESTONES has
been corrected to say so.

## Reproduce

```bash
corpora/fetch.sh && cargo xtask semantic-coverage
```

---

## Amendment 1 — block scalars carry source text, not folded text (2026-08-09)

A block scalar's `value` is its source bytes — header and body — not the string
YAML would fold it into. Implementing indentation stripping and chomping
correctly is real spec work, and getting it subtly wrong makes two *different*
strings compare equal, which is a diff that misses an edit.

The cost is over-reporting: re-indenting a body, or switching `|` for `>`, reads
as a semantic change when the folded string is unchanged. That is the same trade
this ADR already took on numbers, in the same direction — **noisy is
recoverable, silent is not.** Folding becomes worth implementing when a golden
case shows the noise costing more than the risk.

**A lexer wart found while doing it, and pinned rather than papered over:** a
block body absorbs the file's trailing newline *only when nothing follows the
block*. Same logical content, two different node texts, so comparing a block at
end-of-file against the same content mid-file over-reports. Golden cases `180`
and `181` pin both shapes, and a unit test states the rule.

Coverage: YAML **48.4% → 57.2%**.

