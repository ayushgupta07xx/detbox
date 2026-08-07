# ADR-001: `core-cst` uses a green/red tree

**Date:** 2026-08-07 · **Status:** proposed

## Context

`core-cst` is the load-bearing crate of the ecosystem: every product is a head
on it, and K1/K2/K3 live there. MASTER_PLAN §3.1 defers its representation to
this ADR, "made after a 2-day spike comparing edit ergonomics and memory
footprint," and MILESTONES adds *measured, not argued*. The spike is in
[`spikes/adr-001-cst-representation/`](../spikes/adr-001-cst-representation/);
raw output in [`RESULTS.txt`](../spikes/adr-001-cst-representation/RESULTS.txt).

One shared lossless lexer feeds all candidates, so they build structurally
identical trees (235,552 nodes / 983,299 tokens each) and representation is the
only variable. Measured on the real corpus: **750 YAML files, 10,043,614 bytes**
of Helm charts, kustomize overlays and Kubernetes manifests. Memory is counted
by a global allocator rather than sampled from RSS, so every number below is
exactly reproducible.

## Options

- **A — Green/red tree (rowan-style).** Immutable refcounted green nodes with
  interned tokens, no parent pointers or offsets; a red layer materialised on
  demand supplies both. Costs 5.19x memory, the most allocations, the slowest
  build, and a red layer that has to exist at all.
- **B — Owned token tree.** Every node owns its children, every token owns its
  bytes. Simplest code by a wide margin (163 lines vs 288), and worst measured
  memory at 7.71x with no sharing of any kind.
- **C — Flat arena of spans.** One `Vec<Elem>`, children as `u32` links, tokens
  as spans into the source. Best on every resource axis — 3.91x memory, 4.7x
  fewer allocations, 2.6x faster — at the price of untyped `u32` handles the
  compiler cannot check.

## Measurements

| | A green/red | B owned | C arena |
|---|---:|---:|---:|
| memory, bytes per input byte | 5.19x | 7.71x | **3.91x** |
| allocations per input KiB | 190.9 | 140.6 | **40.3** |
| **hold two versions** (v1+v2 ÷ v1) | **1.02x** | 2.00x | 2.00x |
| allocations, one persistent edit (749 edits) | 6,970 | 1,379,128 | **2,996** |
| allocations, one destructive edit | 6,970 | **525** | 1,498 |
| `locate` → absolute range: allocations | 2,736 | **0** | **0** |
| `locate` complexity | O(depth) | O(preceding tokens) | **O(1)** |
| build+serialize 10 MB (machine-dependent) | 104 ms | 76 ms | **40 ms** |
| lines of code | 288 | **163** | 170 |
| K1 round-trip | 750/750 | 750/750 | 750/750 |
| K2 edit locality | 749/749 | 749/749 | 749/749 |

Token interning gives A **84.2% reuse** on real config, which is why it beats B
on memory despite the extra machinery.

## Decision

**A — the green/red tree.** Not because it wins the table; it loses most of it.
Because the two columns it does win are the two that decide correctness rather
than resources.

**First: holding two versions costs 2% instead of 100%.** A structural merge is
tree *composition* — the merged result is mostly subtrees taken unchanged from
base, ours and theirs. In a green tree that splice is an `Rc` bump, and the
result shares storage with its inputs. In an arena it is a deep copy with every
index renumbered, and renumbering is exactly the kind of code that fails
silently rather than loudly. konflux's M6 conflict resolver needs many live
versions for undo; strukt's bulk refactors want a preview against the original.
This is the shape of the workload, and A is the only candidate built for it.

**Second: handles are typed.** C's `u32` indices carry no provenance, so an
index from one tree used against another reads the wrong element and returns a
plausible wrong answer. MASTER_PLAN §0 ranks soundness above everything and
defines the failure as *"never silently wrong."* A representation whose central
operation can be silently wrong under a plain integer mix-up is the wrong
foundation for a merge tool, whatever it costs in nanoseconds. Newtyped or
generation-tagged indices would demote that to a runtime check; A gets it at
compile time for free.

