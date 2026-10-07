---
status: DECIDED
raised: 2026-09-02
decided: 2026-09-02
ratified_by: calef
---

# 139. Who may read the cycle counter, and by what authority

Answered by calef on 2026-09-02, in three parts, each after the evidence for it
was gathered rather than before.

1. The model: option 4, a per-thread grant enforced at the context switch. The read stays one
instruction with no syscall inside the measured operation, it is what Linux arm64 converged on for
the same reason, and it costs the compare that `switch_user_root` already pays for `TTBR0_EL1`.
Option 2 (a first-class capability object) was declined for now on the ground this tree already
applies elsewhere: it buys a new object type before there is a second consumer, and seL4's own
RFC-16 for that shape has been unmerged since 2024-02-02. New methods are allowed within the
established model (AGENTS.md), so option 2 remains reachable if a consumer appears.

2. The grant is a field in the spawn manifest, not a method on a live thread. calef asked
whether milestone 147 (a profiler that holds exactly the counters it was granted) argued for the
live form, since a consumer already exists; checking found it does not. 147 wants *this profiler may
read that subtree's counters*, which is cross-thread authority with a named target, and neither
shape here provides it. So the immutable grant costs nothing 147 needs, and it is the stronger
statement: a program cannot acquire a timing side channel it was not given at creation.
DECISIONS §28 chose the same shape for thread placement at ratification.

3. `x86_64` keeps its ambient counter, recorded as a position rather than left as an accident.
Three options were live and two were closed by measurement, both in this document. There is no
second user-readable clock on that architecture: Linux's own vDSO fast path cannot work without a
userspace TSC read, and it deletes the HPET mapping rather than offer it. Trap-and-emulate was
priced from Xen's "15 to 20 times slower" and measured at 1,667 ns, which is 4.1x the syscall it
was meant to beat, with an in-kernel minor page fault at 1,219 ns as the floor a nife handler could
aspire to.

The deciding cost is not the one this document first named. smoltcp would not notice a coarse clock:
`net_stack.rs`'s `instant()` divides to `Instant::from_millis` and discards the resolution. The
wall clock would notice. `components/src/ntp.rs` advances `local.now()` between syncs by
`monotonic_nanos().saturating_sub(self.mono0)`, and `monotonic_nanos` reads the cycle counter, so on
`x86_64` a tick-resolution page would make every timestamp between NTP syncs step in tick-sized
jumps, with NTP computing corrections against a clock coarser than the corrections. aarch64 and
riscv64 never meet this, because gating the cycle counter leaves them a generic timer.

What the grant buys, stated so nobody reads it as more. Two threads and a shared word
reconstruct a fine clock with no privileged instruction, measured at 6.8 ns of usable resolution on
cordoba and matching Schwarz et al. (FC 2017). That holds on all three architectures. So option 4
buys accountable authority, meaning the cheap accurate path is granted rather than ambient and
the kernel knows which threads hold it. It does not buy timing confinement, and this tree should not
claim it does.


**The number is provisional.** This lane could not see the other lanes running beside it, so 139 is
a claim on the next free slot rather than a mint; the integrator assigns the real one at merge, and
two lanes collided on a number the day before this was written. Cite it as
`design/decisions/139-cycle-counter-authority.md` until it lands.

Written 2026-09-02 by a research lane briefed to answer milestone 75
(who may read the cycle counter), which is `Gate: DECISION` and has been NOT-STARTED since
2026-08-03. It implements nothing. Milestone 75's own block is the question; this is the evidence and
the options, and the parts that are calef's are marked as his.

## What is being decided

May a program running at EL0 read a cycle counter, and if so by what authority? Three registers,
one question, three different answers today:

| | the fine counter | opened to EL0 by | state in this tree |
|---|---|---|---|
| aarch64 | `PMCCNTR_EL0` | `PMUSERENR_EL0.CR` or `.EN` | never written by this kernel |
| riscv64 | the `cycle` CSR | `scounteren.CY` (and `mcounteren.CY` in firmware) | never cleared by this kernel |
| `x86_64` | the TSC, via `rdtsc` | `CR4.TSD` clear | **open, and load-bearing** |

Milestone 74 (cycle counters) needs the answer before its
aarch64 half can land, and milestone 147 (a profiler that holds exactly the counters it was granted)
cannot be scoped at all until the grant unit exists.

## The premise was half false, and that is the most useful thing in this document

Milestone 75's block frames this as a decision about whether to open something that is closed.
Three checks say the framing is wrong in three different directions, and each one changes what the
decision has to cover.

### 1. On `x86_64` the counter is already open to every program, and nothing decided that

`kernel/src/arch/x86_64/boot.s:178` and `:385` are the only writes to `CR4` in the tree, and both
are `or eax, 1 << 5`, which is `PAE`. `TSD` is bit 2 and is never touched, so it holds its reset
value of clear, and ring 3 may execute `rdtsc`. `notes/x86-port/user-mode-runtime.md` states this in its own words and
is the record that it was noticed rather than overlooked:

> `now()` is `rdtsc`, and ring 3 may read it because `CR4.TSD` is clear at reset and this kernel does
> not change it. That is the same shape as aarch64 needing `CNTKCTL_EL1.EL0VCTEN` and RISC-V needing
> `scounteren.TM`, with the difference that here the permissive state is the default and the kernel
> would have to act to *close* it.
>
> -- notes/x86-port/user-mode-runtime.md

So one of the three supported architectures has already answered milestone 75 with option 1, by
inheritance. **A decision that says "closed unless granted" is not a decision to open something; on
`x86_64` it is a decision to close something programs already use.**

### 2. And closing it on `x86_64` would take the clock away, because there is only one register

`crates/user_rt`'s `now()` on `x86_64` is `rdtsc`. There is no coarse alternative on that
architecture the way `CNTVCT_EL0` is the coarse alternative on aarch64. So on `x86_64` the §10
clock exception (§10 says there is "no ambient authority", and notes/abi.md records the counter as
its one eyes-open exception) and this milestone's question are the same register. Setting
`CR4.TSD` today would break `Instant`, `thread::sleep`, the random seed, smoltcp's timestamps, and
the benchmark harness, all at once.

Checked, and it held: a research lane went looking for a second source on 2026-09-02 and found
none. The evidence is its own section below.

This is not an argument for leaving it open. It is the statement of what closing it costs, and the
shape of the fix already exists in this tree: §43 (reading the clock is a page) put the wall clock
in a page rather than a register, and a coarse monotonic value published in a page is the same move
one axis over. Nothing here proposes building that; it is named so the `x86_64` row is a decision
with a price rather than an exception with no plan.

