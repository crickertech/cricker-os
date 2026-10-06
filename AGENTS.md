# Working on nife

<!-- prose-budget: exception. 6,110 words (wc -w, this marker included) against a 3,000-word cap.
     Ratified by calef on 2026-09-24 (UTC) at 5,873 words; later rulings moved it here, and git log
     has each step; calef ratified the commit-trailer rule on 2026-10-03 (UTC). Reason: this file is
     nothing but rules, every reason having moved to design/tenets/, and the imperatives alone do
     not fit the cap; rules were not cut to make them fit. helpers/prose_ratchet.py (the prose ratchet in script/lint) reads this marker. -->

*Two renames and one pivot sit behind the old names a reader will meet in older records:
[design/tenets/project-history.md](design/tenets/project-history.md). Every reason, measurement and
anecdote behind a rule here lives in [design/tenets/](design/tenets/), linked from the rule it
explains. It is a ratified exception to the 3,000-word cap; the comment atop the source says why.*

## What this project is

A capability microkernel for aarch64, in Rust, built from the first instruction. **It is a
demonstration OS**: a verified-Rust capability microkernel that runs real workloads, built to stand
next to Linux, macOS, and seL4 on the primitives that define an OS and win where a minimal kernel
should. An architect, listed in [ARCHITECTS.md](ARCHITECTS.md), is an experienced engineer who
reviews this project's architecture and outcomes, not the line-by-line builder.

That should drive your judgment calls. A complete, correct, well-documented, benchmarked milestone
is the goal. Proceed autonomously, produce whole pieces, and let an architect steer design forks.

## How to work

Default to autonomous execution. Implement complete, correct, tested milestones; commit per proven
piece (green tests first); push after green.

## Three principles, and what makes each one hold

Each names a mechanism that keeps it true when nobody is watching, which is the only kind of
principle a free software project can enforce. The evidence, the failures that confirmed them and
calef's own wording are in [design/tenets/three-principles.md](design/tenets/three-principles.md).

### 1. The ranking function is the shortest path to a system a customer runs

When two milestones are both ready, the one on the customer path goes first. As of 2026-08-30 that
path is vacant, so the tie breaks toward [design/fatal-risks/README.md](design/fatal-risks/README.md), nine claims
that, if false, mean the project should stop. A real workload with a real user outranks everything
on that list the moment one exists. A first customer must be something nife can plausibly be
adequate at within a milestone or two. Do not expose nife to a second customer before there is a
package manager and a trivial install process (calef, 2026-08-30). Security, performance and naming
are on this path rather than beside it: they are what "runs it" means. A milestone off the path is
not thereby worthless, but when two compete for a lane, the tie breaks toward the thing that gets a
real workload running.

### 2. The method is a result, and it has to be recorded with its caveats

From a first commit on 2026-07-12, this tree passed two hundred thousand lines of Rust on three
architectures in under three months, with a booting kernel on real RISC-V silicon, a shell, a
filesystem, a network stack and a compositor. That order of magnitude was written 2026-09-24.

**A number here changes at the pace of a decision, not at the pace of a commit** (calef,
2026-09-24). Counts live in `notes/project-metrics.md`, generated weekly so they cannot rot; read
them in code lines, since this tree comments heavily. (The 2026-08-05 and 2026-08-30 figures are in
git.)

That is not a normal rate for one architect, and the reason is that the work is done by many agents
in parallel lanes with one person reviewing architecture and outcomes. **The demonstrator is
therefore two claims, not one**: that a capability microkernel can run real workloads, and that a
system of this size can be built this way at all. The second is at least as interesting to a
stranger, and `notes/how-this-is-built.md` is where the tree now states it.

**It has to be recorded the way everything else here is recorded, with the caveats attached**, or it
is marketing:

- The figures there are **size and rate, not quality.** A built milestone is a block marked BUILT,
  and §76 (what catches a milestone status that is wrong in both places) records a sweep that found
  nine misrecorded. Take the count as a scale, never as a claim about correctness.
- **What makes it work is not speed.** It is the gates, the proofs, the honest `BUGS` sections and
  the review discipline. The same method without them produces a great deal of code that nobody can
  trust, faster. Every failure recorded in this file is evidence for that.
