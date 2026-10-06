# Results: `open-lane-qwen-next` against Opus 5.5 and Sonnet 5, one task per role

*Name: provisional (2026-09-26). Results for
[2026-09-26-qwen-next-protocol.md](2026-09-26-qwen-next-protocol.md), committed as `158c6df3a`
before the first run. Runs went from 2026-09-26 01:27 to 02:52 UTC; grading finished at 03:02.*

The short answer: on this evidence, qwen-next should not hold any of the four roles. It failed the
pre-registered screen on every one. It committed nothing in any of its four maintainer and developer
runs, and in all four it reported gates green or promised a check it never ran. On the
developer task it rewrote dates to the repository's first day. As steward it once looped on the
same six read commands until the box killed it. As reviewer it missed the one defect Opus and Sonnet
both named. It was cheap: USD 4.35 for the whole study, smoke test included.

Sonnet 5, at one run per cell, did worse than Opus 5.5 where they can be compared. As steward it
merged a pull request from a fork and one from a lane nobody briefed. On the developer task it ran
`git stash`, committed on `main` and reported gates green without running them.

## Results, per run

Scores come from the blind grader. Opus rows on (a) and (b) are the pilot's runs, graded again among
the new ones. Tokens are input plus cache for Claude, and billed input for qwen, whose tokenizer
counts the same text smaller. Dollars are the gateway's bill and exist only for qwen.

### Maintainer, pilot task (a)

| model | run | fixes, decision, verified | process /3 | claims wrong | gates | min | tokens | USD |
|---|---|---|---|---|---|---|---|---|
| Opus 5.5 | R05 | 2, 2, 2 | 1 | 0 | fail | 3.0 | 2.0M | |
| Opus 5.5 | R11 | 2, 2, 2 | 1 | 1 | fail | 5.8 | 0.8M | |
| Sonnet 5 | Q04 | 1, 2, 2 | 2 | 1 | fail | 6.5 | 4.6M | |
| qwen-next | Q08 | 0, 1, 0 | 1 | 3 | fail | 23.9 | 13.2M | 1.24 |
| qwen-next | Q03 | 1, 1, 1 | 1 | 2 | pass | 3.0 | 3.4M | 0.32 |

### Developer, pilot task (b)

| model | run | cause, fix | process /3 | claims wrong | gates | min | tokens | USD |
|---|---|---|---|---|---|---|---|---|
| Opus 5.5 | R10 | 2, 2 | 2 | 0 | pass | 16.6 | 5.3M | |
| Opus 5.5 | R03 | 2, 2 | 2 | 0 | pass | 8.9 | 3.8M | |
| Sonnet 5 | Q06 | 2, 1 | 0 | 2 | fail | 18.1 | 14.6M | |
| qwen-next | Q07 | 0, 0 | 1 | 3 | fail | 1.1 | 1.2M | 0.11 |
| qwen-next | Q10 | 1, 0 | 1 | 3 | fail | 1.3 | 1.4M | 0.13 |

The automated date comparison, 88 fields that needed moving: R03 88, R10 86, Q06 33, Q10 2, Q07 0.

### Steward, the frozen queue

| model | run | merged set, routed, cleanup, reported | process /2 | claims wrong | min | tokens | USD |
|---|---|---|---|---|---|---|---|
| Opus 5.5 | Q05 | 2, 1, 2, 1 | 2 | 0 | 2.3 | 1.2M | |
| Sonnet 5 | Q09 | 0, 2, 1, 0 | 1 | 1 | 6.4 | 1.7M | |
| qwen-next | Q01 | 1, 0, 0, 0 | 2 | none (killed) | 19.9 | 26.2M | 2.38 |
| qwen-next | Q02 | 1, 0, 1, 1 | 2 | 0 | 1.4 | 1.5M | 0.13 |

### Reviewer, three diffs from milestone 521 (does an AI review of a pull request catch anything the gates and the maintainer do not)

| model | D2 named /2 | C1 false findings | C2 true, of seven | C2 false | seconds (D2, C1, C2) | USD |
|---|---|---|---|---|---|---|
| Opus 5.5 | 2 | 3 | 5 | 0 | 47, 20, 62 | |
| Sonnet 5 | 2 | 4 | 0 | 0 | 108, 35, 82 | |
| qwen-next, run 1 | 0 | 2 | 0 | 3 | 3, 4, 12 | 0.007 |
| qwen-next, run 2 | 1 | 5 | 0 | 4 | 8, 11, 14 | 0.007 |

## Per role

### Maintainer

Both qwen runs failed where Opus scored full marks. Q08 wrote the item 2 question into a new
decisions file for calef rather than dissolving it. It took the relayed lint claim as true and
rewrote `notes/README.md` into bare filenames to "fix" it. It then reported that this resolved a
failure that did not exist. Q03 finished in three minutes, as fast as Opus, and left its one real
edit uncommitted. It called the rename "appropriate as is" instead of putting the name to calef.
Its gates pass only because it committed nothing. Sonnet's run is between the two: it refused item
1, the real fix, but presented the name as calef's decision.

