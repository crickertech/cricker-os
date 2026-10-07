---
status: NOT-STARTED
raised: 2026-09-23
promoted_from: constitution-carry-vs-brief
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 579. Which of the constitution must be carried, and which is a brief

*(Number minted at promotion.)* Promoted from the proposal
`constitution-carry-vs-brief`, filed 2026-09-23, on calef's instruction of 2026-09-23 to give the
two unnumbered proposals on `main` a number. The text below is the proposal's own, unedited except
for this paragraph and the `## Index row` section: the argument is its author's and promotion is not
the moment to improve it. As filed: calef asked whether the roles in `AGENTS.md` should become skills,
and whether that would shrink the constitution. The answer worked out in conversation was no to
skills and yes to the underlying instinct: some of what `AGENTS.md` carries is only needed at a
specific, known task, not on every turn of every session, and `briefs/` (built for milestone 118's
still-open "split" piece) is already the right place to put that. This lane ran the test end to end
against the whole file and is reporting what it found, not building the briefs.

`AGENTS.md` cannot be edited by a developer lane, and neither can `briefs/` be
populated on this file's authority alone: calef decides whether to build the five briefs below, cut
the corresponding prose, and settle one open question about where the largest of the five belongs.
Nothing here blocks on hardware or on another milestone.

## Why this is a different question from milestone 118's budget

milestone 118 (CLAUDE.md has a budget, and the rules that get violated move up the ladder) already
asked "does a gate already enforce this," found the file is mostly not duplicate (a verified cut of
about 1.7%), and left one piece undone: the split into a core plus linked documents. This proposal is
that piece, run against a different test than 118 used.

118's test was mechanism: does a script already check this, so the prose is redundant. This
proposal's test is timing: does this rule have to already be in an agent's head, or can it be looked
up at the moment the task that needs it begins. The two tests overlap (the prune-worktree and
`nife-dev`-relink passages below, lines 508-517 and 587-597, turn out to already be duties a role
performs on a schedule) but they are not the same
question, and a passage can fail 118's test (no gate exists) while still passing this one (it is only
needed at one nameable task).

**The line count is not the argument.** `AGENTS.md` is loaded into every session and every lane
brief. Measured 2026-09-22, one maintainer session spent roughly 233M tokens against roughly 5M for
twenty developer lanes combined, dominated by carrying context rather than by thinking on any single
turn. That is a real cost and it is not what decided any row below. A row moved to `brief` because it
is genuinely task-triggered; a row that would save lines but is genuinely ambient stayed in the
file. Using the budget as the reason would be circular: it is exactly the temptation 118's own
"prefer gates over prose" clause had to correct for, extended to a different lever.

## The test

Does this passage need to fire when nobody is looking for it?

- **Yes → carry.** The rule has to already be in an agent's head at the moment it would otherwise be
  violated, because nothing prompts a lookup. Most of the file is this shape: the three principles,
  the ladder, the naming authority rule, the landmines.
- **No → brief.** The passage is consulted when performing a task an agent already knows it is
  doing: rebasing, opening a lane, cleaning up after a merge, writing a proposal for calef. The
  lookup happens because the task itself is the trigger.

**The bar for "when in doubt," stated as two sub-questions, both of which must hold before a passage
moves:**

1. **Is the trigger a specific, nameable, recurring task, not "any moment of any lane's work"?**
   `git stash` is unsafe at any point a lane reaches for it, for any reason; there is no task called
   "using git stash" that a lane would recognize itself as doing and go consult a brief for. That
   keeps it carry even though the passage reads like a procedure. Rebasing onto `main`, by contrast,
   is a task a lane names to itself before doing it, which is why `briefs/rebase-onto-main.md`
   already exists and works.
2. Is the failure mode already caught somewhere if the passage moves? Either the ambient trigger
   survives as a short carried stub ("after a QEMU session, verify ownership before you kill
   anything, see `briefs/...`"), or a role already carries the duty elsewhere in the file (the
   steward's "cleans up behind finished work" line already commits to worktree pruning happening; the
   mechanics of *how* can safely move).

If either sub-question is unclear, the row below is marked `uncertain`, not forced to a verdict, per
the brief for this lane.

## The table

Grouped by `AGENTS.md`'s own sections, in file order, with line ranges against the tree at this
lane's branch point (925 lines total). A verdict of `brief` names the provisional brief it would
join; `uncertain` says why the two sub-questions did not both resolve cleanly.

