---
status: PARTIAL
raised: 2026-08-05
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 118. CLAUDE.md has a budget, and the rules that get violated move up the ladder

Minted 2026-08-05 by calef, who
noticed the file had gotten huge and asked what that costs. Two lanes have run: #309 took the audit,
and a second (2026-08-18) verified it and produced an applicable cut. **The cut is proposed, not
applied**, because a developer may not edit `AGENTS.md`; the integrator applies it. A third lane
(2026-08-22) built two of the three remaining pieces: see "The size gate and the ledger,
2026-08-22" below. The split is the one piece left, and it is untouched for the same reason the cut
is only proposed: it requires editing `AGENTS.md`.

Nothing stops the split from starting; it only needs an `AGENTS.md` edit no
developer lane can make, which is calef's or the integrator's to pick up.

**The size measurement is no longer hand-entered.** Until 2026-08-22 it was `wc -lwc AGENTS.md`,
run by nobody, which was the milestone's own budget argument for itself: this block carried three
different sizes in thirteen days, each hand-entered by whoever last looked. `script/lint` now
derives it (`agents-md-lines` in the counted-claims registry) and this file's own claim below is
checked against the tree on every build; see "The size gate and the ledger, 2026-08-22".

## What it costs, measured 2026-08-05

**738 lines, 8,306 words**, roughly 11,000 tokens, loaded into every session and into every lane
brief, because every brief says "read `CLAUDE.md` first, in full". About a dozen lanes ran on
2026-08-04 alone.

**Re-measured 2026-08-17: 868 lines, 10,001 words** (`wc -lw AGENTS.md`), so it has grown another 130
lines and 1,695 words in the twelve days this milestone has been open. The premise got stronger, not
weaker, which is the argument for the budget rather than against it. The 2026-08-05 figures above are
left as the measurement calef acted on.

**It grew 336 lines in one day, across 11 commits.** Two sections are 46% of it: the roles section at
224 lines and the naming section at 118.

**And the cost that matters is not tokens.** `CLAUDE.md` warns against `pkill`-ing QEMU and against
`git reset --hard` to take a measurement, once each. In one day **one lane killed another lane's
emulator mid-test with `pkill -f`, and four agents clobbered work** with `reset --hard`, `checkout`
or `stash`. Every one of them had been told to read the file in full.

So the warnings are present and were skipped. **Length is already costing compliance**, which is a
measurement rather than an aesthetic complaint.

## The diagnosis: not too long, wrongly stratified

Everything loads at once with equal weight, so "never `pkill` QEMU" competes for attention with the
`snake_case` conventions table. A reader skimming it cannot tell which three rules will bite them this
hour, and there are 130 more lines of it than when that was written.

**The reasoning is why the rules stick.** "The `sed` that rewrote the very row recording that the name
had been refused" is what makes that rule memorable, and compressing it to a bullet would produce a
style guide nobody obeys. **So this milestone stratifies rather than compresses**, and a lane that
finds itself deleting arguments has taken the wrong turn.

There is also an uncomfortable connection to the third principle: a stranger's first encounter with
this project is a document of this length addressed to an agent. Milestone 117 found that
independently, in both of its runs.

## The audit, 2026-08-18: which rules have a mechanism, and which are the budget

calef asked for the measurement this block had been describing: **count the rules, count how many
have a mechanism behind them, and treat the rest as the budget.** Done 2026-08-18, and it changes the
shape of the work rather than confirming it.

**868 lines, 10,001 words, 60,570 bytes, 16 sections.** 54 line-leading bolded claims, of which
**roughly 33 are rules** and the rest are argument, evidence or framing. Against that, `script/lint`
alone runs **32 named checks**, and `roadmap`, `decisions`, `names`, `citations`, `audits`, `verify`
and the suite run more.

**So the file is not mostly unmechanised prose.** That is the finding, and it is not what "the file
has gotten huge" implies.

### Rules that already have a mechanism, where the prose could shrink

| the rule | what enforces it |
|---|---|
| names are calef's; crates and modules in scope | `script/names`, `--check` in `script/lint` |
| ~~`snake_case` for Rust things, hyphens for scripts~~ | **wrong, corrected below**: no such check exists |
| what two binaries share is a crate, not a `#[path]` module | lint counts `#[path]` consumers and fails at two |
| architecture code stays under `arch/` | lint's rule-1 check, **but only partly**: see below |
| delete a lane's branch at merge | `delete_branch_on_merge`, so the platform does it |
| every fence names its counterpart | lint |
| benchmarks are first-class; measure, do not argue | the icount tripwire, and `script/icount` since milestone 78 |
| `nifefs` caps archive names at 32 bytes | the format, enforced by the compiler |

Eight rules whose paragraphs can become a sentence and a pointer, because the mechanism is the
argument now.

**Four of those eight did not survive verification.** This table was assembled from `script/lint`'s
banner list, and a banner names a check's subject rather than its coverage. The second lane read
each gate's body and recorded the result under "The applicable cut" below; two rows are struck
through or qualified above. Read that section, not this table, before deleting anything.

### Two rules whose prose has gone stale, which is the opposite failure

**The citations paragraph is wrong.** It says *"after any renumber, check citations by content, not by
running the gate"*, because `script/decisions --check` proves a `§N` resolves to *some* section and
never the right one. **Milestone 97 built that gate.** `script/citations` opens by naming exactly that
blind spot and closing it, and it caught a wrong gloss on pull request #305 on 2026-08-18. So the
constitution is instructing agents to do by hand what a check now does, which is worse than a rule
with no mechanism: it teaches distrust of a working gate.

**"Never squash-merge a branch" is prose against a permissive setting.** `allow_squash_merge` was
`true` on this repository when this was written, so the platform was configured to allow the thing the
constitution forbids. (**Since fixed**: it is `false` as of #334, 2026-08-18, verified live by the
second lane.) **Turning it off makes the rule unrepresentable**, which is rung one for one API call, and the
paragraph becomes an explanation rather than a prohibition anybody can violate. The reason is worth
keeping either way: milestone 96's lane put the loader unification in its own commit *ahead of* the
migration precisely so a boot failure could not be ambiguous between two changes, and a squash-merge
destroys that.

