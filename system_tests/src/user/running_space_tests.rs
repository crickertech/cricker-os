//! `running_space_tests`: a running address space stays nameable (§249 (a running address space
//! stays nameable), and its amendments), and what that lets milestone 95 (an unmap primitive, and
//! the mappings init never lets go) finally do.
//!
//! Each ruling has a test here, and each test a replayable falsification under
//! `system_tests/falsifications/user.running_space_tests.*`:
//!
//! - **Option A**: a copy of the capability made before `CONFIGURE` names the space while its
//!   thread runs, and `UNMAP` through it takes a page from under the thread, which faults.
//! - **The multicore test** `notes/unmap.md`'s `BUGS` said could not exist: the reader spins on one
//!   core and the `UNMAP` comes from another, so only the TLB shootdown can stop it.
//! - **Amendment (a)**: deleting every capability leaves the thread running on its space; the
//!   thread's reaping ends the space, and every capability to it then fails.
//! - **Amendment (b)**: a second bind of a bound space is refused, so §105 (`std::thread::spawn`
//!   stays declined) stands.
//! - **The corpse gap** `notes/naming-a-running-address-space.md` reasoned about and did not drive:
//!   a corpse whose root was in a destroyed region no longer outlives that region.
//! - **Milestone 95's negative control**: a builder that holds its own space, as the progenitor does
//!   at slot 28, writes to a page it filled for a child and faults; without its own space, the same
//!   write lands.
//!
//! Cross-ISA: every body is portable kernel code or portable userspace, and the hand-written reader
//! is one stub per ISA (DECISIONS §19 (architectural parity is a tenet)).
//!
//! # BUGS
//!
//! - **The multicore test needs two online cores** and skips on one. CI's QEMU legs boot four.
//! - **The multicore test's placement guard can fire on a correct kernel.** Nothing pins the reader
//!   away from the unmapper's core, so when the scheduler puts both on one core the guard's
//!   "proves nothing about another core's TLB" assertion fails the run instead of retrying. Seen
//!   once, on the cpu-matrix `rv64` leg of CI run 37405717281 (2026-10-06 UTC), on a branch that
//!   touched neither this file nor the scheduler. Pinning the reader, or retrying the placement a
//!   bounded number of times before failing, would make it a skip-or-retry rather than a red run.
//! - **The negative control proves the loader, not the boot.** It drives the same
//!   `supervision_protocol` code the progenitor builds boot servers with, in a fixture granted its
//!   own space the way the kernel grants the progenitor's. A boot that writes into a boot server
//!   would kill the progenitor, so the boot itself is proven only by building every server through
//!   that code (`script/swish-check`) without faulting.
//! - **The negative control keeps 32 frames the first time it runs in a boot, and none after.**
//!   Measured on aarch64, 2026-10-05, by running the control and the real run and then a third
//!   witness in one test: 32 frames gone after the first witness, the count unchanged across the
//!   next two. So it is one-time growth the first spawned program of a boot pays, not a frame per
//!   process; `scratch_window_tests` records the same 32 for its own first builder, unexplained.
//!   Run alone, this test is that first spawn; in the whole suite something earlier is.
//! - **The fault's kind is not asserted, only its address.** riscv64 reports a load or store page
//!   fault without saying whether a translation existed, so "it faulted at this page" is the claim
//!   all three ISAs can state alike.

use core::sync::atomic::{AtomicU64, Ordering};

use abi::Error;
use abi::fault::{EVENT_FAULT, FAULT_EP_SLOT};

use super::*;
use crate::cap::Rights;
use crate::sched;
use crate::syscall::invoke;

/// Where a hand-built program's code and stack go: the map's image base and top stack page, as
/// `supervision_tests` gives for its own.
const CODE_VA: u64 = address_space_map::IMAGE_BASE;
const STACK_VA: u64 = address_space_map::STACK_TOP_PAGE;
/// The page the reader spins on, aligned and clear of everything else a test maps.
const VA: u64 = address_space_map::pair_page(0x0060_0000);

