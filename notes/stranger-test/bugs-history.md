# Stranger test BUGS history

*An appendix to [the stranger test](../stranger-test.md), which holds the current state. This page
is the record for every BUGS entry as it stood on 2026-09-24, with the history of each.*

## The rubric was reachable by grep

Found by [run 1](run-1.md) (2026-08-14). Resolved for run 2 (2026-08-16) as far as the answers go.
The fact of the instrument still leaks, and cannot stop while the instrument is in-tree.

Run 1's stranger found this note while researching an ordinary question and read the "pass means"
column of [the rubric](../stranger-test.md#the-rubric-written-2026-08-14-before-the-first-run). That
column leaks partial answers for at least (a), (c), (e), (g) and (h). The stranger disclosed this
unprompted, and that is the only reason the contamination is known rather than silently baked into a
score.

Run 2 withheld the answer key rather than the whole subject, per the rule in [run 2](run-2.md). The
note and its index entry were removed from the tree each stranger got, and nothing else. It worked
as far as it claims and no further. Run 2 met three references to run 1 in ordinary reading, had
`design/roadmap/117-newcomer-onboarding.md` returned by its own grep, and knew the project
instruments onboarding. It simply never opened the block. Only the answers are hidden.

Run 4 is the fourth confirmation and the loudest. It knew inside half an hour, from a table
`notes/adding-a-program.md` grew *because* runs 2 and 3 walked it. That mechanism is its own entry
below: the instrument writing its own history into the page it sends the next stranger to.

## AGENTS.md reached the stranger from the maintainer's checkout

Found by [run 2](run-2.md) (2026-08-16). Resolved for run 3 (2026-08-18), verified rather than
asserted.

The instrument's isolation is the harness's to give, and run 2's harness did not give it. Run 2's
strangers were subagents of a maintainer session whose working directory is the repository. So
`AGENTS.md` arrived in the stranger's context at turn zero, from a checkout it had been told not to
read. It used that document throughout and never opened the copy in its own tree.

This was worse than the rubric leak it replaced. The rubric leaks answers to eight questions. This
leaked the single document the whole reading-order finding is about, before the stranger made any
choice at all. It is the reason run 2 reports no B1 and no M3, M5, M6 or M8. Run 3 had to be a
process that could not see this repository except through the tree it was handed: a container, or a
session whose working directory is the clone.

Run 3's stranger was a separate `claude` process run with `--safe-mode` from the clone's parent,
since instructions load from ancestors and never from descendants. A throwaway probe and the
stranger itself both confirmed that no project-instructions file was in context ([run 3](run-3.md)).
B1 and all eight mental-model rows are scored for run 3. The mechanism costs one line of shell and
should be the default from here.

## The build half cannot be measured from a warm machine

Open. Run 2 (2026-08-16) came closest to a cold machine.

Every contributor's machine is warm. The first run should be from a container with nothing
installed, or the B-rows measure nothing. Run 2 still fell short. The maintainer's own `cargo
--version` inside the repository had installed the pinned nightly, and an attempt that died part way
had already installed both QEMUs. It found the Linux setup blocker anyway, because that one is a
hard failure rather than a saved step.

## A journalled stranger is watching itself

Found by run 2 (2026-08-16). Open.

Run 2 was asked to log its questions and its had-to-work-it-out items as it hit them. That log is
why anything survived its predecessor's death. It also changes the instrument: run 1's stranger
produced friction, and run 2's produced friction it knew was the deliverable. The log is worth more
than the loss, but the two runs are not measurements of quite the same thing.

## The harness's logs were in the stranger's working directory

Found by [run 3](run-3.md) (2026-08-18). Resolved for run 4 (2026-08-18).

Run 3's log files were named `stranger3-stream.jsonl` and `stranger3-stderr.log`. They sat in the
clone's parent, which was the stranger's working directory. So its first tool call, `ls -la`, told
it that it was stranger 3 before it had read a byte of the project. It disclosed this first when
asked, and named the consequence. It added a program rather than only describing one *because* it
knew it was the third walker of a page that asks its walkers to correct it. In its words, its
deliverables are *"long and rhetorically polished, and some of that is performance for a reader I
knew was watching"*.

This is the previous entry's defect in new clothes, introduced by the person closing that one while
closing it. The pattern matters more than the instance: **the isolation keeps failing at the harness
rather than at the tree.** Runs 1 and 2 leaked the rubric and the constitution. Run 3 leaked the
fact of being measured, earlier and more loudly than the in-tree references do.

The run still counts. The stranger never opened the logs, never opened
`design/roadmap/117-newcomer-onboarding.md`, and never used the network, so its actual output
survives the leak. Discount the rhetoric in it, and read the decision to add a program as prompted
rather than spontaneous.

The fix for run 4 was one line: the logs go in a sibling directory, not in the parent. It worked
exactly as priced. No path element carried a run number, and the stranger's first `ls -la` returned
one entry, the clone. The harness told it nothing.

## The rubric was written by an agent that has worked in this tree

Open. Run 3 (2026-08-18) exercised the check on it, and the check worked.

This is the disqualification the instrument exists to avoid, one level up. The author knows which
answers the tree gives, so the questions may be shaped around what is answerable. The check is a
stranger falling down somewhere unasked-about, and [the
rubric](../stranger-test.md#the-rubric-written-2026-08-14-before-the-first-run) says to amend rather
than defend. In run 3, M8 asked for three provenance states when §89 (`provisional` becomes the
fourth provenance state) had made it four. M1 quoted a phrase the tree has never written. Both are
amended in the rubric rather than defended. The shape will recur: a rubric ages against a moving
tree, and the first thing to go stale is any row that counts something.

## Nothing gates this

Half closed 2026-08-18: something now goes red when a run is owed. The other half stays open by
design, since nothing runs the test. Run 6 (2026-09-19) showed the red has no reader; milestone 495
tracks that, NOT-STARTED as of 2026-09-24.

The test is run when somebody runs it. That is rung four of CLAUDE.md's ladder, the same weakness
milestone 117 (the stranger test) was written to fix one level down. A periodic run is possible and
is not built.

Run 3 made a run cheaper rather than automatic. Its harness was a clone, one `sed`, and one `claude
--safe-mode` invocation from the clone's parent. That is a script somebody could write in an
afternoon and nobody had. Run 4 ran the same harness by hand again and did not write it either. That
made four runs of a rung-four mechanism inside the milestone that exists to move things off rung
four, and it is the reason 117 does not move.

### The harness became a script, 2026-08-18

It was written 2026-08-18 as `script/stranger-test`, and the entry did not close. The price of a run
fell from an afternoon of reconstruction to one command. The four isolation failures became the
script's problem rather than the operator's memory. What did not change was the entry's opening
sentence: nothing scheduled the test, and nothing went red when it had not been run in a month. A
cadence is a decision about how often the answer is worth its cost. That is an architect's call
rather than a lane's, and milestone 129 (scheduled execution) is the machinery it would use.

[Run 5](run-5.md) used the script and it held, which is the evidence the lane that wrote it
deliberately did not produce. It took one command, after a `--smoke` run first for `$0.09`. None of
the four kinds of isolation failure that runs 1 through 4 each hit recurred, and the `nife-dev` link
was back where it was found. That did not close the entry either. "The harness works" is the
sentence most likely to be mistaken for "the milestone moved", and a run still happens only when
somebody runs it. What run 5 adds is that the price is now one command, and that the price bought a
real measurement. A fifth run through an unexercised script would have been measuring the script.

### The scheduling half closed, 2026-08-18

calef decided [the cadence](../stranger-test.md#the-cadence-monthly-decided-2026-08-18), monthly.
That was the decision the entry was waiting on, rather than a missing afternoon of work.
`script/stranger-test --due` reads the interval and the last run's date out of the main page and
exits 1 when a run is owed. `.github/workflows/stranger-cadence.yml` asks it weekly. So something
now goes red when the test has not been run in a month, which this entry had asked for since run 1.

What does not close, and never will, is the first clause read literally: nothing *runs* the test. A
run is a person spending half an hour and the budget. A mechanism that pretended otherwise would be
a worse defect than the gap. The residual is the one every record-derived signal has. The tripwire
believes the headings, so a run nobody writes up leaves it red, and a heading nobody earned turns it
green.

### Run 6: the notification has no reader

Run 6 came due on 2026-09-17. The workflow asks on Mondays. It was green on 2026-09-14 and could not
go red before 2026-09-21. [Run 6](run-6.md) happened on 2026-09-19 because a maintainer briefed a
lane.

Had it gone red, the red is an Actions-tab entry. `script/cadence-check` is the watcher that reads
those for `helpers/trunk-health.sh`, and it calls a workflow DEAD only after fifteen days without a
*success*. So "a run is due" and "the job is broken" arrive as one state, a fortnight late. The
audit cadence, whose red means the same thing, had been red five Mondays running on 2026-09-19 with
nobody acting.

So the cadence did not fail by missing its date. It has never been tested, and the path from red to
a person does not exist. The proposal is in milestone 117's block, and milestone 495 (a cadence that
says "due" reaches nobody) tracks it, NOT-STARTED as of 2026-09-24.

## The instrument leaks through the record it is required to keep

Found by [run 4](run-4.md) (2026-08-18). Open, and this one has no fix.

Run 4's stranger knew it was at least the fourth walker within half an hour. The source was not the
harness. It was the `BUGS` table in `notes/adding-a-program.md`, whose rows name run 2's `doubler`
and run 3's `triangle`. Each of those lanes recorded its walk exactly as this project's conventions
require. So the leak is generated by the instrument's own good practice, and it compounds by one row
per run.

Every obvious remedy is worse. Deleting the rows fabricates a tree that never existed. Hiding them
extends the answer-key rule to something that is not an answer key.

The stranger disclosed that it *"performed"*: it chose its deliverable partly for how it would read,
and adopted the tree's prose style as mimicry. It did not invent a breakage, skip a gate, or open
the withheld note. So the shape to expect from run 5 was a stranger who knows it is being measured
before it reads anything, and the response is to ask it, not to try to hide it. From here on,
discount the rhetoric in a run's output by default rather than as an exception.

## The harness isolates a tree and cannot isolate a machine

Found by [run 5](run-5.md) (2026-08-18), in its first ten minutes. Partly addressed before run 6
(2026-09-19); the machine-global link remains shared, so the entry stays open.

`nife-dev` is an account-wide `rustup` link. The script records it before the run and restores it
after, which is necessary and is not sufficient. *During* the run, the clone's first `script/test`
compiled `std` out of whichever worktree built the farm last. Then `std-aborts` failed, naming two
files and two line numbers inside another checkout on this machine.

The task text says *"the repository is in ./nife and it is the only thing you have; do not look for
another checkout on this machine"*. So the run asked for something the machine cannot supply. The
stranger broke the instruction by running `ls -la` against the path its own build had printed, and
disclosed that unprompted.

There are two options, both above a lane. Pre-warm the clone's farm before handing it over. Or say
in the task text that the machine is shared, which costs the comparability of five runs' worth of
identical text. The defect is recorded in `script/stranger-test`'s `BUGS` and in `notes/std.md`'s,
where the missing `farm_dir()` assertion is the underlying defect.

Since before run 6 (2026-09-19), `std-aborts` asserts that its dep-info paths are under
`farm_dir()`. That assertion has not yet met a real contaminated farm. The machine-global `nife-dev`
link remains shared.

## The stranger read the harness

Found by run 5 (2026-08-18). Open; it closes with the entry above.

The harness's own `BUGS` had called this luck: "no run has opened the harness; that is luck rather
than design". Run 5 falsified it about thirty minutes in. It was not curiosity. The foreign path in
the entry above appeared in the build output, and `script/stranger-test` was the only file in the
tree containing that string, so grep sent it there.

The file describes the isolation, the withholding, the shims, the disclosure and the four previous
runs. Its paragraph about restoring the `nife-dev` link is what let the stranger diagnose the
failure instead of committing a false statement to `ABORTS_ACCEPTED`. So the harness contaminated
the run and then rescued it, and a future run that hits the same path will do the same. Fixing the
machine-global defect above closes this one too, which is the reason to price them together.

## Telling the stranger it is measured does not remove what knowing does to it

Found by run 5 (2026-08-18). Open. The disclosure has been live since run 5 and remains the policy
(2026-09-24).

This is run 4's handoff 5, acted on and measured: **disclosure buys honesty rather than
cleanliness.** Run 5 knew from the first message, and from `README.md` within the first minutes. By
its own account the knowledge chose its program: it deliberately picked a manifest combination
nothing in the tree had used, because repeating the earlier walks' shape "would measure nothing"
([run 5](run-5.md)). Its own reading is the one to keep:

> *"That is not what a fresh contributor does. A fresh contributor writes another `doubler`, finds
> nothing, and that null result is data the repository does not get from me."*

It also performed after the task text asked it not to, and named the style it had adopted. Keep the
disclosure, since the alternative is the same effect undisclosed. Stop expecting a run to produce a
naive walk of `notes/adding-a-program.md`; five runs of instrumentation have spent it.

## M2 is the rubric row to watch rather than amend

Found across runs 1 to 5. Open.

Three of five runs cannot answer M2. The two that could each did it from a page they happened to
open (`notes/std.md` for run 4) rather than from `notes/net.md`, which no run has ever opened. The
row is not wrong and the tree's answer is not missing. The answer is unreachable by anyone doing
ordinary work. Run 5's lane did not amend the table, on purpose. Run 4's lane had amended it before
scoring against it, and said in its own contamination section that this made things worse.

## Four runs by four agents is not four data points about a person

Open, and it gets no weaker with repetition. Written after run 4; by 2026-09-24 it is six runs by
six agents.

All four runs were agents. All four read further before asking than a human would, and all four were
told nobody was available. The note's standing caveat holds: **every number here is a lower bound.**

Run 3 sharpens this in one direction only. It spent an hour on a failure whose explanation was four
hundred lines further down a note it had already opened. A human would have given up or asked long
before that. So the *documentation* findings are lower bounds by a wider margin than the build ones.
