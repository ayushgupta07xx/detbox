# ADR-014: Helm templates are opaque lines carried beside the data

**Date:** 2026-08-09 · **Status:** proposed

## Context

145 corpus files — one in five — were refused for a single shape:

```yaml
{{- if .Values.rbac.create -}}
kind: ClusterRole
{{- end }}
```

A Helm chart is not YAML. It is a **template that becomes YAML**, and `{{- if }}`
is control flow: it decides whether the lines around it exist at all. The
semantic view models a mapping as key–value pairs, and a template line has no
key. There was nowhere to put it.

## Options

- **A — Keep refusing.** Safe; konflux stays unusable on a fifth of real charts,
  which is the audience §4.1 names first.
- **B — Opaque lines.** Carry template lines beside the data, compared by source
  text. Diffs work on real charts; konflux does not claim to understand what the
  template *does*.
- **C — Model the conditionals as structure.** Correct, and much larger: it
  means a chart-aware layer that knows a `{{- if }}` guards a range of lines,
  which is a different product from a YAML kernel.

**Not an option: ignoring template lines.** Two charts differing only in
`{{- if .Values.a }}` versus `{{- if .Values.b }}` would then diff as identical.
That is the silently-wrong failure §0 ranks first, and it is the reason this was
brought to Ayush rather than guessed at.

## Decision

**B**, chosen by Ayush on 2026-08-09.

`SemanticNode::Templated { inner, templates }` wraps a collection that contains
template lines. Three sub-decisions carry the weight:

**1. A wrapper, not a field on `Mapping`.** Every collection without templates
keeps exactly the shape and behaviour it had, so nothing already proven moves.
The cost is real and worth stating: a file gaining its *first* template compares
as a different shape and reports a replacement rather than a fine-grained diff.
A chart acquiring a conditional is a large change anyway, and the alternative
was reshaping a type that every existing golden depends on.

**2. Templates are an ordered list, never matched by text.** This is the
sub-decision that is actually load-bearing. Charts repeat themselves — two
identical `{{- end }}` lines in one collection is ordinary — and matching by
text would let one be **deleted with nothing noticing**. Deleting an `{{- end }}`
changes what the chart renders. An ordered list aligned by LCS catches it, and a
test asserts exactly that.

**3. Template changes are reported at the collection's path**, not at a path of
their own. A `{{- if }}` has no key, and no index a reader could point at that
would not either collide with a real key or mean nothing. `before`/`after` say
*which* template changed; the path says *where*. This keeps `--json` at schema
version 1 — no field added, no path segment invented.

### What this buys and what it does not

konflux can now diff real Helm charts. It **cannot** tell you that a key is
guarded by a condition, and a template that *moves* reads as a change beside
whatever it lands next to. Those are approximations, they are visible in the
output rather than hidden by it, and they belong in the README before launch.

Templates used as **values** — `name: {{ include "chart.fullname" . }}` — needed
nothing: a template is a single `VERBATIM` token, so it already resolved like
any other scalar. That was discovered by probing rather than assumed, and it is
pinned by a golden case so it cannot regress on the way past.

## Consequences

- **Coverage: YAML 64.7% → 72.1%**, and the largest bucket in the queue
  disappears. What replaces it is a long tail of small, ordinary YAML gaps.
- **A new refusal appears:** a template line that *owns* indented lines (26
  files), where the template and the block beneath it interact in a way this
  flat reading does not capture. Refused, not guessed.
- **The launch README must say what this does not do.** Claiming konflux
  "understands Helm" would be the overclaim §12 exists to prevent; it reads
  charts and diffs them, which is a smaller and true sentence.
- **C is not foreclosed.** If chart-aware merging becomes the point, `Templated`
  is where it grows, and the goldens written here keep their meaning.

## Proof impact

No invariant changes. `--json` stays at `SCHEMA_VERSION` 1. Adds golden cases
`300` (condition changed) and `310` (template as a value), and the duplicate-
template deletion test that makes sub-decision 2 non-optional.

## Reproduce

```bash
cargo test -p konflux && cargo xtask semantic-coverage
```
