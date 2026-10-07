---
status: NOT-STARTED
raised: 2026-09-19
promoted_from: the-roadmap-after-the-split
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 533. Whether the roadmap should become issues, and what a citation means after the repository split

*(Number provisional until the merge queue lands it.)* Promoted from the proposal `the-roadmap-after-the-split`, filed 2026-09-19, on calef's instruction of 2026-09-20 to give every proposal on `main` a number. The text below is the proposal's own, unedited except for this paragraph: the argument is its author's and promotion is not the moment to improve it. Written by `maintainer/roadmap-after-the-split`, on calef's
question of 2026-09-19: *"We have a lot of machinery now for managing milestones. We could use
issues. Are we sure our current approach is still the right approach? How will we manage this when
we break up the repo and build packages?"* Shaped for the integrator to mint as a
`design/decisions/` section.

The record's *location* is reversible (a `git mv`). The **citation convention is
not**: 3,148 `milestone N` sites in 288 Rust files follow it, and it is in a reader's head. Options
below, with no winner on the identifier. Blocked until answered: nothing today, and that is the
point of raising it now rather than during the split.

**Answered 2026-09-20 by DECISIONS §201 (one roadmap until a citation has to cross), which rules on all three asks: the citation identity is deferred under option E with two conditions, a cross-repository dependency is declared by the blocked side alone, and issues carry a stranger's bug and non-dependency coordination. This file stays as the priced analysis behind that ruling.**

## What is being asked, split into the three questions hiding in it

1. Is the in-tree roadmap still the right medium?
2. Would an issue tracker be better, and at what?
3. What happens to all of it when the split of §151 (the goal of the repository split is
   independent release) arrives?

They have different answers, so they are answered separately. The short version: **(1) yes, for the
layer that gates read and the layer a newcomer reads; (2) yes, at exactly two things, neither of
which is the roadmap; (3) the number breaks and the gloss does not, which is why the identifier is
the only part worth deciding now.**

## The roadmap is three layers, and only one of them is in `design/roadmap/`

Treating it as one thing is what makes the question look hard. Measured in this worktree at
`bfe6a7ae`:

| Layer | What it is | Where it lives today | Size | Can a tracker hold it? |
|---|---|---|---|---|
| **Argument** | why, what was refused, what was measured, the `BUGS` | `design/roadmap/<n>-<slug>.md`, `design/decisions/<n>-<slug>.md` | 441 blocks, 66,304 lines; 194 sections | No. It is co-versioned with the code it describes (see below) |
| **Status** | one word per block, read by gates | the same files | 11 distinct words, 215 `BUILT` | No. A gate cannot read a tracker offline (measured below) |
| **Scheduling** | who is working on what, what is next | **already outside the tree**: `gh pr list --draft` | one draft per lane | It already is one. §90 (the claim is a draft pull request) decided this on 2026-08-16 |

**The third row is the answer to most of the question.** DECISIONS §90 separated claiming from
closing precisely because *"the two want opposite properties. A claim is ephemeral coordination
state, true for two hours and then false forever, and it must be instant. A status is a durable
record, and it should be slow, versioned and gated exactly like every other record here."* The
scheduling layer moved out three years of commits ago in project time, and it moved to GitHub, just
not to Issues. So "we could use issues" is not a proposal to change the roadmap; it is a proposal to
change either the record (row 1) or the gate input (row 2), and those are the two rows a tracker
cannot hold.

**Status words are 11, and 3 blocks carry none.** `**Status: BUILT**` 215, `NOT-STARTED` 173,
`PARTIAL` 35, `SUPERSEDED` 7, `RECORDED` 6, `REMOVED` 3, `PROPOSED` 3, `OPTIONAL` 2, `IN-PROGRESS`
1, `DECIDED` 1, and 3 with an empty status word. A tracker gives you open/closed plus labels, which
is fewer distinctions than the tree already makes and no cheaper to keep accurate.

## What a tracker can and cannot carry, checked rather than asserted

### The four questions from the brief, each answered by a command

**Can a gate read it offline?** No, and the cost is concrete. None of the four gates touches the
network today:

```
$ for f in script/lint script/names script/citations script/roadmap; do
>   grep -nE '^[^#]*\b(gh|curl|wget) ' "$f"; done
(no output)
```