### 3. On aarch64 and riscv64 the tree does not *establish* that the counter is closed, it assumes it

This is the finding worth acting on regardless of which option wins.

aarch64. Nothing in the tree writes `PMUSERENR_EL0`; the grep for it returns milestone 75's own
block, milestone 147's, and nothing else. Arm's register description says of every field in it,
`EN`, `CR`, `SW` and the rest:

> On a Warm reset, this field resets to an architecturally UNKNOWN value.

and of the cycle counter with `CR` and `EN` both 0, that EL0 reads "are disabled" and "generate an
exception to EL1, or to EL2 when EL2 is implemented and enabled for the current Security state".
Both quotations are from the `PMUSERENR_EL0` page of the Arm system-register reference at
`https://arm.jonpalmisc.com/latest_sysreg/AArch64-pmuserenr_el0`, read 2026-09-02. The two sentences
together say the trap is conditional on a value this kernel never sets.

Linux hit exactly this and fixed it by writing the register explicitly. Its commit
*"arm64: kernel: enforce pmuserenr_el0 initialization and restore"* (lkml.iu.edu archive
`1601.3/03556.html`, read 2026-09-02) says:

> The pmuserenr_el0 register value is architecturally UNKNOWN on reset.

and describes the exposure as platforms where "the pmu is not probed, therefore the pmuserenr_el0
register is not reset in the kernel, which means that its value retains the reset value that is
architecturally UNKNOWN".

Under QEMU this is almost certainly zero and the trap almost certainly fires. On argon, the
Jetson TX1 that milestone 127 (the seL4 machine) is about, it is whatever TF-A and the boot ROM
left, and nobody here has looked. This lane did not run a spike to find out; see BUGS.

riscv64. `kernel/src/arch/riscv64/timer.rs:182` opens the time CSR with
`csrs scounteren, TM`, a set of bit 1 and nothing else. The comment four lines above it says:

> CY (cycle) and IR (instret) stay closed.

Nothing clears them. They stay closed only if firmware left them clear, which is the identical
mistake that file's own comment records having found and fixed two paragraphs earlier: `user_rt`
documented U-mode `rdtime` as working "because the kernel sets scounteren.TM"; the kernel never set
it, and it worked on OpenSBI's default. The same sentence, about the same register, is now true of
`CY` and untrue of `TM` only because somebody went and looked. This is a claim stated in a comment
that the code does not establish, which is rung four wearing rung one's clothes.

What follows from all three. Part of milestone 75 is not a decision at all. Whatever authority
model wins, the kernel has to write these registers rather than inherit them, or the answer is
firmware's on every board. That part is a defect fix and this document recommends it outright.

### 4. And 74's aarch64 half is blocked on more than this

Milestone 127's other prerequisite, the EL2 to EL1 entry drop, is **PR #650 and is open, not
merged**, with `mergeStateStatus: DIRTY` as of 2026-09-02. Its diff does carry the `MDCR_EL2 = 0`
write and names `MDCR_EL2.TPM` as the trap that would otherwise catch every EL1 access to
`PMCCNTR_EL0`. So the EL2 half of the path exists and is real, and it is one merge away rather than
landed. Nothing in this document depends on that PR, but a plan that assumed it had landed would be
a day early.

## Checked: does `x86_64` really have only one user-readable clock?

Finding 2 above was written from knowledge rather than from reading, and the maintainer said so when
he presented it. A research lane was briefed on 2026-09-02 to check it. The claim holds. There is
no second user-readable time source on `x86_64` of resolution meaningfully better than a kernel
tick, and the ISA routes every user-visible one through the same `CR4.TSD` bit on purpose. What
follows is what was read, in the order a reader would want to check it.

### The instruction set: one bit closes three instructions, not one

`CR4.TSD` is not the gate on `rdtsc` alone. Intel's instruction reference gives the same `#GP(0)`
condition on three user-mode instructions, and the third is the one that would otherwise be a
loophole. All three quotations are from the Intel SDM instruction pages as reproduced at
`felixcloutier.com/x86/`, read 2026-09-02.

- `RDTSC`: "When the flag is clear, the RDTSC instruction can be executed at any privilege level;
  when the flag is set, the instruction can only be executed at privilege level 0", with
  "#GP(0) - If the TSD flag in register CR4 is set and the CPL is greater than 0."
- `RDTSCP`: the same sentence and the same `#GP(0)`.
- `TPAUSE`: "The instruction execution wakes up when the time-stamp counter reaches or exceeds the
  implicit EDX:EAX 64-bit input value", and its exception list reads
  "#GP(0) If src[31:1] != 0. If CR4.TSD = 1 and CPL != 0."

`TPAUSE` is worth naming because it is the attack a reader should think of and Intel already closed
it. It consumes a TSC *deadline* rather than returning a TSC value, and it reports through the carry
flag, so a program that could execute it while `rdtsc` was shut would recover the counter to full
precision by binary search over deadlines in about sixty-four executions. Intel put it behind
`CR4.TSD` anyway. That is a design statement: the bit is meant to close *user-visible TSC reads*,
not one opcode.

Two further instructions come up and neither is a time source.

- `RDPID` reads "the value of the IA32_TSC_AUX MSR (address C0000103H) into the destination
  register". That is a processor identifier, not a counter, so it does not answer this question at
  any resolution.
- `RDMSR`, which would reach the TSC MSR and every APIC register in x2APIC mode, "must be executed
  at privilege level 0 or in real-address mode; otherwise, a general protection exception #GP(0)
  will be generated."

`RDPMC` is the one real second door, and it is a door to close rather than a clock to keep.
Its gate is a *different* `CR4` bit: "When the PCE flag is set, the RDPMC instruction can be executed
at any privilege level; when the flag is clear, the instruction can only be executed at privilege
level 0", with "#GP(0) If the current privilege level is not 0 and the PCE flag in the CR4 register
is clear." Fixed-function counter 2 counts at the TSC rate, so ring 3 with `CR4.PCE` set and the
fixed counters enabled would have a TSC-rate clock without ever executing `rdtsc`. That is not an
alternative for `user_rt::now()`, because it needs the kernel to enable the counters through MSRs
this kernel never writes, and `kernel/src/arch/x86_64/` contains no reference to
`IA32_PERF_GLOBAL_CTRL` or the fixed counters at all. It is a finding for the *recommended outright*
list: `CR4.PCE` must be established as clear by the same code that establishes `CR4.TSD`, or a
future decision to close the TSC would be closing the front door while the side door is only shut by
a reset value nobody wrote. `boot.s`'s two `CR4` writes are still only `or eax, 1 << 5`.