### Preamble and "What this project is" (lines 1-30)

| Lines | Opens with | Verdict | Why |
|---|---|---|---|
| 1-8 | *This file is `AGENTS.md`...* | carry | Identity of the file and the architect; every other row depends on a reader already having this. |
| 10-24 | *A capability microkernel for aarch64...* | carry | Project mission and calef's role (architect and reviewer). Shapes every judgment call, not one task. |
| 26-30 | *Default to autonomous execution.* | carry | Governs behavior on every task; nothing prompts a lookup of "how autonomous should I be." |

### Three principles (32-161)

| Lines | Opens with | Verdict | Why |
|---|---|---|---|
| 32-36 | *These are not aspirations.* | carry | Frames the three principles that follow; needed to read them. |
| 38-102 | Principle 1, the ranking function | carry | Prioritization ethos applied to every "what do I work on / how do I rank two milestones" judgment, which recurs constantly and unpredictably, not at one named task. |
| 104-134 | Principle 2, the method is a result | carry | Project self-understanding and the honesty-about-numbers rule it sets up; short enough that extracting it saves little and it is read alongside principle 1 and 3. |
| 136-161 | Principle 3, a newcomer must succeed alone | carry | Directly shapes documentation practice (`BUGS` sections, `EXAMPLES`) on every task that touches docs, and states this proposal's own test criterion in miniature ("a stranger, with only this repository"). |

### The mechanism ladder and two tenets (163-302)

| Lines | Opens with | Verdict | Why |
|---|---|---|---|
| 163-204 | *calef, 2026-08-04, after an evening...* the ladder | carry | Core judgment framework for how hard to make any new rule; consulted every time something new is built, which is not one task. |
| 206-250 | *We wouldn't be building this project out of convenience.* | carry | Governs every architectural recommendation; the "would I still choose this at equal cost" test must already be in head before the recommendation is made, not after. |
| 252-302 | *calef, 2026-08-05. The ladder above says how hard...* | carry | Reversibility framework (what is cheap to redo vs. what leaves the machine); applies to any decision at any moment, no single trigger. |

### The three roles (304-623)

