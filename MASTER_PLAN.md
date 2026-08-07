# BRAND — Ecosystem Master Plan
### The Deterministic Toolbox: everything we ship is provably correct, every run.

**Version:** 1.0 · **Date:** 2026-08-07 · **Owner · Design · Implementation:** Ayush Gupta (`ayushgupta07xx`)
**Status of this document:** the single source of truth. Every implementation session starts by reading this file and the relevant crate's `DESIGN.md`. `BRAND` and the binary name `<b>` are placeholders until Decision D1 (§16).

---

## 0. North Star

**Thesis (one sentence, memorize it):** We build local-first developer tools whose excellence is *deterministic and machine-verifiable* — exact correctness, lossless transformations, reproducible runs — never probabilistic quality.

**The admission test.** A technology, feature, or product enters this ecosystem only if it strengthens a proof. Before adding anything, answer in one line: *"What does this prove?"* If the honest answer is "it adds a keyword" or "it might usually work," it is rejected. Restraint is a feature.

**Priority order (permanent, non-negotiable):**

1. **Soundness** — never silently wrong. When uncertain, emit a conflict, a refusal, or a structured report. A wrong answer once destroys the brand.
2. **Completeness** — resolve/handle as much as possible, measured and published, but never at soundness's expense.
3. **Performance** — fast enough to feel instant; benchmarked honestly, including where incumbents beat us.
4. **Features** — last, always.

**Strategy in one paragraph.** This is a platform, not a tab bar (the Astral model: separate products, one brand, one shared philosophy). We ship one product at a time to 100%, each with its own launch, audience, and proof suite. All products stand on one shared kernel, so every product compounds the platform and every launch compounds the brand. The unified "workbench" shell is the season finale, shipped only after individual tools have adoption. The recruiter-facing story this produces: *"I designed a shared lossless-format kernel and a verification harness, and shipped N products on it"* — platform engineering, demonstrated rather than claimed.

**Permanent bans (Appendix C is the full list):** no load-bearing ML anywhere; no semantic/similarity caches; no heuristic scoring that can cry wolf; no telemetry; no network access by default; no cherry-picked benchmarks; no products that violate a platform's ToS; no patent-encumbered codecs.

---

## 1. Ecosystem Map

```
                          ┌─────────────────────────────────────────┐
                          │   WING 0 — VERIFICATION (the spine)     │
                          │   core-verify: golden files, fuzzing,   │
                          │   property tests, differential tests    │
                          │   → grows into `coverify`, the DST      │
                          │     harness product (Phase 5)           │
                          └───────────────┬─────────────────────────┘
                                          │ proves everything below
        ┌─────────────────────────────────┼──────────────────────────────────┐
        │                                 │                                  │
┌───────▼─────────────────────────┐       │            ┌─────────────────────▼──────────┐
│ WING 1 — DOCUMENT KERNEL        │       │            │ WING 2 — DETERMINISTIC AI-INFRA │
│ (launch wing, ships first)      │       │            │ (gated: opens only after ≥2     │
│                                 │       │            │  Wing-1 launches with traction) │
│ 1. konflux   structural 3-way   │       │            │                                 │
│              merge + diff        │       │            │ 7. replaylab  record/replay     │
│ 2. strukt    query/edit any     │       │            │               LLM proxy,        │
│              config, lossless   │       │            │               hash-verified     │
│ 3. bigsheet  offline large-data │       │            │ 8. cage       default-deny      │
│              workbench (+ logs) │       │            │               agent sandbox     │
│ 4. pdfsurgeon lossless PDF ops  │       │            │                                 │
│ 5. veritas   converter with     │       │            │ shared substrate: the cassette/ │
│              proof-of-fidelity  │       │            │ trace format                    │
│ 6. lockproof lockfile differ +  │       │            └─────────────────────────────────┘
│              provenance verify  │       │
└───────────────┬─────────────────┘       │
                │                         │
        ┌───────▼─────────────────────────▼───────┐
        │  SHELL — unified workbench (LAST)       │
        │  Tauri app mounting tools as modules    │
        └─────────────────────────────────────────┘
```

**The connective logic (why nothing is forced):** every Wing-1 product is a different head on the same lossless parse → transform → serialize kernel, proven by the same Wing-0 harness. lockproof is konflux pointed at lockfiles. strukt is the kernel's query/edit surface. veritas is core-verify's fidelity report productized. bigsheet extends "lossless" to tabular data with an exact query engine. Wing 2 reuses the same doctrine (byte-identical replay, deterministic policy) on AI workloads, sharing one cassette format between its two tools. One invariant, eight expressions of it.

---

## 2. Monorepo Architecture

