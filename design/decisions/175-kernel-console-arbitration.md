---
status: DECIDED
raised: 2026-09-19
decided: 2026-09-27
ratified_by: calef
---

# 175. Where the kernel's own output goes once userspace owns the console

Raised 2026-09-19 by milestone 435 (forty-five milestones are gated on a decision nobody wrote down)'s slice B, which read milestone 342 (the kernel and the `console` server drive one UART from two address spaces)'s
`DECISION` gate and found it naming no section. Milestone 230 named the fork while fixing something
else, and the 2026-09-03 proposal sweep carried it forward. *(Section number provisional until the
merge queue lands it.)*

## The ruling

calef ruled 2026-09-27 (UTC): B, with a panic escape and a fallback, the same shape as §242 (a
system log, provisional; pull request #1423). The kernel appends ordinary output, fault reports and
the two gauges below, to a ring. §242's log service drains the ring, stores the records and forwards
whole lines to the console. A panic writes the UART directly, breaking the console lock the way
`console::force_unlock` already does for the single-process case. When the ring is not being
drained, the kernel falls back to writing the UART directly rather than filling silently and going
quiet.

Options and why each lost, from `notes/kernel-console-arbitration-pricing.md`:

- **A (a second port).** Refused. It is free under QEMU on all three architectures, but the
  hardware fact the original proposal below asked for came back "no". Xenon has one serial connector,
  marked optional, in Dell's own layout. Radon does not expose a second port without rewiring the
  shared header. On argon it is unconfirmed: the only usable second UART is one forum post's claim,
  not the vendor's own spec. A decision that "binds every architecture" cannot rest on a port two of three
  real boards do not have.
- **E (release builds go quiet, seL4's shape; priced as "R").** Refused. `swish-check --release` is
  explicitly "Not in CI", so a release-only change would not touch the flake this milestone exists to
  fix. Read faithfully, seL4's own answer is that a panic prints nothing at all once printing is
  compiled out, which reverses what `console::force_unlock`'s own comment already decided for this
  kernel: get the message out, spliced or not.
- **C (a claim, respected except in a panic)** and **D (leave it)** are superseded, not refused: the
  pricing note (part 4) shows B-with-escape is C wearing a buffer. Outside a panic the buffer removes
  the byte-granularity splice C could only avoid by waiting; in a panic it behaves exactly as C's
  claim would; and a dead drainer degrades to a counted fallback rather than either D's constant
  splicing or a silent void.

## What is being decided

Once the `console` server owns the console, two address spaces drive one UART with nothing
arbitrating. The kernel writes directly, because a kernel that cannot print during a fault is a
kernel nobody can debug. The server writes on behalf of userspace. The streams interleave at byte
granularity.

The decision is where kernel output goes: a second port, a buffer the server drains, a claim the
server takes and the kernel respects except in a panic, or something else.

## Why it is not a bug to fix

It corrupts every bench session on argon, radon and xenon, where a serial log is the only thing
those machines can say, and milestone 216 built a tool whose contract is recognising a boot sequence
in that stream. Interleaved bytes break that contract in the least visible way available: the log is
present, it looks like output, and the line being matched has a kernel message spliced through the
middle of it.

Nothing in `design/decisions/` answers it, checked 2026-09-19.
[§149](149-kernel-served-console-endpoint.md) is the nearest and is a different question: it asked
whether the kernel may *answer on an endpoint* where §121 left x86 without a userspace holder, and
it was resolved on 2026-09-15 by dissolving that premise, so x86's console is a userspace driver like
the other two. That makes the interleaving question more live rather than less, because all three
architectures now reach the shape that produces it.

## What the kernel actually writes after the handoff, measured

Milestone 342's block says this is *"a measurable list rather than an opinion"*. The tree has already
measured it, and the answer is in `xtask/src/swish_check.rs`:

> Text the **kernel** prints only in a user-fault report, which is the only thing it writes after
> the userspace console has started.
>
> -- `KERNEL_FAULT_TOKENS`'s doc comment

Six tokens (`user thread `, ` killed: `, `the kernel is fine`, `stval 0x`, `esr 0x`, ` sp 0x`), and
the neighbouring `SWISH_CHECK_MARKER_SLACK` prices the intrusion in the same file: *"one kernel
fault report, three lines and about 150 characters"*, with 400 bytes of slack allowed *"with room to
spare"*.

That narrows the problem sharply and it should be the first thing calef is told. This is not a
kernel that chatters over userspace. In normal operation it writes nothing; the entire collision
surface is one three-line fault report per faulting user thread, plus whatever a panic produces.
Options that would be absurd for a chatty writer are reasonable for this one.

## The options

| | shape | what it costs, and where the cost is unmeasured |
|---|---|---|
| **A** | **A second port.** The kernel keeps a UART of its own; the server owns the other. | Zero coupling, nothing to arbitrate, and a panic path that cannot be starved. It is a hardware fact per board whether a second usable port exists, and this tree has not established it: `notes/uart.md` records two UARTs on the Pi and nothing equivalent for argon, radon or xenon. A bench session answers it; nothing here does. |
| **B** | **A buffer the server drains.** The kernel appends; the server interleaves at line granularity. | Serves the ordinary fault report well and serves the case that matters worst. A panic is exactly when the draining server may be the thing that died, so a buffered panic is a panic nobody reads. Any B has to carry a direct-write escape, at which point it is C with extra machinery. |
| **C** | **A claim the server takes, which the kernel respects except in a panic.** | One flag and one exception, and it matches what the tree already measured: the kernel writes nothing until a fault, so "respect the claim" costs nothing in the common case. The exception is where every argument will be, since a fault report is not a panic and the two want different answers. |
| **D** | **Leave it, and record the limitation where the reader meets it.** | Free today and it is what the tree does. The cost is already being paid by every bench log and by `script/swish-check`'s own `BUGS`, which describes the interleaving as a live defect in the system rather than in the script. |

Ruled. See "The ruling" above: B, with a panic escape and a fallback. The measurement this table
called for, whether each board has a second usable port, came back "no" on all three
(`notes/kernel-console-arbitration-pricing.md`), which is what let calef decide between A and the
rest.

## What settled it

Both items below were answered by `notes/kernel-console-arbitration-pricing.md`, priced 2026-09-27,
and that pricing is what "The ruling" above draws on.

1. Whether argon, radon and xenon each have a second usable serial port, read off the boards.
   No, on all three: that is what removed A.
2. What a panic costs under B and C, reasoned from the fault path. The same, once B carries the
   panic escape, which is why B-with-escape and C were not a real fork by the time this was ruled.

## What was blocked, and what still is

Milestone 342 was blocked on this section; it is decided now. The milestone's own build is not
unblocked by that alone: the ruling routes the kernel's ring through §242 (a system log, provisional;
pull request #1423), which is still PROPOSED and unbuilt, so milestone 342 still waits, now on §242
landing rather than on this decision. Milestone 243 (a machine with no serial port has no way to
say anything) pointed in its `BUGS` at a home for this question, which is why this section exists;
that citation now resolves.
