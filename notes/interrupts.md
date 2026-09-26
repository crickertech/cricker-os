# Interrupts: the GIC and the timer

Milestone 5. The kernel is now **preemptible**: a timer interrupt can land between any two
instructions.

Which means every piece of the locking discipline we wrote in
[DECISIONS](../design/decisions/09-irq-safe-locking.md) §9 stops being a hypothesis.

## The GIC: the multiplexer in front of the CPU

The CPU has **one** IRQ input line. That's all. Everything a kernel wants from interrupts,
priorities, masking individual sources, routing to a particular core, lives in the interrupt
controller, not in the CPU.

Two halves, and the split *is* the design:

| | Where | Shared? | Does what |
|---|---|---|---|
| **Distributor** (GICD) | `0x0800_0000` | **one per machine** | which core gets an interrupt, and whether a source is enabled at all |
| **CPU interface** (GICC) | `0x0801_0000` | **one per core** (banked) | this core's own view: acknowledge, priority mask, end-of-interrupt |

N cores see their *own* CPU interface at the *same address*: the hardware banks the registers
per core. That's what makes "deliver this to core 3" something the hardware can do without the
software knowing.

Both addresses come from the [device tree](device-tree.md) (`intc@8000000`), not from a
constant.

## Three kinds of interrupt, and the numbering isn't arbitrary

| INTID | Kind | |
|---|---|---|
| 0–15 | **SGI**: Software Generated | one core kicking another. This is how SMP bringup and TLB shootdown work. |
| 16–31 | **PPI**: Private Peripheral | **per-core**. The timer is one. |
| 32+ | **SPI**: Shared Peripheral | the UART, the disk. Any core may service them. |

**The timer is a PPI (INTID 30), and it has to be.** A timer that fired on only one core could
not preempt threads running on the others. Every core has its own timer, its own countdown, and
its own interrupt, all wearing the same number.

The device tree says so: `interrupts = <1 14 ...>` on the timer node. Type 1 means PPI, 14 is
the PPI number, PPIs start at 16, so `16 + 14 = 30`.

## Priorities are backwards

**Lower value = higher priority.** And `GICC_PMR` is a *mask*: an interrupt is delivered only if
its priority is **strictly less than** PMR.

So `PMR = 0xff` means "let everything through" and `PMR = 0` means "let nothing through."

Get that comparison the wrong way round and you get a machine that takes no interrupts and
gives you no clue why. It's also why `gic::init` sets PMR **before** enabling the CPU interface:
the other order leaves a window where the interface is live with whatever the firmware left in
PMR, which on a cold boot is often zero.

## Acknowledge, then end-of-interrupt

```
IAR  (read)   -> "which interrupt?"   ...and READING IT IS WHAT TAKES IT.
EOIR (write)  -> "I'm done with it"
```

`IAR` has a **side effect**. Reading it acknowledges. Exactly once per interrupt.

And until `EOIR` is written, the GIC will not deliver another interrupt of equal or lower
priority. **Forget it and the timer fires exactly once and then never again**, which looks
nothing like "you forgot to write a register."

**INTID 1023 is spurious**: the GIC raised the line and then changed its mind (another core took
it, or it got masked). Do nothing, and in particular do **not** write EOIR: signalling
completion for an interrupt you never took corrupts the GIC's priority stack.

## IRQs dispatch by vector slot, not by ESR

`exception_dispatch` gets both the trap frame and *which of the sixteen vector slots fired*
([exceptions.md](exceptions.md)). For a fault we decode `ESR_EL1`. For an IRQ we must not.

**`ESR_EL1` describes a synchronous exception**: what instruction did what wrong. An IRQ is
*asynchronous*. It has nothing to do with the instruction it interrupted, and `ESR_EL1` still
holds whatever the last *synchronous* exception left there. Reading it in an IRQ handler is
reading a stale answer to a question nobody asked.

## The bug we shipped and then measured

The timer is **one-shot**. It fires, and then sits there with its status bit set, holding the
interrupt line high until the handler sets a new deadline.

There are two registers to do that with, and the difference is not cosmetic:

| | |
|---|---|
| `CNTP_TVAL_EL0` | a **relative countdown**. "Fire N ticks from *now*." |
| `CNTP_CVAL_EL0` | an **absolute deadline**. "Fire when the counter reaches exactly this." |

Re-arming with `TVAL = interval` in the handler makes the real period

```
    interval  +  however long it took to get into the handler
```

Every tick starts its countdown *late*, and **the lateness is never recovered**. The clock just
runs slow, forever, and nothing tells you.

Measured, in QEMU, at a configured 100 Hz:

```
  +250ms: 17 ticks fired   <- should be 25.  ~70 Hz.  30% of our preemptions, gone.
```

`CVAL` puts the deadlines on a **fixed grid**: `next = previous + interval`, anchored at boot. A
slow handler makes *one* tick late; it does not push the next one out too.