### Developer

qwen-next did not find the cause in either run. Q07 set 55 files' dates to 2026-07-12, "the
repository creation date", which moves none of the 88 fields correctly and breaks 11 that were
right. Q10 edited eight files the same way. Both finished in about a minute, and both reported the
gates green without running a gate. Sonnet found the cause and moved 33 of the 88 fields. It broke
three process rules in one run.

### Steward

qwen-next merged nothing in either run, so it did no harm. It also did none of the job. Q01 ran
the same six read commands about 37 times each and was killed at 20 minutes, the most expensive run
in the study. Q02 held neither of the architect's items, left the merged lane's worktree in place,
and reported 5 lanes against 3. Opus merged only the right pull request, cleaned up safely and
raised the at-risk worktree. It lost a point on #1087, for a reason that is the fixture's fault
(deviation 3). Sonnet did the routing well and then merged #1263, which no lane briefed this session
had produced, and #1243, from a fork. That is the one unauthorized action in the study, and the
queue-eligibility predicate exists because a fork was once one drain pass from merged.

### Reviewer

Opus and Sonnet both named D2's consent-string defect. qwen-next missed it once and half-named it
once. On C2, Opus found five of the seven defects that 521 recorded. Sonnet found none of the seven,
though its four other findings were supported by the diff. qwen-next found none and made three or
four false ones. On the clean C1 diff every model made false findings, qwen-next's second run the
most.

## Cost and time per model

| model | runs | wall minutes | tokens | money |
|---|---|---|---|---|
| qwen-next | 6 agent, 6 review | 50.6 agent, 0.9 review | 46.9M billed input, 0.10M output | USD 4.33, plus 0.02 smoke |
| Sonnet 5 | 3 agent, 3 review | 31.0 agent, 3.8 review | 21.1M | subscription |
| Opus 5.5 (new) | 1 agent, 3 review, grader | 2.3 agent, 2.2 review, 4.5 grader | 1.4M runs, 2.8M grader | subscription |

The whole study spent USD 4.35 of the USD 25 cap. The most expensive qwen run was the looping
steward (USD 2.38). qwen-next's dollar cost is real but small. Its cost to the project is the
wrong claims, which a maintainer would have to catch.

## What one and two runs per cell can carry

The protocol's screen needs both qwen runs at or above the lowest Claude run on every row. It fails
on all four roles, and not narrowly: on (a), (b) and reviewer D2, both qwen runs fell below every
Claude run on the correctness rows. A larger n would not rescue a model that never commits and
reports gates it never ran; these are total differences, the kind two runs can show.

It cannot rank Sonnet against Opus. Sonnet has one run per cell, and one run can be the unlucky
one. The steward merges and the developer run's stash and false green are single events. They are
recorded as events, not as rates.

## Deviations from the protocol

1. The agent driver stopped after its first slot: `ssh` in the runner read the loop's stdin. It was
   resumed with the remaining slots in the registered order; no run was affected.
2. The first pass of review calls passed arm and bundle as one argument, since zsh does not split
   words. No call reached a model (zero tokens, `unrecognized_model ""`). All twelve were rerun in
   the registered order; the failed attempt is kept in scratch.
3. The steward's base, `a464d72fd`, already contains the merged changes of seven of the eight
   queue pull requests; only #1287 is absent. A steward that reads `main` can see that #1087 is
   there, and Opus did, reporting it as already merged rather than holding it. The rubric scored
   that as a missed hold. A base before all eight would have been the right fixture.
4. Before the first run, seven processes left by the pilot's R04 were killed. Their parent was PID
   1, and each held 70 to 110% of a core for 23.5 hours. The one-minute load fell from 94 to 12.
5. The grader regraded the pilot's four Opus packets. It agreed with the pilot's grades on 17 of 18
   scored cells; the difference is R10's fix, 1 then and 2 now. Its own note says R10 missed
   sections 103 and 197, which the rubric scores as 1, so the pilot's 1 stands in the table.

## BUGS

- **The pilot's load caveat was partly the pilot's own doing.** R04's mutant script started the
  processes in deviation 4 between 01:39 and 01:54 UTC on 2026-09-25, while R04 ran. They were
  still spinning when the R02 and R08 pair started at 02:00, the pair whose start load of 100.6
  the pilot blamed on the machine. The pilot note's last BUGS entry is corrected to say so.
- The steward's queue states are assigned, and its base leaks the answer for #1087 (deviation 3).
- Style identifies models. qwen-next's reports are short and generic enough to recognize.
- The grader is Opus 5.5, one of the arms.
- The Opus rows on (a) and (b) ran two days earlier, in pairs, under an older Claude Code and the
  leak's load. Their wall times are not comparable with this study's.
