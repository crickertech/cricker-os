---
status: NOT-STARTED
raised: 2026-10-04
milestone_dependencies: none
decision_dependencies: 247
machine_requirements: none
specific_machine: none
needs_person: no
---
# 755. basalt v0 pins nife and runs its gate

*(Number provisional until the merge queue lands it. Title and slug are drafts.)* Minted 2026-10-04
(UTC) by the lane `lane/split-order-proposal` from calef's ruling of the same day, recorded in §247
(the split begins with basalt holding nife, and `procps` moves first). basalt v0 is the first step
of the repository split. The ruling includes the go-ahead for a lane's first write to
`nifeos/basalt`. The reasoning and the measurements are in
[notes/the-first-split.md](../../notes/the-first-split.md), section 3.

## What it builds

The smallest thing that is still a distribution, in the empty repository milestone 120 (the
rename: the OS becomes `nife`) reserved:

- One manifest file naming one component: `nife`, pinned by repository and commit. Its file name
  and format are provisional, and calef names them.
- A workflow that checks out the pinned commit and runs that commit's own image build and system
  test on aarch64, riscv64 and x86_64, keeping the images as build artifacts. basalt has no
  toolchain of its own; it builds at the pin the component's commit carries.
- A scheduled job that bumps the pin, the way `toolchain-bump.yml` bumps the nightly, so a pin bump
  costs nobody a pull request by hand.

Nothing moves out of this repository, and nothing here changes: nife does not know basalt exists.

## Exit criteria

- basalt's workflow goes green on a pin to a current `main` commit, on all three architectures.
- A pin to a commit known to fail its system test goes red, so the gate is shown to be able to fail.
- basalt's files cite no milestone and no section, so §201 (one roadmap until a citation has to
  cross)'s revisit is not triggered by it.

## BUGS

- The system gate runs a second time per pin bump. That is the price of the gate living in basalt
  before a split needs it there.
- The repository needs its own ruleset and merge queue, and whether the `nife-smelter[bot]`
  watchers run there is open; v0 can start with the pin-bump job and no queue.

## Index row

The first step of the repository split, ruled by calef 2026-10-04. basalt, the distribution
repository, pins nife at a commit and runs its image build and system test. The whole-system gate
then exists outside the monorepo before any code leaves it.
