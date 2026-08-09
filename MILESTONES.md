# MILESTONES

## Where things stand — read this first

**Updated 2026-08-09.** This section is the session handoff: a new session should
need this file, `ENGINEERING.md` and the ADRs, and nothing else.

**Done.** Phase 0 accepted. konflux **M1 is finished** — every checklist item
below is ticked except the two that only M2 and the calendar can close. `core-cst`
(green/red tree, ADR-001), `core-formats` with YAML and JSON parsers, every M1
oracle green, and the conformance rates now published rather than printed.

| Proof | State |
|---|---|
| K1 golden | 37 YAML + 18 JSON, all passing |
| K1 corpus (**P1** half 1) | **1,000/1,000 files**, 14.9 MB, zero violations — MET |
| K1 fuzzing (**P1** half 2) | **0.00 of 72 hours** on the current parser — NOT MET |
| Conformance JSON | accept 100%, reject 100% — published |
| Conformance YAML | accept 100%, reject 18.1% — published, 77 failures listed in full |
| K3 determinism | double-build + double-run, 3 platforms |
| Performance | allocation counts, exact equality (ADR-006 amendment 2) |

Published rates and the complete failure list live in
[`conformance/REPORT.md`](conformance/REPORT.md), regenerated and byte-compared
by `gate/conformance` (ADR-009). **P4 is still not complete**: §4.1 names
toml-test as its third suite, and `toml` arrives with strukt in Phase 2.

**Next: M2's remaining diff work.** D3 was decided on 2026-08-09 — konflux is the
flagship (ADR-010). The diff golden suite is **10/10 green**: the oracle
(ADR-011), the format-agnostic algorithm, and `semantic_view` for both JSON and
YAML (ADR-012, ADR-013) have all landed. `UNIMPLEMENTED_CASES` reached zero and
the ratchet came out, exactly as ADR-011 said it would.

**Coverage is the number that matters, not the golden count.** `cargo xtask
semantic-coverage` measures the corpus: **YAML 541/750 (72.1%)**, JSON 250/250,
**total 79.1%**. What is left is a long tail of ordinary YAML gaps, no single
one of them large:

| Files | Not modelled yet |
|---:|---|
| 68 | a value of several tokens (anchor, tag, or a template beside literal text) |
| 47 | mapping entries and sequence items at the same level |
| 32 | a sequence item mixing a mapping entry with non-entry lines |
| 26 | a template line owning indented lines |
| 18 | a flow collection that does not start the value |

**Correction to the previous handoff.** It said `semantic_view` would close the
yaml reject-rate gate. **It does not.** That gate is about `parse` refusing
invalid documents, and this work sits entirely above `parse`. The block/flow
knowledge now exists and could be wired back into validation, but that is
separate work and nobody has done it.

**Still blocked on Ayush, but nothing downstream waits on these:**

1. **D1** — the umbrella brand and the `<b>` binary name. Blocks the multicall
   binary and the crates.io/org/domain reservation. Needs an actual name; §16
   makes it Ayush's alone and there is nothing to proceed on without one.
2. **D4** — the LICENSE files. `MIT OR Apache-2.0` is declared in `Cargo.toml`
   and the choice is settled; only the files are missing. **Not written, on
   purpose:** the Apache-2.0 text must come from the canonical source, and the
   two places it exists locally are a Helm chart carrying Broadcom's copyright
   header and my memory. Neither is an acceptable provenance for a legal
   document. One fetch settles it.
3. **Branch protection on `main`**, so `CODEOWNERS` has teeth. It would have
   caught two real slips this session: a three-deep PR stack that merged into
   base branches instead of `main`, and a local `main` left without an upstream
   after the history rewrite.

**ADR-009 through ADR-013 accepted 2026-08-09.** ADR-010 was accepted with its
reasoning as drafted — assembled from the plan's own arguments rather than from
a separately stated rationale — and that provenance is recorded in the ADR.

**Not blocked, but only time will fix it.** P1's fuzzing half accrues ~2 hours
per night and resets whenever `core-cst`, `core-formats` or the fuzz targets
change. Roughly five weeks of a stationary parser.

**Standing gate before M3.** The YAML reject-rate is 18.1%. The remaining 77
must-reject cases need block/flow context tracking, which *is* `semantic_view`
at M2 — and the published failure list now shows that directly: read the case
titles in `REPORT.md` and they are almost all indentation, flow-collection and
plain-multiline cases. Merging a document we failed to recognise as invalid is
the "silently wrong" failure §0 ranks first, so this must rise before M3 ships a
merge.

**Accepted decisions**, so a new session does not re-litigate them: ADR-001
through ADR-013 all accepted; D3 is decided
and konflux is the flagship; the ASan/LSan substitution
for §8's "ASan/UBSan" accepted (Rust has no UBSan; miri covers it); the corpus
expansion to 1,250 files accepted.

---

Live checklist, one section per phase (MASTER_PLAN §9.1). One item per session
(§9.2). Every item names the proof obligation it discharges and the §8 gate that
enforces it.

**Legend:** `[ ]` not started · `[~]` in progress · `[x]` done and proven ·
`[!]` blocked on Ayush.

> **Nothing is `[x]` without the passing command output in its PR's Proof
> Delta.** "It works" is not a status.

---

## Phase 0 — Validate + scaffold  (weeks 0–1)

**Exit gate (§11):** signal read on the validation posts — ≥50 upvotes or ≥20
"I hit this weekly" replies → konflux confirmed. Weak signal → pivot flagship to
bigsheet, konflux slides to Phase 4 (the kernel still gets built, via strukt).

### 0.1 Scaffold

- [x] **Workspace per §2** — 5 kernel crates, 9 tool stubs, `shell/` README-only,
      `corpora/`, `benches/`, `fuzz/`, `xtask/`, `adr/`, `.github/workflows/`.
      *Proof:* `cargo build --workspace` green.
