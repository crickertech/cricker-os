# Can a userspace process hold a timer?

*(Written 2026-09-05 for milestone 263, a spike. This note prices a design and does not build one:
nothing here adds a syscall or an object, and the fork it informs is an architect's. Name
provisional: `timer-capability.md` is a sibling of `timed-wait.md`
rather than a second copy of it, and the two answer different halves of it.)*

The answer, first. The userspace-timer-service answer to milestone 106 does not survive
[§19 (architectural)](../design/decisions/19-architectural-parity.md) parity, and the reason is not the one the
milestone block predicted.

Two clauses, and both are needed:

1. On riscv64 no *architected* timer can be granted to U-mode, on any machine, by the privileged
   architecture rather than by a gap in this kernel. There is no U-mode timer-compare CSR, no enable
   bit that would create one, and no route to SBI from U-mode.
2. The only mechanism that works on all three is per-board MMIO, and it is not uniformly
   available. The two real boards have generous MMIO timer blocks (argon fourteen channels, radon
   four, each with its own interrupt line), but QEMU's aarch64 `virt` has no MMIO timer device at
   all and that is the machine every gate in this tree runs on. So a userspace timer service would
   be a different driver per board, absent on the aarch64 CI machine, which is §19's own definition
   of the bug rather than a parity story.

The correction worth having is on the third architecture the block wrote off. aarch64 can grant a
timer comparator to one thread rather than to all of EL0, using machinery this tree built for a
different reason three days ago.

## What each specification actually says

Three specifications, read rather than recalled, because milestone 263's own `BUGS` section says the
table it was scoped from is this tree's doc comments and *"the maintainer's reading is the thing most
likely to be wrong here."* It was wrong about one of three.

### aarch64: the block's negative is refuted

The block says `CNTKCTL_EL1` "can open EL0 access, but that is ambient across EL0 rather than held by
one process." Both halves need correcting.

The register has four independent enables, not one, and two of them are about the comparators
rather than the counters. From Arm's own machine-readable system-register description of
`CNTKCTL_EL1` (rendered at `arm.jonpalmisc.com/latest_sysreg/AArch64-cntkctl_el1`, read 2026-09-05;
the normative text is the Arm Architecture Reference Manual for A-profile, `CNTKCTL_EL1`):

| bit | field | what it opens to EL0 |
|---|---|---|
| 0 | `EL0PCTEN` | `CNTPCT_EL0`, `CNTPCTSS_EL0` (and conditionally `CNTFRQ_EL0`) |
| 1 | `EL0VCTEN` | `CNTVCT_EL0`, `CNTVCTSS_EL0` (and conditionally `CNTFRQ_EL0`) |
| 8 | `EL0VTEN` | `CNTV_CTL_EL0`, `CNTV_CVAL_EL0`, `CNTV_TVAL_EL0` |
| 9 | `EL0PTEN` | `CNTP_CTL_EL0`, `CNTP_CVAL_EL0`, `CNTP_TVAL_EL0` |

Of `EL0PTEN`: *"Traps EL0 accesses to the physical timer registers to EL1, or to EL2 when it is
implemented and enabled"*, and when the bit is 1, *"this control does not cause any instructions to
be trapped."* `EL0VTEN` says the same of the virtual timer registers.

This kernel writes only bit 1. `kernel/src/arch/aarch64/timer.rs`'s `init` sets `EL0VCTEN` and
nothing else, which is why the tree's own comments describe the timer registers as trapped: they are,
because the bit that would open them has never been written. The registers exist and are grantable;
this tree has never granted them.

And the grant can be per thread, because the tree now has the machinery.
`design/decisions/139-cycle-counter-authority.md` says, in its "what this tree already does" section,
that `CNTKCTL_EL1.EL0VCTEN` and `scounteren.TM` are *"per-machine bits, set once at init. There is no
precedent in this tree for a per-thread system-register bit maintained across a context switch. That
is the one piece of machinery option 4 below needs and the tree does not have."*

