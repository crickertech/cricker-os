//! **The IDT and the trap path, `x86_64`.** The third implementation of the arch contract aarch64
//! states with `VBAR_EL1` and RISC-V with `stvec`.
//!
//! The shape of the difference is worth naming, because it is what the assembly beside this file
//! exists to paper over. aarch64 has one vector base and sixteen 128-byte slots the hardware picks
//! between; RISC-V has one entry point and a cause register. x86 has 256 independent entry points
//! and tells the handler nothing, so the vector number has to be manufactured by having 256
//! different stubs (trap.s), and ten of the 32 architectural exceptions push an extra word the other
//! 246 vectors do not.
//!
//! What the rest of the kernel sees through this module is the same four things it sees on the
//! other two: a [`TrapFrame`], counters it can assert on, the last user fault, and an `init` that
//! makes faults reportable instead of fatal.

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use super::segments::{IST_DOUBLE_FAULT, KERNEL_CODE};
use crate::arch::UserFault;

/// **The registers a trap saves, in exactly the order trap.s pushes them.**
///
/// The first fifteen fields are the general registers, lowest address first, which is the *reverse*
/// of the push order because the stack grows down. Then the vector number and error code the stub
/// manufactured, then the five words the CPU itself pushed.
///
/// `rsp` and `ss` are in the frame unconditionally, which is an `x86_64` change from 32-bit and is
/// load-bearing here: a long-mode interrupt pushes SS:RSP even when the ring does not change, so
/// there is exactly one frame layout rather than two, and `iretq` always pops five words.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrapFrame {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    /// Which of the 256 entries was taken. Pushed by the per-vector stub, because the CPU does not
    /// say.
    pub vector: u64,
    /// The hardware error code, or the zero the stub substituted for a vector that has none.
    pub error_code: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

// trap.s pushes 22 quadwords and `add rsp, 16` at the end assumes the vector and error code are the
// two words directly above the register block. Both facts are checked here rather than trusted.
const _: () = {
    assert!(size_of::<TrapFrame>() == 22 * 8);
    assert!(core::mem::offset_of!(TrapFrame, vector) == 15 * 8);
    assert!(core::mem::offset_of!(TrapFrame, error_code) == 16 * 8);
    assert!(core::mem::offset_of!(TrapFrame, rip) == 17 * 8);
};

impl TrapFrame {
    /// The syscall number the caller passed.
    ///
    /// **`rax`, following Linux**, and that choice is worth stating because it is the one place a
    /// third architecture could gratuitously invent a third convention. The tree's shape is one
    /// number register plus six argument registers, with argument 0 doubling as the return
    /// (aarch64: `x8` + `x0`..`x5`; RISC-V: `a7` + `a0`..`a5`). Applying that shape to `x86_64`'s own
    /// `syscall` convention gives `rax` + `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9`.
    ///
    /// **`r10` rather than `rcx` is not a preference**, it is the instruction: `syscall` overwrites
    /// `rcx` with the return address and `r11` with the caller's RFLAGS, so the fourth argument
    /// cannot ride in the C ABI's fourth register. Every `x86_64` kernel makes the same substitution
    /// for the same reason.
    ///
    /// **Ratified as DECISIONS §124** (2026-08-24). It was written here provisionally, because a
    /// syscall ABI is a boundary rather than a habit (DECISIONS §10, §16) and is exactly the kind of
    /// decision the "move fast on what can be undone" tenet calls expensive; it is now settled, and
    /// it is **spoken**: every hand-assembled program in `user::x86_programs` reaches
    /// `crate::syscall::dispatch` through these accessors, from ring 3, under the scheduler.
    pub fn syscall_nr(&self) -> u64 {
        self.rax
    }

    /// Syscall argument register `i`. See [`syscall_nr`](Self::syscall_nr) for the register list
    /// and why `r10` is in it.
    pub fn arg(&self, i: usize) -> u64 {
        match i {
            0 => self.rdi,
            1 => self.rsi,
            2 => self.rdx,
            3 => self.r10,
            4 => self.r8,
            5 => self.r9,
            _ => panic!("x86_64 syscall argument {i} does not exist (there are six)"),
        }
    }

    /// Set syscall argument/return register `i`. The return value and IPC message words ride back
    /// in the first three, exactly as they do on the other two architectures.
    pub fn set_arg(&mut self, i: usize, v: u64) {
        match i {
            0 => self.rdi = v,
            1 => self.rsi = v,
            2 => self.rdx = v,
            3 => self.r10 = v,
            4 => self.r8 = v,
            5 => self.r9 = v,
            _ => panic!("x86_64 syscall argument {i} does not exist (there are six)"),
        }
    }

    /// Build the frame that drops a brand-new thread to ring 3 at `entry` on `user_sp`, with `args`
    /// in the first three argument registers.
    ///
    /// The privilege change is carried entirely by the selectors: `iretq` returns to the ring named
    /// by the low two bits of the CS it pops, and pops SS:RSP too because it is changing ring. That
    /// is the whole mechanism, and it is why the GDT's layout in `segments.rs` is load-bearing.
    ///
    /// `rflags` is `IF | 0x2`. Bit 1 reads as one on every x86 since the 8086 and a frame without
    /// it is malformed; `IF` set is what keeps a tight-loop user thread preemptible, the same
    /// intent as RISC-V's SPIE and aarch64's DAIF = 0.
    ///
    /// Note what is **not** set: `IOPL` stays 0, so a ring-3 program may not touch the I/O space at
    /// all, and the TSS's I/O permission bitmap is empty (segments.rs). That is the only correct
    /// answer while there is no capability shape for a port (DECISIONS §121).
    #[allow(dead_code)]
    pub fn for_user_entry(entry: u64, user_sp: u64, args: [u64; 3]) -> Self {
        const RFLAGS_IF: u64 = 1 << 9;
        const RFLAGS_ALWAYS_ONE: u64 = 1 << 1;
        TrapFrame {
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: args[2],
            rsi: args[1],
            rdi: args[0],
            rbp: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            vector: 0,
            error_code: 0,
            rip: entry,
            cs: super::segments::USER_CODE as u64,
            rflags: RFLAGS_IF | RFLAGS_ALWAYS_ONE,
            rsp: user_sp,
            ss: super::segments::USER_DATA as u64,
        }
    }
}

