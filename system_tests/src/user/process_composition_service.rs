use super::*;
use crate::cap::{Rights, memory_region_cap, rendezvous_cap};
use crate::sched::RendezvousId;
use crate::thread::ThreadId;

/// The witness's budget, in pages. Counted rather than measured, from what it retypes: a
/// rendezvous, the child's address space and its tables, the child's two image pages and four of
/// stack, the timebase page on x86_64 and riscv64, a thread control block, the probe frame and the
/// tables under it, and the tables under the witness's own scratch window. That comes to about
/// thirty; the rest is headroom, so a failure to build reads as a defect rather than as a budget a
/// release build outgrew.
const BUDGET_PAGES: u64 = 64;

/// What the test needs back: the verdict line, and what to reap and reclaim once it has spoken.
pub struct Witness {
    /// The report endpoint carrying the verdict bits.
    pub report: RendezvousId,
    /// The witness's own thread. Its space is bound to it and goes when it is reaped.
    pub tid: ThreadId,
    /// The region the witness's thread control block was retyped from.
    pub thread_control_block_region: u64,
    /// The witness's budget: the child's space, its thread, the rendezvous and the probe frame.
    pub budget: u64,
}

/// Spawn the witness; returns its verdict line and what it holds.
///
/// It was role 19 of the `hello` multiplexer until milestone 291 (thirty-one programs wearing one
/// name) and is `fixtures/src/process_composition_witness.rs` now.
///
/// **Its endowment is the progenitor's shape**, which is `builder`'s: the archive read-only at
/// `INITRD_VA` with its length in `x1`, a memory region in slot 0 and a report line in slot 1. The
/// region is the spend-only `memory_region_cap` rather than the progenitor's delegable root, because
/// the witness passes no memory on; the claim is about the floor, so it gets the narrower one.
pub fn wire() -> Witness {
    let (initrd_start, initrd_len) = memory::initrd_region().expect("no initrd region");
    let initrd_pages = initrd_len.div_ceil(FRAME_SIZE);
    let bytes = program("process_composition_witness")
        .expect("no process_composition_witness in the archive");
    let elf = Elf::parse(bytes).expect("process_composition_witness is not loadable");

    // A hand-built space rather than `run`, because `run` maps its extra pages from a slice and the
    // archive is a few thousand of them. The same arithmetic as `authority_tests::spawn_tree`.
    let content: u64 = elf
        .segments()
        .map(|seg| {
            let (s, e) = seg.page_range(FRAME_SIZE);
            (e - s) / FRAME_SIZE
        })
        .sum::<u64>()
        + 1
        + initrd_pages / 512
        + crate::revoke::log_pages_for(initrd_pages)
        + INIT_STACK_PAGES
        + 8;
    let mut space = AddressSpace::new(content).expect("no memory for the witness");
    map_segments(&mut space, &elf).expect("could not lay out the witness");
    for k in 0..INIT_STACK_PAGES {
        space
            .map_new(USER_STACK_VA - k * FRAME_SIZE, Flags::user_data())
            .expect("could not map the witness's stack");
    }
    #[cfg(any(target_arch = "x86_64", target_arch = "riscv64"))]
    map_timebase_page(&mut space).expect("could not map the witness's timebase page");
    for i in 0..initrd_pages {
        space
            .map_physical(
                INITRD_VA + i * FRAME_SIZE,
                initrd_start + i * FRAME_SIZE,
                Flags::user_rodata(),
                crate::revoke::PageMapSource::NoCapability,
            )
            .expect("could not map the initrd");
    }
    let aspace = readopt_user_address_space(space).expect("register the witness's space");

    let report = crate::sched::create_rendezvous();
    let budget = crate::memory_region::create(BUDGET_PAGES).expect("no budget for the witness");
    let thread_control_block_region = crate::memory_region::create(2).expect("no tcb region");
    let tid = crate::sched::create_thread_control_block(thread_control_block_region)
        .expect("no tcb for the witness");
    for (want, cap, what) in [
        (0, memory_region_cap(budget), "the budget"),
        (1, rendezvous_cap(report, Rights::WRITE), "the report line"),
    ] {
        let slot = crate::sched::thread_control_block_insert_cap(tid, cap, None)
            .expect("insert a witness capability");
        assert_eq!(slot, want, "the witness's {what} must land in slot {want}");
    }
    crate::sched::configure_thread_control_block(tid, elf.entry(), USER_STACK_TOP, aspace)
        .expect("configure the witness");
    crate::sched::start_thread_control_block(tid, [0, initrd_len, 0]).expect("start the witness");

    Witness {
        report,
        tid,
        thread_control_block_region,
        budget,
    }
}
