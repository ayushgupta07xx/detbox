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

## Current milestone
**Phase 0 — scaffold.** Golden runner only, minimal, so the §8 golden gate is
wired and non-vacuous from commit one (ADR-003).

Build-out order: conformance adapters (M1) → differential runner (M2) →
property kit incl. the merge algebra of §4.1 P2 (M3) → report emitter (M4).

## Downstream products
`veritas` (§4.5) is the report emitter productized. `coverify` (§5) is this
whole crate productized, plus a deterministic simulation executor. Design
decisions here are therefore product decisions later — write the ADR.

## Proof obligations this crate carries
Every proof obligation in the plan is executed by this crate. It is the spine.
