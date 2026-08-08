# `conformance/` — official suites

MASTER_PLAN §3.3 (conformance adapters) and §4.1 **P4** (published pass rates,
*"including honest failure lists"*).

```bash
conformance/fetch.sh
```

```bash
cargo test -p core-formats --test conformance -- --nocapture
```

Fetched suites are **never committed**, same discipline as `corpora/` (ADR-004):
pinned commit, licence recorded and hashed, `MANIFEST.tsv` receipt, exact case
counts asserted so a moved pin fails loudly.

| Suite | Rev | Cases | Licence |
|---|---|---|---|
| [JSONTestSuite](https://github.com/nst/JSONTestSuite) | `1ef36fa0` | 318 — 95 accept, 188 reject, 35 implementation-defined | MIT |
| [yaml-test-suite](https://github.com/yaml/yaml-test-suite) | `6ad3d2c6` (`data`) | 402 — 308 accept, 94 reject | MIT |

yaml-test-suite is taken from its generated `data` branch: the `src/` form on
`main` stores each case *inside a YAML document*, which would require a working
YAML parser to read the suite that tests our YAML parser. That branch carries no
licence file, so the licence comes from a separately pinned `main` commit —
"the branch we used had no LICENSE" is not a licence (§10).

## Two rates, never one

See [ADR-008](../adr/ADR-008-conformance-semantics.md). Measured before a parser
existed, when `parse` did nothing but return `Err`:

```
json-test-suite: accept 0/95 (0.0%)   reject 188/188 (100.0%)
yaml-test-suite: accept 0/308 (0.0%)  reject 94/94  (100.0%)
```

Blended, those would have published **66.4%** and **23.4%** conformance — from a
function whose entire body is `return Err`. So there is no blended figure and no
method to compute one. Both rates ratchet independently, so accepting more can
never be paid for by rejecting less.

## `REPORT.md` — the publication

[`REPORT.md`](REPORT.md) is **generated**, never hand-written:

```bash
cargo xtask conformance-report --write
```

It carries both rates per suite, the pin each was measured at, and the failure
list **in full** — all 77, not the first 15 a console prints. `gate/conformance`
regenerates it and byte-compares, so an edited number fails the build instead of
becoming a claim, and an improvement that was never republished is equally red.
See [ADR-009](../adr/ADR-009-publishing-conformance-rates.md).

## `thresholds.tsv`

What we currently claim. **Ratchets one way**: raising is a normal reviewed
change, lowering requires `[NEEDS-AYUSH-APPROVAL]` (§8), and `golden-guard`
covers the path.

`unrecorded` is an **error**, not a free pass — a conformance rate is
deterministic, unlike a benchmark baseline that may honestly be `uncalibrated`
(ADR-006).

## Status

**Armed at konflux M1** (ADR-003). All four rates are recorded and published:

| Suite | Accept | Reject |
|---|---|---|
| json-test-suite | 100.0% | 100.0% |
| yaml-test-suite | 100.0% | 18.1% |

Those four numbers are duplicated from `REPORT.md` for the reader's convenience
and are **not** the claim — `thresholds.tsv` is the claim and `REPORT.md` is the
measurement. If this table and `REPORT.md` ever disagree, `REPORT.md` is right
and this one is a typo.

**P4 is not complete.** §4.1 names three suites and toml-test is the third;
`toml` arrives with strukt in Phase 2, so P4 finishes there. The yaml reject rate
is also a standing gate before M3 (MILESTONES).