That finding has a home: milestone 228 (the cycle counters are closed by assumption) was minted
on 2026-09-02 to do exactly the "close what we claim is closed" work this document recommends, and
its x86 item is scoped to leaving `CR4.TSD` alone and writing the record down. It does not mention
`CR4.PCE`, because nobody had looked. **`RDPMC` is a second user-readable path to a TSC-rate count on
`x86_64`, gated by a different bit that this kernel also never writes**, and establishing it clear
belongs in that milestone beside the aarch64 and riscv64 writes rather than in a lane report.

### The chipset timers: two exist, and neither is usable from ring 3 at a price worth paying

The ACPI power management timer. ACPI 6.5, section 4.8.2.1, read 2026-09-02 at
`uefi.org/specs/ACPI/6.5/04_ACPI_Hardware_Specification.html`:

> The power management timer is a 24-bit or 32-bit fixed rate free running count-up timer that runs
> off a 3.579545 MHz clock.

That is 279.4 ns per tick, which is genuinely finer than a tick and finer than nothing. The problem
is where it lives. The register is `PM_TMR_BLK`, which the FADT normally places in system I/O space,
and an `in` from ring 3 needs either `IOPL >= CPL` or a cleared bit in the TSS I/O permission bitmap:
"#GP(0): If the CPL is greater than (has less privilege) the I/O privilege level (IOPL) and any of
the corresponding I/O permission bits in TSS for the I/O port being accessed is 1." Granting a
confined program port access is a strictly larger authority than granting it a counter read, and it
is per-task state on the context switch exactly like option 4, with a worse blast radius. This is
the reverse of what the decision wants.

The HPET. This is the strongest candidate on the list, and it is the one that has already been
tried and withdrawn. It is memory mapped, so a kernel *can* map its page read-only into a program's
address space with no new instruction and no new port authority. The IA-PC HPET Specification 1.0a
(Intel, 2004), section 2.2's recommendation table, gives "Clock Frequency Fmin = 10 MHz", and the
General Capabilities register's `COUNTER_CLK_PERIOD` field "indicates the period at which the counter
increments in femptoseconds (10^-15 seconds) ... The value in this field must be less than or equal
to 05F5E100h (10^8 femptoseconds = 100 nanoseconds)". So the architectural floor is 100 ns per tick
and the common part runs at 14.31818 MHz, about 70 ns.

Linux mapped the HPET into userspace through the vDSO and then deleted the capability. Commit
`1ed95e52d902035e39a715ff3a314a893a96e5b7`, "x86/vdso: Remove direct HPET access through the vDSO",
Andy Lutomirski, authored 2016-04-08, reviewed by Thomas Gleixner, read 2026-09-02 via the GitHub
commit API on `torvalds/linux`:

> Allowing user code to map the HPET is problematic.  HPET
> implementations are notoriously buggy, and there are probably many
> machines on which even MMIO reads from bogus HPET addresses are
> problematic.

and, on the point that decides it here:

> The vclock HPET code has also always been a questionable speedup.
> Accessing an HPET is exceedingly slow (on the order of several
> microseconds), so the added overhead in requiring a syscall to read
> the HPET is a small fraction of the total code of accessing it.

A time source whose read costs several microseconds is slower than asking the kernel. Measured
on cordoba (x86_64, Linux, four cores, load average 3.6) with a throwaway spike on 2026-09-02: a
`clock_gettime(CLOCK_MONOTONIC)` through the vDSO costs 28.4 ns, and the same call forced through
`syscall(SYS_clock_gettime, ...)` costs 596.9 ns. So an HPET read at "several microseconds" is
several times *more* expensive than the syscall that a closed TSC would force. It is not a fallback;
it is a worse version of the thing it would be a fallback from. The spike is not shipped.

The HPET is also not free to reach: `crates/machine_discovery/src/acpi.rs` sees the `HPET` table in
the XSDT walk and does nothing with it, so this route costs table parsing, an MMIO mapping, and a way
to hand that mapping to a program, before it buys a clock that loses to a syscall.

The local APIC. Not a candidate, for three independent reasons. In x2APIC mode, which is what
this era's machines and this era's kernels use (cordoba's `/proc/cpuinfo` reports the `x2apic`
flag), there is no page to map at all. The Intel 64 Architecture x2APIC Specification, reference
number 318148, read 2026-09-02:

> To enhance inter-processor and self directed interrupt delivery as well as the
> ability to virtualize the local APIC, the APIC register set can be accessed only
> through MSR based interfaces in the x2APIC mode. The Memory Mapped IO
> (MMIO) interface used by xAPIC is not supported in the x2APIC mode.

and MSR access is ring 0 per the `RDMSR` quotation above. Even in xAPIC mode the page at
`0xFEE00000` is a CPU-local alias, so a migrated thread would silently read a different core's APIC,
and the register in question is the kernel's own timer, a down-counter reloaded on every tick rather
than a monotonic count. Three ways wrong, and the first one is fatal on its own.

### What Linux's vDSO fast path actually does, since this is the category most likely to change the answer

It reads the TSC from userspace, in every mode it has. This was the specific thing the brief
asked to be determined rather than recalled, so it was read from the current tree of
`torvalds/linux`, fetched 2026-09-02.

`arch/x86/include/asm/vdso/clocksource.h` is the whole list of modes:

> #define VDSO_ARCH_CLOCKMODES	\
> 	VDSO_CLOCKMODE_TSC,	\
> 	VDSO_CLOCKMODE_PVCLOCK,	\
> 	VDSO_CLOCKMODE_HVCLOCK

and `arch/x86/include/asm/vdso/gettimeofday.h` dispatches on it:

> 	if (likely(clock_mode == VDSO_CLOCKMODE_TSC))
> 		return (u64)rdtsc_ordered() & S64_MAX;

The other two are the paravirtual ones, and both of them are a *shared page plus a TSC read*, not a
substitute for one. `vread_pvclock()` in the same file ends its seqlock loop with

> 		ret = __pvclock_read_cycles(pvti, rdtsc_ordered());

and `__pvclock_read_cycles` in `arch/x86/include/asm/pvclock.h` is

> 	u64 delta = tsc - src->tsc_timestamp;
> 	u64 offset = pvclock_scale_delta(delta, src->tsc_to_system_mul,
> 					     src->tsc_shift);
> 	return src->system_time + offset;

