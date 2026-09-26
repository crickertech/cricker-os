---
status: NOT-STARTED
raised: 2026-08-31
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 206. A program image has under 896 KiB, and the failure names an overlap rather than a size

Minted 2026-08-31 from milestone 121's (`ripgrep`: enumeration as a
capability) lane, which hit it the hard way. *(Number provisional until the merge queue lands it.)*

§171 (where a program image starts, and where the stack goes) was ruled by calef
on 2026-09-26: option D, with option A shipping alongside. This block is ready to build. §171 was
written up 2026-09-19 by the lane of milestone 435 (forty-five milestones are gated on a decision
nobody wrote down). That lane re-measured the spread. The symbol appears in 23 files, the
protocol-side reference is `counter_frequency_protocol` rather than the two crates named below, and
`user/link.ld` is now `crates/user_mode_runtime/link.ld`.

**In brief.** `user/link.ld` links a program at `0x40_0000`, `USER_STACK_VA` is `0x50_0000`, and 32
std stack pages sit below it. So a program image has under 896 KiB. `ripgrep`'s `.text` alone is
1.37 MiB.

The failure is `Unmappable(AlreadyMapped)`, which names an overlap and not a size, so nobody
hitting it would learn what was wrong. Milestone 121's lane worked around it by relinking at
`0x100_0000`, derived from `user/link.ld` by substitution so the two cannot drift.

## What the ruling asks for

- Write the user address-space map down as one crate, with bands for image, stack, heap, protocol
  pages and fixtures, each carrying the reason it sits where it does. The crate's name is
  provisional until an architect names it.
- Derive `USER_STACK_VA`, the linker script's base and every protocol page's address from it.
  Nothing picks an address on its own any more.
- Bring the fixtures under the same rule: `fixtures/src/window.rs`'s `CTL_VA` and every other
  self-chosen fixture address move into the fixture band.
- Make `AlreadyMapped` on a program image say that the image overlaps the stack, naming both
  addresses (option A).
- The band boundaries are values several programs agree on, so the pull request that first sets
  them carries `needs-architect`.

Done means `ripgrep` loads at the map's image base on all three architectures, without the relink
that milestone 121 (`ripgrep` on nife: enumeration as a capability) needed.

## Why it is not a one-line change

`USER_STACK_VA` appears in `supervision_protocol`, `counter_frequency_protocol`, `c_seam`, `components/src/builder.rs` and
several tests. Measured by that lane: moving it alone breaks `authority_tests` at stage 10. It is
a layout two programs agree on rather than a private kernel detail.

## What this is really about

896 KiB is small for anything with a dependency tree, and the corpus milestone 123 (the
demonstration: somebody else's software, running narrow) wants is made of such things. This ceiling
is the second thing a foreign program meets, right after milestone 205 (how a foreign program is told
what to do).

## BUGS

- The error message is the worst part and is the cheapest to fix. `AlreadyMapped` could say that
  the image overlaps the stack and name both addresses, independently of any layout decision. §171
  puts it in this milestone's scope.
- This block proposes no band boundaries. Drawing them is the work, and it interacts with the std
  heap's base at `0x4000_0000` and the shared pages above it.

## Index row

Minted from milestone 121's lane, which hit it the hard way. `user/link.ld` links at `0x40_0000`, `USER_STACK_VA` is `0x50_0000`, and 32 std stack pages sit below it; `ripgrep`'s `.text` alone is
1.37 MiB. The error is `Unmappable(AlreadyMapped)`, which names an overlap and not a size. `USER_STACK_VA` is a protocol constant in two `_proto` crates and moving it alone breaks `authority_tests` at stage 10, measured.
