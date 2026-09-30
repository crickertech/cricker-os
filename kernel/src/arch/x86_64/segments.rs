//! **The GDT and the TSS**, which have no counterpart at all on the other two architectures.
//!
//! aarch64 and RISC-V have no segmentation. `x86_64` has *almost* none: in 64-bit mode a segment's
//! base and limit are ignored for everything except `fs` and `gs`, so the descriptors below carry
//! essentially no information. What they still carry is the three bits that decide **what mode the
//! CPU is in**: the code segment's `L` bit (64-bit), `DPL` (which ring), and `D/B`. So the GDT
//! cannot be dispensed with even though almost every field in it is dead.
//!
//! The TSS is the part that does real work, and its job is the one x86 gives no other mechanism for:
//! **where does the stack pointer come from when the CPU changes ring?** On aarch64 an exception
//! from EL0 lands on `SP_EL1`, a separate banked register the kernel has already set. On RISC-V the
//! trap handler recovers its stack from `sscratch`. x86 reads `TSS.RSP0` out of memory, so a task
//! that traps in from ring 3 gets whatever `RSP0` says, and if it says something wrong the very
//! first user-mode fault is unrecoverable. The IST entries are the same mechanism with a stronger
//! promise: an IST vector switches stacks **unconditionally**, even for a ring-0 trap, which is the
//! only way to survive a fault whose cause is that the current stack is unusable.
//!
//! # The selector layout is not free
//!
//! `syscall`/`sysret` derive four selectors from two 16-bit fields of `IA32_STAR`, by arithmetic:
//! entering the kernel takes CS = `STAR[47:32]` and SS = that + 8, and returning to user takes
//! CS = `STAR[63:48]` + 16 and SS = `STAR[63:48]` + 8. So the order below (kernel code, kernel data,
//! user **data**, user **code**) is forced by the instruction, and the apparently-backwards user
//! pair is the tell. Reordering these to look tidier breaks `sysret` in a way that shows up as a
//! general protection fault on the way back to a program that did nothing wrong.

use core::arch::asm;

/// Selector for the kernel code segment. Also `IA32_STAR[47:32]`, from which `syscall` derives the
/// kernel SS as this + 8.
pub const KERNEL_CODE: u16 = 0x08;
/// Selector for the kernel data segment (ss/ds/es while in ring 0).
pub const KERNEL_DATA: u16 = 0x10;
/// Selector for the user data segment. `sysret` computes this as `IA32_STAR[63:48] + 8`.
#[allow(dead_code)] // Referenced once user mode exists; see the module header for why it is here now.
pub const USER_DATA: u16 = 0x18 | 3;
/// Selector for the user code segment. `sysret` computes this as `IA32_STAR[63:48] + 16`.
#[allow(dead_code)]
pub const USER_CODE: u16 = 0x20 | 3;
/// Selector for the TSS descriptor, which occupies **two** GDT slots (0x28 and 0x30) because a
/// 64-bit system descriptor is 16 bytes rather than 8.
const TSS_SELECTOR: u16 = 0x28;

/// The IST slot the double-fault handler runs on. **One-based**: the IDT encodes 0 as "do not
/// switch stacks", so slot 1 is `TSS.ist[0]`.
pub const IST_DOUBLE_FAULT: u8 = 1;

/// The x86 I/O port space is 16 bits (`in`/`out` address exactly 64 Ki ports), one bit of "may this
/// ring-3 thread touch this port" each, so a full permission bitmap is `65536 / 8 == 8192` bytes.
const IOMAP_BYTES: usize = 65536 / 8;

/// One trailing guard byte, set to all ones, because the CPU may read **two** bytes of the bitmap
/// when it checks the highest port and a bitmap that ended exactly at the limit would read one byte
/// past it. Linux appends the same byte for the same reason. The `Tss` descriptor's limit covers it.
const IOMAP_GUARD: usize = 1;