`script/lint` mentions `gh` once, in a comment at line 1927. Moving any status a gate reads into a
tracker makes `script/lint` require a network and a token, on a laptop, in CI, and in every lane
worktree. That is a dependency in the sense of §46 (thin primitives or whole subsystems) and it is
in the gating path.

**Does it version with the code?** No, and this is the one that cannot be worked around. A record in
the tree is evaluated *at the revision you checked out*; a tracker reference is always evaluated at
present time. Demonstrated:

```
$ old=da2330141b76219ba91a9fcaac42719d689d9993   # 2026-08-01
$ git show $old:design/roadmap.md | grep '^### 39\.'
### Repository structure for a loosely-coupled OS, and the road to a distribution
$ git show $old:design/roadmap.md | grep -oE '^#{2,3} [0-9]+\.' | tail -1
### 57.
```

At that commit the tree knows what it meant by milestone 39 (repository structure for a
loosely-coupled OS) and knows that 443 does not exist. An
issue tracker cannot answer either question about that commit: `#39` resolves to whatever issue 39
is today, and nothing records what it was. A code comment citing `#39` is therefore a dangling
pointer into the future, which is the failure §76 (what catches a milestone status that is wrong in
both places) is a whole decision about, with the two records
now unable to disagree because only one of them has a past.

**Does a citation survive a checkout of an old tag?** Today, yes, because the target is in the same
checkout. There is exactly one tag in this repository (`decision-151-model-clarification`) and no
release tags, so this is currently theoretical; it stops being theoretical the moment `basalt`
releases anything.

**What happens if the repository moves or the host changes?** The tree has already run half this
experiment. Milestone 120 (the rename: the OS becomes `nife`) moved the repository from the `calef`
user to the `crickertech`
organization on 2026-08-15 and recorded what survived: *"ruleset, Actions and history survived; the
one casualty was the `TOOLCHAIN_BUMP_PAT` secret."* That was a GitHub *transfer*, which carries
issues. **A split is not a transfer.** It is `git filter-repo` or equivalent, which carries files
and history and carries no issues at all, into a repository whose issue numbering starts at 1. Any
citation that named an issue number would need rewriting, and §194 (sessions interleave rather than
serialize) records what rewriting citations
by number costs: *"a citation rewritten by number can be silently wrong and still pass every gate,
because the section it now names exists."*

### What a tracker is genuinely better at, and this is not a formality

- **A stranger filing something without a clone.** Principle 3 is about a newcomer succeeding
  without asking anyone; it has a filing side this tree has no answer for. A person who hits a bug
  in a released `basalt` image cannot open a pull request against a roadmap block.
- **Cross-repository work items.** After §151 there is no single tree that can hold "this needs a
  change in the kernel and in `basalt`". Today every work item is inside one tree by construction.
- Assignment and notification, which this project substitutes for with `helpers/merge-drain.sh`
  and a steward, both of which `notes/merge-queue.md` is honest do not report their own death.
- A backlog with an owner. `script/roadmap --proposed` shows 24 unnumbered proposals, all
  dated 2026-09-19. The pile is young, so there is no ageing evidence yet, and the block for
  milestone 247 (follow-on work named by a finished milestone goes nowhere) has its own
  reasoning says a due-date gate *"would be routed around by not writing proposals, which is
  worse."* A tracker is better at a backlog than a directory is. Whether it is better *enough* to
  pay for a second place where truth lives is the open part.

### The correction this proposal owes, loudly

**§90 says issues are disabled on this repository. They are not, and they are empty.**

```
$ gh repo view nifeos/nife --json hasIssuesEnabled
{"hasIssuesEnabled":true}
$ gh issue list --limit 5
(no output)
```

That sentence was true when §90 was written on 2026-08-16 and is false today; nobody noticed because
nobody looked. It changes the shape of the question in a useful direction: **the experiment is
already available and has zero uptake.** Nothing has to be enabled, configured or decided to start
using issues for the two things above. That is evidence about demand, not about design, and it is
the cheapest evidence on this page.

## The split is the forcing function, and the number is what breaks