/// One IDT entry: a 64-bit interrupt/trap gate.
///
/// The handler address is split across three fields for the same reason the TSS descriptor's is:
/// the layout grew from the 16-bit 286 outward and the new bits were appended rather than the old
/// ones widened.
#[repr(C)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    /// Bits 2:0 select an IST slot (0 = do not switch stacks); the rest is reserved.
    ist: u8,
    /// Present, DPL, and gate type. 0x8E is "present, DPL 0, 64-bit interrupt gate".
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    _reserved: u32,
}

impl IdtEntry {
    const fn empty() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            _reserved: 0,
        }
    }

    /// An interrupt gate at `handler`. **Interrupt gate, not trap gate**, and the difference is one
    /// bit that decides whether the CPU clears `RFLAGS.IF` on entry. An interrupt gate does, so a
    /// handler starts with interrupts masked and cannot be re-entered by the same source before it
    /// has established anything. A trap gate does not, which is what you want for a deliberate
    /// software interrupt and not for anything else.
    fn interrupt_gate(handler: u64, ist: u8) -> Self {
        Self {
            offset_low: handler as u16,
            selector: KERNEL_CODE,
            ist: ist & 0b111,
            type_attr: 0x8E,
            offset_mid: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            _reserved: 0,
        }
    }
}

const _: () = assert!(size_of::<IdtEntry>() == 16);

/// The IDT. 256 entries, 4 KiB, filled by [`init`] from the stub table trap.s exports.
static mut IDT: [IdtEntry; 256] = [IdtEntry::empty(); 256];

unsafe extern "C" {
    /// 256 stub addresses, one per vector, in vector order. Generated by trap.s.
    static ISR_STUBS: [u64; 256];
}

/// **How many interrupts became a message to a userspace endpoint**: the trap handler took one,
/// `sched::irq_route` found an endpoint bound to it, and `sched::irq_notify` was called. The
/// portable counter every architecture's interrupt tests read, and it now means on this
/// architecture what it has always meant on the other two.
///
/// **It used to count the local APIC timer, the reschedule IPI, and device vectors nothing had
/// claimed.** That was a divergence rather than a richer number, and it made every portable test
/// that reads this counter weaker here than elsewhere: an assertion that a *device's* completion
/// reached the kernel was satisfied by a timer tick landing in the same window, so on `x86_64` those
/// tests passed for the wrong reason whenever a tick obliged. That is not what *caused*
/// `user::tests::a_userspace_driver_reads_a_file_over_the_pcie_transport` to flake (it sampled its
/// baseline after the driver was already spawned, and both were fixed together), but it is why the
/// flake was rare instead of constant, and why fixing the sample alone would have been a widening
/// rather than a fix. See notes/load-sensitive-assertions.md for the whole account.
///
/// Nothing lost a number to this. Timer ticks are counted by `arch::timer::ticks()` and printed by
/// the bring-up tour from there; an unclaimed device vector is counted by [`DEVICE_IRQS`] and, when
/// there was nothing to route it to, by [`SPURIOUS_IRQS`].
pub static ROUTED_IRQS: AtomicUsize = AtomicUsize::new(0);

/// **How many interrupts arrived with nothing to route them to.** A spurious interrupt is normal on
/// x86 in a way it is not elsewhere: the 8259 PIC manufactures vector 7 (or 15) when a line drops
/// between assertion and acknowledgement, and the local APIC has a dedicated spurious vector.
pub static SPURIOUS_IRQS: AtomicUsize = AtomicUsize::new(0);

/// **How many NMIs arrived that the TLB shootdown did not claim.** Vector 2 is this kernel's
/// shootdown message (`mmu::shoot_down_others`) and nothing else on this machine sends one, so this
/// should stay at zero; it is counted rather than assumed away because an NMI from a source we do
/// not know about is exactly the kind of fact that is invisible until someone looks.
pub static NMIS_UNCLAIMED: AtomicUsize = AtomicUsize::new(0);

/// **How many *device* interrupts arrived through the IO APIC**, as distinct from the local APIC's
/// own timer. Counted separately because the two prove different things: the timer proves the local
/// APIC delivers and the trap path returns, and this proves a line outside the CPU reached it.
pub static DEVICE_IRQS: AtomicUsize = AtomicUsize::new(0);

/// **System calls served, counted for the system tests only** (`system_tests/src/user/tests.rs`
/// proves a program reached user mode and came back by watching it rise). Every syscall on every
/// core bumps this one line, so it is compiled out of every other build: until 2026-10-04 it was
/// in all of them, and it was one of two shared writes on the cheapest syscall's path
/// (notes/job-mix/null-syscall-under-load.md).
#[cfg(any(test, feature = "system_tests"))]
pub static SVC_COUNT: AtomicUsize = AtomicUsize::new(0);

/// **How many user-mode faults were taken.**
pub static USER_FAULTS: AtomicUsize = AtomicUsize::new(0);

/// **How many breakpoints were taken.** `int3`, vector 3, which is what [`self_test`] uses to prove
/// the trap path round-trips.
pub static BRK_COUNT: AtomicUsize = AtomicUsize::new(0);

/// The last user fault, packed by [`UserFault::encode`]. 0 means no user thread has faulted.
static LAST_USER_FAULT: AtomicU64 = AtomicU64::new(0);
/// The faulting address of the fault [`LAST_USER_FAULT`] describes.
static LAST_USER_FAULT_ADDRESS: AtomicU64 = AtomicU64::new(0);

/// What the last user-mode fault was and where. `None` until one has happened.
pub fn last_user_fault() -> Option<(UserFault, u64)> {
    // Pairs with the `Release` on `USER_FAULTS` in [`user_fault`]: a caller that has already seen
    // the counter rise must see the record that rise was announcing, and not the previous one.
    let code = LAST_USER_FAULT.load(Ordering::Acquire);
    UserFault::decode(code).map(|f| (f, LAST_USER_FAULT_ADDRESS.load(Ordering::Relaxed)))
}