/// Offset, in bytes from the base of a [`Tss`], to its I/O permission bitmap. This is the value
/// [`Tss::iomap_base`] holds while a thread that owns ports is running: everything up to it is the
/// fixed 104-byte TSS, and the bitmap follows.
const IOMAP_OFFSET: u16 = 104;

/// What [`Tss::iomap_base`] holds while **no** thread with a port capability is running: a value
/// past the descriptor's limit, which the CPU reads as "there is no bitmap", so every `in`/`out`
/// from ring 3 faults. This is the lazy form DECISIONS §121 named (2026-08-25, binding here): a
/// thread that holds no port capability switches with `iomap_base` pointing past the segment limit,
/// and the bitmap is written into the TSS only when a holder is on a side of the switch. `0xFFFF`
/// is past the limit for any TSS this kernel builds (the descriptor limit is `size_of::<Tss>() - 1`,
/// far below 64 Ki).
const IOMAP_BASE_DENY_ALL: u16 = 0xFFFF;

/// A 64-bit Task State Segment, with its I/O permission bitmap.
///
/// Almost every field of the 32-bit TSS is gone in long mode; what remains is three ring stacks,
/// seven interrupt stacks, the I/O permission bitmap offset, and the bitmap itself. `#[repr(C,
/// packed)]` because the layout is the CPU's, not Rust's, and the reserved words are load-bearing
/// padding rather than slack.
///
/// **The bitmap lives inside the TSS on purpose** (milestone 299): `iomap_base` is an offset *from
/// the TSS base*, and the CPU reads the bitmap out of the same segment, so the permission bits have
/// to be contiguous with the rest of the structure rather than a separate allocation. One per core,
/// like the rest of the TSS, so two cores never share the bits that say what ports the thread on
/// each may reach.
#[repr(C, packed)]
struct Tss {
    _reserved0: u32,
    /// The stack pointer the CPU loads on a trap that raises privilege to ring 0. This is the field
    /// that makes user mode survivable.
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    _reserved1: u64,
    /// Seven unconditional stacks, selectable per IDT entry. `ist[0]` is what
    /// [`IST_DOUBLE_FAULT`] names.
    ist: [u64; 7],
    _reserved2: u64,
    _reserved3: u16,
    /// Offset from the base of this TSS to the I/O permission bitmap ([`IOMAP_OFFSET`]) while a
    /// thread that owns ports runs, or [`IOMAP_BASE_DENY_ALL`] (past the limit, "no bitmap") while
    /// none does. [`set_port_range_grant`] is the only writer after boot; the initial value denies all,
    /// the only correct answer while no program has been granted a port. See DECISIONS §121.
    iomap_base: u16,
    /// The permission bitmap plus its guard byte. A **set** bit denies the port to ring 3; a
    /// **clear** bit permits it. The invariant [`set_port_range_grant`] maintains is that this is entirely
    /// ones (all denied) whenever no port grant is installed on this core, so installing one is a
    /// matter of clearing the grant's own bits and uninstalling is setting them back.
    iomap: [u8; IOMAP_BYTES + IOMAP_GUARD],
}

impl Tss {
    const fn new() -> Self {
        Self {
            _reserved0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            _reserved1: 0,
            ist: [0; 7],
            _reserved2: 0,
            _reserved3: 0,
            // No holder at boot: no bitmap is consulted. The bitmap below is still all-ones so the
            // "all denied when nothing is installed" invariant holds the instant a holder installs.
            iomap_base: IOMAP_BASE_DENY_ALL,
            iomap: [0xFF; IOMAP_BYTES + IOMAP_GUARD],
        }
    }
}

const _: () = assert!(size_of::<Tss>() == 104 + IOMAP_BYTES + IOMAP_GUARD);
// `IOMAP_OFFSET` is the byte offset of the bitmap; the fixed part of a 64-bit TSS is 104 bytes.
const _: () = assert!(core::mem::offset_of!(Tss, iomap) == IOMAP_OFFSET as usize);

