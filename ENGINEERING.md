# ENGINEERING.md — rules of engagement

**This file is a distillation of MASTER_PLAN.md §0, §8, §9 and Appendix C. The
master plan is the source of truth. Where this file and the master plan differ,
the master plan wins and this file is the bug.**

Read `MASTER_PLAN.md` in full at the start of every session. Then read the
target crate's `DESIGN.md`. Then read `MILESTONES.md`. Only then write code.

Ambiguity is not the implementer's to resolve. If anything in the master plan is
unclear or self-contradictory on scope, invariants or bans — **ask Ayush**. Do
not guess, do not fill the gap with your own judgment, do not proceed on an
assumption you have not stated out loud.

---

## 1. North Star (§0)

**The thesis, in one sentence — memorise it:**

> We build local-first developer tools whose excellence is *deterministic and
> machine-verifiable* — exact correctness, lossless transformations, reproducible
> runs — never probabilistic quality.

**The admission test.** A technology, feature or product enters this ecosystem
only if it strengthens a proof. Before adding anything, answer in one line:
*"What does this prove?"* If the honest answer is "it adds a keyword" or "it
might usually work," it is **rejected**. Restraint is a feature.

**Priority order — permanent, non-negotiable, and it is an order, not a list:**

1. **Soundness** — never silently wrong. When uncertain, emit a conflict, a
   refusal, or a structured report. *A wrong answer once destroys the brand.*
2. **Completeness** — resolve and handle as much as possible, measured and
   published, but never at soundness's expense.
3. **Performance** — fast enough to feel instant; benchmarked honestly,
   including where incumbents beat us.
4. **Features** — last, always.

When two of these conflict, the lower number wins. Every time. There is no
case where shipping a feature justifies a possibly-wrong answer.

**The platform, not a tab bar.** Separate products, one brand, one shared
philosophy, one kernel. Ship one product at a time to 100%. The unified shell is
the season finale and ships only after three tools have adoption.

---

## 2. The kernel invariants

These are named throughout the codebase. Know them by their IDs.

| ID | Invariant | Home |
|----|-----------|------|
| **K1** | `serialize(parse(x)) == x`, byte-identical, for every corpus file and every fuzz input that parses | `core-cst` |
| **K2** | After an edit, all bytes outside the edited span(s) are unchanged | `core-cst` |
| **K3** | Identical input + identical operation sequence → identical output bytes, on every platform | `core-cst` |

K1 is *"the credibility of the whole platform; it is the first CI gate ever
written and it never comes out"* (§3.1).

**Escape hatch for hostile input:** anything the modelled grammar cannot
represent is preserved as an opaque verbatim node rather than normalised.
**Preserving beats understanding; K1 outranks elegance.**

---

## 3. The CI Law (§8)

Applies to every crate, from the first commit. **CI is never weakened to go
faster.**

| Gate | Tooling | Blocking? |
|---|---|---|
| Format + lints | rustfmt, clippy (pedantic) | Yes |
| Unit + property tests | cargo test, proptest | Yes |
| Golden-file suites | core-verify runner | Yes |
| Fuzz smoke (per-PR) | cargo-fuzz, ~5 min/target | Yes |
| Long fuzz (nightly) | cargo-fuzz, capped for free tier | Report → new goldens |
| Conformance suites | yaml-test-suite, JSONTestSuite, toml-test, veraPDF | Yes (published pass rates) |
| Differential suites | vs git merge-file, Mergiraf, jq, DuckDB CLI, cosign, qpdf | Yes on agreement set |
| Memory/UB | miri (kernel crates), ASan/UBSan | Yes |
| Supply chain | cargo-deny (advisories, licenses, bans) | Yes |
| Benchmark regression | criterion vs saved baselines, ±threshold | Yes on regression |
| Determinism check | double-build + double-run output-hash compare | Yes |
| Docs + MSRV | cargo doc, MSRV matrix | Yes |

### 3.1 The anti-reward-hacking law — verbatim from §8

