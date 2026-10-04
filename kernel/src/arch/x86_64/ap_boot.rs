//! **Starting a second logical CPU: INIT-SIPI-SIPI and the real-mode trampoline it needs**
//! (milestone 161's SMP item).
//!
//! aarch64 asks firmware to start a core (PSCI `CPU_ON`) and RISC-V asks SBI (`hart_start`); x86
//! has no firmware call for this at all. Starting a second CPU here means sending it two kinds of
//! inter-processor interrupt through the local APIC: an INIT (reset it into a wait-for-SIPI state)
//! and two STARTUP IPIs ("SIPI"), which name a *physical page below 1 MiB* the target begins
//! executing at, in 16-bit real mode. See `irq::send_init`/`irq::send_startup` for the messages
//! themselves and `boot.s`'s `secondary_boot` (the trampoline that page holds) for what runs there.
//!
//! **This module's whole job is the one thing INIT-SIPI-SIPI cannot do for itself**: getting the
//! trampoline's bytes onto that low page and handing it the one value that varies per core (the
//! stack top), before the messages are sent.
//!
//! # BUGS
//!
//! **Three failures were found bringing this up, and all three are now root-caused and fixed.**
//! They are kept here because this is where each was found and where a reader meets the symptom.
//! `helpers/qemu-runner-x86_64.sh` defaults `NIFE_SMP` to **2** since 2026-09-23; the last thing
//! holding it at 1 was not a bring-up bug at all (see the end of #3). Three or more cores is still
//! opt-in and unmeasured on silicon.
//!
//! **1. A secondary was reported "did not start" when it had started. FIXED 2026-09-19 (milestone
//! 161).** The symptom as first recorded: at `-smp 3` and above, one secondary per boot typically
//! failed to come online, *which* one varied, and the online count fell one short. Two hypotheses
//! were tested and refuted (a `hlt` wait instead of a spin, in case CPU 0 starved the target's vCPU
//! thread; copying the trampoline once instead of per `STARTUP` IPI, in case of QEMU's
//! self-modifying-code detection), and both changes are kept on their own merits.
//!
//! **Neither hypothesis could have worked, because no core was failing.** Reproduced 2026-09-19 on
//! patagonia at `-smp 4`, plain boot, bounded, 40 runs: **26 of 40 had a core reported as not
//! started** (cores 1, 2 and 3 all took turns, including the *first* secondary, so "third or
//! later" was never the boundary), and the transcript of the very first failure gave it away: the
//! "failed" core printed its own `cr4.smep : set on core 2` line, which only `secondary_main` on
//! that core can print, one line *before* `smp: cpu 2 did not start`. An instrumented build then
//! printed the count at each point of `super::cpu_start`, and in all three failures of ten it read
//! the same: **online count at the INIT = n, at the wait loop = n + 1, now = n + 1.**
//!
//! `cpu_start` read the online count before the INIT and then *read it again* just before its wait
//! loop, after the `STARTUP` IPIs and their 200 us settle delays, and waited for the count to move
//! from that second value. A core fast enough to check in during those delays had already moved
//! it, so the loop waited its full ten seconds for a second increment that nobody would make,
//! returned -1, and `smp::bring_up_secondaries` counted an online, running core as absent. Under
//! TCG "fast enough" is a matter of which vCPU thread the host schedules when, which is why it was
//! intermittent and why which core it hit varied. (The earlier port-I/O checkpoint evidence, a core
//! reaching long mode and "never reaching" the checkpoint before `jmp secondary_main`, is not
//! reproduced by anything above and is best read as the instrumentation losing a race of its
//! own; it is recorded rather than explained.)
//!
//! **The fix is one line removed**: the count is read once, before the INIT, and every wait
//! compares against that. Measured after, same host, same bounded loop: **40 of 40 boots at
//! `-smp 4`, 20 of 20 at `-smp 8` (this kernel's `MAX_CPUS`) and 20 of 20 at `-smp 3` brought every
//! core online**, against 14 of 40 before. All of it is QEMU TCG; the defect is in this port's own
//! code rather than in the emulator, so there is no reason to expect silicon to differ, but xenon
//! has not been asked.
//!
//! **2. Two cores crashed under the kernel's own test suite's real scheduler workload. FIXED
//! 2026-08-25 (milestone 161's SMP-crash lane); root cause was a missing cross-core TLB
//! shootdown**, recorded here because this is where the symptom was found and where a reader
//! meets it.
//!
//! The symptom was a `script/test` run at `NIFE_SMP=2` faulting with `rip` = `stack::PAINT`
//! (`0x5afe57ac5afe57ac`), the pattern this kernel writes into a fresh kernel stack, or with
//! `rip` = 0. It reproduced on 10 of 10 runs.
//!
//! The cause was not in this file, and not in the portable scheduler either. `arch::x86_64::mmu`'s
//! `flush_tlb` was **local to the calling CPU**: `invlpg` invalidates one core's TLB and says
//! nothing about any other, and its own doc comment said so and predicted this
//! (*"a multi-CPU kernel needs a software shootdown protocol (an IPI) ... this is the line that
//! will need company"*). SMP arrived and the line never got its company. aarch64 needs none
//! (`tlbi ..., is` is broadcast by the hardware) and RISC-V already had one (an SBI RFENCE), which
//! is why the same portable `sched`/`thread` code runs clean at `-smp 4` on both.
//!
//! The failure that produced is exact rather than vague. `thread::KernelStack::drop` unmaps a dead
//! thread's stack and hands the address range back for reuse; a core that had cached a translation
//! for that range kept it, so when the range was remapped onto **different** physical frames the
//! stale core read the old frame instead. A `Context` read back that way is whatever the old frame
//! now holds, and since a freshly recycled stack page is painted, the `ret` at the end of
//! `switch_to` jumped to the paint. Confirmed by making a stale entry harmless (never reusing stack
//! address space, so no VA is ever remapped onto a new frame): the fault vanished, 8 runs of 8,
//! leaving only that test's own reuse assertion.
//!
//! The fix is `mmu::shoot_down_others`, and the one part of it worth knowing here is **why it is an
//! NMI**: `unmap_page` runs inside `KERNEL_MMU`, an `IrqSafeMutex`, so both the sender and every
//! other core running the same code have interrupts masked, and a maskable IPI would deadlock the
//! first time two cores spawned and reaped concurrently. notes/riscv-tlb-shootdown.md already
//! names that property as load-bearing for RISC-V, which gets it from M-mode; on x86 the NMI is the
//! only delivery `cli` cannot suppress. See that function's own doc comment.
//!
//! **3. A secondary is brought up and idles, but the suite cannot agree on which core booted.
//! FIXED 2026-09-17 (milestone 316)**; the diagnosis below stands and the fix is at the end of it.
//! `arch::x86_64::boot_cpu_id` read CPUID leaf 1's initial local APIC id, which is
//! *"which core am I"* and not *"which core booted"*. aarch64 answers a constant `0` and RISC-V
//! returns the hart id recorded at boot; only this port recomputes it per caller. So any test body
//! that §28's placement has migrated onto a secondary gets that secondary's id as "the boot core",
//! and `smp::tests::every_secondary_runs_scheduled_work` then waits for a `RAN_ON` mark the real
//! boot core never sets ("secondary core 0 never ran scheduled work"). `stack.rs`'s high-water
//! report skips a slot chosen the same way, so it scans a never-painted slot and reports a
//! secondary stack at 65536/65536. One cause, both symptoms, roughly half of runs at `NIFE_SMP=2`.
//! It reproduces with **no shootdown code present at all** (checked against the pre-fix tree), so
//! it predates #2's fix rather than following from it.
//!
//! **The predicted shape was right.** `boot.s`'s `_start_high` now stamps the id into
//! `arch::x86_64::BOOT_CPU_ID` after zeroing `.bss` and before calling `kernel_main`, and
//! `boot_cpu_id` reads that record; see its own doc. Code that runs exactly once, on exactly the
//! boot processor, cannot tell a later caller that the later caller is it, which is the property
//! RISC-V's `BOOT_HARTID` already had and this port was missing. Both symptoms went with it, and
//! the suite reaches `test result: ok. 245 passed` at two cores.
//!
//! **`NIFE_SMP` defaults to 2 since 2026-09-23**, by milestone 315 (a port revoke that reaches
//! every core), and #1, #2 and #3 are all closed.
//! The last thing holding the default at 1 was not an SMP bring-up bug at all:
//! `user::x86_port_tests::a_revoked_holder_faults_on_its_next_port_write` went red intermittently at
//! two cores, because `sched::delete_port_range_caps_impl` reset the TSS I/O bitmap on **the
//! revoker's core only**, so a holder running on the other core kept the ports until its next
//! context switch. That window was found by the audit in milestone 313 (the security audit that
//! was due since August), recorded and accepted at that function,
//! and it was a real confinement defect the second core made visible rather than a flake: 7 of 12
//! full two-core runs and 5 of 30 filtered ones, every failure the same assertion.
//! `segments::revoke_port_grant_everywhere` closes it by broadcasting the reset over #2's NMI.
//!
//! #2's fix is therefore **gated** now rather than only verified:
//! `user::tests::an_asid_flush_reaches_the_other_cpus`, the portable test milestone 58 (RISC-V TLB
//! shootdown) wrote for
//! exactly this property, fails on this port without the shootdown and passes with it, and with two
//! cores the default it runs on every `script/test --arch x86_64`.
//!
//! #1 was this port's own bug and not TCG's, so milestone 87's hardware is no longer needed to
//! settle it; xenon (`bench/xenon-2026-09-17/first-light-095500.log`, four processors) has still
//! never been asked to bring all four online, and the line to read there is `smp: 4 core(s)
//! online`.

