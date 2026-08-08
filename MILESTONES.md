# MILESTONES

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
- [ ] `actions/checkout@v4` emits a Node 20 deprecation warning on every job.
      Bumping to v5 is a one-line change; left alone because a green run today
      beats an untested action version.

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
- [x] **ADR-001** — `core-cst` representation. Spike run, ADR written, `proposed`
      and awaiting sign-off (§9.3).

### 0.5 Validation + decisions — [AYUSH]

- [x] Draft r/devops post — `docs/validation/`
- [x] Draft Ask HN post — `docs/validation/`
- [!] **Post them.** Ayush handles all public communication, always. **This is
      Phase 0's actual exit gate (§11) and it is still open.**
- [!] **D1** — umbrella brand + `<b>` binary name. Blocks the multicall binary
      and the crates.io/org/domain reservation.
- [!] **D2** — Tauri confirmation (needed before Phase 3, bigsheet's grid).
- [!] **D3** — flagship confirmation after the signal read. Blocks konflux M2
      onward. M1 proceeds without it: §11 says a weak signal still builds the
      kernel via strukt, so CST + K1 for YAML/JSON is needed either way.
- [!] **D4** — final licence (`MIT OR Apache-2.0` is declared per §2/§10; the
      LICENSE files are not written pending D4).
- [!] **Enable branch protection on `main`** so `CODEOWNERS` has teeth.
- [ ] r/kubernetes post — §11 names three venues; only two were requested.
      Say the word and I will draft it.

---

## Phase 1 — konflux  (weeks 1–10)

**Launch gate (§4.1):** P1–P4 green in public CI, benchmark table *including
where Mergiraf and diff3 win*, 60-second screencast, README per §12.

**Phase 0 accepted 2026-08-07.** M2 onward is blocked on **D3**; M1 is not
(see §0.5).

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
- [!] **JSON has no corpus.** The fetched corpus is 750 `.yaml` + 250 `.tf` and
      contains no JSON, so konflux **P1** (K1 on ≥1,000 real files) is currently
      unmet for half of M1's promise. The 18 JSON cases are grammar-derived.
      Adding a JSON corpus source changes what the proofs run against and is a
      reviewed change to evidence (§9.3, ADR-004) — yours to approve.
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
- [ ] `core-cst` representation implemented per ADR-001.
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
- [ ] **P1 fuzzing half:** ≥72 cumulative hours with zero violations. The nightly
      job accumulates it; there is no counter yet.
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
- [ ] Verbatim-node escape hatch for anything the grammar cannot represent
      (§3.1: preserving beats understanding).
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
- [ ] Arm `gate/conformance` — publish pass rates **with the honest failure
      list** (P4).
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
- [ ] **P1 partial:** K1 green on all 1,000 corpus files. *Output required.*

### M2 — Structural diff

*Proof obligation: **P4**; `gate/differential` arms. Gates: `gate/golden`,
`gate/differential`.*

- [ ] Oracle first: diff golden suite — hand-built cases where line-based diff
      is wrong and structural diff is right. **Confirm red.**
- [ ] Semantic-tree matching (Chawathe/GumTree-class), Dijkstra-style structural
      diff à la difftastic.
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