The Hyper-V path is the same shape: `vread_hvclock()` calls `hv_read_tsc_page_tsc()`, whose comment
in `include/clocksource/hyperv_timer.h` states the protocol as
"ReferenceTime = ((RDTSC() * ReferenceTscScale) >> 64) + ReferenceTscOffset". So the shared page
carries *scale and offset*, and the counter still comes from the guest's own `rdtsc`. **The page is
how the guest learns to interpret the TSC, not a way to avoid reading it.**

And when the TSC is unusable, the fast path is simply gone. `.vdso_clock_mode` is set only by the TSC
clocksources: `arch/x86/kernel/tsc.c` sets `.vdso_clock_mode = VDSO_CLOCKMODE_TSC` on both
`clocksource_tsc_early` and `clocksource_tsc`, while `clocksource_hpet` in `arch/x86/kernel/hpet.c`
and `clocksource_acpi_pm` in `drivers/clocksource/acpi_pm.c` set no such field, which leaves it at
`VDSO_CLOCKMODE_NONE`, the zero value of the enum in `include/vdso/clocksource.h`. A zero mode falls
through `__arch_get_hw_counter` to `return U64_MAX;` and the caller takes the syscall path.

**So the answer to the question the brief singled out is no: Linux's x86 vDSO fast path cannot work
without a userspace TSC read, and Linux does not pretend otherwise.** Boot an x86 Linux with
`clocksource=hpet` and userspace loses the fast path entirely; it does not get a slower fast path.
That is the strongest single piece of evidence for finding 2, because it is the largest x86 userspace
in existence declining to build the thing this section went looking for.

### What a virtualized guest does when the host denies the TSC

The same problem under a different name, and the answer is that nobody substitutes a different
clock; the read traps and is emulated, and the guest still gets a TSC. Xen documents this most
plainly of the hypervisors. `xen-tscmode(7)`, Dan Magenheimer, read 2026-09-02 at
`xenbits.xen.org/docs/4.13-testing/man/xen-tscmode.7.html`:

> This trap can be detected by Xen, which can then transparently "emulate" the results of the rdtsc
> instruction and return control to the code following the rdtsc instruction.