/// **A thread that reads a page in a loop**, and counts in it so the kernel can see it running:
/// load the word at `VA` (its first argument), add one, store it back, branch to the load. It never
/// syscalls and never yields, so only a fault or a kill ends it.
#[cfg(target_arch = "aarch64")]
const READER_STUB: &[u32] = &[
    0xF940_0001, // ldr x1, [x0]
    0x9100_0421, // add x1, x1, #1
    0xF900_0001, // str x1, [x0]
    0x17FF_FFFD, // b   .-12
];
#[cfg(target_arch = "riscv64")]
const READER_STUB: &[u32] = &[
    0x0005_3583, // ld   a1, 0(a0)
    0x0015_8593, // addi a1, a1, 1
    0x00B5_3023, // sd   a1, 0(a0)
    0xFF5F_F06F, // j    .-12
];
/// `48 8b 07` `mov rax, [rdi]`; `48 ff c0` `inc rax`; `48 89 07` `mov [rdi], rax`; `eb f5` `jmp
/// .-11`; one `nop` to a word. Packed the way `user::x86_programs` packs its listings.
#[cfg(target_arch = "x86_64")]
const READER_STUB: &[u32] = &[0x4807_8B48, 0x8948_C0FF, 0x90F5_EB07];

/// `invoke` through the real dispatcher, the way a program's `svc`/`ecall` reaches it.
fn call(slot: u64, method: u64, a0: u64, a1: u64, a2: u64) -> Result<i64, Error> {
    let mut frame = TrapFrame::for_user_entry(0, 0, [0, 0, 0]);
    invoke(&mut frame, slot, method, a0, a1, a2)
}

fn wait_for(secs: u64, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = crate::arch::timer::now() + secs * crate::arch::timer::frequency();
    while crate::arch::timer::now() < deadline {
        if cond() {
            return true;
        }
        sched::yield_now();
    }
    cond()
}

/// Write `stub` into a fresh frame from `region` and map it at [`CODE_VA`] in `name`, then a stack
/// page and, when `counter` is asked for, a read/write page at [`VA`]. Returns that page's physical
/// address, or 0.
fn lay_out(region: u64, name: u64, stub: &[u32], counter: bool) -> u64 {
    let code = code_page(region, stub);
    let none = crate::revoke::PageMapSource::NoCapability;
    user_address_space_map(name, CODE_VA, code, Flags::user_code(), none).expect("map code");
    let stack = crate::memory_region::retype_page(region).expect("no stack frame");
    user_address_space_map(name, STACK_VA, stack, Flags::user_data(), none).expect("map stack");
    if !counter {
        return 0;
    }
    let page = crate::memory_region::retype_page(region).expect("no counter frame");
    user_address_space_map(name, VA, page, Flags::user_data(), none).expect("map the page");
    page
}

/// What the reader has counted so far, read through the direct map.
fn count(page: u64) -> u64 {
    // SAFETY: `page` is a frame retyped for this test and mapped into the reader; the direct map
    // names it, and a torn read of a counter only the reader writes is still a number.
    unsafe { core::ptr::read_volatile(mmu::phys_to_virt(page) as *const u64) }
}

