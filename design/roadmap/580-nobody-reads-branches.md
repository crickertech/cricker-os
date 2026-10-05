---
status: BUILT
raised: 2026-09-23
built: 2026-09-23
promoted_from: nobody-reads-branches
---
# 580. Nobody reads branches, so give the maintainer a command that does

*(Number minted at promotion.)* Promoted from the proposal
`nobody-reads-branches`, filed 2026-09-23, on calef's instruction of 2026-09-23 to give the two
unnumbered proposals on `main` a number. The text below is the proposal's own, unedited except for
this paragraph, the removal of its `**Gate: NONE.**` line and the two sections appended at the end:
the argument is its author's and promotion is not the moment to improve it. The gate line goes
because a BUILT block may not carry one, and what it said (nothing here touches the syscall
surface, adds a dependency, or needs hardware) was true and is now moot: the mechanism landed in
`helpers/lane-claim-check.sh` the same day it was proposed, in commit `448928d93`. As filed: decided
the same day; see "What calef decided" below. It was kept in the proposals directory rather than
promoted at the time, because the disposition is a fix to an existing script, not a numbered
milestone, and there was no numbered block for it to be drained into. Raised by the
`maintainer/nobody-reads-branches` lane, written while surveying the 25 remote branches calef's
2026-09-23 sweep found carrying no open pull request, the oldest last touched nine days earlier.
Verified independently against the live repository while drafting this: 24 of the 25 are real lane
branches (`milestone/*`, `maintainer/*`, `fix/*`); the 25th, `gh-readonly-queue/main/pr-1080-...`,
is the merge queue's own synthetic candidate branch, not a lane's, and any mechanism built from this
proposal must exclude that shape the same way `lane-claim-check.sh` already excludes the ones it
found.

## What calef decided

calef read this the same day it was written and refused the closing recommendation: a fourth
watcher script, even a narrow one, competes for the same attention the tree already fails to spend
on the three it has (AGENTS.md's own "reported and never acted", the exhibit this proposal itself
leans on). His ruling: **one script that already exists, already runs, and already has a reader,
instead of a second competing for the same attention.** So the mechanism below landed as three
changes to `helpers/lane-claim-check.sh` rather than as `helpers/orphan-branch-check.sh`, in the
same lane, the same day.

That reverses one specific refusal from "What else was considered" below: extending
`lane-claim-check.sh` was refused there because "the two problems want thresholds three orders of
magnitude apart... folding the second into the first risks breaking the tuning that is currently
working." The fix is to keep both thresholds rather than pick one: `milestone/*` keeps the existing
15-minute `GRACE_MINUTES` (the push-to-draft gap §90 protects), and every other branch gets its own,
much longer window (`GRACE_HOURS`, default 24, see below), so widening scope does not touch the
tuning that was already correct. The rest of that refusal's reasoning, that a fourth-watcher script
and a wider `lane-claim-check.sh` are different shapes, is what calef overruled directly rather than
what this lane re-argued.

The original recommendation, its measurements, and every refusal below are kept as written: they
are the reasoning that produced the ruling above, and this tree keeps refusals rather than erasing
the record of a path not taken.

## The failure is at least four shapes, and one bystander shape worth naming alongside them

Lumping these into "orphaned branches" is how the existing tooling already missed most of them.
Each shape has a different cost if handled wrong, and that is the reason to keep them apart rather
than write one blanket rule.

**1. A branch with real unmerged work and no pull request, ever.** The dangerous case: deleting it
destroys work nobody landed anywhere else. Measured against today's tree: `maintainer/294-and-510`
(6 commits, 47 files, +918/-1517), `milestone/434-retire-the-proposed-disposition` (8 commits, 10
files, +503/-438), `milestone/326-machine-discovery-truncation` (5 commits, 26 files, +881/-21),
`milestone/435-slice-a` (3/29), `milestone/435-slice-b` (3/17), `milestone/435-slice-c` (5/31), and
`maintainer/metrics-2026-09-19` (1/7). Every file and commit count above was re-derived from
`git diff --stat origin/main...origin/<branch>` while writing this, not copied from the sweep
uncrosschecked, and it matches. Ages at measurement time ran two to four days.