> TSC emulation is relatively slow -- roughly 15-20 times slower than the rdtsc instruction when
> executed natively. However, except when an OS or application uses the rdtsc instruction at a high
> frequency (e.g. more than about 10,000 times per second per processor), this performance
> degradation is not noticeable (i.e. <0.3%). And, TSC emulation is nearly always faster than
> OS-provided alternatives (e.g. Linux's gettimeofday).

Two things fall out of that and both are useful here. The industry's answer to "this program may not
read the raw counter" is trap and emulate, which keeps the resolution and pays roughly an order
of magnitude in latency, and it is a fourth option for the `x86_64` row that neither milestone 75's
block nor this document had named. And Xen's own measured judgment is that even a trapping TSC beats
the OS-provided alternative, which is the same conclusion the HPET evidence reached from the other
direction.

### Measured: what trap-and-emulate actually costs, and it is worse than the syscall it was meant to beat

Xen's "15-20 times slower than the rdtsc instruction when executed natively" is Xen's number on
Xen's path, and the section above quotes it because it is the industry's answer rather than because
it is ours. A nife trap would not be a VM exit on hardware built to make VM exits cheap. It would be
`#GP` to the IDT, a handler, a read of the faulting instruction out of user memory, a decode, an
emulate, an `RIP` advance and an `iret`. calef asked for our number instead of Xen's, so a research
lane measured it on 2026-09-02. It came back at 1,667 ns, which is four times the syscall this
option existed to avoid.

#### The machine and the method

cordoba: Intel Core i5-4670 (Haswell, 4 cores, no SMT, 3.4 GHz nominal, TSC rate measured at
3.3984 GHz and invariant per `constant_tsc`), Ubuntu, `Linux cordoba 7.0.0-30-generic`, PTI and
retpolines both on per `/sys/devices/system/cpu/vulnerabilities/`. The machine is the family's live
backup server and carried a load average of about 3.8 throughout, which is stated because it is the
main reason to distrust the absolute figures and not the ratios.

A throwaway C program, pinned to one CPU, times five things over 200,000 iterations each, eleven
repetitions, median reported. Linux can set `CR4.TSD` for one task: `prctl(PR_SET_TSC,
PR_TSC_SIGSEGV)` makes `rdtsc` fault. The program installs a `SIGSEGV` handler, checks that the two
bytes at the faulting `RIP` are `0F 31`, writes a value into the saved `RAX` and `RDX`, advances the
saved `RIP` by two, and returns. That is a real trap, decode, emulate and resume round trip on real
silicon. It counted 2,200,000 traps against 2,200,000 expected, so every read in the loop went
through the handler and none was skipped. The timing calls sit outside the window where `TSD` is
set, so the vDSO's own `rdtsc` never faults into the same handler and never contaminates the result.

| | median | across three runs | in TSC cycles | against native |
|---|---|---|---|---|
| native `rdtsc` | 10.0 ns | 9.3, 10.0, 30.4 | 34 | 1x |
| `clock_gettime` via the vDSO | 24.3 ns | 22.4, 24.3, 24.9 | 82 | 2.4x |
| `syscall(SYS_getpid)`, a bare ring transition | 370 ns | 370.3, 370.3, 373.3 | 1,258 | 37x |
| `clock_gettime` forced through `syscall()` | 406 ns | 407.5, 405.9, 415.1 | 1,379 | 41x |
| a minor page fault, handled in-kernel, no signal | 1,219 ns | 1,227, 1,212, 1,219 | 4,143 | 122x |
| **`rdtsc` trapped and emulated** | **1,667 ns** | 1,653.6, 1,667.5, 1,668.2 | **5,665** | **166x** |

Run-to-run variance on the trapped figure is under 1%, and within a run the eleven repetitions span
1,638 to 1,692 ns. This is not a noisy measurement; the one visibly load-perturbed row is native
`rdtsc`, whose third run of 30.4 ns is an outlier against a min of 20.2 in that run and 9.25 in the
first.

#### Two reference points in the brief did not reproduce, and one of them cannot be right

This lane was pointed at a native `rdtsc` of 0.25 ns and a `clock_gettime` syscall of
596.9 ns, both measured on cordoba earlier in the same session. Neither reproduced, and the
first is worth flagging rather than averaging away: 0.25 ns is under one core cycle at this
machine's 3.4 GHz, and `rdtsc` on Haswell has a documented latency in the low tens of cycles. No
single `rdtsc` can retire in a quarter of a nanosecond. The 10.0 ns measured here is 34 TSC cycles
and is what a back-to-back `rdtsc` throughput loop should look like on this part. The 0.25 ns figure
is most likely a loop the compiler hoisted or an amortization over something other than the reads.
The syscall figure is the same order as the 406 ns measured here and the difference is plausibly
load and frequency, so nothing turns on it.

The ratios are the durable quantity and they do not depend on which absolute number is right:
the trap costs 4.1x a `clock_gettime` syscall and 4.5x a bare ring transition, measured in
the same program on the same core within the same second.

#### What this is an upper bound on, and what it is a lower bound on

Upper bound, and it is a generous one. Linux delivers this as a signal. The path is fault into
the kernel, build a signal frame on the user stack, return to ring 3 at the handler, run the
handler, `rt_sigreturn` back into the kernel, restore. That is two ring round trips plus frame
construction and teardown, where a nife `#GP` arm would be one. A nife handler would also not be
running with PTI's address-space switch on every transition. So 1,667 ns is not what nife would pay.

Lower bound, and this is the row that matters. The minor page fault at 1,219 ns is an
in-kernel fault handler with no signal frame at all: `#PF` to the IDT, Linux allocates and zeroes a
page, updates the page tables, shoots the TLB entry, `iret`. One ring round trip, on this silicon,
with these mitigations. It does considerably more work than an `rdtsc` emulate would, so it is not a
substitute for measuring the real thing, and the method here cannot separate the entry and exit cost
from the page work. What it does establish is that **a fault round trip on this machine is in the
same neighborhood as a syscall and above it, not below it**, which is the architectural fact the
option was implicitly betting against. `SYSCALL`/`SYSRET` is the path Intel built to be fast; an
IDT-vectored exception returning through `iret`, a serializing instruction, is the path it did not.

Put together, a nife trap-and-emulate would land somewhere between the 406 ns syscall and the
1,667 ns measured here, and it would land there because it is a ring round trip plus extra work.
It cannot be cheaper than a syscall on the same machine, because a syscall is a ring round trip and
nothing more. Xen escapes this only because its comparison is against a guest syscall into a guest
kernel that then does more work still, on parts with hardware-accelerated exits. Our comparison is
against our own syscall, and that is a fight trap-and-emulate cannot win.

#### The two thresholds the brief named, answered

The brief set them out in advance so the number could not be read after the fact. "Near 150 ns"
would have meant the performance objection evaporates. "Near or above 1 microsecond" would have
meant the option is worse than the syscall it was meant to beat. The measurement is 1,667 ns on
the heaviest honest path and 1,219 ns for the lightest in-kernel analogue available. Both are above
the microsecond. Trap-and-emulate fails on its own terms, and it fails on the one axis it was
proposed to win on.

#### How often `now()` is actually called, since the brief asked

Every ungranted `now()` would pay this, so the count is part of the price. Measured by grep in the
merged tree on 2026-09-02: 42 direct call sites of `user_rt::now`, plus 18 of
`user_rt::monotonic_nanos`, which is `now()` with a divide. The `Instant::now()` sites in
`crates/credentialer` and `crates/board_console` reach the same counter through `std`.

Most of them are `let start = now(); ...; now().wrapping_sub(start)` in benchmark and test code and
would not care. Four shapes would.

1. `components/src/net_stack.rs:97`, smoltcp's clock. `service_until` at `:437` calls `instant()`
   twice per iteration of its poll loop, once to poll and once to check its own 15 second bound, and
   there are eight `iface.poll(instant(), ...)` sites in that file. Every packet the network stack
   services costs at least two counter reads, and at 1.7 microseconds each that is a per-packet tax
   on the one server in this tree with a throughput number.
2. `components/src/watch.rs:171`, `while monotonic_nanos() < deadline { yield_now(); }`. A spin-yield
   standing in for the timed wait this kernel does not have, one counter read per iteration.
3. `components/src/ntp.rs:382`, the retry gap, the identical shape.
4. `fixtures/src/login_test_client.rs:573` to `:580`, a polling loop reading the counter up to three
   times per iteration.

The spin loops are the least alarming of the four despite looking the worst, because a loop whose
job is to wait does not mind that each iteration got slower. The network stack is the real one: it
is not waiting, it is working, and the counter read is on the path of every poll.

#### What the number does not settle

It disposes of the performance argument and nothing else. The other three objections stand exactly
where they were and should not be read as answered by a measurement that never touched them.

- **A `#GP` handler that decodes instructions is a category this kernel has not entered.** Reading
  the faulting instruction out of user memory and interpreting it is a new kind of thing for this
  kernel to do, with a new fault surface of its own (an unmapped page under `RIP`, a decode that
  must be exactly right about the two-byte opcode and about `rdtscp` and `rdpmc` being separate
  instructions the same `CR4` bit closes).
- **The x86 port has never booted on metal.** Nothing here was measured on xenon, the Dell OptiPlex,
  which does not exist as a running nife host. Every figure in this section is Linux on cordoba,
  which prices the mechanisms and not the port.
- It says nothing about whether the counter should be closed on `x86_64` at all. That is the
  confinement position, it is still calef's, and the argument for leaving `rdtsc` ambient there was
  never a performance argument.

#### This was a spike and it was thrown away

Two C programs on cordoba, in `/tmp`, and nothing else. No `CR4.TSD` change, no fault handler and
no decode was written for nife, on any branch. The nife-side spike suggested in the brief was not
run: an x86 nife boot is young enough that
milestone 165 (x86_64 PCI enumeration: wire `kernel/src/pci.rs` to ACPI's MCFG) found on the same
day that its ACPI walk was bounded at 1 GiB, so no machine with real RAM could find an ACPI table, and QEMU could have shown the
instruction path but not a cycle count, which is the part that decides this. The Linux measurement
answers the question at both ends of the bracket without it.

#### What this does to the `x86_64` row

It removes an option rather than choosing one. The recommendation above is unchanged and this
lane did not change it: it names trap-and-emulate and a tick-resolution page as the two realistic
orders for the x86 row, and says `x86_64` keeping `rdtsc` ambient with a recorded reason remains the
right state until one is chosen. What is new is that the first of those two is now priced and
loses. A published page at tick resolution, which is DECISIONS §43 (reading the clock is a page)
one axis over, is the only remaining candidate for giving `x86_64` a second time source, and it is a
coarse clock and cannot be anything else, for the arithmetic reason three sections up.

So the honest reading is that the `x86_64` row's realistic choice narrowed from three to two:
ambient `rdtsc` with a recorded confinement position, or a coarse published page plus a closed TSC.
Trap-and-emulate is off the list on the evidence rather than on taste.

### The thing that does defeat a closed counter, and it defeats it on all three architectures

A program with two threads and a shared page does not need a counter instruction. One thread
increments a word in a loop; the other reads it. Schwarz, Maurice, Gruss and Mangard, *Fantastic
Timers and Where to Find Them: High-Resolution Microarchitectural Attacks in JavaScript*, FC 2017,
read 2026-09-02 at `gruss.cc/files/fantastictimers.pdf`, built exactly this inside a browser:

> We implemented a clock with a parallel counting thread using the SharedArrayBuffer. An
> implementation is shown in Listing A. . The resulting resolution is close to the resolution of the
> native timestamp counter. On our Intel Core i test machine, we achieve a resolution of up to 2 ns
> using the shared array buffer.

Reproduced on cordoba in the same throwaway spike, in C, with no privileged instruction of any kind:
1.692 ns per increment and a smallest observed step of 4 increments, so 6.8 ns of usable resolution
from two ordinary threads, on a machine under load average 3.6.

This does not change the recommendation and should not be read as an argument against option 4. It
changes what option 4 is *for*, and the document already says the true thing in its fatal-risk
section: nife makes no timing-isolation claim, and a per-thread counter grant buys comparable
measurement and accountable authority, not confinement against a program that wants to measure
time. Anything that can spawn a second thread and share memory with it reconstructs a nanosecond
clock, on aarch64 and riscv64 as much as on `x86_64`. The row that section already asks for in
`notes/confinement-claims.md` should say this, since it is the concrete reason the claim is not made.

### What this makes the honest scope note for the `x86_64` row

Not "x86 is different", which explains nothing and reads as an excuse. The reason is specific and
falsifiable, and it has three clauses:

1. On `x86_64` the coarse clock and the fine counter are the same register, so `CR4.TSD` is not
   an analogue of `PMUSERENR_EL0.CR`. It is an analogue of `PMUSERENR_EL0.CR` *and*
   `CNTKCTL_EL1.EL0VCTEN` at once. aarch64 and riscv64 can close the fine one because they have a
   second, coarser, architecturally guaranteed user-readable clock; x86 has no second one at all.
2. The alternatives exist and all of them lose to a syscall. The HPET is mappable and costs
   several microseconds a read, which Linux measured and acted on in 2016. The ACPI PM timer is
   279 ns but lives behind an I/O port grant that is a larger authority than the thing being denied.
   The APIC is not addressable from ring 3 in x2APIC mode. So closing the TSC on `x86_64` does not
   demote userspace from 0.25 ns to 70 ns; it demotes userspace to a syscall, or to a published page
   at tick resolution, which is DECISIONS §43 (reading the clock is a page) one axis over.
3. A published page cannot close the gap by being updated more often, because its update rate is
   the interrupt rate. Getting from a 10 ms tick to a microsecond costs ten thousand interrupts a
   second, per core, forever. This is arithmetic rather than a measurement, and it is why the page is
   a coarse-clock answer and not a fine-clock one.

The consequence for the plan is the one this document already states in option 4 and is now
established rather than asserted: option 4 on `x86_64` is blocked behind giving that architecture a
second time source, and there is no cheap one to give. The realistic order for the x86 row is
therefore a tick-resolution page, and it is a decision with a price rather than a gap in §19
(architectural parity) that a lane can close by trying harder. Trap-and-emulate was the other
candidate and it has since been measured and lost: this paragraph originally priced it at "roughly
the syscall", and the section above measures it at 1,667 ns on cordoba, four times the syscall it
was meant to beat. `x86_64` keeping `rdtsc` ambient, with a recorded
reason, remains the right state until one of those is chosen. That is a published confinement
position and it is still calef's.

### BUGS in this section

- **The clocksource comparison was not measured**, only read. Switching cordoba's
  `current_clocksource` to `hpet` and to `acpi_pm` would have priced Lutomirski's "several
  microseconds" directly; it needs root and passwordless `sudo` is not configured there, and the
  machine is the family's live backup server. The 28.4 ns and 596.9 ns figures were measured; the
  HPET read cost is Linux's number, not ours.
- Nothing was measured on xenon, the x86 target machine, which does not exist as a running nife
  host yet. Every x86 measurement here is Linux on cordoba, which prices the mechanisms and not the
  port.
- **`TPAUSE`'s binary-search argument is reasoned from the instruction's specification, not
  demonstrated.** It does not need to be demonstrated for the conclusion, since Intel gates the
  instruction on `CR4.TSD` either way, but it is reasoning rather than observation and is marked so.
- **The trap-and-emulate measurement is Linux's signal path, not a kernel-internal fault handler.**
  1,667 ns includes building and tearing down a signal frame and two ring round trips where a nife
  `#GP` arm would have one. The minor-page-fault row at 1,219 ns is the closest in-kernel analogue
  available and it does more work than an `rdtsc` emulate, so the method brackets a nife trap
  between the 406 ns syscall and the 1,667 ns measured, and does not pin it. Pinning it needs a nife
  spike, which was not run.
- **The native `rdtsc` and syscall reference points from earlier in the same session did not
  reproduce**, and the 0.25 ns one is arithmetically impossible for a single `rdtsc` on a 3.4 GHz
  part. The figures in the measurement section are this lane's own and were taken in one program on
  one core; the earlier ones are left in the paragraphs that cite them rather than silently
  rewritten, and this note is what reconciles them.
- **The `now()` census is a grep, not a profile.** 42 direct `user_rt::now` sites and 18
  `monotonic_nanos` sites is a count of source locations. Nobody has counted dynamic calls per second
  on a busy path, so "two counter reads per network poll" is read off the code and not observed.
- **AMD's APM was not read.** All ISA quotations are Intel's. `CR4.TSD` and `CR4.PCE` are
  architectural on both vendors and this is not expected to differ, but it was not checked, and
  xenon is an Intel machine so nothing here has been checked against an AMD part.

## What this tree already does in the analogous case

Four analogues, and they do not all point the same way, which is why this needed reading rather than
recalling.

- The generic timer is ambient, deliberately, and the record says why. notes/abi.md calls it "the
  one ambient thing" and defends it: "A monotonic counter grants no authority to *affect* anything,
  only to observe the passage of time". `crates/uptime` inherits it and its module docs make the
  point that the program "needed no manifest field, no new capability, and no wiring".
- The wall clock is a capability, expressed in objects the kernel already had. §43 gives read as
  a read-only page, set as a writable page, and propose as an endpoint, with "No new syscall, no
  new method number, no new object type". That is the shape a cheap answer here would want to
  copy: an authority expressed in existing objects rather than a new type.
- Entropy and the clock are both services, reached by capability, so "a program that needs a
  privileged read asks a service" is the tree's normal case, not an exotic one.
- `CNTKCTL_EL1.EL0VCTEN` and `scounteren.TM` are per-machine bits, set once at init. There is no
  precedent in this tree for a per-thread system-register bit maintained across a context switch.
  That is the one piece of machinery option 4 below needs and the tree does not have.

## Prior art, read rather than recalled

seL4, which is the one that matters, has both answers and ships the weaker one. Its build option
`KernelArmExportPMUUser` is documented on `docs.sel4.systems/projects/sel4/configurations.html`
(read 2026-09-02) as:

> Grant user access to the performance monitoring unit. While useful for benchmarking, this option
> opens the possibility of timing channels.

It defaults off, and the same page records that `KernelVerificationBuild` excludes options of this
kind. So **seL4's published 413/426-cycle numbers are produced by a configuration seL4 does not
verify and does not recommend for production**, which is worth knowing before treating them as the
standard to match.

**seL4's own community has proposed exactly option 2 and has not landed it.** RFC-16, *"New
capability for the PMU"*, Krishnan Winter, proposed 2024-02-02, is `seL4/rfcs` PR #22, still open,
file `src/proposed/0160-pmu.md`. Read in full 2026-09-02. It says:

> Present profiling support uses the PMU through an ad-hoc interface that is designed for debugging
> and is consequently only available in a specific benchmarking configuration of the kernel. The
> same interface cannot be used in a production system as it is inherently insecure.

and

> Obviously the PMU presents a covert channel that exposes information about execution of user-level
> components (as well as the kernel). Therefore, PMU access needs to be explicitly authorised, which
> means we need an access-control model for the PMU.

and, on the current ARM situation:

> Additionally, on ARM systems, the only way to get access to the PMU from user-space is to
> configure the kernel to export access to the PMU registers, making the PMU an uncontrolled
> resource.

Its shape is a new object `seL4_PMU` with badged capabilities, the badge naming which counters
are authorized, and a blocking invocation. Its own unresolved questions include "How will the PMU
object affect verification? Initially it will not be available in verification builds of seL4".

Linux has both answers too, and the arm64 one is the interesting half. The global answer is
`perf_event_paranoid`, documented at `kernel.org/doc/html/latest/admin-guide/sysctl/kernel.html`
(read 2026-09-02) as controlling "use of the performance events system by unprivileged users
(without CAP_PERFMON)", default 2, with `-1` allowing "(almost) all events by all users". It is a
global sysctl, not a per-target grant, which is the criticism milestone 147 already makes of it.

The arm64 half is closer to what this decision needs. The commit *"arm64: perf: Enable PMU counter
userspace access for perf event"* (lkml.rescloud.iu.edu archive `2105.2/02527.html`, read
2026-09-02) enables `PMUSERENR_EL0`'s `ER` and `CR` bits per task, on the context-switch hook,
and states its reason:

> Only support user access when explicitly requested on open and only for a thread bound events.
> This avoids some of the information leaks x86 has and simplifies the implementation.

Two things fall out of that sentence and both bear on this decision. Per-thread, opt-in, maintained
at context switch is the mainstream modern answer, not an exotic one. And the "information leaks
x86 has" that Linux is avoiding are the consequence of the always-on `rdtsc` that this tree has
inherited on `x86_64` by the same default.

**L4Re: I could not source this.** The searches returned a virtualization paper and secondary
summaries rather than an L4Re or Fiasco.OC authority on PMU access control. Recorded as not
established rather than paraphrased.

## The options, with what each costs

Milestone 75's block names three. There is a fourth, it is the one the prior art converged on, and it
did not exist in the block.

### Option 1: ambient, like the generic timer

Set `PMUSERENR_EL0.CR` once at init, set `scounteren.CY`, leave `CR4.TSD` clear.

- Cost to build: aarch64 one `msr` in `timer::init` or `cpu` init; riscv64 one more bit in the
  existing `csrs`; `x86_64` nothing at all, since it is the state today. Call it three instructions.
- Cost to the claim: it spends §10's exception a second time on an instrument roughly 160x finer
  (0.25 ns against 41 ns), and it is the configuration seL4 declines to verify and declines to ship
  on by default. It is also the one that cannot be walked back: an ambient opening becomes something
  programs depend on, which milestone 75's own scope note names as the worst outcome.
- What it is honest about: it is what we already do on `x86_64`, so choosing it makes the tree
  consistent rather than making it worse.

### Option 2: a first-class capability object

A PMU object, a grant in the spawn path, a checked invocation. seL4's RFC-16 shape.

- Cost to build: a new object type, a new method number, spawn-path wiring, `caps` output, and
  Kani reach. Nothing in this tree prices at a morning. Milestone 147 says the counter-set and
  target-naming parts have "no precedent in this tree to price from".
- Cost at the measurement: this is the one that decides it. If the read is an invocation, the
  measured operation now contains a syscall, which is option 3's defect arriving through a different
  door. It is only free if the capability's *effect* is to open the register, at which point the
  capability is a grant of option 4 and the object is bookkeeping around it.
- Cost to reverse: highest on the list. A new object type and method number is the syscall
  surface, which §10 and §16 put in the expensive category, and milestone 147 would build on it.

### Option 3: kernel-mediated

EL0 asks the kernel to time an operation; the register never opens.

- Refused, and 75 already refused it, correctly: the measurement then contains the syscall it is
  trying to measure. Recorded so it stays visibly rejected.
- **One thing it is right for, which 75 does not say.** On riscv64 the SBI PMU route (`EID 0x504D55`)
  is inherently this shape: SBI calls are made from S-mode, so a U-mode program cannot make one and
  the kernel is in the path by construction. So RISC-V's cheap-read story is the `cycle` CSR and
  `scounteren.CY`, not SBI, and milestone 74's RISC-V half will want both for different jobs.

### Option 4: a per-thread grant, enforced at the context switch

The thread that was granted it runs with the counter open; every other thread runs with it closed.
The kernel writes the enable on the switch, the same way it writes the address-space root.

- Cost at the measurement: zero. The read stays one `mrs`, no syscall, no trap. It is the same
  instrument seL4's published numbers were taken with, which is what comparability requires.
- Cost on the context-switch path: one comparison, and one `msr` only when the value changes.
  That is exactly the shape `kernel/src/arch/aarch64/mmu.rs`'s `switch_user_root` already has (it
  early-returns when `TTBR0_EL1` already holds the wanted value), called from `sched.rs:1870`. If no
  thread is granted, the value never changes and the whole cost is a compare.
