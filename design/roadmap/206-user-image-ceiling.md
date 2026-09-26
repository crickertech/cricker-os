---
status: BUILT
raised: 2026-08-31
built: 2026-09-26
---
# 206. A program image has under 896 KiB, and the failure names an overlap rather than a size

Minted 2026-08-31 from the lane for milestone 121 (`ripgrep`: enumeration as a
capability), which hit it the hard way. Built 2026-09-26 by the lane on
`milestone/206-address-space-map`, the day calef ruled DECISIONS §171 (where a program image starts,
and where the stack goes): option D, with option A inside it. *(Number provisional until the merge
queue lands it.)*

**What was built.** The user address-space map, as a crate: `crates/address_space_map` (name
provisional). Eight bands tile the first 2 GiB of every process. Every constant several programs
agree on derives from them or is checked against them at compile time. The loaders refuse an image
that does not fit, with a sentence that says why. [notes/address-space-map.md](../../notes/address-space-map.md)
is the explanation and the crate's own docs are the rule.

**The ceiling, before and after: 896 KiB, now 496 MiB.** `ripgrep`'s 2.61 MiB image fits with room
for 190 of it.

## The map

| band | start | end | size | what goes in it |
|---|---|---|---|---|
| `NULL_GUARD` | `0x0000_0000` | `0x0020_0000` | 2 MiB | nothing, ever |
| `PAIR_PAGES` | `0x0020_0000` | `0x1000_0000` | 254 MiB | pages one builder agrees with the one program it builds; a program's private windows |
| `RUNTIME_WINDOWS` | `0x1000_0000` | `0x4000_0000` | 768 MiB | windows a loader or runtime places: std's runtime pages, socket frames, scratch cursors, the initrd, the swap image |
| `HEAP` | `0x4000_0000` | `0x5000_0000` | 256 MiB | the heap only |
| `SERVICE_WINDOWS` | `0x5000_0000` | `0x6000_0000` | 256 MiB | large windows a service contract names: block channel, C seam, screen aperture |
| `IMAGE` | `0x6000_0000` | `0x7F00_0000` | 496 MiB | the program's code and data |
| `STACK` | `0x7F00_0000` | `0x7FFF_0000` | 16 MiB less 64 KiB | the stack, down from `0x7FFF_0000`; lowest page a guard |
| `PROCESS_PAGES` | `0x7FFF_0000` | `0x8000_0000` | 64 KiB | pages the kernel maps into every process; the current-CPU page is the last |

## What changed

- Option A. `kernel::user::load` checks the image against the band before mapping anything and
  refuses with `LoadError::Misplaced`, printed as *"the program image is too large: it ends at X,
  past the stack's base at Y; an image may be at most 496 MiB"*. An image linked for the old layout
  is refused as outside the band. Three kernel tests: too large, linked below the band, and a 2 MiB
  image (past the old ceiling) that loads. `supervision_protocol`'s loader makes the same check.
- The image and the stack moved together to the top of the second gigabyte. `USER_STACK_VA` and
  `CHILD_STACK_VA` are both the map's `STACK_TOP_PAGE`, so the stage-10 `authority_tests` break that
  §171 records, one moving without the other, cannot recur.
- `crates/user_mode_runtime/link.ld` links at `0x6000_0000`, and the map's host test parses that
  line and fails if it differs from `IMAGE_BASE`.
- **Milestone 121's relink at `0x100_0000` is gone** from `helpers/build-ripgrep.sh`, and the same
  relink in `cryptography_exerciser/build.rs` with it.
- The fixtures are under the map. 188 fixed windows across `fixtures/`, `components/`, the kernel
  harnesses, the progenitor, `redoxfs_server` and five crates are written as
  `pair_page(..)`, `runtime_window(..)` or `service_window(..)`, which stop the build if the address
  leaves its band. Hand-built children put their code at `IMAGE_BASE` and their stack at
  `STACK_TOP_PAGE`.
- The current-CPU page moved from `0x3FFF_F000` to `0x7FFF_F000`, into the stack's leaf table.
  Counted, the page tables per process are unchanged for a program with no pair pages, one more
  for a program with pair pages, one fewer for a std program.
- The screen aperture moved from `0x4000_0000` (the heap band's first page) to `0x5800_0000`.

- Two things the map caught on its first CI run. `mkfs` had never been linked with the shared
  script (lld's default put it at `0x20_0000`); it is now. And the progenitor's job regions were one
  table short once a child's image and pair pages sit in different gigabytes: 40 became 41, and the
  two-space directory region 96 became 98.
- `cargo xtask` refuses to pack an archive program that does not fit the image band, so a program
  no test loads cannot hide outside the map the way `mkfs` did.

## Band choices that were forks, and what was recommended

- A guard page under the stack: yes. It costs one page of address space and makes a stack at its
  limit fault before it reaches the top of a full image.
- The stack band's size: 16 MiB. Twice Linux's default 8 MiB main-thread stack; the deepest stack
  here today is std's 128 KiB. Unmapped address space costs nothing.
- The image band ends at 2 GiB because `x86_64` links with the small code model, not by choice.
- The stack above the image, not below. It is what makes the too-large message literally true.

## Follow-on

- **Proposed.** The timebase page moves into `PROCESS_PAGES`, beside the stack, where it would pay no
  page table instead of three: `design/roadmap/proposals/the-timebase-page-moves-beside-the-stack.md`.
- **Proposed.** The builder's scratch cursor reaches the progenitor's initrd window after about a
  hundred `ripgrep`-sized spawns: `design/roadmap/proposals/the-builders-scratch-cursor-is-bounded.md`.
- **Recorded.** Band checks see membership, not collisions within one program; two protocol crates
  restate their numbers: `crates/address_space_map/src/lib.rs`.
- **Recorded.** The userspace loader refuses a too-large image as `Err(())`, with no reason:
  `crates/supervision_protocol/src/lib.rs`.

## BUGS

- Membership, not collision. The band functions cannot see two pair pages one program maps
  landing on each other; the programs that map several keep their own disjointness asserts.
- `std_runtime_protocol` and `counter_frequency_protocol` restate their numbers, because both are
  generated into std's PAL where no crate can be named; each pins itself to the map with a host test.
- `supervision_protocol`'s loader cannot say why: every refusal there is `Err(())`, so a
  too-large image built from userspace still reads as "could not spawn".

## Index row

Built 2026-09-26 as DECISIONS §171 option D: `crates/address_space_map` (provisional) draws eight bands over the first 2 GiB, the image at `0x6000_0000` and the stack directly above it, so an image may be 496 MiB instead of 896 KiB. Both loaders refuse an image that does not fit, by size, before mapping anything; the linker base is checked against the map; 188 fixed windows are band-checked at compile time; ripgrep's relink is gone.