### The budget: four rules that could have a mechanism and do not

These survive only because somebody remembers, and **the first two both failed on 2026-08-18**:

1. **"The maintainer starts the two watchers at the beginning of every session."** The drain died when
   the maintainer pruned the worktree it was running from, then printed `No such file or directory`
   every 150 seconds for three hours while looking exactly like a quiet queue. `notes/merge-queue.md`
   already records that neither script reports its own death; this is that entry arriving.
2. **"Prune a lane's worktree the moment its pull request merges."** Nineteen worktrees and 44 GB had
   accumulated before anyone looked, on a machine that hit zero bytes free once already.
3. **"Squash against the base commit you branched from, never `origin/main`."** A recorded scar with
   no check: `origin/*` is not a fixed point in a worktree, and the failure stages other lanes' files
   as your own.
4. **`needs-architect` enforcement.** §88 is `PROPOSED` and unbuilt, so the label is enforced by *one
   script choosing not to arm it*. Nothing stops a merge by another route, and nothing stops work
   merging if the label was never applied. On 2026-08-18 a second session re-applied a label the
   maintainer had removed, which is the coordination half of the same gap.

**Everything else unmechanised is judgement and should stay prose**: the top-up rule, the handoff
rule, lane count against the collision surface, "correct yourself loudly", "push back when he is
wrong", "explain on request". A gate over any of those would be a gate about taste.

### The clause the tenet needs

calef, 2026-08-18: **prefer gates over prose in this file.** With one qualification the same day
earned: **a gate is only better when it can be right about the tree, so record what it measured when
you built it.** The branch-prefix check rejected legitimate work four times and was widened after each
one; `script/lint`'s own justification for a disabled check had rotted from "100% false positives" to
no longer true, and nobody noticed because a justification is not a measurement anybody re-runs. A
gate with its measurement beside it can be told stale from live; one without it gets deleted by
whoever it inconveniences.

## The applicable cut, 2026-08-18, second lane

The audit above named eight rules with a mechanism. **This lane checked each claim against the tree
and four of the eight did not survive contact**, which is the finding rather than a footnote: the
audit was assembled by reading `script/lint`'s banner list, and a banner names a check's subject, not
its coverage. What follows is only what was verified by reading the gate's body and testing its
claim against the files it scans.

**This lane did not edit `AGENTS.md`, and could not.** That file forbids a developer to edit it, so
the deliverable is a cut the integrator applies. Line numbers are against `AGENTS.md` at
`64c08ec9`, 955 lines; apply the cuts **bottom-up** so earlier ranges stay valid.

### The measurement

**955 lines, 11,062 words, 66,724 bytes** (`wc -lwc AGENTS.md`), against 738/8,306 when calef minted
this block on 2026-08-05 and 868/10,001 when it was re-measured on 2026-08-17. **+217 lines and
+2,756 words since minting, +29% on lines.**

The growth is not diffuse, and that is the useful half. Counts are of the file as it stood at each
commit that touched it (`git show <sha>:AGENTS.md | wc -lw`, following the 2026-08-14 rename from
`CLAUDE.md`):

| date | commit | lines | what it added |
|---|---|---|---|
| 2026-08-05 | (b0cf1245, 08-04) | 738 | the measurement calef acted on |
| 2026-08-13 | 7c65f10a | 748 | the ranking function is a customer running it |
| 2026-08-14 | 150628ac, 3b54f6fc | 768 | the `AGENTS.md` rename, then usernames as the standard |
| 2026-08-15 | 4f215acb, 5d00b8e9, 9d1d49fc, 6f71615f | 787 | milestone 120, the two-directional leak check, §71, plural maintainers |
| 2026-08-16 | 1ff2a038, daf5e84a, 23aadabe | 822 | lane count against collision surface, draft-PR-first, the agent-voice line |
| 2026-08-16 | 1da0d68b | 868 | the elegance-and-performance tenet (+46) |
| 2026-08-17 | 77017f71 | **955** | milestone 94's convention (+87) |

**Two commits are 61% of the growth since 08-05.** `1da0d68b` (+46) and `77017f71` (+87) added 133
of the 217 lines, and both are whole new tenet sections rather than accretion. The file does not
creep; it steps. That matters for the budget's design: a per-commit size gate would have fired
exactly twice in thirteen days, on the two commits whose authors most believed they were adding
something load-bearing.

**One commit shrank it.** `daf5e84a` took it 807 to 794 while adding the draft-pull-request rule,
by deleting what that rule replaced. That is the shape the budget is trying to make normal, and it
already happened once unprompted.

### Applied 2026-08-18 by the integrator: four cuts, 1,212 bytes

**Landed.** `AGENTS.md` went 955 lines / 11,062 words to 942 / 10,880. The three deletions removed
292, 377 and 543 bytes; cut 4 is a reword and saves nothing. Every line number in the proposal below
was still exact when it was applied, except rule 7's block, whose stated range included a trailing
blank line; applying by text match rather than by index is the lesson, and it is why this was applied
by search rather than by `sed`.

The two rewordings the proposal said an integrator would have to write:

- *"The rule that matters more than the tidiness:"* became a standalone opener, **"An unmerged branch
  is either abandoned..."**, since the paragraph it referred back to is gone.
- *"Three reasons, and the second is the one that matters"* became **"Two reasons, and `script/lint`
  check 5 carries the third"**, which names where the deleted argument went rather than leaving a
  reader to wonder.

Cut 4 kept its prohibition and lost its overreach: *"Never squash across purposes"* stays a rule
because nothing gates it, and the squash-*merge* half now says the platform refuses it.

### Verified: four cuts, ~15 lines and ~1,150 bytes net

Small, and deliberately so. **The honest total is about 1.7% of the file**, against the audit's
estimate of "on the order of 90 lines, about a tenth". The gap between those two numbers is this
lane's actual result.

