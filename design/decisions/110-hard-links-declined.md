---
status: DECIDED
raised: 2026-08-23
decided: 2026-08-23
ratified_by: calef
---

# 110. Hard links are declined, for want of a customer

calef, 2026-08-23, on milestone 47 (`cd`, `pwd`, `ls`, `mkdir`, `rm`, paths, and environment)'s own named fork: *"The backup server is not
a top priority. It is just one goal. There is no customer for hard links so let's defer."*

## The question

Milestone 47's `ln` section names hard links as mechanically easy (RedoxFS already tracks link
counts, and §48's deferred-delete already depends on the same mechanism) but structurally costly:
"hard links make it not a tree... every piece of subtree reasoning written so far quietly assumes a
DAG cannot happen." What to settle before building: offer hard links at all, given that consequence.

## The decision

**Declined, for now.** No consumer needs them:

- The atomic-replace idiom people usually reach for hard links to get is already covered. The
  standard pattern in real Unix practice is write-to-temp-then-`rename()`, not a hard-link trick, and
  `mv`/`RENAME` already exist here.
- The one place hard links are load-bearing in a real system near this project's own scope doesn't
  need them either. Time Machine's incremental-backup deduplication happens inside the sparse
  bundle's own filesystem, which the Mac itself manages; nife serves band-file content and never
  sees or needs to implement that. (Named as one data point, not the deciding one: the backup server
  is one goal among several, not the reason this defers.)
- No other consumer has asked for cross-subtree aliasing.

**What offering them would have cost, which is why "no customer" settles it rather than "build
everything, decide never":** not the implementation (small), but the audit. `fs_subtree_caretaker`'s
whole confinement argument and everything built on "what you weren't granted, you can't reach"
assumes a file lives in exactly one subtree. Hard links don't make that unsafe (a capability still
only reaches what it names), but they change what an operation *means*: "I deleted the subtree" and
"that content is gone" stop being the same claim. Auditing every place that assumption is load-bearing
is real work, worth spending only once something actually needs the feature.

## Prior art and precedent, both pointing the same way

Plan 9 doesn't lean on hard links for "give this content another name": namespace composition
(`bind`) covers it, and this tree already took Plan 9's answer once for the adjacent question (§50).
This tree's own instinct elsewhere is consistent: §16 keeps region ownership a tree, with a
DAG-shaped revocation layer explicitly deferred as "purely additive... if ever wanted," not built
speculatively.

## Reversibility

Nothing is built yet; nothing depends on this. Declining now costs nothing and forecloses nothing.
If a real need for cross-subtree aliasing appears later, it can be added then, likely more cheaply
than now, before more subtree-confinement code has been written against the tree assumption.

## What this does not decide

Directories were never on the table (Unix already forbids hard-linked directories to prevent cycles,
and the argument is stronger here since a cycle would also break `rm -r`'s bottom-up termination).
Symlinks are a separate question, already settled 2026-07-31 by DECISIONS §50 (`bind`, not stored
paths).

## What it unblocks

Milestone 47's `ln` section closes on the hard-links half; only symlink-adjacent naming (already
settled) and the sequencing question ("what a stored path containing `..` means", per §50's own
scope) remain there, if anything.
