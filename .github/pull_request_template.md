## Milestone item

<!-- One paragraph: which MILESTONES.md item this is, and its acceptance
     criteria. MASTER_PLAN §9.2 step 2. One item per PR. -->

## Proof Delta

<!-- REQUIRED. MASTER_PLAN §9.2 step 7: "what is now proven that wasn't before."
     Name the §8 gates that cover it, and paste the passing command output.
     A proof obligation is never claimed without the output. -->

**Now proven:**

**Covered by §8 gates:**

**Command output:**

```

```

## Oracle-first

<!-- MASTER_PLAN §8: tests/goldens/fuzz targets are written and confirmed RED
     before implementation. Link the commit where they were red. -->

- [ ] The oracle was written first and confirmed red
- [ ] Red -> green -> ADR -> PR

## Self-review (MASTER_PLAN §9.2 step 6)

- [ ] No determinism leaks: no `HashMap` in an output path, no wall-clock, stable
      sorts, fixed float formatting, seeded randomness only, OS-normalised paths,
      locale-independent formatting (§9.5)
- [ ] New dependencies each carry a one-line justification below (§2)
- [ ] Errors carry spans, not strings
- [ ] `--json` output is stable within its schema version
- [ ] Diff is ≤ ~600 lines

## New dependencies

<!-- One line of justification each, or "none". -->

none

## ADRs

<!-- Every non-obvious decision gets an ADR in the SAME PR (§9.2 step 5).
     Link them, or "none — no non-obvious decisions in this change". -->

---

<!-- ============================================================ -->
<!-- Add the header below ONLY if this PR changes evidence:       -->
<!--   golden files, thresholds, baselines, corpus manifests,     -->
<!--   fuzz seeds, or removes/skips/weakens a test.               -->
<!--                                                              -->
<!--   [NEEDS-AYUSH-APPROVAL]                                     -->
<!--                                                              -->
<!-- ...with the justification. The `golden-guard` CI job fails    -->
<!-- without it. This is MASTER_PLAN §8, the anti-reward-hacking   -->
<!-- law, and it is not negotiable.                               -->
<!-- ============================================================ -->
