# Open-model lane trials of 2026-09-22, recovered from branches

Name: provisional. Written 2026-10-02 from nine unmerged local branches, deleted the same day, so the
evidence is the commits and not anyone's memory of them. The cost and the verdict of five of the
runs are in [the delegation ledger](../delegation/ledger.tsv); the rest were never recorded there.

## What this is and is not

These are single runs of tasks the maintainer needed done that day, handed to rented models through
`helpers/open-lane.sh`. There was no protocol, no blinding and no second run per cell. The branch
names carry the model for four of them (`open-lane-glm5`, `open-lane-qwenmax`, `open-lane-kimi`,
`open-lane-mistral`). For `bench/open-model-trial-1`, `lane/qwen-real` and `lane/ab-pref` the branch
does not say which model wrote it, and this note does not guess. **No cost was recoverable from any
branch**, so there is no price column. Whether each branch passed `script/lint` was not re-run.

## Task 1: promote sixteen proposals to milestones 524 to 539, then repair the collisions

All branches share the same two base commits (the promotion and its claim). Two follow-up tasks were
given on top.

| Branch | Task | What it did | What a reader would object to |
|---|---|---|---|
| `bench/open-lane-kimi` | renumber the colliding 524 to 540 | moved the file, fixed the H1, rewrote the status paragraph to say why | replaced the sentence "Number provisional until the merge queue lands it" (already a ledger defect row) |
| `bench/open-lane-mistral` | the same | moved the file and fixed the H1, one line changed | recorded nothing about the renumber in the block, so a reader of the file cannot tell it ever had another number |
| `lane/qwen-real` | renumber 525 to 555 and 527 to 556 | moved both files, fixed H1s, appended a renumbering sentence to the status paragraph | none found; kept the provisional-number sentence |
| `lane/ab-pref` | the same | the same result, a slightly shorter record of the move | none found; kept the provisional-number sentence |

`lane/qwen-real` and `lane/ab-pref` differ only in wording, which is the only A/B evidence in this
set: two attempts at one mechanical task that both came out clean.

## Task 2: gloss the two citations the ratchet flagged

The gate wants a gloss grounded in the cited block's title, placed after the number. Two citations
needed it:

- milestone 534 (the soft-float targets could now be flipped), in the follow-on of milestone 447
  (vector registers are a thread's own).
- milestone 532 (the mutation census should write its own row), in the follow-on of milestone 518
  (a census that cannot be attributed).

Five models did it; all five edited the same two follow-on bullets.

| Branch | Result |
|---|---|
| `bench/open-lane-qwenmax` | closest to what landed on `main`: a short title fragment in parentheses, the sentence reworded to fit, no duplicated marker text |
| `bench/open-lane-glm5` | grounded, but rewrapped two unrelated sentences in the 447 bullet and left the marker text doubled, "Milestone 534. milestone 534" |
| `bench/open-lane-kimi` | its message states the rule it was working to (the marker carries the number and nothing else); the diff itself was not compared line by line |
| `bench/open-lane-mistral` | grounded, but pasted the title with its leading capital mid-sentence and dropped the bullet's own lead-in ("The target flip") to make room |
| `bench/open-model-trial-1` | grounded, but left the old lead-ins ("The target flip", "First on that same file's list") after the gloss with a stray capital, and the gloss is longer than the title fragment needs |

The pattern across all five is that the gate passes a doubled "Milestone 534. milestone 534" and a
mid-sentence capital. Those are readable defects that no gate here can see, the same class as the
ledger's two kimi rows. None of the five wrote a clause that was wrong; the differences are
editorial.

## Task 3: the install offer should describe what is on the disk

`lane/551-hybrid` is two commits against what is now milestone 570 (the install offer should say what
is already on the disk), written when it was numbered 551. The first commit has no model trailer. The
second names Claude Haiku 4.5 and splits the string "(unpartitioned or unreadable)", which the
ledger already records as the `qwen3-coder` redo row. A disk safe to wipe and a disk the surveyor
could not read are opposite facts and must not share a description. So this branch is that redo being
repaired, not a third attempt.

What it still got wrong, read from the diff against `main`:

- It copies the surveyor's role, slot and flag constants into `install_service.rs` and spawns a second
  surveyor, where the milestone's block says the file already surveys the disk and the work is one
  spawn and one sentence. The copies can drift from `components/src/disk_surveyor.rs` silently.
- Its tests sit in a new module compiled only for `x86_64` tests. Nothing here shows they were run.
- It ends with "(survey timed out or failed)" and "(no surveyor available)" as descriptions, which
  read as facts about the disk to the person typing the confirmation.

What is worth keeping is the four-way description (already has nife, has partitions, unpartitioned,
cannot read the disk) keyed on the surveyor's `F_NIFE`, `F_PRIMARY` and `F_SIZE` flags. A lane taking
milestone 570 should start from that table and reuse the existing spawn.

## BUGS

- **No costs.** The gateway's per-request figures were not kept with the branches. A re-run would
  have to measure them.
- **The verdicts are one reader's**, taken from the diffs and not from running the gates again.