| Lines | Opens with | Verdict | Why |
|---|---|---|---|
| 304-309 | *Named 2026-08-04, after a night...* | carry | Sets up the role vocabulary the rest of the file uses. |
| 310-326 | **Maintainer.** role definition | carry | Authority and identity a session needs to know it holds; not looked up, it is who you are for the session. |
| 327-343 | **Developer.** role, poll-to-completion, continue-until-blocked | carry | Behavioral rules for every turn of every lane (do not end a turn waiting on your own gate); this is exactly the shape the user's own standing memory rule ("poll my own background work") exists to enforce, which argues it must stay ambient. |
| 344-351 | *Every pull request and comment an agent writes opens by saying so.* | **uncertain** | The mechanics (one line, fixed wording) look brief-shaped and would fit naturally inside `claim-a-lane.md` for PR bodies. But the rule also binds plain **comments**, which are not created through any single scripted task, so moving it risks it surviving only where the draft-PR brief is read and silently dropping off ad hoc comments. |
| 352-360 | **A lane's first act is a draft pull request** §90 (the claim is a draft pull request) | brief | `claim-a-lane.md`. A specific, recognized task (starting a milestone) with a fixed sequence: empty commit, push, draft PR. This lane followed exactly this recipe from its own launch brief, typed out again rather than pointed to, which is itself evidence the recipe belongs in a file. |
| 361-363 | *A developer works in a lane*, the isolation definition | carry | Three-line vocabulary definition used throughout the rest of the file; too small and too load-bearing to extract. |
| 364-384 | **Steward.** role definition | carry | Authority and boundary of a role (does not brief developers, must never hold the main checkout mid-gate); the "must never" clause is a landmine (the race that took `nife-dev` out from under a lane), not a lookup. |
| 385-391 | **The top-up rule** | carry | Central behavioral rule for every maintainer turn ("launch the next thing before writing the report"); by construction this must already be a reflex, since the whole point is that it fires before anyone asks. |
| 392-395 | **A developer's final report ends by handing off** | carry | Short, applies at the end of every report; no separate task to consult it during. |
| 396-410 | **Identified work leaves the lane in a tracked form** + rung three | carry | Must fire the moment a lane notices untracked work, which happens unpredictably mid-task, not at a named task of its own. |
| 411-413 | **The merge checklist grows one line** | brief | `merge-and-cleanup.md`. One line added to a checklist a role already runs at a specific event (merging a lane); belongs beside the prune-worktree and `nife-dev`-relink passages below (lines 508-517, 587-597). |
| 414-418 | *Two things this deliberately does not do* | carry | Short scope clarification (does not gate, does not touch `BUGS`) read together with 396-410. |
| 419-426 | **Open decisions live in a file, not in a conversation.** | carry | Must fire the moment a decision surfaces in chat, which is unpredictable, not scheduled. |
| 427-445 | **And work waiting on calef carries its own label and its own ask** | **uncertain** | Recognizing *when* to hold something (outside standing authority: syscall surface, new dependency, a `DECISIONS` section owed) is judgment that has to be ambient. Writing the `## What I need from you` comment well, once that recognition has fired, is closer to a known task with a checkable shape (three properties). The passage is genuinely both; splitting it cleanly did not resolve. |
| 446-479 | **Lane count is set against the collision surface** | carry | Judgment framework for deciding how many lanes to run and which ones can collide; consulted at the moment of briefing lanes, which happens constantly and is not itself a single named task with a brief anyone would think to open. |
| 480-492 | **And there is a second ceiling**, memory / `VERIFY_JOBS` | carry, flagged landmine | Same shape as the file's other landmines: the failure (an OOM kill) presents as an unrelated timing failure or cancellation, so the rule has to be known before the job is launched, not looked up after a confusing failure. |
| 493-504 | **And a third ceiling**, disk, superseded, points to CI | carry | Short, mostly historical, and already models the pattern this proposal recommends: it points to `briefs/gate-in-ci.md` rather than duplicating it. Worth noting as a working precedent, not worth cutting further, since what remains is the landmine (disk destroys, not delays) rather than a procedure. |
| 505-507 | **The prover is the queue's long pole** | carry | Three lines, context for the ceilings above. |
| 508-517 | **Prune a lane's worktree the moment its pull request merges** | brief | `merge-and-cleanup.md`. Tied to one event (a pull request merging), performed by one role (maintainer or steward) that already commits to doing it (the steward role
definition above, lines 364-384). The failure mode is severe (this file records hitting zero bytes free twice), which argues for precision in a brief rather than for keeping it as prose nobody re-reads carefully. The safety clause ("if a lane is blocked, commit and push before removing anything") must survive verbatim into the brief. |
| 518-522 | **Two watchers run unattended on patagonia via `launchd`** | brief | `session-start.md`. A specific action at the start of a session (confirm or start two named processes), with a fixed check-and-start recipe. |
| 523-531 | **And a maintainer session checks what they already found** | brief | `session-start.md`, and this one is mostly already built: `briefs/survey-the-queue.md` already reads `mergeStateStatus` and `statusCheckRollup` and reports `DIRTY`/`CONFLICTING`/held pull requests. The prose here should shrink to "run `briefs/survey-the-queue.md` every session and treat what it finds as a task," not move wholesale; most of these nine lines duplicate a brief that exists. |
| 532-539 | *They exist because on 2026-08-04...* | carry | Historical rationale for why the watchers exist at all; short, and pairs with the role definitions rather than a task. |
| 540-545 | **Do not try to route this by requesting a review.** | carry, flagged landmine | `gh pr edit --add-reviewer` silently succeeds and does nothing; exactly the "reads like a procedure, is really a landmine" shape the brief for this lane named git stash as the example of. No task boundary contains the risk; anyone at any moment might reach for this. |
| 546-549 | **Stop and bring it to calef only when it is genuinely his call** | carry | The single most load-bearing judgment criterion in the roles section; must already be in head. |
| 550-553 | **Keep the documentation current** | carry | Short, applies to every task that produces a deliverable. |
| 554-570 | **The standard to aim at is FreeBSD's**, four things | **uncertain** | Reads as a natural "how to document a milestone" brief, and doc-writing is a task an agent recognizes itself as doing. But documentation here is not confined to milestones: notes, `design/decisions/` sections, `BUGS` entries and this very proposal are all "documentation" too, so the trigger is broader than one nameable task, and moving it risks losing the standard on exactly the ad hoc writing it should govern. |
| 571-573 | *The point is not the format, which is theirs.* | carry | Three-line close to 554-570; travels with it. |
| 574-586 | **Anything global to the tree is assigned by the integrator at merge, never claimed by a lane.** | carry | A scar-driven rule (three `design/decisions/` collisions in one day); must already be known before a lane reaches for a shared resource, which is unpredictable. This task's own briefing repeats "do not edit `design/decisions/`" for exactly this reason. |
| 587-597 | **Some shared state is global to the *machine*, not the repo**, `nife-dev` | brief | `merge-and-cleanup.md`. The relink command and when to run it is the integrator's task-bound duty at merge time, same event as the prune-worktree and merge-checklist passages above (lines 508-517,
411-413). One clause is genuinely ambient and should stay behind as a short carried sentence even after the mechanics move: "every lane will take `nife-dev`; that is expected, do not try to stop it." |
| 598-603 | **An unmerged branch is either abandoned or it is holding knowledge...** | carry | "Nobody reads branches" has to be known before a lane decides to leave a finding on a branch, which is a decision made at arbitrary points, not a scheduled task. |
| 604-608 | **Benchmarks and cross-OS comparisons are first-class.** | carry | Short, honesty-in-reporting rule applied whenever any number is stated. |
| 609-614 | **Push back when he's wrong** | carry | Short ethos rule for every disagreement, which arises unpredictably. |
| 615-619 | **Correct yourself loudly.** | carry | Short ethos rule. |
| 620-623 | **Explain on request, however basic.** | carry | Short ethos rule. |