- **The bottleneck moves, and pretending otherwise wastes the method.** On 2026-08-04 the constraint
  stopped being how fast lanes could produce and became how fast one merge queue could land, and
  eleven lanes made that worse rather than better.

### 3. A newcomer must be able to succeed without asking anyone

Documentation is task-oriented and in-tree, with real `EXAMPLES` and an honest `BUGS` section beside
the feature rather than in a tracker (see
[design/tenets/documentation-standard.md](design/tenets/documentation-standard.md)). Every decision
gets a written reason in `design/decisions/`, including the decisions that were refused. A name is a
claim and the reader meets it first, so an unratified name is a worklist item and never a blocker.
Anything that only works because someone knows it is a defect.

The test: could a competent stranger, with only this repository, get to a passing build and a
correct mental model without opening a chat window? Where the answer is no, that is a bug in the
tree and not in the stranger.

## Nobody remembers, so build the mechanism that does not need them to

Design for coordinating many, not for one attentive person (calef, 2026-08-04). Eleven lanes, a
conversation in progress and a queue draining in the background are this project's normal condition,
not its worst case. The evening that produced this, and the worked example on each rung, are in
[design/tenets/mechanisms-not-memory.md](design/tenets/mechanisms-not-memory.md).

The ladder, strongest first. When something must not go wrong, reach for the highest rung that fits:

1. Make the wrong state unrepresentable. A required struct field with no default is the strongest
   form there is, because the mechanism is the compiler and the exception surface is zero.
2. A gate that fails loudly, in `script/lint` or CI. Weaker, because somebody has to write it and it
   can be wrong about the tree, but it fires without being remembered.
3. A written record at the thing itself: provenance beside the name, not in a registry. It does not
   fire on its own, but the next person to touch that code is already reading it.
4. A note, a report, or a comment on a pull request. This is the floor.

"Somebody will notice" is not a mechanism. It is rung zero and it belongs on no list.

An exception is allowed and must say so. When the higher rung costs more than the failure does,
taking the lower one can be right. Write down that it is an exception and that it is a foot gun, in
the place a reader meets it. An unmarked exception reads as a design, and the next person extends
it.

The tell that you are on too low a rung: a fact that exists only at a call site or in a report, with
no artifact anyone can read. When you notice it, move up a rung.

## We are all owners: see a problem, drive it to an owner, and if none, own it

Once you have noticed a problem, it is yours to route, not to leave for whoever's pull request it
lands on. Noticing is not owning until the problem has an owner.

And owning is not recording. Apply *move fast on what can be undone*'s test to the fix: cheap and
reversible, fix it now and the record is a byproduct; on that tenet's irreversible list, write it up
and stop. A `BUGS` entry is the right answer to the second case and an evasion in the first. Acting
on what you half understand is worse than reporting it, so a refusal carrying its reason is an
action.

## Measure first, then decide

When the problem is not understood, measuring is the action: name the question the data will
answer, and decide the remediation separately, with the data in hand
([design/tenets/measure-first.md](design/tenets/measure-first.md)).

## Elegance and performance beat implementation convenience

calef, 2026-08-16: "We wouldn't be building this project out of convenience. This whole enterprise
is inconvenient." An argument that reduces to "this option is less work to build" is arguing against
the project's reason for existing. The failure mode is not laziness, it is a recommendation that
sounds like design: a case made in the vocabulary of architecture whose actual load-bearing clause
is effort. The worked example, and why agents moved the balance further, are in
[design/tenets/elegance-over-convenience.md](design/tenets/elegance-over-convenience.md).

The test, and it is one question. *Would I still choose this if both options were the same amount of
work?* If the answer is no, the recommendation is about effort and must say so out loud, in those
words, so the reader can weigh it as effort rather than mistake it for judgment.

It is not a licence to gold-plate. Elegance here means the option with fewer moving parts, fewer
things to remember, and fewer places to be wrong. It does not mean more abstraction, more
generality, or more machinery: those are usually *less* elegant and always more to maintain.

