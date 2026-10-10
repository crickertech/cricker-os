//! `std_layout`: where a `std` child's endowment goes in its table and its space. Moved out of
//! `lib.rs` unchanged but for visibility and one link by milestone 812 (`std::thread::spawn` runs
//! real threads in one address space), whose three new slots grew it past §266 (a Rust source file
//! stays under 2,000 lines)'s baseline.
//!
//! Name: provisional (milestone 812's lane, 2026-10-10 UTC).

/// **A `std` child's endowment, placed where nife's `std` reads it** (milestone 595 (provisional)).
///
/// The progenitor's native spawn fills a child's capability table from slot 0 in a documented order,
/// output first. nife's `std` fixes a slot per authority instead (`crates/std_runtime_protocol`), and
/// the two disagree about slot 0 itself: a native child's output is a `std` child's heap budget.
/// This is the `std` shape, built from the same decisions [`spawn_service`](super::spawn_service)
/// has made (which output, which directory, which of the manifest's pages).
///
/// - slot 0: the job's own region, narrowed to `WRITE`, as the heap's budget. See
///   [`grant_plan::STD_REGION_PAGES`] for why it is the same region the child is built from.
/// - slot 1: the output, exactly the capability a native child would get at slot 0.
/// - slot 4 and a page at `FS_PAGE`: the caretaker's narrowed endpoint, if the line granted a
///   directory. The same frame the caretaker and the file server map, at the `std` address.
/// - slot 5 and a read-only page at `CLOCK_PAGE`, slot 7 and one at `CONFIG_PAGE`: the manifest's
///   clock and configuration pages.
/// - slot 6: the entropy service, `WRITE`, if the manifest declared it and this boot built one.
/// - slot 8 and a read-only page at `ARGS_PAGE`: the line's argv (milestone 205, DECISIONS §170),
///   if the shell sent one. The page is the child's own, copied out of the shell's frame.
///
/// Slots 2 and 3, the network, stay empty: `grant_plan`'s
/// `a_std_program_declares_only_what_the_std_layout_can_hold` keeps any `std` manifest from asking.
///
/// Name: provisional.
///
/// # BUGS
///
/// - **The child holds `WRITE` on the region it is built in**, because that region is its heap
///   (`grant_plan::STD_REGION_PAGES` says why one region). `WRITE` on a region is also `SPLIT`,
///   and a region that has been split cannot be destroyed until its children are, so a program
///   that splits its own heap pins its job region: `job_undertaker` never reclaims it, and the
///   pool is one region smaller until reboot. nife's `std` never splits (its allocator only
///   `MAP`s), so this takes a program written to do it. Closing it needs a right that allows `MAP`
///   and not `SPLIT`, which is the syscall surface and an architect's call.
/// - **The directory half is built and never exercised at the prompt.** No `std` manifest declares
///   a directory yet. DECISIONS §170 (how a foreign program is told what to do) ruled that the
///   directories granted on a line bound what a word reaches, and granting one to a `std` program
///   from the prompt is milestone 205's designation half, not built yet. The kernel harness proves
///   the same slot and page from its side (`fs_service::start_std_full`).
/// - **The network half is not wired.** The progenitor would have to mint slot 3's socket-frame
///   budget as well as place slot 2, and nothing needs it yet.
pub(super) struct StdLayout {
    pub(super) caps: [(u64, u64); 2],
    placed: [(u64, u64, u64); 8],
    placed_n: usize,
    maps: [(u64, u64, u64); 4],
    maps_n: usize,
}

// `caps` lands in order from slot 0, so the two in-order slots must be 0 and 1. Checked here rather
// than assumed, because the contract crate could renumber them and this would build a child whose
// heap and output were swapped.
const _: () = assert!(
    std_runtime_protocol::MEMORY_REGION_SLOT == 0 && std_runtime_protocol::STDOUT_SLOT == 1
);

impl StdLayout {
    pub(super) fn new(
        region: u64,
        out: (u64, u64),
        dir: Option<(u64, u64)>,
        clock: Option<u64>,
        config: Option<u64>,
        entropy: Option<u64>,
        args: Option<u64>,
    ) -> Self {
        use std_runtime_protocol as rt;
        let mut l = StdLayout {
            caps: [(region, abi::rights::WRITE), out],
            placed: [(0, 0, 0); 8],
            placed_n: 0,
            maps: [(0, 0, 0); 4],
            maps_n: 0,
        };
        let place = |l: &mut StdLayout, slot: u64, cap: u64, rights: u64| {
            l.placed[l.placed_n] = (slot, cap, rights);
            l.placed_n += 1;
        };
        let map = |l: &mut StdLayout, va: u64, cap: u64, mode: u64| {
            l.maps[l.maps_n] = (va, cap, mode);
            l.maps_n += 1;
        };
        if let Some((ep, page)) = dir {
            place(&mut l, rt::FS_DIR_SLOT, ep, abi::rights::WRITE);
            map(&mut l, rt::FS_PAGE, page, abi::address_space::MAP_RW);
        }
        if let Some(page) = clock {
            place(&mut l, rt::CLOCK_SLOT, page, abi::rights::READ);
            map(&mut l, rt::CLOCK_PAGE, page, abi::address_space::MAP_RO);
        }
        if let Some(ep) = entropy {
            place(&mut l, rt::ENTROPY_SLOT, ep, abi::rights::WRITE);
        }
        if let Some(page) = config {
            place(&mut l, rt::CONFIG_SLOT, page, abi::rights::READ);
            map(&mut l, rt::CONFIG_PAGE, page, abi::address_space::MAP_RO);
        }
        if let Some(page) = args {
            place(&mut l, rt::ARGS_SLOT, page, abi::rights::READ);
            map(&mut l, rt::ARGS_PAGE, page, abi::address_space::MAP_RO);
        }
        // **Threads** (milestone 812 (`std::thread::spawn` runs real threads in one address
        // space)): every `std` program is built as a process of its own and may add threads to it,
        // within its own region's budget. Its own process with `BIND` and nothing more, its own
        // first thread to set its thread pointer through, and its own space with `READ` for the
        // futex waits its locks make. None of the three reaches anything outside the program.
        place(
            &mut l,
            rt::PROCESS_SLOT,
            supervision_protocol::CHILDS_OWN_PROCESS,
            abi::rights::BIND,
        );
        place(
            &mut l,
            rt::THREAD_SLOT,
            supervision_protocol::CHILDS_OWN_THREAD,
            abi::rights::WRITE,
        );
        place(
            &mut l,
            rt::SPACE_SLOT,
            supervision_protocol::CHILDS_OWN_SPACE,
            abi::rights::READ,
        );
        l
    }

    pub(super) fn placed(&self) -> &[(u64, u64, u64)] {
        &self.placed[..self.placed_n]
    }

    pub(super) fn maps(&self) -> &[(u64, u64, u64)] {
        &self.maps[..self.maps_n]
    }
}