/// **Diagnostic: the ring-3 PC of a thread, from the [`TrapFrame`] at the top of its kernel
/// stack.** The twin of the other two architectures' `user_pc`, and the watchdog dump's
/// user-PC column.
///
/// The frame lives at `stack_top - size_of::<TrapFrame>()` on all three architectures, which is
/// what makes this readable at all; see `user::enter_frame` for why that placement is a rule rather
/// than a convenience. Meaningless for a pure kernel thread, which never builds one.
pub fn user_pc(stack_top: u64) -> u64 {
    let frame = (stack_top - size_of::<TrapFrame>() as u64) as *const TrapFrame;
    // SAFETY: a diagnostic read of the frame the entry path writes at the stack top. Volatile
    // because the owning thread may be running on another CPU while we read, and this must not be
    // hoisted.
    unsafe { core::ptr::read_volatile(&raw const (*frame).rip) }
}

unsafe extern "C" {
    /// Load `frame` as the register state and `iretq` into it: the first entry to ring 3. Defined
    /// in trap.s, sharing the restore path with every trap return. Its first instructions are `cli`
    /// and `mov rsp, rdi`, so it does not touch the caller's stack.
    fn user_return(frame: *mut TrapFrame) -> !;
}

/// Drop to ring 3 by loading `frame` and `iretq`ing into it. The x86 side of the userspace-entry
/// seam (the counterpart of aarch64's `enter_user` and RISC-V's).
///
/// **The privilege change is carried entirely by the frame**, which is the piece of x86 worth
/// stating plainly: `iretq` returns to the ring named by the low two bits of the CS it pops, and
/// pops SS:RSP as well because it is changing ring. There is no "return to user" instruction and no
/// mode bit to set; there is one instruction that pops a context, and the context says which ring.
///
/// **`#[inline(always)]` is load-bearing**, exactly as on the other two architectures: the frame
/// sits at the top of the caller's own kernel stack, so a real call frame pushed here could land on
/// top of it. Inlining makes the caller tail-jump to `user_return` with no push.
///
/// # Safety
/// `frame` must be a correctly-built, writable [`TrapFrame`] at the top of the current thread's
/// kernel stack, with the user address space installed and `TSS.RSP0` naming a kernel stack the
/// next trap can be pushed onto (`segments::set_kernel_stack`).
#[inline(always)]
pub unsafe fn enter_user(frame: *mut TrapFrame) -> ! {
    // **Refuse to enter ring 3 with no entry point**, for the reason the RISC-V twin carries at
    // length: a thread dispatched with `rip == 0` fetches its first instruction from address 0,
    // page-faults, and dies, and whatever it was supposed to serve then never answers. That is a
    // lost-wakeup hang arbitrarily far from the cause; this converts it into a loud failure that
    // carries its own evidence. The comparison is a load and a branch on a frame this function
    // already touches, so nothing is pushed over the frame at the top of this stack.
    //
    // SAFETY: the caller's contract says `frame` is a valid, writable `TrapFrame`.
    let rip = unsafe { (*frame).rip };
    if rip == 0 {
        // SAFETY: as above; read before the cold call below is allowed to use this stack.
        let user_sp = unsafe { (*frame).rsp };
        entered_user_with_no_entry_point(user_sp);
    }

    // SAFETY: the caller's contract; `user_return` never returns.
    unsafe { user_return(frame) }
}

/// The `rip == 0` case, out of line so [`enter_user`] stays a tail jump.
///
/// Its argument is read from the frame **before** it is called, because the frame lives at the top
/// of this very stack and this call is entitled to overwrite it.
#[cold]
#[inline(never)]
fn entered_user_with_no_entry_point(user_sp: u64) -> ! {
    panic!(
        "a thread on cpu {} was dispatched to ring 3 with rip = 0 (user rsp {user_sp:#018x}). \
         Its context was never built, or was built and not seen by this cpu.",
        crate::cpu::id(),
    )
}

/// Unmask external interrupts at the controller.
///
/// **Nothing to do on this architecture, and that is a real difference rather than a stub.** RISC-V
/// has a second gate, `sie.SEIE`, that has to be opened before an external interrupt can be
/// delivered even with `sstatus.SIE` set. x86 has one gate, `RFLAGS.IF`, which
/// `arch::interrupts::enable` owns. What is *per source* here is the IO APIC's mask bit, which
/// `irq::enable` clears when it arms a line, and the local APIC's Task Priority Register, which
/// `irq::init_local_apic` sets to zero so no priority class is dropped.
///
/// # Panics
/// If the local APIC is not up. An empty function that silently succeeded before the controller
/// exists would let a caller believe interrupts are unmasked when nothing can deliver one.
pub fn enable_external() {
    assert!(
        super::irq::is_local_apic_ready(),
        "external interrupts are the local APIC's to deliver, and it is not up yet",
    );
}

/// **Whether [`IDT`] has been built yet.** Guards the one write every core used to make
/// unconditionally, back when there was only ever one core to make it (milestone 161's SMP item).
static IDT_BUILT: AtomicBool = AtomicBool::new(false);