That sentence is now stale, and its own decision is what made it stale. Milestones 229 and 237
built exactly that: a `cycle_counter_grant` bool on `Thread`, read in `sched::schedule` at a
`#[cfg(any(test, feature = "cycle_counter_grant"))]`-gated switch site (milestone 300 removed the
const-`false` helper `cycle_counter_grant_of` that used to carry it through the shipping switch
tuple), and installed on the core about to run the thread by
`install_cycle_counter_grant` -> `arch::timer::set_cycle_counter_grant`, which is a cached
compare-and-skip around a `PMUSERENR_EL0` write on aarch64 and an `scounteren` write on riscv64. A
per-thread `CNTKCTL_EL1.EL0PTEN` grant is the same shape at the same call site, on a register two
lines away in the same file. The precedent §139 wanted exists.

What is genuinely scarce is comparators, not authority. At EL1/EL0 aarch64 offers two: the
physical timer (`CNTP_*`, GIC INTID 30) and the virtual timer (`CNTV_*`, INTID 27). This kernel's
tick owns the virtual one, and owns it *because* of a portability finding this tree paid for: the
physical timer *"traps under a hypervisor: the physical timer belongs to EL2, and a guest at EL1 that
writes `CNTP_CVAL_EL0` takes an 'Unknown reason' trap"*, found on Apple's Hypervisor.framework
(`kernel/src/arch/aarch64/timer.rs`, milestone 9). So:

- There is exactly one spare comparator on aarch64, the physical timer.
- It is spare on bare metal and under QEMU/TCG, and not under a hypervisor, which is the same
  finding that pushed the tick onto the virtual timer in the first place. A grant that works on
  argon and fails under HVF is a parity problem of its own kind.
- It is per PE, so a service holding it is pinned to a core, or its deadline follows it.

### riscv64: the block's negative is confirmed, and it is total

Three routes, all closed.

Sstc's `stimecmp` is S-mode, and U-mode is not mentioned because it was never a candidate. From
the ratified *"Sstc" Extension for Supervisor-mode Timer Interrupts, Version 1.0* (RISC-V
International; the ratified PDF at `docs.riscv.org/reference/isa/extensions/sstc/`, read 2026-09-05):

> When the TM bit in the mcounteren register is clear, attempts to access the stimecmp register while
> executing in S-mode will cause an illegal instruction exception. When this bit is set, access to
> the stimecmp register (if implemented) is permitted in S-mode.

and

> Bit 63 of menvcfg [...] named STCE (STimecmp Enable) enables stimecmp for S-mode when set to one,
> and the same bit of henvcfg enables vstimecmp for VS-mode.

Every enable in the extension gates S-mode and VS-mode. There is no U-mode enable, and no U-mode
timer-compare CSR exists anywhere in the privileged architecture to be enabled.

The CSR numbering forecloses it independently of Sstc's text. The RISC-V Privileged Architecture's
CSR address convention (*Control and Status Registers (CSRs)*, the CSR listing chapter) says the two
bits `csr[9:8]` *"encode the lowest privilege level that can access the CSR"*, and that *"attempts to
access a CSR without appropriate privilege level or to write a read-only register raise
illegal-instruction exceptions."* `stimecmp` is `0x14D`: bits `[9:8]` are `0b01`, supervisor.
A U-mode `csrw stimecmp` is an illegal instruction by the address it is written at, and no
configuration bit changes that.

SBI is not reachable from U-mode either. This kernel's tick arms through `sbi_set_timer`, an
`ecall` from S-mode to OpenSBI in M-mode. An `ecall` executed in U-mode raises *Environment call from
U-mode* (`scause`/`mcause` 8), a different cause from *Environment call from S-mode* (9); it is
delivered to S-mode when `medeleg` delegates it and to M-mode otherwise, and in neither case does it
enter the SBI dispatch, which decodes only the S-mode and M-mode ecall causes. A U-mode program
cannot make an SBI call at all, which is exactly what the milestone block said.

The one comparator the machine has is already spent. RISC-V gives one `mtimecmp` per hart, in
M-mode's CLINT, and SBI TIME is the multiplexer over it. There is no second one, so even a
hypothetical MMIO grant of the CLINT would be handing away the kernel's own tick rather than a spare.

### x86_64: the exception is real, but it is not confirmed on the machine this project owns

The HPET is memory-mapped, has several independent comparators, and is therefore the one timer on the
three architectures that could be handed to a process with the tree's existing `DeviceFrame`
capability and no new mechanism at all.

