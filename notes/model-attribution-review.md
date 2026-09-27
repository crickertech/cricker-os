# Reviewing the work of one model, without answering the question in advance

*Name provisional (`model-attribution-review`). This is a **plan**, not a review. Nothing here is a
verdict on any model, and the reconnaissance below is deliberately reported in the order it was run
rather than in the order that would make a story.*

calef raised a concern about the quality of the code written by Claude Sonnet 5 in this tree and
asked for a plan he could approve, redirect, or decline. This note is that plan. It also carries the
cheap measurements that were run first, because they were expected to narrow the expensive work and
they did something better than that: they moved the whole question.

## The trap this plan exists to avoid

A review that reads only one model's commits cannot answer the question it was sent to answer.

All code contains defects. Reading 180 diffs will find some. Those findings will read as
confirmation whether or not the work is actually worse than anyone else's, because there is nothing
in the exercise capable of producing the other answer. Such a review spends an architect's attention
and hands back a false result with evidence stapled to it, which is worse than handing back nothing.

So every tier below is built to be capable of exonerating as well as indicting: a control group, a
rubric fixed before the reading, blinding where blinding is achievable, and a stated null result. The
sections on what each of those costs here, and where blinding does not work at all, are the load
bearing part.

## First: the numbers this lane was handed do not reproduce

The brief gave a table of commits per model. Measured on `origin/main` at `0b993b27`, it does not
come out that way, and the discrepancy is not rounding.

| Model | Handed to the lane | Measured here |
|---|---|---|
| Claude Opus 5 (1M context) | 658 | 623 |
| Claude Fable 5 | 360 | 338 |
| Claude Opus 4.8 | 233 | 233 |
| Claude Sonnet 5 | 181 | 180 |
| Claude Opus 5 | 35 | 32 |
| (no trailer) | 648 | 648 |
| total non-merge | 2054 | 2054 |

The total and the untrailered count agree exactly; four of the five per-model rows do not. The handed
per-model rows sum to 1467 against 1406 commits that carry a trailer at all, so that table
double-counts 61 commits somewhere. Nothing here reproduces it, and no commit in this tree carries
more than one `Co-Authored-By: Claude` line, which is the obvious way it could have happened.

Two derivations, checked against each other, both giving the measured column:

```sh
# via git's trailer parser
git log origin/main --no-merges \
  --format='%(trailers:key=Co-Authored-By,valueonly)' \
  | sed 's/ *<.*//' | grep '^Claude' | sort | uniq -c | sort -rn

# via the raw message, in case the trailer block is malformed somewhere
git log origin/main --no-merges --format='%H%x00%B%x00%x00' | python3 -c '
import sys,re
for rec in sys.stdin.read().split("\x00\x00"):
    p=rec.split("\x00")
    if len(p)>1: print(*set(re.findall(r"(?im)^\s*Co-Authored-By:\s*(Claude[^<\n]*?)\s*<",p[1])) or ["(none)"])
' | sort | uniq -c | sort -rn
```

This is milestone 125 (a number in the prose is a claim) arriving uninvited. The point is not that
someone miscounted. It is that the first number in a study about trustworthiness was itself
unverified, which is exactly the failure the study would be looking for.

## What the reconnaissance found

### 1. Sonnet's work is one week and one session, and the session is still open

Every one of the 180 Sonnet-trailered commits falls between 2026-08-22 and 2026-08-27. No other
model has a single commit after 2026-08-21.

| Model | Commits | Span | Distinct `Claude-Session` ids |
|---|---|---|---|
| Claude Opus 5 (1M context) | 623 | 07-29 .. 08-20 | 4 |
| Claude Fable 5 | 338 | 07-27 .. 08-16 | 2 |
| Claude Opus 4.8 | 233 | 07-22 .. 08-13 | 3 |
| Claude Sonnet 5 | 180 | 08-22 .. 08-27 | **1** |

