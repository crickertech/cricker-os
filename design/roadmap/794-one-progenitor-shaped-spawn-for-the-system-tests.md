---
status: NOT-STARTED
promoted_from: one-progenitor-shaped-spawn-for-the-system-tests
raised: 2026-10-05
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 794. One progenitor-shaped spawn for the system tests

<!-- writing-standards: exception. Granted 2026-10-06 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

Raised by the build lane of §185 (what carries the claim that userspace composes a process from
an authority you can count on one hand) on 2026-10-05 (UTC), which wrote the fifth copy of this sequence
because the four it found were private to their test modules.

## The duplication

Five harnesses in `system_tests/src/user/` start a program the way the kernel starts the
progenitor: lay out its ELF in a hand-built space, map a stack, map the timebase page on x86_64 and
riscv64, map the whole archive read-only at `INITRD_VA`, insert a memory region in slot 0 and a
report line in slot 1, configure, and start with the archive's length in `x1`. They are
`authority_tests::spawn_tree`, `c_seam_tests::spawn_confiner`, `live_swap_tests::spawn_swapper`,
`timetable_tests::spawn_timetable` (a variant that copies a narrowed archive into fresh pages
rather than mapping the real one) and `process_composition_service::wire`. Each sizes the space with
the same arithmetic, including the `log_pages_for(initrd_pages)` term, which had to be added to every
copy when `map_physical` began recording.

`run` cannot serve them, because it takes the extra pages as a slice and the suite has no allocator
to build a slice of a few thousand mappings.

## The proposal

One `pub(super)` function in `system_tests/src/user.rs` (or beside it), taking the program's name,
its budget in pages, the region capability's rights and the report rendezvous, and returning once
the program is started. The five call sites shrink to one line each, and the next change to the
progenitor's endowment lands in one place. The kernel's own `spawn_hello` is a sixth copy, in the
kernel crate; folding it in is a separate question, because it also measures the program and routes
interrupts.

The cost is a touch of five files in the test-wiring hotspot, which is why the §185 lane did not do
it while the lane for milestone 95 (an unmap primitive, and the mappings init never lets go) was
open in the same files.

**Reuse:** the helper would be the reuse. `kernel::user::run` was considered and does not fit
(above); `spawn_hello` measures and routes interrupts, which no test harness wants. Searched
`system_tests/src/user/` and `kernel/src/user.rs` for `INITRD_VA`.

## What is blocked

Nothing. The copies agree today; the risk is the next edit that reaches four of them.

## Index row

Five system-test harnesses lay out a program the way the kernel starts the progenitor, each with its own copy of the stack, timebase, archive and log-page arithmetic. One shared spawn removes the copies and the drift between them.