```
BRAND/                            # Cargo workspace, public from day one
├── ENGINEERING.md                # distilled operating rules for implementation (§9)
├── MASTER_PLAN.md                # this file
├── MILESTONES.md                 # live checklist, one section per phase
├── adr/                          # ADR-NNN-title.md, template in §9.4
├── crates/
│   ├── core-cst/                 # lossless concrete syntax trees (§3.1)
│   ├── core-formats/             # Format trait + per-format impls (§3.2)
│   ├── core-verify/              # proof harness: golden/fuzz/property/differential (§3.3)
│   ├── core-cli/                 # shared CLI conventions, diagnostics, JSON output (§3.4)
│   └── core-tui/                 # shared ratatui components + theme (§3.4)
├── tools/
│   ├── konflux/                  # Phase 1
│   ├── strukt/                   # Phase 2
│   ├── bigsheet/                 # Phase 3  (Tauri app + CLI)
│   ├── pdfsurgeon/               # Phase 4  (own object-model crate, not core-cst)
│   ├── veritas/                  # Phase 4
│   ├── lockproof/                # Phase 4  (CLI + GitHub Action)
│   ├── coverify/                 # Phase 5  (productized core-verify + DST executor)
│   ├── replaylab/                # Phase 5+ (Wing 2)
│   └── cage/                     # Phase 5+ (Wing 2)
├── shell/                        # Phase 6, Tauri workbench
├── corpora/                      # test corpora as git submodules / fetch scripts
│   ├── helm-charts/  kustomize/  terraform/  lockfiles/  pdfs/  csv/
├── benches/                      # criterion + published benchmark pages
├── fuzz/                         # cargo-fuzz targets, one per format per operation
├── xtask/                        # repo automation (corpus refresh, badge gen, release)
└── .github/workflows/            # the CI law (§8)
```

**Conventions.** Rust stable, MSRV pinned and CI-checked. Single multicall binary `<b>` (BusyBox-style: `<b> merge`, `<b> q`, `<b> sheet`, … plus symlinked names `konflux`, `strukt`, …) — itself a small deep-tech flourish. Every tool also builds standalone. License: MIT OR Apache-2.0. Dependency policy: minimal and pinned; every new dependency needs a one-line justification in the PR; `cargo-deny` (advisories + licenses) blocking in CI. GUI stack: Tauri (Decision D2) so the web-frontend surface stays thin and the core stays Rust.