/// Install the IDT on this CPU.
///
/// Every vector gets a gate, including the 224 that no hardware currently drives: an IDT entry that
/// is not present makes an unexpected vector a **general protection fault** whose error code is the
/// vector, which is a second puzzle stacked on the first. A present gate that prints "vector 87,
/// unexpected" is strictly more informative and costs 4 KiB.
///
/// The double fault is the one entry with an IST. It has to be: the reason a double fault happens
/// is that the CPU could not deliver a first exception, and the commonest cause of that is a stack
/// that cannot be pushed to. A handler that tries to push its frame onto the same broken stack
/// triple-faults, which QEMU reports as a silent machine reset and a real machine as a reboot.
///
/// **The table itself is built at most once, by whichever core gets here first** (in practice
/// always the boot core: `smp::bring_up_secondaries` never starts a second core before its own
/// `arch::init` has long since returned). Every core still calls this and every core still `lidt`s
/// its own IDTR (`lidt` names a per-core register, not shared state), but a secondary rewriting 256
/// already-correct gate descriptors used to be harmless only because nothing else was running.
/// **Fixed, milestone 161's SMP item:** the boot core keeps interrupts enabled across
/// `bring_up_secondaries` (its own timer must keep ticking), so a secondary's rewrite loop used to
/// race the boot core's live interrupt delivery reading the SAME table; a torn read of one gate
/// descriptor mid-write is a spurious or garbled trap on the boot core, not a compile error, which
/// is exactly the shape of bug this file's own comments elsewhere warn does not announce itself.
pub fn init() {
    if !IDT_BUILT.swap(true, Ordering::AcqRel) {
        // SAFETY: guarded by the swap above, so at most one core ever runs this loop, and it runs
        // to completion before any OTHER core's `lidt` (below) can possibly point at this table:
        // that core would have to have gotten here first, which the swap already ruled out. The
        // stub table is 256 entries and so is the IDT, so every index is in bounds.
        unsafe {
            for vector in 0..256 {
                let ist = if vector == 8 { IST_DOUBLE_FAULT } else { 0 };
                IDT[vector] = IdtEntry::interrupt_gate(ISR_STUBS[vector], ist);
            }
        }
    }

    let idtr = super::segments::DescriptorTablePointer {
        limit: (size_of::<[IdtEntry; 256]>() - 1) as u16,
        base: (&raw const IDT) as u64,
    };
    // SAFETY: `idtr` describes the table, which the branch above guarantees is fully built by the
    // time any core reaches this `lidt`, whose every entry is a well-formed gate pointing at a stub
    // in this image. `lidt` writes only this core's own IDTR.
    unsafe {
        core::arch::asm!("lidt [{}]", in(reg) &idtr, options(readonly, nostack, preserves_flags));
    }
}

/// **Prove the trap path round-trips**, by taking a breakpoint and returning from it.
///
/// Returns how many breakpoints were caught **during this call**, which the boot tour prints. The
/// same shape as the RISC-V and aarch64 twins, and it earns its place for the same reason: an IDT
/// that is installed but whose return path is wrong produces a kernel that runs fine until the
/// first real fault and then vanishes, and there is no other cheap moment to find that out.
///
/// **It returned the cumulative count until milestone 268**, which read the same as the other two
/// right up until something called it twice: the tour calls it once during bring-up and
/// `self_test::run` calls it again a few hundred lines later, and the second call reported `2`.
/// Harmless here (both tests are `>= 1`) and exactly the shape of bug that is not harmless in a
/// counter somebody later compares for equality, so the three now answer the same question.
pub fn self_test() -> usize {
    let before = BRK_COUNT.load(Ordering::Relaxed);
    // SAFETY: `int3` raises vector 3, which the IDT above routes to the common handler. The handler
    // treats a breakpoint as an event to count and step over, so this returns to the next
    // instruction.
    unsafe { core::arch::asm!("int3", options(nomem, nostack)) };
    BRK_COUNT.load(Ordering::Relaxed) - before
}

// ---------------------------------------------------------------------------------------------
// The `syscall` instruction pair (milestone 161, roadmap item 3).
//
// x86 has TWO ways into the kernel and this is the second one. The IDT above is how a *fault* or an
// *interrupt* arrives; `syscall` is how a program asks on purpose, and it shares almost nothing with
// the IDT path: no gate, no descriptor, no stack switch, no pushes. Four MSRs are the whole of its
// configuration, and none of them has a default worth having.
// ---------------------------------------------------------------------------------------------

/// `IA32_EFER`. Bit 0, `SCE`, is what makes `syscall` a legal instruction rather than `#UD`. The
/// same register carries `LME` and `NXE`, which `boot.s` set before any table was built.
const IA32_EFER: u32 = 0xC000_0080;
/// `IA32_STAR`: the selectors. `[47:32]` is what `syscall` loads into CS (and, plus 8, into SS);
/// `[63:48]` is the base `sysret` derives the user pair from.
const IA32_STAR: u32 = 0xC000_0081;
/// `IA32_LSTAR`: where a 64-bit `syscall` jumps. There is no default and no descriptor: this is a
/// raw address, so a zero here is a jump to zero.
const IA32_LSTAR: u32 = 0xC000_0082;
/// `IA32_FMASK`: the `RFLAGS` bits a `syscall` clears on the way in.
const IA32_FMASK: u32 = 0xC000_0084;

/// `IA32_EFER.SCE`.
const EFER_SCE: u64 = 1 << 0;

/// The selector base `sysret` derives the user pair from, and `syscall`'s kernel pair sits at
/// [`KERNEL_CODE`].
///
/// **The GDT's order is this arithmetic, not a style choice** (segments.rs says so at its top). The
/// three assertions below are the mechanism that keeps the two files agreeing: change a selector in
/// segments.rs without changing this and the build stops, rather than `sysret` landing a user
/// program on the kernel's data segment.
const SYSRET_SELECTOR_BASE: u16 = 0x10;

const _: () = {
    assert!(
        SYSRET_SELECTOR_BASE + 8 == super::segments::USER_DATA & !3,
        "sysret computes the user SS as IA32_STAR[63:48] + 8"
    );
    assert!(
        SYSRET_SELECTOR_BASE + 16 == super::segments::USER_CODE & !3,
        "sysret computes the user CS as IA32_STAR[63:48] + 16"
    );
    assert!(
        KERNEL_CODE + 8 == super::segments::KERNEL_DATA,
        "syscall computes the kernel SS as IA32_STAR[47:32] + 8"
    );
};