#### Cut 1: delete lines 508-511, the stale citations paragraph (293 bytes)

> **After any renumber, check citations by content, not by running the gate.** `script/decisions
> --check` verifies that a cited `§N` resolves to *some* section, never that it resolves to the right
> one, so a well-formed wrong citation is invisible to it. This has already produced two of them.

**Mechanism: `script/citations --check`, wired into `script/lint` at line 565** under the `==>
citations` banner. Verified by reading the check's body: it compares a citation's parenthetical
gloss against the record's own title or body, and confirms an attributed block quote still exists in
the file it names. `script/lint`'s comment above it (lines 555-563) names the exact blind spot this
paragraph describes and says it is closing it.

**Safe because the prose is not merely duplicated, it is wrong.** It instructs an agent to do by
hand what a gate now does, which teaches distrust of a working check. Nothing here is an argument
worth preserving: the reasoning (a well-formed wrong citation is invisible to a resolve-only gate)
is stated more fully in `script/lint`'s own comment and in `notes/citations.md`.

#### Cut 2: delete lines 544-548, the branch-deletion rule (377 bytes)

> **Delete a lane's branch when you merge it, and never use a branch as a filing cabinet.** Forty-seven
> branches accumulated in about two days of lane work ... So it belongs in the merge, not in a
> periodic cleanup.

**Mechanism: `delete_branch_on_merge` is `true` on this repository.** Verified live:
`gh api repos/nifeos/nife --jq .delete_branch_on_merge` returns `true`. The platform deletes
the branch, so the rule is rung one and cannot be violated by forgetting.

**The paragraph that follows must stay, and the integrator must reword its opening.** Lines 549-553
carry a different rule with no mechanism at all: *an unmerged branch is either abandoned or it is
holding knowledge that is not on `main`*, with `fix/redoxfs-write-loop` as its worked example.
That is ungated and stays. Its first clause currently reads "The rule that matters more than the
tidiness:", which refers back to the deleted paragraph; it becomes a standalone opener. **This is
the only cut in this set that requires the integrator to write a word rather than delete one.**

#### Cut 3: delete lines 686-692, rule 7's second reason (546 bytes)

> **A `#[path]` module inside a `no_std` binary is unreachable by host tests and by Kani.** ... `cseam`
> is the case that proves it ... A drift there shows up as a C component scribbling on the wrong page.

**Mechanism: `script/lint` check 5** (lines 1174-1212), which counts consumers per `#[path]` include
target and fails at two. Verified by reading it. Its comment carries this reason **verbatim in
substance**, including the `cseam`/`c_seam.c` worked example and the drift-as-wrong-offset
consequence, plus something `AGENTS.md` lacks: that a single-consumer module is fine and stays fine,
with `net_transport` and `socket_test_client` named as today's instances.

