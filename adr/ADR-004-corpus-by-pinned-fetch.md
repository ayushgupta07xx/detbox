# ADR-004: Corpora are fetched at pinned SHAs with exact yields, never vendored

**Date:** 2026-08-07 · **Status:** proposed

## Context

konflux **P1** requires K1 to hold on "a corpus of ≥1,000 real-world files."
MASTER_PLAN §2 allows submodules or fetch scripts; §10 requires that corpus
files respect upstream licences and prefers fetch scripts over vendored copies.
Whichever mechanism is chosen has to make "≥1,000 files" a *fact* — a number
somebody can reproduce — rather than a figure in a README.

## Options

- **A — Vendor the files into the repo.** Reproducible and offline, but
  redistributes third-party code, bloats every clone, and quietly makes us a
  redistributor with the licence obligations that implies.
- **B — Git submodules.** Native and pinned, but pulls whole repositories
  (bitnami/charts alone is gigabytes) and gives no control over *which* files
  count toward the corpus.
- **C — Fetch scripts over partial clones at pinned commit SHAs, with per-source
  include patterns and exact file counts.** More script to write and maintain;
  full control over size, composition and licence recording.

## Decision

**C.** `corpora/sources/*.sources` declares each source's repo, a full 40-char
commit SHA, its SPDX licence, its licence file, its include patterns, and its
exact file count. `corpora/fetch.sh` does a `--depth 1 --filter=blob:none`
partial clone at that SHA, selects files in include-pattern order then byte-wise
sorted order, copies the upstream `LICENSE` alongside and records its SHA-256,
and writes a sorted `MANIFEST.tsv` of every file's hash — the receipt saying
exactly which bytes a proof ran against.

Three properties are enforced rather than intended:

1. **Pinned.** `cargo xtask corpus-verify` rejects any rev that is not a
   canonical 40-char lowercase SHA. Branches and tags move; a moving corpus
   turns "P1 is green" into "P1 was green against whatever upstream looked like
   this morning," which is not a proof.
2. **Exactly capped.** A source yielding a number other than its `max_files` is
   a hard error, not a warning. Because the rev is pinned, the yield is
   deterministic, so "1,000 files" is checkable. This already caught a Terraform
   source silently contributing 67 files instead of 85.
3. **Licence-clean.** An SPDX allow-list — Apache-2.0, MIT, BSD-2-Clause,
   BSD-3-Clause, ISC, MPL-2.0 — is enforced offline in CI. `hashicorp/terraform`
   is deliberately excluded: it relicensed to BUSL-1.1.

The manifest check is **offline** and runs on every PR; the network fetch is
scheduled weekly, where its job is to catch the day an upstream disappears,
force-pushes the pinned SHA away, or relicenses.

## Consequences

- The corpus is not available in an air-gapped clone without one network fetch.
  Acceptable: a full fetch is about one minute over a partial clone.
- Upstream force-pushing a pinned SHA breaks the fetch. That is the correct
  failure — the alternative is silently proving something against different
  bytes.
- Growing the corpus is a reviewed change to evidence, covered by
  `golden-guard` and `CODEOWNERS`.

## Proof impact

Underwrites konflux **P1** (K1 on ≥1,000 real-world files), lockproof **P1**
(top-1,000 lockfiles per ecosystem) and bigsheet **P2** (nasty-real-world CSV).
Adds the offline manifest check to the §8 determinism gate and a scheduled fetch
job that asserts the 1,000-file count.