/// **A reader, running.** Its space and TCB come out of `region`; it was bound through the real
/// `CONFIGURE`, which consumed one capability, while the test kept a copy made first. Returns
/// `(tid, name, kept, tcb_slot, counter_page)`.
fn start_reader(region: u64) -> (u64, u64, u64, u64, u64) {
    let name = user_address_space_create(region).expect("no address space");
    let page = lay_out(region, name, READER_STUB, true);
    let kept = sched::grant(crate::cap::address_space_cap(name, Rights::WRITE)).expect("grant");
    let given = sched::grant(crate::cap::address_space_cap(name, Rights::WRITE)).expect("grant");
    let tid = sched::create_thread_control_block(region).expect("no tcb");
    let tcb = sched::grant(crate::cap::thread_control_block_cap(tid, Rights::ALL)).expect("grant");
    assert_eq!(
        call(
            tcb,
            abi::thread_control_block::CONFIGURE,
            CODE_VA,
            STACK_VA + page_frames::FRAME_SIZE,
            given,
        ),
        Ok(0),
        "premise: CONFIGURE refused the reader's space",
    );
    assert!(
        sched::current_cap(given).is_err(),
        "CONFIGURE no longer consumes the capability it is passed, which §249 kept",
    );
    sched::start_thread_control_block(tid, [VA, 0, 0]).expect("start");
    assert!(
        wait_for(2, || count(page) > 0),
        "premise: the reader never ran, so nothing below is about a running space",
    );
    (tid, name, kept, tcb, page)
}

/// The address the most recent user fault named, if one has happened since `before`.
fn fault_since(before: usize) -> Option<u64> {
    (crate::arch::exceptions::USER_FAULTS.load(Ordering::Acquire) > before)
        .then(|| crate::arch::exceptions::last_user_fault().map(|(_, addr)| addr))
        .flatten()
}

/// **A capability copied before `CONFIGURE` still names the running space, and `UNMAP` through it
/// takes a page from under the thread, which faults on its next touch** (§249's option A).
///
/// Until 2026-10-05 `CONFIGURE` retired the space's name, so the kept copy resolved to nothing and
/// `UNMAP` answered `BadPointer`: milestone 95's method existed and could not reach the one window
/// it was for.
///
/// Falsification: replayable `system_tests/falsifications/user.running_space_tests.unmap_through_a_copy_made_before_configure_faults_the_running_thread.patch`
#[test_case]
fn unmap_through_a_copy_made_before_configure_faults_the_running_thread() {
    let region = crate::memory_region::create(16).expect("no region");
    let (tid, _name, kept, tcb, page) = start_reader(region);
    let faults = crate::arch::exceptions::USER_FAULTS.load(Ordering::Acquire);

    assert_eq!(
        call(kept, abi::address_space::UNMAP, VA, 0, 0),
        Ok(0),
        "a capability made before CONFIGURE no longer names the space its thread runs in",
    );
    assert!(
        wait_for(2, || !sched::is_thread_present(tid)),
        "the reader kept running after its page was given up: it counted to {}",
        count(page),
    );
    assert_eq!(
        fault_since(faults),
        Some(VA),
        "the reader ended, but not by faulting on the page UNMAP took",
    );

    for s in [kept, tcb] {
        let _ = sched::delete_current_cap(s);
    }
    sched::reclaim_region(region).expect("the reader's region did not come back");
}

/// What the unmapping thread saw, for the test thread to read back.
static UNMAP_RESULT: AtomicU64 = AtomicU64::new(u64::MAX);
static UNMAPPER_CPU: AtomicU64 = AtomicU64::new(u64::MAX);
static READER_CPU_AT_UNMAP: AtomicU64 = AtomicU64::new(u64::MAX);

