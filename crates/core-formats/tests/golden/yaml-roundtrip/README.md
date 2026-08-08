# YAML round-trip suite — K1

`serialize(parse(x)) == x`, byte-identical, for every file here.

**There is no `expected` file.** The input *is* the expectation. That is not a
convenience — it means a K1 case **cannot be doctored**. The only way to make a
failing case pass is to delete it, which `golden-guard` catches and a shrinking
case count makes obvious. Every case file is raw bytes; `.gitattributes` marks
this tree `-text -diff -merge` so git never rewrites a line ending here.

## Where the case list came from

MILESTONES M1 asks for suites *"seeded from the 1,000-file corpus."* Copying
corpus files in would vendor third-party bytes that ADR-004's fetch-script
design exists to avoid, and would produce a suite whose coverage nobody could
state. So the corpus was **surveyed** instead, and the case list derives from
what it actually contains:

```bash
cargo xtask corpus-survey
```

Measured over 750 YAML files / 10,043,614 bytes of Helm charts, kustomize
overlays and Kubernetes manifests:

| Case | Construct | Corpus share |
|---|---|---:|
| `010` | `key:` with no value — **not** `key: null` | 98.3% |
| `020` | double-quoted scalars, escapes, `\uXXXX` | 67.3% |
| `030` | flow collections `{a: 1}` / `[1, 2]` | 55.9% |
| `040` | **Helm Go templating** `{{ ... }}` | **41.2%** |
| `050` | comments, including unindented and un-spaced | 40.3% |
| `060` | blank lines as authorial spacing | 36.5% |
| `070` | single-quoted scalars, `''` escaping, `#` inside quotes | 28.0% |
| `080` | block scalars `\|`, `>`, `\|-`, `\|+`, explicit indent | 25.7% |
| `090` | multi-document `---` / `...` | 13.5% |
| `100` | no final newline | 8.5% |
| `110` | trailing comments with column alignment | 5.5% |
| `120` | anchors and aliases | 5.2% / 0.9% |
| `130` | 10-level nesting | 4.9% |
| `140` | trailing whitespace | 3.2% |
| `150` | 1-, 3- and 4-space indentation | 1.3% |
| `160` | non-ASCII: emoji, CJK, accents, RTL | 1.2% |
| `170` | 600-column line | 0.4% |

### The finding that mattered

**41.2% of the corpus contains Helm's `{{ }}`** — text that is not YAML at all.
MASTER_PLAN §3.1 describes the verbatim escape hatch as a fallback for "exotic
YAML tags, weird encodings." The corpus says otherwise: for two files in five,
preserving-without-understanding is the *main* path, not the exception. Case
`040` is a realistic Helm deployment template, not a toy.

## Cases the corpus does not contain

Every construct below measured **0.0%** in the survey. They are here anyway,
because K1 is a claim about YAML, not a claim about our corpus — and a construct
we have never seen is precisely the one a parser gets wrong.

| Case | Construct |
|---|---|
| `200` | merge keys `<<:`, including `<<: [*a]` |
| `210` | tags: `!!str`, `!!binary`, `!Custom`, verbatim `!<...>` |
| `220` `221` | CRLF, and mixed CRLF/LF in one file |
| `230` | UTF-8 BOM |
| `240` | tabs inside scalars and block scalars |
| `250` | invalid UTF-8 mid-document |
| `260` | `%YAML` / `%TAG` directives |
| `270` `271` | empty file; a single newline |
| `280` | control bytes, including NUL |
| `290` | trailing garbage after a valid document |

## Status

**RED, deliberately.** konflux M1 is at the oracle stage: there is no YAML
parser, `Yaml::parse` returns a `ParseReport` saying so, and all 29 cases fail
with that one reason. Observed failing on 2026-08-08 before any parser was
written — a test never seen red is not known to test anything (MASTER_PLAN §8).

Adding a case is welcome and is a reviewed change (§9.3). **Removing or
weakening one requires `[NEEDS-AYUSH-APPROVAL]`.**