178 of the 180 carry a `Claude-Session` trailer and all 178 name the same session,
`01XbKDohBwwjgJtdWD9qmoa4`. That is the session this lane is running inside. A session is not a
model, since the model can be switched mid-session, but the direction of the inference still holds:
**Sonnet's entire footprint in this tree came out of one session over six days.**

This is the finding that reshapes the plan. The comparison anyone would naturally draw, Sonnet
against the other models, is a comparison of one session against another, over a different week, on a
different tree, at a different milestone range, with different lanes and different briefs. Model is
perfectly confounded with date, and date carries the merge queue's arrival, milestone numbers in the
150s to 180s, an x86_64 port in flight, and a tree-wide rename campaign. Nothing in the commit record
can separate those.

It also means the concern itself has a plausible alternative explanation that costs nothing to state:
the work calef has been reading most recently is Sonnet's, because all of it is recent.

### 2. Attribution is clean per pull request, and that recovers a third of the blind spot

648 of 2054 non-merge commits carry no model trailer, so coverage is 68%. The gap is not random.
Mapping every non-merge commit onto the pull request that merged it:

- 95 pull requests contain at least one Sonnet-trailered commit.
- **Zero of those 95 contain a commit trailered to any other model.** A pull request is a lane, and a
  lane ran one model.
- Those 95 pull requests hold 242 commits: 180 trailered, 62 untrailered, 0 other-model.

So the 62 untrailered commits inside Sonnet pull requests are Sonnet's in everything but the trailer,
and **Sonnet's real footprint is at least 242 commits, 34% more than the trailer count**. Attribute
by pull request, not by commit. Across the whole tree the same rule holds well: of 533 pull requests
carrying any trailer at all, 338 are single-model and only 5 mix models.

### 3. The rest of the blind spot cannot be recovered from the commits

No untrailered commit carries any other Claude marker. Not a `Claude-Session` line, not a "Generated
with Claude Code" footer, zero out of 648. Against 1022 of the 1406 trailered commits that do carry a
session id. The markers are all-or-nothing per commit, so there is no second source to fall back on.

That matters most in the window under question. From 2026-08-22 onward there are exactly two
populations:

| Population | Pull requests | Commits |
|---|---|---|
| Sonnet-trailered lanes | 95 | 242 (180 trailered + 62 not) |
| Lanes with no trailer of any kind | 80 | 140 |

**There is no other model in the window at all.** The only same-window comparison available is
against 80 pull requests whose author is unknown, and they could be Sonnet lanes whose trailer never
fired.

### 4. Where the work went, and it is not mostly code

| Bucket, % of that model's touched-file rows | Opus 5 (1M) | Fable 5 | Opus 4.8 | Sonnet 5 | (none) |
|---|---|---|---|---|---|
| `kernel/` | 11.0 | 11.5 | 28.1 | **25.7** | 19.4 |
| `kernel/src/arch/` | 2.7 | 2.2 | 8.8 | **6.5** | 5.3 |
| `crates/` | 14.4 | 17.2 | 12.7 | 13.5 | 17.6 |
| `user/` | 14.6 | 9.1 | 8.1 | **15.6** | 8.9 |
| `design/` | 16.7 | 22.9 | 8.4 | 14.9 | 14.9 |
| `notes/` | 17.9 | 14.6 | 15.1 | 11.4 | 15.9 |
| `script/`, `xtask/`, CI | 9.0 | 7.7 | 9.1 | **3.7** | 7.0 |

Sonnet sits second only to Opus 4.8 in kernel share, which is the highest-risk destination in the
tree, and lowest of all five in tooling. Its commits touch 7.3 files each against 4.1 to 5.2 for
everyone else. 80 of its 180 commits touch no `.rs` or `.s` file at all.

Sizes, for pricing anything below: 46,223 diff lines total, of which 38,459 are in commits that touch
code and 31,414 are in the 72 commits that touch `kernel/`. 22,407 of the lines it added are live in
`HEAD` as `.rs` or `.s`.

### 5. It sat squarely on the irreversible list, and that is the review worth doing