/// **The reader spins on one core, `UNMAP` runs on another, and the reader faults.**
///
/// The test `notes/unmap.md`'s `BUGS` said could not exist before §249: no capability named a space
/// a thread was running in, so no core could be caching a translation `UNMAP` had to reach. Here
/// one is. The reader's core has the page in its TLB from the loop, and nothing on that core runs
/// the unmap; only the shootdown `mmu::unmap_user_at` issues (`tlbi vaae1is`, an SBI remote fence,
/// the NMI round) can take the translation away from it. Which core each side was on is read, not
/// assumed: the reader's from its own current-CPU page at the moment of the unmap, the unmapper's
/// from `cpu::id()`.
///
/// Falsification: replayable `system_tests/falsifications/user.running_space_tests.unmap_on_another_core_faults_a_reader_spinning_on_the_page.patch`
#[test_case]
fn unmap_on_another_core_faults_a_reader_spinning_on_the_page() {
    if crate::smp::online_count() < 2 {
        crate::testing::skip!("needs two online cores: the reader and the unmapper must differ");
    }
    let region = crate::memory_region::create(16).expect("no region");
    let (tid, name, kept, tcb, page) = start_reader(region);
    let cpu_page =
        with_user_address_space(name, |s| s.and_then(|s| s.current_cpu_page_kernel_va()))
            .expect("a bound space has no current-cpu page");
    // SAFETY: the reader's own current-CPU frame, through the direct map; it lives as long as the
    // space, which outlives this test's reads (the space dies with the reader, after them).
    let reader_cpu = || unsafe { current_cpu_protocol::CurrentCpuPage::new(cpu_page) }.cpu();
    // **Placement is retried, not pinned, and a retry is weaker than pinning.** The kernel has no
    // affinity primitive (the syscall surface is an architect's call, and `cpu.rs` records why
    // tests are not pinned), so `spawn_on` is a hint: DECISIONS §28 (SMP placement: two random choices at spawn), idle stealing can pull the unmapper
    // onto the reader's core, or the reader onto the unmapper's, before either runs. The unmapper
    // therefore checks where the reader is and stands down, unmapping nothing, when it shares its
    // core; the test then picks a fresh target and tries again, up to `ATTEMPTS` times. What
    // remains is the window between that check and the `UNMAP` itself, which the assertion below
    // still catches (and which would only fail a run, never pass one wrongly).
    const ATTEMPTS: usize = 16;
    const STOOD_DOWN: u64 = 2;
    let faults = crate::arch::exceptions::USER_FAULTS.load(Ordering::Acquire);
    let mut target = 0;
    let mut placed = false;
    for _ in 0..ATTEMPTS {
        let here = reader_cpu().expect("premise: the reader has run, so its page names a core");
        target = crate::smp::online_cpus()
            .find(|&c| c != here)
            .expect("two cores online and none but the reader's");
        for a in [&UNMAP_RESULT, &UNMAPPER_CPU, &READER_CPU_AT_UNMAP] {
            a.store(u64::MAX, Ordering::Relaxed);
        }
        sched::spawn_on(target, move || {
            // SAFETY: as above.
            let theirs = unsafe { current_cpu_protocol::CurrentCpuPage::new(cpu_page) }.cpu();
            let me = crate::cpu::id();
            if theirs == Some(me) {
                UNMAP_RESULT.store(STOOD_DOWN, Ordering::Release);
                return;
            }
            let slot = sched::grant(crate::cap::address_space_cap(name, Rights::WRITE))
                .expect("grant the unmapper its capability");
            READER_CPU_AT_UNMAP.store(theirs.map_or(u64::MAX - 1, |c| c as u64), Ordering::Relaxed);
            UNMAPPER_CPU.store(me as u64, Ordering::Relaxed);
            let r = call(slot, abi::address_space::UNMAP, VA, 0, 0);
            let _ = sched::delete_current_cap(slot);
            UNMAP_RESULT.store(if r == Ok(0) { 0 } else { 1 }, Ordering::Release);
        })
        .expect("spawn the unmapper");
        assert!(
            wait_for(2, || UNMAP_RESULT.load(Ordering::Acquire) != u64::MAX),
            "the unmapper never ran on cpu {target}",
        );
        if UNMAP_RESULT.load(Ordering::Acquire) != STOOD_DOWN {
            placed = true;
            break;
        }
    }
    assert!(
        placed,
        "the scheduler put the unmapper on the reader's core {ATTEMPTS} times running (last target \
         cpu {target}); with no affinity primitive this test cannot place them apart",
    );
    assert_eq!(
        UNMAP_RESULT.load(Ordering::Acquire),
        0,
        "UNMAP of the running reader's page was refused",
    );
    let (them, us) = (
        READER_CPU_AT_UNMAP.load(Ordering::Relaxed),
        UNMAPPER_CPU.load(Ordering::Relaxed),
    );
    assert_ne!(
        them, us,
        "the reader was on the unmapper's own core ({us}) when it unmapped, so this run proves \
         nothing about another core's TLB",
    );

    assert!(
        wait_for(2, || !sched::is_thread_present(tid)),
        "the reader on cpu {them} kept counting ({}) after cpu {us} gave its page up: its core \
         still holds the translation, and UNMAP's flush did not reach it",
        count(page),
    );
    assert_eq!(
        fault_since(faults),
        Some(VA),
        "the reader ended, but not by faulting on the page UNMAP took",
    );

    for s in [kept, tcb] {
        let _ = sched::delete_current_cap(s);
    }
    sched::reclaim_region(region).expect("the reader's region did not come back");
}

