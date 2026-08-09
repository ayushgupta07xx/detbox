# ADR-010: konflux is the flagship (D3), decided without the validation signal

**Date:** 2026-08-09 · **Status:** accepted (2026-08-09)

## Context

**D3** (§16) is the flagship confirmation: konflux or bigsheet. It gates konflux
M2 and everything after it, because M2–M6 are konflux-specific — structural
diff, 3-way merge, a git merge-driver — where M1 was not. The kernel M1 built
serves either product, which is why M1 was safe to finish with D3 open.

§11 makes D3 a *read*: post the validation drafts, then confirm konflux on ≥50
upvotes or ≥20 "I hit this weekly" replies, and pivot to bigsheet on a weaker
signal.

**That read did not happen.** The two drafts in `docs/validation/` are written
and unposted, and Ayush made the call directly on 2026-08-09. Recording that
plainly is the whole reason this ADR exists: the repository must not imply
evidence it does not have, and a later reader finding "konflux confirmed" in
MILESTONES is entitled to know it was confirmed by judgement rather than by
signal.

## Options

- **A — konflux.** The plan's default. M1's kernel already points at it, and
  §4.1's moat (K8s-aware list merging, format-preserving output) is the piece
  incumbents lack. Unvalidated: nobody outside this repo has said they want it.
- **B — bigsheet.** §11's named pivot and, per §4.1, the broadest audience.
  Costs the most: a Tauri app, DuckDB, a virtualized grid, and D2 unresolved —
  and it needs the CSV/Parquet formats the kernel does not speak yet.
- **C — Post first, decide in a week or two.** The plan as written. Buys real
  evidence at the cost of stalling M2 behind a mockup, two posts, and a wait
  whose length nobody controls.

## Decision

**A — konflux, decided now, on Ayush's own read rather than on the signal.**

The honest form of the reasoning is *"the evidence C would have bought is worth
less than the fortnight it costs, and the plan's own signal thresholds are weak
instruments."* `docs/validation/README.md` already says as much in its own
words: it ranks upvotes as the **weakest** signal and states that "a reply
naming a specific painful merge is worth more than ten upvotes." A gate whose
headline number can be met while telling you almost nothing is a gate worth
overriding deliberately — which is different from ignoring it.

Three things make A the cheaper bet independent of any signal, and they are
reasons the repository can already defend:

1. **M1 already committed to it.** The corpus is 750 Helm/kustomize/Terraform
   YAML files and 250 JSON — konflux's inputs. Choosing B would leave that
   corpus, and the conformance work built on it, serving a product two phases
   away.
2. **B is gated on an open decision.** bigsheet needs D2 (Tauri) confirmed;
   konflux needs nothing that is not already decided.
3. **The pivot stays cheap.** §11's own escape hatch is that the kernel survives
   either branch. If konflux's launch reads as a failure by §14's kill rule,
   bigsheet is still there and the kernel it needs is more built, not less.

> **On the provenance of that reasoning.** It is assembled from arguments the
> master plan already makes, not from a rationale Ayush stated separately — the
> instruction was "build konflux first". It was offered for replacement before
> sign-off and **accepted as written on 2026-08-09**, so this is now the record.
> Noted rather than quietly dropped, because §13's whole point is that the why
> trail be truthful about where each why came from.

## Consequences

- **konflux M2 is unblocked** and Phase 1 proceeds: M2 structural diff → M3
  3-way merge + P2/P3 → M4 merge-driver → M5 K8s semantics → M6 TUI → Launch 1.
- **Phase 0's §11 exit gate is closed by decision, not by measurement.** This is
  the first time this repository has resolved a gate that way, and it should
  stay the last unless the reason is written down as it is here.
- **The unvalidated bet is deferred, not removed.** It comes due at §14's kill
  rule: 8 weeks post-launch, <100 meaningful engagements and zero organic
  issues, and konflux freezes. Deciding D3 early moves that risk to launch day
  instead of retiring it.
- **The validation drafts are not obsolete.** They stop being D3's input and
  become pre-launch audience work — the same question, asked before there is
  something to show, still tells us which incumbent is the real bar (§14).
- **bigsheet's slot is now Phase 3 as planned**, and D2 (Tauri) is no longer on
  anyone's critical path until then.

## Proof impact

None. No invariant, gate, threshold or golden is touched — this is a sequencing
decision, and it is in `adr/` because §13 says the "why" trail is a credibility
artifact, not because it changes a proof. What it *does* change is which proof
obligations come next: **P4** (differential, arming at M2), then **P2** and
**P3** at M3, which is where konflux's soundness claim actually lives.