**Only this reason goes, not the rule and not the other two.** The rule statement (676-678) stays,
because a reader needs to know the rule before they trip the gate. Reason 1 (682-684, "it removes a
category that nothing enforces") and reason 3 (693-694, "makes location self-enforcing for free")
stay, because **neither appears in the gate's comment** and deleting them would delete an argument.
Line 680's "Three reasons, and the second is the one that matters" becomes "Two reasons", which is
the second word the integrator writes.

**The recorded caveat on this cut**: the gate scans `user/src` only. `fs_server/` also declares
three `[[bin]]` targets, so a `#[path]` module shared between two of them would be invisible.
`fs_server/src` contains no `#[path]` today (verified by grep), so the gap is latent rather than
live, but it is real and it belongs in the gate's `BUGS`-equivalent rather than in nobody's head.

#### Cut 4: reword line 878, the squash-merge prohibition (a reword, not a saving)

**Mechanism: `allow_squash_merge` is now `false`.** Verified live via `gh api`. **This corrects the
audit above**, which recorded it as `true` and proposed turning it off; that landed in the meantime
(#334, the commit this lane branched from). Squash-merging is now unrepresentable, which is rung one.

So *"and never squash-merge a branch"* becomes an explanation rather than a prohibition. **"Never
squash across purposes" stays a prohibition**, because nothing gates it: a lane can still collapse
two purposes into one commit locally, and milestone 96's loader-ahead-of-migration structure is
exactly what that would destroy. The milestone 96 reasoning (879-883) stays either way.

This one saves almost nothing. It is in the cut because a prohibition against something the platform
already refuses reads as a live hazard, and a reader who believes the constitution is guarding
something the settings guard will not check the settings.

### Declined: five rules examined and not cut

**This is the more informative number**, and four of the five were on the audit's list of eight.

| rule | why not |
|---|---|
| **rule 1, arch code stays under `arch/`** (643-646) | The gate greps `asm!\(` and `core::arch::` under `kernel/src`. Rule 1 also claims **system registers**, and `kernel/src/main.rs:733` and `:1123` read `aarch64_cpu::registers::CurrentEL` outside `arch/` today. Both are invisible to the gate. Cutting the prose would silently unenforce the clause that the check's own comment says the discipline already slipped on. **Extend the gate to the register crates first, then cut.** |
| **`snake_case` for Rust things, hyphens for scripts** (776-818) | **The audit's claim is false.** `script/lint`'s naming-conventions block runs seven checks and none is a case convention: no `-d` suffix, no "daemon", `*_proto` spelling, branch prefixes, milestone-branch-touches-roadmap, `#[path]` consumers, provenance presence. The only case gate is check 6, and it covers **one of the six rows** of that table (`notes/`, `design/` markdown filenames). Crates, programs, modules, `script/` entry points, repo-root markdown and directory names are ungated. |
| **names are calef's call** (700-775) | `script/names --check` verifies a name carries **a provenance state**, and `script/lint`'s own comment is explicit that it checks "presence of a STATE, never the state `ratified`", precisely so the gate is not a wall. 54 of 126 names are unratified. The gate does not enforce the rule; it records whether the rule was applied. |
| **benchmarks are first-class; measure, do not argue** (555-558) | The icount tripwire catches a performance regression. The rule is about **honesty in reporting**: state what a number means, name where it is not apples-to-apples, prefer a recorded tie to an overclaimed win. No gate reads a claim's honesty. This is judgement and stays prose. |
| **`nifefs` caps names at 32 bytes** (820-825) | The constant is real (`crates/nifefs/src/lib.rs:118`, `pub const NAME_LEN: usize = 32`) and the compiler enforces the cap. But the paragraph's operative content is advice the compiler cannot give: *do not let it pick a name, do not spend a format change on bytes nothing needs*. A build failure tells you the name is too long; it does not tell you to pick a different name rather than widen the format. **Could shrink to two lines; not a deletion.** |

### What this lane did not get to

Named as remaining work rather than guessed at, because a proposed deletion that turns out to be
unenforced is the failure this milestone exists to prevent.

- **"Every fence names its counterpart"** was on the audit's list. There is a gate (`script/lint`
  line 268), but this lane did not find the corresponding *prose rule* in `AGENTS.md` to cut: rule 4
  (664-666) is "assume weak memory ordering", which is judgement, not the fence-pairing bookkeeping
  the gate checks. Either the audit row names a rule that is not in the file, or it is in a section
  this lane did not read. **Unresolved.**
- **The split** (the core-plus-linked-documents piece) is untouched. So is the size gate and the
  violation ledger. Those are the three remaining pieces.
- **The four budget rules** are unchanged: the watchers, worktree pruning, squashing against the
  base SHA, and `needs-architect` enforcement all still have no mechanism.

### What this changes about the milestone's premise

**The file is not carrying much deletable duplicate.** Two lanes have now looked, and the verified
cut is 1.7%. The audit's own finding stands and is stronger than it reads: roughly 33 rules against
32 named `script/lint` checks plus six other gate scripts, and the overlap between those two sets is
much smaller than the counts suggest, because **the gates mostly check things the constitution never
claimed to rule on** (unsafe contracts, dead-code suppression, conflict markers, dependency
direction, counted claims).

So the growth problem is real and the duplication problem is mostly not. **The budget and the split
are where the value is**, and a future lane should not spend itself re-auditing for cuttable rules;
this lane and #309 have now covered that ground.

## Three pieces

### Split, the way this tree has split three monoliths already

`DECISIONS.md` became `design/decisions/` (milestone 114), the roadmap became `design/roadmap/`, and
`notes/` is indexed. `CLAUDE.md` is the last monolith and the one loaded most often.

A **core** of the rules that change behaviour on *every* task, with linked documents for the rest.
The roles section and the naming section are the obvious first moves at 46% of the file between them.
**The test is not a line count but whether an agent will genuinely read the whole core**, so the
lane should say how it judged that rather than picking a round number.

### The first cut: delete the eight rules a gate already enforces

**Superseded in part by "The applicable cut" above**, which executed this and found four of the
eight unenforced. The reasoning below stands and is why the cut is worth making at all; the
estimate of "on the order of 90 lines" does not, and the verified figure is about 15.

calef, 2026-08-18, asked whether a rule with a mechanism can simply be eliminated. **Yes, and the
reasoning does not go with it, because the reasoning is already somewhere better.**

**The evidence is what the gates' own comments contain.** `script/lint`'s rule-1 check carries twelve
lines: the scar that produced it (a raw `SPSel` read in `user/tests.rs`), the three spellings
architecture code has in Rust, why there is deliberately no allowlist, and the precedent for what to
do instead (`current_sp` and `sync_icache`, where the register read moves behind an `arch::` helper).
`AGENTS.md`'s version of the same rule is three sentences with none of that. The naming and dead-code
checks carry eight and twelve lines respectively.

**So this is deleting a duplicate, not an argument**, which is the distinction that keeps it inside
this block's own warning that a lane deleting arguments has taken the wrong turn.

**And the gate is the better home, not merely an equal one.** A rule in `AGENTS.md` is read at session
start by an agent that cannot know which three rules will bite it that hour. A rule in a gate's
comment is read by somebody who has just tripped it. On 2026-08-18 `script/lint` refused a lane's
crate-level `allow(dead_code)`, and that lane found the *right* fix (`icount = ["bench"]`, riding
conditions that already existed) because §38's reasoning was there when the check fired. That is rung
three, and it is milestone 115's shape exactly: the record beside the thing rather than in a registry.

**The test before deleting each one, and it is falsifiable.** *Does the gate's failure message plus
its comment tell somebody enough to fix it correctly, including the case where the gate is wrong about
the tree?* The counted-claims check passes: it prints "fix the number, **or fix the derivation** if the
tree is right and the gate is asking the wrong question". The branch-prefix check failed it, which is
why it was widened four times instead of fixed once. **Where the constitution holds something the gate
lacks, move it to the gate first; then delete.**

**No pointer left behind.** A "see `script/lint`" line is a tax on every session for a rule that can
no longer be violated silently. The eight are listed in the audit above; the paragraphs are on the
order of 90 lines, about a tenth of the file, and they are the tenth that needs the least reading.

**What this does not touch.** The four budget rules have no mechanism, so deleting their prose deletes
the rule. They move *up* the ladder (next subsection) rather than out of the file. And the judgement
rules stay prose, because a gate over "push back when he is wrong" would be a gate about taste.

### The most-violated rules stop being prose

This is the ladder turned on itself. "Do not `pkill` QEMU" and "do not `reset --hard` to take a
measurement" are **rung four**, prose relying on memory, and they failed five times in one day. Both
have a higher rung available:

- a wrapper that finds and kills only the caller's own emulator, so the dangerous form is never the
  convenient one;
- `git show <sha>:<path>` as the read-only way to look at another revision, which is what every one
  of those four agents actually wanted.

**A rule that is violated repeatedly is not stated too quietly. It is on the wrong rung.**

### A budget, so this does not have to be done again

A one-time cut re-grows; this file added 336 lines in a day without anyone deciding to. So the lane
adds a **gate on the core's size** to `script/lint`. Crude, and that is the point: it converts "should
I add this rule?" into "**what does this replace?**", which is the question nobody asks unaided.

**Pair it with the signal that actually matters, which is not size.** A rule nobody breaks is cheap at
any length; a rule that gets broken is either mis-stated or on the wrong rung. So keep a short ledger
of **times a documented rule was violated anyway**, with the rule named. Three strikes and it must
move up the ladder or be deleted as unenforceable.

The evidence for that ledger already exists in lane reports, honestly self-declared: *"I killed one of
dev-97's QEMU processes by mistake"*, *"I clobbered my own working tree"*. Those reports are the
input; nothing currently reads them.

## The size gate and the ledger, 2026-08-22

A third lane built both, and neither could be a mechanical extension of the earlier work: the split
that would produce a "core" has not happened, and the eight-rule cut and the four budget rules the
audit named are unrelated to what a size gate or a ledger need.

### The size gate

`script/lint`'s existing `count-at-most` relation (built for milestone 134's unsafe-density ceiling)
is exactly the mechanism this section already asked for without naming it: a claimed number that
fires when the tree exceeds it and stays silent when the tree falls below. `script/lint` gained an
`agents-md-lines` registry entry (the file's own `wc -l`-equivalent line count) and the claim lives
here, since a developer lane may not edit `AGENTS.md` to carry its own marker:

**`AGENTS.md` carries at most 573 lines** <!--count-at-most:agents-md-lines-->, written at the
tree's exact value with **zero headroom**, deliberately (it read 1009 until 2026-09-19, when §155
moved the naming conventions to `design/naming.md` and the file fell to 922; milestone 262 had asked
for 988 against its own smaller diff, which §155 (naming conventions) superseded):
Lowered to 573 on 2026-09-25 by calef's seven cuts. Lowered from 935 to 584 on 2026-09-24, and
made a ratchet the same day: #1189 (calef, 2026-09-24) split the file into imperatives plus
`design/tenets/` appendices, and a ceiling left at 935 would let it regrow 351 lines. So
`agents-md-lines` is in `script/lint`'s `RATCHET_CEILINGS`: the gate also fails when the file falls below this number,
and the fix is to lower it, so every shrink is banked. A raise is still an edit here with its reason
beside it, as every raise below was.
Before that, **lowered from 949 to 935 on 2026-09-23**, the first time this ceiling has ever moved down, and the
fourteen lines went to `briefs/merge-and-cleanup.md` and `briefs/session-start.md` rather than being
deleted. The number was taken with `wc -l` from the finished file, not by arithmetic against the
diff, because two branches earlier the same day each raised this ceiling honestly from the same base
and produced a wrong number between them. The considered act, and it is a different one from every
raise below: milestone 579 (which of the constitution must be carried, and which is a brief)
classified every passage of the file by whether it has to fire when nobody is looking for it, and
found **85% must be carried**, so this is a targeted extraction rather than the split this section
has been asking for. The three passages that moved (the merge checklist's one line, the prune-the-worktree paragraph, the
`nife-dev` relink) all fire at one event performed by one role, and the watcher pair moved because
most of its nine lines already duplicated `briefs/survey-the-queue.md`. What stayed behind in each
case is the ambient clause with no task to trigger it: prune promptly and never prune uncommitted
work, expect every lane to take `nife-dev`, a queue reports and does not resolve.
Before that, **raised from 925
to 929 on 2026-09-23**, for four lines, and the considered act is that a session's own context had
become the same shape of unrecorded fact this milestone exists to catch: real (233 million tokens
against 5 million for twenty dispatched lanes, measured 2026-09-22), consequential (it inverts why
delegation is worth doing here, context isolation before cost-per-token), and living only in one
session's head. The full argument and the practices it earns went to `notes/what-a-session-carries.md`
rather than here, per this section's own rule that the note carries the argument and the constitution
carries the rule; what is here is three lines pointing at it. Nothing was removed to pay for it; that
is the cost, stated. **Raised from 929 to 949 on 2026-09-23**, measured from the merged tree after two branches raised it independently from the same base of 925, one to 929 and one to 945, arriving at 949, for a new tenet: calef stated on 2026-09-23 that everyone owns
a problem they notice, not only the ones with a mechanism already pointed at them, the complement to
the existing "Nobody remembers, so build the mechanism that does not need them to" tenet (2026-08-04),
because a load-sensitive test flake was correctly written into `notes/load-sensitive-assertions.md`
twice and left, since every pull request that hit it was not the one that caused it. Nothing was
removed to pay for it; the section earned its lines by staying to one paragraph of quote, one of
relation to the existing tenet, and one of the failure it names.
**Raised again from 924 to 925 on 2026-09-22**, for one line, because the disk ceiling it repriced
was not merely stale but actively misleading: it told a reader the lane count was three or four for a
reason that had stopped being true, and the paragraph that replaces it has to carry both the retired
number and the lever that replaced it (lanes gate in CI). Compressed first, from an eleven-line
original to twelve. Before that, **raised from 922 to 924 on 2026-09-21**,
which is the first time this ceiling has moved, and the reason is worth reading because it is the
case this budget exists to permit rather than the case it exists to refuse: the file's advice on
avoiding a shared `git stash` **recommended `/tmp/x.patch` by name**, a fixed path that is shared
machine-wide exactly like the stash stack it was protecting against, and two lanes duly collided on
`/tmp/pr-body.md` that day. Two lines bought a correction to a hazard the constitution was itself
teaching. Compressed first: the addition was five lines before being cut to three. the point, per this section's own
words above, is that every line added should replace one removed, or be a considered act that says
why the growth was worth it. That is the same choice `unsafe-thread-safety-claims` made for a
different reason (a population small and consequential enough that every addition deserves the
stop); here the reason is this milestone's own diagnosis, that the file grows in whole deliberate
steps rather than by diffuse creep, so a lane adding one of those steps is exactly the lane that
should also write the sentence justifying it. See notes/counted-claims.md for the mechanism and
notes/rule-violations.md's neighbor for the same pattern applied to a different number.