/// **One TSS per core** (milestone 161's SMP item, fixing exactly the limitation this static's own
/// doc used to name: "every core needs its own, since `rsp0` names a per-core stack"). Indexed by
/// `cpu::id()`, the same way every other per-core array in this kernel is; `TSS.rsp0` is what makes
/// this matter, since sharing one across cores would let two cores race on the same ring-0 stack
/// pointer the instant both took a trap from ring 3.
static mut TSS: [Tss; crate::cpu::MAX_CPUS] = [const { Tss::new() }; crate::cpu::MAX_CPUS];

/// The GDT: seven 8-byte slots, the last two of which are one 16-byte TSS descriptor. **One per
/// core**, for the same reason as [`TSS`]: the TSS descriptor's base address is this core's own
/// `TSS[cpu::id()]`, so sharing one GDT would mean every core's `ltr` loaded the SAME task
/// register, aliasing every core's ring-0 stack onto whichever one initialized last.
static mut GDT: [[u64; 7]; crate::cpu::MAX_CPUS] = [BASE_GDT; crate::cpu::MAX_CPUS];

/// Every core's GDT starts identical: only the TSS descriptor (slots 5 and 6) differs per core, and
/// [`init`] fills those in from this core's own [`TSS`] entry.
const BASE_GDT: [u64; 7] = [
    0,                     // 0x00: the mandatory null descriptor
    0x00AF_9A00_0000_FFFF, // 0x08: kernel code, DPL 0, L=1
    0x00CF_9200_0000_FFFF, // 0x10: kernel data, DPL 0
    0x00CF_F200_0000_FFFF, // 0x18: user data, DPL 3
    0x00AF_FA00_0000_FFFF, // 0x20: user code, DPL 3, L=1
    0,                     // 0x28: TSS descriptor, low half, filled in by `init`
    0,                     // 0x30: TSS descriptor, high half
];

/// The operand `lgdt` and `lidt` take: a 16-bit limit (size minus one) and a 64-bit base.
/// `#[repr(C, packed)]` because the CPU reads exactly ten bytes and a Rust-chosen alignment hole
/// between the two fields would make it read the base from the wrong offset.
#[repr(C, packed)]
pub struct DescriptorTablePointer {
    pub limit: u16,
    pub base: u64,
}

