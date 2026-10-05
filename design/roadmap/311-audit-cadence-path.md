---
status: BUILT
raised: 2026-09-17
built: 2026-09-17
---
# 311. The audit cadence tripwire, and the month of correct alarms nobody acted on

Minted by the maintainer after `script/cadence-check` (milestone 238)
reported `audit cadence` as a workflow with no successful run. Built by a lane on
`milestone/311-audit-cadence-path`.
*(Number provisional until the merge queue lands it.)*

## What it reports, first, because it is the deliverable

`script/audits`, repaired, on 2026-09-17:

```
$ script/audits
documentation  last 2026-08-17 (The ABI surface as documented, read from the wire outward: e)
  milestones built     76 -> 188  (+112, fires at 10)   <-- FIRED
  components           110 -> 155  (+45, fires at 10)   <-- FIRED
  ABI constants        50 -> 52  (+2, any change fires)   <-- FIRED
  external packages    108 -> 108  (+0, fires at 30)
  calendar             31 days since (12 weeks)
  DUE: milestones built +112 (fires at 10); components +45 (fires at 10); ABI constants +2 (fires at 1)
  ? has a subsystem been rewritten inside its existing crate since the last sweep? That moves no count here, and the note describing it from the outside is where it shows up.
  ? has a decision superseded a plan that a note still prescribes? notes/cpu-models.md's closing line was the 2026-08-03 exhibit.

security      last 2026-08-17 (Newly minted authority, read adversarially: the seven ABI co)
  milestones built     76 -> 188  (+112, fires at 15)   <-- FIRED
  components           110 -> 155  (+45, fires at 8)   <-- FIRED
  ABI constants        50 -> 52  (+2, any change fires)   <-- FIRED
  external packages    108 -> 108  (+0, any change fires)
  calendar             31 days since (6 weeks)
  DUE: milestones built +112 (fires at 15); components +45 (fires at 8); ABI constants +2 (fires at 1)
  ? has a new component taken device or network authority since the last audit?
  ? has this booted on a new machine class (a board, a cloud) since the last audit?

The `?` lines are triggers nothing here can count, because they are a judgment and not a
number. If one is yes, that kind is due regardless of everything above (milestone 92, §74).

audits: 7 on record, DUE: documentation, security
Red means run the audit. Nothing here ran one, and nothing here can.
design/audit-reports/README.md says how, and which lens the last one lacked.
```

Both kinds are due, and the security one by seven times its own trigger. §74 set the security
count at 15 milestones or 8 components; 112 milestones and 45 components have landed since the last
audit of any kind. The calendar backstop, the trigger a reader reaches for first, is the only one
that has *not* fired (31 days against 42), which is the count triggers doing exactly the job §74 gave
them. Neither kind of audit is this milestone's work: this block hands over a question, not an
answer, and the Follow-on says where it went.

## The premise this milestone was minted on is wrong, and the correction is the finding

The brief was that the workflow had been dead for five weeks on a stale path. That is what the
Actions tab looks like: five scheduled runs, five red. Reading the logs rather than the colours says
something else.

```
2026-08-17  audits: 5 on record, DUE: documentation, security   exit 1
2026-08-24  audits: 7 on record, DUE: documentation, security   exit 1
2026-08-31  audits: 7 on record, DUE: documentation, security   exit 1
2026-09-07  audits: 7 on record, DUE: documentation, security   exit 1
2026-09-14  FileNotFoundError: .../user/Cargo.toml              exit 1
```

Four of the five red runs are the tripwire working. The path defect is the fifth and it is four
days old, not five weeks: milestone 175 split `user/` into `components/` and `fixtures/` on
2026-09-13, and the next scheduled run broke. What the record actually shows is that **an audit has
been overdue every week since 2026-08-17, this mechanism said so on schedule every single time, and
no audit was run.** Milestone 92 built a tripwire to stop auditing depending on somebody
remembering; the tripwire fired for a month, and auditing still depended on somebody remembering.