### A fork reaches calef with its questions already answered (624-691)

| Lines | Opens with | Verdict | Why |
|---|---|---|---|
| 624-639 | *calef, 2026-08-18, sharpening his own rule...* | carry | Sets up why the seven questions exist; the recognition that "this is a fork and needs this treatment" must be ambient. |
| 641-673 | **The seven questions, because they are stable** | brief | A provisional `raise-a-fork-for-calef.md` (name and even the directory are open, see below). Writing up a fork for calef is a task a lane already recognizes itself as doing once the "stop and
bring it to calef" criterion above has fired (lines 546-549); the seven-question checklist is
exactly the kind of reference a lane would want spelled out rather than reconstructed from memory at
that moment. **Flagged open question**: `briefs/README.md` currently scopes that directory to "work that recurs, whose failure modes are already known, and whose correctness a gate can check... not design, not prose a reader will trust," aimed at rented mechanical-work models. This passage is prose a reader will trust, produced by whichever agent is presenting to calef, not mechanical work for a cheap model. It may belong in `briefs/` with that scope note widened by one sentence, or it may belong somewhere else (a `design/` reference doc). That choice is calef's, not this lane's. |
| 675-691 | **Two limits, so this does not become a tax** | carry | Judgment for *whether* to spin a lane or a research pass at all; must fire before the seven questions are even reached, so it has to be ambient rather than found by consulting something after the decision to look is already made. |

### Codebase rules and naming (693-782)

| Lines | Opens with | Verdict | Why |
|---|---|---|---|
| 693-747 | **The rules that hold the codebase together**, seven numbered rules | carry | Architectural constraints (arch code location, driver isolation, syscall surface, memory ordering, dependencies, the `#[path]` rule) that must already be known before a line of code is written in the wrong place; there is no "check architecture rules" task separate from writing code itself. |
| 748-772 | **calef names the crates, the programs, and the shared modules** | carry | Naming is on the file's own list of irreversible categories (the reversibility tenet, "move fast on what can be undone"); must fire the instant something new is created, which is constant and unscheduled. |
| 774-782 | *Milestone 7's process-model question is decided*, milestone 7 (user mode: EL0, capabilities, the ELF loader, and IPC) | carry | Short; defines what counts as a design fork on the syscall surface, needed the moment new syscall work is touched. |

### Testing, commits, dates, comments, style (783-878)