/// **A second bind of a bound space is refused, and a refused `CONFIGURE` consumes nothing**
/// (§249's amendment (b)). Two embryos, one space, two capabilities to it made before either bind.
/// The first binds; the second is answered `WrongObject`, the answer a second `CONFIGURE` of a
/// started thread gives. Before §249 the consumed name was the only thing that stopped this, and
/// §105 (`std::thread::spawn` stays declined) stood on it.
///
/// And the bound space does not outlive its embryo: reclaiming the region reaps both TCBs, and the
/// space bound to one of them goes in the same `DESTROY`, collected by the sweep as a space whose
/// thread is gone.
///
/// Falsification: replayable `system_tests/falsifications/user.running_space_tests.a_second_bind_of_a_bound_space_is_refused.patch`
#[test_case]
fn a_second_bind_of_a_bound_space_is_refused() {
    let region = crate::memory_region::create(16).expect("no region");
    let name = user_address_space_create(region).expect("no address space");
    let first = sched::grant(crate::cap::address_space_cap(name, Rights::WRITE)).expect("grant");
    let second = sched::grant(crate::cap::address_space_cap(name, Rights::WRITE)).expect("grant");
    let mut tcbs = [0; 2];
    for t in tcbs.iter_mut() {
        let tid = sched::create_thread_control_block(region).expect("no tcb");
        *t = sched::grant(crate::cap::thread_control_block_cap(tid, Rights::ALL)).expect("grant");
    }
    let configure = |tcb, aspace| {
        call(
            tcb,
            abi::thread_control_block::CONFIGURE,
            CODE_VA,
            STACK_VA + page_frames::FRAME_SIZE,
            aspace,
        )
    };

    assert_eq!(configure(tcbs[0], first), Ok(0), "premise: the first bind");
    assert_eq!(
        configure(tcbs[1], second),
        Err(Error::WrongObject),
        "a second thread was bound to a space already bound to one: §105 rests on this refusal",
    );
    assert!(
        sched::current_cap(second).is_ok(),
        "a refused CONFIGURE consumed the capability it refused",
    );

    for s in [second, tcbs[0], tcbs[1]] {
        let _ = sched::delete_current_cap(s);
    }
    sched::reclaim_region(region).expect("the region did not come back");
    assert!(
        user_address_space_root(name).is_none(),
        "a space bound to an embryo the region's DESTROY reaped outlived it",
    );
}