- Cost to build: a bit on the TCB, a write at the switch site on three architectures, and a way
  to set the bit. The last part is the expensive one: setting it is a syscall-surface change, either
  a field on TCB configure or a new spawn-path input, and that is calef's rather than a lane's.
- The `x86_64` asymmetry survives this option and has to be decided separately. `CR4.TSD` is
  writable per switch too, but closing it for ungranted threads removes `user_rt::now()` from every
  x86 program, so option 4 on `x86_64` is blocked behind giving that architecture a second time
  source. Until then `x86_64` is option 1 whatever the other two do, and a scope note should say so
  rather than letting §19 (architectural parity) report a gap it cannot close.
- **What it does not do:** it does not name a target the way milestone 147 wants. It says "this
  thread may read the counter", not "this profiler may read that subtree's counters". 147's work is
  still 147's.

## Recommendation

Split three ways, because the parts have different costs and different owners.

Recommended outright, and reversible: close what we claim is closed. Independent of the
authority question, and before any of milestone 74 lands:

1. Write `PMUSERENR_EL0 = 0` explicitly in aarch64 CPU init, per-core, rather than inheriting an
   architecturally UNKNOWN value. This is Linux's fix, for Linux's reason.
2. Clear `scounteren.CY` and `.IR` explicitly in the riscv64 per-hart timer init, so the comment
   that says they "stay closed" is made true by the code that says it.