*Move fast on what can be undone* names the expensive categories: anything two programs agree on,
names, dependencies, the syscall surface, and facts that left the machine. The Sonnet window is
unusually concentrated in exactly those.

| Category | Sonnet commits | Detail |
|---|---|---|
| `crates/abi` (the syscall surface) | 7 | all of them renames, 649 diff lines across this row and the next two |
| `kernel/src/syscall.rs` | 6 | |
| `crates/*_proto` (wire crates) | 23 | 14 files across 10 protocol crates |
| Cargo manifests | 14 | 17 manifests, plus 12 commits touching a `Cargo.lock` |
| Roadmap and decision records | 90 | 47 files |
| `AGENTS.md` | 3 | |
| Renames | 52 | including `untyped.rs` to `memory_region.rs`, `Tcb` to `ThreadControlBlock`, `CSpace` to `CapabilityTable` |
| New crates | 7 | `jh7110_entropy_source`, `login_protocol`, `schedule_store`, `counter_frequency_protocol`, `uptime`, `user_mode_runtime` additions, `watch` |
| New user programs | 7 | `login`, `printenv`, `uptime`, `watch`, `session_reviver`, `jh7110_entropy_source`, `login_test_client` |

Milestone 158 (build the eleven kernel object and identifier renames) is most of that, and it landed
in this window. So the week's largest single act was a tree-wide rename of the capability vocabulary,
which is the most expensive thing on the reversibility list and the one that lands in a reader's head
rather than in a file.

**The naming half of this needs no new review**, and that is worth saying because it is the part
that looks most alarming. `script/names` already tracks it, calef already ratified the big ones in
the window (`memory_regions`, `page_frames` and `login_protocol` all carry 2026-08-23), and the rest sit
on the existing worklist as `provisional` or `unrecorded` (`uptime`, `watch`, `printenv`,
`session_reviver`, `schedule_store`, `counter_frequency_protocol`, `jh7110_trng`, `login`, `login_test_client`).
That is the mechanism working exactly as designed. `script/names --unratified` is the queue; it does
not want a study.

*Two of the names in the three rows above have since moved: the crate and program `jh7110_trng`
became `jh7110_entropy_source` when calef ratified it on 2026-09-13, and `jh7110_entropy` when he
replaced that later the same day (performed 2026-09-14). They are spelled here as they were in the
window this reconnaissance measured, which is what makes the counts checkable against the commits
they came from.*

### 6. Three defect metrics, and they rank the models three different ways

This is the part that most wanted to become a finding and must not.

**Survival.** Share of a model's added lines still live in `HEAD`:

| | Opus 5 (1M) | Fable 5 | Opus 4.8 | Sonnet 5 | (none) |
|---|---|---|---|---|---|
| lines added | 147,042 | 72,991 | 30,536 | 36,886 | 131,111 |
| survival | 70% | 77% | 63% | **88%** | 77% |

Sonnet looks best by a wide margin. The number is worthless: survival rises monotonically as code
gets younger, and Sonnet's is the youngest by three weeks.

**File-level fix-up.** For each code commit that went through a pull request and has a full 72 hours
of follow-up available, did a later commit *in a different pull request* touch one of the same files
with a fix-shaped subject:

| Opus 5 (1M) | Fable 5 | Opus 4.8 | Sonnet 5 | (none) |
|---|---|---|---|---|
| 36% (124) | 24% (140) | 0% (4) | **57% (51)** | 31% (232) |

Sonnet looks worst by a wide margin. This number is also close to worthless, in the opposite
direction. It is file-level, so any commit landing near another in the same file counts, and a
tree-wide rename campaign guarantees a wave of follow-up commits repairing stale references in files
the renamer touched. Read the actual pairs and most of them are coincidence: a clippy lint fix
following an unrelated builtin, a docs pass on stale symbol names following an x86_64 interrupt
change.

**Line-level, blamed back.** The measurement worth running. For each fix-shaped commit, take the
lines it deleted, blame them at its parent, attribute them to whoever wrote them. Normalised per
1,000 code lines added, split by subsystem, with the follow-up window capped at 6 days for every
model so exposure is equal:

| per 1k lines, fixes within 6 days | Opus 5 (1M) | Fable 5 | Opus 4.8 | Sonnet 5 | (none) |
|---|---|---|---|---|---|
| `kernel/` | 2.1 | 12.1 | 16.2 | **11.5** | 3.9 |
| everything else | 1.2 | 1.4 | 11.9 | **8.9** | 2.4 |

Sonnet lands mid-pack in the kernel, indistinguishable from Fable and better than Opus 4.8. It is
higher than everyone but Opus 4.8 outside the kernel. The rename campaign was checked as a confound
and cleared: only 9 of the 251 lines blamed to Sonnet were written by a rename commit.

**Three metrics, three orderings, and the whole tree has 77 fix-shaped code commits to build all of
them on.** The counts behind each cell are in the low hundreds of lines, the follow-up window for
Sonnet's most recent commits is a day rather than six, and no confidence interval drawn around these
would exclude "the models are the same". There are zero commits in this tree whose subject begins
with `Revert`, so the sharpest available signal does not exist at all.

The honest reading is that **the cheap evidence does not discriminate**, and that anyone who wants it
to will find a metric that says what they came for. That is the finding, and it is why the tiers
below exist rather than a verdict.

### 7. The sample is uneven, and the good end of it is very good

`1259dc07` is Sonnet's, dated 2026-08-27. The icount tripwire caught an 11.4% `spawn_el0` regression;
the commit bisects it by swapping `kernel/` alone between three of its own branch's commits with the
rest of the tree held constant, attributes 29,302 of the 35,512 ticks to a specific reclamation sweep
by stubbing that sweep out, halves it, states the remaining 6.4% as the price of a correctness fix
rather than hiding it, re-records the baselines with the reason, and reports a further optimisation
that was **built, measured, recovered 3,820 ticks, and was dropped** because it destabilised block
layout. It also names a movement on an unrelated bench row that nobody asked about.

That is better work than most of this tree, by any model. Commit-message length says the same thing
about the middle of the distribution: Sonnet's median body is 1,038 characters against Opus 5's 1,457
and Fable's 673, and 8% of Sonnet's are under 200 characters against Opus 5's 1%.

If the sample is uneven rather than uniformly weak, then the thing to review is **conditions** and
not a model: which lanes were rushed, which briefs were thin, which subsystems were unfamiliar, how
many lanes were running at once against the hot files. A review that reports "the model is worse"
when the truth is "four lanes were racing in `kernel/src/user/tests.rs` on Tuesday" has found nothing
actionable, because nobody can change the model but everybody can change the brief.

## The design a real answer needs, and where it breaks here

### The control group

A defect count with no base rate is not a finding. The control must be matched on subsystem, commit
size, and date, reviewed by the same reviewer against the same rubric.

**Date is the one that cannot be matched.** Sonnet's window is disjoint from every other model's. The
options, none of them clean:

1. **Same-window control: the 80 untrailered pull requests (140 commits).** Perfectly matched on
   date, tree state, milestone range, and lane conditions. Unattributed, so a difference is
   interpretable as "Sonnet lanes versus lanes that did not stamp a trailer" and no further. Cheapest
   and most defensible. This is the recommended control.
2. **Subsystem-and-size-matched control from Opus 5 and Fable, drawn from earlier weeks.** Matched on
   what the reviewer reads, unmatched on everything the tree was doing. Any difference confounds
   model with three weeks of tree evolution.
3. **Both**, which is the only way to see whether the two controls disagree, and they will.

### Blinding, and the honest statement that it leaks

Blinding is implementable and is the difference between a study and a confirmation exercise. The
mechanism already exists in this tree: `script/stranger-test` runs a **separate `claude` process
rather than a subagent**, with its working directory set to the *parent* of the tree, since project
instructions load from ancestors and never from descendants, and it **probes the isolation before the
run** rather than assuming it. Milestone 117 (the stranger test) also withholds its own answer key by
amending the note out of the tree for the run.

