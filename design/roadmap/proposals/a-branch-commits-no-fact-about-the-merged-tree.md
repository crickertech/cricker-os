---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# A branch commits no fact about the merged tree

Raised by the lane that wrote the
[2026-10-03 queue correction](../../../notes/coes/2026-10-03-the-queue-judged-one-pull-request-at-a-time.md),
failure 3. On that day 47 of 142 group builds failed. 33 of them failed on a lint check that
judges the merged tree, and every branch involved had passed it on its own. The textual conflicts
were cheaper: 13 hand merges and 2 `merge_conflict` ejections.

## The change, one rule applied twice

1. **A block promotes only a proposal its branch held** (rung 2). `script/roadmap --check` fails
   when a numbered block that this branch adds says `promoted_from: <slug>` and the branch's history
   never held that proposal (one `git log --diff-filter=DR` over the proposals directory, 0.08 s).
   The message tells the lane to wait for the proposal to merge before it arms. This is the README's own rule ("an integrator ... `git mv`s it up a
   directory"), enforced. It would have caught milestones 720 to 724 at their first push, ahead of
   31 failed groups (632 group minutes) and the 96 minutes from 19:18 with one merge in them.
2. **The Kani harness count leaves the prose** (rung 1). `notes/unsafe-obligations.md` and
   `notes/verification.md` type 214, 215 or 216 by hand behind `<!--count:kani-harnesses-->`. Every
   merge that adds a harness conflicts there: 3 hand merges on 2026-10-03 and 31 count-fix commits
   since 09-01. The project-wide count already lives in `notes/project-metrics.md`, generated
   weekly. CLAUDE.md puts counts there, and `notes/verification.md`'s BUGS has named the
   regeneration as the fix since 2026-10-02.

## The fork, answered

1. **Alternatives.** *Delete promoted proposals at merge with a bot*: the group lint still fails
   before the bot runs, so the check would have to be weakened in groups. *A union merge driver*
   (`merge=union`) for append-only files: on the day's three append conflicts it gets one right
   (#1547's risk-7 entries) and one silently wrong (#1529, where it would bring back two resolved
   BUGS bullets). *One file per dated entry*: one conflict a day does not pay for restructuring every
   risk page. *A conflict check before queueing*: only 2 of 122 entries reached the queue conflicting,
   and the detector in `a-queue-eviction-goes-to-a-maintainer-session.md` covers `DIRTY`. The order
   of the queue is the queue's own job.
2. **In the tree.** Milestone 125 (a number in the prose is a claim) gates counts. The pace rule in
   CLAUDE.md says a number should change at the pace of a decision, and this one changes with every
   merge.
3. **Prior art.** None read.
4. **Premise.** Checked. #1520's own CI was green at the head whose group failed on the prose
   ratchet, and the four "remove the promoted proposal" commits came 32 minutes after #1513 merged.
5. **Cost.** About 15 lines in `script/roadmap` and a fixture. The check covers only blocks the
   branch adds, because 5 of the 216 existing blocks name proposals that history does not hold
   (paths that moved). Plus three sentences reworded.
6. **Reversible.** Yes for both.
7. **Same cost?** Yes.

## BUGS

- The prose ratchet's per-file baseline is a fact about the merged tree too (2 failed groups, 1
  trim commit). It is left alone: a budget the union breaks is something the group should catch.
- The check needs the branch's full history wherever lint runs, and CI's `clippy` checkout depth has
  not been confirmed.