/// **Deleting every capability to a bound space leaves its thread running; reaping the thread ends
/// the space, and every capability to it then fails** (§249's amendment (a)).
///
/// The first half is the absence of a final-capability rule, which nothing here has ever had. The
/// second half is the one that needs a mechanism now that the registry owns bound spaces: the
/// reaper must take the space out by the name the thread kept, or a dead process's memory stays
/// nameable (and its root, its ASID and its revocation records stay spoken for) forever.
///
/// It also prints what the registry costs since `MAX_USER_SPACES` grew, measured from the type.
///
/// Falsification: replayable `system_tests/falsifications/user.running_space_tests.a_bound_space_dies_with_its_thread_not_with_its_capabilities.patch`
#[test_case]
fn a_bound_space_dies_with_its_thread_not_with_its_capabilities() {
    crate::println!(
        "    the address-space registry: {} bytes of .bss",
        registry_footprint()
    );
    let region = crate::memory_region::create(16).expect("no region");
    let (tid, name, kept, tcb, page) = start_reader(region);

    let _ = sched::delete_current_cap(kept);
    let after = count(page);
    assert!(
        wait_for(2, || count(page) > after),
        "the reader stopped when the last capability to its space was deleted",
    );
    assert!(
        user_address_space_root(name).is_some(),
        "deleting every capability to a bound space ended it",
    );

    assert!(
        sched::kill_thread(tid),
        "premise: the reader was alive to kill"
    );
    assert!(
        wait_for(2, || !sched::is_thread_present(tid)),
        "premise: the killed reader was never reaped",
    );
    assert!(
        user_address_space_root(name).is_none(),
        "a space outlived the thread it was bound to: the reaper did not take it from the registry",
    );
    let late = sched::grant(crate::cap::address_space_cap(name, Rights::WRITE)).expect("grant");
    assert_eq!(
        call(late, abi::address_space::UNMAP, VA, 0, 0),
        Err(Error::BadPointer),
        "a capability to a dead space still reached it",
    );

    for s in [late, tcb] {
        let _ = sched::delete_current_cap(s);
    }
    sched::reclaim_region(region).expect("the reader's region did not come back");
}

/// **A corpse whose space is rooted in a destroyed region does not keep that space past the
/// region** (the gap `notes/naming-a-running-address-space.md` found by reading, confirmed closed).
///
/// The shape: the space and its root from region R, the TCB from region A, and the thread
/// supervised, so its fault leaves it `Dead` and unreaped. Before §249 the corpse owned its space,
/// `DESTROY(R)` saw neither it (its TCB is not in R) nor its space (bound spaces were not in the
/// registry), and R came back with the corpse still holding a root in it. The revocation registry
/// went on naming that root and its log pages, which R had just handed back, and the corpse's
/// eventual reap would `forget_root` a page that could by then be someone else's root.
///
/// Now the registry owns the space and the sweep takes a corpse's: the name is dead and the
/// revocation registry has forgotten the root before R's pages are free.
///
/// Falsification: replayable `system_tests/falsifications/user.running_space_tests.a_corpse_does_not_keep_a_space_rooted_in_a_destroyed_region.patch`
#[test_case]
fn a_corpse_does_not_keep_a_space_rooted_in_a_destroyed_region() {
    let space_region = crate::memory_region::create(16).expect("no space region");
    let tcb_region = crate::memory_region::create(2).expect("no tcb region");
    // The supervision endpoint comes from a region this test owns and gives back, not from
    // `sched::create_rendezvous`, whose kernel chunks are never freed (see the BUGS on
    // `testing::SUITE_PAGE_FRAME_BUDGET`).
    let ep_region = crate::memory_region::create(1).expect("no endpoint region");
    let supervisor = sched::create_rendezvous_from(ep_region).expect("no supervision rendezvous");
    let name = user_address_space_create(space_region).expect("no address space");
    lay_out(
        space_region,
        name,
        super::supervision_tests::FAULT_STUB,
        false,
    );
    let tid = sched::create_thread_control_block(tcb_region).expect("no tcb");
    sched::thread_control_block_insert_cap(
        tid,
        crate::cap::rendezvous_cap(supervisor, Rights::READ),
        Some(FAULT_EP_SLOT),
    )
    .expect("insert the supervision endpoint");
    sched::configure_thread_control_block(tid, CODE_VA, STACK_VA + page_frames::FRAME_SIZE, name)
        .expect("configure");
    sched::start_thread_control_block(tid, [0; 3]).expect("start");

    let msg = sched::ipc_receive(supervisor);
    assert_eq!(msg[0], EVENT_FAULT, "premise: the child did not fault");
    assert!(
        wait_for(2, || sched::with_binders(|b| b(tid))
            == sched::Binder::Corpse),
        "premise: the corpse never left its core",
    );
    let root = user_address_space_root(name).expect("premise: a corpse's space is still named");
    assert!(
        !matches!(
            crate::revoke::list_mapping(root, 0),
            crate::revoke::Listing::Done
        ),
        "premise: the corpse's space has mappings on record",
    );

    sched::reclaim_region(space_region).expect("R's DESTROY was refused");

    assert!(
        user_address_space_root(name).is_none(),
        "a corpse kept a space whose root came from a region that has just been destroyed",
    );
    assert!(
        matches!(
            crate::revoke::list_mapping(root, 0),
            crate::revoke::Listing::Done
        ),
        "the revocation registry still names a root, and log pages, its region gave back",
    );

    sched::reap_supervised(supervisor, tid).expect("the corpse could not be reaped");
    assert!(
        !sched::is_thread_present(tid),
        "the reaped corpse is still in the table"
    );
    // Only the endpoint's region is ours to give back. `reap_supervised` already reclaimed
    // `tcb_region` (it is the reap's whole teardown), so its name is dead and a second
    // `reclaim_region` on it is refused.
    sched::reclaim_region(ep_region).expect("the supervision endpoint's region did not come back");
}