**Why the memory column did not decide this, despite §3.1 naming it.** The
corpus says config files are small: p50 811 bytes, p90 43 KiB, max 266 KiB. At
5.19x, the p90 file is a 223 KiB tree and the worst file in 1,000 is 1.4 MiB.
The gap between 3.91x and 5.19x is real and it is not binding at this scale. It
would bind for bigsheet, which opens a 5 GB CSV — and that is a different crate
with a different answer, noted below.

**B is dropped outright.** It is dominated: worse memory than both, worse
persistence than A, worse everything than C except a destructive-edit path we
cannot use when a merge needs its inputs intact. This is worth recording plainly
because §3.1 framed the choice as *"green/red or owned token tree"* — measured,
that pair contains a clear loser, and the strongest challenger was not in it.

## Consequences

**What gets harder.**
- A red layer must be built and maintained. `locate` allocated 2,736 times over
  749 calls in the spike — an **upper bound**, since the naive `Rc<Red>` per step
  here is what real rowan amortises with an internal free-list. If profiling at
  M2 shows span lookup is hot, that free-list is the fix, and it is a contained
  one.
- 288 lines against B's 163, and `Rc` in the type of everything.
- Build is 2.6x slower than the arena. For a merge driver on one file this is
  invisible; if a repo-wide `strukt` grep ever makes it visible, it is a
  measured regression with a named cause, not a mystery.

**What we are betting on.** That structural sharing is what a merge kernel
actually needs, and that config files stay small. Both are checkable, and the
second is already checked: 1,000 real files, p90 43 KiB.

**When to revisit — falsifiable, not vibes.**
1. If konflux M3's merge turns out **not** to splice shared subtrees — if the
   result is rebuilt from scratch anyway — then A's decisive advantage is
   imaginary and C wins on every remaining axis. Re-open this ADR at M3.
2. If a corpus file exceeds ~10 MB, or the p90 crosses ~1 MB, memory becomes
   binding and the arena's 25% saving starts to matter.
3. **bigsheet is expected to answer differently.** Its workload is a 5 GB file
   where nothing is shared and everything is scanned. Nothing in this ADR
   applies to it, and it should not inherit this decision by default.

**Not decided here.** Interning policy (the 64-byte cutoff is a spike constant,
not a considered threshold), the exact red-layer caching strategy, and whether
green nodes are `Rc` or a custom thin pointer. Those are M1 implementation
decisions and land with their own measurements.

## Proof impact

- **K1** — verified 750/750 for every candidate, so this decision does not rest
  on round-trip correctness; all three deliver it. The spike's lexer is total
  and lossless independently, so a K1 failure would have been attributable.
- **K2** — verified 749/749 against an oracle computed from the lexer, not from
  any tree under test.
- **K3** — the deterministic half of the spike report is byte-identical across
  runs (`sha256 c09a10c6…`), checked with `xtask assert-equal`. Timings are
  reported separately and labelled machine-dependent; they are not a published
  benchmark and carry no methodology page (Appendix C).
- Gates touched: `gate/golden`, `fuzz-smoke`, `gate/determinism`, `gate/miri`
  all re-point from `roundtrip_identity` onto the real parse/serialize pair at
  M1 (ADR-003).

**One honest note on the numbers.** The first run of this spike reported K2 as
749/**750** for all three candidates. That was not a tree failure — one corpus
file is 251 bytes of pure comment with no editable token, so the edit was
*skipped*, and the harness was counting a skip as a failure. The counter now
separates attempted from passed and names the skipped file. A metric that
conflates "not tried" with "tried and failed" is the same defect in the other
direction as a vacuous green gate, and it was caught here only because three
independent implementations failed identically.

## Reproduce

```bash
corpora/fetch.sh && cargo run --release --manifest-path spikes/adr-001-cst-representation/Cargo.toml
```