**Raised 1001 to 1009 on 2026-09-14**, net of a consolidation, and the eleven added lines are a third lane-count ceiling: disk. The considered act: the file already bounds concurrent lanes by collision surface and by memory, and neither of those warns about the third. Five lanes at roughly 3 GB of `target/` each, plus 7.2 GB in the main checkout's own, took the volume to 1.9 GB free and then to a command failing mid-write; the maintainer session met it four times in one night and cleaned up four times without ever writing the limit down, which is rung zero wearing diligence. It earned lines rather than a note for the reason named in the text: disk is the only pressure in this system that destroys work rather than delaying it, and a lane pruned under it loses uncommitted work outright. **Three lines WERE removed to pay part of it**, which is the first time this ledger records a raise partly funded rather than simply taken. calef asked whether the prune rule could be consolidated; it stood in four places, and two of those are role duty lists (the maintainer's and the steward's hygiene clauses) where naming the duty is the point rather than a restatement. The other two were genuine duplicates, each stating one rule with different evidence: eight worktrees with one holding 3.3 GB, and 2026-07-31's zero-bytes-free with 42 worktrees holding 78 GB. They are now one paragraph carrying both measurements and the push-before-removing clause, sited beside the disk ceiling where a reader now meets the subject. **What NOT added**, and the near-miss is worth the sentence: the same session proposed also adding "prune the worktree at merge", which this file already says in four places (323, 367, 496, 615). The rule was not missing and the session had simply not followed it, so a fifth copy would have been milestone 262's own defect, the file growing something it already had.

