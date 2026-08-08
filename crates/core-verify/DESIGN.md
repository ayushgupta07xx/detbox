# DESIGN — `core-verify`

## Scope
The proof harness (MASTER_PLAN §3.3). A library plus internal CLI used by every
other crate: golden runner, round-trip fuzzer, property kit, differential
runner, conformance adapters, report emitter.

**In scope:** running proofs and reporting them.
**Out of scope:** being a proof. This crate must never contain product logic,
and product crates must never contain their own bespoke harnesses.

## Invariants
| ID | Statement | How it is checked |
|----|-----------|-------------------|
| V1 | Golden files are read, never written, by anything in this workspace | no blessing mode exists; `golden-guard` CI job; CODEOWNERS |
| V2 | A failure prints case path, first differing offset, and a windowed view of both sides | unit test `a_wrong_transform_is_caught` |
| V3 | The harness is itself deterministic: sorted discovery, ordered reports, no wall-clock in report fields | unit test `discovery_order_is_sorted_not_filesystem_order` |
| V4 | A suite with zero cases is an error, not a pass | unit test `an_empty_suite_is_an_error_not_a_pass` |
| V5 | A published rate is regenerated and byte-compared, never hand-maintained | `the_published_report_is_what_this_run_measures`; ADR-009 |

## Current milestone
**konflux M1 — conformance adapters, armed.** Golden runner (Phase 0) plus the
JSONTestSuite and yaml-test-suite adapters, the two-rate ratchet (ADR-008), and
the published report emitter (ADR-009).

The adapters live here rather than in the test that first needed them because
§3.3 makes them part of the harness, and because two callers now read the same
suites — the gate and the report generator. A second copy of "how a suite is
laid out" is a second place for a case count to drift.

Build-out order: differential runner (M2) → property kit incl. the merge algebra
of §4.1 P2 (M3) → the rest of the report emitter, human/JSON/badge (M4).

## Downstream products
`veritas` (§4.5) is the report emitter productized. `coverify` (§5) is this
whole crate productized, plus a deterministic simulation executor. Design
decisions here are therefore product decisions later — write the ADR.

## Proof obligations this crate carries
Every proof obligation in the plan is executed by this crate. It is the spine.
