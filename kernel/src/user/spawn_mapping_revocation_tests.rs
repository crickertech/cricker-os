//! **Revocation against a mapping the kernel wired, rather than a mapping a capability made**
//! (the `map_physical` mapping record, 2026-09-21).
//!
//! **The defect this module was written to demonstrate was live in the tree and is fixed here.**
//! What follows describes the tree as it stood on 2026-09-21; [`AddressSpace::map_physical`] now
//! records, and `kernel/falsifications/` carries the patch that takes the record back out and turns
//! this test red.
//!
//! Every unmap sweep in `crate::revoke` is driven by the mapping log, and the only way into that
//! log is [`crate::revoke::record_mapping`]. `AddressSpace::map_physical` did not call it. So a
//! page the kernel wires into a process it is building (a [`super::Spawn`]`::maps` entry, a
//! [`super::DeviceRun`], the initrd read-only, the `x86_64` timebase page) was mapped into a live
//! address space with no record anywhere that it had been, and `PageFrame::REVOKE`,
//! `DeviceFrame::REVOKE` and `MemoryRegion::DESTROY` all walked straight past it. The capability
//! went and the mapping stayed, which is a take-back doing half its job.
//!
//! **This was reachable in an ordinary boot, and the path is one the tree wires deliberately.**
//! `user::fs_service::spawn_fs_server` puts the file channel's shared pages into the FS server
//! through `Spawn::maps`, and [`super::boot_progenitor`] hands the progenitor
//! `page_frame_cap(file_shared, WRITE|GRANT)` in slot 6 over the first of exactly those pages. A
//! `PageFrame::REVOKE` on that slot is a syscall the progenitor may make: it deleted every
//! capability naming the run and unmapped every mapping recorded under it, and the FS server kept
//! its writable mapping of a page the progenitor had just un-shared. The two halves were wired by
//! different modules, which is why nobody had put them side by side.
//!
//! **Why the fix is a required argument rather than a call.** `record_mapping` already takes
//! [`crate::revoke::PageMapSource`] as an argument with no default, for AGENTS.md's rung one: a
//! mapping that cannot say which capability made it is the record
//! DECISIONS §132 (what `PageFrame::REVOKE` owes an overlapping run) found missing. `map_physical`
//! was the
//! one mapping site that never had to answer, because it never recorded. It takes the same argument
//! now, so a caller must say, and recording is no longer something a caller can forget to do after
//! the fact: [`super::user_address_space_map`] used to be the one path that remembered, and it is
//! now the one path that has nothing extra to remember.
//!
//! **This module is a new file on purpose.** `kernel/src/user/tests.rs` is this tree's worst merge
//! hotspot and AGENTS.md's lane rules say to stay out of it. It is named to sort before
//! `thread_leak_police`, whose own header explains why that matters.
//!
//! # BUGS
//!
//! - **The test drives `PageFrame::REVOKE`'s sweep, not the other two.** `revoke_device_from_others`
//!   ([`DeviceFrame::REVOKE`]) and `revoke_region` ([`MemoryRegion::DESTROY`]) read the same log and
//!   are fixed by the same record, but nothing here drives a wired mapping through either, so read
//!   those as reasoned from the code rather than measured. `revoke_region`'s limb is the narrower
//!   one in practice: no `map_physical` call site in the tree maps a page that came from a
//!   `MemoryRegion`, so nothing reaches it today (see `map_physical`'s own docs).
//! - **The test proves the mapping is gone, not that the process faults on it.** Walking the page
//!   tables is what `crate::revoke`'s own suite asserts and it is what the sweep controls;
//!   demonstrating the fault needs a live process, which `user::live_swap_tests` does for the
//!   device case and at a whole boot's cost.
//! - **`AddressSpace::map_new` still does not record**, and is deliberately left alone. Its frames
//!   are retyped from the space's own backing region and freed with it, so no capability names them
//!   and no sweep can be asked about them. That is a different question from this one and its
//!   answer would be a different fix.

use paging::Flags;

use super::{
    AddressSpace, take_user_address_space, user_address_space_create, user_address_space_map,
    user_address_space_root,
};
use crate::arch::mmu;
use crate::revoke::PageMapSource;

/// Where the recorded mapping lands. Two distinct low addresses, so a walk of one space's tables
/// can never be mistaken for a walk of the other's.
const RECORDED_VA: u64 = address_space_map::pair_page(0x0050_0000);
/// Where the kernel-wired mapping lands.
const WIRED_VA: u64 = address_space_map::pair_page(0x0060_0000);