**Raised 992 to 1001 on 2026-09-13**, and the nine lines are a new section saying every date in this tree is UTC. The considered act: provenance blocks, roadmap rows, `design/decisions/` sections, notes and commit messages all carry dates, `script/names` *fails* a ratification that lacks one, and nothing anywhere said what zone a date meant. The agents writing most of them run UTC and calef does not, so a name he ruled in his evening was already being filed under the next day, silently and in a record nobody can correct later. This is the *facts that leave the machine* category of the **move fast on what can be undone** tenet, which is why it earned lines rather than a note. Nothing was removed to pay for it; that is the cost, stated. Deliberately **not gated**: `script/names` could compare a ratification date against the commit's author date, but a record honestly written the morning after a ruling would fail it, so the check would be wrong about the tree in the way §77 already is. Rung three, at the thing itself.

**Raised 988 to 992 on 2026-09-13**, and the four lines are the rename procedure in the naming section. The considered act this budget asks for: the rule came out of a rename performed the same day that missed three live proposals and nearly rewrote a quotation, the detail lives in `design/naming.md`, and what is here is the four-line rule itself, because `AGENTS.md` is the rule and the note is the argument. Nothing was removed to pay for it; that is the cost, stated.

**Lowered from 1048 to 988 on 2026-09-05 by milestone 262, and a lowering deserves the sentence as
much as a raise does.** The naming section went 137 lines to 77 without losing a rule: the argument
moved to `design/naming.md`, which now says in its own first paragraph that it is the case and not the
authority, and that where the two disagree this file is the rule and the note is the bug.

**Most of the 60 lines were not moved, they were deleted as duplicates.** The two-tier rejection, the
`wc` example, the "0 of 57" measurement, the standard-terms guard rail and the `NAME_LEN` history
were each already written out in full in both files. That is the defect milestone 262 was minted for,
found four more times while fixing it once, and it is the answer to the question this budget could not
ask: the file was not only growing, it was growing things it already had.

**The ceiling is set at the new exact value with zero headroom**, per this section's own rule, so the
next addition argues for itself the way the three on 2026-09-05 did.

**Raised from 1043 to 1048 on 2026-09-05, the third raise that day, and this paragraph is the
sentence the rule asks for.** Five lines add a **seventh question** to the list a fork must arrive
answered: *would we still choose this if both options cost the same?* It is not new reasoning. It is
§92's test, which already existed and lived inside a tenet, so it fired only when somebody remembered
it. calef asked for it as a standing question after it had gone unasked for a month on milestone 51's
"three problems, one addition", which counted effort and read as design until he applied the test
himself.

**Three raises in one day is the argument for milestone 262, not against the raises.** Each was a
considered act with its sentence, which is what this budget asks. But the file grew fifteen lines
between 1033 and 1048 in a morning, and the naming section alone is 12% of it and mostly argument.
The constitution should carry the tests a lane applies and the notes should carry the cases; until
that split is made, every good rule costs the budget the same as a bad one.

**Raised from 1037 to 1043 on 2026-09-05, and this paragraph is the sentence the rule asks for.**
Six lines, and they replace a list with a test. The naming section protected eight standard terms by
enumeration, which told a reader nothing about the ninth; it now carries calef's rule that **an
acronym is spelled out unless its expansion teaches nothing**, which can be applied to a name nobody
has coined yet. It also overturns a ratification of his own (`dma_validator`, 2026-08-01) and says
so, because a reader who remembers that decision should be able to see it reconsidered rather than
quietly dropped.

**The growth is six lines and not thirty because the argument went to design/naming.md**, where the
same clause was also stated and would otherwise have contradicted the constitution the moment this
landed. That duplication is itself the subject of a milestone now: the constitution should carry the
tests a lane applies and the note should carry the history, and today the naming section is 12% of
`AGENTS.md` with most of that being argument.

**Raised from 1035 to 1037 on 2026-09-05, and this paragraph is the sentence the rule asks for.**
Two lines, and they buy a correction rather than an addition: §90's paragraph justified the
draft-pull-request convention with *"a draft cannot be stuck in the merge queue because a draft
cannot be merged"*, and that is false. GitHub closes a draft as **merged** when its head holds
nothing its base does not, which happens whenever a lane bases on a `maintainer/mint-*` branch that
lands before the lane's first commit. The board then reads empty while a lane is working, which is
the single property the board exists to provide, and it had done so four times since 2026-09-02.