/// Install the GDT and the TSS on this CPU, and reload every segment register from it.
///
/// # Why the code-segment reload is a return and not a jump
///
/// `lgdt` changes the table but not the selectors already loaded, and `mov cs, ax` does not exist:
/// CS can only be changed by a control transfer. The idiomatic 64-bit way is a far return: push the
/// new selector and a target address, then `retfq`, which pops both. A far *jump* would work too but
/// has to go through memory (`ljmp [mem]`) in 64-bit mode, which needs a scratch location; the
/// stack is already there.
///
/// # Safety
/// Must be called once per CPU, with a valid stack and `cpu::init_this_cpu` already run on it
/// (this fills `TSS`/`GDT` at `cpu::id()`'s own slot, and restores this core's per-CPU pointer
/// afterward; see below), before anything depends on the GDT: it replaces the boot GDT that
/// `boot.s` installed, and the old one goes away.
pub unsafe fn init() {
    let id = crate::cpu::id();

    // The TSS descriptor cannot be a constant: it holds the TSS's own address, which is only known
    // once the image is linked and loaded. A 64-bit system descriptor spreads that address across
    // three disjoint fields, which is a layout inherited from the 16-bit 286 and is why this looks
    // the way it does rather than being a single store.
    // SAFETY: indexing this core's own slot with `id` in bounds (`cpu::id()` cannot exceed
    // `MAX_CPUS`, which is exactly how big `TSS` is); reading only the address, not the (possibly
    // concurrently-written-by-another-core-at-a-different-index) contents.
    let base = unsafe { (&raw const TSS[id]) as u64 };
    let limit = (size_of::<Tss>() - 1) as u64;

    // Hand trap.s the address of the one field it writes: this core's OWN `TSS[id].rsp0`, through
    // the per-CPU block `gs` already names, so there is nowhere a second core's write could land.
    // Before the IDT exists, so no trap can have taken the path that reads it. See
    // `cpu::PerCpu::x86_trap` / `cpu::X86TrapPerCpu::tss_rsp0_ptr`.
    // SAFETY: writes this core's own `PerCpu` slot, reached the same way every per-CPU write on
    // this architecture is, before any trap can be taken on this core.
    unsafe {
        *crate::cpu::current().x86_trap.tss_rsp0_ptr.get() = (&raw const TSS[id].rsp0) as u64;
    }
    let low = (limit & 0xffff)
        | ((base & 0x00ff_ffff) << 16)
        | (0x89 << 40)                    // present, type 9 = available 64-bit TSS
        | (((limit >> 16) & 0xf) << 48)
        | (((base >> 24) & 0xff) << 56);
    let high = base >> 32;

    // SAFETY: this core writes only its OWN slot (`id`), before anything else on this core reads
    // its GDT; a different core's `init` writes a different slot.
    unsafe {
        GDT[id][5] = low;
        GDT[id][6] = high;
    }

    // SAFETY: as `base` above, this core's own slot only.
    let gdt_base = unsafe { (&raw const GDT[id]) as u64 };
    let gdtr = DescriptorTablePointer {
        limit: (size_of::<[u64; 7]>() - 1) as u16,
        base: gdt_base,
    };

    // **Loading a segment register in long mode DESTROYS that segment's base MSR**, and `gs`'s base
    // is where this kernel keeps its per-CPU pointer (the analog of aarch64's `TPIDR_EL1` and
    // RISC-V's `tp`). `cpu::init_this_cpu` has already run by the time anything calls this, because
    // the console lock reads the per-CPU block, so the `mov gs, ax` below would silently zero the
    // pointer and the very next `println!` would dereference null.
    //
    // That is exactly what happened during bring-up, and it is worth recording how it presented:
    // not as a null dereference but as an instruction fetch from the middle of a static, several
    // frames away, with the register dump showing a perfectly correct GDT, TSS and IDT. Nothing
    // about the symptom pointed here.
    //
    // So the base is saved and put back around the reload. Doing it here rather than making the
    // caller re-arm the pointer keeps the ordering constraint from existing at all, which is the
    // ladder's first rung: the wrong state is not representable rather than merely documented.
    // `fs` gets no such treatment because nothing uses its base; if anything ever does, it needs the
    // same two lines.
    let gs_base = super::percpu();

    // SAFETY: `gdtr` describes the table above, which is well-formed by construction. The far
    // return reloads CS from a descriptor whose L bit is set, so the CPU stays in 64-bit mode; the
    // data selectors are reloaded before anything can use a stale one.
    unsafe {
        asm!(
            "lgdt [{gdtr}]",
            // Push the new CS and the address to continue at, then far-return into it.
            "push {code:r}",
            "lea {tmp}, [rip + 2f]",
            "push {tmp}",
            "retfq",
            "2:",
            "mov ds, {data:e}",
            "mov es, {data:e}",
            "mov ss, {data:e}",
            "mov fs, {data:e}",
            "mov gs, {data:e}",
            "ltr {tss:x}",
            gdtr = in(reg) &gdtr,
            code = in(reg) KERNEL_CODE as u64,
            data = in(reg) KERNEL_DATA as u32,
            tss = in(reg) TSS_SELECTOR,
            tmp = lateout(reg) _,
            options(preserves_flags),
        );
    }

    // Put the per-CPU pointer back; see the comment above the `asm!`.
    super::set_percpu(gs_base);
}

// Point this CPU's ring-0 trap stack at `top`. DELETED 2026-09-30 by the CI-warnings lane:
// swish-check's x86_64 leg warned `set_kernel_stack` (and the `exceptions::set_syscall_kernel_stack`
// half it existed to keep in step) was never used in any configuration, because `isr_restore` in
// trap.s writes `TSS.RSP0` and the syscall path's slot together on every return to ring 3. The
// property this pair existed for, "a trap and a `syscall` cannot name different stacks", is held
// by that one code path; on the other two architectures the same job is `sscratch.kernel_sp`
// (RISC-V) or nothing at all (aarch64 banks `SP_EL1`).

