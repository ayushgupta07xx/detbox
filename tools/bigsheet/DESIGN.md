# DESIGN — `bigsheet`

## Scope
Offline large-data workbench: open CSV/Parquet/XLSX/JSONL far past Excel's
limits; virtualised grid; SQL pane (embedded DuckDB); exact row/column counts
always visible.

**Non-goals:** charting suites, collaboration/cloud, Excel formula breadth.

## The brand promise
**Never silently truncate.** Rows in == rows reported, always, everywhere,
including on malformed input. This is the anti-Excel promise and it is a
property test, not a slogan.

## Proof obligations (MASTER_PLAN §4.3)
| ID | Statement |
|----|-----------|
| P1 | Exactness — SQL results differentially tested vs DuckDB CLI + hand-computed goldens; any divergence blocks release |
| P2 | Fidelity — CSV↔Parquet round-trip preserves values + types, or itemises exactly what changed (report golden-tested) |
| P3 | No-truncation — property test: rows in == rows reported |
| P4 | Performance **budgets** — honest framing. Open-time and scroll-latency targets on NYC Taxi and the Stack Overflow dump, published with methodology |

Note P4's wording: **budgets, not proofs.** Do not let it drift into
proof-language in any public artifact.

## Absorbed scope
Structured-log ingestion (logfmt et al.) enters as *input formats*. We do not
build a second query engine.

## Current milestone
**Phase 0 — scaffold.** Phase 3, weeks 14–26. GUI stack is Tauri (Decision D2).
Designated pivot flagship if the Phase 0 signal for konflux is weak (D3).