use super::mmu::phys_to_virt;

unsafe extern "C" {
    /// The trampoline's own low VMA (`AP_TRAMPOLINE_PHYS`, link-x86_64.ld): where it executes from,
    /// and so also where its bytes must be copied to before a `STARTUP` IPI can name it.
    static __ap_trampoline_start: core::ffi::c_void;
    /// One past the trampoline's last byte, at the same low VMA. `__ap_trampoline_end -
    /// __ap_trampoline_start` is exactly how much [`prepare`] copies.
    static __ap_trampoline_end: core::ffi::c_void;
    /// Where the trampoline's bytes actually sit in the *loaded* image: link-x86_64.ld gives
    /// `.ap_trampoline` a low VMA but leaves its load address (LMA) an ordinary spot beside
    /// `.rodata`, which is where the loader actually put the bytes.
    static __ap_trampoline_lma: core::ffi::c_void;
    /// The one value the trampoline cannot know at link time: the target core's stack top.
    /// [`prepare`] writes it; `boot.s`'s 64-bit tail reads it back RIP-relative.
    static ap_trampoline_stack_top: u64;
}

/// The physical page a `STARTUP` IPI names: `secondary_boot`'s own address, which is also
/// `__ap_trampoline_start`'s, since the trampoline's VMA *is* where it runs from.
///
/// Page-aligned by construction (`link-x86_64.ld`'s `.ap_trampoline` names `AP_TRAMPOLINE_PHYS`,
/// itself a multiple of 4096), which is what lets [`super::cpu_start`] derive a `STARTUP` vector
/// from it with a plain shift.
pub fn trampoline_phys() -> u64 {
    (&raw const __ap_trampoline_start) as u64
}

