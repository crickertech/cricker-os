---
status: BUILT
built: 2026-10-03
raised: 2026-10-03
promoted_from: the-spawn-service-holds-the-display-grants-and-the-shell-holds-none
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 715. The spawn service holds the display grants, and the shell holds none

Promoted from `design/roadmap/proposals/the-spawn-service-holds-the-display-grants-and-the-shell-holds-none.md` on 2026-10-03 (UTC). The number 715 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

<!-- writing-standards: exception. Granted 2026-10-03 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

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

## What was measured first (2026-10-03, UTC, aarch64)

A `caps` census was added first, so the shell could say what is in its table rather than what it
believes it was given. On the unfixed tree, `script/swish-check --arch aarch64`'s second boot (a
gpu, no keyboard) read `slots held: 0 1 2 3 4 5 20 21 22 23 24 25 30`: the gpu's four at 22 to 25,
as the block said. The same table printed "it can name no devices" two lines above, which was
false on that boot.

**The block's premise about `caps` was wrong.** It said `caps` listed seven `gpu` or `keyboard`
slots. It listed none: its rows are written from what the shell believes it holds, and nothing
wrote a row for slots 22 to 28. That is why the census exists.

## What is built

- The spawn service keeps the gpu's four and the keyboard's three in the boot endowment's own slots
  for the life of the boot (`GraphicalTerminalCaps::gpu`, `::kbd`), and lends each session's
  drivers narrowed copies. The builder no longer deletes them. Nothing is placed at
  `spawnproto::SHELL_GPU_SLOT` onward.
- The shell's request carries `GRAPHICS_BIT` alone; no capability follows it. `HOLDS_DISPLAY`,
  `HOLDS_KEYBOARD`, `display_wiring` and `delegate_display` are gone. A request that sets
  `KEYBOARD_BIT` is refused.
- The shell learns there is no display by asking: the spawn service answers
  `spawnproto::SPAWN_NO_DISPLAY` (provisional), and the shell prints the same sentence it printed
  before. The refusal line in `script/swish-check`'s first boot is unchanged and passes.
- `caps` with no tail ends with `slots held: ...`, read with `is_granted` below the fault slot
  (`swish::write_census`, provisional).
- The kernel's boot line now says the spawn service holds the grants.

## The test

`script/swish-check` types `caps` on the second boot (gpu, no keyboard) and, new, on the keyboard
boot before it launches. Both want `slots held: 0 1 2` and ` 21 30` at the end of the census,
meaning nothing between the configuration page (21) and the run-unvouched slot (30). After the
fix, aarch64 read `slots held: 0 1 2 3 4 5 20 21 30` on both boots. Row 32 of
`notes/confinement-claims.md`.

Falsification: `xtask/falsifications/swish_check.swish_check_leg.patch` places the
devices in the shell again. Replayed on aarch64: `script/swish-check --arch aarch64` exits 1 at
the second boot's `caps`, which read `slots held: 0 1 2 3 4 5 20 21 22 23 24 25 30`. The run stops
there, so the keyboard boot's census (slots 26 to 28) was not replayed red; the same defect places
those three.

## Parity: §19 (architectural parity is a tenet)

aarch64 and riscv64 boot a gpu in `swish-check`, and the census line is falsifiable there. x86_64's
runner attaches no virtio-gpu (milestone 632's gap), so its census line passes on a boot that never
had a display to hold. The code is shared; the gap is the runner's, recorded in milestone 632.
riscv64 and x86_64 were gated in CI, not booted here.

## What it cost

The seven grants now sit on the progenitor's login block, which is its capability table's peak.
`kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED` goes from 30 to 31: the keyboard boot reads 31 of
32 before its first prompt (it read 30, at the launch, before), the serial arm 28 (was 27), and a
boot with no gpu stays at 24. The configuration that reaches 31 is QEMU's: no board here has a
virtio keyboard. The block's "the kernel is untouched" is true of behaviour; that recorded
constant and the boot sentence changed.

## BUGS

- **Headroom is one slot on a gpu and keyboard boot.** `CAPABILITY_TABLE_PEAK_MEASURED`'s own doc
  says the next capability held across that peak buys a slot back or raises the table, and calls
  that a decision. This milestone spent seven and recorded the spend rather than deciding it.
  See Follow-on.
- **`KEYBOARD_BIT` (bit 46) is unused.** The shell never sets it and the spawn service refuses it.
  Its name and position were ratified (#1493), so retiring or reusing it is an architect's call.
- **`SHELL_GPU_SLOT` (22) names an empty block.** Kept, with its fence assertions, because a slot
  probed by nothing is still a slot nothing should allocate into; whether the name stays is an
  architect's call.
- **`SPAWN_NO_DISPLAY`'s name is provisional.** The block left the mechanism open ("reports at
  the prompt's start"); calef ruled it 2026-10-04 (UTC): the shell learns "no display" from a spawn
  result word, not a boot fact. The ruling covered the mechanism only, so the name still wants one.
- A child holding `result_ep` can send `SPAWN_NO_DISPLAY` about itself, as it can `SPAWN_FAILED`;
  `components/src/swish.rs`'s BUGS has that entry.

## Follow-on

- **Proposed.** `design/roadmap/proposals/trace-the-progenitors-login-block-peak.md`. Which
  capabilities sit on the peak is "not traced" in `kernel/src/cap.rs`, and knowing that is how a
  slot gets bought back rather than the table raised.
- **Recorded.** Milestone 709 (the no-keyboard arm holds only the raw half of the boot
  discipline) names "the seven device capabilities at slots 22 to 28 with `GRANT`" among what a
  hostile session gains through the shell. After this milestone the shell holds none of them, which
  narrows that finding; this sentence is its record, here where 709's reader of 715 meets it.

## Index row

Since milestone 632 the boot shell held the GPU's and keyboard's capabilities for its whole life. BUILT: the spawn service holds them and lends each session's drivers copies; a `caps` census proves the shell holds none, with a replayable falsification.
