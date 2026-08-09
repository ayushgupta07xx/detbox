# `diff` golden suite — konflux M2

Hand-built cases where **line-based diff is wrong and structural diff is right**
(MILESTONES, M2 item 1). Each case is `a.<ext>`, `b.<ext>`, and an `expected`
holding konflux's `--json` diff output, compared byte-for-byte.

The `expected` files are a **specification written before the implementation**
(§8: oracle first). Nothing computes a diff yet, so the suite is red.

Why the golden is the `--json` output rather than the rendered CLI view, and why
paths are RFC 6901 pointers: [ADR-011](../../../../../adr/ADR-011-diff-golden-contract.md).

## The cases

Two families. The first is where a line diff *cries wolf*; the second is where
it points somewhere unhelpful.

| Case | What a line diff does | What konflux must say |
|---|---|---|
| `010-mapping-keys-reordered` | three lines changed | `moved` · **formatting** at the mapping — mapping order is not meaning |
| `020-scalar-quoting-style` | one line changed | `changed` · **formatting** at `/name` — `web` and `"web"` are the same string |
| `030-json-key-order` | the whole line changed | `moved` · **formatting** — same, in JSON |
| `040-flow-mapping-reordered` | the whole line changed | `moved` · **formatting** — flow mapping order is not meaning either |
| `100-sequence-insert-in-middle` | often renders as *"gamma changed to beta, gamma added"* | one `added` · semantic at `/items/1` |
| `110-nested-value-changed` | one line, no path context | `changed` · semantic at `/spec/template/spec/containers/0/image` |
| `120-key-containing-a-slash` | one line | `changed` · semantic at `/nodeSelector/kubernetes.io~1os` |
| `130-sequence-reordered` | two lines changed | `moved` · **semantic** at `/initContainers` |
| `140-key-removed` | one line removed | `removed` · semantic at `/resources/limits` |
| `150-json-nested-change` | one line changed | `changed` · semantic at `/dependencies/left-pad` |
| `160-flow-sequence-value-changed` | one line changed | `changed` · semantic at `/ports/1` — inside a flow sequence |
| `170-flow-sequence-item-added` | one line changed | `added` · semantic at `/args/0` — `[]` is empty, not a scalar |
| `180-block-scalar-content-changed` | one line changed | `changed` · semantic at `/script` — block at end of file, body owns the final newline |
| `181-block-scalar-followed-by-a-key` | one line changed | `changed` · semantic at `/script` — same content, and the body does **not** own the newline because a line follows |
| `900-identical` | nothing | nothing — **the control**, see below |

**`010` against `130` is the pair that matters most.** Both reorder a
collection. A line diff renders them identically. One is formatting and one
changes what the document means, and a merge tool that confuses them either
invents conflicts or loses changes.

## The control, and what it costs

`900-identical` is the only case a do-nothing implementation passes, and it is
here on purpose: an oracle that no possible output satisfies is as broken as one
everything satisfies, so the empty answer must be reachable and must be pinned.

The other fourteen all fail a diff that reports nothing, including the three
formatting-only cases — which is why formatting changes are *reported* rather
than left as an empty list. `the_suite_is_not_vacuous` asserts exactly this: it
runs a null diff over the suite and requires 14 of 15 cases to fail. If someone
later makes formatting changes silent, that test goes red rather than the suite
quietly becoming satisfiable by returning `[]`.

## Deliberately not here yet

`180` against `181` is a lexer wart pinned as evidence: the same logical block
scalar has different node text depending on whether anything follows it. Safe
direction (it over-reports), and now written down rather than lurking.

Two constructs whose *right answer is not yet obvious*, and guessing in a golden
file is worse than omitting it, because a golden is evidence and this one would
be a guess wearing evidence's clothes:

- **Comments.** Which node owns a comment — the key below it, the mapping
  containing it, the blank line above — is a trivia-attachment decision that
  belongs with the CST walk, not ahead of it.
- **Re-indentation.** Changing a block's indent changes bytes belonging to no
  single node. Attributing it needs the algorithm to exist.

Both are M2 work and both get cases in the implementation PR, as normal reviewed
golden additions. Neither is optional: konflux's pitch is *"comments and key
order preserved"*, and the comment half is currently proven by nothing.

## Adding a case

Adding cases is free. Editing or deleting an existing `expected` to make a run
pass is the §8 anti-reward-hacking law, and `golden-guard` covers this path.