DECISIONS §151 rules the goal is independent release and third-party programs, on the Linux
distribution model: *"The kernel is one independently-released component, not a privileged base."*
A milestone number is one global namespace over one tree. After the split, a number means nothing
outside the repository that holds the block.

### The measurement that decides this, and it rules out the obvious option

Citations are not local to a subsystem. Distinct milestone numbers cited from Rust and Markdown
sources, by directory group:

| Cited from | distinct milestones | distinct `§` sections |
|---|---|---|
| `kernel/` | 168 | 69 |
| `crates/` | 150 | 72 |
| `components/` | 78 | 52 |
| **union** | **207** | **85** |
| **cited from two or more of the three** | **128 (62%)** | **65 (76%)** |
| cited from all three | 61 | 44 |

Occurrence counts, for the size of any sweep: 3,148 `milestone N` in 288 `.rs` files under
`kernel/`, `crates/` and `components/`; 1,796 more in `notes/`; 2,939 inside `design/roadmap/`
itself.

So 62% of cited milestones are cited from more than one of the three groups most likely to become
separate repositories. Per-repository numbering does not decompose the citation graph; it cuts
across it. Whatever the grain of the split turns out to be, the majority of citations are
cross-repository citations the day it happens.

### The second measurement, which is the one with good news in it

`script/citations` classifies every glossed citation by how it resolves:

```
$ script/citations
194 decisions, 440 milestones

GLOSSED CITATIONS (560)
  cross-reference  12
  date             29
  exempt            1
  path             24
  quotation        29
  title           465
```

465 of 560, or 83%, resolve against the target's own title. Nothing requires a gloss;
`notes/citations.md` is explicit that *"the rule is 'what you write is checked', not 'you must write
it'"*. Authors wrote titles anyway, because a number does not say what it names and they wanted the
sentence to be readable.

That is the finding underneath every option below. **A citation of the form `milestone N (lanes
wait on each other for three reasons)` already resolves without the number**, by `grep` in any
repository, under any renumbering, after any move. The tree is 83% of the way to a
split-proof citation convention by habit, and nobody decided to do it.

### Options for what a citation means after the split, priced. No winner.

A. Records travel with their code; each repository numbers its own.
Blocks move to the repository they describe; `nife-kernel` milestone 1 (boot to Rust on QEMU
`virt`) and `basalt` milestone 1 both
exist. *Cost:* the 62% figure. Every cross-group citation needs a repository prefix added by hand or
becomes ambiguous, and `script/citations`' path mode (24 citations) breaks outright since paths stop
resolving in the local checkout. Its title mode (465) survives only if the sibling repository is
checked out. *Kills the strongest gate this tree has for cross-repository citations.*

B. Per-repository prefixes on a single scheme, `nife-kernel#443`, `basalt#12`.
*Cost:* a new thing every reader learns, doing real work in 62% of citations rather than the rare
case, plus a mechanical rewrite of 3,148 sites whose failure mode §194 names: silently wrong and
still passing. *Buys:* nothing that C does not buy, unless numbers stay authoritative.

C. The slug becomes the identity; the number becomes a nickname.
The record is `design/roadmap/decouple-the-lanes.md`; the number is a display convenience that may
be dropped, duplicated across repositories, or left with gaps. Citations resolve by the gloss, which
83% of them already do. *Cost:* the tree must stop treating the number as authoritative, which
`script/roadmap --check`, `script/lint` check 4b and the `milestone/N-*` branch convention all do
today. *Buys:* every existing well-formed citation survives the split untouched, and no sweep is
required, because the gloss is already there in the cases that matter.

D. A citation resolver that knows several repositories.
`script/citations` grows a manifest of sibling checkouts and resolves across them. *Cost:* a gate
whose answer depends on what the operator has cloned, which is the opposite of the offline property
measured in section 2 above, and which degrades to a warning in CI. *Buys:* it is the only option
that
keeps a machine-checked cross-repository citation at all, so it is probably a companion to C
rather than an alternative to it.

E. One roadmap forever, in `basalt`; code repositories cite it and hold none of it.
*Cost:* it contradicts "records travel with their code" and re-creates a single global namespace
that every independent release depends on, which is the coupling §151 exists to remove. *Buys:*
nothing changes for 441 blocks or 3,148 citations, and the split touches code only.