**2. A branch that never grew past its §90 claim commit.** §90 (the claim is a draft pull request;
the status flip is a gate) is the mechanism this proposal leans on throughout: a lane died between
`git push` and `gh pr create --draft`, or never even reached the push in the shape §90 expects.
Zero cost to delete: nothing on the branch differs from `main`. `milestone/433-slice-1` through
`-4` and
`maintainer/ratify-boot-slot-format` carry no commits ahead of `main` at all; `fix/qemu-bounded-
stdin-under-dash`, not named in the original sweep, is the same shape one prefix over, which is
worth noting because it shows the pattern is not confined to the two prefixes `lane-claim-check.sh`
and the sweep both happened to be looking at.

**3. A branch whose pull request already merged and was never deleted.** Also zero-cost to delete;
this is `lane-claim-check.sh`'s own `LEFTOVER` case, working exactly as designed, for the one
prefix it watches. The gap is prefix, not shape: of five such branches found live
(`maintainer/mutation-sweep-2026-09-14`, `maintainer/open-model-lanes`,
`maintainer/the-counter-thesis`, `maintainer/what-a-component-costs`,
`milestone/442-crypto-provider`), four are `maintainer/*`, which `lane-claim-check.sh`'s own `BUGS`
section says it does not watch, on purpose, because widening its pattern would also catch the
short-lived branches its narrow scope is protecting.