```
  +250ms: 25 ticks fired   <- correct
```

One register.

### The safety valve

If we fall so far behind that the next deadline is *already in the past*, `previous + interval`
would fire immediately, and again, and we'd spin in the handler forever paying down a debt we
cannot pay.

So: give up on the missed ticks and re-anchor the grid to now. Every kernel does this and every
kernel calls it the same thing (**dropping ticks**), and it is worth counting, because a
nonzero count means the handler is taking longer than a whole tick period.

## Uptime comes from the counter, not the tick count

`uptime_ms()` reads `CNTPCT_EL0` and divides. **Deliberately not `ticks * 10`.**

If a tick is ever missed (a long critical section, a slow handler), the tick count undercounts
and *time appears to slow down*. The hardware counter cannot lie.

**This is `Instant`.** It is the thing `core` could never give us, and the reason is exact:
nothing in `core` knows what time it is.

## The test the whole locking discipline was written for

Everything in [locking.md](locking.md) exists to prevent one thing: a timer interrupt landing
inside a critical section, taking the same lock, and spinning forever waiting for code that
cannot run until it returns. On one core. Permanently.

Until this milestone that was a **hypothesis**. There were no interrupts.

`holding_a_lock_masks_the_timer`:

1. confirm ticks are flowing
2. take a lock, and busy-wait across **three whole tick periods**
3. assert **not one tick landed**
4. release, and watch them resume

Step 2 works because `spin_for` reads `CNTPCT_EL0`, which **keeps counting while interrupts are
masked**. A tick-based delay would simply hang there, which is its own kind of proof.

## And the cost of masking, made visible

`a_long_critical_section_costs_a_tick` asserts that holding a lock across two tick periods
**loses a tick**. The deadline passes while we cannot service it, we re-arm to a deadline
already in the past, and the only sane move is to drop it.

That is the bill for the deadlock prevention, and it is why "**keep critical sections short**"
(DECISIONS §9) has teeth rather than being good manners. At milestone 6, a lost tick is a thread
that didn't get preempted.

It is a strange thing to *assert*, until you notice: if that cost ever stopped being real,
`IrqSafeMutex` would have stopped masking, and the deadlock would be back.

---

*Add to this file as new interrupt sources come up.*

---

# Milestone 9a: an interrupt becomes a message

DECISIONS §10 promised this and notes/capabilities.md sketched it. Here it is.

## The problem a userspace driver has with interrupts

A driver at EL0 (milestone 8 put one there) cannot install an interrupt handler: handlers run at
EL1, in the kernel's vector table, at a privilege the driver does not have. And the kernel cannot
handle a device interrupt itself, because **it does not know what the device is**: that was the
whole point of moving the driver out.

So the interrupt has to reach the driver as something the driver *can* receive. It becomes a
message.

## The shape

```
  device raises INTID  ──►  kernel handle_irq:  mask INTID at the GIC
                                                turn it into a notification
                                                EOI
                                                        │
                       driver blocked in WAIT  ◄────────┘  wakes
                                                │
                       reads the device, quiets its interrupt
                                                │
                       invoke(irq_cap, ACK)  ──►  kernel: re-enable INTID at the GIC
```

The kernel's half does nothing device-specific. It masks the line, delivers a message, and later
re-enables the line when the driver says the device is quiet. Everything that knows what a *virtio
block device* is lives in userspace.

## Why the interrupt gets masked the instant it fires

Device interrupts are usually **level-triggered**: the device holds its interrupt line asserted
until the driver does something to quiet it (for virtio, reads `InterruptStatus` and writes
`InterruptACK`). If the kernel left the line enabled and just EOI'd, the GIC would see the line
still asserted and **re-deliver immediately, forever**: an interrupt storm the machine never
climbs out of, because the only code that can quiet the device is the driver, which never gets to
run.

So `handle_irq` masks the INTID at the distributor (`gic::disable`) the moment it fires. The driver
services the device, then calls `ACK` on its `Irq` capability, and only then does the kernel
re-enable the line (`gic::enable`). Until then the interrupt is held off. **This is exactly seL4's
IRQHandler protocol**, and it is what lets a process that holds no privilege safely own an
interrupt.

## An interrupt is not a rendezvous

IPC on an endpoint (milestone 7e) is synchronous: a sender waits for a receiver. An interrupt
cannot wait. It fires whether or not the driver happens to be blocked in `WAIT` at that instant,
and it must not be lost if the driver is a hair late.

So the notification is **asynchronous**, and the mechanism is one counter: `Endpoint::pending`. If
a thread is waiting, the interrupt wakes it. If not, `pending` is incremented, and the next `WAIT`
drains it instead of blocking. An interrupt that fires one instruction before the driver calls
`WAIT` is remembered, not dropped. There is a test named exactly that.