### The mechanism the prior art hands us for free

FreeBSD's ports tree has run exactly this problem for thirty years and solved it with a flat file.
From the Porter's Handbook: each line of `MOVED` is *"the name of the port, where the port was
moved, when, and why,"* in the form `old name|new name (blank for deleted)|date of move|reason`,
and *"if the port was removed, the section detailing where it was moved can be left blank."*

A `design/roadmap/MOVED` of that shape, written at the split, turns every option above from
lossy into merely indirect: a number that no longer resolves locally resolves to a line saying where
it went and why. It is rung three of the ladder (a written record at the thing itself), it costs one
file, and it is the only part of this page that is cheap in every option.

## Prior art, read rather than recalled

Fetched 2026-09-19. Every quotation below was read from the URL given.

| Project | Durable record | Identifier | Minted | Tracker's job |
|---|---|---|---|---|
| **Rust RFC** | in-tree `text/NNNN-slug.md` in `rust-lang/rfcs` | number **and** slug | the **pull request number**, at PR-open | a tracking issue in a *different* repo holds implementation status |
| **Go proposal** | in-tree `design/NNNN-shortname.md` | number and shortname | the **GitHub issue number** | the issue **is** the identity and holds the state |
| **Kubernetes KEP** | in-tree `keps/sig-x/NNNN-short-name/README.md` + `kep.yaml` | number and short name | the **tracking issue number** | issue is the breadcrumb; `kep.yaml` holds status and `replaces`/`superseded-by` |
| **Fuchsia RFC** | in-tree `//docs/.../rfcs/`, indexed by `_rfcs.yaml` | `RFC-NNNN` | by the Eng Council **at acceptance** | a filed bug, linked from the RFC header, for implementation |
| **Python PEP** | in-tree rST in `python/peps` | number | by PEP editors at draft approval | separate; routine work never becomes a PEP |
| **Debian** | `debian/changelog` in the source package | global bug number, package binding is a mutable field | the BTS | the BTS is the record; the tree references it by string |
| **FreeBSD ports** | `Makefile`, `distinfo`, plus tree-wide `UPDATING` and `MOVED` | port origin path (`category/name`) | by the tree | Bugzilla PR number, referenced from commits by string |
| **Homebrew** | formula file in a tap | **`user/repo/formula`**, a path-shaped name | by the tap | issues per tap |

Four things this table says that are worth saying out loud.

Every one of the eight keeps the durable record in a versioned tree. Not one of them put the
argument in the tracker. The projects that use a tracker heavily (Go, Kubernetes) use it for the
*number* and the *state*, and still check the design document into a repository. That is a strong
answer to question (1), and it is unanimous.

The dominant pattern is that the tracker mints the identifier and the tree holds the record.
Rust: *"Don't assign an RFC number yet; This is going to be the PR number and we'll rename the file
accordingly if the RFC is accepted."* Go: *"A proposal is a suggestion filed as a GitHub issue,
identified by having the Proposal label,"* and *"The design doc should be checked in to the proposal
repository as `design/NNNN-shortname.md`, where `NNNN` is the GitHub issue number."* Kubernetes:
*"KEPs are now prefixed with their associated tracking issue number. This gives both the KEP a
unique identifier and provides an easy breadcrumb."*

This tree already opens a claim pull request as a lane's first act (§90), and GitHub already mints a
unique never-reused number for it. The hand-minted contiguous number that two sessions collided on
three times in one day is a number GitHub was already giving us and we were throwing away. That is
a real option and it is listed as such in the BUGS below, not recommended here, because PR numbers
are per-repository and therefore have option B's problem after the split, and because reconciling
441 existing blocks with a PR counter now past 999 is §194's dangerous edit at full scale.

**Every cross-reference mechanism in the table is a plain string convention, not a link.** Debian's
deb-changelog(5): *"If this upload resolves bugs recorded in the distribution bug tracking system,
they may be automatically closed on the inclusion of this package into the distribution archive by
including the string: `Closes: #nnnnn` in the change details."* FreeBSD: *"Many FreeBSD changes refer
to the PR number that prompted the change."* Homebrew qualifies across taps by path:
*"Use the fully qualified name to select the formula from a particular tap: `brew install vim` /
`brew install username/repository/vim`."* Nothing structural survives a split; a grep-able string
does. Option C is that observation applied to this tree.