What is confirmed in this tree: `crates/machine_discovery/src/acpi.rs` sees an `HPET` table in the
XSDT walk and does nothing with it, and `notes/x86-port/acpi-and-pci.md` records the QEMU q35 table list read on
2026-08-23 with `0x000ffe22a8 HPET (56 bytes)` in it.

What is not confirmed: xenon's own HPET. `notes/x86-uefi-boot.md`'s first-light section (2026-09-05)
says the boot tour printed *"the full table list"*, but the transcript is a photograph
(`art/bench/xenon-2026-09-05-first-light.jpg`) and the list is not written down in the note. Nothing
in this tree names an HPET on xenon, and nothing has read its `NUM_TIM_CAP`. So the x86_64 row is
"the architecture has a spare, and this project has not looked at its own machine's".

`design/decisions/139-cycle-counter-authority.md` already read the HPET specification for a different
purpose and its findings apply: the architectural floor is 100 ns per tick, the common part runs at
14.31818 MHz, and a *read* costs several microseconds, which is why Linux deleted the vDSO mapping.
None of that prices arming one, which is a write and a one-shot interrupt rather than a polled
read, so §139's conclusion (the HPET loses to a syscall as a *clock*) does not carry over to using it
as a *timer*. That distinction is worth keeping straight, because the two uses share a device and
nothing else.

## The other route: an MMIO timer the kernel already knows how to delegate

The three sections above are about the *architected* timer on each ISA. There is a second route that
needs no new mechanism at all, because this tree already has it: an MMIO timer block is a
`DeviceFrame` plus an `Object::Irq`, which is exactly how every userspace driver in this system
already owns a device. If a machine has a spare MMIO timer, a userspace timer service can hold it
today.

The machines do have them, and QEMU does not. Read 2026-09-05 from the vendor manuals, mainline
Linux bindings and QEMU's own source:

| machine | spare MMIO timer | channels | per-channel interrupt |
|---|---|---|---|
| argon, Jetson TX1 (Tegra X1 / T210) | yes, `timer@60005000` | 14 29-bit counters plus a 32-bit timestamp | yes, 14 distinct GIC SPIs |
| radon, VisionFive 2 (JH7110) | yes, `timer@13050000` (`si5_timer`) | 4, 24 MHz | yes, 4 distinct PLIC lines |
| xenon, OptiPlex 7050 | HPET, presumed present, not confirmed on this machine | up to 32; the spec's recommended minimum is 3 | yes, per-timer routing when the legacy route is off |
| QEMU `virt`, aarch64 | none | | |
| QEMU `virt`, riscv64 | the goldfish RTC's alarm only | 1 comparator | yes, IRQ 11 |

Sources and the exact text:

- Tegra X1. `Documentation/devicetree/bindings/timer/nvidia,tegra-timer.yaml` in mainline Linux:
  *"The Tegra210 timer provides fourteen 29-bit timer counters and one 32-bit timestamp counter... Each
  TMR can be programmed to generate one-shot, periodic, or watchdog interrupts."* and *"A list of 14
  interrupts; one per each timer channels 0 through 13"*, with `arch/arm64/boot/dts/nvidia/tegra210.dtsi`
  listing all fourteen SPIs. The Tegra X1 TRM itself is behind an NVIDIA developer login and was not
  read, so this row is mainline Linux written by NVIDIA's own maintainers rather than the vendor
  manual.
- JH7110. StarFive JH-7110 TRM, Preliminary V2 (2023-04-24, JH7110-TRMEN-001), *Timer → Overview*:
  *"Si5_timer consists of 4 decrement bd_timer that cause interrupts in single-run or continuous-run
  mode and have their own clock input."* Its *Interrupt Connections* table gives `TIMER_INTR[0..3]`
  four separate PLIC sources. There is no mainline Linux driver: the binding was posted ten times
  from December 2022 and never merged, and mainline's `jh7110.dtsi` has only the CLINT. The driver
  exists in StarFive's vendor tree (`drivers/clocksource/timer-starfive.c`,
  `compatible = "starfive,jh7110-timers"`). So this hardware would be a driver this project writes.
- QEMU aarch64 `virt`. `hw/arm/virt.c`'s `base_memmap[]` has no timer device. The only
  timer-adjacent entries are `VIRT_RTC` (a PL031) and `VIRT_GWDT_*`, the SBSA watchdog, which is not
  created by default. And the PL031's alarm is one-second resolution: `pl031_set_alarm` computes
  `ticks = s->mr - pl031_get_count(s)` and arms `now + ticks * NANOSECONDS_PER_SECOND`. Useless as a
  general-purpose timer.