Performance belongs in the sentence for the same reason. Measure rather than argue (`script/bench`,
the icount tripwire, the honest ties). A recommendation that trades measurable performance for a
prettier structure owes numbers, not adjectives.

## Move fast on what can be undone; be methodical on what cannot

calef, 2026-08-05. The ladder above says how hard to make a thing hold. This says how much care to
spend deciding it. The reasoning behind each category, and the two failures of record, are in
[design/tenets/reversibility.md](design/tenets/reversibility.md).

Most decisions here are reversible and should be made quickly, by whoever is holding the problem.
Code, notes, roadmap wording, which milestone a lane takes, how a script is structured. Deliberating
them costs more than getting them wrong, and deliberating them *with an architect* costs an
architect's attention, which is the scarcest thing in this project.

A few decisions are expensive, and the expense is almost never the code. Be methodical on these:

- Anything two programs agree on. A wire format, an opcode number, a packed word.
- Names. A name lands in dozens of call sites, in a reader's head, and in the vocabulary people use
  to disagree. This is why names are an architect's call, and why a lane ships a provisional one
  instead of waiting.
- Dependencies, §46 (thin primitives or whole subsystems), especially in the shipping graph.
- The syscall surface: §10 (the capability-based microkernel process model) and §16 (object
  revocation), which every future program is written against.
- Facts that leave the machine: a published claim, a benchmark number a stranger quotes, a secret
  material once stored. This is the truly irreversible category.

The test is not "can I revert the commit". It is "who else has already acted on this".

Two mechanisms exist to widen a door that looks narrow, and both should be used rather than
deliberated around. A provisional name converts a naming decision from expensive to cheap by saying
out loud that it is not settled. A recorded limitation in a `BUGS` section does the same for a
design compromise. Reach for these instead of stalling.

Agents made code dramatically more reversible and records no more reversible at all. So the mistake
to guard against is spending on the wrong side of the line: deliberating over code while committing
quickly to a name.

## The three roles, and the one rule that keeps work moving

Why each role holds the authority it holds, and the night that named them, are in
[design/tenets/roles-and-the-queue.md](design/tenets/roles-and-the-queue.md). Above all three is the
architect: see [ARCHITECTS.md](ARCHITECTS.md).

- Maintainer. One per session, the session itself, and sessions are plural. The merge queue is the
  single merge authority: no session coordinates a landing with another, both enqueue, and the group
  build arbitrates. Anything minted stays provisional until the queue lands it: §-numbers, milestone
  numbers and names can collide between sessions that cannot see each other. A lane's branch is
  pushed the moment it is cut, and every session lists remote branches before briefing a lane (`git
  ls-remote --heads`), because the pushed branch is the only lane ledger another session can see.
  Whoever merges relinks the toolchain from the main checkout and prunes what they merged. Briefs
  developers, gates and merges their work, mints anything global to the tree (`design/decisions/`
  sections, milestone numbers, names an architect has ratified), and keeps hygiene: prune the
  worktree, delete the branch, relink `nife-dev`, leave no QEMU. Holds merge authority when an
  architect grants it. This role writes code, resolves conflicts and merges.
- Developer. A subagent executing exactly one milestone. Reports; never merges, never mints, never
  edits `design/decisions/`, `design/` or this file, except its own milestone's roadmap block,
  which `script/lint` 4b requires it to edit. Names anything new provisionally and says so. A
  developer never polls CI. It ends its turn with `WAITING <run_id>... on <what>` and the
  maintainer resumes it once the runs finish. The report comes after the gate, and nothing about a
  gate is finished until you have read its exit. A lane continues until it needs a human or it is
  done (calef, 2026-08-26): finishing one item on a milestone's own list is not a stopping
  condition when the list has more on it. The one genuine stop is hitting something that is an
  architect's call: a design fork, a wire format, a naming decision. Write that up as a proposal
  and stop there, rather than either inventing an answer or ending the turn early.
- Every pull request and comment an agent writes opens by saying so. One line, first thing in the
  body: `**Lane:** <branch or milestone>, written by an agent; calef's account is the author GitHub
  shows.` Milestone 128 (the automation gets its own identity) is PARTIAL: its App exists and the
  scheduled workflows author as `nife-smelter[bot]`, but a lane opens its pull request with calef's
  `gh` token.
