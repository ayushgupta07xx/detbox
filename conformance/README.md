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

See [ADR-008](../adr/ADR-008-conformance-semantics.md). With today's stubbed
parser, which does nothing but return `Err`:

```
json-test-suite: accept 0/95 (0.0%)   reject 188/188 (100.0%)
yaml-test-suite: accept 0/308 (0.0%)  reject 94/94  (100.0%)
```

Blended, those would publish **66.4%** and **23.4%** conformance — from a
function whose entire body is `return Err`. So there is no blended figure and no
method to compute one. Both rates ratchet independently, so accepting more can
never be paid for by rejecting less.

## `thresholds.tsv`

What we currently claim. **Ratchets one way**: raising is a normal reviewed
change, lowering requires `[NEEDS-AYUSH-APPROVAL]` (§8), and `golden-guard`
covers the path.

`unrecorded` is an **error**, not a free pass — a conformance rate is
deterministic, unlike a benchmark baseline that may honestly be `uncalibrated`
(ADR-006).

## Status

**RED.** Every threshold is `unrecorded` because there is no parser. The first
real rates get recorded at M1, under review.