- QEMU riscv64 `virt`. `hw/riscv/virt.c`'s `virt_memmap[]` has the CLINT and a goldfish RTC and no
  general-purpose timer. The goldfish RTC's alarm *is* usable: `hw/rtc/goldfish_rtc.c` has
  `RTC_ALARM_LOW/HIGH`, a nanosecond counter, and a real one-shot `timer_mod`. One comparator, and
  this tree already drives that device for the wall clock (`crates/clock_protocol`, §43).
- **HPET.** IA-PC HPET Specification 1.0a (Intel, October 2004). §2.3.4, `NUM_TIM_CAP` (bits 12:8):
  *"This indicates the number of timers in this block. The number in this field indicates the last
  timer"*, so the count is `NUM_TIM_CAP + 1`. §2.2's recommended minimum is **3 comparators**, all
  three one-shot capable and one periodic capable. §2.3.5's `LEG_RT_CNF`, when set, spends timer 0 on
  IRQ0 and timer 1 on IRQ8 and leaves *"Timer 2-n... routed as per the routing in the timer n config
  registers"*. On a minimum implementation with the legacy route on, that is exactly one free
  general-purpose timer.

Why this route does not rescue the userspace-service answer. It is a per-board driver rather than
a portable capability, so §19's *"a kernel capability ships on every supported architecture, proven by
the same suite"* is not met by it: the suite runs on QEMU, and QEMU's aarch64 `virt` has nothing to
drive. A userspace timer service would exist on argon, exist differently on radon, exist a third way
on xenon, and not exist at all on the machine this tree gates against. That is three drivers and a
hole, against one syscall method.

It is a good answer to a different question. If a *particular* workload on a *particular* board
wants a private high-resolution timer, this is how it gets one, and it needs nothing built. It is not
how a kernel serves `thread::sleep`.

## What the fourth shape costs

`Timer::ARM(deadline, notification)` -> the kernel signals that notification at the deadline.

The bookkeeping was already priced and is not re-measured here. `notes/timed-wait.md` measured any
deadline structure at one comparison per idle tick (1.000 comparisons and 0.000 writes over 100,000
ticks for a scan and for a sorted list, 1.004 for a wheel) and a per-thread `deadline: u64` at zero
bytes, because TCBs are page-resident with 3,352 bytes of slack. What was unpriced is the object:
a new `Object` variant, its creation from untyped, its dispatch arm, and whether any of it lands on
the IPC fastpath.

Method. A throwaway scaffold was built on 2026-09-05 and then deleted: `Object::Timer(u64)`, an
`abi::timer` module with `ARM` and `CANCEL`, a `RETYPE_OBJ` arm minting one from the caller's own
untyped, two `#[inline(never)]` dispatch bodies, a 64-entry armed-timer table with a cached earliest
deadline, and an `expire` call from `sched::on_tick` that signals through the existing
`sched::irq_notify`. `script/test` passed with it wired in (205 passed, 69 skipped, all three ISAs
plus the OVMF leg) before it was removed, so the numbers are from a working system and not a
half-compiled one.

### The object is free in bytes

`kernel/src/cap.rs` asserts `size_of::<Object>() == 24` and `size_of::<Cap>() == 32` at compile time,
and both assertions still hold with the variant added, because `PageFrame(u64, NonZeroU64)` is
already the widest arm. So a `Timer` capability costs zero additional bytes per capability slot,
zero per capability table, and zero per TCB. That is the whole of the per-thread and per-process cost.

### The IPC fastpath is untouched, measured

`script/fastpath-footprint`, before and after, on the same machine within twenty minutes:

| ISA | `ipc_fastpath` before | after | `syscall_entry` before | after | delta |
|---|---|---|---|---|---|
| aarch64 | 7,028 | 7,028 | 1,504 | 1,516 | +12 B (+0.8%) |
| riscv64 | 5,936 | 5,936 | 1,828 | 1,986 | +158 B (+8.6%) |
| x86_64 | 8,122 | 8,122 | 1,637 | 1,733 | +96 B (+5.9%) |