/// Point an IST slot (1-based, as the IDT encodes it) at `top`, on this core.
///
/// # Safety
/// `top` must be the top of a stack no other IST vector shares. Two vectors on one stack means a
/// fault taken while handling the other overwrites its frame, which is precisely the situation IST
/// exists to survive.
pub unsafe fn set_interrupt_stack(slot: u8, top: u64) {
    assert!((1..=7).contains(&slot), "IST slots are 1..=7, not {slot}");
    let id = crate::cpu::id();
    // SAFETY: writes this CPU's own TSS `ist[slot - 1]`, in bounds by the assertion above. Through
    // a raw pointer because the TSS is `packed` and a reference to a field would be misaligned.
    unsafe {
        (&raw mut TSS[id].ist)
            .cast::<u64>()
            .add(slot as usize - 1)
            .write_unaligned(top);
    }
}

/// **Which port grant is installed in this core's TSS bitmap right now**, or `None` for the
/// overwhelming majority of cores that never run a port-holding thread. Indexed by `cpu::id()`, the
/// same per-core discipline the TSS and GDT arrays use.
///
/// This is the state that makes the switch cheap: [`set_port_range_grant`] compares the incoming thread's
/// grant against it and does nothing when they match, which is every switch on a core where nothing
/// holds a port (both sides `None`) and every switch that keeps the same holder running.
static mut INSTALLED_PORT_GRANT: [Option<(u16, u16)>; crate::cpu::MAX_CPUS] =
    [None; crate::cpu::MAX_CPUS];

/// Set every bit of ports `[base, base + count)` in `iomap` to `deny` (`true` = the port faults from
/// ring 3, `false` = the port is permitted). A port's bit is bit `port % 8` of byte `port / 8`. The
/// range is clamped to the real bitmap so a grant near the top of the port space cannot touch the
/// guard byte or run off the end; `count == 0` writes nothing.
///
/// `iomap` is a raw pointer to the bitmap's first byte rather than a `&mut [u8; _]`, because the
/// array is a field of a `#[repr(C, packed)]` `static mut` and taking a reference into it is exactly
/// the shape the rest of this file avoids; a pointer plus an in-bounds index is the honest form.
///
/// # Safety
/// `iomap` must point at a live `[u8; IOMAP_BYTES + IOMAP_GUARD]` (a core's own TSS bitmap), and the
/// caller must hold it exclusively (interrupts masked, this core's own slot). The index never
/// reaches the guard byte, because it is clamped to `IOMAP_BYTES * 8` ports.
unsafe fn write_range_bits(iomap: *mut u8, base: u16, count: u16, deny: bool) {
    for port in (base as usize)..(base as usize + count as usize).min(IOMAP_BYTES * 8) {
        let bit = 1u8 << (port % 8);
        // SAFETY: `port / 8 < IOMAP_BYTES`, so this is inside the array `iomap` points at; the
        // caller guarantees exclusive access to it.
        unsafe {
            let byte = iomap.add(port / 8);
            if deny {
                *byte |= bit;
            } else {
                *byte &= !bit;
            }
        }
    }
}

