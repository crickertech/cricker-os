---
status: PROPOSED
raised: 2026-10-03
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# The spawn service holds the display grants, and the shell holds none

Raised by the 2026-10-03 security audit's follow-up (item (a) of
`design/audit-reports/2026-10-03-eight-constants-and-thirteen-components.md`'s reconciliation).

## What the shell holds, for its whole life

Since milestone 632 (graphics on demand: `graphical_terminal`, launched from the swish prompt) the
progenitor places the GPU's four capabilities and the keyboard's three in the boot shell at
`spawnproto::SHELL_GPU_SLOT` onward (`crates/system_initializer/src/lib.rs`, the `slots` table
in the login block): the two transports with `WRITE | GRANT`, the two interrupts with
`READ | GRANT`, and the DMA run, the surface and the keyboard DMA page with
`READ | WRITE | GRANT`. The shell delegates narrowed copies when it launches a session and keeps
its own, "so the session can be run again once it ends" (`components/src/swish.rs`,
`delegate_display`).

What those copies let the shell do, read from the kernel's rights checks: map the DMA run, the
surface and the keyboard DMA page read-write into its own address space (`map_page_frame` with
the `tables` it already holds; `MAP_RW` needs `WRITE`, which it has), so it could read every
keystroke the keyboard driver's DMA lands and write the surface behind the session; and `RECV`
on either interrupt rendezvous (`READ`), where `irq_notify` wakes one waiter, so a shell parked
there would take a wake the driver was waiting for. It does neither. Its only use of the seven is
`delegate`.

## Why this is wider than it needs to be, and what the tree already does one slot over

The spawn service keeps `term_ep`, the boot discipline's endpoint, with `WRITE | GRANT` for
exactly the same purpose, the next session, and the comment at `cap_delete(term_out)` says why
the shell's copy of *that* carries no `GRANT`: "nothing at the prompt can hand the terminal to
any program it likes". The display grants take the opposite posture in the same launch: the shell
holds `GRANT` on all seven, so the prompt can hand the GPU to any program that asks for the right
spawn wiring. The spawn service refuses any combination but `graphical_terminal` with all seven
today, which is what keeps this a width rather than a hole.

## The change

The spawn service holds the seven with the rights the boot endowment carried, and builds the
session's drivers from its own copies, as it already does with `term_ep`. The shell's launch
request stops carrying capabilities and carries the one bit it carries today, "this is a
graphical terminal launch". The shell holds no device capability at any point. `HOLDS_DISPLAY`
becomes a fact the spawn service reports at the prompt's start rather than one the shell infers
from its own slots.

Considered and refused: the shell deletes its copies after the first launch and asks the
progenitor for them again. That is the same authority in the same place with a round trip added.
A second shell at a second session: not a case today, one session at a time is milestone 632's
own rule.

## What it costs

The spawn service's launch arm reads from its own table instead of the request's delegations,
and the shell's `delegate_display` goes away. One spawn-protocol bit is unchanged. The kernel is
untouched. A test: `caps` at the boot prompt lists no `gpu` or `keyboard` slots, where today it
lists seven.

## What is blocked until it lands

Nothing. The width is recorded in `components/src/swish.rs`'s BUGS where a reader of the shell's
holdings meets it.