Why a month of correct alarms was as invisible as silence. Red *is* this job's signal, by
deliberate design (see `.github/workflows/audit-cadence.yml`'s own header: an audit coming due is
information, not a defect, so it gets its own workflow rather than a gate). The consequence nobody
priced is that the Actions tab shows the same colour whether the tripwire is firing or the tripwire
is broken. A reader who has learned that this job is "the red one" has stopped distinguishing the
two, and the path defect then hid inside the alarm it replaced. Milestone 238's `script/cadence-check`
found it by asking when each workflow last *succeeded*, which is the right question for a dead job
and the wrong one for a live one: a job whose healthy state is red has no green to be stale against.
It happened to catch this one because this one's healthy state is red *and* it had never succeeded,
which is luck rather than coverage.

This is AGENTS.md's ladder read from the far end. The cadence check is rung two, a gate that
fires without being remembered, and it fired. What has no rung at all is the step after: a red run
that repeats identically four times is nobody's, in exactly the way the two green pull requests of
2026-08-04 were nobody's. Both follow-on items below are about that step rather than about this
script.

## The repair

`script/audits` line 167 and its line 42 comment. `user/Cargo.toml` became
`components/Cargo.toml` plus `fixtures/Cargo.toml`.

Both packages, and the reason is the baseline table rather than the word "component." Reading
only `components/` is the tempting repair, and milestone 175's own classification rule supports it: a
fixture is a test client or a stand-in server, and a distribution would not ship one. It is also
wrong, quietly. Every baseline row in `design/audit-reports/README.md` was counted when `user/` held
both halves, so a delta taken against `components/` alone compares 114 with a baseline of 110 that
meant 155, under-reporting the components trigger by forty. An audit signal may err toward firing,
which the script's own `BUGS` already says about the ABI count; it may not err toward silence. The
split moved no program across the boundary (72 `[[bin]]` targets the commit before, 73 the commit
after), so counting both is what keeps the number continuous, and continuity is the only property a
delta needs.

**No second cause sat behind the first.** After the fix all five modes run end to end: the report
(exit 0), `--due` (exit 1, correctly), `--check` (exit 0, and it is what `script/lint` runs),
`--baseline`, `--worklist`, and an unknown argument still exits 2.

## The sweep, and the one it found that was not crashing