/// How many bytes [`prepare`] copies: `__ap_trampoline_end - __ap_trampoline_start`.
pub fn trampoline_size() -> u64 {
    (&raw const __ap_trampoline_end) as u64 - trampoline_phys()
}

/// **Where the trampoline's bytes actually live in the loaded image**, physically: link-x86_64.ld
/// places `.ap_trampoline`'s LMA in the gap it deliberately opens between `.rodata` and `.data`
/// (see that file's own comment on why not `.boot_scratch` or the secondary stacks).
///
/// **Neither the direct map nor the image's own section-by-section map covers this range on its
/// own**, and both for the same reason: it sits inside `memory::image_start()..image_end()` (so
/// `mmu::map_everything`'s direct-map step skips it, the same way it skips `.text`, to avoid a
/// second writable alias), and it is *between* two mapped sections (`.rodata`, `.data`) rather than
/// inside either one, so neither section's own `map_range` reaches it either. `mmu::map_everything`
/// direct-maps it explicitly, by name, the same way `map_firmware_regions` already covers other
/// gaps the general rules miss.
pub fn trampoline_lma() -> u64 {
    (&raw const __ap_trampoline_lma) as u64
}

/// **Copy the trampoline to the page it has to execute from, and hand it this core's stack top.**
///
/// # Safety
/// Must not run while another core might still be reading the SAME copy: there is one trampoline
/// scratch page, shared by every `STARTUP` IPI this kernel ever sends. [`super::cpu_start`] is the
/// only caller, and it does not return until the core it just started is either fully online (past
/// the point it could still be reading this page) or given up on, which is what makes calling this
/// again, for the next core, safe.
pub unsafe fn prepare(stack_top: u64) {
    let dst = trampoline_phys();
    let src = trampoline_lma();
    let len = trampoline_size();

    // SAFETY: `src..src+len` is part of the loaded image (ordinary, file-backed bytes in the gap
    // between `.rodata` and `.data`; see the linker script's comment on why NOT `.boot_scratch` or
    // the secondary stacks, both of which are mutated at runtime), and `mmu::map_everything` maps
    // exactly this range into the direct map by name ([`trampoline_lma`]'s own doc explains why it
    // needs to: neither the general direct-map rule nor either neighboring section's own mapping
    // reaches it). `dst..dst+len` is the low megabyte, which `mmu::init`'s `map_firmware_regions`
    // maps writable (`Flags::kernel_data()`) and which the frame allocator never hands out
    // (`machine::LOW_MEGABYTE`), so nothing else owns those bytes. Non-overlapping: `dst` is a
    // fixed low page (`AP_TRAMPOLINE_PHYS`) and `src` sits beside `.rodata`, comfortably above 1 MiB.
    unsafe {
        core::ptr::copy_nonoverlapping(
            phys_to_virt(src) as *const u8,
            phys_to_virt(dst) as *mut u8,
            len as usize,
        );
    }

    let stack_top_slot = (&raw const ap_trampoline_stack_top) as u64;
    // SAFETY: `stack_top_slot` is inside the page just copied, reached through the direct map, and
    // this is the one caller (see this function's own safety contract).
    unsafe {
        core::ptr::write_volatile(phys_to_virt(stack_top_slot) as *mut u64, stack_top);
    }
}
