# Spike: `core-cst` representation (ADR-001)

**Question.** MASTER_PLAN §3.1 defers `core-cst`'s tree representation to
ADR-001, "made after a 2-day spike comparing edit ergonomics and memory
footprint." MILESTONES adds: *measured, not argued*.

**Answer.** [ADR-001](../../adr/ADR-001-cst-representation.md). Raw output in
[`RESULTS.txt`](RESULTS.txt).

## Run it

```bash
corpora/fetch.sh
```

```bash
cargo run --release --manifest-path spikes/adr-001-cst-representation/Cargo.toml
```

## Method

One shared lossless lexer (`lex.rs`) produces the token stream and the nesting
plan, so all three candidates build **structurally identical trees** — 235,552
nodes and 983,299 tokens each — and representation is the only variable. The
lexer is asserted total (`covered() == src.len()`) before any tree is judged, so
a K1 failure downstream would be the tree's fault and not the lexer's.

| File | Candidate |
|---|---|
| `green_red.rs` | **A** — immutable refcounted green nodes, interned tokens, red layer on demand |
| `owned.rs` | **B** — every node owns its children, every token owns its bytes |
| `arena.rs` | **C** — one `Vec<Elem>`, `u32` child links, tokens as spans into the source |

Measured per file, over 750 real YAML files (10,043,614 bytes):

1. **K1** — `serialize(build(x)) == x`, byte-identical.
2. **K2** — replace one token, then compare against an expected output computed
   straight from the lexer. The oracle is independent of every tree under test.
3. **Memory** — live heap bytes held by the tree, counted by a global allocator
   in `alloc.rs`. Not RSS: RSS is polluted by allocator arenas and page
   granularity and is not reproducible. These counts are exact and identical on
   every run.
4. **Persistent edit** — live bytes to hold the pre-edit *and* post-edit trees
   at once. This is the number that decided the ADR.
5. **`locate`** — absolute source range of a token, which konflux needs for
   every span-anchored conflict.

The edit target is deterministic (the `Word` token nearest the middle of the
token stream) and the replacement text is a fixed 10 bytes, chosen to differ in
length from most tokens so a length-change bug cannot hide.

## Reading the numbers honestly

- The **deterministic** section of `RESULTS.txt` is byte-identical across runs,
  verified with `cargo xtask assert-equal`.
- The **timings** section is machine-dependent and clearly marked. It is a
  relative, same-machine sanity check, not a benchmark: no methodology page, no
  fixed runner. Publishing it as one would be a cherry-picked benchmark, which
  Appendix C bans permanently.
- `locate` allocations for candidate A are an **upper bound**. The `Rc<Red>` per
  navigation step here is naive; real rowan amortises it with an internal
  free-list. The shape of the cost is real, the constant is pessimistic, and
  ADR-001 says so.
- One corpus file (251 bytes, entirely comments) has no editable token. It is
  **skipped and named**, not counted as a pass or a failure.

## Status

Frozen. The question is answered. If it reopens — ADR-001 lists three
falsifiable triggers — this is re-run, not edited.
