# ADR-016: The CLI has no argument-parsing dependency, and the exit code tracks meaning

**Date:** 2026-08-09 · **Status:** proposed

## Context

konflux M2 asks for "side-by-side CLI output via `core-cli`". Until now nothing
in this repository was runnable — nine crates, a proof harness, 79% semantic
coverage, and no way for a person to point it at two files. `core-cli` was a
42-line doc comment whose own DESIGN said *"first real content at konflux M2"*.

Building it forced two decisions, one about dependencies and one about what a
number means.

## Decision 1 — hand-rolled argument parsing

**No `clap`, no argument-parsing dependency in `core-cli`.**

The obvious objection is that everyone uses clap and it is good. The reasons not
to, here specifically:

- **The surface is small on purpose.** §4.2 describes strukt's query language as
  *"small and boring on purpose"*, and the flag surface inherits that restraint:
  `--json`, `--check`, `--help`, `--`, positionals. That is a afternoon's parser
  and a permanent dependency otherwise.
- **`core-cli` is a leaf that every tool imports.** A dependency here is a
  dependency in all nine products and in everything downstream of the published
  crates, and `cargo-deny` audits all of it forever.
- **xtask already proved the pattern**, with a stricter rule (zero third-party
  deps) and no friction.

**What would change this:** shell completions. §4.2 lists them for strukt, and
generating completions by hand is genuinely worse than taking the dependency.
That is the moment to revisit — not before, and the revisit is an ADR.

## Decision 2 — the exit code tracks meaning, not bytes

Four codes, and the interesting ones are the third and fourth.

| | |
|---|---|
| `0` | no **semantic** change |
| `1` | semantic changes found |
| `2` | usage error |
| `3` | **refused** — konflux cannot model this input |

**`0` for a formatting-only difference is the product thesis expressed as a
number.** Two files whose keys were reordered differ in bytes and not in
meaning, and a CI job asking "did anything change?" must not be woken for it. A
line differ cannot make that distinction; making it is why konflux exists. The
change is still *printed* — it is simply not a *finding*.

**`3` is separate from `1` because a merge driver must tell them apart.** At M4
konflux becomes a git merge driver, and "these differ, resolve them" and "I
cannot read this, hand it back to git" demand opposite responses. Collapsing
both into 1 would make the driver take one side of a file it never understood,
which is ADR-012's silently-wrong failure arriving through the exit code instead
of through the diff. `core-cli` therefore owns this code, not konflux, so every
future tool inherits the distinction rather than reinventing it.

`>2` in `core-cli`'s C2 was written as "internal failure". `3` is not a failure —
it is a boundary, reported honestly. C2's wording is amended accordingly.

## Consequences

- **konflux is runnable.** `konflux diff a.yaml b.yaml`, `--json`, `--check`.
- **Fourteen CLI tests run the real binary**, because a unit test on an enum
  proves the numbers exist and not that the process returns them. Two of them
  are the ones that would bite hardest in a script: a typo'd path exits `2`
  rather than reading as clean, and piped output carries no ANSI.
- **Colour is off unless stdout is a terminal**, `NO_COLOR` is unset, and
  `--check` is absent. Reading `NO_COLOR` is not a C3 violation: C3 forbids
  inputs that vary run-to-run on the same machine — clock, locale, unseeded
  randomness — and configuration a user deliberately sets is not one of those.
- **Diffing a `.yaml` against a `.json` is a usage error**, not a conversion.
  That is veritas's job (§4.5) and doing it quietly here would be a second
  product hiding inside the first.
- **The human renderer collapses values to one line.** Block scalars and Helm
  templates are multi-line and would otherwise shred the table. `--json` keeps
  them whole; the human view is allowed to be a summary and says so by
  truncating visibly with `...`.

## Proof impact

Discharges `core-cli` **C2** (exit codes, tested through the process) and **C3**
(no colour in a pipe, no clock or locale read). **C1** — `--json` append-only
within a schema version — was already carried by the diff golden suite and is
now also asserted at the process boundary. **C4**, span-rich diagnostics, is
still outstanding: today a refusal is a sentence, not a span.

## Reproduce

```bash
cargo test -p konflux --test cli && cargo run -p konflux -- --help
```