Applied here, the scheme is:

1. A harness extracts each sampled commit as `cases/<random-id>.patch`: the diff from
   `git show --format=`, which emits no message and therefore no trailer, plus the commit subject and
   body with every `Co-Authored-By` and `Claude-Session` line stripped.
2. Cases are written in randomised order under ids that encode nothing. The case directory holds the
   patches and the rubric and no git repository, so `git log` is not available to the reviewer.
3. The reviewer is a separate `claude` process started in the case directory's parent, `--safe-mode`,
   no network.
4. Isolation is probed before the run: the reviewer is handed one case and asked which model wrote
   it. The run proceeds only if the answer is that it cannot be determined.
5. Attribution is joined to scores only after every score is written and committed.

**Three leaks that this does not close, and they are large enough that the blinding must be called
partial in the report rather than claimed:**

- **Milestone numbers.** Sonnet's commits cite milestones 158 to 184; the earlier models' cite lower
  numbers. Stripping them mutilates the message that the rubric is partly scoring.
- **Content that only exists after a date.** An x86_64 file, `CapabilityTable`, `login_protocol`. The
  post-rename vocabulary alone identifies the window.
- **The window is the model.** Because the two are perfectly confounded, any leak of the date is a
  leak of the attribution. Blinding cannot do better than the confound allows.

Option 1's same-window control is the only configuration where blinding actually holds, because there
the cases are contemporaneous and none of the three leaks discriminate. That is a second, independent
reason to prefer it.

### The rubric, fixed before the reading

Written and committed before the first case is opened, so severity cannot be decided post hoc in the
direction the reader expects. Six axes, each scored 0 to 2 with the anchors written out:

1. **Correctness.** Does the change do what its message says, on the paths it touches.
2. **Confinement.** Does it widen authority, and does the message say so if it does.
3. **Irreversibility handled.** If it touches the syscall surface, a wire format, a name, or a
   dependency, is the decision recorded and is it calef's where it should be.
4. **Test earns its keep.** Does the test prove something nothing else would have proved, or is it
   filler.
5. **The message explains why.** `git blame` is what a commit is for.
6. **Honest limits.** Does a `BUGS` entry or a stated caveat exist where the change fell short.

Axes 3, 5 and 6 are the ones this tree cares about most and the ones a generic code review would not
score at all.

### The null result, stated in advance

**The study reports "no differential found" when the blinded per-axis mean for the Sonnet arm is
within one standard error of the control arm on every axis, and the count of severity-2 findings per
1,000 diff lines differs by less than a factor of two.** With the sample sizes below, a factor of two
is roughly the smallest effect the study can see, and saying so up front is what stops a null being
written up as a failure to look hard enough.

If the null comes back, the reportable conclusion is not "Sonnet is fine". It is: *this tree cannot
distinguish them at this sample size, and the observed variation is better explained by subsystem and
lane conditions than by model.* That sentence is worth as much as the other one.

## The tiers

Prices are lanes, wall-clock, and rough tokens. They assume the reconnaissance above as done and not
repeated.

### Tier 0: what you already have. 0 lanes, 0 tokens.

This note. It establishes that the window is one session and six days, that model and date are
perfectly confounded, that the cheap metrics disagree with each other, that the naming half is
already tracked by `script/names`, and that the irreversible surface Sonnet touched is 649 diff lines
across 25 commits rather than anything like the whole 46,223.

**If calef declines everything below, the reconnaissance still says: there is no cheap evidence of a
differential, and the expensive evidence would be hard to obtain and easy to fake.**

### Tier 1: the irreversible surface, unblinded, no control. 1 lane, 2 to 3 hours, ~400k tokens.

Read every Sonnet diff touching `crates/abi`, `kernel/src/syscall.rs`, and `crates/*_proto`: 25
commits, 649 diff lines. Then the 7 new crates' and 7 new programs' manifests and capability
declarations, and the 14 Cargo manifest changes against §46's rule that a dependency is a decision.