`ipc_fastpath` does not move at all on any architecture, which is the number that matters: an
IPC round trip fetches exactly what it fetched before. `syscall_entry` moves because that half is
measured *flat* (the decoder's own bytes are on every syscall, and its other arms are not on this
path), so one more `Object` arm in `invoke` is one more decode step's worth of bytes for every
syscall in the system whether or not it is a timer.

The riscv64 figure is the one to argue about. 158 bytes is 8.6% of that ISA's entry set, and
`script/fastpath-footprint`'s own `BUGS` names this exact mechanism as an open problem
(`design/roadmap/368-a-flat-entry-set-counts-bytes-no-syscall-fetches.md`). Three instances were
closed with `#[inline(never)]` in two days on 2026-09-04; both scaffold bodies here already carry it,
so this is the cost *after* that mitigation rather than before it.

### The kernel grows by under a quarter of a per cent

Sum of symbol sizes in the release kernel (`llvm-nm --print-size`), before and after:

| ISA | before | after | delta |
|---|---|---|---|
| aarch64 | 1,009,252 | 1,009,990 | +738 B (+0.073%) |
| riscv64 | 1,011,788 | 1,014,054 | +2,266 B (+0.224%) |
| x86_64 | 962,936 | 964,322 | +1,386 B (+0.144%) |

That is the whole feature: object variant, two methods, retype arm, expiry table, tick hook, and the
signalling call. The riscv64 figure is again the largest, for the same reason its entry set is.

### What this does not price

- The scaffold's expiry table is a 64-entry array with a linear rescan on every arm. That is the
  crudest of the three structures `notes/timed-wait.md` modelled and was chosen because it is the
  smallest thing that runs; a real implementation would put the deadline on the object's own page,
  the way every other page-resident object in this kernel works, and the arm would not rescan.
  The size figures above are therefore an over-estimate of the table and an under-estimate of the
  revocation and generational-naming machinery a real object needs.
- Revocation was not built. A real `Timer` is retyped from untyped and must die when its region
  is destroyed (`MemoryRegion::DESTROY`, object revocation), which is bookkeeping the scaffold has
  none of. Priced 2026-09-13, below: it is 268 bytes and it is not optional.
- Nothing was measured in time, only in bytes. `notes/timed-wait.md`'s +30/+31 instructions per
  tick is the executed-path number and it still stands; nothing here changes it.
- The kernel-side consumer was not priced, because on 2026-09-05 nobody had found one.
  Milestone 106's own census found one on 2026-09-13 (`kernel/src/soak.rs`'s supervisor), and it is
  priced below.

## What the holder dying with a timer armed costs, measured

Priced 2026-09-13 by a second scaffold, built on the shape the section above says a real
implementation would take (page-resident state, a generational registry beside `rendezvous_table`)
rather than on the 64-entry array, and deleted like the first. Method and error bars below.

The answer is 268 bytes of code on aarch64, 212 on riscv64, 276 on x86_64, and zero bytes of
data. What matters is not the size, though, it is the category, and the category was wrong in the
line above: this is not bookkeeping that can be deferred to a later milestone.