A project that moved, with its reason, since the brief asked for one. LLVM moved from Bugzilla
to GitHub Issues, and the RFC's stated reason is entirely about the tool, not about where records
belong: *"Our bugzilla installation is...not great. It's been not-great for a long time now,"* and
*"Importantly, Github Issues is significantly less user-hostile than our bugzilla is, for new
contributors and downstream developers who just want to tell us about bugs!"* The move is
tracker-to-tracker for bug reports from strangers, which is the first item in section 2's "genuinely
better at" list and not the roadmap.

A correction to this lane's own research, recorded because this tree carried a fabricated block
quote for twelve days. A first pass reported that the LLVM RFC discussed preserving `PRxxxxx`
numbers already baked into source comments, which would have been a direct precedent for the 3,148
sites here. Fetching the page found no such discussion. The claim is withdrawn and is not used
anywhere above. Rust's move in the opposite direction is also worth one line since it was checked:
the compiler team's MCP process put a *lighter* tier of proposal into tracker issues while leaving
the heavy tier in the RFC repository, which is a proportionality argument rather than a medium one.

## Recommendation, with the test from §92 (a caretaker is supervised by the client it serves)

**Keep the argument and the status in the tree. Do not move the roadmap to issues. Enable nothing
and decide nothing about the identifier today, and instead spend one ratchet on the gloss, which is
what makes the identifier decision cheap whenever it is taken.**

Concretely, three items, smallest first:

1. A ratchet on new citations: a `milestone N` or `§N` on a line this commit adds must carry a
   gloss. Answered and built on 2026-09-20: calef approved it the day this was filed, and
   milestone 444 (a citation says what it cites) shipped it as `script/citations --ratchet`, wired
   into `script/lint`. Its census also corrected this file's own number, and the correction is worth
   carrying: the 83% below is 83% *of the citations that already carry a gloss*, which is 505 of
   9,483 (file, scheme, number) pairs, or 5.3% of the tree. The ratchet's rate argument is
   unaffected and the starting point is far lower than this file assumed. Two limits the ratchet
   turned out to have are in `notes/citations.md`'s BUGS. `script/lint` already has this shape at line 1995 (`git merge-base HEAD origin/main`),
   and the word "ratchet" appears in it eleven times. This answers `notes/citations.md`' standing
   objection exactly, which is about a *sweep*: *"Requiring a gloss on the first mention of each
   number in each file means 2,911 sites, every one of which has to be read to know what its author
   meant... So the rule is 'what you write is checked', not 'you must write it'."* A ratchet reads
   nothing retroactively and writes no gloss mechanically. It moves the tree from 83% split-proof to
   asymptotically 100%, at the rate the tree changes, starting now.
2. Write `design/roadmap/MOVED` when the split happens, in FreeBSD's four-field form. One file,
   and it makes every option in section 3 survivable.
3. Use issues for the two things a tree cannot do, as soon as either one is real: a stranger's
   bug report on a released artifact, and a work item spanning two repositories. Not for milestones,
   not for status, not for claims. §90's reason against a second place for truth is intact; that
   reason is about the *same* fact in two places, and neither of these facts is in the tree at all.

The §92 test. *Would I still recommend this if all the options cost the same?* Yes, and the
reason is not effort. A slug says what it names and a number does not; that is why 465 of 560 glosses
are titles although nothing required them. An in-tree record answers a question about the revision
you are standing on and a tracker cannot; that is a property, not a saving. The one place cost is
genuinely deciding is the refusal to renumber, and that is said plainly rather than dressed up:
renumbering 441 blocks is cheap in agent-hours and expensive in §194's terms, and the recommendation
is to not do it because it is dangerous, not because it is long.