/// **Install a thread's port grant into this core's TSS, the lazy way** (milestone 299, DECISIONS
/// §121 reversed 2026-09-15). `sched::schedule` calls this on switch-in with the incoming thread's
/// `(base, count)` grant, or `None` if it holds no port capability.
///
/// The cost this pays, and the cost it does not, are the whole design:
///
/// - **When the grant is unchanged, it returns after one comparison.** Both sides `None` (a switch
///   between two ordinary threads, which is nearly every switch on nearly every machine) costs a
///   load and a branch and writes nothing. This is what makes the enforcement free for the threads
///   that do not use it, the property §121's 2026-08-25 refinement said the naive always-write
///   (~2,682 ns/switch) threw away.
/// - **When it changes, it writes only the bits that move**, not the whole 8 KiB bitmap: it re-denies
///   the outgoing holder's range (restoring the all-ones invariant for those bytes) and permits the
///   incoming holder's, then points `iomap_base` at the bitmap (a holder is on the CPU) or past the
///   limit (none is). For a 16550's eight ports that is one byte plus the `iomap_base` word.
///
/// No `ltr` re-issue is needed: the CPU reads `iomap_base` and the bitmap out of the TSS in memory
/// on each `in`/`out`, so writing them takes effect on the next port access. The caller runs with
/// interrupts masked (it is on the switch path), and this touches only this core's own TSS, so the
/// writes cannot race a port access on this core or a TSS write on another.
///
/// **`#[cold]`, because its effect is rare even though it is called on every switch.** On a machine
/// where one process holds a port capability, the early return is taken on all but a handful of
/// switches, and marking the function cold keeps its body (and `write_range_bits`) out of the IPC
/// fastpath's hot instruction footprint (`script/fastpath-footprint`, which follows non-cold calls
/// out of `schedule()`'s switch), for the price of a call and a compare on the common switch.
#[cold]
pub fn set_port_range_grant(grant: Option<(u16, u16)>) {
    set_port_range_grant_on(crate::cpu::id(), grant);
}

/// [`set_port_range_grant`] for a core that names itself by number rather than through `gs`.
///
/// **The NMI half of a revocation broadcast cannot call `cpu::id()`**, which reads `IA32_GS_BASE`,
/// and `mmu::serve_shootdown_nmi`'s own doc has the reason: an NMI can land in the window where that
/// MSR still holds the *user's* value while `cs` says ring 0. The handler names its core from the
/// local APIC id register instead, which is hardware ground truth, and hands it here. On this port
/// the two numbers are the same (`smp::seat_cpus_from_acpi` seats every core at the slot its own
/// APIC id names), and the sender `debug_assert`s it rather than assuming it.
///
/// # The one rule that makes every writer of a TSS bitmap safe
///
/// **A core's port bitmap is written only by a thread holding `sched::IPC_TABLES`, or by an NMI that
/// such a thread sent.** That is what stops the revocation broadcast landing inside a switch-path
/// install on the target core and leaving the two writers interleaved: while the revoker holds the
/// lock, no other core can be inside `sched::install_port_grant`, because milestone 315 (a port
/// revoke that reaches every core) moved that
/// call inside the locked region for exactly this reason. The one exception is `bench_*` below,
/// which runs in a `feature = "bench"` boot that pins a single hart and sends nothing.
#[cold]
fn set_port_range_grant_on(id: usize, grant: Option<(u16, u16)>) {
    // SAFETY: this core's own slot, read and written only here and only with interrupts masked on
    // the switch path; a different core touches a different index.
    let installed = unsafe { INSTALLED_PORT_GRANT[id] };
    if installed == grant {
        return;
    }
    // This core's own TSS bitmap, as a raw pointer to its first byte: the CPU reads it only on a
    // ring-3 `in`/`out`, which cannot happen while this runs with interrupts masked on this core, and
    // a pointer avoids taking a reference into the `packed` `static mut`.
    // SAFETY: forming a raw pointer into this core's own TSS slot; no reference is taken.
    let iomap = unsafe { (&raw mut TSS[id].iomap).cast::<u8>() };
    if let Some((base, count)) = installed {
        // SAFETY: this core's own bitmap, held exclusively (interrupts masked, own slot).
        unsafe { write_range_bits(iomap, base, count, true) }; // restore the outgoing holder's deny bits
    }
    let base_value = match grant {
        Some((base, count)) => {
            // SAFETY: as above.
            unsafe { write_range_bits(iomap, base, count, false) }; // permit the incoming holder's ports
            IOMAP_OFFSET
        }
        None => IOMAP_BASE_DENY_ALL,
    };
    // SAFETY: as above; `iomap_base` is a `u16` in the same packed TSS, written through a raw
    // pointer because a reference to a packed field would be misaligned.
    unsafe { (&raw mut TSS[id].iomap_base).write_unaligned(base_value) };
    // SAFETY: this core's own slot.
    unsafe { INSTALLED_PORT_GRANT[id] = grant };
}

