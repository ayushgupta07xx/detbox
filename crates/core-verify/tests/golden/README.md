# Golden files — evidence, not code

**MASTER_PLAN §8, the anti-reward-hacking law, applies to every file in this
tree.** No one may edit a golden file, loosen a threshold, or delete/skip/weaken
a test in order to make CI pass.

If a golden is genuinely wrong:

1. **Stop.** Do not change it.
2. Propose the change in the PR description under a `[NEEDS-AYUSH-APPROVAL]`
   header, with the justification and the exact byte-level delta.
3. Wait for explicit sign-off. Golden-file changes are a non-delegable human
   review point (§9.3).

There is deliberately no `--bless` / `--update-goldens` mode in the runner.
Adding one requires an ADR.

## Layout

```
<suite>/
  <NNN-slug>/
    input      raw bytes in
    expected   raw bytes out
```

Cases are discovered in byte-wise sorted order. An empty suite is an **error**,
not a pass (`core-verify` invariant V4).

## `roundtrip-identity`

The K1 oracle in its weakest true form: `serialize(parse(x)) == x` for the empty
grammar. Every case is a byte sequence that a careless implementation would
normalise — CRLF, trailing whitespace, YAML anchors and comments, invalid UTF-8,
a BOM, an embedded NUL.

At konflux M1 these same cases are re-pointed at the real YAML/JSON
`parse`/`serialize` pair, and the suite grows. They are not deleted: an input
that round-trips today must still round-trip once there is a parser.

`.gitattributes` marks this tree `-text -diff -merge` so git never rewrites a
line ending here.