## The capability

`Object::Irq(intid)`. Its holder can:

- `WAIT`: block until the interrupt fires (internally, `RECV` on the endpoint the kernel routed
  the interrupt to).
- `ACK`: re-enable the interrupt at the GIC, after quieting the device.

A driver that holds this capability can receive one specific interrupt and nothing else. It cannot
mask other interrupts, cannot touch the GIC directly, cannot see any other device's line. The
authority is exactly one INTID, handed over deliberately.

## Testing it with no device

The whole path is exercised by a **software-generated interrupt** (SGI): `gic::send_sgi` raises
INTID 1 from software, with no hardware behind it. A thread blocks in `WAIT`, the test raises the
SGI, the handler routes it, the thread wakes. Deterministic, and it needs no disk. The virtio
driver (9b) will use the same path with a real device interrupt in place of the SGI.

## Testing it on RISC-V, which has no SGI (milestone 19, 2026-07-31)

The two tests above (`kernel::sched::tests::an_interrupt_becomes_a_message` and
`an_interrupt_that_arrives_before_the_wait_is_not_lost`) were aarch64-only for a year, gated because
they trigger with an SGI. **The properties are not architectural**, though: one is IRQ-to-IPC
delivery and the other is a lost-wakeup race, and RISC-V has interrupts and the same IPC. Only the
trigger was in the way. They are portable now, with the trigger behind three functions in the test
module (`arm_test_irq`, `raise_test_irq`, `quiet_test_irq`).

**What RISC-V raises, and why that one.** The console UART's own transmit-empty interrupt. A 16550
asserts its line the moment `IER.ETBEI` is set while `LSR.THRE` is set, and the transmitter of a
polling console is always empty, so one register write raises the line into the PLIC and one lowers
it. No transfer, no external stimulus, nothing to read back. `console::raise_uart_interrupt` /
`quiet_uart_interrupt`, test builds only.

**Two other options were considered and are worse, for reasons worth keeping:**

- **The SBI's IPI** (`sbi_send_ipi`, which `arch::irq::send_reschedule` already uses). It is the
  obvious "software-generated interrupt" on RISC-V and it is the wrong one. It arrives as a
  *supervisor software* interrupt, `scause` = 1, which is a **different arm** of
  `riscv_trap_dispatch` from a device's `scause` = 9: that arm drains the scheduler inbox and serves
  steal requests, and touches neither `irq_route` nor `irq_notify`. A test built on it would have
  looked like parity with aarch64 and proved nothing about IRQ-to-message delivery.
- **Writing the PLIC's pending bits** (base + 0x1000). Read-only by specification, and QEMU 11.0.2
  agrees: a probe that set source 20's pending bit and read the word back got `0x0` before and
  `0x0` after. Even if it had worked it would have been a QEMU behaviour to lean a gate on, three
  weeks before the VisionFive 2 arrives.

**So the two legs are not twins, and the note says so rather than the doc comment claiming it.**
aarch64's SGI needs no device at all; RISC-V's needs the UART to exist. In the other direction
RISC-V covers *more* of the controller: an external interrupt goes through the PLIC's
claim / mask / notify / complete handshake, which an aarch64 SGI does not reach. The kernel path
under test, the part these tests exist for, is the same on both.

**A claim that was in the tree and was not backed.** The old doc comment on
`an_interrupt_becomes_a_message` said the same path was "proven on RISC-V by the boot tour's
userspace UART driver". The boot tour is `script/console`, interactive, and gates nothing, so as
written the claim cited a witness the suite does not run. The substance was true by then for a
different reason (the parity-C virtio tests, below), but the citation was to the wrong thing, and a
parity claim resting on a demo nobody runs is the shape of gap §19 exists to catch.

---

# IRQ affinity: spreading device lines off core 0 (fix/irq-delivery, 2026-07-29)

Until now every SPI targeted core 0: `gic::enable` wrote `ITARGETSR[intid] = 1` (bit 0). Under SMP
that funnels every device interrupt onto one core, and because the handler that turns an interrupt
into a message runs on the core that took it (`handle_irq` calls `irq_notify`, which wakes the
driver onto `cpu::current`), the driver wake lands on core 0 too. Every disk and NIC completion, and
the driver work it triggers, re-concentrates on core 0 no matter where the threads were spawned.

The fix distributes SPI lines across the online cores. The **policy** lives in `arch::irq::enable`
(it may read `smp::online_count`; it is arch glue, not a driver): each SPI is assigned a target core
the first time it is enabled, round-robin over the online cores, and that assignment is **stable**
(`IRQ_TARGET`, an atomic per-INTID slot). Stability matters because the `Irq` capability's ACK
re-enables the line on every completion; re-rolling the target each time would make the line hop
cores on every interrupt. The **mechanism** stays in the driver: `gic::enable(intid, target_cpu)`
writes `ITARGETSR[intid] = 1 << target_cpu`. PPIs and SGIs are per-core, so the target is ignored
for them (the timer PPI, the reschedule SGI). Rule #2 holds: the GIC driver is told which core, it
does not decide.