/// The witness's three report words. Mirrored from `fixtures/src/scratch_release_witness.rs`, as
/// `authority_tests` mirrors `root_supervisor`'s.
#[cfg(initrd)]
const REPORT_BUILT: u64 = 1;
#[cfg(initrd)]
const REPORT_WRITE_LANDED: u64 = 2;
#[cfg(initrd)]
const REPORT_FAILED: u64 = 9;

/// **Start the witness, with or without a capability to its own space at slot 3**, the way the
/// kernel starts the progenitor (`user::boot_progenitor`): a space it builds and names, caps
/// inserted by slot, `CONFIGURE` by name. Returns `(tid, report, regions)`; the last region holds
/// `report`, so the caller gives it back after the others.
#[cfg(initrd)]
fn start_witness(own_space: bool) -> (u64, sched::RendezvousId, [u64; 4]) {
    let bytes = program("scratch_release_witness").expect("no scratch_release_witness program");
    let elf = Elf::parse(bytes).expect("scratch_release_witness is not loadable");
    let content: u64 = elf
        .segments()
        .map(|seg| {
            let (s, e) = seg.page_range(FRAME_SIZE);
            (e - s) / FRAME_SIZE
        })
        .sum::<u64>()
        + 8;
    let mut space = AddressSpace::new(content).expect("no memory for the witness");
    map_segments(&mut space, &elf).expect("could not lay out the witness");
    space
        .map_new(USER_STACK_VA, Flags::user_data())
        .expect("could not map the witness's stack");
    #[cfg(any(target_arch = "x86_64", target_arch = "riscv64"))]
    map_timebase_page(&mut space).expect("could not map the witness's timebase page");
    let name = readopt_user_address_space(space).expect("register the witness's space");

    // From a region the caller reclaims, not `sched::create_rendezvous`'s never-freed chunks.
    let ep_region = crate::memory_region::create(1).expect("no endpoint region");
    let report = sched::create_rendezvous_from(ep_region).expect("no report rendezvous");
    // Seven stack pages and a one-page child with its stack, tables, space and thread, with room
    // over; the scratch window's first table and the two above it.
    let budget = crate::memory_region::create(72).expect("no budget");
    let tables = crate::memory_region::create(8).expect("no table budget");
    let tcb_region = crate::memory_region::create(2).expect("no tcb region");
    let tid = sched::create_thread_control_block(tcb_region).expect("no tcb");
    let caps = [
        crate::cap::memory_region_cap(budget),
        crate::cap::rendezvous_cap(report, Rights::WRITE),
        crate::cap::memory_region_cap(tables),
    ];
    for (want, cap) in caps.into_iter().enumerate() {
        let slot = sched::thread_control_block_insert_cap(tid, cap, Some(want as u64))
            .expect("insert a witness capability");
        assert_eq!(slot, want as u64);
    }
    if own_space {
        sched::thread_control_block_insert_cap(
            tid,
            crate::cap::address_space_cap(name, Rights::WRITE),
            Some(3),
        )
        .expect("insert the witness's own space");
    }
    sched::configure_thread_control_block(tid, elf.entry(), USER_STACK_TOP, name)
        .expect("configure the witness");
    sched::start_thread_control_block(tid, [0; 3]).expect("start the witness");
    (tid, report, [budget, tables, tcb_region, ep_region])
}