/// **Reset this core's TSS if it currently grants exactly `(base, count)`** (milestone 299): the
/// core-local half of revocation. If the running thread's installed grant is the range being
/// revoked, re-deny its bits and point `iomap_base` past the limit, so a port access faults even
/// before the next context switch. If a different grant (or none) is installed, this does nothing.
///
/// This is what [`sched::delete_current_cap`](crate::sched::delete_current_cap) needs and the whole
/// of what it needs: the thread dropping its own port capability is the thread whose grant is
/// installed *here*, and a core's bitmap only ever permits the range for the thread currently
/// running on it, so there is no second core to tell. A `PortRange::REVOKE` names a holder that may
/// be running anywhere and calls [`revoke_port_grant_everywhere`] instead.
pub fn revoke_installed_port_grant(base: u16, count: u16) {
    revoke_installed_port_grant_on(crate::cpu::id(), base, count);
}

/// [`revoke_installed_port_grant`] for a core that names itself by number. The receiving half of
/// [`revoke_port_grant_everywhere`], called from the NMI handler; see [`set_port_range_grant_on`]
/// for why the handler cannot ask `cpu::id()` who it is.
pub(super) fn revoke_installed_port_grant_on(id: usize, base: u16, count: u16) {
    // SAFETY: this core's own slot; see `set_port_range_grant_on`.
    if unsafe { INSTALLED_PORT_GRANT[id] } == Some((base, count)) {
        set_port_range_grant_on(id, None);
    }
}

/// **Take the grant out of every online core's TSS, and do not return until they have done it**
/// (milestone 315). `PortRange::REVOKE`'s arch half.
///
/// A revoked holder can be *running on another core* at the instant the revoker deletes its
/// capability, and that core's bitmap still permits the range: the cached grant
/// (`thread::Thread::port_range_grant`) is cleared under `sched::IPC_TABLES`, but nothing had told
/// the hardware. The audit in milestone 313 (the security audit that was due since August) accepted
/// that as a window of at most one tick, on the
/// reasoning that the next context switch on that core closes it and it cannot reopen. **It was not
/// theoretical.** At `NIFE_SMP=2` the suite's own
/// `user::x86_port_tests::a_revoked_holder_faults_on_its_next_port_write` went red in 7 of 12 full
/// runs (the campaign in milestone 316 (which core booted)) and 5 of 30 filtered ones, every
/// failure the same shape: the
/// child's `out` was permitted and it exited cleanly where the test demands a fault. The instrument
/// that named the cause was a snapshot of [`INSTALLED_PORT_GRANT`] taken at the revoke: on both
/// captured failures it read "revoker on cpu 0, cpu 1 holds a grant".
///
/// The local reset comes first, so this core's own bitmap is right before anyone else is told to
/// look, which is the order [`super::mmu::flush_tlb`] uses for the same reason. The remote half
/// rides the TLB shootdown's NMI, and that choice is forced rather than preferred: see
/// notes/x86-tlb-shootdown.md for why an ordinary IPI deadlocks against a core spinning for a lock
/// with interrupts masked, which is exactly what a core waiting for `IPC_TABLES` is doing while the
/// revoker holds it.
///
/// **`spare_this_core` is the take-back's asymmetry reaching the hardware** (2026-09-24 security
/// audit). `PortRange::REVOKE` deletes the range from every table *but the invoker's*, and the
/// invoker is by definition the thread running on this core, so this core's installed bitmap is
/// the invoker's own grant whenever it names the range. Until that audit the local reset ran
/// unconditionally: a boot-endowed holder that took its range back from everyone else lost its own
/// bitmap until its next switch-in, and its next `out` faulted it to its supervisor. Fails closed,
/// and latent, because nothing in the tree invokes the take-back yet; a whole-machine sweep
/// (`keeper == None`) passes `false` and resets here too. The remote half is unaffected either way:
/// no other core can be running the invoker.
pub fn revoke_port_grant_everywhere(base: u16, count: u16, spare_this_core: bool) {
    if !spare_this_core {
        revoke_installed_port_grant(base, count);
    }
    super::mmu::revoke_port_grant_others(base, count);
}