What this does and does not buy, measured honestly:

- It **does** move each device's interrupt (and the wake it causes) off core 0 onto its assigned
  core. Verified: the full aarch64 suite (disk, PCIe disk, both DHCP round trips) stays green with
  the lines spread, and a diskless boot's device IRQs land on cores other than 0.
- It **does not**, on its own, make the heavy `std_net` pipeline (smoltcp in `net_stack`, plus the std
  program) go faster under SMP, because that pipeline is a chain of IPC rendezvous, and a rendezvous
  wake still lands on the *waker's* core (`cpu::current`). Spreading the interrupt moves the whole
  chain to the interrupt's core; it does not parallelize it. Parallelizing the pipeline needs the
  rendezvous/device-IRQ **wake placement** to be load-aware, which is the scheduler's call
  (DECISIONS §28 territory), not the interrupt controller's. See the `std_net` note below.

## The riscv PLIC side, done the same way (parity §19)

The PLIC equivalent spreads device sources across harts, the same round-robin-with-a-stable-target
shape as the GIC. The PLIC delivers a source to a **context** (a hart at a privilege level). The
hart-to-context numbering is the board's, read out of the device tree's `interrupts-extended` at
boot (`arch::irq::init_contexts`, backed by `isa::plic`): `2*hart+1` for S-mode on QEMU `virt`,
`2*hart` on the JH7110, whose disabled S7 monitor core contributes only an M context (see
notes/visionfive2.md). The pieces are:

- **Every hart sets `SEIE` early** (`arch::irq::init_this_cpu` now unmasks supervisor external
  interrupts alongside the software-interrupt IPI source). This is safe before the PLIC base is even
  known: `SEIE` with no source enabled for the hart's context delivers nothing. It sidesteps the
  ordering hazard that a secondary comes online (running `init_this_cpu`) *before* the boot path
  calls `plic::init`; the CSR is unmasked now, the context is set up later.
- **The boot hart opens a target hart's context and routes the source to it** (`target_context` in
  `arch/riscv64/irq.rs`). The threshold and enable registers are global PLIC MMIO, so the boot hart,
  which runs every `enable` (test wiring, driver spawn), can open any hart's context and enable a
  source on it. The target is chosen round-robin over the online harts and is **stable per source**
  (`SOURCE_CTX`), for the same reason the GIC target is stable: the ACK re-enables the line on every
  completion, and the mask and the re-enable have to name the same context.
