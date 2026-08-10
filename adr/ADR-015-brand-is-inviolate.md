# ADR-015: The umbrella brand is `inviolate`, the multicall binary is `invio`

**Date:** 2026-08-09 · **Status:** proposed

## Context

**D1** (§16) is the umbrella brand and the multicall binary name `<b>`. It has
been open since Phase 0 and ADR-007 froze every brand-named artifact behind it,
so nothing in the repository carries a name yet — the directory is `detbox`, a
working name, and the multicall binary does not exist.

§16's criteria: ≤2 syllables, pronounceable, free on crates.io + npm + GitHub +
a `.dev`/`.io` domain, no trademark collision, meaning-adjacent to
*proof / exact / lossless*.

## Options

The search ran in four registers, checked against live registries rather than
guessed. Two candidates were killed by reasoning rather than availability, and
those are the interesting ones.

- **A — `lossproof`.** Free everywhere. **Rejected:** it names one of §0's three
  pillars and only covers five of nine products. `cage` is a sandbox and
  `coverify` a determinism harness — neither has anything to do with loss. It
  also sits two letters from the existing product `lockproof`, which is
  confusing in speech, in search and in a URL.
- **B — `hallmark`.** The best *meaning* found anywhere: the assay office's
  stamp certifying purity, which is exactly what these products emit. crates.io
  free. **Rejected on trademark** — Hallmark Cards is a major and actively
  defended mark, and §16 requires no collision.
- **C — `unerring`.** Free on crates.io and npm, and it means "never wrong",
  which is §0 priority 1 word for word. **Rejected on our own marketing law:**
  Appendix C bans *"unescapable"/"unbreakable"* claims because no honest tool
  can make them. A brand called `unerring` makes exactly that claim before any
  product has opened its mouth.
- **D — `inviolate`.** Below.

## Decision

**`inviolate`**, with the multicall binary **`invio`**.

*Inviolate* means **intact; nothing was violated**. That is K1 and K2 stated in
English, and unlike A it stretches across the whole ecosystem rather than the
document half of it:

| Product | What stays inviolate |
|---|---|
| konflux · strukt · pdfsurgeon | the bytes you did not touch |
| bigsheet | the row count |
| veritas | whatever the receipt says survived |
| lockproof | the signature |
| coverify | a replayed run |
| **cage** | the host |

It is also a claim about **the artifact, not about us**, which is why it passes
where C failed. "Nothing was violated" is checkable; "we are never wrong" is a
boast.

### Availability, measured three ways

Domains were checked with RDAP, `whois` and DNS independently, because the first
pass was wrong twice — rdap.org's redirects were not followed, and a 404 from a
TLD with no RDAP service was misread as "available".

| | |
|---|---|
| crates.io, npm | free |
| `inviolate.dev` `.io` `.app` `.sh` `.rs` | unregistered; RDAP, whois and DNS agree |
| `inviolate.com` | registered, parked on a reseller |
| GitHub `inviolate` | taken |
| GitHub `inviolate-dev` / `-sh` / `-tools` | free |

The bare GitHub name being taken is **not** a blocker, and §16 over-constrained
here: Astral's own org is `astral-sh` because `astral` was gone. crates.io is
the namespace that actually binds, because that is where we publish.

### The binary

`inv` and `vio` are taken on crates.io, and `inv` is a landmine regardless —
Python's `invoke` installs a binary by that name, so a user with both would get
whichever came first on `PATH`. A merge driver that is sometimes a Python task
runner is not a tool anyone can trust. **`invio`** is free and unambiguous.

## Consequences

- **ADR-007 is discharged, not superseded.** It said no brand-named artifact
  exists until D1; D1 is now made, so the freeze lifts on its own terms.
- **The binary crate is still not created here.** §11 puts the install one-liner
  at konflux M4, and a crate that exists only to hold a name is the placeholder
  ADR-007 was written to prevent. It lands when something needs to be installed.
- **Two actions remain Ayush's**, and neither is code: reserving the GitHub org
  and the domain, and a trademark search on `inviolate` in the software classes.
  That last one is the only §16 criterion still unverified, and publishing under
  the name before it is done is the risk.
- **§16 asks for ≤2 syllables and this is four.** Relaxed deliberately: every
  free two-syllable candidate was either meaning-poor (`exactkit`), clunky
  (`proofkern`) or colliding (`lossproof`). Recorded so it reads as a decision
  rather than drift.

## Proof impact

None. No invariant, gate or threshold moves. This is a naming decision, and it
is in `adr/` because §13 wants the why-trail — including for the two candidates
that were better on paper and were rejected anyway.
