---
status: NOT-STARTED
raised: 2026-09-19
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 436. Forty citations of a proposal path that now resolve to nothing

Minted 2026-09-19 by milestone 434's lane, which found the citations while
retiring the token and deliberately did not fix them. It minted this as 435 and the integrator
renumbered it to 436 at merge, because a concurrently-running maintainer session had minted 435 for
a different sweep from a commit this lane's base predates. That is the collision the rule adopted the
same evening predicts, on its first use, resolved the way that rule says: the newer file moves and
the older number stands, so a block already collecting citations never renumbers. It cost one
`git mv` and four lines of `sed`, which is the measured answer to whether provisional numbering is
affordable. *(Number provisional until the merge queue lands it.)*

Every one of them is a path in a file already in this tree, and the hard half is
reading rather than deciding.

## What is broken

`design/roadmap/proposals/` was drained by milestone 433 and no longer exists. About forty
backticked citations of `design/roadmap/proposals/<slug>.md` survive it, spread across roughly
twenty-seven files: milestone blocks (296 has five, 267 and 304 have three each), two audit reports
under `design/audit-reports/`, `design/naming.md`, `design/decisions/0152-port-range-capability.md`,
four notes, `kernel/src/pci.rs` and `helpers/qemu-runner-x86_64.sh`.

A handful of finished blocks also describe the directory in the present tense, which is a
smaller defect of the same family: milestone 276's block says `script/roadmap --proposed` "already
computes" a number, and milestone 301 (one grant order for the progenitor)'s says `helpers/roadmap_proposals.py` "matches PROPOSED and
nothing else, so a proposal cannot be retired" in place. Both were true when written. A BUILT block
is a record of its moment and the tree's convention lets older records describe the past, so these
want a dated amendment rather than a rewrite, and deciding which of the two they get is part of
this work.

A citation that resolves to nothing is a broken footnote, which is the defect `script/roadmap`'s
own `**Recorded.**` path check exists to catch, one directory over. None of these is caught, because
nothing checks a backticked path outside a follow-on bullet.

**They are readable, which is why this is a milestone and not an emergency.** Milestone 433 chose
deliberately to keep the slug when it numbered each proposal, so
`proposals/<slug>.md` became `<N>-<slug>.md` and a reader resolves one with a single
`ls design/roadmap/ | grep <slug>`. That was the right call at the time and it does not make the
paths true.

## The two halves, and only one is mechanical

The resolvable ones. Where the slug survives on a numbered block, the rewrite is mechanical:
`design/roadmap/303-x86-64-fs-disk.md` cites
`design/roadmap/proposals/an-unclaimed-function-behind-the-iommu.md` in its body, and its own
`## Follow-on` section already names milestone 325 for the same item, so the body is simply behind
its own block.

**The ones whose slug has no numbered file are the work.** Milestone 433 recorded about thirty
proposals that earlier promotions deleted without a one-to-one successor, because the work was
folded into a cluster block. Mapping one of those wants a reading of the cluster block to find where
its content went, and a citation rewritten to the wrong block is worse than one that dangles, since
a dangling path announces itself and a wrong one does not.

## What would make it stick

A rewrite alone is rung four and will rot the same way. The cheap mechanism is the one this tree
already runs elsewhere: teach a gate that a backticked `design/roadmap/...` path has to exist, the
way `script/roadmap` already checks the paths a `**Recorded.**` bullet names. Whether that belongs
in `script/citations` or in `script/roadmap`, and how wide its noise surface should be, is the part
worth pricing before writing.

## Index row

Milestone 433 drained `design/roadmap/proposals/` and kept each proposal's slug on the numbered
block that replaced it, so a reader can still resolve one by hand. About forty backticked citations
of the old paths survive across twenty-seven files, in milestone blocks, two audit reports,
`design/naming.md`, four notes, one kernel source file and one shell script, and nothing in this
tree checks a backticked path outside a follow-on bullet. Roughly ten are mechanical, because the
slug still names a numbered block; the rest are the ones earlier promotions folded into cluster
blocks without a one-to-one successor, and each of those wants a reading of the cluster block,
since a citation rewritten to the wrong block is worse than one that dangles.