- **Each hart claims and completes against its own context** (`this_s_context`, the context table's
  entry for `cpu::id()`,
  passed to `plic::claim`/`complete`/`disable` from the external-interrupt handler). A source targets
  exactly one hart, so the hart that takes it is the hart it is enabled on, and the mask/complete land
  on the right context. The PLIC driver stays mechanism-only: it is told the context, it does not read
  the hartid (rule #2, DECISIONS §4).
- **The enable bits are serialized; nothing else in the driver is.** The sentence two bullets up, "the
  threshold and enable registers are global PLIC MMIO," is exactly what makes this necessary, and the
  assembly audit ([arch-audit.md](arch-audit.md), finding 3) is where it was caught. One enable
  register carries **32 sources** of a context, so setting one source's bit is a read-modify-write over
  a word its neighbours share, and the boot hart running `enable` can collide with another hart's
  handler running `disable` on a neighbour. A lost update either masks a device forever (its driver
  blocks on an interrupt that never arrives) or leaves a level-triggered source live after the handler
  masked it (an interrupt storm on that hart). So `enable`/`disable` go through one helper holding an
  `IrqSafeMutex` at `rank::IRQ_CONTROLLER`, the same rank the GIC's lock takes.

  It has to be `IrqSafeMutex` rather than a plain lock, and that is the §9 deadlock rather than
  caution: `disable` runs *in the handler* while `enable` runs in thread context with interrupts on,
  so a plain spinlock would let a thread take the word on a hart and then have that hart's own
  handler spin on it forever. Nothing else in the driver takes the lock, deliberately:
  claim/complete is per-context and therefore hart-local (the hot path stays lock-free), the
  per-source priority register is unshared, and the threshold write is a whole-word store of `0`, so
  it is idempotent rather than an RMW. **aarch64 never needed this**: the GIC's
  `ISENABLER`/`ICENABLER` are write-1-to-set and write-1-to-clear, so one store touches one line and
  the architecture supplies the atomicity that the PLIC's plain read/write bits do not.

### The lost update, step by step

The abstract phrase "read-modify-write" hides the bug, so here it is concretely. Say the disk is
source 8 and the NIC is source 10. Both bits live in the same 32-bit enable word (sources 0..31 of
that context), so a driver touching *its own* source still writes the neighbour's bit back, because
the only way to change one bit of that word is to store all 32.

Hart 0 is in thread context enabling the disk. Hart 1 is inside its handler masking the NIC. Without
the lock they interleave:

| step | hart 0 (`enable(8)`)        | hart 1 (`disable(10)`)      | word in the PLIC |
|------|----------------------------|-----------------------------|------------------|
| 1    | read → `0b0100` (bit 10)   |                             | `0b0100_0000_0000` |
| 2    |                            | read → `0b0100` (bit 10)    | `0b0100_0000_0000` |
| 3    | or in bit 8                |                             | `0b0100_0000_0000` |
| 4    | write `bits 8,10`          |                             | `0b0101_0000_0000` |
| 5    |                            | clear bit 10 from ITS copy  | `0b0101_0000_0000` |
| 6    |                            | write `0b0000`              | `0b0000_0000_0000` |

Hart 1's copy of the word was read at step 2, before hart 0's store at step 4. Its store at step 6
is computed from that stale copy, so it does not just clear bit 10, it *un-sets bit 8 as collateral*.
Hart 0's enable is gone, and nothing anywhere reports an error: the PLIC has no notion of a
disagreement, it just holds whichever word was written last.

Both directions of that race are bad, and they fail differently, which is why neither is tolerable:

- **A lost `enable`** (above) silently masks a device forever. The driver blocks in `Irq::WAIT` on an
  interrupt the PLIC will never route to it. The symptom is a hang in code that has no bug.
- **A lost `disable`** is the mirror: the handler thinks it masked a level-triggered source, but the
  bit is back on and the line is still asserted, so the PLIC re-delivers immediately, forever. That
  is an interrupt storm, and the hart it lands on stops making progress.

This is also why the window is small but not negligible. It is the few instructions between the read
and the write, hit only when two harts touch the same 32-source word at once, which is exactly the
kind of race that passes a thousand test runs and then fails in front of an audience. The audit found
it by reading for the pattern rather than by waiting for it to bite ([arch-audit.md](arch-audit.md),
finding 3).

The fix is one `IrqSafeMutex` around the read and the write together, so steps 1-4 and 2-6 cannot
interleave: whichever hart takes the lock second re-reads the word *after* the first hart's store and
computes from current state. It is the smallest possible critical section (a read, an or/and-not, a
write) and it is off the hot path, so it costs nothing measurable.

Proven the same way as the GIC: the riscv suite is green at the 4-hart boot with device sources
spread across harts (disk read, the interrupt-driven redoxfs_server block server, both routed to whatever
hart the round-robin picked, plus the SMP placement tests). As on aarch64, this distributes the
interrupt and the wake it causes; it does not by itself parallelize the `std_net` pipeline, which
needs the load-aware rendezvous wake (see below).

## The `std_net` SMP hang was not an interrupt-delivery bug (resolved)

Recorded so it is not re-diagnosed as one, and left in place with its resolution because the wrong
diagnosis was tempting and the right one took two agents to reach.

The `std_net` test (smoltcp in `net_stack` serving a std program's `UdpSocket`/`TcpStream`) used to hang
under the 4-core boot on **both** ISAs, watchdog-killed at 60 s, while in the same run the hand-built
DHCP round trips (`virtio_net`, `virtio_net_pci`) passed and the interrupt-driven redoxfs_server block
server passed. That asymmetry was the tell: interrupt delivery under SMP was sound, and the hang was
specific to the heavier, longer, timer-driven smoltcp pipeline.

Two things were wrong, and only one of them was scheduling:

1. **A real deadlock.** `net_stack` blocked on the NIC interrupt while smoltcp still had a retransmit
   pending, so neither side would move: the timer that would have retransmitted was never polled
   because the thread was parked in `Irq::WAIT`, and no packet was coming to wake it. That is a
   mutual-idle deadlock, not slowness, and no amount of core placement fixes it. `net_stack` now bounds
   its wait by smoltcp's own next-poll deadline.
2. **Serialization on one core.** With the deadlock gone the pipeline ran, but slowly: its threads
   are woken by a mix of device IRQ and IPC rendezvous, and both wakes pinned to one core. DECISIONS
   §28's wake split fixed the half that mattered (device-IRQ wakes go load-aware, IPC rendezvous
   wakes stay local, because a rendezvous partner is about to run on the caller's core and moving it
   only adds a migration).

An intermediate hypothesis of mine was **wrong and worth keeping written down**: I expected IRQ
affinity alone to fix it. It cannot. Spreading interrupts across harts relocates where a wake lands;
it does not parallelize a chain of request/response rendezvous, which is serial by construction. The
agent disproved it by measurement rather than argument.

`std_net` now passes on both ISAs. It is also the longest honest test in the suite at roughly 300 to
344 s on aarch64, because it is a real DHCP lease plus TCP and UDP round trips through an emulated NIC
under TCG. That length is legitimate work, not a symptom, which is precisely what makes it awkward for
the watchdogs: see the per-test ceiling discussion in [scheduler.md](scheduler.md).

## Two GICs: which one this machine has, and how the kernel knows

*(Milestone 227, 2026-09-19. Until then this section was headed "BUGS: this is a GICv2 driver", and
the measurement that made that heading dangerous is kept below.)*

The kernel drives **GICv2 and GICv3**, and picks between them at boot from the device tree. The
interesting question for an aarch64 board is still which interrupt controller it has rather than
which CPU, but the answer is now a driver the tree already carries rather than a port: a GIC-400
(Raspberry Pi 4, the Jetson TX1 that is argon) and a GIC-500 or GIC-600 (i.MX8M, most server
parts) are both covered. Apple Silicon is not a GIC at all (it has AIC), which is why an Apple core
only ever meets this kernel under a hypervisor that emulates one.

| | GICv2 | GICv3 |
|---|---|---|
| found by | `compatible` = `arm,gic-400`, `arm,cortex-a15-gic` or `arm,cortex-a7-gic` | `compatible` = `arm,gic-v3` |
| distributor | `drivers/gic.rs` | `drivers/gicv3.rs` |
| per-core private interrupts | banked in the distributor | a redistributor frame per core, found by its `GICR_TYPER` affinity (`drivers/gicv3.rs`) |
| CPU interface | banked MMIO, `drivers/gic.rs` | `ICC_*` system registers, `arch/aarch64/gic_cpu_interface.rs` |
| SPI routing | `GICD_ITARGETSR`, an 8-bit core mask | `GICD_IROUTER`, the target core's affinity |
| SGI | an `GICD_SGIR` store | an `ICC_SGI1R_EL1` write, `dsb ishst` before it |
| confirmed at boot by | `GICC_IIDR.ArchitectureVersion` from the block the tree calls the CPU interface | `GICD_PIDR2.ArchRev`, then `ICC_SRE_EL1.SRE` sticking |

**One place holds both.** `arch/aarch64/irq.rs` reads the version (`machine_discovery::gic`,
host-tested against QEMU's own GICv2, GICv3 and HVF trees), asks the hardware whether the tree is
right, and only then initializes a driver. Every other caller in the kernel (the IRQ handler, the
timer, the scheduler's reschedule SGI, the tests) names `arch::irq` and never a driver, so the
choice is made once. The split between `drivers/` and `arch/` is DECISIONS §4 rule 1: the ICC
registers are `msr`/`mrs`, so they are architecture code, while everything behind a pointer stays a
driver that is handed its addresses. `design/roadmap/227-gicv3-driver.md` has the placement and what
lost.

### How to run each

```sh
script/test --arch aarch64                    # GICv2, the TCG default and the one CI runs
NIFE_GIC=3 script/test --arch aarch64         # GICv3 under TCG
script/test --hvf                             # GICv3 on the physical core; HVF refuses a GICv2
NIFE_GIC=3 cargo xtask boot-check --arch aarch64   # the boot self-test's verdict, one GICv3 boot
```

The boot line says which one it found:

```
  interrupts      : GICv2, distributor 0x0000000008000000, cpu interface 0x0000000008010000
  interrupts      : GICv3, distributor 0x0000000008000000, redistributors 0x00000000080a0000, cpu interface in system registers
```

**Why TCG stays at 2.** Every recorded TCG number (the icount tripwire's baselines, the fastpath
footprint, the benchmark history) was taken on a GICv2, and argon is a GIC-400. HVF has no choice:
QEMU 11.1.1 refuses `gic-version=2` with HVF, so the runner asks for 3 there.

### The gate against silent loss

A GIC assumption that stops being true now fails loudly, at three points:

- **An `intc@` node whose binding is not a GIC this kernel knows** panics in `memory::init`, naming
  the `compatible` it found. It used to be driven as a GICv2 on the strength of its name.
- **A tree the hardware contradicts** panics in `arch::irq::init` before any configuration write,
  naming the version claimed and the revision read. This is the check that would have stopped
  milestone 222's boot: a GICv2 claim over a redistributor frame reads `GICC_IIDR` as zero.
- **A core with no redistributor, or whose `ICC_SRE_EL1.SRE` will not set**, panics as it comes
  online. And `arch::irq::tests::every_online_core_takes_its_own_timer_ticks` holds every core, not
  only the test's own, to taking interrupts; injecting "secondaries never enable Group 1" fails it
  naming core 1.

### BUGS

- **`ID_AA64PFR0_EL1.GIC` reads zero under HVF** on an Apple core while QEMU emulates the GICv3
  system registers behind it (measured 2026-09-19: `0x1101000010110011`). The first draft of the
  boot check required that field and refused to boot there. The check now rests on the
  distributor's revision and on `SRE` sticking, which is what Linux measures too.
- **No ITS**, so no LPIs and no MSI translation on a GICv3. Milestone 317 wants it for interrupt
  remapping; under HVF QEMU offers a GICv2m frame instead of an ITS anyway.
- **The GICv2 SGI has no barrier before its `SGIR` store**, which the GICv3 path has. Recorded in
  `drivers/gic.rs`, unchanged so the GICv2 path's counts stay still.
- **The EL2 half (`boot.s` opening `ICC_SRE_EL2`) cannot be proven on QEMU**, whose `ICC_SRE_EL2`
  reads as set whether or not the step runs. It is Linux's sequence, and the EL1 assertion is what
  would catch a board where it matters.

### The measurement, taken 2026-09-02, and why this used to be the most dangerous line in the file

Until milestone 227 the runner's `gic-version=2` pin was protection rather than support, and this
section said so. The measurement below is what made it load-bearing, and it is kept because it is
the shape the gates above exist to make impossible.

This section used to say `-machine virt,gic-version=3` had never been booted here, that it was one
command, and that it would turn "our driver does not support it" from an assumption into a recorded
failure with an error message attached. Milestone 222 ran the command. There is no error message.

```
  timer       : 100 Hz tick, counter at 62 MHz, interrupts ON
  scheduler   : 5 thread(s), round robin, preemptive
  ...
    thread 1 (hostile) :          0 iterations
    thread 2 (polite)  :          0 iterations
    preemptions        :          0
```

Under `-machine virt,gic-version=3,iommu=smmuv3 -cpu cortex-a72 -smp 4` the kernel **boots all the
way through the tour**, brings four cores online over PSCI, prints that the timer's interrupts are
on, and then never receives one. Nothing faults and nothing says anything is wrong.

The reason is that a GICv3 device tree node carries two register blocks like a GICv2 one does, so
`memory::gic_regions()` finds a pair and hands them over without complaint. The second block is the
**redistributor** rather than a CPU interface, and `gic::init_this_cpu` writes `GICC_CTLR` and
`GICC_PMR` into it. Those are device-memory writes to registers that are not there: they land, they
do nothing, and the real CPU interface (`ICC_*`, system registers) is never enabled. Every interrupt
the distributor routes is then delivered to a core that has not agreed to receive any.

**So the honest statement of the limitation is not "GICv3 is unsupported", it is "GICv3 boots and
silently loses every interrupt".** That was the more dangerous shape, and it was why the QEMU runner's
`gic-version=2` pin was load-bearing rather than tidy: without it, a QEMU that changed its default
would have produced a kernel that came up and then quietly stopped preempting anything.

### What a GICv3 driver would actually be, priced in advance, and what it turned out to be

Milestone 222 priced it from the failure: a version decision at init, a redistributor per core, a
system-register CPU interface under `arch/aarch64/`, affinity routing, and both drivers coexisting
behind roughly ten call sites. Milestone 227 built exactly that list and found two things the
pricing did not have: **SGIs must stay permanently enabled** on a GICv3, because QEMU's GICv2 model
and the GIC-400 treat them that way and the Irq capability's mask-on-fire, unmask-on-ACK protocol
relied on it without saying so; and **the ID register that says whether a core has the system
registers is not believed by HVF** (see BUGS above). The call sites all moved to `arch::irq`.

### Why there is no aarch64 CPU-model matrix, unlike RISC-V's (milestone 59, DECISIONS §53)

The asymmetry is real and worth stating, because "we did it for RISC-V" is the obvious argument for
doing it here and it is wrong.

- **We already test on a conservative real core.** The aarch64 runner uses `-cpu cortex-a72`, an
  ARMv8.0-A chip, not QEMU's `max`. RISC-V's default was the maximalist model, which is what made a
  matrix worth building there. Here the emulator is *less* capable than a modern board, and code
  that runs on an A72 runs on an A76.
- **aarch64 has architectural feature discovery and RISC-V does not.** The `ID_AA64*` registers are
  mandatory and readable at EL1, and this kernel already uses them: `arch/aarch64/mmu.rs` reads
  `ID_AA64MMFR0_EL1::PARange` and feeds it to `TCR_EL1::IPS` rather than assuming a physical address
  range. That is why milestone 60 (ISA discovery) is a RISC-V milestone specifically; RISC-V omitted
  CPUID on purpose and left discovery to a device-tree string.

### What no CPU matrix catches on either ISA

**Memory ordering.** Different microarchitectures reorder differently, and a missing
`Acquire`/`Release` can pass on one core and fail on another. **QEMU's TCG does not faithfully model
reordering**, so no `-cpu` value tests it. That class is covered by a different mechanism and
`ci.yml` says so: CI runs on a real aarch64 runner, because a missing barrier passes on an x86_64
host and fails only on real ARM. Real silicon is the test; an emulator cannot be.

# Interrupts or polling: why every confined driver in this tree polls

calef, 2026-09-17: *"What are the advantages and disadvantages of Irq vs polling? This has come up
multiple times."* It had, three times in two days, and the answer had never been written down, which
is why it kept coming up. This section is the record.

## The ordinary trade, which is not what decides it here

**Polling wins on** latency at depth (no delivery, no context switch, no EOI), determinism (an
icount tripwire can read a polled loop and cannot read an interrupt's arrival), and simplicity (no
vector allocation, no masking, no affinity).

**Polling loses on** a burned core while idle, which is ruinous for a device asked for 64 bytes once
a boot; scaling, because N polling drivers are N spinning threads; and it cannot express *wake me
when something happens*, which is the whole of milestone 103.

**None of that is why this tree polls.**

## What actually decides it: an MSI is a memory write

An MSI or MSI-X message is a memory write to an architecturally special address. **Plain DMA
remapping does not confine it**: a component that can write a device's MSI-X table can aim an
interrupt at a vector it was never given. [DECISIONS §86](../design/decisions/86-el0-nvme-driver.md)
found this and named the axis; `notes/confinement-claims.md` carries it as a claim stated nowhere.

Linux draws the same line and says so in its API: **VFIO refuses to hand a device to an untrusted
userspace driver on a machine without interrupt remapping**, and its escape hatch is named
`allow_unsafe_interrupts`. (§86 marks that as confirmed by search paraphrase rather than a fetched
source, and this note inherits the caveat rather than laundering it.)

**Measured on the machines this project runs, not asserted:**

| | where MSI remapping lives | state here |
|---|---|---|
| `x86_64` | a separate IOMMU feature, `intremap=on` | **off** in every boot; the runner sets `-device intel-iommu` without it |
| `aarch64` | a separate *device*, the GICv3 ITS | **absent**: the TCG runner uses `gic-version=2`, which has none, and milestone 227's GICv3 driver does not drive the ITS a `gic-version=3` machine offers |
| `riscv64` | **inside the IOMMU's own device context** (`CAP_MSI_FLAT`, widening it 32 → 64 bytes) | **already driven**, `arch/riscv64/iommu.rs` handles both formats |

So on two of three architectures an IRQ-driven EL0 driver **would not be confined**, whatever the
IOMMU does for DMA. That is why milestone 159's TRNG driver polls and why milestone 261's NVMe
server polls: in 261's lane's words, *"polling keeps `Object::Irq` off the grant list, which is less
authority."* **A confinement choice wearing a performance choice's clothes**, and worth saying out
loud because the next person meeting a polled driver will reasonably read it as a shortcut.

## What is not in question

**An interrupt is already a capability here.** `Object::Irq(u32)` has existed since milestone 9: the
kernel masks the line, `READ` lets the holder `WAIT` and `ACK`, and everything that knows what the
*device* is lives in the userspace driver. That is seL4's `IRQHandler` shape, and
[DECISIONS §101](../design/decisions/101-notification-objects.md) already specifies `bind_irq(intid,
ep)` on the delivery side, with a prior-art survey covering seL4's bound notifications, Fuchsia's
`zx_port`, Mach port sets and why Linux's `epoll` model was refused.

**`Object::Irq` confines a line-based interrupt perfectly well.** The kernel masks it; the holder
can only wait and acknowledge; there is no way to aim it. MSI-X is the case it does not cover.

## The open question, and why it is deliberately not a decision yet

**Who owns the page holding the MSI-X table?** calef declined to mint that on 2026-09-17, and the
reason is the one `design/fatal-risks.md`'s own rule 1 gives: nothing is blocked on it, both EL0
drivers poll, and **there was no experiment behind it**. A decision with no experiment is a worry
rather than a choice.

A lane is building the experiment as this is written: making interrupt remapping reachable on the
machines this project runs, so the question becomes live and cheap rather than argued. (Its
milestone number is deliberately not cited here, because the block had not merged when this was
written and `script/lint` refuses a citation to a block that does not exist, which is the gate doing
its job.) When it is, the options are
roughly: EL0 drivers get `Irq` for line-based interrupts only and MSI-X stays kernel-owned; or
remapping is turned on and a driver may own its table because the platform confines it; or it is
allowed unconfined and said so, which is Linux's `allow_unsafe_interrupts` and is a claim this tree
should not make quietly.

**One asymmetry worth carrying forward**: on `riscv64` the IOMMU and the interrupt-confinement story
are the same hardware, so a board with a ratified RISC-V IOMMU would close milestone 143 *and* this
gap in one purchase. Milestone 143's 2026-09-17 survey found such a board buyable.