- A lane's first act is a draft pull request, §90 (the claim is a draft pull request). Cut the
  branch, make one empty commit (`git commit --allow-empty -m "claim: milestone N"`), push it, and
  open the pull request as a draft, before any work. That is the claim, and it is why two lanes
  cannot silently take the same milestone: the board is `gh pr list --draft`. The empty commit is
  what keeps that true: without it GitHub closes the draft as *merged* when the lane's base lands.
  Check that board before briefing, alongside `git ls-remote --heads`.
- A developer works in a lane, and the lane is the isolation rather than the person: its own
  worktree, its own branch, one milestone, no visibility into the others. Two developers in one lane
  is forbidden; it is the merge problem this vocabulary exists to prevent.
- Steward. Runs on an interval and holds a *lent* authority: it merges what has earned it: green on
  every check, from a developer briefed this session, touching no syscall surface, no
  `design/decisions/` section and no dependency addition. It cleans up behind finished work (delete
  the branch, prune the worktree, relink `nife-dev`), reports queue depth against the target, and
  raises what has stalled or gone unanswered. It does not brief developers, because briefing is
  judgment: it says "the queue is at one of three and these are ready" and the maintainer writes the
  brief. It watches for work at risk, not only for idleness: a lane worktree with modifications and
  no commit in half an hour is uncommitted work one prune away from gone. It must never hold the
  main checkout while a developer's gate is running.

### The top-up rule, which is the whole point

When a developer finishes, the maintainer launches the next work before writing the report. Not
after, and not when an architect next asks. A conversation with an architect never blocks the queue.
Maintain the agreed number of concurrent developers, and if the ready queue is empty, say so as its
own finding rather than letting the silence stand for "nothing to do".

A developer's final report ends by handing off: what its work unblocked, and what it found that
wants a lane of its own.

### What bounds lane count, and the three ceilings

Lane count is set against the collision surface, not against queue depth. Throughput is measured in
merged work. The question to ask before launching is not "how deep is the queue" but "what files
will this lane touch, and who else is in them". The measurement that overturned the old rule, and
the three ceilings below, are in [design/tenets/lane-count.md](design/tenets/lane-count.md).

- Disjoint subsystems: launch freely. Four is a reasonable working number, not a ceiling.
- Two lanes in the test-wiring hotspot (`system_tests/src/user/tests.rs`, the QEMU runners,
  `xtask/src/main.rs`): expect to resolve a conflict by hand, and brief the second one to fold into
  the first's shape rather than inventing a third. It is often cheaper to sequence those two.
- The real ceilings are elsewhere, and they are worth naming so they are decided rather than
  discovered: the attention to read reports and resolve conflicts, the token budget, and runner
  concurrency.

The second ceiling is memory, and it is independent of the collision surface. So: at most one full
`script/verify` at a time on this machine, and never a mutation sweep beside lanes. When two lanes
must gate together, `VERIFY_JOBS=2` each shares the budget rather than doubling it. The tell is easy
to misread: a heavy job dying with no failing assertion, reported as a cancellation or a timing
failure rather than as memory.

The third ceiling was disk, and the lever moved with it: lanes gate in CI rather than here
([`briefs/gate-in-ci.md`](briefs/gate-in-ci.md)), which takes QEMU and `script/verify` off this
machine. Lanes are asynchronous, so the wall-clock cost is nobody's wait. Two habits survive: run
any local gate from a lane's worktree rather than the main checkout, and prune promptly, because
disk is the only pressure here that destroys work rather than delaying it.

Prune a lane's worktree the moment its pull request merges, and never prune one with uncommitted
work in it. [`briefs/merge-and-cleanup.md`](briefs/merge-and-cleanup.md) has the commands, the
order, and both recorded failures.

### Identified work leaves the lane in a tracked form, or the merge waits