/// The witness's next report, or `None` if it sent nothing within two seconds.
#[cfg(initrd)]
fn next_report(report: sched::RendezvousId) -> Option<[u64; 5]> {
    wait_for(2, || sched::rendezvous_waiting_senders(report) > 0)
        .then(|| sched::ipc_receive(report))
}

/// **A builder that gave its scratch page up faults writing to the child's page it filled; one that
/// could not give it up writes it** (milestone 95's negative control, the shape milestone 22 (trusted init) used).
///
/// Both runs build one child with `supervision_protocol::build_child`, the code the progenitor builds
/// every boot server with, and then write to the scratch page the loader filled the child's code
/// through. Given a capability to its own space (as the kernel gives the progenitor at slot 28), the
/// loader `UNMAP`ped that page as soon as it was in the child, and the write faults at exactly that
/// address. Without one, the write lands, which is the window the progenitor held onto every boot
/// server's memory until §249; that second run is what stops the first passing because nothing was
/// mapped there in the first place.
///
/// Falsification: replayable `system_tests/falsifications/user.running_space_tests.a_builder_that_gave_its_scratch_up_faults_writing_to_a_childs_page.patch`
#[cfg(initrd)]
#[test_case]
fn a_builder_that_gave_its_scratch_up_faults_writing_to_a_childs_page() {
    // The control first: the window exists without the capability.
    let (tid, report, regions) = start_witness(false);
    let built = next_report(report).expect("the control witness never reported");
    assert_eq!(
        built[0], REPORT_BUILT,
        "the control witness could not build its child"
    );
    let va = built[1];
    let landed = next_report(report).expect("the control witness's write never landed");
    assert_eq!(
        landed[0], REPORT_WRITE_LANDED,
        "premise: without its own space the builder's window onto the child's page must be open, \
         or the run below proves nothing",
    );
    assert!(
        wait_for(2, || !sched::is_thread_present(tid)),
        "the control witness never exited"
    );
    for r in regions {
        sched::reclaim_region(r).expect("a control witness region did not come back");
    }

    // Now with its own space, as the progenitor has it.
    let faults = crate::arch::exceptions::USER_FAULTS.load(Ordering::Acquire);
    let (tid, report, regions) = start_witness(true);
    let built = next_report(report).expect("the witness never reported");
    assert_ne!(
        built[0], REPORT_FAILED,
        "the witness could not build its child"
    );
    assert_eq!(
        built[1], va,
        "premise: both runs filled the child through the same page"
    );
    assert!(
        next_report(report).is_none(),
        "the builder wrote to its child's page after the loader should have given it up",
    );
    assert!(
        wait_for(2, || !sched::is_thread_present(tid)),
        "the witness neither faulted nor exited after its build",
    );
    assert_eq!(
        fault_since(faults),
        Some(va),
        "the witness ended, but not by faulting on the scratch page it gave up",
    );
    for r in regions {
        sched::reclaim_region(r).expect("a witness region did not come back");
    }
}
