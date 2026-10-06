---
status: NOT-STARTED
promoted_from: pin-the-hot-trap-paths-placement
raised: 2026-10-05
milestone_dependencies: none
decision_dependencies: none
machine_requirements: riscv64 silicon
specific_machine: radon (the effect was measured there, and the acceptance sweep is radon's)
needs_person: yes
---
# 796. Pin the hot trap path's placement, so a radon per-crossing number stops moving with unrelated code

Raised 2026-10-05 (UTC) by `lane/radon-2026-10-05-record`, recording that evening's radon session.
Title and slug are drafts. `needs_person` is yes only because the acceptance sweep is boots on
radon.

## What was seen

[`notes/job-mix/radon-2026-10-05.md`](../../notes/job-mix/radon-2026-10-05.md#why-one-task-rose-from-99-to-110):
`main` built four times with milestone 370 (a layout control)'s knob (`fastpath_pad`, `NIFE_FASTPATH_PAD=0`,
`NIFE_FASTPATH_SHIFT` 0, 16, 32, 48), which moves the whole kernel text by N bytes and changes no
code. The job mix's `null_syscall` at one task read 111, 118, 116 and 110 ticks of radon's 4 MHz
timer. An 8-tick (7%) spread with no code change. A bisect had already put a 16-tick one-task
step on a merge (#1659) that added no instruction to the kernel's trap path.

`trap_entry`'s own offset is not the variable: offset 40 mod 64 was the best layout in the bisect
(99, 100) and the worst in the sweep (118). The U74's L1 instruction cache is 32 KiB, two-way,
256 sets indexed by virtual address bits 6 to 13 (`script/fastpath-footprint`'s `--layout`
comment, citing the core manual). Three hot pieces of code landing on one set (the kernel's entry,
its dispatch and lookup, and the userspace stub, which shares the index bits) would evict each other
on every trap. That is the leading reading, and it is unmeasured.

## Why it matters

Every single-crossing number on radon is a comparison between builds, and until this is fixed any
difference under about 8 ticks is noise from where the linker happened to put things. Fatal risk 4
is argued in those numbers. On 2026-10-05 the cost was concrete: a one-task guard written before the
run fired on placement, and an evening of boots went to proving that.

## What the tree already does

- Milestone 370 pins one section, `.text.fastpath_pad_body`, first in `.text` after the boot stub
  (`kernel/link-riscv64.ld`), so its knob moves everything after it uniformly. Nothing else in
  `.text` is placed: `*(.text .text.*)` takes input order.
- `script/fastpath-footprint --layout` already names the hot set (both fast-path closures plus the
  entry set), prints each symbol's address and L1i set on riscv64, and hashes its instructions.
  That is the list to pin and the instrument to check the pin with.

**Reuse:** milestone 370's pinned section and linker-script pattern, and `script/fastpath-footprint
--layout`'s hot set and set indices, both taken. No outside tool considered; the work is a linker
script and a list.

## Options, not yet decided

1. One output section for the hot set, placed and aligned. Put the `--layout` hot symbols in
   `.text.hot` (by `#[link_section]` on the Rust functions and a section directive on the entry
   assembly), first after the boot stub, aligned to the L1i way size (16 KiB), in a fixed order. The
   kernel's half is then the same addresses in every build. Cost: a list of symbols someone must
   keep in step with the code, which `--layout`'s closure can check; and inlining decisions can
   still pull hot code out of the section.
2. Option 1 plus the userspace stub at a set range the kernel's hot section does not use. The
   stub's placement comes from `crates/user_mode_runtime/link.ld`. Needed only if option 1 alone
   leaves a spread.
3. Measure only. Control layout per experiment with milestone 370's knob, sweeping every time.
   Costs four boots per comparison forever; it is the status quo plus discipline.

This lane recommends 1, then 2 only if the sweep says so. It has not measured which pieces collide.

## Done means

- The hot set's addresses and L1i sets identical across `main` and a build with unrelated code
  added ahead of it (a `script/fastpath-footprint --layout` diff), in a check that fails the build
  when they move.
- On radon, the 2026-10-05 sweep repeated (shifts 0, 16, 32, 48) reads `null_syscall` at one task
  flat within 2 ticks, where it read 110 to 118.
- `script/fastpath-footprint` and `script/bench` within their floors on all three ISAs; the same
  pin applied, or a scope note recording why not, on aarch64 and x86_64.

## Index row

A merge that added no instruction to the trap path moved radon's one-task `null_syscall` by 16 ticks, and four builds that only shifted kernel text spread by 7%. Pinning the hot trap path's placement stops a per-crossing number from moving with unrelated code.