/// The grant this core's TSS currently permits, if any: `(base, count)` as
/// [`set_port_range_grant`] installed it. Read for tests and diagnostics; the switch path keeps its
/// own copy in the thread.
pub fn installed_port_grant() -> Option<(u16, u16)> {
    // SAFETY: this core's own slot, read only; a different core reads a different index.
    unsafe { INSTALLED_PORT_GRANT[crate::cpu::id()] }
}

/// **Bench-only** (DECISIONS §121's amendment, 2026-08-24): the I/O permission bitmap option 1
/// would write into the current CPU's TSS on every switch-in, sized for the whole port space,
/// **not installed as live**.
///
/// The x86 port space is 16 bits (`in`/`out` address exactly 64 Ki ports), one bit of "may this
/// ring-3 thread touch this port" each, so the bitmap is `65536 / 8 == 8192` bytes exactly (the
/// module doc's "8 KiB" is not a round number picked for convenience; it is what the architecture
/// requires). Some real implementations (Linux's, e.g.) append one further guard byte set to all
/// ones, because the CPU may read two bytes when checking the highest port and a bitmap that ends
/// exactly at the limit would read past it; that byte is not needed here because nothing ever
/// checks this array against a real port access (see below).
///
/// **What this measures, and what it does not.** §121's option 1 would extend the live [`Tss`]
/// with this array, point `iomap_base` at it, and have the scheduler overwrite it on every
/// switch-in. This does the write (the dominant cost the amendment names) without any of the
/// rest: `iomap_base` above still points past the end of `TSS`, `ltr` is never re-issued, and no
/// `in`/`out` from ring 3 ever runs in this benchmark boot (there is no ring-3 program on this
/// port yet; see `user::x86_programs`). So the number this produces is the cost of an 8 KiB
/// per-CPU memory write on the switch path, not a proof that the bitmap enforces anything; that
/// second half is option 1's real implementation, out of scope here (`design/decisions/121-port-io-capability.md`).
///
/// (Milestone 299 built that real implementation; this bench's naive always-write stays as the
/// upper-bound baseline the lazy `tss_iomap_lazy_switch`/`tss_iomap_lazy_nop` read against. It reuses
/// the production [`IOMAP_BYTES`] rather than redefining it.)
#[cfg(feature = "bench")]
#[repr(C, align(8))]
struct BenchIoBitmap([u8; IOMAP_BYTES]);

/// A second CPU-owned 8 KiB region, separate from the live `TSS` static above so this benchmark
/// can never be mistaken for having wired the real one in. One instance because this port has one
/// CPU (`smp::bring_up_secondaries` is a refusal on `x86_64` today); a real per-CPU version would
/// need `crate::cpu::PerCpu`, which option 1 would also need and this benchmark does not.
#[cfg(feature = "bench")]
static mut BENCH_IOMAP: BenchIoBitmap = BenchIoBitmap([0; IOMAP_BYTES]);

/// Write a full I/O permission bitmap's worth of bytes into this CPU's bench-only scratch region,
/// the way `schedule()` would write the incoming thread's bitmap into the TSS under option 1. The
/// pattern argument (varied per call by the caller) and the touch after are both there so nothing
/// about this write is provably dead code to the optimizer.
///
/// Safe to call from anywhere: the internal `unsafe` is this CPU's own static, single-hart, no
/// concurrent access, the same fact `bench.rs`'s single-threaded caller already relies on.
#[cfg(feature = "bench")]
pub fn bench_write_io_bitmap(pattern: u8) -> u8 {
    // SAFETY: single-hart bench boot; writes and reads this CPU's own static, never aliased.
    unsafe {
        let base = (&raw mut BENCH_IOMAP.0).cast::<u8>();
        base.write_bytes(pattern, IOMAP_BYTES);
        base.add(IOMAP_BYTES - 1).read()
    }
}
