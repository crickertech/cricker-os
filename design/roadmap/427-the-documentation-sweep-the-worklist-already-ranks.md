---
status: NOT-STARTED
raised: 2026-09-17
promoted_from: the-documentation-sweep-the-worklist-already-ranks
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 427. The documentation sweep the worklist already ranks

Promoted from the proposal
`the-documentation-sweep-the-worklist-already-ranks`, filed 2026-09-17 by milestone 311 alongside
its security sibling; the same repaired tripwire reported both. *(Number provisional until the merge
queue lands it.)*

`script/audits --worklist` runs today and needs nothing built.

Premise re-checked 2026-09-19: still due, more overdue, and the sibling is gone.
`script/audits --due` still names `documentation`, last swept 2026-08-17, now at +122 milestones
against a trigger of 10 and 33 days on the calendar. The security half is no longer due: it was
audited on 2026-09-17 by milestone 313, so `documentation` is the only standing example rather than
one of a pair. The worklist head has also moved since this was filed, which is the ranking working
as designed rather than a correction: `notes/architecture-list-sweep.md` now leads at 20 of 20 cited
paths moved over 117 commits, and `284-finish-the-progenitor-rename.md` has fallen to 11 of 16.
Re-run `script/audits --worklist` rather than working from the table below.

## In brief

```
documentation  last 2026-08-17 (The ABI surface as documented, read from the wire outward: e)
  milestones built     76 -> 188  (+112, fires at 10)   <-- FIRED
  components           110 -> 155  (+45, fires at 10)   <-- FIRED
  ABI constants        50 -> 52  (+2, any change fires)   <-- FIRED
  external packages    108 -> 108  (+0, fires at 30)
  calendar             31 days since (12 weeks)
```

Overdue by eleven times the milestone trigger. Unlike its security sibling this one arrives with its
scope already computed, which is why it is the cheaper of the two to brief.

## The worklist, which is the scope

`script/audits --worklist` ranks every document by how many of the files it cites have moved since
the document was last edited. Its head on 2026-09-17:

```
 moved/cited  commits  last edit   document
   16/16           72  2026-09-14  design/roadmap/284-finish-the-progenitor-rename.md
   14/19           53  2026-09-14  design/roadmap/158-kernel-object-rename-build.md
```

Read `notes/documentation-audit.md` first. It is the procedure, it says what counts as a finding,
and it is honest that the ranking has never read a sentence: a document at the top may be perfectly
true, and one absent from the list may be a year out of date. 138 of 269 documents cite no resolvable
code path and are simply not on it.

## One scope this sweep can start from, measured rather than guessed

Milestone 311 swept for paths that milestone 175's `user/` split moved, and found the sweep is
larger than its own lane. **Eight distinct `components/` and `fixtures/` paths cited in backticks in
markdown across the tree do not resolve today.** Six of them are historical prose and correct under
AGENTS.md's rename rule, which says a record of the past keeps the old name; two were live rot and
are fixed in 311. Separating those two categories needs a reader, which is the whole argument for
routing this to a sweep rather than to a gate, and `script/lint`'s own comment on the matter says so.

`design/roadmap/383-a-backticked-path-that-does-not-resolve.md` is the standing proposal for
gating this class tree-wide. It is worth reading alongside, and it is worth noting that its own
enumeration of root directories still lists `user/`, three days after that directory stopped
existing, which is a small exhibit for why the sweep is due.

## What closes it

A report in `design/audit-reports/`, both index rows, every finding dispositioned. `script/audits
--due` stops naming `documentation` on its own.

## Index row

The documentation audit was last run on 2026-08-17 and is overdue by eleven times its milestone
trigger, which `script/audits --due` has reported every week since. Unlike the security audit it
arrives with its scope already computed: `script/audits --worklist` ranks every document by how many
of the files it cites have moved since the document was last edited, and
`notes/documentation-audit.md` is the procedure and is honest that the ranking has never read a
sentence, so a document at the top may be perfectly true and 138 of 269 cite no resolvable code path
at all. One measured scope to start from is milestone 311's finding that eight distinct
`components/` and `fixtures/` paths cited in backticks across the tree do not resolve, six of them
historical prose that the rename rule says to keep and two live rot, which is the argument for
routing this class to a reader rather than to a gate. What closes it is a report in
`design/audit-reports/`, both index rows, and every finding dispositioned.