/// The `RFLAGS` bits cleared on every `syscall`, named one at a time because each is a hazard
/// rather than tidiness.
///
/// `IF` (9) is the important one: clearing it makes `syscall` arrive with interrupts masked exactly
/// as an interrupt gate does, so the handler cannot be re-entered before it has a stack. `TF` (8)
/// would single-step the kernel on behalf of a user debugger. `DF` (10) must be clear on entry to
/// any System V function, and a user program is free to leave it set. `IOPL` (13:12) is the
/// all-or-nothing I/O gate, and a value inherited from ring 3 is not one the kernel chose. `NT` (14)
/// changes what `iret` means. `RF` (16) suppresses instruction breakpoints. `AC` (18) turns
/// unaligned kernel accesses into faults, which is a user-settable bit that has been a real
/// privilege-escalation lever elsewhere.
const SYSCALL_FLAG_MASK: u64 = (1 << 8)      // TF
    | (1 << 9)                               // IF
    | (1 << 10)                              // DF
    | (0b11 << 12)                           // IOPL
    | (1 << 14)                              // NT
    | (1 << 16)                              // RF
    | (1 << 18); // AC

/// The pseudo-vector a `syscall` frame carries, so a fault report or a dump can say where the frame
/// came from. **256 is out of the IDT's range by construction**, which is what keeps it from
/// colliding with a real vector; the assertion says so rather than leaving it to be noticed.
pub const SYSCALL_VECTOR: u64 = 0x100;
const _: () = assert!(
    SYSCALL_VECTOR > 255,
    "a real IDT vector would be ambiguous here"
);

// **Point the `syscall` path's kernel stack at `top`, on this core.** DELETED 2026-09-30 by the
// CI-warnings lane: swish-check's x86_64 leg warned this setter and its only caller
// (`segments::set_kernel_stack`) were never used in any configuration, because `isr_restore` in
// trap.s writes this slot (`SYSCALL_KERNEL_RSP_OFF`, reached `gs`-relatively through
// `cpu::PerCpu::x86_trap`; see `super::global_asm!`'s substitution) and `TSS.RSP0` together on
// every return to ring 3, the first one included. The two-writes-in-step property these doc
// blocks sold ("a trap and a `syscall` cannot name different stacks") lives there now; this
// comment stays so a reader grepping for the old name finds where the job went. It had been
// per-CPU since the SMP item of milestone 161 (the x86_64 kernel port), after starting as one flat
// `static mut` every CPU raced.
/// **Program the four MSRs that make `syscall` work**, once per CPU, at boot.
///
/// Order matters in one place: `SCE` is enabled last, so the instruction becomes legal only after
/// the address it jumps to and the flags it clears are already in place. The reverse order leaves a
/// window in which a `syscall` would jump to whatever `IA32_LSTAR` happened to hold, which on a cold
/// machine is zero.
///
/// # Safety
/// Must be called with the GDT installed (the selectors this writes index it) and before anything
/// enters ring 3.
pub unsafe fn init_syscall() {
    let star = ((SYSRET_SELECTOR_BASE as u64) << 48) | ((KERNEL_CODE as u64) << 32);
    // SAFETY: these are the architectural `syscall` MSRs, present on every long-mode CPU (they are
    // part of the mode itself, not an optional feature). The values are checked against the GDT by
    // the const assertions above.
    unsafe {
        super::write_msr(IA32_STAR, star);
        super::write_msr(IA32_LSTAR, x86_syscall_entry as *const () as u64);
        super::write_msr(IA32_FMASK, SYSCALL_FLAG_MASK);
        let efer = super::read_msr(IA32_EFER);
        super::write_msr(IA32_EFER, efer | EFER_SCE);
    }
}

unsafe extern "C" {
    /// The `IA32_LSTAR` target: where a ring-3 `syscall` lands. Defined in trap.s.
    fn x86_syscall_entry();
}

/// **The one Rust function every `syscall` reaches**, called by `x86_syscall_entry` with a pointer
/// to the [`TrapFrame`] it built. The `syscall` twin of [`x86_trap_handler`].
///
/// It hands straight to the portable dispatcher, which reads the number and the arguments through
/// `TrapFrame::{syscall_nr, arg, set_arg}` and so never names an x86 register. That is the whole
/// point of those accessors, and this function is where the third architecture cashes them in.
///
/// # BUGS
/// **The return is an `iretq`, not a `sysretq`.** `sysret` is the faster half of the instruction
/// pair and this port does not use it yet, deliberately: it returns to whatever `rcx` holds without
/// checking that the address is canonical, and a non-canonical `rcx` faults **in ring 0 on the
/// user's stack**, which is the shape of CVE-2012-0217 and needs an explicit canonicality check and
/// an `iretq` fallback to use safely. Sharing `isr_restore` costs some tens of cycles per syscall
/// and buys one return path with one `swapgs` rule. Worth revisiting with a benchmark rather than an
/// argument, once there is a syscall-heavy workload on this architecture to measure.
///
/// # Safety
/// Called only from `x86_syscall_entry`, which has just constructed a complete [`TrapFrame`] at the
/// address it passes. Nothing else may call it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn x86_syscall_handler(frame: *mut TrapFrame) {
    // SAFETY: `x86_syscall_entry` built the frame directly below the pointer it passed, and no
    // other caller exists (the symbol is only referenced from trap.s).
    let frame = unsafe { &mut *frame };
    #[cfg(any(test, feature = "system_tests"))]
    SVC_COUNT.fetch_add(1, Ordering::Relaxed);
    crate::syscall::dispatch(frame);
}

/// Read `CR2`, which holds the faulting address after a page fault (vector 14) and nothing
/// meaningful otherwise. The x86 analog of `FAR_EL1` and of RISC-V's `stval`.
fn faulting_address() -> u64 {
    super::instructions::read_cr2()
}

/// The names of the 32 architecturally defined exceptions, so a fault report says what happened
/// rather than only which number it was.
const EXCEPTION_NAMES: [&str; 32] = [
    "divide error",
    "debug",
    "non-maskable interrupt",
    "breakpoint",
    "overflow",
    "bound range exceeded",
    "invalid opcode",
    "device not available",
    "double fault",
    "coprocessor segment overrun",
    "invalid TSS",
    "segment not present",
    "stack-segment fault",
    "general protection fault",
    "page fault",
    "reserved (15)",
    "x87 floating-point exception",
    "alignment check",
    "machine check",
    "SIMD floating-point exception",
    "virtualization exception",
    "control protection exception",
    "reserved (22)",
    "reserved (23)",
    "reserved (24)",
    "reserved (25)",
    "reserved (26)",
    "reserved (27)",
    "hypervisor injection exception",
    "VMM communication exception",
    "security exception",
    "reserved (31)",
];

