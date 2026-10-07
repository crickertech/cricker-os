---
status: BUILT
raised: 2026-08-23
built: 2026-09-19
---
# 161. The x86_64 kernel port: bring up the HAL's third architecture

The four items the 2026-09-03 follow-on left open were closed by the
lane on `milestone/161-x86-64-port-remainder`; see "Closing the follow-on (2026-09-19)" below the
follow-on list. What remains is recorded where a reader meets it rather than held here: the
two-core runner default belongs to milestone 315 (DECISIONS §153), and whether to set `CR4.PGE`
waits on a machine with a real TLB (`kernel/src/arch/x86_64/mmu.rs`'s BUGS).

*The status history, kept because the rest of the block cites it:* PARTIAL. Minted 2026-08-23, splitting real work out of milestone 20's stale text; early
boot built and gated the same day. **The kernel boots on QEMU's `q35`, reaches Rust in the high half
of a 4-level address space, prints over a 16550, installs a GDT/TSS and an IDT, catches a breakpoint
and steps over it, takes a calibrated timer interrupt, brings up the frame allocator, replaces the
boot map with fine-grained W^X page tables of its own, routes a real device line through the IO
APIC's redirection table, brings up the scheduler and preempts, runs a kernel thread, builds two
processes out of untyped memory and runs them at ring 3 (one invokes a capability it was granted and
exits, one is refused by the page tables and its death is delivered to its supervisor), gives both
regions back leaving the frame count where it found it, and runs a userspace built from **real ELF
programs**, read out of an initrd it found in PVH's module list, before it halts.**
`script/test --arch x86_64` runs
170 of the kernel's own tests and skips 67 (~~stale~~: 226 and 72 as of 2026-09-21, counted by the `x86-gates` lane; the suite grows and this number was not re-derived), every skip naming its missing fixture. What is built
and what is still open are spelled out at the bottom of this block; `notes/x86-port.md` is the
working record. The scope note below (milestone 20's "enough of each ISA to boot, confine a ring-3/U
process, and run the test suite") is met as of 2026-08-24, on QEMU. What keeps this milestone
PARTIAL rather than BUILT is item 5, SMP: the mechanism that starts a second core is built, but
nothing downstream of "a second core exists" has been shown safe yet, and two unresolved bugs are why
(`arch::x86_64::ap_boot`'s own `BUGS`; see item 5 below). Item 6, VT-d, is now BUILT (2026-08-25).
Item 0's wide half was split into its own milestone, **176** (PARTIAL, minted 2026-08-25), the same
way item 4's `fs_server`/`aes` gap was split into milestone 164: two of the four device windows this
item's text used to name as still tree-only turned out to already be built (the interrupt controller,
by item 2 below; PCI, by milestone 165's ACPI/MCFG work), so 176 tracks only what is actually left
(the CMOS RTC) without holding this milestone's own status hostage to it, the same precedent item 4
already set for 164. Item 4's own hand-off, no user program compiled for
`x86_64-unknown-none`, was closed the same day; see step 13 of "What was built".
DECISIONS §19 declared the target set (aarch64, riscv64, x86_64) and recorded honestly that
*"x86_64 is a declared target that does not exist yet."* Milestone 20's own "Deliverable, in two
parts" already named "bring up a second ISA, then a third: RISC-V first, x86_64 second," but 20 is
marked **BUILT** for the HAL split plus RISC-V alone (its own title says "proven on a second
architecture," singular) -- checked directly, `kernel/src/arch/` holds only `aarch64/` and
`riscv64/`, no `x86_64/` exists anywhere in the tree. The x86_64 half of 20's own deliverable was
never actually tracked as open work; this milestone is that tracking.

*While it was open, nothing gated it:* DECISIONS §19 already settled that x86_64 is a target.
Milestone 87's own text is explicit that this can start now, under QEMU TCG, the way RISC-V's port
did -- it is not gated on the physical machine.

## What this carries forward from milestone 20's original scope

Why x86_64 second (unchanged from 20's own reasoning, restated here since it now lives with the
work rather than beside it): the hard proof that the HAL abstraction is real rather than an
accident of two similar RISC ISAs. RISC-V is structurally close to aarch64 (device tree, weak
memory, a similar MMU shape); x86_64 is a genuinely different model: CISC, strong TSO memory
ordering, GDT/TSS, ACPI + PCI instead of a device tree, port I/O, the `syscall` + `swapgs`
trampoline, and INIT-SIPI-SIPI SMP bring-up instead of PSCI/SBI. If the `arch/` split survives x86,
it is real. Also the reach: x86_64 is what most machines are.

What x86_64 will stress (DECISIONS §19's own list): a different boot world (UEFI/ACPI, not
device tree; no OpenSBI/PSCI analog), the APIC instead of GIC/PLIC, a third page-table format
behind the `paging` seam, and TSO memory ordering -- where rule #4's weak-first discipline finally
pays out in the direction it was bet on: code proven correct on a weak machine (ARM, RISC-V) is
correct on TSO, and nothing about x86-first development could have said the reverse. The PCIe
transport (§18) is already x86's native bus, and the ECAM bridge both `virt` boards already use is
the same `pci-host-ecam-generic` shape x86 presents through ACPI.

## Sequencing against the physical machine (milestone 87)

**Starts under QEMU now; does not wait for the real machine.** Milestone 87 tracks the physical
OptiPlex 7050's bring-up (a real 16550 COM port, VT-d, an `igb`/`e1000e`-family NIC QEMU can stand
in for, four real cores, remote power cycling) and is the eventual parity-proof hardware, the same
role the VisionFive 2 played for RISC-V (milestone 16). QEMU's `q35` machine emulates the same
16550 UART the real hardware carries, so the boot/serial driver spans both from the start, the
NS16550/PL011 pattern both existing ISAs already follow.

As of 2026-08-23: the Dell C4PDJ serial module and the dev-side RS-232 chain have arrived and
are installed. Milestone 87's remaining blocker (recorded 2026-08-18 as *"the machine is here
and the serial module is not"*) is closed on the hardware side; what remains for 87 to complete is
the actual bring-up (boot code, the UART driver, printing a byte over serial on real silicon),
which is downstream of this milestone's own boot/console work reaching a point worth trying on the
box. See milestone 87 for the machine's own status.

## What this does not decide

The exact boot path (a minimal UEFI stub vs. a bootloader) and the SMP bring-up sequence's precise
shape are implementation judgment for whoever builds this, not decided here -- DECISIONS §19 named
the stress points, not the mechanism.

## What it unblocks

Architectural parity (DECISIONS §19) reaching all three declared targets rather than two. Milestone
25's `sel4bench` cross-OS comparison, once real x86 hardware (87) is behind it, gets a third
comparison point.

## What was built (2026-08-23, plus steps 9 through 12 on 2026-08-24)

Ordered as it was built, because each step is what made the next one debuggable.

1. The boot path. `kernel/link-x86_64.ld` and `kernel/src/arch/x86_64/boot.s`: a 32-bit
   trampoline into long mode and the high half. The boot protocol is the surprise and is written up
   at length in both files and in `notes/x86-port/boot.md`: Multiboot 1 **cannot** boot a 64-bit kernel
   (QEMU refuses the image rather than ignoring the header) and QEMU 11 has no Multiboot 2, so the
   image carries a **PVH** ELF note instead. PVH also hands over the ACPI RSDP address, which is the
   nearest thing x86 has to the single device-tree pointer the other two architectures pass.
2. The console. The existing NS16550 driver, unchanged in substance: x86 puts the same part at an
   I/O **port** rather than a memory address, so `drivers/ns16550.rs` gained a `RegisterSpace` type
   parameter (defaulting to MMIO, so nothing else changed) and `arch/x86_64/port.rs` supplies the
   `in`/`out` half. One driver, two address spaces, which is also what milestone 87's real machine
   will want.
3. The trap path. `segments.rs` (GDT + TSS, with the double fault on its own IST) and
   `exceptions.rs` + `trap.s` (an IDT, 256 generated stubs, one trap frame). Proven by taking an
   `int3` and returning from it. Before this line a fault is a triple fault and a silent machine
   reset.
4. The page-table format, behind milestone 20's existing seam: `crates/paging/src/x86_64.rs`,
   sixty lines of entry codec plus seven host tests and six Kani harnesses, with no change to the
   generic walk. That is the milestone-20 claim tested and holding.

5. The boot handoff, parsed. `crates/machine_discovery/src/x86_64.rs` decodes
   `hvm_start_info`, host-tested (seven tests), beside the two ISA records already in that crate; it
   is a parser, so it lives where a parser can be proved in milliseconds rather than in an emulator.
   The tour prints the memory map, which is what proves the handoff carries real data: nine regions,
   255 MiB usable, and the PCIe ECAM window reported as reserved at exactly the address
   `arch::mmu::PCI_ECAM_PHYS` hardcodes. QEMU's PVH loader leaves the ACPI RSDP field zero, so
   the root pointer has to be found by scanning the BIOS area, which is a prerequisite for the APIC
   step rather than a detail.

6. ACPI, which is what x86 has instead of a device tree.
   `crates/machine_discovery/src/acpi.rs` decodes the RSDP, the root-table walk, the SDT header, the
   MADT and the MCFG, host-tested (thirteen tests), beside the arch records rather than inside the
   x86_64 one because ACPI is not an x86 standard. The kernel side scans for the RSDP, since QEMU's
   PVH loader leaves the field zero, and every table's checksum is verified before it is believed.
   Verified against real firmware: five tables found on q35, the local APIC at `0xfee00000`, one
   enabled CPU, the 8259s reported present, the IO APIC at `0xfec00000`, and the ECAM window at
   `0xb0000000`, which is exactly what `arch::mmu::PCI_ECAM_PHYS` hardcodes and what the PVH memory
   map independently reports as reserved.

7. The local APIC and a calibrated periodic timer. `irq.rs` masks the 8259s (an obligation the
   MADT states, and their power-on vectors overlap the CPU's own exception vectors), enables the
   local APIC at the address ACPI gave, and owns the timer's LVT; `timer.rs` measures both the TSC
   and the APIC timer against the PIT in one ten-millisecond window, because x86 has four clocks and
   no architected way to ask any of them its rate. Verified: 20 ticks in ~0.2 s at 100 Hz, 20
   routed, 0 spurious, which proves the whole interrupt path rather than only the clock. A missed
   EOI is a hang on this architecture, so exactly one tick would have been the failure to expect.

8. The frame allocator. `memory::init` was split into a device-tree front end and
   `memory::bring_up_page_frames`, which takes a RAM slice and a forbidden slice and does not care where
   they came from; the x86 front end builds those from the PVH map. Verified: 254 MiB total, 254
   MiB free, first frame 0x16f000. The whole first megabyte is clipped rather than reserved (it
   holds the IVT, the BDA, the EBDA, the boot info, and the page a STARTUP IPI's vector can name),
   and the kernel image is the one entry on the forbidden list, which covers the boot page tables
   because the linker script puts `.boot_scratch` inside the image bounds.

9. Fine-grained page tables, on a direct map with a base of its own (2026-08-24, the open
   list's item 1 below). `mmu::init` builds a W^X four-level map and switches `CR3` to it:
   `.text` executable and read-only, `.rodata` neither writable nor executable, everything else
   non-executable, every guard page a hole, device registers device-typed, and no identity map.
   The direct map now covers all of physical memory rather than the low gigabyte.

   The address-space decision item 1 recorded was taken as recorded: two bases, Linux's, the
   image at `KERNEL_VA_BASE` where the code model pins it and the direct map at
   `0xffff888000000000` (PML4[273]), with `virt_to_phys` inverting both, which is Linux's `__pa()`
   shape. `crates/paging` again needed nothing.

   The sequencing hazard was removed rather than sequenced around, and that is the part worth
   carrying forward. Item 1 prescribed carrying the old alias across the `CR3` switch and dropping
   it afterwards, because `phys_to_virt` changes meaning at that instant and the frame bitmap is a
   stored `&'static mut [u8]` derived through it. Instead `boot.s` installs PML4[273] itself,
   pointing at the same low PDPT the identity map already used: eight bytes, no memory, and
   `phys_to_virt` means one thing for the machine's whole life. `mmu::device_va`, the foot gun that
   existed only because the direct map could not reach a device, was deleted outright rather than
   migrated.

   Verified on q35 with `-m 256M`: the tour keeps printing across the `mov cr3`, the timer
   keeps ticking, and the map costs 556 KiB of page tables for 254 MiB of RAM. That 0.2% is the
   price of 4 KiB leaves and is recorded in `arch/x86_64/mmu.rs`'s `BUGS`, with the two other honest
   limits: `mmu::init` must draw its tables from below 4 GiB (all the boot map reaches), and
   `CR4.PGE` is still off, so the `G` bit the kernel's `Flags` set is ignored and every `mov cr3`
   flushes everything.

10. The IO APIC, and a real device line routed through it (2026-08-24, the open list's item 2
    below). `irq.rs` gained the redirection table: the `IOREGSEL`/`IOWIN` index-and-data register
    pair, an entry count read from the version register, every entry masked on the way in, and
    `route_gsi`/`mask_gsi`/`enable` writing one. The arch contract's `enable(intid)` now means what
    it means on the other two architectures, and `exceptions::enable_external` stopped being an
    `unimplemented!()` and became a documented no-op, because x86 has one interrupt gate
    (`RFLAGS.IF`) where RISC-V has two.

    The trap the item flagged is real, and the fix is in the crate rather than the kernel. A
    legacy IRQ number is not an IO APIC input: q35's MADT rewires ISA IRQ 0 to GSI 2 (the PIT is on
    pin 2; pin 0 carries the 8259 cascade) and IRQs 5, 9, 10 and 11 to level-triggered. Resolving
    that is pure logic over table bytes, so it is `machine_discovery::acpi::isa_irq_table`, with
    six host tests, one of which is the IRQ-0 case by name. The flags word is two two-bit
    fields, not two bits, and a test exists for exactly that misreading: `0x000d` is active
    *high*, level, and a decoder testing bit 1 for polarity would answer active low and arm a line
    that never fires.

    Verified on q35: `pit irq 0 -> gsi 2 on vector 0x32: 20 interrupts in ~0.2 s at 100 Hz`,
    with the local APIC timer masked for that window so the count is the PIT's alone. Twenty is the
    same number the local APIC timer produces in the same window against the same TSC, which is
    what makes it a measurement rather than a nonzero. The vector map is flat
    (`GSI_VECTOR_BASE + gsi`, base 0x30), so a stray vector in a fault report names its line by
    subtraction; it costs the ability to express a priority policy, and there is not one to express.

11. Ring 3, the `syscall` pair, and a program the page tables refuse (2026-08-24, the open
    list's item 3 below). Four pieces, and the order below is the order they have to work in.

    The user address spaces. `arch/x86_64/mmu.rs`'s user block stopped being a wall of
    `unimplemented!()`. x86 is RISC-V's single-root model, not aarch64's: `CR3` names the whole
    address space, so `share_kernel_half` copies PML4 entries 256..512 out of the kernel root into
    every process root, which shares the image (PML4[511]), the direct map (PML4[273]) and every
    device window in one `copy_from_slice`. Without it the `mov cr3` unmaps the instruction after
    it.

    The `swapgs` pair, guarded on the interrupted CPL. The guard is the whole design: `swapgs`
    is an *exchange*, so a trap from ring 0 that swapped would install the user's GS base while
    running kernel code, and the next nested trap would hand the kernel's back to the user. Both
    sites test the saved `CS`'s low two bits, which are the interrupted privilege level. It does
    **not** trip the hazard `segments.rs` documents (loading a segment register in long mode zeroes
    that segment's base MSR), because it writes the MSR pair and loads no selector; that was checked
    against the instruction's definition rather than assumed, since it is exactly the shape of bug
    that cost this port an afternoon once.

    The `syscall` entry. Four MSRs (`IA32_STAR`, `IA32_LSTAR`, `IA32_FMASK`, and `EFER.SCE`
    last, so the instruction becomes legal only after the address it jumps to exists), programmed in
    `arch::init` beside the GDT and the IDT because they are per-CPU state of the same kind. The
    entry path is the part with no counterpart on the other two architectures: `syscall` **does not
    switch stacks** and pushes nothing, so the first three instructions are `swapgs`, park the
    user's `rsp`, and load the kernel's. `segments::set_kernel_stack` writes `TSS.RSP0` and the
    `syscall` path's stack together, so the two doors into the kernel cannot come to name different
    stacks.

    **The return is an `iretq`, not a `sysretq`, and that is a decision rather than an omission.**
    `sysret` returns to `rcx` without checking it is canonical, and a non-canonical `rcx` faults *in
    ring 0 on the user's stack* (the shape of CVE-2012-0217); using it safely needs an explicit
    canonicality check and an `iretq` fallback. Sharing one restore path costs some tens of cycles
    per syscall and buys one `swapgs` rule with one place to get it wrong. Recorded in that
    function's `BUGS` as worth revisiting with a benchmark, once there is a syscall-heavy
    workload here to measure.

    What it was proved with, and the honest size of the claim. There is no ELF built for
    `x86_64-unknown-none` and no scheduler on this architecture, so the proof is a hand-assembled
    probe (`arch/x86_64/ring3_probe.s`) entered from the boot thread, the same shape the other two
    ports shipped on their first day. Verified on q35:

    ```
      ring 3      : a program ran at cpl 3 (cs 0x0023, ss 0x001b) and made 2 syscalls
                    the portable dispatcher answered BadSyscall (-6) to an unknown number
                    reading the kernel's .text at 0xffffffff80109000 from ring 3: Permission(Read)
    ```

    Three separate facts, and the third is the strongest. `cs 0x23` is the **hardware's** answer to
    what ring this is, since CPL is literally `CS[1:0]`. `BadSyscall (-6)` came out of the
    *portable* `crate::syscall::dispatch` through `TrapFrame::{syscall_nr, arg, set_arg}` and
    reached the program in `rdi`, which means the ABI accessors, the entry and the return all work;
    asking for a refusal is deliberate, because it is the one answer that dispatcher can give on a
    kernel with no scheduler. And `Permission(Read)` rather than `Translation(Read)` is the page
    tables saying the walk **found** the kernel's `.text` and refused it, which is the privilege
    boundary doing its job rather than an accident of an incomplete map.

    **Two latent bugs on the untested user-thread path were found and fixed while reading it.**
    `Context::for_user_thread` wrote the child's three arguments into `r13`/`r14`/`r15` while
    `user_entry_trampoline` read them from `r12`/`r13`/`r14`, so a child would have received
    `(0, arg0, arg1)`; and the trampoline did not reserve a trap frame's worth of stack, which
    milestone 71 established both other architectures need. Neither could have been caught before
    this item, because nothing had ever entered ring 3 here. Both are still unexecuted: that
    trampoline is the *scheduler's* entry path, and the self test enters through `enter_user`
    directly.

12. The scheduler, real processes, and the kernel's own test suite (2026-08-24, the open list's
    item 4). The distance between item 11's "a program ran at CPL 3" and "a process", closed.

    Almost none of it was new x86 code, which is the milestone-20 claim holding a third time.
    `kmem`, `untyped`, `sched` and `user::AddressSpace` are portable and came up here by being
    compiled for this target. What had to be written was the arch layer underneath them; what had to
    be *found* was four places where portable code encoded an assumption true of two architectures.

    The trap path grew the split both other ports already had. `x86_trap_handler` became
    `x86_trap_dispatch` (outer, on the interrupted thread's stack, where the deferred `schedule()`
    runs) plus `x86_trap_body` (inner, which may run on this CPU's interrupt stack), with
    `dispatch_on_interrupt_stack` in trap.s between them. Not an optimization: `schedule()` parks the
    running `rsp` in the outgoing thread's `Context`, so calling it from a per-CPU stack would park a
    per-CPU address in a thread.

    `TSS.RSP0` and the `syscall` path's stack are recomputed from the frame on every return to
    ring 3. x86 has two doors from ring 3 and they find their stack differently; with a scheduler
    there is one kernel stack per thread, so the pair has to be re-pointed at every switch. The
    frame's own address is the answer (`rsp + 176` at the top of `isr_restore`, because milestone 71
    put every thread's `TrapFrame` at `stack_top - 176`), which makes the wrong state unrepresentable
    rather than a duty to remember. RISC-V does exactly this at exactly this point.

    `send_reschedule` stopped being `unimplemented!()`: the local APIC's ICR is an IPI send, and
    its "self" shorthand is also how this ISA raises an interrupt by hand, which puts x86 on
    aarch64's side of `sched`'s test-fixture split rather than RISC-V's. The intid for a local
    APIC source is its vector, because there is no controller input to name; legacy IRQs (0..15) and
    these vectors (0x20..0x2f) cannot collide.

    The ring-3 fault arm became a real teardown, and `ring3_probe.s` plus the
    `x86_enter_user_and_wait`/`x86_leave_user` pair were deleted, exactly as item 3 said they would
    be.

    **Four bugs in portable code, all one shape**: a default arm or an expression naming one of two
    answers, silently wrong at three. `thread.rs`'s stack area was `KERNEL_VA_BASE | 0x10_0000_0000`,
    which is the **identity** on x86 (that base already carries the bit), so every kernel thread
    stack would have been mapped over `.text`. `crates/elf`'s `EXPECTED_MACHINE` was
    `not(riscv64) => EM_AARCH64`, so the x86 kernel was compiled to accept **aarch64** binaries and
    refuse its own. `xtask`'s `ArchLegs` predicates were `self != the_other_one`. And
    `smp::bring_up_secondaries` computed the secondary entry with `virt_to_phys` before checking
    whether any core can be started, which panics here because `secondary_boot` is in `.boot`, linked
    at its physical address.

    Verified on q35, and the frame line is the one that means the most:

    ```
      scheduler   : up on 1 cpu, preempting at 100 Hz (idle thread registered)
      kernel task : a spawned thread ran and carried its captured state (0x16100004)
      userspace   : a process built from untyped ran at cpl 3 and sent 0x1610004 on a granted cap
                    thread 8589934594 died at pc 0x400005 on addr 0xa50000, delivered to its supervisor
                    two children cost 0 frames the first round and 0 the second (steady state)
    ```

    Two children, because the two halves of "a process" fail differently: one is built out of a
    single untyped region (address space, code page, stack page, TCB, two endpoints), granted a
    capability, dispatched to ring 3 by the scheduler, invokes that capability and exits; one
    faults and its death arrives on its supervision endpoint naming the thread, the pc and the
    address. The round runs **twice** because a first round pays first-use carves that are not leaks,
    and a system in steady state charges the second one zero. The first version of it reported
    sixteen frames a round going missing, which was true and was the demo's own fault.

    And the suite: `script/test --arch x86_64` runs 97 tests, skips 7, in 2.7 seconds.
    `--arch` takes a third value and the default runs all three legs, which is what DECISIONS §19
    means by parity being a gate. The seven skips each print a reason: two are fixtures the runner
    does not attach, four are SMP or roster facts a single-core machine with no device tree cannot
    have, and one is the device-tree magic, which PVH's `hvm_start_info` genuinely is not.

13. Userspace: every program in `user/` compiled for this target, an archive, and the thirty
    gated test modules running (2026-08-24, item 4's own hand-off). The suite went from 97 pass /
    7 skip to 170 pass / 67 skip, and every skip prints the fixture it wants.

    `crates/user_rt` cost five transliterations and two decisions. The five follow DECISIONS
    §124 mechanically. Three facts at those sites have no counterpart on either other architecture
    and each is the instruction rather than a choice: `syscall` clobbers `rcx` and `r11`
    unconditionally, `r10` carries the fourth argument because `syscall` has already taken `rcx`,
    and `syscall` pushes nothing.

    **`now()` and `cntfrq()` are the two that are not transliteration, and the answer, ratified as
    [DECISIONS §127](../decisions/0127-x86-64-timer-rdtsc.md), is PIT-calibrated `rdtsc` with the
    measured frequency delivered through a mapped page.** `now()` is `rdtsc`, readable from ring 3
    because `CR4.TSD` is clear at reset and this kernel does not change it; it answers in
    `edx:eax`, so a single `out(reg)` reads a counter that wraps every four seconds. `cntfrq()` is
    RISC-V's recorded gap, one architecture worse: there is no architected TSC rate (`CPUID`
    leaf 0x15 gives a ratio to a crystal leaf 0x16 may not report, and neither leaf is universal),
    so the kernel **measures** it against the PIT at boot (`kernel/src/arch/x86_64/timer.rs`) and
    delivers the number through `counter_frequency_protocol::TimebasePage`, mapped read-only into every
    process the kernel builds directly, the same aux-vector-at-process-start shape riscv64's own
    `cntfrq` doc comment already predicted. A ring-3 program cannot repeat the PIT measurement
    itself because the PIT is at ports 0x40..0x43 behind `IOPL` 0 and an empty TSS bitmap; that is
    §121 rather than an oversight. The 1 GHz constant that remains is narrower than it first
    reads: only a process built by `supervision_protocol::build_child_space` (the userspace ELF
    loader, not the kernel) falls back to it, because that loader maps a freshly retyped, zeroed
    placeholder page rather than a capability naming the kernel's real one. Closing that gap is
    [milestone 167](0167-timebase-page-delegation.md)'s own, separately-scoped remaining piece, not
    a live design fork on this one.

    Three programs refuse rather than pretend. `console::uart_put`, `input`'s `uart` module and
    `swap_protocol::probe_device` cannot reach a device from ring 3; their x86 arms `trap()` rather
    than no-op, because a silent no-op is a console that acknowledges every byte and prints none.
    `user/build.rs` compiles the C seam with `--target=x86_64-unknown-none-elf -mno-sse -mno-mmx
    -mno-red-zone`, matching the Rust target's own `-mmx,-sse,+soft-float` and `disable_redzone`.

    The archive arrives as a PVH module, which is `/chosen/linux,initrd-start` one machine over:
    `machine_discovery::x86_64::module` decodes the list (host-tested), the x86 memory front end
    reserves the region and records it, and free memory drops from 254 MiB to 249 with a 4.4 MB
    archive attached. `xtask` grew `initrd-x86`, and the two packers now share one
    `portable_archive_entries()` table.

    **Four bugs, every one latent since the day it was written**, and none catchable before there
    were user programs here. `arch::x86_64::irq::enable` conflated two numbering schemes (a legacy
    IRQ and a local APIC vector) and panicked on `SELF_TEST_VECTOR`; `user_rt::trap()`'s `int3` is
    refused from ring 3 by a DPL-0 gate and raised #GP 0x1a instead of #BP, so it is `ud2` now;
    `user/link.ld` did not name `.got`, which x86 emits and the other two do not, so the orphan
    landed after `.bss` and the `data` segment lost its zero-fill tail entirely; and
    `fs_service::blk_server_image()`'s x86 arm was a `panic!` that fired before any disk check.
    `xtask`'s `read_stripped` cache tag was a fifth, latent and never yet fired: x86 shared
    aarch64's `"host"` namespace, so packing both archives in one run would have measured one
    architecture's bytes under the other's name.

    **What still bounds this architecture is devices, not userspace.** 21 of the 67 skips are one
    toolchain failure (see item 4's block), 13 are the missing RTC, 15 are the unenumerated PCI bus,
    5 are §121, and 4 are SMP.

Plus the build wiring it all needs: `kernel/build.rs`, `.cargo/config.toml` (including the static
relocation model, which this target does not default to), `rust-toolchain.toml`,
`helpers/qemu-runner-x86_64.sh`, and an x86_64 pass in `script/lint`.

The evidence that the `arch/` split is real: making the *entire* kernel compile for a third
architecture took 42 compiler errors, every one of them a missing `arch::` name, and four `cfg` arms
in files that already had two. `crates/paging` needed nothing.

## What is still open

In the order it should be done, because each is a prerequisite for the next.

0. The device-discovery seam, milestone 20's other promised abstraction. Its narrow half is
   built (`memory::bring_up_page_frames`), because without it the frame allocator could not come up on
   a machine with no device tree and the port stopped there; `memory::init` is now explicitly a
   device-tree front end and `arch::x86_64::machine::bring_up_memory` is the x86 one.

   The wide half was split into its own milestone, [176](0176-x86-64-discovery-seam-wide-half.md)
   (PARTIAL, minted 2026-08-25), once checking this item's own claim directly against the tree found
   it had gone stale in two of its four places rather than being uniformly still open. The
   interrupt controller is built, by item 2 below: the boot tour hands the MADT's answers to
   `arch::x86_64::irq` directly, a path that goes around `memory.rs`'s statics rather than through
   them. PCI is wired, and has been since milestone 165's ACPI/MCFG work: `main.rs` calls
   `memory::record_pci_regions` from the ACPI `ecam` path, so `memory::pci_regions()` returns `Some`
   on x86_64 today. What 176 actually tracks is the two windows that were genuinely still open:
   piece 1, COM1's ISA IRQ into `memory::UART_IRQ` (`Acpi::isa_irqs[4]`, via a new
   `memory::record_uart_irq`), is BUILT (2026-08-25); **piece 2, a CMOS RTC seam, is DECIDED but
   not yet built**. Sizing it found a real design fork (the PC-compatible CMOS RTC is two fixed I/O
   ports, not a page, so the "map the device, let userspace drive it" pattern the other two
   architectures' RTCs use has no CMOS equivalent), resolved as
   [DECISIONS §130](../decisions/0130-cmos-rtc-delegation.md) (**DECIDED** 2026-08-26, "Ratify option
   3"): the kernel reads CMOS once at boot and hands the seed to the clock service as a `Spawn`
   argument, the same way `kind` already crosses that boundary. `kernel/src/arch/x86_64/machine.rs`'s
   own `BUGS` section now names only the CMOS RTC as the device window with no seam at all;
   notes/x86-port/acpi-and-pci.md has the table of which fact has which source. Milestone 176
   (the discovery seam's wide half) has the current state of both pieces.
1. Fine-grained page tables, and the address-space layout they force a decision about. BUILT
   2026-08-24; see step 9 of "What was built" for what landed and what it cost. The number is kept
   in place rather than struck out because the items below cite each other by it.

   The decision this item existed to record (two bases, Linux's) was taken as recorded, and the
   hazard it flagged was removed rather than sequenced around: `boot.s` installs the direct map's
   PML4 entry itself, so `phys_to_virt` never changes meaning. `mmu::device_va` is gone. What is
   still open here is not the map but its **leaves**: `crates/paging` maps 4 KiB pages and nothing
   else, so the direct map costs 0.2% of RAM in page tables. 2 MiB and 1 GiB leaves are a change to
   the shared format trait that all three architectures would want, and they want their own
   milestone rather than an x86 patch.
2. The IO APIC. BUILT 2026-08-24; see step 10 of "What was built". The number is kept in place
   rather than struck out because the items below cite each other by it.

   The trap this item existed to flag turned out to be exactly as stated and is now proven rather
   than asserted: q35's MADT says ISA IRQ 0 arrives as GSI 2, and the resolution lives in
   `machine_discovery::acpi::isa_irq_table` where a host test holds it. What is still open here is
   PCI interrupt routing, which is blocked on AML rather than on tables, and **MSI**, which
   bypasses the redirection table entirely by writing a vector straight to the local APIC and is
   the path worth building for that reason. Neither is this item, and neither was.
3. Ring 3. BUILT 2026-08-24; see step 11 of "What was built" for what landed, what it proved,
   and the two things it deliberately did not. The number is kept in place rather than struck out
   because the items below cite each other by it.

   What is proven is the arch layer, and the boundary between that and a process is where this
   item now stops. A program runs at CPL 3 (the hardware's own `cs` reads `0x23`), makes syscalls
   that reach the *portable* `crate::syscall::dispatch` and get its answer back in `rdi`, returns to
   ring 3, and is refused by the page tables when it reads the kernel's `.text`. What is **not**
   proven is anything above the arch layer: there is no ELF loader path here, no capability grant,
   no argument passing beyond the three registers `TrapFrame::for_user_entry` sets, and no
   scheduler, so the program is a hand-assembled probe entered from the boot thread rather than a
   process. Item 4 is where that gap closes, and it is the reason item 4 is the next thing.

   The syscall ABI (`rax` + `rdi`/`rsi`/`rdx`/`r10`/`r8`/`r9`) is now spoken and ratified: it was
   a boundary rather than a habit (DECISIONS §10, §16), and one probe program agreeing with the
   kernel was not the same as a decision having been made, so it went up as
   [DECISIONS §124](../decisions/0124-x86-64-syscall-abi.md), which calef ratified 2026-08-24.

   The two `CR4` bits stay off, and that is now a recorded choice rather than a deferral.
   `CR4.PCIDE` off means `crates/address_space_identifier`'s tags have nowhere to live: PCID is `CR3[11:0]`, and with
   PCIDE clear those bits are reserved-zero, so writing a tag into them faults rather than tagging
   anything. `arch/x86_64/mmu.rs`'s `ttbr0_value` therefore drops the number and `flush_asid`
   flushes the whole TLB, each saying so in its own words rather than implying a selectivity the
   hardware does not have. `CR4.PGE` off means every `mov cr3` discards the kernel's mappings too.
   Both are worth turning on and neither can be *shown* to be worth it yet: nothing on this
   architecture switches address spaces often enough to measure, so turning them on now would buy a
   number nobody could check. That measurement arrives with the scheduler (item 4), which is where
   the decision belongs.

   **Item 4 landed and the measurement is now available but was not taken, deliberately.** There is
   a workload to measure against (the suite context-switches throughout, and `mmu::switch_user_root`
   already skips a `CR3` write that would change nothing, which is worth more here than on either
   other architecture precisely because PGE is off). Turning either bit on is an architect's call
   and wants a number rather than an argument: `script/icount` has no x86 leg, so producing one is
   its own small piece of work rather than a line in item 4's diff.

   **The tooling half is now done (2026-08-25), the decision is not.** `cargo xtask bench --x86`
   gates on a deterministic instruction count the same way the other two ISAs already do
   (`bench/baseline-x86_64.txt`, `-icount shift=0,sleep=off`; see notes/benchmarks.md's
   2026-08-25 section for the measurement that established `rdtsc` tracks icount's virtual clock
   on `q35`). This is a leg of `cargo xtask bench`, not of `script/icount` itself: milestone 78's
   instrument (`kernel/src/icount.rs`) still refuses `--arch x86_64` and correctly so, since its
   claims need a re-armed deadline timer to compare against and this port's LAPIC timer is
   periodic hardware reload with no deadline to read. What exists now is `yield_switch`'s
   deterministic baseline (9,132 instructions/switch, a bare kernel-thread switch with no `CR3`
   write). The CR4 question itself is still unmeasured: nothing yet exercises
   `switch_user_root`'s address-space-switch path under load, so this baseline is not yet the
   number the two bits would be judged against, only the tooling that would report it once such a
   workload exists. This item's status does not move.
4. The scheduler, real processes, and the kernel test suite. BUILT 2026-08-24; see step 12 of
   "What was built" for what landed, what it cost, and the four portable-code bugs it found. The
   number is kept in place rather than struck out because the items around it cite each other by it.

   All three hand-overs it named were taken: the `x86_enter_user_and_wait`/`x86_leave_user` pair and
   `ring3_probe.s` are deleted, and the ring-3 fault arm is a real teardown through `sched::fault`.
   The bring-up was indeed mostly portable code meeting a third architecture for the first time.

   Its own hand-off was closed the same day it was written (2026-08-24; see step 13 of "What
   was built"). Every program in `user/` compiles for `x86_64-unknown-none`, `xtask` packs an
   archive, QEMU's PVH loader hands it over as a module, `cfg(initrd)` is on, and the suite runs
   170 tests and skips 67 where it ran 97 and skipped 7. The block's own prediction held
   exactly: turning the cfg on was one match arm in `kernel/build.rs`, and all thirty modules
   came back with nothing else edited.

   Of the five concrete pieces it listed, four are done and the fifth was already true:
   `crates/user_rt` has its x86 arms (five transliterations, plus `now()`/`cntfrq()`, which went to
   `rdtsc` and a PIT-measured rate for the reasons in step 13, [DECISIONS §127](../decisions/0127-x86-64-timer-rdtsc.md));
   `user/build.rs` compiles the C seam;
   `xtask` packs an x86 archive, with the `read_stripped` cache-tag collision fixed before it could
   fire and a real `target/init-measure-x86_64.txt`; `helpers/qemu-runner-x86_64.sh` passes
   `-initrd`; and `crates/elf` was indeed ready.

   What replaced it is a shorter list, and none of it is userspace. Two items are genuinely new
   findings rather than restatements:

   - **`fs_server` does not compile for `x86_64-unknown-none`, and it is not our bug.** It links
     the vendored RedoxFS engine, which depends on `aes` unconditionally (the crypto is not behind
     a feature), and `aes` for this target ends in
     `rustc-LLVM ERROR: Do not know how to split the result of this operator!` at every optimization
     level including zero. The target spec is the cause: `-mmx,-sse,+soft-float` leaves LLVM no
     128-bit vector register to legalize an AES block into and no scalar fallback. 21 of the 67
     skips are this one fact. The routes out are a patch against the vendored crate to make its
     crypto optional (`patches/` is where a carried patch belongs) or an x86 userspace target that
     keeps SSE; both want their own milestone (now minted: milestone 164), and until one lands
     there is no point attaching a
     disk to the x86 runner, because there would be nothing to open it with.
   - **One foot gun is marked rather than removed** (AGENTS.md's ladder: an exception must say it
     is one). `spawn_init` grants slot 2 a device capability over `user::UART_PHYS`, which on this
     architecture is *physical page zero*. The slot is positional, so declining to grant would
     renumber the interrupt capability and every role that names it, and there is nothing better to
     put there until DECISIONS §121 answers what a port capability is. Nothing reaches it: every
     fixture that would map it asks `user::machine_has_no_device_page_for_the_console()` first and
     skips. A role that mapped it anyway would read the real-mode interrupt vector table and look
     like it worked.

   The rest of the 67 were the items above and below this one, doing what they were always going to
   do, though what closes some of them has since changed: 13 skips wanted the RTC, which is now
   milestone 176 piece 2's job (decided, [§130](../decisions/0130-cmos-rtc-delegation.md); not §121,
   which forecloses a CMOS port capability rather than building one); 15 wanted a PCI bus enumerated,
   which the discovery seam itself no longer blocks (item 0's PCI window is wired, milestone 165) but
   `helpers/qemu-runner-x86_64.sh` still does, since it attaches no PCI device (`-device
   virtio-blk-pci` and friends) for anything to enumerate; 5 want a UART page, which §121 now answers
   permanently rather than pending (kernel-resident forever, no port capability, so these five stay
   skipped by design rather than by omission); 4 want a second core (item 5); 1 wants
   `CR4.PCIDE` (item 3's measurement), and 1 wants an `x86_64-unknown-nife` target and a `std` farm
   (milestone 27).

5. SMP, via INIT-SIPI-SIPI. PARTIALLY BUILT 2026-08-25: the mechanism that starts a second
   logical CPU is built and does what it says; nothing downstream of "a second core exists" has
   been shown safe yet, and the standard test suite does not exercise it by default because of
   that, not because the mechanism itself is unfinished. Both of this item's own two named bugs
   are fixed, not inherited.

   What landed. The Interrupt Command Register sequence (`arch::x86_64::irq::send_init`/
   `send_startup`) and a real-mode trampoline (`boot.s`'s `secondary_boot`, now real code rather than
   a `hlt` stub) linked at a fixed low virtual address (`AP_TRAMPOLINE_PHYS = 0x8000`, `link-x86_64.ld`'s
   `.ap_trampoline`) so its own address already is the physical page a `STARTUP` IPI has to name, and
   copied there at runtime from an ordinary spot in the loaded image (`arch::x86_64::ap_boot::prepare`)
   because its two natural-looking homes, `.boot_scratch` and the secondary stacks, are both
   runtime-mutated and would hand back corrupted bytes. The trampoline replays `_start`'s own
   16→32→64-bit transition against the same boot page tables, reads its own local APIC id via
   `CPUID` (the same "Initial APIC ID" `arch::boot_cpu_id` now reads on the boot core, fixed from a
   hardcoded 0, so the roster's seating and the boot core's own logical id agree), and jumps to the
   portable `secondary_main` already used by the other two architectures. A per-CPU TSS and GDT
   (`segments.rs`, indexed by `cpu::id()`) replace the single boot-CPU pair item 4 left behind, and
   the three words `trap.s`'s `isr_restore`/`x86_syscall_entry` reach per CPU (the syscall path's
   kernel stack, its scratch for the interrupted user `rsp`, and a pointer to this core's own
   `TSS.rsp0`) moved into `cpu::PerCpu::x86_trap`, reached through a `gs`-relative offset the same
   way every other per-CPU write on this architecture already is. The ACPI MADT's local-APIC-id
   roster is read into the same `HWID`/`STARTABLE` arrays `smp::read_cpu_list` fills from a device
   tree (`smp::seat_cpus_from_acpi`), seating each core, boot core included, at the slot its own
   local APIC id names. A single secondary, started this way, reaches `secondary_main` and its own
   idle loop reliably, measured across dozens of boots.

   **Both named bugs are fixed.** `smp::bring_up_secondaries` no longer calls `arch::mmu::virt_to_phys`
   on `secondary_boot`'s address on this architecture (that address is already physical, by
   construction of the linker script above; `arch::secondary_boot_entry` is the identity function
   here and says why). `irq::send_reschedule` no longer uses the logical cpu id as a local APIC id
   directly; it looks the real one up in the roster (`smp::hwid`) the same MADT read above built.

   **What did not land: anything past "a second core idles". Two separate, unresolved failures.**

   (1) A third or later secondary fails intermittently. Brought up while an earlier one is
   already online and running, exactly one secondary typically fails to reach `secondary_main`'s
   online mark, and *which* one varies run to run. Instrumented with raw port-I/O checkpoints, the
   failing core reaches 64-bit long mode but not the last checkpoint before jumping to
   `secondary_main`, across five ordinary instructions identical to what the succeeding core(s)
   just ran. Two direct hypotheses were tested and neither held up: routing `cpu_start`'s wait
   through `hlt` instead of a busy spin (in case CPU 0's own loop was starving the target vCPU
   thread of host scheduling time under QEMU TCG) turned an occasional full hang into a reliable
   clean give-up but did not fix the underlying failure; copying the trampoline's code bytes only
   once instead of once per `STARTUP` IPI (in case QEMU's self-modifying-code detection was
   mishandling a rewrite-and-re-execute of a page another vCPU might be concurrently running from)
   made no measurable difference either. Both changes are kept, on their own independent merits, but
   neither is the fix.

   (2) Two cores crashed under the kernel's own test suite's real scheduler workload. FIXED
   2026-08-25, by the lane that took this finding. The symptom was a `script/test` run at
   `NIFE_SMP=2` faulting at `rip 0x0` or at `rip 0x5afe57ac5afe57ac`, which is `stack::PAINT`, the
   pattern this kernel writes into a fresh kernel stack. It reproduced 10 times in 10.

   The cause was a missing cross-core TLB shootdown, and it was neither in the INIT-SIPI-SIPI
   mechanism above nor in the portable scheduler. `mmu::flush_tlb` on this port was local to the
   calling CPU, because `invlpg` is; its own doc comment said exactly that and predicted exactly
   this ("this is the line that will need company"). aarch64 needs no remote half at all (`tlbi
   ..., is` is broadcast in hardware) and RISC-V has had one since milestone 58's RISC-V TLB shootdown,
   which is why the same `sched`/`thread` code runs clean at `-smp 4` on both. So a dead thread's
   kernel-stack address range, handed back by `KernelStack::drop` and remapped onto *different*
   frames for the next thread, was still translated by the other core to the old frame; a `Context`
   read back through that stale entry is whatever the recycled frame now holds, and a freshly
   recycled stack page is painted. Confirmed before any fix was written, by making a stale entry
   harmless (never reusing stack address space): the fault vanished, 8 runs of 8.

   The fix is `mmu::shoot_down_others`, and it is an NMI rather than an ordinary IPI, which is
   the one design point worth carrying up here. `unmap_page` runs inside `KERNEL_MMU`, an
   `IrqSafeMutex`, so the sender and every other core running the same code have interrupts masked;
   a maskable IPI would deadlock the first time two cores spawned and reaped at once. That is the
   property notes/riscv-tlb-shootdown.md already names as load-bearing there ("a hart with S-mode
   interrupts masked still services it"), which RISC-V gets from M-mode and aarch64 does not need.
   On x86 the NMI is the only delivery `cli` cannot suppress.

   Verified rather than gated: `user::tests::an_asid_flush_reaches_the_other_cores`, the portable
   test milestone 58 wrote for this exact property, fails on this port without the shootdown and
   passes with it, and the suite reaches `test result: ok. 177 passed` at two cores once (3) below
   is stepped over. 20 further runs produced no paint fault.

   (3) The suite cannot agree on which core booted (found by the same lane while verifying (2),
   **not fixed**, and a different subsystem's bug). `arch::x86_64::boot_cpu_id` reads CPUID leaf 1's
   initial local APIC id, which answers *"which core am I"* rather than *"which core booted"*;
   aarch64 answers a constant `0` and RISC-V returns the hart id recorded at boot. So a test body
   that §28's placement has migrated onto a secondary reads that secondary's id as "the boot core",
   and `smp::tests::every_secondary_runs_scheduled_work` waits for a `RAN_ON` mark the real boot
   core never sets. `stack.rs`'s high-water report picks the slot to skip the same way, scans a
   never-painted one, and reports a secondary stack at 65536/65536. One cause, both symptoms, about
   half of two-core runs. It reproduces with no shootdown code present at all, checked against the
   pre-fix tree, so it predates (2)'s fix. The likely answer is a boot-time record, which is what
   RISC-V's `boot_hartid` already is. Small, wants no hardware, and wants a lane.

   Full account of all three: `arch::x86_64::ap_boot`'s own `BUGS`, which stays the authoritative
   record. `helpers/qemu-runner-x86_64.sh` keeps `NIFE_SMP` at 1 because (1) and (3) are open and
   either can fail a run, so the `smp` tests this item's roster work would otherwise unblock keep
   skipping, honestly, until those two are answered. Whether (1) is a QEMU TCG emulation quirk or a
   real bug in this port's own code is exactly the kind of question milestone 87's real hardware
   would settle, and it is the one still worth a lane of its own.
6. VT-d, x86_64's IOMMU. BUILT 2026-08-25. The same milestone-16b role the SMMUv3 and RISC-V
   IOMMU drivers already fill, this time over an interface that is register-driven rather than
   memory-queue-driven: VT-d has no command or fault queue in memory, so invalidation is a register
   write-and-poll (`CCMD_REG`) and faults are read out of a small bank of Fault Recording Registers
   instead. `arch::iommu::init` (`kernel/src/arch/x86_64/iommu.rs`) builds the root and context
   tables, turns translation on over an all-absent root so every bus faults until its context table
   exists (the same default-deny posture the other two drivers give their own table shapes), and
   wires into `main.rs`'s boot tour right after the fine page tables and before anything could
   attach a device: one DRHD (`machine_discovery::acpi::first_drhd`), translation enable/disable
   through `GCMD`/`GSTS`, register-based context-cache and IOTLB invalidation, and fault detection
   through `FSTS.PPF` and the first Fault Recording Register.

   **What is deliberately scoped out, per the driver's own `BUGS`:** exactly one DRHD is brought up,
   so a machine reporting more than one VT-d unit has devices this driver never sees; no interrupt
   remapping (`GCMD.IRE` is never set); invalidation is always global-granularity, never domain- or
   device-selective; `RWBF` (`CAP_REG` bit 4) is honored in code but has never actually been
   exercised, since QEMU's model does not set it; and the fault path decodes only the first Fault
   Recording Register, since QEMU reports `CAP.NFR = 0`. No PCI device is confined through it
   yet, because no virtio-pci or NVMe driver exists on `x86_64` (item 4's own hand-off), so
   `main.rs` proves the driver against the hardware's own status registers rather than against a
   downstream escape.

One thing that is not a step, and is now resolved rather than owed:

- A capability shape for port I/O. On the other two architectures a device is a page, so a device
  capability is a mapping and the MMU enforces it. x86's legacy devices, the console UART included,
  are in an I/O space with no page tables; the only mechanism with the right granularity is the TSS
  I/O permission bitmap, which is per-task rather than per-page. `user::UART_PHYS` is zero on this
  architecture and that zero is the marker.
  [DECISIONS §121](../decisions/0121-port-io-capability.md) (**DECIDED** 2026-08-25, "Ratify option 2,
  permanently") answered it: x86's legacy port-I/O devices, the console included, stay
  kernel-resident forever. No new capability object gets added; only memory-mapped devices get a
  userspace driver on this architecture, which is a permanent, recorded parity gap rather than an
  interim stance. The measured cost of the alternative supported closing it rather than leaving it
  open (the TSS I/O-bitmap write costs ~2,682 ns/context-switch, 423% overhead on the naive
  always-write implementation), and nothing on the customer path is affected: every device the Time
  Machine thesis touches (network, disk, everything SMB needs) is already memory-mapped PCI/PCIe and
  already gets the ordinary capability treatment identically on all three architectures. There is
  nothing further to build here.

Resolved: the two arch-contract names that did not stretch to a third architecture,
`arch::psci_cpu_on` and `kernel_main`'s `dtb` argument, were renamed `arch::cpu_start` and
`boot_info_pointer`; see `notes/x86-port.md` and `arch/x86_64/mod.rs`.
## Follow-on

- **Milestone 176.** Item 0's last piece, the CMOS RTC seam, is built rather than "DECIDED but not
  yet built": `kernel/src/arch/x86_64/rtc.rs` reads CMOS at boot and `clock_protocol`'s RTC seam
  carries the seed to the clock service.
- **Done.** `crates/paging` maps 2 MiB and 1 GiB leaves on all three CPU formats, and
  all three kernels' direct maps use them for RAM. See "Closing the follow-on" below.
- **Milestone 215.** Item 2's MSI half is built: an MSI intid is its vector, a `virtio-blk-pci`
  completion reaches a ring-3 driver, and the enable and acknowledge paths are correctly no-ops for
  one.
- **Refused.** Item 2's other half, PCI interrupt routing over INTx, was deliberately not taken by
  milestone 215: ACPI's `_PRT` is AML and this tree will not grow an interpreter, and hardcoding
  q35's swizzle would pass every gate here and might still fail on xenon.
  This refusal is milestone 459 (design/roadmap/0459-legacy-intx-interrupt-routing.md), which carries
  it with the condition that would change it.
- **Recorded.** Measured 2026-09-19; both bits stay off: the pinned QEMU flushes its whole TLB on
  every `CR3` write, so neither this tree's icount instrument nor plain TCG can see what PGE saves.
  `kernel/src/arch/x86_64/mmu.rs`'s `BUGS` and `notes/benchmarks/x86-instruments.md`'s 2026-09-19
  section hold the numbers and the trigger (KVM on cordoba, or xenon).
- **Milestone 186.** The bench tooling item 3 built has no caller: the x86 baseline and its check
  flag exist and CI's bench job runs two legs of three, which 186 tracks as one of its eleven gaps.
- **Milestone 164.** `fs_server` builds for `x86_64-unknown-none`. One build flag turned exit 101
  into exit 0; `redoxfs_server` and `mkfs` ride the x86_64 archive, and the 21 skips this item
  blamed on `aes` are gone.
- **Milestone 215.** The runner attaches a PCI device now: `helpers/qemu-runner-x86_64.sh` carries
  a `virtio-blk-pci` line with `iommu_platform=on` plus an `nvme` line, so the 15 skips that wanted
  an enumerated bus with something on it no longer have that cause.
- **Done.** Item 6's "no PCI device is confined through it yet" is closed by that same disk: it
  goes through `-device intel-iommu` with `iommu_platform=on`, and milestone 215 (a PCI function's interrupt reaches nothing on x86_64) records the IOMMU
  escape test running on x86_64 for the first time and passing.
- **Milestone 364.** The fixtures still missing from that runner (RedoxFS, GPT and blank disks,
  NIC, GPU, keyboard, RNG, and a transport-blind FS-server disk lookup) are
  `design/roadmap/0364-x86-64-test-fixtures.md`, which milestone 303 and a newer block have since
  taken over; that one is SUPERSEDED and says where each half went.
- **Milestone 184.** The skip that wanted an `x86_64-unknown-nife` target and a `std` farm is no
  longer milestone 27's tail; it has its own block.
- **Recorded.** The five UART skips stay skipped by design: DECISIONS §121 keeps x86's legacy port
  devices kernel-resident permanently, and the device capability init hands out in a positional
  slot stays a marked foot gun in `kernel/src/user.rs`.
- **Recorded.** VT-d's scoped-out limits hold as written in `kernel/src/arch/x86_64/iommu.rs`'s
  `BUGS`: one DRHD, no interrupt remapping, global-granularity invalidation only, `RWBF` never
  exercised, and only the first Fault Recording Register decoded.
- **Done.** SMP failure (1), fixed 2026-09-19, was a counting bug in `cpu_start`, not a core failing to
  start; `kernel/src/arch/x86_64/ap_boot.rs`'s `BUGS` #1 has the evidence.
- **Milestone 316.** SMP failure (3) was fixed there on 2026-09-17: `boot.s` stamps
  `BOOT_CPU_ID` once and `boot_cpu_id` reads it. Re-verified against this tree on 2026-09-19.
- **Milestone 315.** The x86_64 runner's `NIFE_SMP` default moves to 2 as that milestone's closing
  step (DECISIONS §153, answered 2026-09-18). Nothing in the SMP bring-up holds it at 1 any more.
- **Milestone 167.** The 1 GHz `cntfrq` fallback for children built through the supervision
  protocol is that block's scope, and it is still NOT-STARTED.

## Closing the follow-on (2026-09-19)

Each item was re-checked against the tree first, because the list above said "Checked 2026-09-03".
One had already been done by another milestone (SMP failure 3, milestone 316); the other three were
open as written.

1. Blocks in `crates/paging`. `PageFormat` grew two required methods, `block_entry` and
`is_block`, with no defaults so that a fourth format has to say what it does. `Mapper` gained
`map_block` and `map_span` (the largest leaf that fits, up to a ceiling the caller passes), and its
existing walks learned that a block is a leaf met early: `map` refuses to descend through one,
`unmap` refuses to take a page out of one (`MapError::InsideBlock`), and `translate` stops at one.
Nothing splits a block. `PageSize`, `map_block`, `map_span`, `InsideBlock` and `UnsupportedSize` are
**provisional names**. VT-d declines blocks (`UnsupportedSize`) until something reads `SLLPS`.

- Parity (rule 5): no scope note is needed. aarch64 and riscv64 had block mappings only in their
  boot maps (two 1 GiB blocks and six gigapages, hand-built in assembly and Rust respectively); their
  fine maps used 4 KiB leaves exactly as x86's did. All three formats now encode blocks, all three
  are proved, and all three kernels put RAM in blocks and devices in pages.
- Measured: x86_64 on QEMU's 256 MiB, 560 KiB of kernel page tables became 60 KiB; at 4 GiB,
  8,252 KiB became 64 KiB (1 GiB leaves) or 72 KiB (`-cpu qemu64`, no `Page1GB`, 2 MiB leaves).
- Proofs: eight new Kani harnesses, two over the leaf choice (soundness: a block never leaves its
  span; completeness: no larger fitting leaf is skipped) and two per format (the block encoding over
  every address and flag set; no table pointer reads as a block). No existing harness was changed.
  Five carry replayable falsification patches, swept red. notes/verification.md has the table.
- **Recorded, not fixed:** the kernel does not read the MTRRs, so a block over RAM relies on firmware
  keeping that RAM one memory type (`mmu.rs`'s BUGS).
- The fastpath is byte-identical to `main` on all three architectures, measured after merging
  it: aarch64 5,356 / 7,028 / 1,504, riscv64 4,644 / 5,948 / 1,870, x86_64 6,256 / 8,190 / 1,701,
  the same figures `script/fastpath-footprint` reports for `main` alone. On the pre-merge tree the
  block check the walk now makes cost aarch64's `syscall_entry` four bytes (1,504 to 1,508, measured
  by neutralizing the check and remeasuring); merging `main` moved the inlining and it is gone.
  The gate's riscv64 and x86_64 `syscall_entry` figures sit above `bench/fastpath-*.txt` and did so
  before this lane, so that drift is `main`'s and is left for whoever re-saves those baselines.

2. `CR4.PGE` and `CR4.PCIDE`. Measured with an initrd attached, so the EL0 benches that switch
`CR3` actually ran on x86_64: icount tick counts were byte-identical with PGE on and off on every
line, and plain TCG timing overlapped within 1%. Both are the expected result, because the pinned
QEMU flushes its whole TLB on every `CR3` write whatever the two bits say, and icount counts
instructions, which a TLB refill is not. **So the bits stay off and the question is not answered.**
No `script/icount` x86 leg was built: milestone 78's instrument asserts deadline-timer claims this
port's periodic LAPIC timer cannot make, and a leg would measure instructions, which is the one
thing already known not to move. Milestone 186's bench gap (CI running two legs of three) is
untouched. What would answer it is KVM on cordoba, one `usermod` away, or xenon. PCIDE is a design
(INVPCID on three flush paths plus a tag-reuse rule), not a bit, and `mmu.rs` says why.

3. SMP failure (1). Reproduced at `-smp 4` on the first run: 26 of 40 plain boots reported a
core "did not start", and the failing core had printed its own `cr4.smep` line from
`secondary_main` first. An instrumented build confirmed it in three failures of three: `cpu_start`
re-read the online count after the STARTUP IPIs and waited for it to move again, so a core that
checked in during the 200 µs settle delay was counted absent. Fixed by reading the count once,
before the INIT. After: 40 of 40 at `-smp 4`, 20 of 20 at `-smp 8`, 20 of 20 at `-smp 3`,
all cores online. It could reach two cores as well, which is plausibly the UEFI leg's one-in-three
(milestone [412](0412-the-uefi-boot-gate-asserts-two-cores-that-do-not-always-start.md), whose
question 2 this answers: `ap_boot`'s "third or later" bound was wrong. Not measured here.)

4. SMP failure (3). Already fixed by milestone 316 (`BOOT_CPU_ID`); verified in the tree.

Can the x86_64 runner default to more than one core? Not from this milestone. Both bring-up bugs
are closed, and DECISIONS §153 makes the flip milestone 315's closing step, after its port-revoke
broadcast makes `a_revoked_holder_faults_on_its_next_port_write` reliable at two cores.

## Index row

Splits real work out of milestone 20's stale text: DECISIONS §19 declared x86_64 a target,
milestone 20's own "Deliverable, in two parts" named it, but 20 is BUILT for the HAL split and
RISC-V alone (`kernel/src/arch/` has no `x86_64/`). BUILT 2026-09-19. The fine map, IO APIC,
ring 3, real processes, VT-d, a real userspace and SMP bring-up are built; the last four items closed
together: 2 MiB and 1 GiB leaves in `crates/paging` on all three formats, proved, taking x86's
direct map from 560 KiB of tables to 60 KiB; SMP failure (1), which was `cpu_start` counting a
started core as absent (26 of 40 four-core boots before, 80 of 80 at three to eight cores after);
failure (3), fixed by milestone 316; and `CR4.PGE`/`PCIDE`, measured and left off because the pinned
QEMU flushes its TLB on every `CR3` write, so no instrument here can see them. The two-core runner
default is milestone 315's closing step.