| Lines | Opens with | Verdict | Why |
|---|---|---|---|
| 783-798 | *`script/test` (a thin wrapper...)* | carry | Short definition plus the "tests should prove something specific" ethos, needed whenever any test is written. |
| 799-804 | *One purpose per commit.* | carry | Applies to every commit. |
| 805-817 | **Commit early and push, then curate before reporting.** | carry | Core practice for every lane's whole session, not one task; matches the user's own standing "push cadence" memory. |
| 818-824 | **Squash against the base commit you branched from, never against `origin/main`** | carry, landmine | Recorded scar (a lane staged four other lanes' files); must be known before *any* squash, which can happen at arbitrary points, not a scheduled task. |
| 825-836 | **`git stash` is unsafe in these worktrees** | carry, landmine | This is the passage this lane's brief named directly as the example of a procedure that is really a landmine; kept carry for exactly the stated reason: no task boundary contains the risk. |
| 837-847 | **Never squash across purposes.** | carry | Short, applies whenever committing. |
| 849-857 | *calef, 2026-09-13, closing a gap...* every date is UTC | carry | Applies to every dated line written anywhere in the tree; far too broad a trigger to confine to one task. |
| 858-867 | *The kernel is commented far more heavily...* | carry | Applies whenever writing a comment. |
| 868-878 | *calef's global preferences apply...* Style | carry | Applies to all prose written in this tree; short. |

### QEMU (879-925)

| Lines | Opens with | Verdict | Why |
|---|---|---|---|
| 879-891 | **Never leave QEMU running**, `wfi` not `wfe` | carry, landmine | A halted kernel using the wrong instruction burns 99.7% of a host core; must be known before the code is written, not after the burn is noticed. |
| 892-898 | Environment facts (macOS, QEMU via Homebrew, Rust nightly, target triple) | carry | Five lines of reference facts; too small to be worth the overhead of a separate file. |
| 899-906 | **`perl -e 'alarm N; exec @ARGV'` DOES NOT WORK ON QEMU.** | carry, landmine | Leaked eleven QEMU processes burning 729% CPU in one day when this was not known ahead of time. Must be ambient. |
| 908 | **After any session that ran QEMU, check `pgrep -x qemu-system-aarch64` and clean up.** | carry | The one-line ambient trigger that must survive on its own even after the hunting-recipe passage below moves; see the caveat under "what could go wrong." |
| 910-925 | *That check is not sufficient after you kill a harness...* the hunting recipe | brief | `hunt-leaked-qemu.md`. Once the trigger at line 908 has fired, this is a specific, bounded task (find and safely kill leaked QEMU) with a precise recipe: `lsof` the image file, walk `ps -o pid,ppid` in both directions before killing anything, kill the tree at its root. Exactly `briefs/triage-a-failing-check.md`'s shape. |

## Totals

Classified: **909 of 925 lines** (the remainder is section headers and blank separators, not separately tallied).

| Verdict | Lines | Share |
|---|---|---|
| carry | 769 | 85% |
| brief (five candidates) | 96 | 10% |
| uncertain (three passages) | 44 | 5% |

**This is worth doing, but it is a targeted extraction, not a rewrite, and it should be described to
calef that way.** Ten percent is not nothing at roughly 11,000 tokens a session, but the honest
headline is the 85%: most of `AGENTS.md`, examined passage by passage against "does this need to
fire when nobody is looking for it," says yes. That matches milestone 118's own finding on a
different axis (the file is not mostly unmechanised duplicate prose either); this proposal extends
the same conclusion along the timing axis instead of the mechanism axis, and gets a similarly modest
number. **Recommendation: do the five `brief` extractions, because two of them are nearly free** (one
mostly already exists as `briefs/survey-the-queue.md`, one is this lane's own launch procedure typed
out again rather than pointed to) **and the other three match the shape of briefs already in the
directory.** Do not force the three `uncertain` passages into briefs without calef's call; each has a
genuine reason the split did not resolve cleanly, listed in the table.

## The five new briefs (provisional names)

1. **`claim-a-lane.md`** (lines 352-360, 9 lines). Cut the branch, make one empty commit
   (`git commit --allow-empty -m "claim: milestone N"`), push it, open a draft pull request before
   any work. This lane's own launch instructions restated exactly this recipe inline, which is itself
   the evidence it belongs in a file rather than being retyped into every brief that starts a lane.

2. **`merge-and-cleanup.md`** (lines 411-413, 508-517, 587-597, roughly 24 lines). What the
   maintainer or steward does the moment a pull request merges: delete the branch (already automatic,
   `delete_branch_on_merge`), prune the worktree, relink `nife-dev` from the main checkout with the
   exact command, and check off any identified work from the lane's report against a home. Carries
   forward verbatim the safety clause that a blocked lane's work must be committed and pushed before
   anything is removed, since this is the one failure mode in the whole system that destroys rather
   than delays.

3. **`session-start.md`** (lines 518-531, roughly 14 lines). Confirm the two `launchd` watchers are
   alive (`launchctl list | grep nife`) and start them if not; then run `briefs/survey-the-queue.md`
   (already built) and treat what it reports as a standing task, not a status check to skim.

4. **`hunt-leaked-qemu.md`** (lines 910-925, 16 lines). Find a leaked `qemu-system-aarch64` (or
   `-riscv64`, `-x86_64`) process and kill it safely: `lsof` the image file it holds, walk the process
   tree in both directions (a QEMU whose ancestry ends in a live harness is someone's gate in flight,
   not a leak), kill the tree at its root rather than the leaf.

5. **`raise-a-fork-for-calef.md`** (provisional name, provisional location; lines 641-673, 33 lines).
   The seven questions a fork must answer before it reaches calef, and the tell that one has not been
   answered (reaching for an adjective where a command would do). See the open question in the table
   about whether this belongs in `briefs/` at all, since it is judgment-shaped prose rather than the
   mechanical, gate-checkable work `briefs/README.md` currently scopes that directory to.

## What could go wrong

The QEMU-hunting cut is the one most likely to be forgotten, and it is the highest-consequence
one. If line 908's carried stub stays exactly as written ("check `pgrep` and clean up") and the
safe-killing mechanics move out wholesale, a lane that has not internalized "verify ownership before
killing" from memory alone might `pkill` on sight, which is the precise failure already recorded
here: a maintainer killed a lane's mid-suite emulator this way and that lane's run failed for a
reason no one could see from inside it. **Mitigation, to apply at cut time and not skip**: reword the
carried stub to compress the landmine into one clause, e.g. "check `pgrep`, but do not kill on sight;
walk the process tree in both directions first, see `briefs/hunt-leaked-qemu.md`," so the warning
survives even for a reader who never opens the brief.

`merge-and-cleanup.md` carries the file's worst recorded failure (zero bytes free, two lanes died
mid-work), but the risk here is lower than the QEMU one, because the ambient duty already survives
in the carried steward role definition ("cleans up behind finished work"), which commits to the
outcome happening even if the exact commands are only in the brief. What would catch a lapse: disk
pressure is visible (a failing write, a full volume), unlike the QEMU case, which fails silently as
someone else's confusing test failure.