unsafe extern "C" {
    /// Switch to `top` (or stay put if it is 0), call [`x86_trap_body`], come back. Defined in
    /// trap.s, because moving `rsp` is assembly and policy is not.
    fn dispatch_on_interrupt_stack(frame: *mut TrapFrame, top: u64) -> bool;
}

/// **The outer half of the trap path, and the half that stays on the interrupted stack.**
///
/// The third implementation of a split the other two architectures already have, and it exists for
/// exactly their reason: it picks the stack the handler runs on, and it runs the deferred
/// `schedule()` afterwards. `schedule()` parks this thread's `rsp` in its `Context` and resumes it
/// there later, so it may only be called on a stack that belongs to the thread; calling it from the
/// per-CPU interrupt stack would park a per-CPU address in a thread and the thread would resume on
/// bytes the next interrupt had already spent. See `kernel/src/interrupt_stack.rs`.
///
/// # Safety
/// Called only from `isr_common`, which has just constructed a complete [`TrapFrame`] at the
/// address it passes. Nothing else may call it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn x86_trap_dispatch(frame: *mut TrapFrame) {
    // **An NMI is served here and returns from here**, before either of the two things below can
    // happen to it, and both would be wrong for it.
    //
    // It may not move to the interrupt stack. An NMI arrives at an arbitrary instruction boundary,
    // including one inside a handler already running on that stack, and `top_for_trap` would hand
    // it the same top and let it overwrite the frames underneath it.
    //
    // It may not owe a deferred `schedule()`. The whole point of an NMI shootdown is that its
    // target is often mid-critical-section with interrupts masked; switching threads out from under
    // that would be a far larger surprise than the interrupt itself.
    //
    // Serving it costs an `invlpg` and one atomic (`mmu::serve_shootdown_nmi`), so the frame it is
    // charged to, wherever it landed, is the trap frame plus a shallow call.
    // SAFETY: `isr_common` built the frame directly below the pointer it passed; reading the vector
    // it manufactured, before anything else looks at the frame.
    if unsafe { (*frame).vector } == 2 {
        if !super::mmu::serve_shootdown_nmi() {
            NMIS_UNCLAIMED.fetch_add(1, Ordering::Relaxed);
        }
        return;
    }

    // SAFETY: `isr_common` built the frame directly below the pointer it passed.
    let from_user = unsafe { (*frame).cs } & 3 == 3;
    let top = crate::interrupt_stack::top_for_trap(from_user);
    let deferred_switch = if top == 0 {
        // The common case, and it must not pay for the uncommon one: the other two ports measured
        // 8.6 instructions per syscall for routing every trap through the trampoline to do a stack
        // move that does not happen.
        //
        // SAFETY: the caller's contract; forwarded unchanged.
        unsafe { x86_trap_body(frame) }
    } else {
        // SAFETY: `top` is this CPU's own interrupt-stack top, from the module that owns the
        // region; the trampoline calls `x86_trap_body` with our own argument and restores `rsp`
        // before returning. The frame outlives the call: it is on the stack we are standing on.
        unsafe { dispatch_on_interrupt_stack(frame, top) }
    };

    // Back on the interrupted thread's stack, whichever branch ran. Preemption happens HERE.
    if deferred_switch {
        crate::sched::preempt_if_needed();
    }
}