**Platform matrix.** Linux x86_64 is the source of truth (CI runners; Ayush's WSL2 Ubuntu 22.04 matches). Windows and macOS are release targets built and smoke-tested in the matrix. cage (sandbox) is Linux-first with an honest platform-support table in its README — never fake cross-platform security claims.

---

## 3. Kernel Specifications

### 3.1 `core-cst` — lossless concrete syntax trees

The load-bearing crate of the entire ecosystem. A CST here owns **every byte** of the input: keys, values, comments, whitespace, ordering, YAML anchors/aliases, quoting style, line endings, trailing garbage. Design: green/red tree (rowan-style) or owned token tree — the choice is **ADR-001**, made after a 2-day spike comparing edit ergonomics and memory footprint.

**Invariants (machine-checked, permanent):**
- **K1 Round-trip:** `serialize(parse(x)) == x` byte-identical, for every x in the corpus and every fuzz input that parses. This is the credibility of the whole platform; it is the first CI gate ever written and it never comes out.
- **K2 Edit locality:** after an edit operation, all bytes outside the edited span(s) are unchanged.
- **K3 Determinism:** identical input + identical operation sequence → identical output bytes, on every platform. No iteration-order leaks (BTreeMap/IndexMap only in output paths), no timestamps, stable sorts everywhere.

**Escape hatch for hostile input:** anything the modeled grammar can't represent (exotic YAML tags, weird encodings) is preserved as an opaque verbatim node rather than normalized. Preserving beats understanding; K1 outranks elegance.

### 3.2 `core-formats` — one trait, many formats

```rust
trait Format {
    fn parse(&self, input: &[u8]) -> Result<Cst, ParseReport>;   // never panics; report has spans
    fn serialize(&self, cst: &Cst) -> Vec<u8>;                   // total function
    fn semantic_view(&self, cst: &Cst) -> SemanticTree;          // typed layer for diff/merge/query
    fn merge_hints(&self) -> MergeHints;                         // e.g. K8s list-identity keys
    fn conformance_suite(&self) -> Option<SuiteAdapter>;         // official test suite hookup
}
```

Rollout order: **yaml, json** (Phase 1) → **toml, hcl** (Phase 2) → **csv, jsonl, logfmt** (Phase 3, feeding bigsheet) → **lockfiles**: package-lock.json, Cargo.lock, uv.lock, yarn.lock, pnpm-lock.yaml, go.sum (Phase 4, feeding lockproof). PDF is architecturally different (object graph + xref, not a text CST) and lives in its own crate under pdfsurgeon; it still answers to core-verify.

The semantic layer is where domain intelligence lives: K8s-aware list merging (match containers by `name`, env vars by key — not by list position), Helm/kustomize awareness, TF block identity. This is konflux's moat over generic tools and the piece incumbents (diff3, Mergiraf today) lack.

### 3.3 `core-verify` — the proof harness (Wing 0's seed)

A library + internal CLI providing, uniformly to every crate:
- **Golden runner:** directories of `(input(s) → expected)` cases; failure prints a span-level diff. Golden files are write-protected by policy (§8: they may never be edited to make CI pass).
- **Round-trip fuzzer:** cargo-fuzz targets per format per operation; corpus-seeded; any K1 violation is minimized and auto-filed as a failing golden case.
- **Property kit:** proptest strategies per format; the merge/edit algebra laws (§4) as reusable properties.
- **Differential runner:** run ours vs an external oracle (`git merge-file`, Mergiraf, jq, yq, DuckDB CLI, cosign, qpdf) on the same inputs; classify agreements/divergences; divergences become triaged golden cases.
- **Conformance adapters:** yaml-test-suite, JSONTestSuite, toml-test, BurntSushi/toml-test harness style; veraPDF for PDF/A; published pass-rate badges.
- **Report emitter:** human, JSON, and markdown-badge output. veritas (§4.5) is this emitter productized; coverify (§5) is this whole crate productized.

### 3.4 `core-cli` / `core-tui` — one feel across every tool

Every tool, by law: `--json` (stable machine-readable output, schema versioned), `--check` (exit-code-only mode for CI), deterministic output ordering, `NO_COLOR` respected, **no network access ever** unless an explicit `--online` flag exists and the README explains why, span-rich error diagnostics (miette-style: show the bytes, point at the problem, suggest the fix). TUI components (ratatui) share one theme: **crimson `#e5484d` on near-black `#0a0a0c`** — Ayush's existing identity system, carried through every tool, the docs site, and social cards. [AYUSH owns all visual design.]

---

## 4. Wing 1 Product Specifications

Every product spec follows the same skeleton: Mission · Pitch · Scope (MVP → Full) · Non-goals · Deep core · **Proof obligations (numbered, testable — these ARE the definition of done)** · Milestones · Launch gate · Role signal.

### 4.1 `konflux` — structural diff & 3-way merge for configs — **FLAGSHIP**

**Mission:** end line-based merge for structured config. **Pitch:** *"Git merges that finally understand your YAML. Structural 3-way merge and diff for Kubernetes, Terraform, and Helm configs — comments and key order preserved, zero false conflicts."*

**MVP:** YAML + JSON structural diff and 3-way merge, shipped as a git merge-driver + `git mergetool` integration, with the byte-identical round-trip guarantee. **Full:** TOML + HCL; Kubernetes semantic merging (list-by-key); Helm/kustomize awareness; TUI conflict resolver; `--check` CI mode ("will these branches conflict structurally?").
**Non-goals:** general text merge (delegate to git); auto-resolving semantic conflicts it cannot prove safe; any AI-assisted resolution (banned).

**Deep core:** core-cst parsing → semantic-tree matching (Chawathe/GumTree-class algorithms, Dijkstra-style structural diff à la difftastic) → true 3-way merge over trees with path-based conflict detection → **format-preserving serialization of the merged result** (the genuinely hard part and the reason the gap exists) → conflict presentation as structured, span-anchored blocks.

**Proof obligations:**
- **P1 Round-trip:** K1 holds on a corpus of ≥1,000 real-world files (top Helm charts, kustomize overlays, Terraform repos, K8s manifests) plus ≥72 cumulative hours of fuzzing with zero violations.
- **P2 Merge algebra (property tests):** `merge(A,A,A)=A` · `merge(Base,X,Base)=X` · `merge(Base,Base,X)=X` · stability (same inputs → same output bytes) · conflict symmetry (swapping ours/theirs swaps conflict sides, never changes what conflicts).
- **P3 Soundness gate:** **zero incorrect merges** on a golden suite of ≥2,000 triples — hand-built edge cases plus real conflicts mined from OSS git histories (replay historical merges; compare against the human's committed resolution). Anything uncertain must surface as a conflict. Auto-resolution rate vs `git merge-file`/diff3 and vs Mergiraf is *measured and published*, but never bought with soundness.
- **P4 Conformance:** yaml-test-suite, JSONTestSuite, toml-test pass rates published as badges, including honest failure lists.

**Milestones:** M1 CST + K1 for yaml/json → M2 structural diff (side-by-side CLI output) → M3 3-way merge core + P2/P3 harness → M4 git merge-driver + mergetool + install one-liner → M5 K8s semantic layer → M6 TUI conflict view → Launch. **Launch gate:** P1–P4 green in public CI, benchmark table (including where Mergiraf/diff3 win), 60-second screencast, README per §12.

**Role signal:** SWE-generalist, developer tools/DX, language tooling, platform engineering, DevOps/SRE → Google EngProd, Meta Dev Infra, GitHub/GitLab, HashiCorp, JetBrains, Sourcegraph.

### 4.2 `strukt` — deterministic query & edit for every config format

**Mission:** the yq/jq category, rebuilt on a lossless kernel. **Pitch:** *"Query and edit YAML/JSON/TOML/HCL from the command line — without destroying comments, key order, or formatting."* Today's tools normalize the whole file to change one value; strukt touches only what you asked.

**MVP:** path query language (jq-inspired, small and boring on purpose), get/set/delete/insert, in-place edit with K2 edit-locality. **Full:** structural grep across repos, format-aware bulk refactors ("bump image tag across 40 charts"), shell completions, editor integration.
**Non-goals:** a Turing-complete query language; jq's full feature surface.

**Proofs:** **P1** K2 edit-locality — bytes outside the edited span byte-identical, fuzz-verified. **P2** differential vs jq on JSON: semantically identical query results on a 10k-query corpus. **P3** idempotence: applying the same edit twice = applying once. **P4** determinism across platforms.
**Why it ships second:** it is ~90% kernel reuse — a fast follow that proves the platform thesis publicly ("second product in 4 weeks *because* of the kernel"), and it is the single most daily-useful tool in the set for working DevOps engineers.
**Role signal:** same as konflux, plus CLI/DX craft; the pair together is the dev-tools story.

### 4.3 `bigsheet` — the offline large-data workbench

**Mission:** kill the 1,048,576-row wall. **Pitch:** *"Open a 5 GB CSV instantly and query it like a spreadsheet — offline, and it never silently drops a row."*

**MVP:** open CSV/Parquet/XLSX/JSONL far past Excel's limits; virtualized grid (Tauri) with instant scroll; SQL pane (embedded DuckDB); exact row/column counts always visible — the **never-silently-truncate guarantee** is the anti-Excel brand promise. **Full:** logfmt + structured-log ingestion (this *absorbs* the earlier "log query engine" idea as input formats — we do not build a second query engine), lossless CSV↔Parquet↔XLSX conversion with typed fidelity reports (veritas integration), saved views, joins across files, `bigsheet-cli` for headless use.
**Non-goals:** charting suites, collaboration/cloud, competing with Excel on formula breadth.

**Proofs:** **P1** exactness — SQL results differentially tested against DuckDB CLI and golden hand-computed queries; any divergence is a release blocker. **P2** fidelity — CSV↔Parquet round-trip preserves values + types or says exactly what changed (report golden-tested); multi-dialect CSV parsing validated on a nasty-real-world corpus. **P3** no-truncation — property tests: rows in == rows reported, always. **P4** performance *budgets* (honest framing: budgets, not proofs) — open-time and scroll-latency targets on named public datasets (NYC Taxi, Stack Overflow dump), published with methodology.
**Role signal:** data engineering, data-infra SWE, query/storage, analytics engineering; the log angle adds observability → Databricks, Snowflake, ClickHouse, MotherDuck, DuckDB Labs, Datadog, Grafana, Elastic. Broadest non-developer audience in the ecosystem; the designated pivot flagship if konflux's validation signal is weak (§11).

### 4.4 `pdfsurgeon` — lossless PDF operations + forms

**Mission:** the PDF tasks everyone needs, done provably losslessly, locally. **Pitch:** *"Fill, sign, flatten, merge, and split PDFs on your machine — untouched pages stay byte-identical, and forms render the same in every viewer."*

**MVP:** merge/split/reorder/rotate with incremental-save (untouched objects byte-preserved); AcroForm fill → flatten with correct appearance-stream generation (the documented open sub-gap: forms that render identically in Chrome, Firefox, Acrobat, Preview). **Full:** PDF/A conversion with veraPDF-verified conformance, redaction (true content removal, verified), attachment/metadata surgery.
**Non-goals:** a WYSIWYG editor; content *editing*; OCR (probabilistic — banned as a core promise); out-featuring Stirling-PDF wholesale. We win the narrow lossless/forms lane.

**Proofs:** **P1** qpdf structural-equivalence + byte-preservation checks on untouched pages. **P2** render-diff golden images across ≥2 independent renderers (pdfium + Poppler/MuPDF) within a pixel threshold, per operation, per corpus file. **P3** filled+flattened forms render-diff-identical across the renderer set. **P4** veraPDF conformance wherever PDF/A is claimed. **P5** fuzzing the parser on a hostile-PDF corpus — parser never panics, never mis-saves.
**Role signal:** systems SWE (binary formats, rendering, spec conformance), document infra → Adobe, DocuSign, Dropbox, Google Docs/Drive, Chrome-pdf.js-class teams.

### 4.5 `veritas` — the converter that proves what it kept

**Mission:** convert files and *tell the truth about it*. **Pitch:** *"Convert files locally — with a receipt proving exactly what survived and what, if anything, was lost."*

The product IS the fidelity report: veritas re-parses its own output, structurally diffs it against the input's semantic tree, and emits a signed, structured receipt (perfect | lossy-with-itemized-losses | refused). Conversion pairs are admitted only when a decidable fidelity report is possible — depth over breadth, permanently.
**MVP:** the format pairs the kernel already speaks (yaml↔json↔toml, csv↔parquet↔xlsx via bigsheet's engine, markdown→pdf via pdfsurgeon). **Full:** more pairs strictly as kernel coverage grows; a public "fidelity matrix" page that doubles as marketing.
**Non-goals:** "convert anything" breadth; media/codec formats (patent trap — banned).
**Proofs:** **P1** round-trip on every pair claiming lossless. **P2** report-completeness golden suite: seeded known-loss inputs must produce exactly the expected loss items — the report itself is under test. **P3** determinism: same input → same output bytes + same receipt.
**Role signal:** correctness engineering + formats; strong general-SWE story ("I built a converter that audits itself").

### 4.6 `lockproof` — lockfile intelligence + provenance verification

**Mission:** make dependency updates legible and verifiable. **Pitch:** *"Know exactly what changed when your lockfile did: every new transitive dep, loosened pin, install script, and unsigned package — before you merge."*

This is the strongest crossover in the ecosystem: **lockfiles are just JSON/TOML/YAML — lockproof is a konflux head plus a verification layer.** **MVP:** structural, semantic lockfile diff for npm (package-lock.json) + Cargo.lock + uv.lock, as a CLI and a GitHub Action that comments on PRs: "+3 transitive deps (list), 1 version pin loosened (^ → *), +1 preinstall script, 2 packages without provenance." **Full:** yarn/pnpm/go.sum; Sigstore signature + SLSA provenance + hash verification (pure pass/fail — cryptographic, binary, provable); policy gates ("fail CI if any new install scripts").
**Non-goals (permanent):** typosquat heuristics, "suspiciousness" scores, behavioral ML — anything that can cry wolf. lockproof states facts and verifies signatures; it never guesses.
**Proofs:** **P1** parse fidelity on the top-1,000 real lockfiles per ecosystem (mined from popular repos) — zero parse failures, K1 round-trip. **P2** verification differential vs reference implementations (cosign, npm/cargo native checks) — full agreement or triaged divergence. **P3** diff completeness golden suite: seeded lockfile changes must each surface exactly once, correctly classified. **P4** deterministic reports.
**Role signal:** security engineering, supply-chain, DevSecOps, platform security → GitHub, Google OSS-Security, Chainguard, Socket, AWS Security. Distribution advantage: lives inside CI where the free tier is the natural habitat.

---

## 5. Wing 0 — `coverify`: verification as a product

**Path:** internal harness (Phase 1) → published crate other Rust projects can adopt (Phase 3) → full product (Phase 5): a **deterministic simulation testing** harness — deterministic async executor, simulated clock/network/disk fault injection, seed-based reproduction, time-travel trace replay, and (stretch) a linearizability checker.

**Pitch:** *"The harness we used to prove every one of our tools lossless — now pointed at your code. Find the 1-in-a-million bug, get a seed, replay it forever."*

**Proofs (the product proves itself):** **P1** same seed → byte-identical execution trace, enforced by trace-hash in CI across platforms. **P2** demo suite: deterministically reproduce ≥3 known historical concurrency bugs from public OSS issues, each with a one-command repro. **P3** the entire BRAND monorepo runs under coverify in CI — we are user zero, publicly.
**Why this is the most senior artifact in the plan:** "my internal test infrastructure was good enough to productize" is a staff-engineer sentence. It is also the connective tissue that makes the ecosystem a *doctrine* rather than a pile of tools.
**Role signal:** distributed-systems SWE, correctness/test-infra, database/storage teams → AWS, Antithesis, TigerBeetle-class infra, any serious DB company.

---

## 6. Wing 2 — Deterministic AI-Infra (gated)

**Gate to open this wing:** ≥2 Wing-1 launches complete **and** at least one showing real adoption signal (§14). Wing 2 exists because the doctrine transfers perfectly to the 2026 AI-infra frontier — but it must stand on an established brand, not launch one.

### 6.1 `replaylab` — record/replay proxy for LLM workloads

**Pitch:** *"Make any agent run 100% reproducible. Record once, replay byte-identically, forever — and know exactly what every run cost."*
**Deep core:** OpenAI/Anthropic-compatible local proxy; **cassette** recording of full request/response streams; hash-verified byte-identical replay for tests/CI/debugging; **exact-prefix caching only** (the semantic cache is permanently banned — an embedding-similarity cache hit that returns a subtly wrong answer is precisely the probabilistic failure this brand exists to reject); deterministic token/cost ledger (arithmetic, not estimates); streaming with backpressure.
**Proofs:** **P1** replay determinism — replayed run's transcript hash equals recorded hash, always. **P2** passthrough transparency — proxied vs direct responses byte-identical modulo an allow-listed header set, differentially tested. **P3** cassette schema versioned with migration tests. **P4** ledger exactness against provider-reported usage on golden traces.
**Role signal:** AI-infra SWE, LLMOps/agent platform, SRE-for-AI → Anthropic, OpenAI, AWS Bedrock, Cloudflare AI.

### 6.2 `cage` — default-deny sandbox for agent-executed code

**Pitch:** *"Let AI agents run code on your machine without trusting them. Deny by default; every allowed capability is an explicit, logged decision."*
**Deep core:** capability-based sandbox — WASI preview-2 for portable workloads plus Landlock/seccomp/namespaces on Linux for native processes; path-jailed filesystem; egress allowlists with SSRF-safe resolution; CPU/mem/time limits; MCP tool-call gating; tool-call record/replay sharing replaylab's cassette substrate (the Wing-2 integration).
**Proofs:** **P1** adversarial escape suite — a versioned battery of escape attempts (secret reads, exfil, process spawn, path traversal, symlink games, DNS rebinding) all denied, running as a permanent CI regression gate that only grows. **P2** policy determinism — same policy + same request → same allow/deny + same audit log line. **P3** overhead benchmarks vs raw execution, honest.
**Marketing law:** claim **"provable default-deny policy enforcement"** — never "unescapable." No honest security tool claims the latter; pretending otherwise would burn the exact credibility the brand is built on. Linux-first with an explicit platform matrix.
**Launch:** replaylab + cage together — *"run agents reproducibly and safely"* — one story, two tools, shared cassettes.
**Role signal:** security + systems SWE (OS sandboxing, WASM, capabilities), agent platform → Anthropic, OpenAI, Cloudflare Workers, gVisor/Chrome-sandbox-class teams.

---

## 7. The Shell — unified workbench (Phase 6, the finale)

A Tauri workbench mounting each tool as a module: konflux's conflict view, strukt's query pane, bigsheet's grid, pdfsurgeon's page view, veritas's fidelity matrix, lockproof's PR reports, coverify's trace explorer. Ships **only** after ≥3 tools have standalone adoption. Its launch story writes itself: *"the deterministic toolbox — everything provably lossless, one app."* Until then, the shell is a directory with a README and zero code. No exceptions; the tab-bar temptation is how ecosystems die as demos.

---

## 8. Verification Doctrine — the CI Law

Applies to every crate, from the first commit. CI is the objective feedback loop that makes fast-moving development trustworthy; it is never weakened to go faster.

| Gate | Tooling | Blocking? |
|---|---|---|
| Format + lints | rustfmt, clippy (pedantic) | Yes |
| Unit + property tests | cargo test, proptest | Yes |
| Golden-file suites | core-verify runner | Yes |
| Fuzz smoke (per-PR) | cargo-fuzz, ~5 min/target | Yes |
| Long fuzz (nightly) | cargo-fuzz, capped for free tier; extended runs on Ayush's machine via cron | Report → new goldens |
| Conformance suites | yaml-test-suite, JSONTestSuite, toml-test, veraPDF | Yes (published pass rates) |
| Differential suites | vs git merge-file, Mergiraf, jq, DuckDB CLI, cosign, qpdf | Yes on agreement set |
| Memory/UB | miri (kernel crates), ASan/UBSan jobs | Yes |
| Supply chain | cargo-deny (advisories, licenses, bans) | Yes |
| Benchmark regression | criterion vs saved baselines, ±threshold | Yes on regression |
| Determinism check | double-build + double-run output-hash compare | Yes |
| Docs + MSRV | cargo doc, MSRV matrix | Yes |

**The anti-reward-hacking law (verbatim into ENGINEERING.md):** It is never permissible to (a) edit golden files, (b) loosen thresholds, (c) delete/skip/weaken tests, or (d) special-case test inputs in product code, in order to make CI pass. Any such change must be proposed in the PR description under a `[NEEDS-AYUSH-APPROVAL]` header with justification, and lands only after human sign-off. **Oracle-first development:** for every milestone, the tests/golden cases/fuzz targets are written and merged *before* the implementation. Red → green → ADR → PR.

---

## 9. Implementation Operating Protocol

**9.1 Repo contracts.** `ENGINEERING.md` (distillation of §0, §8, §9, Appendix C — the rules of engagement), per-crate `DESIGN.md` (scope, invariants, current milestone), `MILESTONES.md` (live checklist), `adr/` (every non-obvious decision).

**9.2 Session loop (every session, no exceptions):**
1. Read ENGINEERING.md + the target crate's DESIGN.md. One crate per session; small scopes.
2. Pick exactly one milestone item; restate it + acceptance criteria in one paragraph.
3. **Write/extend the oracle first** (tests, goldens, fuzz target, property). Confirm red.
4. Implement until green. Paste failing output verbatim when stuck; no speculative rewrites.
5. If any design decision was made, write the ADR in the same PR.
6. Self-review the diff (checklist: determinism leaks? new deps justified? error spans? `--json` stable?).
7. PR ≤ ~600 lines with a **Proof Delta** section: *"what is now proven that wasn't before."*

**9.3 Human review points (Ayush, non-delegable):** every ADR; every public API change; every golden-file change; every new dependency; every README/launch artifact. Ayush's role is architect + editor + the person who can defend every decision live — the plan fails without this layer regardless of code quality.

**9.4 ADR template:**
```
# ADR-NNN: <decision>
Date · Status (proposed/accepted/superseded by NNN)
Context: what forced a choice (1 short para)
Options: A / B / C with one honest sentence each, incl. costs
Decision: what and WHY (the sentence Ayush says in an interview)
Consequences: what gets harder; what we're betting on
Proof impact: which invariants/gates this touches
```

**9.5 Determinism hygiene (recurring failure modes to preempt):** no HashMap iteration in any output path; no wall-clock in outputs unless flagged; stable sorts; fixed float formatting; seeded randomness only, seed logged; path handling identical across OS (normalize separators in output); locale-independent formatting.

---

## 10. Hard Constraints

- **Budget: $0.** Ayush's laptop (WSL2 Ubuntu 22.04 primary dev; CI Linux runners are the truth source) + free GitHub Actions on a public repo. Long fuzz/bench runs execute locally on a schedule; CI stays within free minutes by capping fuzz smoke and using path-filtered workflows + caching.
- **Distribution:** GitHub Releases (static binaries, all three OS), crates.io, Homebrew tap, one-line install script, Docker image for lockproof's Action. Docs: MkDocs (existing skill) on GitHub Pages.
- **Privacy is a feature:** no telemetry, no phone-home, offline by default, forever. Stated in every README.
- **Legal rails:** MIT OR Apache-2.0; no patent-encumbered codecs (HEVC-class — banned); no ToS-violating products (LinkedIn automation-class — banned); corpus files respect upstream licenses (fetch scripts, not vendored copies, where required).
- **Solo-maintenance reality:** every README carries a "Maintained scope" section (what is in/out of support) and issue templates that route noise; a product can be frozen honestly (§14) — the kernel survives any single product's death.

---

## 11. Sequencing, Gates, Timeline

Aggressive but honest; slippage rule at the bottom. Each phase ends with a complete, standalone, launch-grade asset — the plan survives Ayush getting hired at any checkpoint (that is a success path, not an interruption).

| Phase | Weeks | Work | Exit gate |
|---|---|---|---|
| 0 — Validate + scaffold | 0–1 | Brand shortlist (D1), monorepo + CI skeleton (all §8 gates green on hello-world), corpus fetchers, validation posts (r/devops, r/kubernetes, Ask HN): the konflux one-liner + mockup | Signal read: ≥50 upvotes or ≥20 "I hit this weekly" replies → konflux confirmed. Weak signal → **pivot flagship to bigsheet**, konflux slides to Phase 4 (kernel still gets built, via strukt) |
| 1 — konflux | 1–10 | M1–M6 (§4.1) | P1–P4 green publicly → **Launch 1** |
| 2 — strukt | 10–14 | Kernel query/edit surface, toml+hcl formats | Proofs green → **Launch 2** ("second product in 4 weeks because of the kernel") |
| 3 — bigsheet | 14–26 | Grid + DuckDB + formats; core-verify published as a crate | Proofs green, budgets met → **Launch 3** |
| 4 — pdfsurgeon **or** veritas, + lockproof | 26–38 | Pick pdfsurgeon vs veritas by user pull from Launches 1–3 (D5); lockproof rides the kernel + Action | **Launches 4–5** |
| 5 — coverify + Wing 2 gate | 38–50 | coverify productized; if Wing-2 gate (§6) passes → replaylab + cage joint build | **Launch 6** (+7 if gated in) |
| 6 — Shell | 50+ | Workbench, only if ≥3 tools have adoption | The finale launch |

**Slippage rule:** if a phase runs >50% over, cut *scope* (formats, features) — never proofs, never soundness. **Job-hunt checkpoints:** every launch triggers an application wave with that product as the lead artifact for its role cluster (§17 table); DEFENSE.md (§13) updated same week.

---

## 12. Launch Playbook (per product)

- **README = the landing page.** 15-second sell at top: demo GIF, one-line install, proof badges (round-trip ✓ fuzz-hours counter, conformance %, soundness suite ✓), then the benchmark table **including the cells where incumbents win** — calibration is the credibility.
- **Show HN:** "Show HN: <name> — <plain what-it-does>." Maker comment: why I built it, how the proofs work, one honest limitation, one question for the room. Post US weekday morning; concentrate HN + r/devops/r/kubernetes + Product Hunt + LinkedIn into the same day (GitHub Trending rewards a spike).
- **Benchmark methodology page:** hardware, versions, commands, raw data in-repo, reproduction one-liner. Anything less gets shredded, correctly.
- **Per-launch content:** one deep-dive blog post (the hardest subsystem), ADR links, 60-second screencast, LinkedIn post whose hero image is the proof/benchmark chart — the comprehensible face over the deep core, which is the whole brand in one image.

---

## 13. Credibility Layer — the Authorship Defense

The decisive 2026 risk: a repo is discounted unless its author demonstrably owns the architecture and can defend every decision live. Standing artifacts, per product:
- Public **ADR corpus** (the "why" trail a rushed repo never has).
- One **deep-dive post** per product on its hardest subsystem (format-preserving serialization; appearance streams; deterministic scheduling).
- A recorded **whiteboard walkthrough**: why CST over AST, why Chawathe-class matching, why soundness-over-completeness, why exact-prefix-only caching.
- **DEFENSE.md** per product (private): the 20 questions a Google interviewer would ask + Ayush's answers; updated every launch; the pre-interview self-test is "explain any file in the repo in 2 minutes."
- Interview law: the repo gets the interview; the whiteboard gets the offer. Time is budgeted for understanding, not just shipping — reading the merge algorithm until it can be re-derived from memory is part of the schedule, not optional.

---

## 14. Metrics, Success, Kill/Pivot Criteria

**Track (in order of truth):** GitHub *dependents/"used by"* and merge-driver installs (proxied via install-script + release download deltas, since we have no telemetry) → organic issues from strangers → crates.io/Homebrew downloads → stars last. **Base-rate honesty:** ~90% of even-popular repos never pass 5k stars; the win condition is becoming the *default in a niche* (the K8s merge driver; the "open huge CSV" answer), not virality.
**Per-product kill rule:** 8 weeks post-launch with <100 meaningful engagements and zero organic issues → freeze honestly (README notice), extract a post-mortem post (itself a credibility artifact), move on. The kernel and harness survive every freeze — the platform is the sunk-cost shield.
**Ecosystem success at 12–18 months = any two of:** (a) ≥2 tools with real adoption, (b) kernel + coverify published and used by others, (c) hired into a target role on the strength of this work.

---

## 15. Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Format-preserving serialization is brutally hard (YAML anchors, merge keys, exotic tags) | It gets the majority time budget by design; verbatim-node escape hatch (§3.1); K1 fuzzing from week 1 |
| A single incorrect merge in the wild | Soundness gate P3; conflict-on-uncertainty; incident playbook: reproduce → golden case → fix → public post-mortem |
| Incumbent response (Mergiraf adds formats; difftastic reverses no-merge stance) | Move fast on K8s/Helm semantics + soundness guarantee + UX — the moats they lack; being second with proofs beats first without |
| Authorship doubt — architecture ownership questioned | §13 in full; ADR-first culture from commit 1 |
| PDF spec hostility | Narrow scope (page ops + forms), hostile-corpus fuzzing, two-renderer verification, never claim "editor" |
| Adoption base rate | Niche-default strategy; distribution inside existing workflows (git driver, GH Action, `brew install`) |
| Solo burnout | Phase gates = clean stopping points with complete assets; kill rule removes zombie obligations |
| GH Actions minute limits | Public-repo free tier + path filters + caching; heavy runs local-nightly |
| WSL2 quirks (BTF, perf) | CI Linux runners are the truth source; WSL2 is a dev convenience only |
| Naming/trademark collision | D1 checklist: crates.io + GH org + domain + basic TM search before any public use |

---

## 16. Naming & Design — [AYUSH-owned]

**D1 Umbrella brand criteria:** ≤2 syllables, pronounceable, crates.io + npm + GitHub org + .dev/.io domain free, no trademark collision, meaning-adjacent to *proof/exact/lossless*. Tool names stay as sub-brands (konflux, strukt, bigsheet, pdfsurgeon, veritas, lockproof, coverify, replaylab, cage) — rename any that clash at D1 time. **Visual identity:** crimson `#e5484d` on `#0a0a0c` (existing system) → design-tokens file consumed by TUI theme, docs site, social cards, GIF frames. **Open decisions:** D1 brand · D2 Tauri confirm · D3 flagship confirm post-validation · D4 license final · D5 pdfsurgeon-vs-veritas order · D6 docs IA.

---

## 17. Final Expected Outputs (the complete artifact list)

**Code:** the monorepo — 5 kernel/shared crates, 9 tools, shell; one multicall binary + standalone binaries for Linux/Windows/macOS; a GitHub Action (lockproof); published crates (core-verify/coverify at minimum).
**Proof surface (public, live):** CI dashboards per §8; fuzz-hour counters; conformance pass rates; soundness suite status; benchmark pages with raw data; the fidelity matrix (veritas).
**Content:** 6–9 Show HN launches; ≥6 deep-dive posts; the ADR corpus; benchmark methodology docs; screencasts; MkDocs site.
**Career artifacts:** DEFENSE.md set; a portfolio narrative page mapping products → roles (Appendix A); each launch feeding a targeted application wave.

### Appendix A — Role-Signal Map
| Product | Primary roles | Example teams |
|---|---|---|
| konflux + strukt | SWE, dev-tools/DX, platform eng, DevOps/SRE | Google EngProd, Meta Dev Infra, GitHub/GitLab, HashiCorp, JetBrains, Sourcegraph |
| bigsheet | Data eng, data-infra SWE, query/storage, analytics eng, observability | Databricks, Snowflake, ClickHouse, MotherDuck, DuckDB Labs, Datadog, Grafana |
| pdfsurgeon + veritas | Systems SWE (binary formats, rendering, conformance) | Adobe, DocuSign, Dropbox, Google Docs/Drive, Chrome/pdf.js-class teams |
| lockproof | Security eng, supply-chain, DevSecOps, platform security | GitHub, Google OSS-Security, Chainguard, Socket, AWS Security |
| coverify | Distributed-systems SWE, correctness/test-infra — the staff-level signal | AWS, Antithesis, TigerBeetle-class infra, DB companies |
| replaylab | AI-infra SWE, LLMOps/agent platform, SRE-for-AI | Anthropic, OpenAI, AWS Bedrock, Cloudflare AI |
| cage | Security + systems SWE, agent platform | Anthropic, OpenAI, Cloudflare Workers, gVisor-class teams |
| Kernel + monorepo itself | Platform engineering, system design — the umbrella interview story | Everywhere |

*Not covered (deliberately):* frontend, mobile, ML research — the first two off-brand; the third already carried by the existing portfolio (SentinelOps, SafetyVision, eval work).

### Appendix B — Proof-Obligation Glossary
**Round-trip:** serialize(parse(x)) == x, byte-identical. **Golden:** frozen input→expected pairs; human-approved changes only. **Property:** algebraic laws over generated inputs (proptest). **Differential:** our output vs an independent oracle on identical inputs. **Conformance:** official/community spec suites, pass rates published. **Fuzz:** coverage-guided hostile-input generation; violations become goldens. **Regression gate:** any previously-green proof turning red blocks merge. **Determinism:** identical inputs → identical bytes, cross-platform, enforced by double-run hashing.

### Appendix C — Permanent Bans
Load-bearing ML/probabilistic components · semantic/similarity caches · heuristic "suspiciousness" scoring · telemetry/phone-home · network-by-default · editing goldens/thresholds to pass CI · cherry-picked or irreproducible benchmarks · "unescapable/unbreakable" security claims · ToS-violating integrations · patent-encumbered codecs · shipping the shell before three tools have adoption · cutting proofs to save a deadline.

---

*End of master plan. First action: Phase 0, Decision D1, and the validation posts. Hand ENGINEERING.md the distillation of §0/§8/§9/Appendix C and begin at konflux M1.*