This tier is deliberately not a study and makes no claim about any model. It asks a different and
more useful question: **did anything expensive to undo land in that week without the record it
should have carried.** A control group would add nothing, because the standard is absolute rather
than comparative.

**It should be folded into the security audit that is already due rather than run as its own thing.**
`script/audits` currently reports `documentation` and `security` both overdue, each with three
triggers fired, and the last security audit's lens was "newly minted authority, read adversarially"
on 2026-08-17. Pointing the next one at the window is one row in
`design/audit-reports/README.md`, which is milestone 93's own precedent for adding a kind without
adding machinery. Cost in that shape: **zero additional lanes**, because the audit is owed anyway.

Recommended, and recommended first.

### Tier 2: blinded sample with the same-window control. 2 to 3 lanes, ~1 day, ~2 to 3M tokens.

- Lane A builds the harness: case extraction, scrubbing, randomisation, the isolation probe. Roughly
  the shape of `script/stranger-test`, reusing its isolation mechanism rather than reinventing it.
- 60 Sonnet cases, stratified by subsystem (kernel, crates, user, prose) and by size, drawn from the
  242 pull-request-attributed commits rather than the 180 trailered ones.
- 60 control cases from the 80 untrailered same-window pull requests, stratified to match.
- Lanes B and C score independently against the pre-committed rubric, in isolated processes.
  Inter-rater agreement is reported; a rubric two readers cannot agree on is not a rubric.
- Attribution joined last.

**What it can conclude:** whether Sonnet-trailered lanes differ from same-window untrailered lanes at
an effect size of roughly two-to-one or larger. **What it cannot:** anything about Sonnet versus Opus
or Fable, since neither is in the window.

That limitation is severe and is the reason this tier is second rather than first.

### Tier 3: exhaustive, with a cross-window control. 5 to 6 lanes, 2 to 3 days, ~8 to 12M tokens.

All 242 pull-request-attributed Sonnet commits, plus a subsystem-and-size-matched 242 drawn from
Opus 5 and Fable in earlier weeks, all scored blind against the same rubric.

This is the only tier that produces a Sonnet-versus-other-model number. It is also the tier whose
number is confounded by three weeks of tree evolution and by partial blinding, and it costs an order
of magnitude more than tier 1.

**Not recommended.** The confound is not fixable by spending more, so the extra spend buys precision
on a quantity that is not the one anyone wants.

### Tier 4: the prospective A/B, which is the only design that isolates the model. Costs no review lanes.

Assign the next N ready milestones alternately between two models, same brief template, disjoint
subsystems, both lanes running in the same days against the same tree. Score the results blind
against the tier 2 rubric.

This is the only design in which model is not confounded with date, tree state, or milestone
difficulty. It costs nothing in review lanes because the lanes were going to run anyway. It costs
scheduling discipline and a delay, since the answer arrives when the milestones do.

If calef wants a real answer about a model rather than about a week, this is the one to approve, and
it is compatible with approving tier 1 today.

## What no tier can find

Stated plainly, because a plan that hides its blind spots is the thing it is trying to review.

- **A design decision that was wrong but implemented cleanly.** Every tier reads diffs. A diff that
  correctly implements the worse of two options looks exactly like a diff that correctly implements
  the better one. The rename campaign is precisely this shape: whether `CapabilityTable` was the
  right call is not visible in the patch that performs it.
- **Work that was never written.** A lane that stopped early, declined a hard item, or produced a
  thinner milestone than the block asked for leaves no diff to read. The roadmap blocks would show
  it and the commits will not.
- **Anything about the 648 untrailered commits.** They are 32% of the tree and no source in the
  repository attributes them.
- **Whether a defect was caught or landed, before roughly 2026-08-15.** Per-branch check history is
  only reliably available for recent pull requests, so "did the gates catch it" is answerable for the
  Sonnet window and progressively less so going back, which biases any gate-based comparison toward
  finding the recent window worse.
- **Effects smaller than about two to one.** With 77 fix-shaped commits in the whole tree and 120
  cases in tier 2, the study is underpowered for anything subtler. It will not distinguish a good
  model from a slightly better one, and should not be asked to.