/// **The trap handler proper**: everything that runs on the interrupt stack when there is one.
///
/// Returns whether the caller owes a deferred `schedule()`, which is true for an interrupt and
/// false for everything else.
///
/// # Safety
/// Called only from [`x86_trap_dispatch`] or from `dispatch_on_interrupt_stack`, both of which pass
/// a complete [`TrapFrame`]. Nothing else may call it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn x86_trap_body(frame: *mut TrapFrame) -> bool {
    // SAFETY: `isr_common` built the frame directly below the pointer it passed, and no other
    // caller exists (the symbol is only referenced from trap.s and from the function above).
    let frame = unsafe { &mut *frame };

    match frame.vector {
        // A breakpoint is an event to count and step over: `int3` is one byte and the CPU already
        // advanced rip past it, so returning is enough. This is what `self_test` proves.
        3 => {
            BRK_COUNT.fetch_add(1, Ordering::Relaxed);
            false
        }
        // **A thread asked for the FP unit for the first time**
        // (milestone 447 (a thread's vector registers are its own)). Vector 7 is `#NM`,
        // "device not available", which `CR0.TS` raises on the first FP or SSE instruction a thread
        // executes. Enabling is the whole handler: `#NM` is a fault, so `rip` already points at the
        // instruction that trapped and the `iret` re-executes it with the unit open.
        //
        // **This is the bit LazyFP was about** (CVE-2018-3665), and `crate::fp`'s header says why
        // using it this way is not that: `TS` answers "has this thread ever wanted the unit", and
        // the registers themselves are always either the running thread's or the initial state.
        //
        // Like the other two architectures' arms it serves ring 0 as well as ring 3, because every
        // userspace target in `targets/` is soft-float and milestone 447's concurrency proof is
        // therefore written as kernel threads.
        //
        // `enable_for_current` returning false means there is no thread to record it against;
        // falling through to the handler below is better than an `iret` that retakes the same fault
        // forever.
        7 if crate::fp::enable_for_current() => false,

        // The local APIC timer. Counted, then acknowledged below with every other interrupt.
        //
        // **Record and defer** (DECISIONS §9): `on_tick` marks a reschedule due and this function
        // returns `true`, so the switch happens in `x86_trap_dispatch`, one frame out, on the
        // interrupted thread's own stack. Doing it here would be doing it mid-handler with the
        // local APIC not yet told we are done.
        v if v == super::irq::TIMER_VECTOR as u64 => {
            super::timer::tick();
            crate::sched::on_tick();
            // Not counted in ROUTED_IRQS: nothing routed this to a driver. It used to be, and
            // the cost was that a tick could answer a test asking whether a *device's* completion
            // had arrived. `timer::ticks()` is the count of these.
            super::irq::end_of_interrupt();
            true
        }
        // A reschedule IPI from another CPU (`irq::send_reschedule`). It carries two meanings, as
        // aarch64's reschedule SGI does: the sender may have handed us a thread through our inbox,
        // or an idle CPU may be asking us for work. Both are served, and the deferred `schedule()`
        // at the caller runs whatever arrived.
        //
        // Unreachable today: there is one CPU, and every `send_reschedule` caller in `sched` is
        // guarded by "the target is another core". It is here rather than under the catch-all
        // because the catch-all would call it spurious, which it is not.
        v if v == super::irq::RESCHEDULE_VECTOR as u64 => {
            crate::sched::drain_inbox();
            crate::sched::serve_steal_request();
            // Not counted in ROUTED_IRQS, for the timer arm's reason: this is the kernel talking
            // to itself, not an interrupt becoming a driver's message.
            super::irq::end_of_interrupt();
            true
        }
        // **A self-directed interrupt, and it becomes a message.** This is the one delivery path
        // that is real on this architecture today, and it is aarch64's case rather than RISC-V's:
        // the local APIC delivers a vector to its own CPU on demand, through the ICR, the IRR, the
        // ISR and an EOI, so the interrupt is the hardware's rather than a function call wearing a
        // handler's name. aarch64 proves the same property with a software-generated interrupt;
        // RISC-V, which can raise nothing at all, has to assert its console UART's line instead.
        //
        // **The intid here is the vector number**, which is the whole of the x86 naming rule and is
        // worth stating because `irq::enable` takes a *legacy IRQ* instead: a local APIC source is
        // named by the vector it raises (there is no line and no controller input to name), and a
        // device line is named by the legacy IRQ the MADT resolves to a GSI. The two domains cannot
        // collide, because legacy IRQs are 0..15 and these vectors are 0x22 and 0x23.
        v if v == super::irq::SELF_TEST_VECTOR as u64
            || v == super::irq::SELF_TEST_VECTOR_B as u64 =>
        {
            match crate::sched::irq_route(v as u32) {
                Some(ep) => {
                    ROUTED_IRQS.fetch_add(1, Ordering::Relaxed);
                    // Nothing to mask: a self-IPI is edge-delivered and one-shot, so there is no
                    // asserted line to hold off until a driver quiets it. That is the same reason
                    // aarch64's `quiet_test_irq` for an SGI is empty.
                    crate::sched::irq_notify(ep);
                }
                None => {
                    SPURIOUS_IRQS.fetch_add(1, Ordering::Relaxed);
                }
            }
            super::irq::end_of_interrupt();
            true
        }
        // **A PCI function's MSI-X message, and it becomes a driver's wakeup** (milestone 215).
        //
        // This is the arm that makes a userspace device driver possible on this architecture, and
        // it is short for a reason worth stating rather than hiding: an MSI vector IS its intid
        // (`irq::MSI_VECTOR_BASE`), because the device writes the vector straight to the local
        // APIC and there is no controller input in between to name it by. So the inversion the
        // arm below still owes for an IO APIC line does not exist here; `sched::irq_route` can be
        // asked with the vector the CPU just took.
        //
        // Nothing is masked, exactly as for the self-IPI above and for aarch64's SGI: an MSI is a
        // posted write that already happened, so there is no asserted line to hold off until the
        // driver quiets its device. The driver's `Irq::ACK` is correspondingly a no-op
        // (`irq::enable`), and the used-ring loop in `crates/virtio` is what actually decides a
        // request is complete.
        v if super::irq::is_msi_vector(v) => {
            match crate::sched::irq_route(v as u32) {
                Some(ep) => {
                    DEVICE_IRQS.fetch_add(1, Ordering::Relaxed);
                    ROUTED_IRQS.fetch_add(1, Ordering::Relaxed);
                    crate::sched::irq_notify(ep);
                }
                None => {
                    SPURIOUS_IRQS.fetch_add(1, Ordering::Relaxed);
                }
            }
            super::irq::end_of_interrupt();
            true
        }
        // A device line, routed by the IO APIC's redirection table onto a vector this kernel chose
        // (irq::GSI_VECTOR_BASE plus the line's redirection index; milestone 308, and it was plus
        // the GSI until then), **and it becomes a driver's message** (milestone 505 (an x86_64
        // input driver that never lets the core idle)), as on the other two architectures.
        //
        // `irq::intid_of_vector` turns the vector back into the intid that armed it, which is the
        // number a driver bound; subtraction alone gives the redirection index, and an index is not
        // a legacy IRQ (that function's table has why). COM1's receive line, legacy IRQ 4, is the
        // line this was built for: the input driver waits on it rather than polling the port.
        //
        // `irq::disable` holds a level-triggered line off until the driver's ACK re-arms it
        // (`irq::enable`); an edge-triggered line, COM1's among them, is left live, because an IO
        // APIC drops an edge that arrives while its entry is masked. A line nothing has bound (the
        // tour's PIT) is counted and acknowledged, as every device vector was before.
        v if super::irq::is_device_vector(v) => {
            DEVICE_IRQS.fetch_add(1, Ordering::Relaxed);
            let routed = super::irq::intid_of_vector(v)
                .and_then(|intid| crate::sched::irq_route(intid).map(|ep| (intid, ep)));
            if let Some((intid, ep)) = routed {
                ROUTED_IRQS.fetch_add(1, Ordering::Relaxed);
                super::irq::disable(intid);
                crate::sched::irq_notify(ep);
            }
            super::irq::end_of_interrupt();
            true
        }
        // A spurious interrupt: the local APIC had to deliver something and had nothing real. It
        // is **not** acknowledged, and that is architecture rather than an oversight: the APIC does
        // not set an in-service bit for a spurious interrupt, so an EOI here would acknowledge
        // whatever genuinely is in service and lose it.
        v if v == super::irq::SPURIOUS_VECTOR as u64 => {
            SPURIOUS_IRQS.fetch_add(1, Ordering::Relaxed);
            false
        }
        // Any other vector at or above 32 is a device interrupt nothing has claimed. Counted and
        // acknowledged rather than fatal: an unowned interrupt that is not acknowledged wedges the
        // local APIC, which turns a stray line into a dead machine.
        v if v >= 32 => {
            SPURIOUS_IRQS.fetch_add(1, Ordering::Relaxed);
            super::irq::end_of_interrupt();
            true
        }
        // An architectural exception taken **in ring 3**: the user program's fault, not the
        // kernel's. Recorded rather than fatal, which is the difference the whole privilege
        // boundary exists to make. Only vectors below 32 reach here; a device interrupt that
        // happened to land while a user program was running was already handled above, because an
        // interrupt is not the interrupted program's fault.
        vector if frame.cs & 3 == 3 => user_fault(frame, vector),
        // Everything below 32 is an architectural exception, and every one of them is fatal here.
        vector => {
            let name = EXCEPTION_NAMES
                .get(vector as usize)
                .copied()
                .unwrap_or("reserved");
            crate::println!();
            crate::println!("=== x86_64 trap: vector {vector} ({name}) ===");
            crate::println!("  error code : {:#018x}", frame.error_code);
            crate::println!(
                "  rip        : {:#018x}   cs : {:#06x}",
                frame.rip,
                frame.cs
            );
            crate::println!(
                "  rsp        : {:#018x}   ss : {:#06x}",
                frame.rsp,
                frame.ss
            );
            crate::println!("  rflags     : {:#018x}", frame.rflags);
            if vector == 14 {
                // Only meaningful for a page fault, so only printed for one: CR2 is stale
                // otherwise, and a stale address printed as if it were current is worse than no
                // address at all.
                crate::println!("  cr2        : {:#018x}", faulting_address());
            }
            panic!("unhandled x86_64 exception: vector {vector} ({name})");
        }
    }
}

