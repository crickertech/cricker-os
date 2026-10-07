---
status: NOT-STARTED
raised: 2026-09-15
promoted_from: the-two-loader-names-the-tree-still-carries
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 416. Two loader names the tree still carries, and neither one exists

Promoted from the proposal `the-two-loader-names-the-tree-still-carries`,
filed 2026-09-15 by the lane that performed milestone 166's ratified
`spawn_progenitor` -> `spawn_hello` rename (PR #884), which found these while enumerating and
deliberately did not sweep them. *(Number provisional until the merge queue lands it.)*

`boot_progenitor` is already ratified (calef, 2026-09-15), so nothing here waits on a
naming decision. It is bounded cleanup with a discipline attached, not a fork.

Premise re-checked 2026-09-19 and the count has grown, which is why this file said to re-enumerate
rather than trust it. Neither `fn riscv_shell_boot` nor `fn spawn_init` exists anywhere in the
tree. `riscv_shell_boot` appears 48 times across 25 files and `spawn_init` 32 times across 22
files, against the roughly 35 occurrences counted mid-rename on one branch. The sites span
`design/`, `notes/`, `kernel/`, `components/` and `xtask/`, and include the test-wiring hotspot this
file's `BUGS` warns about.

## The finding

Two function names survive in prose and code comments after the functions themselves are gone:

- `riscv_shell_boot` became `boot_progenitor` at milestone 166, when the per-architecture boot
  loaders were unified.
- `spawn_init` predates 166 and was already stale before it.

The lane counted roughly 35 occurrences across `design/`, `notes/`, `kernel/` and `xtask/`. Treat
that number as a starting point and re-enumerate rather than trust it: it was counted on one
branch, mid-rename, and this tree's rule is that a count spanning the tree is taken at merge from the
merged tree.

## Why it matters, and why it is not cosmetic

A reader who greps `riscv_shell_boot` finds prose describing a function that exists nowhere, with
nothing telling them what it became. That is the newcomer principle failing in the specific way this
tree cares about: the question is answerable only by asking someone, and a contributor who has to ask
will leave instead. The same sweep that fixed `spawn_progenitor` proved the cost is small; the reason
it was not done at the same time is discipline, not effort.

## Why milestone 166's lane did not do it

Folding a sweep about *other* names into a ratified rename would have been the wrong sweep. This
tree's recorded scar is a blind `sed` that swept a rename across the tree and rewrote the very row
recording that a name had been *refused*. A rename lane that quietly widens its own scope is how that
happens, so the lane enumerated, reported, and stopped, which is the behavior the rules ask for.

## What doing it involves

Mechanical in the edits, and the judgment is the whole job. The same rules milestone 166's rename
followed apply, and they are why this cannot be a single `sed`:

- Enumerate first, then apply through an assertion-checked list that aborts when a line is not
  what was enumerated. 166's lane caught an occurrence wrapping across two lines this way, which a
  tree-wide replace would have silently mangled.
- A `BUILT` roadmap block is an account of what happened and keeps the old name where it narrates
  history; it moves only where it makes a present-tense claim about the system as it is now.
- A measurement table keeps the name it was measured under. `notes/frames.md` and `notes/net.md`
  count runs made under the old name, and sweeping them would make a true measurement cite a function
  that never produced it.
- **A quotation never moves**, whoever said it.
- Where an old name is kept deliberately, say so in a clause ("then `riscv_shell_boot`, renamed
  `boot_progenitor` at milestone 166") so a reader does not grep for a vanished name.

## BUGS

- **`spawn_init` may not have one successor.** It predates 166 and the lane did not establish what
  each occurrence should become; some may be describing a world that no longer maps onto one function
  at all. Whoever takes this must read each site rather than assume a one-to-one rename, and should
  expect at least one occurrence to need rewriting rather than renaming.
- **The count is provisional** for the reason stated above, and the sweep touches the same
  test-wiring hotspot (`kernel/src/user/tests.rs`, `xtask/src/main.rs`) where lanes collide, so it
  wants to run when nothing else is in those files.

## Index row

Two function names survive in prose and code comments after the functions themselves are gone:
`riscv_shell_boot` became `boot_progenitor` when milestone 166 unified the per-architecture boot
loaders, and `spawn_init` was already stale before that. Re-counted 2026-09-19 at 48 and 32
occurrences across 40-odd files, against the roughly 35 the filing lane saw mid-rename. A reader who
greps either one finds prose describing a function that exists nowhere, with nothing saying what it
became, which is the newcomer principle failing in the way this tree cares about most. The edits are
mechanical and the judgment is the whole job: a `BUILT` block narrating history keeps the old name, a
measurement table keeps the name it was measured under, a quotation never moves, and `spawn_init`
may not have one successor at all.