It is a use-after-free, and it is one the timer interrupt performs. The expiry walk resolves a
registry entry to a page (`phys_to_virt(phys)`) and both reads and writes it (the disarm). A timer
page freed by `MemoryRegion::DESTROY` and handed to somebody else is therefore read and written by
`on_tick`, on every core, for the life of the machine, with no syscall involved and nothing to
correlate it to the process that died. That is the same class of defect `revoke.rs`'s header opens
with (*"wiring up any reclamation before revocation exists turns those 'harmless' dangling mappings
into a use-after-free"*), and it lands on the one path that runs when nothing else is happening.

So the shape a `Timer` needs is the shape a `Rendezvous` already has, and that is why it is
cheap: `sched::reap_region_objects` already sweeps `rendezvous_table` for objects whose page lies in
the dying region, one at a time, rescanning rather than building a worklist array (that function's
own comment explains why: it is the deepest frame in the kernel and a scratch buffer does not fit).
A timer phase is the same loop against `timer_table`, placed before the rendezvous phase, plus
one line resetting the cached earliest deadline. Measured as the delta between two builds that
differ only by that loop:

| ISA | code bytes | data bytes |
|---|---|---|
| aarch64 | +268 | 0 |
| riscv64 | +212 | 0 |
| x86_64 | +276 | 0 |

Three properties fall out of the existing design rather than needing to be built, and each is
worth naming because each is a question a reader will ask:

1. A timer armed at a notification that dies first is already harmless. The expiry signal goes
   through `sched::irq_notify`, whose own comment is *"a stale name ... is simply dropped: an
   interrupt with no live rendezvous has nowhere to go, which is not an error."* Generational naming
   does the work; nothing is owed here.
2. The cached earliest deadline may be early but never late, so the reap does not have to
   recompute it. Setting it to 0 costs one extra walk on the next tick and cannot miss an expiry.
   A hint that is wrong in the safe direction is the cheaper half of this design.
3. The registry slot comes back with the page. Without the sweep a dead process's armed timers
   hold registry slots forever, which is a denial of service on a fixed registry that a process can
   drive by spawning and dying in a loop. The sweep closes that as a side effect of closing the
   memory-safety hole.

What is still not priced here. `PageFrame::REVOKE`'s capability-scoped question (§132 (what)) has no
analogue above: this prices reclamation (object-blind, the `DESTROY` path), not "take this holder's
authority back while the object lives". A `Timer::REVOKE` is a separate question and this scaffold
did not ask it.

## What serving a kernel thread would cost, measured

This does not decide whether the kernel thread should be served, which is an architect's under
§101 (notification)'s carve-out and milestone 106 (deadline)'s to reopen. It
says what it would cost, because a spike that
noticed the question and left it unpriced sends the decision back for a second round.

The consumer is real and it is in the tree. `kernel/src/soak.rs`'s supervisor, whose own `BUGS`
says *"It yields in a loop rather than blocking on a timer, because this kernel has no sleep-until
primitive a kernel thread can use. That is load on the machine under test."* It is a watchdog, which
is the first item on §101's own list of kernel needs.

A userspace timer service cannot serve it, and neither can `Timer::ARM`. The reason is the same
one for both and it is not about architecture: a kernel thread runs at EL1 (S-mode, ring 0) and
cannot issue a syscall at all. `syscall::invoke` is reached from the trap path and takes a
`&mut TrapFrame`; a kernel thread never traps. It is not for want of a cspace, which is the thing a
reader expects to be missing and is not: `Thread::spawn_into` gives every kernel thread a
`CapabilityTable::new()`, empty, commented *"it can name nothing until it is handed something."* The
authority is available and the door is not.

So what a kernel thread needs is smaller than the fourth shape, not larger, because inside the
kernel authority is not the question being asked. No object, no capability, no notification, no
syscall: a deadline word on the TCB and a walk that wakes it.

Measured the same way, as the delta of one build against the build above it:

| ISA | code bytes | data bytes |
|---|---|---|
| aarch64 | +312 | 0 |
| riscv64 | +282 | 0 |
| x86_64 | +400 | 0 |

The data figure is the interesting one and it is exact. `notes/timed-wait.md` predicted a
per-thread `deadline: u64` at zero bytes, on the argument that a TCB is page-resident with slack.
Measured on this tree: `size_of::<Thread>()` is 1,152 bytes with the field, in a 4,096-byte page,
so 2,944 bytes of slack remain. The prediction holds. (`notes/timed-wait.md` quotes 3,352 bytes of
slack, which was true when written; the TCB has grown since, and the conclusion has not changed.)

The whole primitive is `sched::sleep_until(deadline)`: under `IPC_TABLES`, write the deadline,
`handshake.park` on no rendezvous (which is what a `CALL` caller already does), lower the cached
earliest, release, `schedule()`. The expiry walk gains a second loop that wakes threads whose
deadline has passed, `serve()` first so the boot-8 wake gate lets it through. It shares the cached
earliest with the object path, so **an idle tick still costs one comparison** whether or not anything
is sleeping.

**And it was applied to the real consumer, which is what makes this a measurement rather than a
sketch.** `soak.rs`'s beat loop (a `now()`/`wrapping_sub` deadline comparison wrapped around
`sched::yield_now()`, six lines) becomes `sched::sleep_until(due)`, and the kernel builds clean with
`--features soak_test` (spelled `--features soak` when this was measured on 2026-09-13; milestone
297 renamed it the following day). That closes the `BUGS` entry quoted above rather than merely addressing it.