3. Record in `notes/x86-port.md` and in `crates/user_rt`'s `now()` that on `x86_64` the cycle counter
   is ambient today, that this was inherited rather than chosen, and what closing it would cost.

Rung one is not available here (a register cannot be made unrepresentable), so this is rung two done
at init, plus a `BUGS` line where the reader meets it. It is three small writes, it is not the
decision, and it is the difference between a claim and a fact on argon.

Recommended, and calef's to confirm because it touches the syscall surface: option 4. It is the
only option that keeps the measured path free of a syscall while making the authority checkable, it
is what Linux arm64 converged on for the same reason, and it costs a compare on a path that already
does exactly this compare for `TTBR0_EL1`. Option 2 is the more seL4-shaped answer and is what
milestone 147 would eventually want; it is also unbuilt in seL4 after two and a half years, and
choosing it now buys a new object type before there is a second consumer, which is the speculative
abstraction both 74's and 147's scope notes already refuse.

Options rather than a recommendation, because it is irreversible: how the grant is expressed.
A field on TCB configure, a new spawn input, or a badge on an existing capability are three shapes
with three different syscall-surface costs, and a lane should not pick one. Nor should a lane decide
the `x86_64` row, since keeping `rdtsc` ambient there is a published confinement position and not
only an implementation state.

