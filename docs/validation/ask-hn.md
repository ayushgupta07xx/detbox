# DRAFT — Ask HN

**Status: unposted. Ayush posts, or does not.**

---

**Title:**

> Ask HN: Is structural 3-way merge for config files a problem you still have?

*Alternates:*
- *"Ask HN: Why is there still no format-preserving merge tool for YAML?"*
- *"Ask HN: Do you hit line-based merge conflicts on config that aren't real conflicts?"*

*HN titles: no clickbait, no em-dash flourish, no exclamation. The first one is
the safest — it asks a question and names the domain.*

---

**Body:**

Git merges structured config as lines. A Kubernetes manifest, a Helm
`values.yaml`, a Terraform file — all of them are trees, and git treats them as
text.

The failure modes I keep hitting:

- **False conflicts.** Two branches add different keys to the same map, or
  different env vars to the same container. No semantic disagreement; git
  conflicts because the lines are adjacent.
- **Silent wrong merges.** A YAML sequence is just indented lines to git, so a
  clean-looking merge can produce a list with two entries named the same thing.
  This is the one that scares me — it doesn't fail, it just ships.
- **Reformatting as collateral damage.** Every tool I've found that understands
  the structure also rewrites the file: comments dropped, key order normalised,
  quoting style changed, anchors expanded. The result is a diff nobody can
  review, which means nobody does.

The third point is, I think, why the gap exists. Structural merge is a solved
problem in the literature — Chawathe-style tree matching is decades old.
**Format-preserving serialization of the merged result is the hard part**, and
it is unglamorous: you need a concrete syntax tree that owns every byte of the
input, including whitespace, comment placement, anchors, and the trailing
garbage after the document end marker, and you need the merge to write back
through it without touching a byte it didn't have to.

What I'm considering: a git merge driver for YAML and JSON that merges the tree
and preserves everything it didn't change, byte-for-byte. Two properties I'd
want to hold it to, because otherwise it's just another tool that's usually
right:

1. **Round-trip is byte-identical.** `serialize(parse(x)) == x` for every file
   in a real-world corpus and every input a fuzzer can find. If it can't
   round-trip a construct, it preserves it verbatim rather than normalising it.
2. **Conflict on uncertainty.** If a resolution can't be shown safe, it emits a
   conflict. A merge tool that guesses well 98% of the time is a tool you can't
   trust at all, because you don't know which 2% you got.

I haven't built it. There's no repo, no demo, no waitlist — I'd rather find out
the problem isn't real before I spend three months on it than after.

Prior art I know of, and I don't want to pretend otherwise: **Mergiraf** does
structural merge (primarily for programming languages), **difftastic** does
structural diff and has deliberately stayed out of merging, **GumTree** is the
academic tree-diff line of work, and `git merge-file`/diff3 is what most of us
actually use. If one of these already covers this for you, please say so — that
is the most valuable answer in the thread.

Questions:

- Do you still hit line-based merge conflicts on config that aren't real
  conflicts, and how often?
- Has a bad config merge ever reached production? What was the failure?
- Would you actually install a custom `git merge-driver`, or is that friction
  too high for a team?
- Is byte-identical preservation a hard requirement, or would you accept a
  reformat in exchange for a correct merge?

---

## Posting notes — for Ayush, not part of the post

- **Timing (§12):** US weekday morning, Tue–Thu. Ask HN posts do best when the
  question is genuinely open, which this one is.
- **No links.** Not to a repo, not to a mockup, not to a profile. An Ask HN with
  a link reads as a launch and gets flagged as one. The mockup is for the reddit
  post; this one stands on the prose. If someone asks to see it, reply in-thread
  with a description or offer email.
- **The maker comment (§12) is for the *launch*, not for this.** Do not
  pre-announce. This post's only job is to find out whether the problem is real.
- **Answer every substantive comment within the first two hours** — that is when
  HN threads live or die.
- **Expect and welcome the sceptical replies.** The three most likely:
  - *"Just don't merge generated YAML / use a single source of truth."* Fair.
    Ask what they do when two people edit the source of truth.
  - *"Mergiraf already does this."* Ask specifically about YAML, comment
    preservation, and K8s list identity. Their answer is the moat question.
  - *"Semantic merge is undecidable in general."* Agree, immediately and
    without hedging — that is exactly why the second property is
    conflict-on-uncertainty rather than clever resolution. This is the comment
    to engage hardest with; it is also the best material for the eventual
    deep-dive post.
- **Signal threshold (§11):** ≥50 points **or** ≥20 substantive "I hit this"
  replies → konflux confirmed. Below that → pivot flagship to bigsheet (D3).
- **Note on calibration:** if the thread converges on "this is real but nobody
  would install a merge driver," that is not a no — it is a distribution
  problem, and it argues for leading with `--check` CI mode and the GitHub
  Action rather than the driver. Record that; it changes M4's ordering.