**What this does not tell you.** Nothing was run: the scaffold builds and is deleted, so there is no
evidence the supervisor wakes on time, only that the code the wake would run compiles and
that its cost is four hundred bytes or less. The `BUGS` section below carries that.

### Method and error bars, both scaffolds

**Four kernels, each differing from the one above it by exactly one change**, built release for all
three ISAs and measured with `llvm-nm --print-size`, splitting symbols by type: `t` is code,
everything else (`d`, `b`, `r`) is data.

| build | what it adds | aarch64 code | riscv64 code | x86_64 code |
|---|---|---|---|---|
| B0 | nothing; `origin/main` at `7317cdff` | 193,984 | 163,990 | 140,842 |
| B1 | the fourth shape, page-resident | 199,148 | 165,830 | 146,618 |
| B2 | the reap sweep | 199,416 | 166,042 | 146,894 |
| B3 | `sleep_until` and the thread walk | 199,728 | 166,324 | 147,294 |

**The error bar is zero bytes.** B1 was built, reverted to B0, and rebuilt from the same patch, and
both B1 runs produced byte-identical figures on all three ISAs. Nothing here is within noise of
anything, because there is no noise.

**`ipc_fastpath` does not move**, reproducing the 2026-09-05 measurement exactly: 7,028 / 5,936 /
8,122 before and after, on `script/fastpath-footprint`. `syscall_entry` moves by +12 B on aarch64
(+0.8%) and +96 B on x86_64 (+5.9%), **both identical to the figures recorded on 2026-09-05**, which
is a reproduction across a week of unrelated commits and a differently-shaped scaffold. riscv64 came
out at **+68 B (+3.7%)** here against +158 B then; the page-resident shape is the cheaper one on that
ISA, and the earlier figure is not wrong, it priced a different structure.

### One finding that is not about timers at all

**B1's data figures did not behave, and the reason belongs where somebody pricing the next addition
to `IpcTables` will find it.** The registry itself is arithmetic: a
`generational_table::Table<u64, 512>` is 512 `Option<u64>` (16 bytes each, no niche) plus 512 `u32`
generations plus two `usize`, so 10,256 bytes, and `IPC_TABLES` grew from 15,432 to 25,696 bytes on
**every** ISA, which is that plus the cached-earliest word. Expected.

**riscv64 then paid for it a second time.** In B0 that ISA's largest `.rodata` symbol is three bytes.
In B3 it carries a 25,680-byte anonymous `.rodata` aggregate, which is `EMPTY_TABLES` materialized as
a template to copy from rather than a struct initialized in place. aarch64 and x86_64 have no such
symbol in either build. So riscv64's data delta is **+37,061 bytes against aarch64's +10,980** for
identical source, and it is a **cliff rather than a slope**: the struct crossed a size the backend
treats differently, and the next field added to `IpcTables` may cost that ISA nothing or another
whole copy of the struct.

Nothing in this tree records that, and it is not specific to timers: it prices every future addition
to the scheduler's tables. It is left here rather than acted on because a lane does not choose
`MAX_TIMERS`, and because the right response may be to shrink the registry rather than to chase the
backend.