Reversibility, per the *move fast on what can be undone* test, which is "who else has already
acted on this". The gloss ratchet is reversible: delete the check, keep the glosses. Which script
reads what is reversible; the decouple-the-lanes lane on #999 is proving it this week by moving two
gates off the generated index. `MOVED` is append-only and additive. The identifier convention is the only
irreversible thing on this page, which is why it is presented as five options with no winner, and
why item 1 exists: a gloss ratchet makes options A, B, C and E converge, so the decision gets cheaper
every week it is deferred rather than more expensive. That is unusual enough to be worth stating,
because the normal shape of a deferred decision here is the opposite.

## What it costs to do nothing, after the decouple-the-lanes work lands

The in-flight work on #999 removes three couplings, and they should not be re-argued here. From its
block: `script/fatal-risks` reads the blocks rather than the generated index; `script/audits` does
too; and `script/decisions` no longer fails on a gap, with `design/naming.md` gaining the rule
*"Both schemes are sparse: a duplicate is the defect, a gap is not."* A new `script/lint` check stops
the index coupling growing back, allowing only `script/metrics` and `script/catch-up`, which read
historical revisions where no block-derived answer exists.

What remains painful after that, which is the real case for changing anything further:

- The number is still hand-minted and still global. 443 makes a collision cheap to *resolve*
  (take the next free number, leave the hole) rather than impossible to *have*. Two sessions that
  cannot see each other still both reach for the next number, and the integrator still arbitrates at
  merge. The prior art's answer to this (let the tracker mint it) is untouched by 443.
- **The citation identifier is a bare number and 443 does not look at it.** This is the whole of
  section 3
  above and it is entirely unaffected.
- 21 scripts under `script/` read `design/roadmap/`, not 12 as first estimated: `audits`, `bench`,
  `board-netboot`, `boot-check`, `catch-up`, `ci-build`, `citations`, `fastpath-footprint`,
  `fatal-risks`, `icount`, `job-mix`, `journeys`, `lint`, `metrics`, `names`, `repeat-under-load`,
  `roadmap`, `swish-check`, `vendor-verify`, `vendor-watch`, `verify`, plus `xtask/src/main.rs` and
  two files under `helpers/`. Every one of them is a reason the record must stay readable from a
  checkout with no network. 443 changes which *file* two of them read, not that they read the tree.
- The proposals pile has no owner. 24 files, all dated the same day, promoted only by an
  integrator who happens to look. Milestone 247 argued a due-date gate would be routed around, and
  that argument is good; it leaves the pile's health depending on someone noticing, which is rung
  zero in the vocabulary AGENTS.md uses for exactly this.
- Nothing here works for a person without a clone, and §151's third-party programs are the point
  of the split.

So the honest answer to "are we sure our current approach is still the right approach" is: yes for
the record, yes for the gates, and the scheduling layer already left. What is not right is that
the identifier is a number, and that is not a problem 443 was scoped to touch.

## BUGS

- **This page does not price letting GitHub mint the number**, although the prior art points
  straight at it and it would dissolve the decouple-the-lanes block's third cause. It is left as an
  option in section 4
  because it has option B's post-split problem, because 441 existing blocks cannot be reconciled with
  a PR counter past 999 without §194's dangerous edit, and because it would make `script/lint` and
  `script/roadmap` depend on numbers minted by a host this project might leave. A lane could price it
  properly; nobody has.
- The 62% cross-group figure assumes the split's grain is `kernel/` / `crates/` / `components/`.
  §151 explicitly does not decide the grain (*"This decides the goal, not the grain"*), so the figure
  is an indication rather than a prediction. The direction is robust: any grain finer than one
  repository makes some citations cross-repository, and the citation graph was never built to
  respect a boundary that did not exist.
- **The gloss ratchet is not free of false positives and this page has not measured them.** A line
  that mentions a number in a sentence that is not a citation would
  trip it, and `git grep -w TODO`'s 82% false-positive rate is this tree's standing warning about
  greps that judge prose. Whoever builds it should measure the rate against the last two hundred
  commits before turning it into a failure rather than a report.
- Nothing here measures how often a citation is actually followed. The whole argument that
  citations must resolve rests on the assumption that readers follow them. That is plausible, it is
  what `script/citations` exists for, and it is unmeasured.

## Index row

Answered 2026-09-20 by DECISIONS §201 (one roadmap until a citation has to cross), which rules on
all three asks: the citation identity is deferred under option E with two conditions, a cross-
repository dependency is declared by the...