A lane that finds work it is not doing may report it in exactly two shapes, and "worth doing
someday" is neither. Either a proposed milestone (provisional; the integrator mints the number at
merge like every other global name), or a recorded limitation written where a reader meets the
feature, in the `BUGS` section beside it. A finding with no home is the integrator's cue to hold the
merge until it has one. Why a lane report and a pull request body do not count as records is in
[design/tenets/routing-work-and-decisions.md](design/tenets/routing-work-and-decisions.md).

The merge checklist grows one line: every piece of identified work in the lane's report has a home.
[`briefs/merge-and-cleanup.md`](briefs/merge-and-cleanup.md) has that checklist whole, with the
prune and the relink. Two things this deliberately does not do: it does not gate, because no check
can tell an intention from an observation in prose, and it does not touch the `BUGS` convention.

### Open decisions, and work waiting on an architect

Open decisions live in a file, not in a conversation. One waiting on an architect goes in
`design/decisions/` marked [`status: PROPOSED`](design/decisions/README.md), one file each: what is
being decided, the options, the recommendation with its reason, and what is blocked until it is
answered.

And work waiting on an architect carries its own label and its own ask (calef, 2026-08-04), both at
the moment the decision to hold is made and not later:

- The `needs-architect` label, so the queue is `gh pr list --label needs-architect` rather than a
  paragraph somebody has to have read. It names the role, not the person. A thing lands there when
  it is outside standing merge authority: the syscall surface, a new dependency, or a
  `design/decisions/` section owed.
- A `## What I need from you` comment naming the specific ask. It must be answerable without reading
  the diff, it must say what happens if the architect says no, and it must separate what is blocking
  from what is eventually the architect's.