**And it makes `MAX_TIMERS` a real decision rather than a constant copied from `MAX_RENDEZVOUS`.** At
512 it is 10 KiB of kernel data, doubled on riscv64. At 64 (the 2026-09-05 scaffold's number) it is
1.3 KiB. This lane picked 512 to mirror the rendezvous registry and has no evidence that is right.

### The dependency on milestone 151, stated

The fourth shape signals **a notification**, and notification objects are
[§101](../design/decisions/101-notification-objects.md), decided 2026-08-20 and **unbuilt** (milestone
151). What the pricing above assumes about it:

1. That the signal target is a `Rendezvous`, not a separate object. The scaffold signalled through
   `sched::irq_notify`, which takes a `RendezvousId`, because that is what exists today. §101's whole
   argument is that a notification should be *its own object with its own queue*, separate from the
   endpoint. If 151 builds that, `Timer::ARM`'s second argument names a `Notification` and not a
   `Rendezvous`, and the dispatch arm's capability check changes shape. **The byte figures do not
   move much; the syscall's meaning does.**
2. That binding to a TCB is what makes the shape useful. Milestone 106's title is met only if a
   thread blocked in `RECEIVE` on an endpoint wakes on *either* a message or the deadline, and §101 says
   that is what TCB binding is for. Without 151 the fourth shape gives a thread a timer it can block
   on and no way to block on a timer *and* a message at once, which is milestone 106's actual
   complaint. **So the fourth shape is not independently useful: it is 151 plus one object.**
3. **That §101's carve-out still stands.** §101 anticipated a userspace timer process and named a
   kernel timed wait as the alternative for kernel needs. This spike's finding is that the userspace
   process cannot exist on riscv64, which moves the fourth shape from "an option nobody listed" to
   "the only one of the four that is buildable on all three architectures". That is a finding for
   calef, not a recommendation from this lane.

## BUGS

- The 2026-09-13 scaffolds were never run, not even under QEMU. The 2026-09-05 one was gated
  green with `script/test` before it was deleted; these three builds were only *compiled*, on all
  three ISAs and with `--features soak_test`. So every figure here is a size, and nothing on this page is
  evidence that a timer fires, that a sleeping kernel thread wakes, or that the reap sweep runs at
  the right moment. A byte count is the cheapest half of a pricing and it is the half that was
  bought. Running them is perhaps an hour and would turn "it compiles" into "it works".
- The kernel-side pricing measures the cheapest correct shape, which may not be the one wanted.
  `sleep_until` parks on no rendezvous with a sentinel, wakes directly, and returns nothing, so a
  kernel thread woken this way cannot tell a deadline from an abort. Every kernel consumer beyond a
  watchdog (§101 names a scheduling deadline and an in-kernel retransmit) plausibly wants more than
  that, and more than that was not priced.
- The reap sweep was priced, not proved. There is no test that destroys a region holding an
  armed timer, which is precisely the use-after-free the section above says the sweep exists to
  close. Whoever builds this owes that test before the sweep is believed.
- `MAX_TIMERS = 512` is this lane's guess and it drives the only figure here that is large.
  See the `IpcTables` finding above.
- Nothing here was run on hardware. The aarch64 refutation is a register specification plus this
  tree's own code, not an EL0 program that armed `CNTP_CVAL_EL0` and took the interrupt. The cheap
  version of that experiment (set `EL0PTEN`, have a user program write the comparator, see whether
  INTID 30 arrives) was not run, and it is the thing that would turn this section from a reading into
  a measurement. Under a hypervisor it is *expected* to fail, for the reason milestone 9 recorded.
- The Tegra X1 TRM was not read. NVIDIA's download redirects to a developer login. The argon row
  is mainline Linux's devicetree binding and DTS, which NVIDIA's own maintainers wrote, so the channel
  count and the fourteen SPIs are solid; anything register-level is not established here.
- The JH7110 timer has no mainline driver and this project has not driven it. Four channels with
  four PLIC lines is what the TRM says; nothing in this tree has touched the device.
- The Arm citation is a rendering of Arm's machine-readable system-register description, not the
  Architecture Reference Manual PDF. developer.arm.com's register pages render their content in
  JavaScript and returned no field text to a fetch; the rendering used is generated from Arm's own
  published XML and agrees with this tree's existing `EL0VCTEN`-is-bit-1 fact, but it is a secondary
  source and is marked as one.
- xenon's HPET is unconfirmed. See the x86_64 section. Reading it needs either the first-light
  photograph transcribed or the boot tour re-run with the table list captured, and neither was done
  here.
- The "one spare comparator" claim for aarch64 is about EL1/EL0 only. `CNTHP_*`, `CNTHV_*` and
  `CNTPS_*` exist at EL2 and EL3 and are not this kernel's to give.
- The scaffold is gone. The variant, the `abi::timer` module, the retype arm, the dispatch bodies,
  the expiry table and the tick hook were built to obtain the numbers and deleted. Rebuilding them is
  an hour; shipping them would have settled a syscall-surface fork by accident, which is what
  milestone 106's lane refused to do and what §10 (process) and §16 (revocation) reserve to an
  architect.
- **`Timer`, `Timer::ARM` and `Timer::CANCEL` are the milestone block's provisional coinages**, minted
  so the measurement could exist. Names are an architect's (§75 (provenance)).