- [x] **Per-crate `DESIGN.md`** (§9.1) — scope, invariants, current milestone,
      proof obligations, for all 14 crates.
- [x] **`ENGINEERING.md`** — distillation of §0, §8, §9, Appendix C.
- [x] **No brand-named artifact** — virtual manifest, no multicall binary, no
      placeholder name anywhere (ADR-007, D1 unblocked).
- [x] **Determinism hygiene enforced mechanically** — `clippy.toml` bans
      `HashMap`/`HashSet`, `SystemTime::now`, `Instant::now`, `sort_unstable*`,
      `to_lowercase`/`to_uppercase` (§9.5).

### 0.2 CI — every §8 gate present

- [x] `gate/format` — `cargo fmt --check`
- [x] `gate/lints` — clippy pedantic + restriction lints, config in-repo so a
      local run reproduces CI
- [x] `gate/tests` — `cargo test --workspace`
- [x] `gate/golden` — `core-verify` runner; **invariant V4: an empty suite is an
      error, not a pass**
- [x] `gate/golden-guard` — evidence changes require `[NEEDS-AYUSH-APPROVAL]` in
      the PR body (§8 anti-reward-hacking law, mechanised)
- [x] `gate/determinism` — double-build + double-run output-hash compare, on
      Linux **and** Windows **and** macOS (ADR-005)
- [x] `gate/msrv` — 1.90 build + test (ADR-002)
- [x] `gate/docs` — `cargo doc -D warnings`
- [x] `gate/platform` — Windows + macOS test matrix (§2)
- [x] `gate/shellcheck` — `corpora/*.sh`
- [x] `fuzz-smoke` — 300s/target per PR, corpus-seeded (K1)
- [x] `fuzz-nightly` — 3600s/target scheduled; report → new goldens
- [x] `gate/miri` — kernel crates, `-Zmiri-strict-provenance`
- [x] `gate/sanitizers` — ASan + UBSan on kernel crates
- [x] `gate/supply-chain` — `cargo-deny` advisories / bans / licenses / sources
- [x] `gate/benchmark-baseline` — criterion vs saved baseline (ADR-006)
- [x] `corpus/fetch` — scheduled; asserts exactly 1,000 files and that nothing
      fetched is committable
- [~] **Verify all gates green on GitHub Actions.** First run 2026-08-07 was
      **red in five places**, all real, all now fixed and reproduced locally
      first. Recorded because a CI skeleton that was never run is not a gate:

      | Gate | Failure | Fix |
      |---|---|---|
      | `gate/determinism`, `gate/platform` ×2 | **false positive** — CI wrote `run-a.json` into the repo root, so run two observed a tree containing run one's output | outputs go to `$RUNNER_TEMP`; the diagnostic now names self-observation as a cause |
      | `gate/supply-chain (bans)` | a bare `path` dependency reads as a wildcard requirement, and `wildcards = "deny"` correctly rejected it | explicit `version` on path deps — **not** a relaxed ban |
      | `gate/miri` | `opendir` unsupported under miri's isolation; the golden runner reads directories | `-Zmiri-disable-isolation`, so those tests run under miri rather than being excluded from it |
      | `gate/sanitizers (undefined)` | Rust's `-Zsanitizer` has **no `undefined` value** — UBSan does not exist for Rust | matrix is now `address` + `leak`; UB is covered by miri. **[NEEDS-AYUSH-APPROVAL]**, see below |
      | `fuzz-smoke` | cargo-fuzz defaulted to its musl host triple; a sanitizer cannot link against static libc | explicit `--target x86_64-unknown-linux-gnu` |

      The determinism false positive is the one that mattered most: a gate that
      cries wolf trains everyone to ignore red, which is worse than no gate.
- [x] **Accepted 2026-08-08.** §8 says "ASan/UBSan jobs". There is no UBSan for
      Rust — `-Zsanitizer` accepts address, cfi, dataflow, hwaddress, kcfi,
      kernel-address, kernel-hwaddress, leak, memory, memtag, safestack,
      shadow-call-stack, thread, realtime, and nothing named `undefined`. I have
      substituted ASan + LSan, with undefined behaviour covered by the `miri`
      job, which is strictly better at it for Rust. Nothing is skipped and no
      threshold is loosened, but §8 is law and the substitution is yours to
      accept or replace.
- [x] **No workflow job had `timeout-minutes`.** A single slow test could burn
      free-tier minutes until GitHub's 6-hour default, against §10's budget
      constraint. Found when `gate/miri` ran 19 minutes on a 100,000-node drop
      test and had to be cancelled. Every job is now capped, and miri is
      installed on the dev machine — it had caught two problems in two sessions,
      both discovered only in CI, which is one more than a local toolchain costs.
- [x] **`actions/checkout` bumped v4 → v7**, 20 call sites across 7 workflows.
      The reason to leave it alone had inverted: the warning now reads *"target
      Node.js 20 but are being **forced to run on Node.js 24**"*, so v4 was
      already executing on a runtime it does not target. Staying put stopped
      being the conservative option.
      - **v7, not the v5 this line named.** v5 was current when the line was
        written and is now two majors behind, so bumping to it would buy one
        more bump. Measured before choosing: the only breaking changes across
        v5→v7 are `allow-unsafe-pr-checkout` and blocking fork checkout for
        `pull_request_target` / `workflow_run`. **No workflow here uses either
        trigger** — all seven are `push` / `pull_request` / `schedule` /
        `workflow_dispatch` — so none of it reaches us.
      - **Only `checkout` was affected**, and that was measured rather than
        assumed: the warning on a job using `rust-toolchain` + `rust-cache`, and
        on one additionally using `install-action` + `upload-artifact`, names
        `actions/checkout@v4` and nothing else. The other five actions are
        already on Node 24, so this is the whole fix, not the first of five.
      - `fetch-depth: 0` is unchanged across these majors, which matters twice:
        `golden-guard` needs it for the base/head diff and the fuzz-hours ledger
        needs it for `git merge-base --is-ancestor`. Both verified intact.
