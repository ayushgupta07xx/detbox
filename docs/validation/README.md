# Phase 0 validation posts — DRAFTS

**Nothing in this directory has been posted, and nothing in it may be posted by
anyone but Ayush.** All public communication is Ayush's, always (MASTER_PLAN
§16, and standing instruction). These are drafts for review and editing.

## What these are for

MASTER_PLAN §11, Phase 0: *"validation posts (r/devops, r/kubernetes, Ask HN):
the konflux one-liner + mockup."*

**Exit gate:** ≥50 upvotes **or** ≥20 "I hit this weekly" replies → konflux
confirmed as flagship. Weak signal → **pivot the flagship to bigsheet**, konflux
slides to Phase 4 (the kernel still gets built, via strukt). That call is
**Decision D3** and it is Ayush's alone.

## Files

| File | Venue | Status |
|---|---|---|
| [reddit-devops.md](reddit-devops.md) | r/devops | draft, unposted |
| [ask-hn.md](ask-hn.md) | Hacker News (Ask HN) | draft, unposted |
| — | r/kubernetes | **not drafted** — §11 names three venues; two were requested. Say the word. |

## What is deliberately missing

**The mockup.** §11 pairs the one-liner with a mockup. All visual design is
Ayush's (§3.4, §16), so both drafts mark where an image goes and describe what
it needs to show, rather than specifying how it looks.

## Rules these drafts follow

Because a validation post is the brand's first public sentence, and the brand is
calibration:

- **Nothing is built, and the posts say so.** No demo link, no repo link, no
  "coming soon" signup. Asking whether a problem is real is a different act from
  launching, and mixing them poisons the signal — people upvote a launch out of
  politeness and never tell you the problem does not exist.
- **No claim that is not already true.** No performance numbers, no
  auto-resolution rates, no "zero false conflicts" — those are P2/P3, unproven,
  and stating them now would be exactly the overclaiming this brand exists to
  reject.
- **Incumbents are named and credited.** Mergiraf, difftastic, `git
  merge-file`/diff3, yq and kdiff3 all exist and some of them are good. A post
  that pretends otherwise gets corrected in the comments, correctly, and the
  signal is lost.
- **One specific question**, so replies are data rather than applause.

## Reading the signal (§14 base-rate honesty)

The win condition is becoming the *default in a niche*, not virality. When
reading replies, weight them in this order:

1. *"I hit this weekly"* + a **concrete** example — the strongest signal there is.
2. *"I already use X for this"* — tells you the incumbent and the real bar.
3. *"Just don't put YAML in git"* — noise, but read for the constraint underneath.
4. Upvotes with no comments — the weakest signal. Threshold met is not the same
   as demand proven.

A reply naming a **specific painful merge** is worth more than ten upvotes.