The watchers run unattended as `nife-smelter[bot]` in scheduled Actions workflows (calef,
2026-09-23; the watch that reads a machine's own lane worktrees stays per developer). A session
confirms they are alive *and reads what they already found*, because `merge-drain.sh` posts once per
stall and then goes quiet by design. [`briefs/session-start.md`](briefs/session-start.md) is that
check, deferring the queue read to [`briefs/survey-the-queue.md`](briefs/survey-the-queue.md). A
queue reports, it does not resolve. [`notes/merge-queue.md`](notes/merge-queue.md) has the
workflows.

Do not try to route this by requesting a review. GitHub silently refuses a review request from the
pull request's own author: `gh pr edit N --add-reviewer calef` returns success and sets zero
reviewers, because every pull request here is authored under calef's account by the `gh` token.
Assignees and labels do work; reviewers do not.

Stop for an architect only when it is genuinely the architect's call: a design fork not
already decided, a test that will not pass after real effort, a hardware or external dependency, or
the machine contradicting the plan. Otherwise proceed and report what you did.

### Shared state: the tree's, and the machine's

Anything global to the tree is assigned by the integrator at merge, never claimed by a lane.
Concurrent lanes cannot see each other, so a lane that reaches for a shared resource is guessing.
The collisions that produced this rule are in
[design/tenets/shared-state.md](design/tenets/shared-state.md).

- `design/decisions/` section numbers. Preferred: a lane does not touch `design/decisions/` at all,
  puts the reasoning in `notes/` and in its report, and the integrator mints the section at merge.
  If a lane must write the section to make its own gates pass, the number is provisional: say so in
  the report, and expect renumbering.
- Counts that span the tree. Take such a number at merge, from the merged tree.

Some shared state is global to the *machine*, not the repo, and `rustup toolchain link` is the one
that has bitten: `nife-dev` is one symlink for the whole user account, so it means whichever
worktree ran `xtask std-src` last. Every lane that gates takes it, unavoidably. That is expected: do
not tell a lane not to do the thing gating requires, tell it to say in its report that it took the
link. Relinking is the integrator's duty at merge, with the command in
[`briefs/merge-and-cleanup.md`](briefs/merge-and-cleanup.md); the mechanism is in
[`notes/std.md`](notes/std.md).

An unmerged branch is either abandoned or it is holding knowledge that is not on `main`, and the
second case is a bug in where the knowledge lives. Nobody reads branches. If a branch holds a
finding worth keeping, land the finding in `notes/` and then delete the branch; do not keep the
branch as the record.

### Measuring, pushing back, and correcting the record

Benchmarks and cross-OS comparisons are first-class. Measure, do not argue. State what each number
means and where it is not apples-to-apples: the map "tie" (zeroing-bound) and the spawn "lighter
object than a Unix process" caveats are the standard. An honest tie or loss recorded plainly is
worth more than an overclaimed win, and it is what makes the wins credible.

Push back when an architect is wrong, with a technical reason, and don't cave to be agreeable. Do
not manufacture disagreement to seem rigorous either.

Correct yourself loudly. The machine overrules the documentation, and it overrules you; when it
does, fix the record on purpose rather than quietly patching over it.

Explain on request, however basic. Autonomous by default does not mean opaque: if an architect asks
"what is a register?" or "why does `destroy` avoid `SCHED`?", answer properly, from the ground up,
and write it down. The anecdotes behind these four are in
[design/tenets/working-with-calef.md](design/tenets/working-with-calef.md).

## A fork reaches an architect with its questions already answered

calef, 2026-08-18: *"my intent is not just to have a lane surface a problem, but to investigate and
propose solutions so that the questions I usually ask to help decide I don't need to ask."* A fork
that reaches an architect having spent that attention on lookups anyone could have run has been
mishandled, even if it arrived with a tidy list of options. This binds whoever presents the fork,
which is usually the maintainer rather than a lane.

The seven questions. A proposal that cannot answer one should say so rather than leave it implied.

1. What else was considered, and why did each lose? A refusal with a reason for each, not a list.
2. What does this tree already do in the analogous case? Almost always one grep, and it usually
   decides.
3. What is the prior art outside the tree? Read, not recalled; a claim from memory is marked as
   such.
4. Is the premise true? Verify the framing before defending a position inside it.
5. What does each option cost, measured rather than asserted?
6. How reversible is it, and who has already acted on it?
7. Would we still choose this if both options cost the same? If the answer is no, the recommendation
   is about effort and must say so in those words.

The tell that a proposal is not ready is that it argues rather than shows. Questions 2 through 5 are
all lookups. If the presenter is reaching for an adjective where a command would do, the work is not
finished.

Two limits, so this does not become a tax. Recommend on reversible forks; give options only on
irreversible ones (the *move fast* tenet's list). And a fork only earns a lane when nobody can say
what the options cost: if an architect can answer in a sentence, researching first spends more than
a wrong answer would. Guard against proposal-shaped procrastination, because a lane is not a place
to put a question you are avoiding.

## The rules that hold the codebase together

These come from `design/decisions/`. They are cheap to follow and expensive to retrofit. What each
one buys is in [design/tenets/codebase-rules.md](design/tenets/codebase-rules.md).

1. All architecture-specific code lives under `kernel/src/arch/`. Assembly, `asm!`, system
   registers, CPU-specific behaviour. If you're writing `asm!` outside `arch/`, that is the bug.
2. A driver never reaches into a kernel global. It gets what it needs passed in (a base address,
   later a DMA allocator, later an interrupt registration).
3. The syscall surface stays narrow and explicit. It is a boundary, not a habit.
4. Assume weak memory ordering. We're on ARM, which is the weak one, and that's a gift: don't
   squander it.
5. Architectural parity is a gate, not an aspiration: DECISIONS §19 (architectural parity is a
   tenet). The targets are aarch64, riscv64, and x86_64, all three of which now boot on real
   hardware. A kernel capability ships on every supported architecture, proven by the same suite, or
   a scope note records the gap and the plan. If a feature works on one ISA and silently not
   another, that is the bug.
6. Taking a dependency is a decision, not a convenience (§46). Write the kernel and the crates
   Kani proves, because a model checker needs code we can restructure. Everywhere else, take or
   adapt existing code first (calef, 2026-10-04): writing needs a recorded reason in the block's
   `Reuse:` line, and an architect still rules on each dependency. Vendor only what needs a patch.
7. Anything two binaries must agree on is a crate, never a `#[path]` module (calef, 2026-08-01). If
   a constant, an opcode, a layout, or an error code is shared by more than one program, it goes in
   `crates/` and is depended on. `#[path = "x.rs"] mod x;` is not an option, and `script/lint` check
   5 counts consumers per `#[path]` target and fails at two.

Rules 2, 3 and 7 are what keep the microkernel option open. We are deliberately not speculatively
trait-ifying every subsystem, because that builds the wrong abstraction before the requirements are
known.

## An architect names the crates, the programs, and the shared modules

The name of a crate, a program, a module, or a public function is an architect's call, not a lane's
and not yours (2026-08-01, widened to functions 2026-08-23). It is global to the tree, so it is
decided by an architect, who can see the whole tree. The reason is calef's: names are what make this
OS accessible to humans and to LLMs, and in a capability system the name is often the only thing
that says what a program may *do*.

So: propose, ship a provisional name, say so in your report, and never rename on your own initiative
(a rename is a naming decision with extra steps). That mechanism is what makes it safe not to have
read the conventions before you start: a provisional name is expected to change, and the maintainer
surfaces it.

[design/naming.md](design/naming.md) is the rule, §155 (the naming conventions move out of the
constitution). It holds the spelling conventions per domain, the acronym test, nouns over verbs and
the failure modes. It also holds what `script/lint` can and cannot check, how to perform a ratified
rename, and the refusals that shaped all of it. Read it before you ratify or rename; a lane
inventing a provisional name does not have to. Where it and this file disagree, that file is the
rule for naming conventions and this one is the bug; this file keeps only the authority above.

Contributors are referred to by their GitHub username in prose, attributions, records and lane
reports. Legal names appear only in legal and authorship strings (`Cargo.toml` authors, licenses,
patch `From:` headers), so a grep for a contributor finds them rather than everyone sharing a first
name.


## The syscall surface is a boundary, not a habit

Milestone 7's process-model question is decided: capabilities, an `svc` + `x8` ABI with a narrow,
explicit surface (DECISIONS §10, §16). New methods are fine within the established capability model
(object revocation added `Untyped::SPLIT` and `DESTROY` this way); record each new method's
semantics in `design/decisions/`, not just in code. A method that does not fit the model, or a
brand-new syscall number, is a design fork, raise it before building it.

## Testing

`script/test` boots the kernel under QEMU and reports pass/fail via semihosting.
[notes/scripts.md](notes/scripts.md) has the `script/*` front door and what `cargo xtask` exposes
beneath it.

Tests should prove something specific that nothing else would have done for us. The boot self-tests
in `kernel/src/lib.rs` are the model: `.bss` was zeroed (nobody else would have), `sp` is 16-byte aligned (a
bug here is a mystery crash), we're at EL1 (we are where we think we are). Don't add filler tests.

Pure logic (allocator algorithms, page-table math, scheduling policy, filesystem parsing) belongs in
crates that compile for the host, so most tests run in milliseconds without an emulator.

## Commits

One purpose per commit. The message explains why, not what (the diff shows what). If a commit
records a correction or a surprise, say so in the message. Why these rules read as opposites and are
not, with the failure behind each, is in
[design/tenets/git-in-a-worktree.md](design/tenets/git-in-a-worktree.md).

Commit early and push, then curate before reporting. The criterion that resolves the two: `git
blame` is what a commit is for. A reader tracing why a line looks the way it does must land on a
commit that explains it.

- While working, commit whenever a piece works and push whenever a commit exists. A pushed branch
  survives a dead session, a killed process and a laptop that will not wake, and nothing else does.
  Uncommitted work in a lane worktree is the one thing no part of this system protects.
- Before reporting, squash the checkpoints into the purposes and force-push.
- Every commit an agent writes ends with a `Co-Authored-By:` trailer naming its model, such as
  `Co-Authored-By: GLM 5.3 <noreply@z.ai>`, whatever the vendor or tool; `script/metrics` reads it.
- Squash against the base commit you branched from, never against `origin/main`. Agent worktrees
  share one `.git`, so `origin/main` moves under a lane while it works, and `git reset --soft
  origin/main` has silently staged four other lanes' files as one lane's own. Record the base SHA
  when the branch is cut and squash against that. The wider rule: in a worktree, `origin/*` is not a
  fixed point.
- `git stash` is unsafe in these worktrees, for the same reason one level over: the stash stack is
  per-`.git`, so it is shared machine-wide across every lane. Use a patch file (`git diff >
  /tmp/<lane>-<what>.patch`, later `git apply`) instead, and name it what no other lane would.
- Never squash across purposes. Squash-*merging* is already impossible (`allow_squash_merge` is
  `false` on this repository); this clause stays a prohibition because nothing gates it. The
  exceptions worth keeping unsquashed: a commit that records a correction or a surprise, and a
  commit whose separateness is itself the argument.

## Every date in this tree is UTC

calef, 2026-09-13, closing a gap that had been open since the first commit. Provenance blocks,
roadmap rows, `design/decisions/` sections, notes and commit messages all carry dates, and
`script/names` fails a ratification that lacks one, yet nothing said what zone any of them meant.
The agents writing most of them run UTC and an architect need not, so a ruling made in calef's
evening was already being filed under the next day. **Write UTC.** Where a date is load-bearing and
the hour is near midnight, put the time in the record too, because a reader cannot recover it later.

## Comments

The kernel is commented far more heavily than production code would be, deliberately. A comment
should explain a constraint the code can't show: *why* `sp` must be set before the first `bl`, *why*
`.bss` needs zeroing by hand, *why* the baud divisors are ignored by QEMU but needed by a real Pi.
Cross-reference the notes (`See notes/stack.md`) so the code and the glossary stay stitched
together.

Do not write comments that restate the next line.

## Style

These rules bind every note, which is prose an architect rereads for months:

- No em-dashes. Use commas, periods, semicolons, or parentheses.
- No "delve", "comprehensive", "landscape", "moreover", "furthermore", "notably", "it's worth
  noting", "straightforward".
- No sycophantic openers, no filler conclusions that restate what was just said.
- Plain, direct language. Vary sentence length. Write like a person. The numbers are [§213 (writing standards)](design/decisions/213-writing-standards.md), and length is [§212 (a prose budget)](design/decisions/212-a-prose-budget-for-every-document.md).

## Never leave QEMU running

A nife kernel that has finished its work calls `arch::halt()`, which is `loop { wfi }`. It never
exits, so QEMU never exits either unless something kills it or the kernel asks the host to terminate
via semihosting (which only the test build does). Two consequences:

1. Every interactive or demo QEMU run must be bounded, with `helpers/qemu-bounded.sh <seconds>
   <cmd...>`. `timeout(1)` does not exist on macOS, and `perl -e 'alarm N; exec @ARGV'` does not
   work on QEMU: QEMU installs its own `SIGALRM` handler and swallows the alarm, so the process runs
   forever.
2. `halt()` must use `wfi`, not `wfe`. QEMU implements `wfi` as a real vCPU halt and the host thread
   sleeps; it merely spins on `wfe`, burning 99.7% of a host core. With `wfi` it is 0.0%.

After any session that ran QEMU, check `pgrep -l qemu` and clean up (`-l` rather than `-x
qemu-system-aarch64`, because it matches every architecture's binary). Three rules for that cleanup, and
[`notes/qemu.md`](notes/qemu.md) has the four attempts it took to learn them:

- Killing a harness does not kill its children, so a `pgrep` that reports nothing can still be
  followed by a QEMU holding `target/nifefs.img`. Kill the tree at its root: walk `ps -o
  pid,ppid,command` up to the harness and kill that.
- The check runs in both directions. Before killing a "leaked" QEMU, walk `ps -o pid,ppid` UP from
  it: a QEMU whose parent chain ends in a live harness is somebody's gate in flight, not a leak.
- Ask who holds the file, not whether a process matches a name: `lsof target/nifefs.img` names the
  holder even when your pattern does not.

## Environment

- macOS on Apple Silicon (itself aarch64, which is a nice coincidence: kernel assembly is the same
  ISA the laptop runs)
- QEMU via Homebrew: `qemu-system-aarch64`, `qemu-system-riscv64`, `qemu-system-x86_64`
- Rust nightly, pinned in `rust-toolchain.toml` (needed for `custom_test_frameworks`)
- Targets: `aarch64-unknown-none-softfloat`, `riscv64imac-unknown-none-elf`, `x86_64-unknown-none`
