# `spikes/` — decision evidence

A spike is built to answer one question, measured once, cited by an ADR, and
then left alone. It is **not product code**: it never ships, it is excluded from
the workspace (see the root `Cargo.toml`), and it is not subject to the
workspace lint law or the CI gates.

It is kept rather than deleted because MASTER_PLAN §13 makes the ADR corpus a
standing artifact — *"the 'why' trail a rushed repo never has"* — and an ADR
that says "measured, not argued" is only worth reading if the measurement can be
re-run by whoever is reading it.

| Spike | Question | ADR |
|---|---|---|
| `adr-001-cst-representation` | Green/red tree, owned token tree, or flat arena for `core-cst`? | [ADR-001](../adr/ADR-001-cst-representation.md) |

## Rules

- A spike answers **one** question and stops.
- Its numbers must be reproducible by a command written in the ADR.
- Deterministic measurements (counts, bytes, pass/fail) and machine-dependent
  ones (timings) are reported **separately and labelled**. Timings from a spike
  are never published as benchmarks — that needs a methodology page, and
  Appendix C bans anything less.
- When the ADR is accepted the spike is frozen. If the question reopens, the
  spike is re-run, not quietly edited.