/// **A page wired into an address space by the kernel must be unmapped when its frame is revoked.**
///
/// The sequence is the one the boot path actually builds, with the two halves put side by side:
///
/// 1. One physical frame is mapped into two live address spaces. The first takes
///    [`user_address_space_map`], which is the `MAP_INTO` syscall's engine and records. The second
///    takes [`AddressSpace::map_physical`] directly, which is what `Spawn::maps` does for every
///    process the kernel builds.
/// 2. `PageFrame::REVOKE` runs over that frame, whole-machine
///    ([`crate::revoke::revoke_page_frame`], the single-page case of the syscall's own
///    `revoke_page_frame_run`).
/// 3. Neither space may still map it.
///
/// **Which assertion fires**, since this tree has been bitten by the readable one being unreachable
/// (`notes/confinement-claims.md`, milestones 305 and 307): the headline is the last assertion and
/// it is reached on every path, because both arms are plain page-table walks with no early return
/// between them. The two above it are a vacuity guard (the frame really was mapped in both places,
/// so there was something for the revoke to miss) and a premise check (the revoke really did reach
/// the mapping a sweep can see). Either failing means this test proved nothing, which is why they
/// say that rather than stating the claim a second time.
///
/// Falsification: replayable `kernel/falsifications/user.spawn_mapping_revocation_tests.a_page_the_kernel_wired_is_unmapped_when_its_frame_is_revoked.patch`
#[test_case]
fn a_page_the_kernel_wired_is_unmapped_when_its_frame_is_revoked() {
    // Out of the frame allocator rather than out of a region, which is where every real
    // `Spawn::maps` frame comes from too (`fs_service::page_frame`, `credential_service::page_frame`
    // and the stack-page loops all call `memory::alloc_zeroed`). It also keeps the revoke below a
    // pure capability question: `PageFrame::REVOKE` reclaims nothing, so nobody's budget moves.
    let shared = crate::memory::alloc().expect("no frame to share").addr();

    // The recorded half: a user-built address space, mapped through the `MAP_INTO` engine. Thirty-two
    // pages is the root, its tables, and its log page, with room to spare.
    let region = crate::memory_region::create(32).expect("no region for the recorded space");
    let recorded = user_address_space_create(region).expect("no recorded address space");
    user_address_space_map(
        recorded,
        RECORDED_VA,
        shared,
        Flags::user_data(),
        // The frame is named by no capability here, exactly as it is named by none at boot. The
        // record is filed under the page itself, which is what a single-page `REVOKE` looks for.
        PageMapSource::NoCapability,
    )
    .expect("the recorded space could not map the shared frame");
    let recorded_root = user_address_space_root(recorded).expect("the recorded space has no root");

    // The wired half: a space the kernel built and mapped into directly, which is `Spawn::maps`.
    let mut wired = AddressSpace::new(4).expect("no wired address space");
    wired
        .map_physical(
            WIRED_VA,
            shared,
            Flags::user_data(),
            PageMapSource::NoCapability,
        )
        .expect("the wired space could not map the shared frame");
    let wired_root = wired.root();

    assert!(
        mmu::translate_at(recorded_root, RECORDED_VA).is_some()
            && mmu::translate_at(wired_root, WIRED_VA).is_some(),
        "the shared frame was not mapped in both spaces before the revoke, so there was nothing \
         for the revoke to miss and this test proves nothing",
    );

    crate::revoke::revoke_page_frame(shared);

    assert!(
        mmu::translate_at(recorded_root, RECORDED_VA).is_none(),
        "the revoke did not reach even the recorded mapping, so this test's premise is false and \
         its result means nothing",
    );

    assert!(
        mmu::translate_at(wired_root, WIRED_VA).is_none(),
        "a page the kernel wired into an address space survived the revoke of its frame: \
         `map_physical` filed no record, so every unmap sweep in `crate::revoke` walked past a \
         live mapping of a frame whose capabilities are all gone",
    );

    // Teardown, innermost first: take the recorded space out of the registry so its `Drop` runs
    // (it forgets its root and frees its ASID; its backing is `Lent`, so the region survives),
    // then the wired space's own `Drop`, then the region and the frame.
    drop(take_user_address_space(recorded));
    drop(wired);
    crate::memory_region::destroy(region);
    crate::memory::free(page_frames::PageFrame::from_addr(shared));
}