**The growth is two lines because the evidence went elsewhere**, which is this budget working as
designed rather than being worked around. The instruction (`git commit --allow-empty` before the
draft) is in `AGENTS.md` because that is where a lane reads it; the four cases, the window, and why a
rate-dependent failure hid for three days are in
[§90](../decisions/90-claiming-and-closing.md)'s amendment, because that is where evidence belongs.
The constitution gained the rule and not the argument.

**Raised from 990 to 1012 on 2026-08-30, and this paragraph is the sentence the rule asks for.** The
addition is principle 1: the customer path it had named since 2026-08-05 ended that day, when calef
reported the family's backups running on borg over SSH on cordoba, built with the existing Linux
ecosystem while nife was not ready. A ranking function that still named milestone 55 would have been
ordering work by a workload nobody runs, which is the one failure the whole principle exists to
prevent, so the correction is not optional and it does not compress: it has to say what happened,
that the principle was confirmed rather than refuted, what replaces the ordering while the path is
empty, and the sizing lesson (the first customer was among the largest things a home system can be
asked to be). Principle 2's figures were refreshed in the same commit and cost two lines net, since
every number in them had drifted since 2026-08-05.

**Twenty-two of the twenty-seven lines proposed were kept and five were cut** before raising this,
which is the order the rule intends: the superseded figures went to git, two bullets saying one
thing became one, and a paragraph lost a clause. A ceiling raised without that pass is a ceiling that
is not doing anything.

**Raised again to 1019 on 2026-08-30, seven lines, and the rule's own test is why they were worth
it.** calef: *"I don't think we expose nife to third parties (aka other customers) until we have a
package manager and a trivial install process."* That is a **precondition on principle 1's ranking
function**, not an item ranked by it, and a roadmap that ranks by the shortest path to a customer
while being unable to accept one is ranking against a door it has not built. The seven lines say
that, say it is ours rather than the customer's constraint, and record that he wants package
management early for the builders' sake and not only for a stranger's. Nothing was cut this time:
the bullet it extends is the shortest in the section and had nothing spare in it.

**Raised to 1035 on 2026-09-01, three lines, and the cheapest raise this file has taken.** The
developer role said a lane never edits `design/`. `script/lint` check 4b requires a `milestone/N-*`
branch to edit `design/roadmap/N-*.md`, and calef had made that exception on 2026-08-23, but it was
written only in a comment above the check. So the rule a failing lane could read forbade the only
edit that clears the gate. Two lanes hit it on one day and both reported the gate as impossible,
correctly. The three lines are the exception itself; the reasoning stays beside the check, which is
where the next person meets it.

**Raised again to 1032 on 2026-08-31, thirteen lines, and this one is a cost the file did not know
it had.** The lane-count section bounds concurrency by *collision surface*, which is correct and
incomplete: it says nothing about memory, so a maintainer obeying it exactly ran two lanes whose
gates were eight parallel SAT solvers against a budget `script/verify`'s own header tunes at four.
That cost an out-of-memory kill which took the session with it, a `ci-build` invalidated by 2.7x
oversubscription, and two proof runs killed by SIGTERM, all in one day, all read as three unrelated
problems. The addition is thirteen lines because the failure is easy to misread: it never arrives as
"out of memory", it arrives as a cancellation or a timing failure, and naming that tell is most of
what the lines buy.

**Raised from 942 to 947, 2026-08-23**: milestone 160's naming-tenet extension (three lines
recording that calef's crate/program/module naming authority now also covers public function
names, plus a blank line and citation) is exactly the "considered act that says why the growth was
worth it" this section asks for -- a durable rule change belongs in the file the rule lives in, not
only in the milestone that decided it.

**Raised from 947 to 957, 2026-08-26**: a real, previously-unrecorded footgun found live (the
milestone 161 two-core-crash lane's `git stash` collided with another session's stash, since the
stash stack is per-`.git` rather than per-worktree) was recorded beside the existing
squash-against-`origin/main` trap it shares a family with, so the next lane to reach for `git stash`
in a worktree finds the warning before it needs it.