## Does this touch fatal risk 7?

**Yes, and the honest form of the answer is that it touches a claim the tree does not currently
make.** `notes/confinement-claims.md` enumerates 26 claims and names three more that are "stated
nowhere". The strings `timing`, `side channel` and `covert` appear in that note zero times, and zero
times in `DECISIONS.md` and in `design/fatal-risks.md`. So nothing in the confinement enumeration is
falsified by any answer here, because timing isolation is not among the things nife claims.

That absence is the finding, and it is the same category milestone 202 (every confinement test is a
ritual until somebody breaks the confinement) already found three members of. seL4 states the
position explicitly and in one clause ("this option opens the possibility of timing channels"), and
this tree, which will publish cycle-denominated numbers against seL4's, states nothing.

**What this decision should therefore also produce, whichever option wins: one row in
`notes/confinement-claims.md` stating what nife does not claim.** A confined component's *timing* is
not confined. That belongs beside the row saying a confined device's values are not confined, for
exactly the same reason: so nobody reads the capability rows as covering it.

## What it costs the benchmark story to say no

Real, and smaller than it looks, and it is worth being exact about who pays.

- Milestone 25's `sel4bench` comparability is the part that genuinely needs it. seL4's published
  413 and 426 are single-shot PMU measurements taken from user level. Reproducing that instrument on
  argon needs a user-level cycle read; a kernel-mediated timing of the same operation is not the same
  measurement and would not referee anything.
- **Our own numbers do not need it.** notes/pmu.md's whole point is that the long-loop generic-timer
  method is valid and is what survives virtualization: "Both are valid; they fail under different
  conditions." Milestone 168 (a multi-tasking workload benchmark) is a workload benchmark rather than
  a single-operation one, so it is a long-loop measurement and risk 4's decisive experiment is **not**
  blocked by a "no" here. That is worth saying plainly, because the brief that produced this document
  assumed otherwise, and the chain from this decision to risk 4 is weaker than it looks.
- Milestone 74's most-cited payoff survives a no. Turning "roughly 1,120 cycles at an assumed
  3.2 GHz" into a read number needs the counter read *somewhere*, and the kernel may read
  `PMCCNTR_EL0` at EL1 with no EL0 opening at all. What a no costs is the seL4-identical instrument,
  not cycles as a unit.

So a no is affordable for everything except the one comparison milestone 127 bought a board for.

## What is blocked until this is answered

- Milestone 74's aarch64 half, by its own gate. Its riscv64 SBI half is not, and neither is a
  kernel-side EL1 read.
- Milestone 147, entirely, by its own gate, since it cannot know what a grant unit is.
- Nothing else. Milestone 168 and risk 4 are not blocked, per the section above.

## BUGS

- No spike was run. The claim that an EL0 `mrs x0, pmccntr_el0` traps today under QEMU is
  inferred from Arm's register description plus the absence of any write to `PMUSERENR_EL0` in this
  tree; it was not observed. It was not run because the answer that matters is on argon, where the
  reset value is UNKNOWN and no emulator can report it, and because the aarch64 EL0 read is the one
  measurement that a QEMU run would answer least usefully. The `x86_64` claim was not spiked either
  and rests on reading `boot.s`'s two `CR4` writes and on notes/x86-port/user-mode-runtime.md's own statement.
- **The context-switch cost is priced by shape, not measured.** "One compare, one `msr` on change" is
  read off `switch_user_root`'s structure. Nobody has measured what an added `msr` costs on the
  switch path on any of the three architectures, and on `x86_64` a `CR4` write is serializing and
  would not be free.
- The riscv64 `mcounteren` half is untested. Even with `scounteren.CY` set, U-mode reads of the
  `cycle` CSR require `mcounteren.CY` from firmware, which is OpenSBI's on radon and is not ours.
  Whether it is set there is unknown and is a bench check, the same shape as milestone 127's
  "`PMCCNTR_EL0` readable at EL1" item.
- L4Re is missing from the prior art and it is the one gap in that section. See above.