How. `git grep -nE '(^|[^a-z/_-])user/'` across the tree with `kernel/src/user` and `vendor/`
excluded, then every hit read for tense. Milestone 175 left a great deal of prose that names `user/`
**correctly**, because it is describing what was true before the split ("it was `user/link.ld` until
milestone 175", `crates/user_mode_runtime`'s account of a 123-call-site sweep). AGENTS.md's rename
rule says a record of the past keeps the old name, so those are not defects and were not touched.
The defects are the present-tense ones, and the test that separates them is whether a reader would
try to open the path today.

Two were live, and both are in this milestone's diff. The second is the one worth the sweep:

`script/roadmap`'s `PATHISH` list had stale and missing entries, and only the missing ones
mattered. That check decides whether a backticked span in a `**Recorded.**` bullet is a path claim,
by testing it against a hand-typed list of "a directory this repository actually has". The list still
said `user`, and named neither `components` nor `fixtures`; `.cargo` and `.githooks` had never been
in it. A stale entry is harmless, since a dead `user/...` citation would be caught. **A missing entry
is not**: a bullet citing `components/src/...` was not recognised as a path claim at all, so the
broken footnote this check exists to catch walked straight past it, reported clean.

It is now derived from `git ls-tree -d --name-only HEAD` instead of copied from the tree by hand,
which is rung one rather than rung two: the list cannot drift again because there is no list.
`git ls-tree` rather than `os.listdir` so that an untracked `target/` is not a directory a citation
can resolve into.

**It found two real broken footnotes on its first run**, both in milestone 289's block, both citing
`components/src/builder.rs`, which calef retired on 2026-09-14 (milestone 295). One limitation still
stands and its surviving home is `notes/trusted-init.md`, where that bullet already said it lived;
the other bullet asked calef for a category and a name, and retiring the program answered both out of
existence, so it is now `**Done.**` citing commit `0fa40ee8`. That block is not this lane's, the
edits are marked as such in it, and they are named in the Follow-on below.

## BUGS

- A red run that repeats is still nobody's. This milestone fixed the path and changed nothing
  about the reason four correct alarms were ignored. `script/cadence-check` reports a workflow with
  no recent success; nothing reports a workflow that has been red for the same reason four weeks
  running, and for this workflow specifically those two states are the healthy one and the ignored
  one. See the first Follow-on bullet.
- **The tree-wide staleness this sweep touched is a sample, not a sweep.** Eight distinct
  `components/` and `fixtures/` paths cited in markdown across the tree do not resolve. Six are
  historical prose and correct; the two that were not are fixed here. Nothing gates the rest, and
  `script/roadmap`'s check reaches only `**Recorded.**` bullets in roadmap blocks, which is a few
  dozen lines of a 270-document corpus. `script/lint`'s own header (the 2265 comment) already
  records why a tree-wide version of this check stays ungated: the false-positive rate needs a
  reader, and that reader is the documentation sweep.
- Counting `fixtures/` as a component is a continuity choice, and it overcounts what §74 meant.
  §74's event trigger is "a new component holding device or network authority", and a fixture holds
  neither by construction. The count now fires on a new test client too. That is the tolerable
  direction and it is the same tolerance the ABI count already takes, but it means the components
  number is a proxy for tree churn rather than for attack surface, and a future audit that wants the
  narrower question has to ask it by hand. Re-baselining both tables against `components/` alone
  would fix it and would also discard every historical row's comparability; that trade was not
  worth making inside a repair.
- Nothing here ran an audit. Both kinds are due, hard, and this block is the record of a
  tripwire, not of a review. Closing the alarm by adding a row to the index is available, cheap, and
  the one thing that makes the mechanism a lie; the index says so too.

## Follow-on

- **Milestone 422.** Nothing
  notices a scheduled workflow that has been red for the same reason four weeks running, and for
  `audit-cadence` that is precisely the state the mechanism exists to produce. `script/cadence-check`
  asks when a workflow last succeeded, which cannot distinguish a firing tripwire from a broken one.
  The four identical `DUE: documentation, security` runs between 2026-08-17 and 2026-09-07 are the
  worked example.
- **Milestone 313.** The security audit both trigger sets said was due, overdue by 112 milestones
  against a threshold of 15. It sits on `design/fatal-risks/README.md`'s risk 7 path. It was routed through
  a proposal rather than a lane brief because the lens was calef's call and 112 milestones is more
  tree than one lens can hold.
- **Milestone 427.** The
  documentation sweep the same run says is due, arriving with its scope already computed by
  `script/audits --worklist`. The eight dead `components/` and `fixtures/` citations in the second
  BUGS entry above are a starting scope.
- **Recorded.** That counting `fixtures/` as a component overcounts §74's event trigger stays a
  limitation, written in the third BUGS entry above and in `design/audit-reports/README.md` beside
  the baseline table, which is where a reader meets the number.
- **Done.** Two bullets in `design/roadmap/289-the-riscv-tour-earns-its-place.md`'s Follow-on were
  edited by this lane, which is not that block's own, because `script/roadmap --check` began failing
  them the moment its directory list stopped being wrong. Both cited a file milestone 295 deleted.
  The edit is marked as this lane's inside that block.

## Index row

`.github/workflows/audit-cadence.yml` is milestone 92's tripwire and sits on `design/fatal-risks/README.md`
risk 7's path; it had never once succeeded, and `script/cadence-check` reported it. The stale path
was real and four days old (milestone 175 split `user/` on 2026-09-13 and the next scheduled run
died on `user/Cargo.toml`), but reading the logs rather than the colours inverted the premise this
milestone was minted on: four of the five red runs were the tripwire working, and an audit has been
overdue every week since 2026-08-17 with nobody acting on it. Red is this job's signal by design,
so a firing tripwire and a broken one are the same colour in the Actions tab, and the defect hid
inside the alarm it replaced. The count now spans `components/` **and** `fixtures/`, because every
baseline row was taken when `user/` held both and reading only the first would under-report the
trigger by forty, which is the one direction an audit signal must not err. The sweep for other
casualties of the split found one that was not crashing and mattered more: `script/roadmap`'s list of
real root directories still said `user` and knew neither new name, so a `**Recorded.**` bullet citing
`components/src/...` was not recognised as a path claim at all and the broken footnote the check
exists to catch passed it in silence. That list is now derived from `git ls-tree` rather than typed,
and found two dead citations to milestone 295's retired `builder` on its first run. Both audits are
now due, hard: 112 milestones against a security threshold of 15, with the calendar backstop the only
trigger that has *not* fired.