- [ ] **Actions are pinned to moving major tags, and our own fetch scripts
      refuse to be.** `corpora/fetch.sh` and `conformance/fetch.sh` both reject
      anything that is not a 40-char SHA — *"Never a branch or a tag: those
      move"* — while the CI that enforces that discipline pins its seven actions
      to `@v7`, `@v2`, `@stable`, which move under us. Pinning actions by SHA
      would close it. Not done here: it is ~58 call sites and a supply-chain
      policy change touching §8/§10, so it is ADR-shaped and Ayush's call.
      **Say the word and I will write it.**

#### Gate arming schedule (ADR-003)

Four gates cannot be meaningful before a parser exists. They are wired and
blocking today against the weakest *true* statement available, and arm here:

| Gate | Today | Arms at |
|---|---|---|
| `gate/golden` | K1 on the empty grammar, 6 hostile-byte cases | **M1** — same cases against the real YAML/JSON parse/serialize |
| `fuzz-smoke` | `roundtrip_identity` target | **M1** — `yaml_roundtrip`, `json_roundtrip` |
| `gate/conformance` | harness builds and runs; no conformance claim made | **M1** — yaml-test-suite + JSONTestSuite at pinned revs, pass rates published |
| `gate/differential` | `git merge-file` oracle present and correct on a fixture | **M2** — konflux diff vs diff3/Mergiraf |
| `gate/benchmark-baseline` | names tracked, values `uncalibrated` | **M1** — numbers recorded from a run on `main` |

### 0.3 Corpora

- [x] **Fetch scripts, not vendored copies** (§2, §10) — pinned 40-char SHAs,
      SPDX allow-list, upstream licence recorded with its hash, sorted
      `MANIFEST.tsv` receipt per source.
- [x] **Exactly 1,000 files** — helm-charts 400, terraform 250, k8s-manifests
      200, kustomize 150. Measured, not targeted: a short yield is a hard error.
- [x] **Offline manifest verification blocking in CI** — `cargo xtask corpus-verify`.
- [ ] `corpora/lockfiles/` — Phase 4, lockproof P1
- [ ] `corpora/csv/` — Phase 3, bigsheet P1/P2
- [ ] `corpora/pdfs/` — Phase 4, pdfsurgeon P1/P5 (hostile-PDF corpus)

### 0.4 ADRs

- [x] ADR-002 toolchain + MSRV · ADR-003 gate arming · ADR-004 corpus ·
      ADR-005 determinism · ADR-006 baselines · ADR-007 no brand-named artifacts.
      **Accepted** with Phase 0 on 2026-08-07.
- [x] **ADR-001** — `core-cst` representation. Spike run, ADR written,
      **accepted 2026-08-08**.
- [~] **ADR-009** — publishing conformance rates. `proposed`, awaiting sign-off
      (§9.3).

### 0.5 Validation + decisions — [AYUSH]

- [x] Draft r/devops post — `docs/validation/`
- [x] Draft Ask HN post — `docs/validation/`
- [~] **Post them.** Ayush handles all public communication, always. **No longer
      D3's input** — D3 was decided without them (ADR-010), so §11's exit gate
      closed by decision rather than by measurement. The drafts stay: asked
      before there is anything to show, the same question still tells us which
      incumbent is the real bar (§14). Now pre-launch work, not a blocker.
- [!] **D1** — umbrella brand + `<b>` binary name. Blocks the multicall binary
      and the crates.io/org/domain reservation.
