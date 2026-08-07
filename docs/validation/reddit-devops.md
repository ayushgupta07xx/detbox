# DRAFT — r/devops

**Status: unposted. Ayush posts, or does not.**

---

**Title:**

> How often does git mangle your YAML during a merge — and what do you actually do about it?

*Alternates, if the first reads too leading:*
- *"Does anyone else lose an hour a week to YAML merge conflicts?"*
- *"Structural merge for K8s/Helm YAML: does this problem still exist for you, or have you solved it?"*

---

**Body:**

I keep hitting the same wall and I want to know whether it's just me before I
spend three months on it.

Git merges YAML as lines. It has no idea that a Kubernetes manifest is a tree.
So:

- Two people add a different env var to the same container. Different keys,
  no semantic conflict at all — git reports a conflict because the lines are
  adjacent.
- Someone reorders a `values.yaml` block. Now every downstream branch conflicts
  on content that didn't change.
- The merge "succeeds" and silently produces a list with a duplicated container
  name, because a YAML sequence is just indented lines as far as git is
  concerned.
- I reach for a tool to fix it up, and it reformats the whole file — comments
  gone, key order normalised, quoting style changed. The diff is now 400 lines
  and unreviewable.

That last one is the part that gets me. Every tool I've tried that *understands*
YAML also *rewrites* it.

**The thing I'm considering building:** a structural 3-way merge driver for
config — YAML and JSON first — that merges the tree rather than the lines, and
writes the result back **byte-identically except where the merge actually
changed something.** Comments, key order, anchors, quoting style, line endings
all preserved. Plugs in as a `git merge-driver`, so it works where you already
work.

The design constraint I care about most: **when it isn't certain, it must
conflict.** A merge tool that silently produces a plausible-but-wrong manifest
is worse than no tool. I'd rather it hand me a conflict than guess.

<!-- [AYUSH — MOCKUP GOES HERE]
     One image. What it needs to show, in one glance:
       left  : the same merge under `git merge-file` — conflict markers cutting
               through a container spec, comments displaced
       right : the same merge resolved structurally — both env vars present,
               comments and key order untouched, unchanged bytes visibly unchanged
     Visual design is yours. If you'd rather post without it, the text stands
     alone — but §11 pairs the one-liner with a mockup and the image is what
     makes the ask concrete for a scrolling reader.
-->

**Nothing is built yet.** No repo, no beta, no signup — I'm not launching
anything, I'm trying to find out if the problem is as common as it feels.

I know about the prior art and some of it is genuinely good: **Mergiraf** does
structural merge (mostly aimed at programming languages), **difftastic** does
structural diff and deliberately doesn't merge, **kdiff3**/**Meld** help you
resolve by hand, and **yq** edits YAML but normalises formatting in the process.
If one of these already solves this for you, that's exactly what I want to hear —
please say which and how.

**So, the actual questions:**

1. How often do you hit a config merge conflict that is *purely* a line-based
   artifact — no real semantic disagreement?
2. When it happens, what do you do? (Take-ours, hand-merge, re-generate from a
   template, revert and redo, something smarter?)
3. Has a bad config merge ever reached a cluster? What did it look like?
4. Is byte-identical preservation of untouched content a hard requirement for
   you, or would you accept a reformat if the merge were correct?

If you hit this weekly, saying so plainly is the most useful reply you can leave.
If you don't hit it at all — that's just as useful, and I'd rather find out now.

---

## Posting notes — for Ayush, not part of the post

- **Timing (§12):** US weekday morning. Don't concentrate this with anything
  else; this is a validation post, not a launch, and it should not share a day
  with a launch spike.
- **Subreddit rules:** r/devops removes self-promotion. This post has no link,
  no repo and no signup precisely so it reads as a question. Keep it that way —
  do not add a GitHub link in the body or in a comment, even if asked. Offer to
  DM instead.
- **Flair:** "Discussion" if required.
- **Comment strategy:** reply to every concrete example with a follow-up
  question. A named repo, a named chart, a real incident — that is the data.
  Do not defend the idea; you are collecting evidence, not selling.
- **The reply you most want:** someone naming a specific merge that went wrong.
  Ask for the file if they'll share it — that is a golden case (§3.3), and a
  golden case mined from a real incident is worth more than any synthetic one.
- **Signal threshold (§11):** ≥50 upvotes **or** ≥20 "I hit this weekly"
  replies → konflux confirmed. Below that → pivot flagship to bigsheet (D3).
- **If the top comment is "just use Mergiraf":** that is a real answer, not a
  failure. Ask whether they use it on YAML specifically and whether it preserves
  their comments. The answer determines whether the moat is formats + K8s
  semantics, or whether there is no moat.