## The six questions

*A fork reaches calef with its questions already answered.*

1. **What else was considered, and why did each lose?** An unblinded read of all 180 Sonnet diffs
   (the request as first framed) loses because it cannot produce a negative result. A metrics-only
   answer with no reading loses because §6 above shows three metrics giving three orderings. A review
   scoped to "commits with the Sonnet trailer" loses to pull-request attribution, which recovers 62
   more commits for free. Tier 3 loses to tier 4 on the only axis that matters, which is whether the
   confound is removable.
2. **What does this tree already do in the analogous case?** It runs audits as a mechanism:
   `script/audits`, `design/audit-reports/README.md`, seven audits on record, each a named lens with
   a cadence row and findings dispositioned as fixed, minted, or accepted. It blinds a reader by
   spawning a separate `claude` process whose working directory is the tree's parent, and it probes
   the isolation before trusting it (`script/stranger-test`). It tracks unratified names in
   `script/names` rather than in a study. Tier 1 uses all three rather than growing twins of them.
3. **What is the prior art outside the tree?** The line-level attribution in §6 is the SZZ algorithm
   (Śliwerski, Zimmermann and Zeller, *When Do Changes Induce Fixes?*, MSR 2005), and its known
   failure modes are the ones observed here: keyword-matched fix commits over-select, and
   refactoring commits absorb blame for lines they only moved. The rename-campaign check in §6 is the
   standard mitigation for the second. Cited as a method, not quoted; nothing here is a block quote
   from memory.
4. **Is the premise true?** Partly. "Sonnet wrote 181 commits" is close (180 trailered, 242 by pull
   request). "Sonnet's commits can be compared against other models' commits in this tree" is
   **false**, and that is the finding: there is no overlap in time, and the comparison everyone would
   naturally draw is between two different weeks.
5. **What does each option cost, measured rather than asserted?** Priced above in lanes, hours and
   tokens, from measured diff sizes (46,223 total, 38,459 code, 649 on the irreversible surface)
   rather than from estimates.
6. **How reversible is it, and who has already acted on it?** The review itself is fully reversible;
   nothing it produces changes the tree. Its *conclusion* is not. A written verdict that a model is
   worse is a fact that leaves the machine, and this project's own tenet puts that in the
   irreversible category alongside a published benchmark. That is the reason for the rubric, the
   control and the stated null: a wrong verdict here cannot be recalled by deleting a note.

## BUGS

- **The follow-up window for Sonnet's newest commits is under a day, not six.** The exposure-equalised
  table in §6 caps every model at 6 days, but a commit written on 2026-08-27 has had one day for a
  fix to arrive. Sonnet's line-level rates are therefore a **floor** and will rise as the tree moves
  on. They were not adjusted for this, because any adjustment is a model of arrival rates that
  nothing here has measured.
- **The fix-shaped-commit regex is keyword-matched and this tree writes self-critical prose.** Words
  like "wrong", "broken" and "stale" appear in commit bodies describing the *reason* for a change
  rather than a defect being repaired. 77 commits matched; they were eyeballed and most look genuine,
  but no one has classified them one by one.
- **Nothing here checks whether a defect was caught by CI or landed.** The distinction the brief asked
  for, gate-caught versus gate-escaped, was approximated by "was it fixed in a different pull
  request", which is a proxy and not the thing. Several Sonnet fix commits say "caught by CI" in their
  own subjects, which is the gates working and which this note's numbers count as defects anyway.
- **Session id is used as a proxy for a lane and it is not one.** A maintainer session and every
  subagent it spawns share an id, and the model can change inside a session. The claim "Sonnet's work
  is one session" is supported; the claim "one session means one set of conditions" is an inference
  and is not measured.
- **This note was written by an agent inside the very session whose output is under review.** That is
  a conflict of interest and no mitigation was applied beyond running measurements rather than
  offering impressions and reporting all three of them when they disagreed. Any tier that runs should
  not be scored from this session.
