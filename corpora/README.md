# `corpora/` — test corpora

MASTER_PLAN §2: *"test corpora as git submodules / fetch scripts."*
MASTER_PLAN §10: *"corpus files respect upstream licenses (fetch scripts, not
vendored copies, where required)."*

**Nothing fetched here is ever committed.** `.gitignore` enforces it. What is
committed is `sources/*.sources` — the manifests that say exactly which bytes a
proof ran against.

## Fetch

```bash
corpora/fetch.sh
```

```bash
corpora/fetch.sh helm-charts
```

```bash
DRY_RUN=1 corpora/fetch.sh
```

## Verify the manifests (offline, blocking in CI)

```bash
cargo xtask corpus-verify
```

## Phase 0 budget — 1,000 files

| Category | Files | Sources | Feeds |
|---|---:|---|---|
| `helm-charts` | 400 | prometheus-community 150, grafana 130, bitnami 120 | konflux P1, M5 |
| `terraform` | 250 | terraform-aws-modules: vpc 65, eks 85, rds 100 | strukt (HCL, Phase 2) |
| `k8s-manifests` | 200 | kubernetes/examples 100, istio samples 100 | konflux M5 semantic layer |
| `kustomize` | 150 | kubernetes-sigs/kustomize 72, argo-cd 78 | konflux P1, M5 |
| **total** | **1,000** | | konflux **P1**: K1 on ≥1,000 real-world files |
| `lockfiles` | — | Phase 4 | lockproof P1 |
| `pdfs` | — | Phase 4 | pdfsurgeon P1, P5 |
| `csv` | — | Phase 3 | bigsheet P1, P2 |

These are **measured** counts, not targets: `fetch.sh` fails if any source
yields a number other than its `max_files`. Verified 2026-08-07 — a full fetch
produces exactly 1,000 files in about one minute over a partial clone.

The three empty categories are directories with a cap of zero until their phase
opens. `cargo xtask corpus-verify` only knows about categories that have a
manifest, so adding one is how a category comes online.

## The four properties, and why each is load-bearing

**Pinned.** Every source is fetched at a full 40-character commit SHA — never a
branch, never a tag. Those move. A moving corpus turns "P1 is green" into "P1
was green against whatever upstream looked like this morning," which is not a
proof. `corpus-verify` rejects anything that is not a canonical SHA.

**Capped, exactly.** Each source contributes exactly `max_files` files, chosen
in include-pattern order and then byte-wise sorted order, and the per-source
counts must sum *exactly* to the category cap. A short yield is a hard error,
not a warning: the rev is pinned, so the count is deterministic, and "1,000
files" in the table above is therefore a fact rather than an estimate. (This
gate has already earned its keep — it caught a terraform source silently
contributing 67 files instead of 85 because an include pattern missed a
directory.)

**Attributed.** The upstream `LICENSE` is copied next to the fetched files as
`UPSTREAM-LICENSE` and its SHA-256 is recorded. If upstream relicenses, that
shows up as a diff in a manifest rather than as a surprise. The licence
allow-list is enforced by `corpus-verify`: Apache-2.0, MIT, BSD-2-Clause,
BSD-3-Clause, ISC, MPL-2.0. Anything else needs a decision, not a default.

**Manifested.** Every fetched file's SHA-256 goes into a sorted `MANIFEST.tsv`.
That file is the receipt for a proof run.

## Sources deliberately not used

- **hashicorp/terraform** — relicensed to BUSL-1.1, which is not on the
  allow-list. The Terraform corpus comes from Apache-2.0 community modules
  instead.
- **kubernetes/website** — mixed CC-BY-4.0 docs and Apache-2.0 code in one tree;
  per-file licence attribution would be guesswork.

## Growing the corpus

Adding a source is a reviewed change: it alters what the proofs run against.
Raising `GLOBAL_FILE_CAP` past 1,000 is a deliberate act, not a side effect —
it lives in `xtask/src/corpus.rs` with a test asserting its value.

**Shrinking a cap, removing a source, or relaxing an include pattern to make a
proof go green is the anti-reward-hacking law (§8) and requires
`[NEEDS-AYUSH-APPROVAL]`.**