> **It is never permissible to (a) edit golden files, (b) loosen thresholds,
> (c) delete/skip/weaken tests, or (d) special-case test inputs in product code,
> in order to make CI pass. Any such change must be proposed in the PR
> description under a `[NEEDS-AYUSH-APPROVAL]` header with justification, and
> lands only after human sign-off.**

Read that again. It is the single most important paragraph in this file.

**What counts as evidence, and may not be quietly changed:**

- golden files (`**/tests/golden/**`)
- fuzz seed corpora (`fuzz/corpus/**`)
- benchmark baselines and tolerances (`benches/baselines/**`)
- corpus source manifests, caps and licence allow-lists (`corpora/sources/**`)
- conformance pass-rate thresholds
- any assertion in a test, property or fuzz target

**Tightening a threshold is free. Loosening one requires sign-off.**

If you believe a test, golden or threshold is genuinely wrong: **stop.** Do not
change it. Write the proposal under `[NEEDS-AYUSH-APPROVAL]` with the
justification and the exact delta, and wait. Being blocked is the correct
outcome; going green by weakening the oracle is not.

The `golden-guard` CI job enforces this mechanically, and `CODEOWNERS` routes
these paths to Ayush. Neither is a substitute for not doing it.

### 3.2 Oracle-first development — §8

> **For every milestone, the tests / golden cases / fuzz targets are written and
> merged *before* the implementation. Red → green → ADR → PR.**

Confirm red. Paste the failing output. *Then* implement. A test that has never
been observed failing is not known to test anything.

---

## 4. Session protocol (§9.2) — every session, no exceptions

1. Read this file + the target crate's `DESIGN.md`. **One crate per session.
   Small scopes.**
2. Pick **exactly one** milestone item. Restate it and its acceptance criteria
   in one paragraph before writing any code.
3. **Write or extend the oracle first** (tests, goldens, fuzz target, property).
   **Confirm red.**
4. Implement until green. When stuck, paste the failing output verbatim. **No
   speculative rewrites.**
5. If any design decision was made, write the ADR **in the same PR**.
6. Self-review the diff against the checklist: determinism leaks? new deps
   justified? error spans? `--json` stable?
7. PR ≤ **~600 lines** with a **Proof Delta** section: *"what is now proven that
   wasn't before,"* and which §8 gates cover it.

**Never claim a proof obligation is met without showing the passing command
output.** Not "tests pass" — the output.

### 4.1 Human review points (§9.3) — Ayush's, non-delegable

Every **ADR** · every **public API change** · every **golden-file change** ·
every **new dependency** · every **README or launch artifact**.

### 4.2 Ayush's decisions alone (§16) — D1–D6

**Stop and ask. Do not pick, do not recommend-then-proceed, do not "assume for
now."**

| | Decision |
|---|---|
| **D1** | The umbrella brand name and the multicall binary name `<b>` |
| **D2** | Tauri confirmation for the GUI stack |
| **D3** | Flagship confirmation after the Phase 0 validation read (konflux vs bigsheet) |
| **D4** | Final licence |
| **D5** | pdfsurgeon vs veritas ordering in Phase 4 |
| **D6** | Docs information architecture |

Also Ayush's alone: **all visual design** (§3.4, §16) and **all public
communication**. Never post, publish, tweet, comment or email anything. Drafts
go to Ayush; Ayush posts.

### 4.3 ADR template (§9.4)

```
# ADR-NNN: <decision>
Date · Status (proposed/accepted/superseded by NNN)
Context: what forced a choice (1 short para)
Options: A / B / C with one honest sentence each, incl. costs
Decision: what and WHY (the sentence Ayush says in an interview)
Consequences: what gets harder; what we're betting on
Proof impact: which invariants/gates this touches
```

**ADR-001 is reserved** for the `core-cst` representation choice (green/red tree
vs owned token tree), made after a 2-day spike at konflux M1. Do not use that
number for anything else.

---

## 5. Determinism hygiene (§9.5)

These are the recurring failure modes to preempt. Most are enforced by
`clippy.toml`; the enforcement is a safety net, not permission to stop thinking.

- **No `HashMap`/`HashSet` iteration in any output path.** `BTreeMap`/`BTreeSet`
  when order should be sorted; `IndexMap`/`IndexSet` when insertion order is the
  contract. Both hash types are banned workspace-wide by `clippy.toml`.