- [!] **D2** — Tauri confirmation (needed before Phase 3, bigsheet's grid).
- [x] **D3 — DECIDED 2026-08-09: konflux is the flagship.** M2 onward is
      unblocked. → **ADR-010**.
      - Decided **without the validation signal**: §11 made D3 a read of the
        posts, and the posts are unsent. That is recorded rather than smoothed
        over, because a later reader finding "konflux confirmed" is entitled to
        know it was confirmed by judgement, not by evidence.
      - The unvalidated bet is **deferred, not removed**. It comes due at §14's
        kill rule — 8 weeks post-launch, <100 meaningful engagements and zero
        organic issues, and konflux freezes.
      - Consequence for D2: Tauri is off the critical path until Phase 3.
- [!] **D4** — final licence (`MIT OR Apache-2.0` is declared per §2/§10; the
      LICENSE files are not written pending D4).
- [!] **Enable branch protection on `main`** so `CODEOWNERS` has teeth.
- [ ] r/kubernetes post — §11 names three venues; only two were requested.
      Say the word and I will draft it.

---

## Phase 1 — konflux  (weeks 1–10)

**Launch gate (§4.1):** P1–P4 green in public CI, benchmark table *including
where Mergiraf and diff3 win*, 60-second screencast, README per §12.

**Phase 0 accepted 2026-08-07. D3 decided 2026-08-09: konflux is the flagship**
(ADR-010), so M2 onward is unblocked. M1 is finished.

### M1 — CST + K1 for YAML and JSON

*Proof obligation: **P1** (round-trip). Gates: `gate/golden`, `fuzz-smoke`,
`gate/conformance`, `gate/determinism`, `gate/miri`.*

- [x] **Spike:** green/red vs owned token tree — plus a flat arena of spans,
      added because excluding the obvious third option would have made the
      comparison a formality. Measured on 750 real YAML files / 10,043,614 bytes:
      memory by allocator counting, persistence cost, `locate` cost, K1, K2.
      → **ADR-001: green/red tree** (`proposed`). Evidence:
      `spikes/adr-001-cst-representation/`, reproducible in one command.
      *Finding:* the owned token tree is **dominated** — §3.1's stated pair
      contained a clear loser and the strongest challenger was not in it.
- [x] Oracle first: golden suites for YAML and JSON round-trip. **Confirmed red
      2026-08-08: 47 of 47 cases fail, all with one cause (no parser), zero
      byte-mismatch failures.**
      - Contract landed: `core-cst` green/red types per ADR-001 + `Cst::serialize`;
        `core-formats::Format` with `parse`/`serialize`; `core-verify::roundtrip`.
      - "Seeded from the corpus" read as *the corpus decides what the cases must
        cover*, not *copy corpus files in* — copying would vendor third-party
        bytes ADR-004 exists to avoid. `cargo xtask corpus-survey` measures the
        constructs; 17 YAML cases are corpus-derived with their shares recorded,
        12 more cover spec constructs the corpus happens not to contain.
      - A round-trip case has **no `expected` file** — the input is the
        expectation, so a K1 case cannot be doctored, only deleted.
      - *Finding:* **41.2% of corpus YAML contains Helm's `{{ }}`**, which is not
        YAML at all. §3.1 frames the verbatim escape hatch as a fallback for
        exotic tags; the corpus says it is the main path for two files in five.
      - *Defect found and fixed:* the deep-nesting test caught the **destructor**,
        not the serializer — dropping a refcounted tree recursed and aborted the
        process with `SIGABRT`. `core-cst` now has an iterative `Drop`. Recorded
        as a consequence in ADR-001 that the original decision missed.
- [x] **JSON has no corpus** — **resolved.** Approved 2026-08-08; 250 SchemaStore
      files were added and K1 holds on 1,000/1,000. See "P1 corpus half: MET"
      below. Kept because the finding is why the corpus source exists.
- [x] Oracle first: `yaml_roundtrip` and `json_roundtrip` fuzz targets, corpus-
      seeded from the golden cases (§3.3), both wired into `fuzz-smoke` and
      `fuzz-nightly`. Each asserts **F1** (never panics, on any bytes) and **K1**
      (round-trip when parse succeeds).
      - **F1 measured:** 43M executions across the three targets, zero crashes.
        Trivially true today — there is no parser to panic.
      - *The finding:* a K1 fuzz target **cannot be red** while parse always
        fails. It never reaches its assertion, so it reports success having
        verified nothing — and unlike a golden suite there is no case count to
        notice shrinking. Demonstrated: 200,000 inputs, exit 0, zero K1
        assertions evaluated.
      - The red therefore lives in a **non-vacuity guard**
        (`crates/core-formats/tests/fuzz_seeds.rs`): every seed in a target's
        corpus must parse, and the seed count must not fall behind the golden
        suite. **Confirmed red 2026-08-08: 0 of 47 seeds parse.** This is what
        makes the eventual green mean something.
- [x] Oracle first: yaml-test-suite + JSONTestSuite adapters at pinned revs.
      **Confirmed red 2026-08-08.** 720 cases fetched with exact counts asserted:
      JSONTestSuite 318 (95 accept / 188 reject / 35 implementation-defined),
      yaml-test-suite 402 (308 / 94). Suites fetched, never vendored (ADR-004
      discipline); yaml's licence comes from a separately pinned `main` commit
      because its generated `data` branch carries none.
      - *The finding:* a single "pass rate" is a badge that lies. With today's
        stubbed parser — a function whose body is `return Err` — the blended
        figure would publish **66.4% JSON conformance** and 23.4% YAML, because
        rejecting everything is perfect on the must-reject class. Accept-rate and
        reject-rate are therefore reported separately, ratcheted independently,
        and there is no method that combines them. → **ADR-008**.
      - `unrecorded` is an error, not a free pass: a conformance rate is
        deterministic, unlike a benchmark baseline (ADR-006). All four thresholds
        are `unrecorded`, which is what makes this red.
      - *A claim I checked instead of publishing:* I expected the conformance and
        K1 oracles to be mutually unsatisfiable, since K1 asserts round-trip on
        three spec-invalid YAML files. Measured: **0 of yaml-test-suite's 94
        must-reject cases involve invalid UTF-8 or control bytes** — all 94 are
        structural, and our three awkward cases are encoding-level. The oracles
        are compatible as written, so ADR-008 draws the accept/reject line at
        structure and §3.2's `parse` signature stands unchanged.
- [x] `core-cst` representation implemented per ADR-001 — green/red tree,
      `crates/core-cst/src/lib.rs`, with the iterative `Drop` the spike missed.
      *(Was left unticked while the parsers that use it were ticked; corrected
      here, not newly done.)*
- [x] **JSON parse/serialize.** Lossless, RFC 8259, three separate passes: a
      total lexer covering every byte, an iterative validator, an order-preserving
      builder. `Json::parse` is live; the three JSON oracles are green.
      - K1 golden 18/18 · fuzz vacuity guard 18/18 seeds parse ·
        **JSONTestSuite accept 95/95 (100%), reject 188/188 (100%)**, 22/35
        implementation-defined accepted. Thresholds recorded at 1.0/1.0, so the
        ratchet is at its ceiling and any regression fails.
      - **K1 verified on 117 accepted JSONTestSuite documents** — third-party
        input we did not write, a broader corpus than our 18 golden cases.
        Anything accepted must round-trip: a lossless kernel that accepts a
        document it cannot reproduce has broken its central promise.
      - F1 under the fuzzer: 1,690,296 executions, 509 coverage points, no crash
        and no K1 violation. Iterative throughout — JSONTestSuite ships 100,000
        opening brackets, and a recursive parser aborts the process there.
      - *Scope note:* MILESTONES said "K1 on every corpus JSON file" and the
        corpus contains **zero** JSON files, so that claim is unmakeable as
        written. The JSONTestSuite documents are the strongest substitute
        available. Closing the gap properly needs a JSON corpus source, which
        remains **[NEEDS-AYUSH-APPROVAL]** in PR #1.
      - *Defect found:* libFuzzer **writes** coverage-increasing inputs into its
        first corpus directory, so pointing the vacuity guard at the curated
        seeds made it fail the moment the fuzzer did its job — one 1-byte input
        (`}`) landed there and had already been committed. Curated seeds now
        live in `fuzz/seeds/` (evidence, golden-guarded); `fuzz/corpus/` is
        gitignored scratch.
- [x] **YAML parse/serialize.** Lossless and structural, not semantic: total
      lexer, line nesting by indentation, verbatim escape hatch. Comments,
      anchors/aliases, merge keys, quoting style, line endings, multi-document
      streams, block scalars and directives all round-trip.
      - K1 golden **31/31** · fuzz vacuity 31/31 seeds ·
        **yaml-test-suite accept 308/308 (100%), reject 1/94 (1.1%)**.
      - F1 under the fuzzer: 1,619,579 executions, 708 coverage points, no crash
        and no K1 violation.
      - *A rule tried and removed:* a tab-in-indentation check rejected **12
        documents yaml-test-suite calls valid** — tabs are legal in blank lines,
        as separation, and before flow indicators. Rejecting valid input is the
        worse error: a refused file is one konflux cannot help with at all.
      - *K1 violation found on the real corpus, fixed by the §3.3 loop.* A `"`
        inside a Go template — `service="{{ template "x" . }}"` — closed the YAML
        quoted scalar early, and because YAML permits quoted scalars to span
        lines the mis-parse ran 230 bytes into the next block scalar and ate its
        header. Minimised to golden cases 045/046, confirmed red, then fixed:
        a quote now only opens a scalar at a value position, never mid-token.
- [x] Verbatim-node escape hatch — carrying Helm's `{{ }}`, which the survey
      found in **41.2%** of corpus files. §3.1 frames it as a fallback for exotic
      tags; on real config it is the main road for two files in five.
- [x] **P1 (corpus half): K1 green on 750/750 corpus YAML files**, 10,043,614
      bytes, zero violations and zero rejections. `cargo xtask corpus-k1`, wired
      into the `corpus` CI job.
- [x] **P1 corpus half: MET.** Approved 2026-08-08, so 250 real-world JSON files
      were added (SchemaStore schemas + test instances, Apache-2.0, pinned).
      **K1 holds on 1,000/1,000 files** in a format konflux parses — 750 YAML
      (10,043,614 bytes) and 250 JSON (4,883,405 bytes) — zero violations, zero
      rejections. The 250 HCL files stay in the corpus, unparsed, until Phase 2.
      - The 250 JSON files round-tripped **on first contact**, with no parser
        change. Real third-party input the parser had never seen.
      - `corpus-k1` now reports MET or NOT MET against the ≥1,000 threshold
        instead of always warning. Under-claiming a proof is as misleading as
        over-claiming one.
- [~] **P1 fuzzing half:** ≥72 cumulative hours with zero violations.
      `cargo xtask fuzz-hours` now measures it, and reports **0.00 of 72 hours**.
      - The number comes from GitHub's workflow-run history, not a committed
        ledger: a file we maintain about ourselves would be editable in the same
        commit that needed it to be larger.
      - **Only runs descending from the last change to the fuzzed code count.**
        Fuzzing proves the code that was fuzzed, so a parser change resets this.
        There are 2.00 cumulative hours on record and 0.00 that describe today's
        parser — reporting the former as P1 would be true arithmetic making a
        false claim.
      - Reported, not gated: §8 lists the nightly fuzz row as
        "Report → new goldens", and failing every night for the months this takes
        would teach everyone to ignore the workflow.
- [~] **GATE before M3:** yaml-test-suite reject-rate raised **1.1% → 18.1%**
      on 2026-08-09, with accept held at 100% and corpus K1 at 1,000/1,000.
      Four rule families, each unambiguous from the token stream: block scalar
      indicators, comment separation, anchor placement, directive and
      document-marker structure.
      - The two ratchets are what made this tractable. `accept` pinned at 1.0
        rejected three drafts that would have refused valid documents; corpus K1
        rejected a fourth that refused two real Helm charts.
      - *Root cause worth keeping:* a `%` at column zero is a directive only in
        the directive section. Inside a document it is content — `%!PS-Adobe-2.0`
        in a block scalar, `% : 20` in a flow mapping. Four valid documents were
        rejected for want of that distinction, and the fix was in the lexer.
      - The remaining 77 must-reject cases need block/flow context tracking,
        which arrives with `semantic_view` at M2. This gate stays open until
        the rate is high enough that M3 can merge safely.
- [x] Verbatim-node escape hatch for anything the grammar cannot represent
      (§3.1: preserving beats understanding). *Duplicate of the ticked item
      above; both refer to the same `SyntaxKind::VERBATIM` work.*
- [x] **Deleted `core_cst::roundtrip_identity`** and retired the
      `roundtrip_identity` fuzz target, exactly as ADR-003 said would happen —
      its doc comment said leaving it would be "a bug in the milestone, not a
      feature".
      - Its six golden cases were **preserved, not deleted**: they are now
        `300`–`305` in the YAML round-trip suite, checked against the real
        `parse`/`serialize` pair. All six still round-trip, which is what ADR-003
        predicted when it said an input that round-trips under the empty grammar
        must still round-trip under a real one.
      - `crates/core-verify/tests/golden/roundtrip-identity/` remains as the
        golden **runner's** own fixture, which is all it ever tested.
      - Determinism re-pointed: `xtask parse-digest` emits the accept/reject
        verdict and the SHA-256 of the serialised tree for all 55 golden cases,
        double-built and double-run on Linux, Windows and macOS. K1 asks whether
        the bytes came back; **K3 asks whether they come back the same way every
        time**. The diagnostic text is hashed too — a parser whose *message*
        varies between runs is as nondeterministic as one whose bytes do, and
        those messages reach `--json`.
      - Benchmarks re-pointed at the real parsers, names changed accordingly —
        which `bench-compare`'s name-parity check makes a reviewed change rather
        than a silent one.
- [x] **Armed `gate/conformance` — pass rates published with the honest failure
      list** (P4). `conformance/REPORT.md`, generated by
      `cargo xtask conformance-report --write`, regenerated and byte-compared by
      the gate. → **ADR-009**.
      - The rates were already measured and ratcheted; they were **published
        nowhere**. The only rendering was an `--nocapture` line in a CI log that
        expires, and the list truncated at *"first 15 of 77"* — so 62 of
        konflux's YAML conformance failures had never been written down at all.
      - *Why a committed file is safe here and a committed fuzz-hours ledger was
        not:* fuzz hours are a fact about runs that happened and cannot be
        recomputed, so a file we maintain about ourselves is worthless as
        evidence. A conformance rate is a pure function of two pinned commits
        and our own source, so CI recomputes it every run. Committing a derived
        value is safe exactly when the derivation is reproducible.
      - **Proven to have teeth, both directions.** Editing the published YAML
        reject rate 18.1% → 94.7% fails with the differing line named; deleting
        one case from the 77-line failure list fails the same way. Neither can
        be fixed by editing the file.
      - The list carries each case's own title, so it reads as *"Wrong
        indentation in Sequence"* rather than as 77 four-character IDs. That
        also corroborates the M3 gate below: nearly every remaining failure is a
        block/flow context case, which is what `semantic_view` brings at M2.
      - *Scope note:* **P4 is not complete and this does not complete it.** §4.1
        names three suites; toml-test is the third and `toml` does not exist
        until Phase 2, so P4 finishes with strukt.
      - Badge *values* are generated; badge *images* are not — colour and form
        are visual design and Ayush's alone (§16), landing with the launch
        README (§12).
- [x] **Calibrated benchmark baselines**, recorded from the bench job on `main`
      at `9ef31af` (ADR-006). The regression gate is now numeric, not just
      structural.
      - Tolerances are **per-benchmark**, because noise is. Measured over three
        CI runs: spread ranged from 1.3% (`json_parse_real`) to 7.6%
        (`yaml_parse_small`). A single global figure would flake on the small
        benchmarks or go blind on the large ones. Each is ~3x its observed
        spread, floored at 10%; `n=3` is thin, so they start loose and tighten
        as samples accumulate.
      - Confirms ADR-006's refusal to calibrate locally: the same benchmarks run
        **30–38% faster on the dev machine** than on the shared runner. Laptop
        numbers would have made every CI run a regression.
- [x] **P1 partial:** K1 green on all 1,000 corpus files. *Duplicate of "P1
      corpus half: MET" above, which carries the output.*

### M2 — Structural diff

*Proof obligation: **P4**; `gate/differential` arms. Gates: `gate/golden`,
`gate/differential`.*

- [x] **Oracle first: diff golden suite** — 10 hand-built cases where line-based
      diff is wrong and structural diff is right. **Confirmed red 2026-08-09: 9
      of 10 fail, all with one cause — no diff implementation.** → **ADR-011**.
      - Contract landed: `konflux::diff` with `Change`/`ChangeKind`/
        `Significance`/`DiffReport`, and `core-verify::golden::run_pairs_dir`,
        a two-input runner (a diff takes two documents; `run_dir` takes one).
      - **The golden is the `--json` output**, not the rendered view, so
        `core-cli` C1's stable machine contract is under test before it has an
        implementation rather than retrofitted after it leaks into a script.
      - **Paths are RFC 6901 pointers.** A path is an identity and may not be
        ambiguous: real Helm charts contain `kubernetes.io/os`, which a dotted
        path cannot round-trip. Case `120` holds that decision in place.
      - **`kind` and `significance` are separate fields**, and cases `010` and
        `130` are why: a reordered *mapping* is `moved`+formatting, a reordered
        *sequence* is `moved`+semantic. Line diff renders them identically, and
        confusing them either invents conflicts or loses changes.
      - **It lands red-but-recorded, not red.** M1's oracle PR could merge green
        because the parser landed with it; a diff implementation cannot (§15
        gives it the majority of the time budget). So `UNIMPLEMENTED_CASES = 9`
        is checked exactly, per **ADR-003's idiom** — blocking today against the
        weakest true statement, arming on a published schedule. Proven red in
        both directions: recording 8 gives the goldens-are-evidence failure,
        recording 10 gives *"good news, and a constant to lower"*.
      - *Non-vacuity guard, the lesson M1 paid for:* a null diff must fail 9 of
        10 cases. If formatting-only changes were reported as *no* change, the
        three formatting cases would be satisfied by `[]` and a third of the
        suite would prove nothing. `900-identical` is the deliberate control —
        an oracle no output can satisfy is as broken as one everything does.
      - *Deferred, and recorded rather than guessed:* **comments** and
        **re-indentation**. Both depend on trivia attachment in the CST walk,
        and a guessed `expected` is a guess wearing evidence's clothes. konflux
        promises *"comments and key order preserved"* and the comment half is
        currently proven by nothing.
- [~] **Semantic-tree matching.** The algorithm is landed and format-agnostic;
      **JSON is green, YAML is not.** `UNIMPLEMENTED_CASES` **9 → 8**.
      → **ADR-012**.
      - *The finding that reshaped this item:* **YAML's CST has no structure to
        match.** M1 built it as `STREAM → DOCUMENT → LINE*` — a flat list of
        lines with indentation tokens, lossless and structural by design. JSON's
        is already nested. So this splits three ways, very unequally: the
        matching algorithm (done), JSON's `semantic_view` (done, small), and
        YAML's `semantic_view` (**not done, and the largest piece left in M2**).
      - Landed: `core_formats::semantic` with `SemanticNode`/`Scalar`, the
        `Format::semantic_view` trait method, and konflux's walk — LCS alignment
        so a mid-sequence insert is one `added` rather than a positional
        cascade, permutation detection so a reorder is one `moved`, RFC 6901
        paths, and output sorted by path bytes then kind (§9.5).
      - **Only strings are normalised.** `1.0` vs `1.00` reads as *semantic*,
        deliberately: calling them equal needs a numeric interpretation this
        layer refuses to make, the same line ADR-008 drew on `yes`. Over-
        reporting a change is noisy; under-reporting one loses an edit in a
        merge, and only one of those is recoverable.
      - **A format with no view is refused, never answered** (ADR-012). An empty
        diff and an agreement are the same bytes, so returning `[]` for a file
        we cannot read would give one spelling to two opposite meanings. That is
        why `900-identical` is currently red: konflux cannot say even "no
        changes" about a YAML file yet.
      - *Gap found and closed in the same session:* the two JSON goldens cover a
        mapping reorder and a nested scalar change and **no sequences at all**,
        so the LCS and permutation code would have shipped untested behind a
        green suite. Eight unit tests now cover it — JSON has arrays, so they
        did not have to wait for YAML.
      - *Lint conflict worth knowing:* clippy pedantic's `stable_sort_primitive`
        demands `sort_unstable`, which `clippy.toml` bans under §9.5. The
        project's law wins; byte-wise `sort_by` satisfies both.
- [x] **YAML `semantic_view`** — indentation becomes nesting, dashes become
      sequences, quoting becomes spelling. → **ADR-013**.
      - **Two inference bugs found by measurement, not by review**, while the
        golden suite sat at 10/10 green and the corpus at 14.5%:
        - **Comment lines attach to the preceding line, not as siblings.** The
          lexer gives an unindented line to the innermost open line, so the
          comment in `imageRegistry: ""` / `## E.g.` becomes that scalar's
          *child*, and a literal reading refuses the file. **235 of 750 files.**
        - **Zero-indented sequences.** `items:` with `- a` beneath it at the
          same indent is the dominant Kubernetes style, and indentation makes
          those dashes siblings of the key. **139 files.**
      - Coverage after both: **14.5% → 23.3% → 31.6%**, and every remaining
        bucket is a genuine unmodelled feature rather than a misreading.
      - *Why the golden suite did not catch either:* ten hand-built cases are a
        specification, not a sample. `semantic-coverage` is the sample, and it
        is the reason this item shipped correct rather than merely green.
- [x] **Flow collections** — `{a: 1}` and `[1, 2]`, nested, empty, and inside
      block structure. A small recursive parse of one line's tokens, separate
      from the block reader because the two disagree about what ends a value:
      indentation there, a bracket here.
      - Coverage **31.6% → 48.4%** for YAML, 48.7% → **61.3%** overall. The
        largest single unlock the queue had.
      - Depth-capped at 64 and refused past it. A fuzzer will send a document
        that is nothing but brackets, and `core-cst`'s destructor taught this
        exact lesson at M1 — recursion must decline rather than take the
        process with it. Tested with 200 levels.
      - Three golden cases added (`040`, `160`, `170`), **confirmed red first**.
      - *Known gap, deliberately not guessed at:* switching a mapping between
        flow and block spelling reports **no change at all**. It is not a
        semantic change, and attributing a formatting change to a container
        would need containers to carry source text — which they deliberately do
        not, because inlining a 400-line subtree into a JSON field is
        unreadable. Reporting mapping *reorder* as formatting but flow↔block as
        nothing is an inconsistency worth a decision, and it is Ayush's.
- [x] **Block scalars** — `|` and `>` with their indicators. Modelled as a
      scalar carrying its own source text. → **ADR-013 amendment 1**.
      - **Not folded, deliberately.** Implementing indentation stripping and
        chomping correctly is spec work, and a subtle error makes two different
        strings compare equal — a diff that misses an edit. The cost is
        over-reporting a restyle as semantic, which is the trade already taken
        on numbers and in the same direction.
      - Coverage **48.4% → 57.2%** for YAML, **67.9%** overall.
      - *Lexer wart found and pinned:* a block body absorbs the file's trailing
        newline **only when nothing follows the block**. Golden cases `180` and
        `181` pin both shapes rather than leaving it to surprise someone at M3.
      - *A golden I wrote and then corrected, in the same unmerged PR:* case
        `180`'s expected text was my prediction of the node's bytes and was
        wrong by that newline. Recorded because §8 is about not weakening
        evidence, and the distinction that matters is that this was an unmerged
        draft corrected toward the truth, not a merged golden bent toward the
        code. The code was right; the prediction was not.
- [x] **Multi-document streams** — `---` and `...` split the top level into
      documents. → **ADR-013 amendment 2**.
      - `SemanticNode::Stream`, deliberately not reusing `Sequence`: one
        document whose root is a two-item list and a two-document file produce
        the same paths and mean different things, and a shared variant would
        diff document 0 against list item 0.
      - One document with explicit markers is **not** a stream — otherwise
        `---\na: 1` versus `a: 1` reports a formatting difference as structural.
      - A marker indented inside a collection is refused: there is no answer to
        which side of it the surrounding keys belong to.
      - Coverage **57.2% → 64.7%** for YAML, **73.5%** overall.
- [x] **Helm templates** — option 2, chosen by Ayush. `{{- if }}` lines are
      carried beside the data as opaque, ordered text. → **ADR-014**.
      - Coverage **64.7% → 72.1%** for YAML, **79.1%** overall; the largest
        bucket in the queue disappears.
      - **Templates are an ordered list, never matched by text.** Charts hold
        two identical `{{- end }}` lines routinely, and matching by text would
        let one be deleted with nothing noticing — which changes what the chart
        renders. A test asserts exactly that.
      - Reported at the **collection's** path, not a path of their own: a
        template has no key and no index a reader could point at that would not
        collide with a real key. `--json` stays at schema version 1.
      - *Found by probing rather than assumed:* templates used as **values**
        already worked — a template is a single `VERBATIM` token, so it resolved
        like any scalar. Case `310` pins it.
      - **Honest limits, for the launch README:** konflux cannot tell you a key
        is guarded by a condition, and a template that moves reads as a change
        beside whatever it lands next to. Claiming it "understands Helm" would
        be the overclaim §12 exists to prevent.
- [ ] **Wire block/flow context back into `parse`** to raise the yaml
      reject-rate. The knowledge now exists in `semantic_view`; the validator
      does not use it. This is what the standing M3 gate actually needs.
- [ ] Side-by-side CLI output via `core-cli` — `--json` stable and schema-versioned.
- [ ] Differential runner online: ours vs `diff3`/`git diff` on the corpus;
      divergences triaged into golden cases, never ignored.
- [ ] `NO_COLOR`, non-TTY, deterministic ordering (C1–C3).

### M3 — 3-way merge core + P2/P3 harness

*Proof obligations: **P2** (merge algebra), **P3** (soundness). Gates:
`gate/tests` (proptest), `gate/golden`, `fuzz-smoke`.*

- [ ] Oracle first: the P2 laws as proptest properties — `merge(A,A,A)=A`,
      `merge(Base,X,Base)=X`, `merge(Base,Base,X)=X`, stability, conflict
      symmetry. **Confirm red.**
- [ ] Oracle first: P3 soundness suite scaffolding — ≥2,000 triples, hand-built
      edge cases plus real conflicts mined from OSS git histories, compared
      against the human's committed resolution. **Confirm red.**
- [ ] 3-way merge over trees with path-based conflict detection.
- [ ] **Format-preserving serialization of the merged result** — the genuinely
      hard part, and the reason the gap exists. Majority of the time budget by
      design (§15).
- [ ] Conflict presentation as structured, span-anchored blocks.
- [ ] **Conflict-on-uncertainty**, enforced by test: anything unproven surfaces
      as a conflict.
- [ ] **P3: zero incorrect merges.** *Output required.* This one does not get
      rounded.
- [ ] Publish auto-resolution rate vs `git merge-file`/diff3 and vs Mergiraf —
      including where they beat us.

### M4 — git merge-driver + mergetool + install one-liner

*Proof obligation: **P3** on replayed OSS merges. Gates: `gate/golden`,
`gate/platform`.*

- [ ] Oracle first: merge-driver integration golden suite (driver protocol, exit
      codes, `%O %A %B %L %P` handling). **Confirm red.**
- [ ] `git merge-driver` implementation + `.gitattributes` guidance.
- [ ] `git mergetool` integration.
- [ ] `--check` CI mode: "will these branches conflict structurally?" (C2 exit
      codes).
- [ ] Install one-liner; static binaries for Linux, Windows, macOS.
- [ ] Report emitter: human / JSON / markdown badge (`core-verify` §3.3).

### M5 — Kubernetes semantic layer

*Proof obligation: **P3** extended. Gates: `gate/golden`, `gate/differential`.*

- [ ] Oracle first: list-by-key golden cases — containers by `name`, env vars by
      key, ports by `containerPort`, volumeMounts by `mountPath` — **never by
      list position**. **Confirm red.**
- [ ] `merge_hints` for K8s in `core-formats`.
- [ ] Helm and kustomize awareness.
- [ ] Re-run P3 with the semantic layer: still zero incorrect merges.

### M6 — TUI conflict view

*Gates: `gate/tests`, `gate/golden` (golden-image), `gate/platform`.*

- [ ] Oracle first: golden-image tests rendered with colour disabled (T2).
      **Confirm red.**
- [ ] `core-tui` theme from design tokens — **[AYUSH owns all visual design]**.
- [ ] Conflict resolver view; T3 (rendering is a pure function of state);
      T4 (every destructive action maps to a CLI invocation).

### Launch 1

- [ ] P1–P4 green in **public** CI, with badges.
- [ ] Benchmark methodology page: hardware, versions, commands, raw data
      in-repo, reproduction one-liner (§12).
- [ ] Benchmark table **including the cells where incumbents win**.
- [ ] 60-second screencast; README as landing page (§12).
- [ ] Deep-dive post on format-preserving serialization (§13).
- [ ] `DEFENSE.md` — the 20 questions a Google interviewer asks (§13, private).
- [ ] Show HN + r/devops + r/kubernetes + LinkedIn, same day — **Ayush posts.**

---

## Later phases

Tracked at one line each until their phase opens. Full breakdowns land in the
session that starts the phase.

- **Phase 2 — strukt** (10–14): kernel query/edit surface, `toml` + `hcl`.
  P1 K2 edit-locality · P2 differential vs jq on 10k queries · P3 idempotence ·
  P4 cross-platform determinism.
- **Phase 3 — bigsheet** (14–26): grid + DuckDB + CSV/Parquet/XLSX/JSONL;
  `core-verify` published as a crate. P1 exactness · P2 fidelity · P3
  no-truncation · P4 performance **budgets**.
- **Phase 4 — pdfsurgeon *or* veritas (D5), + lockproof** (26–38).
- **Phase 5 — coverify + Wing 2 gate** (38–50). Wing 2 opens only on ≥2 Wing-1
  launches with ≥1 showing real adoption.
- **Phase 6 — Shell** (50+). Only if ≥3 tools have standalone adoption. Until
  then `shell/` is a directory with a README and zero code. **No exceptions.**

**Slippage rule (§11):** if a phase runs >50% over, cut **scope** — formats,
features. Never proofs. Never soundness.