/// **A ring-3 thread faulted: record what happened, say so, and kill it.**
///
/// The third implementation of the seam that makes "a driver bug is a crashed process, not a dead
/// machine" (DECISIONS §10) true on this architecture. Until roadmap item 4 this arm recorded the
/// fault and then panicked, because there was no scheduler and so no thread to kill; it is what a
/// scheduler turns into a real teardown.
///
/// **This never returns**, exactly as the other two ports' twins do not: `sched::fault` delivers the
/// death to a supervisor if this thread had one and otherwise takes the unsupervised path, and
/// either way it switches to somebody else and does not come back. `isr_restore` is never reached,
/// the `iretq` never happens, and the user program is simply not resumed. The kernel stack we are
/// standing on is freed by the next thread (the reaper), because a thread cannot free the stack it
/// is standing on.
fn user_fault(frame: &TrapFrame, vector: u64) -> ! {
    let (fault, address) = classify_user_fault(vector, frame.error_code);

    // **The record first, the counter last, and the counter's store is the release.** Every test
    // that reads this record finds it by watching `USER_FAULTS` rise and then calling
    // [`last_user_fault`], so the counter is the publication flag for the record and must be
    // written after it. Both other ports carry the same fix and the same comment; this port was
    // written with the counter first, which is a race a passing test cannot see (the reader gets
    // either an earlier fault's record, or on the boot's first fault a zero that reads as "nothing
    // faulted").
    LAST_USER_FAULT_ADDRESS.store(address, Ordering::Relaxed);
    LAST_USER_FAULT.store(fault.encode(), Ordering::Relaxed);
    USER_FAULTS.fetch_add(1, Ordering::Release);

    let name = EXCEPTION_NAMES
        .get(vector as usize)
        .copied()
        .unwrap_or("reserved");
    crate::println!();
    crate::println!(
        "  user thread {} killed: vector {vector} ({name})",
        crate::sched::current(),
    );
    crate::println!(
        "    rip {:#018x}   addr {:#018x}   user rsp {:#018x}   err {:#010x}",
        frame.rip,
        address,
        frame.rsp,
        frame.error_code,
    );
    crate::println!("  the kernel is fine.");

    // Deliver the fault to a supervisor if this thread had one, and become a corpse; otherwise this
    // is `exit`'s unsupervised path (Finished, reaped by the next thread). See DECISIONS §26 and
    // sched::depart. `frame.rip` is the faulting pc, `address` the faulting address.
    crate::sched::fault(frame.rip, address);
}

/// **Turn an x86 exception into the portable [`UserFault`] both other architectures already
/// state.**
///
/// The page fault's error code is the interesting half, and x86 is the *most* forthcoming of the
/// three about it: aarch64 is told the fault class in `ESR_EL1`, RISC-V has to derive it by walking
/// the tables (see its `user_fault`), and here the CPU pushes a word saying so outright.
///
/// | Bit | Set means |
/// |---|---|
/// | 0 (`P`) | the page **was** present, so this is a permission refusal rather than a missing map |
/// | 1 (`W/R`) | the access was a write |
/// | 2 (`U/S`) | the access came from ring 3 |
/// | 3 (`RSVD`) | a reserved bit was set in some entry of the walk |
/// | 4 (`I/D`) | the access was an instruction fetch |
///
/// The `P` bit is what separates [`UserFault::Permission`] from [`UserFault::Translation`], and that
/// distinction is the one worth having: a permission fault means the hardware **found** the page,
/// read its bits, and said no.
///
/// Anything that is not a page fault is [`UserFault::Other`] at the faulting instruction, matching
/// what the other two report for an illegal instruction or a misaligned access.
fn classify_user_fault(vector: u64, error_code: u64) -> (UserFault, u64) {
    use crate::arch::UserFaultAccess;

    if vector != 14 {
        return (UserFault::Other, 0);
    }
    let access = if error_code & (1 << 4) != 0 {
        UserFaultAccess::Fetch
    } else if error_code & (1 << 1) != 0 {
        UserFaultAccess::Write
    } else {
        UserFaultAccess::Read
    };
    let fault = if error_code & 1 != 0 {
        UserFault::Permission(access)
    } else {
        UserFault::Translation(access)
    };
    (fault, faulting_address())
}