4. A branch whose pull request was closed without merging. Two found:
`maintainer/baselines-and-282` (PR #1078) and `maintainer/icount-baselines-stale` (PR #883). Both
closed PRs carry a body and at least one comment, so the reasoning likely survives on GitHub even
after the branch goes, but "likely" is doing real work in that sentence and `lane-claim-check.sh`
today does not distinguish this from case 3: its `LEFTOVER` line fires on any non-`OPEN` state and
recommends deleting either way. That conflation is a real bug in a script this proposal otherwise
leans on, worth fixing on its own account; flagging it here is this proposal's way of giving it a
home rather than letting it die a second time in a lane report nobody re-reads, which is the exact
failure this whole document is about.

The bystander shape: a stale draft holding real work, whose one existing report already fired and
was ignored. PR #1087 is not branch-without-PR (it has one, open, draft), so it is out of this
proposal's scope by definition. It earns a mention because it is the cleanest evidence available for
the design argument below: `merge-drain.sh`'s `stale_drafts` check fired on it correctly, at
2026-09-22T02:22:54Z, posting the comment it is built to post ("has not committed in over 75
minutes... If its lane is finished: `gh pr ready`"). Twenty-four hours later the draft, 8 commits and
+1398/-83, was still sitting there, untouched, found again only because calef looked by hand. The
mechanism worked. The report changed nothing. That is not a case this proposal fixes; it is the
proof that adding a fifth one-shot reporter to the four failure shapes above would not fix them
either.

## Case 3's assumption was wrong, and this branch proved it

Case 3 above, and `lane-claim-check.sh`'s `LEFTOVER` check that this proposal leans on, both treated
"pull request merged" as "safe to delete," the same zero-cost case as case 2's dead claim. That
looked right because it usually is: most of the time a branch tip is exactly the commit GitHub
merged, nothing else lands on it afterward, so checking pull request state is a cheap stand-in for
checking the tree itself.

It is wrong whenever more commits get pushed to the branch after its pull request merges, and
`maintainer/open-model-lanes` is the branch that disproved it. PR #1089, cut from this branch,
merged; three more commits (on the gateway's no-key posture, the token, and the bind address) were
then pushed to the same branch and never landed anywhere else. The old rule, "pull request merged,
therefore delete the branch," would have told a reader those three commits were safe to discard,
and they were not: this document is being written from the branch that carries them, being rebased
onto `main` in the same pull request that adds this subsection.

The fix does not trust pull request state at all: `git merge-base --is-ancestor <branch>
origin/main` asks whether every commit on the branch is already reachable from `main`, which is the
question "safe to delete" actually depends on. A merged-and-untouched branch and a merged-and-then-
extended branch give pull request state the same answer and this test different ones.
`helpers/lane-claim-check.sh` (PR #1116) now tests the tree this way instead of trusting the merge
flag.

## What else was considered

**Extend `helpers/merge-drain.sh`.** Refused: its whole domain is pull requests, and these branches
are defined by the absence of one. Its `notify()` dedup logic posts a comment *on a pull request*;
there is nowhere to attach that comment when there is no pull request. It also already carries three
distinct single-purpose checks (admission, stuck workflow runs, stale drafts) for a reason: each is
one act with its own threshold, and a fourth unrelated one blurs a script that earns its keep by
staying narrow.

**Extend `helpers/lane-claim-check.sh`** to cover every branch prefix and to escalate past its
15-minute grace window into a days-long sweep. This is the closest real contender and the one worth
the longest refusal. Its own `BUGS` section already explains why the `milestone/*` restriction is
deliberate: widening it "would also sweep in short-lived maintainer branches that are not claims and
are not meant to be." Today's data proves the restriction has teeth rather than being theoretical:
`maintainer/capture-the-actions` and `maintainer/subscription-stays` were both pushed by the
maintainer *during the same session* that produced the sweep this proposal answers, and
`subscription-stays` had already resolved itself into PR #1106 before this proposal was finished.
The two problems want thresholds three orders of magnitude apart (minutes to protect a lane between
push and draft, versus days to catch one that never will) and different actions (a nudge to open a
pull request, versus a person deciding whether to claim, land-and-delete, or just delete). Folding
the second into the first risks breaking the tuning that is currently working.

Overruled 2026-09-23; see "What calef decided" above. The risk named here was real but
narrower than the refusal treated it: it is a risk to a *shared* threshold, not to folding the
checks into one file. Two separate grace windows in one script, `GRACE_MINUTES` untouched for
`milestone/*` and a new `GRACE_HOURS` for everything else, keep the 15-minute tuning exactly as it
was measured while adding the wider sweep beside it.

A new `helpers/` watcher of the same shape, run on the existing five-minute `launchd` cadence
and reporting once per stall the way `merge-drain.sh`'s `notify()` already does. Refused on the
strongest evidence available: this shape already exists for the nearest analogous case (`stale_drafts`)
and PR #1087 shows it firing exactly on schedule and changing nothing. A fourth reporter, printing to
a log on patagonia that nobody tails, repeats a failure this tree has already measured rather than
fixing it.

A scheduled GitHub Actions workflow, running independent of any session. `notes/merge-queue.md`
records that calef declined exactly this shape on 2026-08-26 for the analogous case, "an unattended
scheduled agent," preferring something that "shuts down when the session driving it does" over
"standing on a timer with nobody watching." That reasoning was about an agent empowered to *resolve*
a queue problem, and this proposal's mechanism only reports, so the objection does not transfer
mechanically. But a pure report-only workflow still lands in a place nobody has an established habit
of reading (a workflow run's log, or a check summary), which is the same failure as the option
above with GitHub Actions minutes spent to reach it. If it were extended to also *act*, deleting
branches unattended, that runs straight into AGENTS.md's own caution that deleting a branch destroys
work and needs a person's judgment first.

Make it the steward's standing duty, in prose, with no new script. Refused for a reason this
proposal does not have to argue for, because AGENTS.md already argues it: the steward tried exactly
this shape once and the tree's own record of what happened is "it reported and never acted."
Assigning one more duty to a role that has already failed at prose-only duties, in the same
document that names the failure, is rung four dressed as an assignment.

**Do nothing; accept the cost.** The honest baseline, and the one this proposal has to out-argue
with numbers rather than assert past. The oldest of today's 24 is nine days old
(`maintainer/ratify-audits`). §90's own claim mechanism assumes a lane reaches a draft pull request
inside one sitting; the branch that built the check measuring that assumption took three minutes.
AGENTS.md names "somebody will notice" as rung zero and says plainly it "belongs on no list." The
backlog also does not self-correct: nothing currently shrinks it, and every missed claim, closed PR,
or forgotten `git push origin --delete` adds to it. Twenty-four today; the number has no reason to
be smaller next month.

## Which rung, and what this does about "reported and never acted"

Honestly: this stays on rung four. A gate cannot fail a build over a branch that is not part of any
pull request, which is the exact reason `lane-claim-check.sh` refused to be a gate for the narrower
version of this same question, and that reasoning holds here without modification. There is also no
single place in the tree to put rung three's "record beside the thing," because the thing, an
abandoned branch, has no reader who meets it by default; that is the whole problem. So this proposal
does not climb the ladder. What it changes is which rung-four channel the report lands in, and that
distinction is not cosmetic: PR #1087's comment is also rung four, correctly built, and it changed
nothing, because a PR comment and a `launchd` log are both channels nobody has a standing habit of
rereading once the moment passes.

Two design choices aim this report at a channel that is already read, rather than inventing one
that will not be:

- Land the output next to the check AGENTS.md already makes a session read every time.
  AGENTS.md's own merge-queue section already tells a maintainer session to read
  `gh pr list --json number,mergeStateStatus,statusCheckRollup` for `DIRTY`, `CONFLICTING`, or a
  `FAILURE` conclusion, "a standing check, same priority as keeping lanes full." Every fact this
  proposal is built on came from running commands in that same family by hand. Putting the orphan
  survey there, as one more thing that command line checks, costs a maintainer nothing new to
  remember; it extends a habit already proven to work rather than asking for a second one.
- Let branch age do the escalating, because it is already the only state that exists. A branch
  that shows up in this survey unresolved does not go quiet between one session and the next; it
  shows up again, older. No counter, no database, no second file to keep synchronized with the
  branches themselves, which is the same reasoning `lane-claim-check.sh` already applies by reading
  the branch's own birth time instead of maintaining one. This does not make the report act on its
  own, and it should not: deleting a branch is exactly the kind of judgment AGENTS.md reserves for a
  person. What it does is make *not acting* visibly compound, the same age climbing every session,
  instead of resetting to silence the moment the report has been printed once.

## What this would cost, measured against today's twenty-four

The false-alarm question is really a grace-window question, and today's data answers it directly.
Of the 24, only `maintainer/capture-the-actions` was under a day old at measurement time; every
other branch, including all seven holding real unmerged work, was at least two days old and up to
nine. A grace window set at 24 hours, one order of magnitude above `lane-claim-check.sh`'s 15
minutes (which protects the gap between a push and a draft) and comfortably inside the gap between
two maintainer sessions, would have deferred exactly one branch today and flagged the other 23
correctly. `maintainer/subscription-stays`, pushed the same session and in the same position as
`capture-the-actions`, is the clean case for why the window has to exist at all: it stopped being
orphaned on its own, gaining PR #1106, before anyone had to act on a report about it.

Judgment cost per flagged branch is small and case-dependent, which is the point of keeping the
four shapes separate rather than a single list. Six of the 24 (case 2) and five more (case 3) need
no judgment at all, delete is always correct. Two (case 4) need one look at the closed pull
request's own thread, which GitHub keeps regardless of the branch, to confirm the closure carries
its reasoning. The remaining eleven, including the seven detailed above, are the ones actually worth
a maintainer's attention, and that is eleven out of 24, not 24 out of 24, which is what "one blanket
rule" would have cost instead.

## Threshold and action, per shape

No single threshold, because no single action fits every shape. All four share one constraint:
the mechanism reports; a person deletes. Nothing here is proposed to run unattended and delete
anything, for the reason the refused Actions-workflow option already names.

| Shape | Past the 24h grace window | Action, and who decides |
|---|---|---|
| Real work, no pull request (case 1) | Report age, commit and file counts | A person's call: `gh pr create --draft` to claim it visibly per §90, or, if it is genuinely abandoned, land what it found in `notes/` and delete, per AGENTS.md's own line that "a branch holding a finding should have the finding landed in `notes/` first" |
| Empty §90 claim, dead lane (case 2) | Report age | Delete; nothing is lost |
| Merged, not deleted (case 3) | Report age and pull request number | Delete; this widens `lane-claim-check.sh`'s existing `LEFTOVER` check past `milestone/*` |
| Closed, not merged (case 4) | Report age and pull request number, kept visually distinct from case 3 | Read the closed pull request's thread for the reason it closed. **Never recommend deletion from this line alone**; the work may exist nowhere but this branch |

**Implemented, not as a new script.** calef's ruling (see "What calef decided" above) put this in
`helpers/lane-claim-check.sh`: it already exists, already runs, and already has a reader, which beat
a fourth thing competing for the same attention. The script now covers every branch except `main`
and `gh-readonly-queue/*`, splits `MERGED` from `CLOSED` (`state` alone is a reliable GraphQL enum
for this, no need for `mergedAt`), and classifies each unclaimed branch as empty or holding work
before printing it. See its own header for the mechanism and its `BUGS` section for what this
widening cost.

## What is out of scope

- **Not a general queue-automation proposal.** This does not touch `merge-drain.sh`'s admission
  policy, the stuck-checks report, or how the merge queue itself orders candidates.
- **Not auto-deleting anything, ever**, regardless of which shape or how old. Every deletion in the
  table above is a person's action after reading the report.
- **Not resurfacing stale drafts.** PR #1087 is evidence for this proposal's design argument, not a
  case it handles; that gap belongs to `merge-drain.sh`'s existing `stale_drafts` check, or to a
  proposal of its own about re-surfacing a report that already fired once.
- **Not fixing `lane-claim-check.sh`'s `LEFTOVER`/`CLOSED` conflation.** Named above so it has a
  home; fixing it is a separate, smaller piece of work.
- **Not replacing `lane-claim-check.sh`'s early-window check or `merge-drain.sh`'s stale-draft
  check.** Both are tuned for a different clock, minutes rather than days, and both are working for
  the case they were built for.
- **Not a change to §90's claim mechanism itself.** The claim stays a draft pull request; this
  proposal is about what happens when a branch never reaches one, or reaches one and then nobody
  looks again.

## Follow-on

- **Done.** The `LEFTOVER`/`CLOSED` conflation this proposal named as out of scope was fixed in the
  same change that implemented it. `helpers/lane-claim-check.sh` now reports a merged pull request
  and a closed-unmerged one on separate lines, and never recommends deleting from the closed line.
- **Recorded.** What the widening cost stays a limitation, in that script's own `BUGS` section: the
  activity feed is read one page deep and an older branch reports with an unknown age, the script
  writes to stdout only so nothing reaches a lane that is not looking, and "holds work" is sized by
  `git diff --shortstat` rather than read.
- **Refused.** A fourth watcher script, `helpers/orphan-branch-check.sh`, which was this proposal's
  own closing recommendation. calef refused it on 2026-09-23: a narrow new watcher competes for the
  same attention the tree already fails to spend on the three it has. The same reasoning covers
  re-surfacing a stale draft that `merge-drain.sh` already reported once, which this proposal listed
  as out of scope and which stays with `merge-drain.sh`.

## Index row

A branch with no open pull request is invisible to every watcher the tree had, and a 2026-09-23
sweep found 25 of them, the oldest nine days old. This block separates that failure into four
shapes with different correct actions, measures the grace window that tells a dead branch from a
live one, and records calef's ruling that the fix belongs in a script that already has a reader
rather than in a new one. `helpers/lane-claim-check.sh` now covers every branch except `main` and
`gh-readonly-queue/*`, on a second and much longer clock.