**Raised from 957 to 965, 2026-08-26**: "a lane continues until it needs a human or it is done"
(calef, launching milestone 47's and 49's second-round lanes the same day) moved from a per-brief
instruction, typed into every lane's prompt by hand, into the Developer role's own standing default,
beside the sibling rule it now sits next to ("a developer polls its own background work to
completion"). The same shape of fix as both entries above it: a rule a maintainer had to remember to
restate every time is now a rule every brief inherits without anyone retyping it.

**Raised from 965 to 975, 2026-08-26**: the watcher-start instruction was rewritten after it failed
exactly the way it was written to prevent, a maintainer session reading it and not acting on it. The
addition records what replaced it (`launchd` on patagonia, rung one for the part of the gap a
session can close) and the gap calef accepted rather than solved (asleep or shut down, neither
watcher runs), which is the same "durable rule change belongs in the file the rule lives in" case
the entry above already made.

**Raised from 975 to 990, 2026-08-26**: calef asked, mid-session, whether clearing a stalled PR's
conflicts needed new machinery or a human pointing at it each time; the honest answer surfaced that
`notify()` posts once and goes quiet, so nothing re-announces a stall to a session that opens later.
The addition records the standing check this became (a maintainer session reads the queue for
`DIRTY`/`FAILURE` each pass) and the alternative considered and declined (an unattended scheduled
agent, which calef turned down in favour of work that shuts down with the session driving it), the
same shape of record as both entries above it.

**Targets the whole file, not "the core", because the core does not exist yet.** Once the split
happens, this ceiling should move to whatever the core becomes and stop counting the linked
documents; that is a known adjustment for whoever does the split, not a defect today.

**Raising it is the correct response to a deliberate addition, and the mechanism for doing so is
unchanged from every other ceiling in the registry**: raise the number in the same commit that grows
the file, and say beside it why the growth was worth it. Since a developer lane cannot make that edit
either, raising the ceiling is the integrator's or calef's act, exactly like applying the proposed cut
already is.

### The violation ledger

`notes/rule-violations.md` and `script/rule-violations` (both provisional names). One row per
incident, an `instances` count per row (a source that reports an aggregate without naming individual
incidents gets one row with that count, honestly, rather than invented distinct rows), and a status
that keeps a row from re-triggering once a rule has already been addressed. `--check` fails when an
`open` rule reaches three strikes.

**Seeded from the incidents this milestone and `AGENTS.md` already recorded against themselves**, not
invented for the occasion: the squash-against-`origin/main` scar (`AGENTS.md`, "Commits"), the
`pkill`/mid-test-emulator incident and the reset-`--hard`/`checkout`/`stash` clobbers this section's
own "What it costs, measured 2026-08-05" already quotes. **The git-clobber rule crossed the
threshold at four open strikes**, one past three, exactly the finding this ledger exists to
surface. [DECISIONS §128](../decisions/128-git-clobber-enforcement.md) priced the real enforcement
options and calef's own call, on the evidence of zero repeats in the three weeks since, was to
accept rather than add a mechanism: the existing `AGENTS.md` prose already appears to be doing the
job. The ledger row is marked `resolved` on that basis. See notes/rule-violations.md, including its
own honest limits (self-reporting only, exact-text matching, not wired into any mandatory gate).

**Deliberately not wired into `script/lint` or CI.** The git-clobber rule's threshold was already
crossed by history this lane did not create; wiring `--check` into the mandatory suite today would
fail every lane's pull request over a decision that is not a lane's to make, which is the same
restraint DECISIONS §61 already states for an ordinary lint. Whether and when to wire it in is left as
an open question in the note rather than answered here.

### What is left

**The split is now the only untouched piece.** It requires editing `AGENTS.md` and so is not
startable by a developer lane at all; it waits on calef or the integrator, the same as the proposed
cut from 2026-08-18 and the ceiling-raise mechanism above. Everything else this milestone named as
remaining (the size gate, the ledger) is now built and gated.

## Scope note

**Not a rewrite, and not a trim.** No argument in that file is deleted; the reasoning is the asset.
Text moves, and only the rules that need to change rung change form.

**The budget number is provisional and belongs to whoever builds this**, informed by what the core
actually needs rather than chosen first and filled to.

**The honest limit**: a size gate measures the wrong thing on purpose. It cannot tell a rule that
earns its lines from one that does not, and a lane that games it by moving text into a linked file
nobody reads has satisfied the gate and defeated the milestone. The ledger is the counterweight and
it is weaker than a gate, because it depends on lanes continuing to report their own mistakes
honestly, which is a culture rather than a mechanism. **Say so where the reader meets it.**
## Follow-on

- **Outstanding.** The split into a core plus linked documents is untouched: `AGENTS.md` is still
  one file, and cutting it needs an `AGENTS.md` edit no developer lane may make. Checked
  2026-09-03.
- **Outstanding.** The line ceiling still counts the whole file rather than a core, because the
  core does not exist, and `script/lint`'s registry entry sits at zero headroom exactly as
  designed. Checked 2026-09-03.
- **Done.** Budget rule 1, the watchers nobody starts, has a mechanism: `AGENTS.md` records
  `com.nife.merge-drain` and its sibling running unattended on patagonia under `launchd`, with the
  asleep-or-shut-down gap named as accepted rather than closed.
- **Outstanding.** Budget rule 2, pruning a lane's worktree at merge, is still prose only, and
  nothing under `script/` or `helpers/` prunes or counts worktrees. Checked 2026-09-03.
- **Outstanding.** Budget rule 3, squashing against the recorded base commit, is still prose only
  and no gate reads it; the `git stash` scar beside it in `AGENTS.md` is the same shape. Checked
  2026-09-03.
- **Done.** Budget rule 4 is mechanised: DECISIONS §88 was ratified 2026-08-25
  (`design/decisions/88-needs-architect-as-a-check.md`) and
  `.github/workflows/architect-hold.yml` is a required check that fails any pull request carrying
  the `needs-architect` label.
- **Done.** The unresolved "every fence names its counterpart" row is answered, and the answer is
  that the row named a rule which is not in the file: `AGENTS.md` contains the word "fence" zero
  times, and only the gate in `script/lint` carries it.
- **Outstanding.** Rule 1's gate still greps for inline assembly and `core::arch::` only, and
  `kernel/src/main.rs` still reads an `aarch64_cpu` register outside `arch/`, so the prose cannot
  be cut yet. Checked 2026-09-03.
- **Outstanding.** The `#[path]` gate's `user/src`-only scope is still in nobody's head: check 5's
  comment in `script/lint` does not record it, and the scan is unchanged. Checked 2026-09-03.
- **Recorded.** The naming-conventions table is still ungated apart from check 6, which covers
  `notes/` and `design/` markdown filenames; no case convention for crates, programs, modules or
  `script/` entry points is enforced anywhere.
- **Recorded.** The ledger stays out of `script/lint` and CI: `notes/rule-violations.md` and
  `script/rule-violations` exist and nothing in `script/lint` references either, with the
  git-clobber row marked resolved on §128's basis exactly as this block says.

## Index row

`AGENTS.md` (942 lines today) warns against `pkill` and `git reset --hard` once each, and both had
already been violated when that was written: one lane killed another's emulator mid-test and four
agents clobbered work with a destructive git operation in one day, all having been told to read it
in full. Audited (#309), a cut verified and proposed (2026-08-18, awaiting the integrator), and a
size gate (`script/lint`'s `agents-md-lines` ceiling) plus a violation ledger built (2026-08-22):
the ledger's first run found the git-clobber rule already four strikes over the three-strike
threshold, a decision for calef or the integrator. Only the split (core plus linked documents)
remains, and it too needs an `AGENTS.md` edit no lane can make