- **No wall-clock in outputs** unless explicitly flagged. Inject time at the
  boundary; never read it in a serialiser.
- **Stable sorts.** `sort_by`, never `sort_unstable_by`. Equal elements keep
  source order.
- **Fixed float formatting.** Never rely on the default `Display` for a value
  that reaches an output.
- **Seeded randomness only**, and the seed is logged.
- **Path handling identical across OS** — normalise separators in output.
- **Locale-independent formatting** — `to_ascii_lowercase`, never
  `to_lowercase`, for format tokens.

Verify with:

```bash
cargo xtask scaffold-report > a.json && cargo xtask scaffold-report > b.json && cargo xtask assert-equal a.json b.json
```

---

## 6. Appendix C — permanent bans

These are not preferences. They do not expire, and they are not overridden by a
deadline, a benchmark, or a reviewer's enthusiasm.

- **Load-bearing ML or probabilistic components** — anywhere, in any product.
- **Semantic / similarity caches** — an embedding-similarity hit that returns a
  subtly wrong answer is precisely the failure this brand exists to reject.
  Exact prefix match, or a miss.
- **Heuristic "suspiciousness" scoring** — anything that can cry wolf.
- **Telemetry / phone-home** — none, ever, in any tool.
- **Network by default** — offline unless an explicit `--online` flag exists and
  the README explains why.
- **Editing goldens or thresholds to pass CI.**
- **Cherry-picked or irreproducible benchmarks** — publish the cells where
  incumbents win; publish hardware, versions, commands, raw data, and a
  reproduction one-liner.
- **"Unescapable" / "unbreakable" security claims** — `cage` claims *"provable
  default-deny policy enforcement"* and nothing stronger.
- **ToS-violating integrations.**
- **Patent-encumbered codecs.**
- **Shipping the shell before three tools have adoption.**
- **Cutting proofs to save a deadline.** If a phase runs over, cut *scope* —
  formats, features. Never proofs. Never soundness.

---

## 7. Repo conventions

**Layout** (§2): `crates/` kernel · `tools/` products · `shell/` (README only
until Phase 6) · `corpora/` fetch scripts · `benches/` · `fuzz/` · `xtask/` ·
`adr/` · `.github/workflows/` the CI law.

**Every crate carries a `DESIGN.md`** with scope, invariants, and current
milestone (§9.1).

**Every tool obeys `core-cli`** (§3.4): `--json` (stable, schema-versioned),
`--check` (exit-code-only), deterministic output ordering, `NO_COLOR`, no
network, span-rich miette-style diagnostics.

**Dependencies** (§2): minimal and pinned. Every new dependency needs a one-line
justification in the PR and passes `cargo-deny`. `xtask` has zero third-party
dependencies on purpose — the tool that proves determinism should not itself be
a supply chain.

**Licence:** MIT OR Apache-2.0 (D4 finalises the file set).
**MSRV:** 1.90, CI-checked. Raising it is an ADR (ADR-002).
**Privacy is a feature:** no telemetry, no phone-home, offline by default,
forever. Stated in every README.

### Commands

```bash
cargo fmt --all -- --check
```

```bash
cargo clippy --workspace --all-targets --all-features
```

```bash
cargo test --workspace --all-features
```

```bash
cargo xtask corpus-verify
```

```bash
corpora/fetch.sh
```

```bash
cargo +nightly fuzz run roundtrip_identity -- -max_total_time=300
```

---

## 8. The authorship defense (§13)

The decisive 2026 risk is that a repo is discounted unless its author
demonstrably owns the architecture and can defend every decision live. That is
why the ADR corpus exists, why every non-obvious decision is written down with
its *why*, and why Ayush reviews every one.

Practical consequence: **write the ADR as the sentence Ayush says in an
interview.** Not a changelog entry — the reasoning, the options rejected, and
the cost accepted. If an ADR does not survive being read aloud to a Google
interviewer, rewrite it.

> *Interview law: the repo gets the interview; the whiteboard gets the offer.*