**`raise-a-fork-for-calef.md` has no mechanical catch at all, same as today.** A degraded fork
(options with no cost, no prior-art check) reaching calef is caught only by calef noticing and
pushing back, which is exactly how this section of `AGENTS.md` came to be written in the first place
(2026-08-18, a maintainer asked seven questions calef had to ask him). Moving the checklist to a
brief does not make this worse than it already is, and does not make it better either; nothing in
this proposal changes that this is judgment, not a gate.

## The refusal of skills, for the record

calef asked whether the roles should become Claude Code skills, loaded on demand from
`.claude/skills/`. The answer is no, for three reasons that were given in conversation and belong in
the tree rather than only in a chat log:

1. **`AGENTS.md` is deliberately the cross-tool convention.**
   [`design/tenets/project-history.md`](../tenets/project-history.md) records that `CLAUDE.md` is a
   symlink to it for exactly this reason: the file addresses any competent agent, not one vendor's
   CLI. `.claude/skills/` is Claude Code's own format, invisible to any other tool that might read
   this tree.
2. **Principle 3 fails outright.** The test this file sets for itself is "could a competent stranger,
   with only this repository, get to a passing build and a correct mental model without opening a
   chat window." A skill that loads only inside one CLI, on a trigger phrase that stranger cannot see
   in advance, is invisible to them by construction.
3. **On-demand loading is the wrong direction for anything that must fire unprompted, which is most
   of the carry column above.** A skill is consulted when an agent (or the harness) decides the
   moment calls for it; the whole problem this proposal is solving for the `brief` column is
   "consulted at a known task," which is a solved problem already, in `briefs/`, in plain Markdown,
   read by any tool that can read a file. Nothing about the `uncertain` or `carry` rows above would
   be improved by putting them behind a trigger phrase; several of them (the landmines) are precisely
   the rows that must fire when nobody thought to ask.

`briefs/` already does what a skill would be asked to do here, without the vendor lock-in and without
weakening principle 3, and five candidates above are ready to populate it further.

## Index row

Classifies every passage of `AGENTS.md` by whether an agent must already be carrying it or can look
it up when the task arrives, and finds five briefs' worth of prose that fails the carry test. It is
the still-open "split" half of milestone 118 (CLAUDE.md has a budget, and the rules that get
violated move up the ladder), run against a timing test rather than 118's duplication test, and it
is blocked on calef because neither `AGENTS.md` nor `briefs/` is a developer lane's to edit.
