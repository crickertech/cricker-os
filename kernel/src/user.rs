//! Userspace. EL0. The actual operating system boundary.
//!
//! Everything before this was a Rust program that boots. From here on, the machine runs code
//! that **we did not compile and do not trust**, and the kernel's job stops being "do things"
//! and starts being "decide what is allowed."
//!
//! # Entering EL0 is returning from an exception that never happened
//!
//! There is no "drop to EL0" instruction. There is only `eret`, which restores whatever
//! `SPSR_EL1` says and jumps to `ELR_EL1`, and the exception level to return to is *in*
//! `SPSR_EL1`. So we do not need a new way down. We need a **fake way back**: fabricate a
//! [`TrapFrame`] with `SPSR = EL0t`, point `sp` at it, and fall into the `exception_restore`
//! that milestone 2 already wrote.
//!
//! This is the second time the project has pulled exactly this trick. `Thread::spawn` fakes a
//! `switch_to` frame so that the `ret` which *resumes* a thread also *starts* one
//! (notes/threads.md). Both times the "start" path turned out to be the "resume" path with a
//! forged frame, and no new code at all.
//!
//! # What milestone 4 already paid for
//!
//! The kernel lives entirely in `TTBR1`, at `0xffff_...`. Userspace lives in `TTBR0`, at
//! `0x0000_...`. **The hardware picks the table register from bits 63:48 of the address**, so:
//!
//! - The kernel is mapped in every address space, for free. Nobody had to copy anything.
//! - A syscall **does not switch page tables**. There is nothing to flush and nothing to remap.
//! - Installing a process is one `msr ttbr0_el1`.
//!
//! None of that was written for milestone 7. It fell out of a higher-half decision made three
//! milestones ago, and `Flags::user_code()` / `Flags::user_data()` have been sitting in the
//! `paging` crate, unused, waiting for today.
//!
//! # What is deliberately NOT here
//!
//! **A syscall ABI.** The user program below executes `svc #0` and asks for nothing. There is
//! no syscall number, no argument convention, no return value. DECISIONS §10 chose
//! capabilities, and the syscall surface gets designed against a capability table at 7d, in one
//! piece, on purpose. Not accreted here because it was convenient.

use elf::Elf;
use page_frames::{FRAME_SIZE, PageFrame};
use paging::{Flags, Half, MapError, Mapper};

use crate::arch::exceptions::{TrapFrame, enter_user};
use crate::arch::mmu::{self, phys_to_ptr};
use crate::arch::sync_icache;
use crate::memory;

/// Where a user program's stack goes. One page, and `sp` starts at the top of it: stacks grow down.
///
/// There is no matching `USER_CODE_VA` any more: it existed for `exec`, the one-page raw
/// machine-code loader the hand-assembled programs needed, and every program the kernel runs now
/// names its own load address in its ELF header.
pub const USER_STACK_VA: u64 = 0x0000_0000_0050_0000;
pub const USER_STACK_TOP: u64 = USER_STACK_VA + FRAME_SIZE;

/// A user address space: an L0 table for `TTBR0`, and every frame that hangs off it.
///
/// The `frames` vec holds **both** the pages we mapped and the intermediate page tables the
/// mapper allocated to reach them, because the allocator we hand the `Mapper` records
/// everything it hands out. That is the fix for the leak milestone 6 found the hard way
/// (`unmap_page` frees a leaf and leaves its L1/L2/L3 standing), applied *before* it bites:
/// an address space dies all at once, so we do not need `unmap` at all. We free the frames and
/// throw the whole table away.
pub struct AddressSpace {
    root: PageFrame,
    /// **This address space's TLB tag, for life** (milestone 15; `crates/address_space_identifier`). Every user
    /// mapping is `nG`, so its TLB entries carry this number, and a context switch flushes
    /// nothing: the other spaces' entries just stop matching. Freed at drop, after
    /// `flush_asid` has made every entry so tagged vanish, which is what makes the number
    /// reusable.
    asid: u16,
    /// **The untyped region every page of this address space comes from, and who frees it**
    /// (milestone 14 phase B.4): the root table, the intermediate tables, and every owned leaf
    /// are retyped out of one region. The region *is* the record of what this address space
    /// owns, which is why there is no frame list: teardown is `memory_region::destroy`, one call,
    /// made safe by §13 revocation.
    ///
    /// It carries the owner rather than just the name because **two different things build an
    /// address space and only one of them owns its memory**. See [`Backing`].
    backing: Backing,

    /// **The frame this space's thread reads its own CPU out of**, or `None` if it has none.
    ///
    /// calef ruled on 2026-09-21 that a thread observing *itself* is a per-thread page rather than
    /// a crossing, because the consumer is a memory allocator asking on every allocation. (That
    /// ruling's `design/decisions/` section is on another branch and not on `main` yet, so it is
    /// named here rather than cited.) `crates/current_cpu_protocol` holds the layout and the
    /// argument; this field is the kernel's end of it.
    ///
    /// **Per address space is per thread only because of §105 (`std::thread::spawn` stays
    /// declined)**: `Tcb::CONFIGURE` consumes the address-space capability, so no two TCBs name one
    /// space. The crate's `BUGS` section carries what has to change if that ever stops being true.
    ///
    /// Allocated from the global frame allocator rather than retyped from `backing`'s region, which
    /// is the one place this differs from every other page a space owns. A lent region is sized by
    /// whoever lent it, and spending one more page of it unconditionally is exactly what cost two
    /// regressions when the timebase page tried it in `user_address_space_create`; the comment
    /// recording that is still beside that function. So `Drop` frees this frame by hand, the one
    /// thing in this struct that `memory_region::destroy` does not cover.
    current_cpu_page: Option<PageFrame>,
}

/// **Who returns the region an [`AddressSpace`] spends, and the reason this is a type rather
/// than a comment.**
///
/// A space is built two ways, and they differ in exactly this. [`AddressSpace::new`] carves its
/// own region out of the frame allocator, so nobody else has a name for it and its `Drop` is the
/// only thing that can ever free it. [`user_address_space_create`] is handed a region that the caller
/// already holds a `MemoryRegion` capability to (the `RETYPE_OBJ(ADDRESS_SPACE)` engine, milestone 19b), and
/// that caller reclaims it with `MemoryRegion::DESTROY`. **A lent region has two names for one run of
/// memory, and only one of them may free it.**
///
/// Until 2026-08-18 both cases stored a bare `u64` and `Drop` called `memory_region::destroy`
/// unconditionally, on the theory that a lent region is still pinned (`retype_object_page` pins,
/// `sched::reclaim_region` unpins) so the borrower's `destroy` is refused. That reasoning holds
/// only while the pin is still set, and `reclaim_region` clears it **before** the reaper's
/// deferred drop can land: `sched::finish_switch` hoists a dead thread's space out from under
/// `IPC_TABLES`, releases the lock, and only then drops it. Two `memory_region::destroy` calls for one
/// region then overlap, both pass the refusal check, and both free every page of the run. That is
/// the intermittent `double free of frame 0x82a3e000` in
/// `force_kill_tests::destroy_reclaims_a_region_whose_resident_is_blocked_in_recv`
/// (notes/object-revocation.md BUGS, one sighting in 45 runs on riscv64).
///
/// Making it a two-variant enum with the name inside is rung one of AGENTS.md's ladder: a space
/// cannot be constructed without saying who frees its region, so the borrower's `Drop` cannot
/// free memory it does not own even if the pin is gone.
#[derive(Clone, Copy)]
enum Backing {
    /// The space carved this region itself and holds the only name for it. `Drop` frees it.
    Owned(u64),
    /// The region was handed in and belongs to whoever holds its `MemoryRegion` capability. `Drop`
    /// **must not** free it; the memory comes back at that owner's `sched::reclaim_region`.
    Lent(u64),
}

impl Backing {
    /// The region to retype from. Spending a lent region is correct and is the whole point of
    /// `RETYPE_OBJ(ADDRESS_SPACE)`: the space runs on the caller's budget. Only *freeing* is restricted.
    fn region(self) -> u64 {
        match self {
            Backing::Owned(region) | Backing::Lent(region) => region,
        }
    }
}

/// Page-table-and-slack overhead an address space needs beyond its content pages: the L0 root,
/// an L1 and L2, a handful of L3s (one per 2 MiB window touched, `Spawn` maps included), and
/// margin. Sixteen pages = 64 KiB, generous for every process this kernel builds.
const AS_OVERHEAD: u64 = 16;

/// **The window cost [`AS_OVERHEAD`]'s margin already carries**, which [`load`] subtracts rather
/// than charging a second time.
///
/// One log page ([`crate::revoke::log_pages_for`]), which is 170 recorded mappings, and no table
/// pages, because a window of that size touches at most one 2 MiB L3 that the "handful of L3s"
/// above is already for. Every caller in this tree but one maps a handful of pages and lands
/// inside it.
///
/// **It is subtracted because charging it again is measurable and was measured.** The first
/// version of this accounting charged the full window and the aarch64 suite's frame ledger went
/// from 22249 kept frames to 22317: one frame per long-lived process, permanently, reserved into a
/// region that never used it, to fix a window that one caller has. A budget that is right for the
/// exceptional caller and wasteful for all sixty-eight ordinary ones is the wrong shape; what a
/// caller owes is the cost **above** what the overhead was always providing.
const WINDOW_IN_OVERHEAD: u64 = 1;

impl AddressSpace {
    /// Carve this address space's budget: `content_pages` of expected leaves plus the
    /// page-table overhead. Everything the address space ever owns comes out of this region,
    /// and running out is a clean `OutOfPageFrames` at map time, spending nobody's memory but its
    /// own. The region's pages are retyped zeroed, so the root needs no separate scrub.
    pub fn new(content_pages: u64) -> Option<Self> {
        let region = crate::memory_region::create(content_pages + AS_OVERHEAD)?;
        let root = crate::memory_region::retype_page(region)?;

        // Share the kernel into this root. On RISC-V a process runs on a single `satp` that must map
        // both the process (low half) and the kernel (high half), so the root gets copies of the
        // kernel root's high-half entries. On aarch64 the kernel lives in a separate TTBR1 and this
        // is a no-op. See arch::mmu::share_kernel_half and DECISIONS §17.
        mmu::share_kernel_half(root);

        // Into the revocation registry (phase C): this is how a later revoke finds our mapping
        // log, whose pages this same region will pay for. Full registry = no address space.
        if !crate::revoke::register_space(root, region) {
            crate::memory_region::destroy(region);
            return None;
        }

        // A TLB tag of our own (milestone 15). Cannot exhaust: the allocator holds 255 and the
        // registry above admitted us, bounding live spaces at 160. The `?` is honesty, not a path.
        let Some(asid) = ASIDS.lock().alloc() else {
            crate::revoke::forget_root(root);
            crate::memory_region::destroy(region);
            return None;
        };

        let mut space = AddressSpace {
            root: PageFrame::from_addr(root),
            asid,
            backing: Backing::Owned(region),
            current_cpu_page: None,
        };

        // Unconditional, here rather than at the six places that build a process by hand, for the
        // reason `map_timebase_page` learned empirically: `load`'s own coverage misses every
        // kernel-built spawn, and each one found that out as a page fault. Every space this
        // function returns is a space a thread will run in, so this is the one place that covers
        // all of them and cannot be forgotten by a seventh.
        space.attach_current_cpu_page();

        Some(space)
    }

    /// **Give this space the page its thread reads its own CPU from**, if it has not got one.
    ///
    /// Idempotent, because two paths reach it: `new` for the spaces the kernel builds, and
    /// `sched::configure_thread_control_block` for the ones userspace builds and hands over. A
    /// space that goes through both gets one page.
    ///
    /// **Every failure is silent and leaves the space without a page**, which is deliberate and is
    /// the honest shape rather than the convenient one: a thread with no page reads an unmapped
    /// address, and a thread with a page reads the truth, but a `load` that failed outright because
    /// one frame was unavailable would turn a diagnostic convenience into a reason a program will
    /// not start. The states are told apart at the reader (`current_cpu_protocol::CurrentCpuPage`
    /// answers `None`), which is where somebody can act on it.
    pub fn attach_current_cpu_page(&mut self) {
        if self.current_cpu_page.is_some() {
            return;
        }
        let Some(frame) = crate::memory::alloc_zeroed() else {
            return;
        };

        // Zeroed above, then stamped: the magic makes a prepared page tell itself from a frame
        // nobody wrote, and the sentinel makes "this thread has never run" a state rather than
        // CPU 0. `alloc_zeroed` rather than `alloc` because the *rest* of this frame is mapped
        // into the process too, and whatever the last owner left in it would go with it.
        let bytes = current_cpu_protocol::build_page();
        // SAFETY: `frame` is freshly allocated and owned by nobody else yet, the direct map is
        // valid for it, and `PAGE_BYTES` (16) is far under `FRAME_SIZE`, so the copy stays inside
        // the frame.
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                mmu::phys_to_virt(frame.addr()) as *mut u8,
                bytes.len(),
            );
        };

        // Read-only to the process, which is what keeps this out of Tock's necessarily-unsafe
        // kernel category (notes/trusted-base.md): the kernel writes a frame it owns and lends the
        // process a view, rather than writing into memory the process supplied.
        if self
            .map_physical(
                current_cpu_protocol::PAGE_VA,
                frame.addr(),
                Flags::user_rodata(),
                crate::revoke::PageMapSource::NoCapability,
            )
            .is_err()
        {
            crate::memory::free(frame);
            return;
        }
        self.current_cpu_page = Some(frame);
    }

    /// **Publish the core this space's thread is about to run on.** Called from the context
    /// switch, on the core doing the switching, which is the core the thread will execute on.
    ///
    /// One branch and one relaxed store when the space has a page, one branch when it has not. The
    /// ordering argument is `current_cpu_protocol`'s and is not repeated here; its short form is
    /// that writer and reader are the same hardware thread, and that two successive writers are
    /// ordered by the scheduler's own release/acquire handoff rather than by anything this adds.
    #[inline]
    pub fn publish_current_cpu(&self, cpu: u64) {
        if let Some(frame) = self.current_cpu_page {
            // SAFETY: the frame is this space's own, allocated by `attach_current_cpu_page` and
            // freed only by `Drop`, so the direct-map view is live and 16 bytes wide here. The
            // caller is the one core switching this thread in, and a thread is on one core, so
            // this is the only writer for as long as the store takes.
            unsafe { current_cpu_protocol::publish(mmu::phys_to_virt(frame.addr()), cpu) };
        }
    }

    /// **The kernel's own view of this space's current-CPU page**, for the tests that read it from
    /// the side the thread cannot: the unset state is unobservable from inside a thread, because a
    /// thread that can ask has already been switched in. `None` if the space has no page.
    #[cfg(test)]
    pub fn current_cpu_page_kernel_va(&self) -> Option<u64> {
        self.current_cpu_page
            .map(|frame| mmu::phys_to_virt(frame.addr()))
    }

    /// Map one fresh, zeroed page at `va`, and hand back a **kernel** view of it.
    ///
    /// The returned slice is at `pa | KERNEL_VA_BASE` (the direct map), because the kernel
    /// cannot address `va` itself: `va` is a *low* address and means something entirely
    /// different from EL1's point of view. Two names for one frame, which is what the direct
    /// map is for.
    pub fn map_new(&mut self, va: u64, flags: Flags) -> Result<&'static mut [u8], MapError> {
        // Out of the address space's own region: the watermark is the ownership record, so
        // there is nothing to push anywhere. `retype_page` hands the page back zeroed, which is
        // what keeps `.bss` free for the loader.
        let frame = crate::memory_region::retype_page(self.backing.region())
            .ok_or(MapError::OutOfPageFrames)?;
        self.map_at(va, frame, flags)?;

        // SAFETY: the frame is ours (retyped from our region), and the direct map is valid for
        // it. 'static is a lie we tell for convenience and then keep: the frame outlives every
        // use of this slice, because the region is freed only at `Drop`.
        let page = unsafe {
            core::slice::from_raw_parts_mut(
                mmu::phys_to_virt(frame) as *mut u8,
                FRAME_SIZE as usize,
            )
        };
        Ok(page)
    }

    /// Map an **existing** physical page into this address space, at `va`, with `flags`.
    ///
    /// The frame is **not** recorded for freeing, because we do not own it: it is either a
    /// device's MMIO (the PL011, for a console server) or a page **shared** with another address
    /// space (a message buffer). Freeing MMIO is meaningless, and freeing a shared page when one
    /// of its two holders dies would hand live memory to the allocator. So `Drop` leaves it
    /// alone. The intermediate page tables reaching it *are* recorded, exactly as in `map_new`,
    /// because those genuinely belong to this address space.
    ///
    /// This one function is what lets a driver leave the kernel: it is how the UART's registers
    /// get into a userspace server's address space, and how a shared buffer gets into both a
    /// client's and a server's.
    ///
    /// **The mapping is recorded** (`under`), because a mapping revocation cannot see is the
    /// DECISIONS §13 (capability revocation and untyped reclamation) use-after-free, and until
    /// 2026-09-21 this function was the one mapping site in the kernel that recorded nothing. The
    /// paragraph above is about `Drop` and frame *ownership*, which is a different question that a
    /// reader can easily take this for: not freeing a frame and not being able to unmap it are
    /// unrelated, and the tree read the first as covering the second for as long as this function
    /// existed. An unrecordable mapping is unmapped and refused as `OutOfPageFrames`, exactly as at
    /// the `PageFrame::MAP` syscall, because the alternative is a mapping no sweep can reach.
    ///
    /// `under` says which capability's authority made this mapping, and it is a required argument
    /// with no default for [`crate::revoke::PageMapSource`]'s own reason (AGENTS.md's ladder, rung
    /// one). Every kernel-wiring caller passes `NoCapability`, truthfully: a [`Spawn`]`::maps`
    /// entry, a [`DeviceRun`], the initrd and the `x86_64` timebase page are all endowments the
    /// process holds no capability for, so the page stands as its own object and a single-page
    /// revoke of it finds the record. [`user_address_space_map`] is the one caller that passes a
    /// capability through, because the `MAP_INTO` syscall has one to pass.
    ///
    /// **What the defect was, since a reader will meet the fix without the failure.** Every unmap
    /// sweep in `crate::revoke` is driven by the mapping log, so a page wired here was invisible to
    /// `PageFrame::REVOKE`, `DeviceFrame::REVOKE` and `MemoryRegion::DESTROY` alike: the capability
    /// went and the mapping stayed. Recorded by risk 7's adversarial pass as latent; it was not.
    /// [`fs_service::spawn_fs_server`](crate::user::fs_service) wires the file channel's shared
    /// pages into the FS server through `Spawn::maps`, and [`boot_progenitor`] hands the progenitor
    /// `PageFrame(file_shared, 1)` with `GRANT` over the first of them, so `PageFrame::REVOKE` on
    /// that slot left the FS server writing to a page the progenitor had just un-shared.
    /// `user::spawn_mapping_revocation_tests` is the falsification.
    ///
    /// # BUGS
    ///
    /// **[`Self::map_new`] still does not record**, and is deliberately left alone: its frames are
    /// retyped from this space's own backing region and freed with it, so no capability names them
    /// and no sweep can be asked about them. If that ever stops being true, this is the second half
    /// of the same hole.
    pub fn map_physical(
        &mut self,
        va: u64,
        phys: u64,
        flags: Flags,
        under: crate::revoke::PageMapSource,
    ) -> Result<(), MapError> {
        self.map_at(va, phys, flags)?;
        let root = self.root.addr();
        if !crate::revoke::record_mapping(phys, root, va, under) {
            mmu::unmap_user_at(root, va);
            return Err(MapError::OutOfPageFrames);
        }
        Ok(())
    }

    /// Map `phys` at `va`. Intermediate tables come from this address space's own region, so
    /// they are covered by the one teardown call; the target page is whoever's it was.
    fn map_at(&mut self, va: u64, phys: u64, flags: Flags) -> Result<(), MapError> {
        let root = self.root.addr();
        let region = self.backing.region();

        // SAFETY: `root` is a zeroed L0 table. Half::Low, so the mapper refuses a high address:
        // mapping the kernel's half into TTBR0 would build a translation the hardware never
        // consults, and we would chase the ghost for hours.
        let mut mapper = unsafe {
            Mapper::<_, _, crate::arch::mmu::Format>::new(
                root,
                Half::Low,
                || crate::memory_region::retype_page(region),
                phys_to_ptr,
            )
        };

        mapper.map(va, phys, flags)
    }

    /// The physical address of the L0 table: what page-table walks (translate, unmap,
    /// revocation) use. Not what goes in `TTBR0_EL1` any more; that is [`ttbr0`](Self::ttbr0),
    /// which carries the ASID too.
    #[cfg_attr(not(test), allow(dead_code))] // the walkers that use it live in the tests
    pub fn root(&self) -> u64 {
        self.root.addr()
    }

    /// The composed `TTBR0_EL1` value: root plus this space's ASID, ready to install.
    pub fn ttbr0(&self) -> u64 {
        mmu::ttbr0_value(self.root.addr(), self.asid)
    }
}

/// The machine's ASID allocator (milestone 15; the crate carries the proofs). Taken alone, at
/// address-space creation and teardown, holding nothing else that matters; a leaf-adjacent rank.
static ASIDS: crate::sync::IrqSafeMutex<address_space_identifier::Allocator> =
    crate::sync::IrqSafeMutex::new(
        crate::sync::rank::ASIDS,
        address_space_identifier::Allocator::new(),
    );

/// The most user-built address spaces alive at once (milestone 19b). They are immortal until
/// 19c wires process death, so this bounds creations for now; the revocation registry's
/// `MAX_SPACES` (160) leaves room for all of them beside the exec-built spaces.
const MAX_USER_SPACES: usize = 32;

/// **The user address-space registry** (milestone 19b): the kernel-side records behind
/// `Object::AddressSpace` capabilities, named generationally like everything since milestone 14. The
/// `AddressSpace` in the slot is the same type exec builds, so every mechanism that works on a
/// process's space (region-paid tables, revocation logs, ASID tagging) works on a user-built
/// one identically. Entries are never removed in 19b; their `Drop` (which would destroy a
/// region the creator still holds a capability to) stays dormant until 19c designs teardown.
static USER_SPACES: crate::sync::IrqSafeMutex<
    generational_table::Table<AddressSpace, MAX_USER_SPACES>,
> = crate::sync::IrqSafeMutex::new(
    crate::sync::rank::ADDRESS_SPACES,
    generational_table::Table::new(),
);

/// Create an address space **in and backed by** `region` (the `RETYPE_OBJ(ADDRESS_SPACE)` engine): the
/// root page is retyped from it (pinning it, atomically with the carve), and the region becomes
/// the space's table-and-record budget, exactly as for an exec-built space. `None` on an
/// exhausted region, a full registry, or ASID exhaustion (unreachable; the type is honest).
pub fn user_address_space_create(region: u64) -> Option<u64> {
    let root = crate::memory_region::retype_object_page(region)?;
    mmu::share_kernel_half(root); // RISC-V single-satp: the process root carries the kernel high half

    if !crate::revoke::register_space(root, region) {
        return None; // registry full; the carved page is spent, the caller's own loss (B.4 rule)
    }
    let Some(asid) = ASIDS.lock().alloc() else {
        crate::revoke::forget_root(root);
        return None;
    };

    let space = AddressSpace {
        root: PageFrame::from_addr(root),
        asid,
        // Lent, not owned: the caller holds the `MemoryRegion` capability to this region and reclaims
        // it with `DESTROY`. See `Backing` for the double free that taught us to say so.
        backing: Backing::Lent(region),
        // **Not attached here**, for the same reason the timebase page is not mapped here (the
        // comment below): this syscall serves every purpose that wants a bare address-space
        // object, most of which never run a thread and some of which are sized to the page. The
        // space gets its page when a TCB binds it, in `sched::configure_thread_control_block`,
        // which is the moment it becomes a thread's space and therefore the moment the question
        // "which CPU am I on" starts having an answer.
        current_cpu_page: None,
    };

    // The timebase page is **not** mapped unconditionally here (an earlier version of this
    // lane's work did, and a full-suite run under `script/test --arch x86_64` caught two
    // regressions: `a_process_can_build_an_address_space_from_el0`'s hand-sized demo region ran
    // out of table budget, and it makes no sense for the many callers of this syscall that build
    // nothing resembling a real ELF process at all). This syscall is shared by every purpose that
    // needs a bare address space object, not only the userspace ELF loader
    // (`supervision_protocol::build_child_space`), and the loader is where this page actually
    // belongs: see that crate's own code for the targeted fix, which writes the page from the rate
    // the *parent* already holds, so a child reads its parent's measured number and a parent that
    // knows nothing hands down nothing rather than a plausible constant.
    let name = USER_SPACES.lock().insert_with(|_| space);
    if name.is_none() {
        // Undo the bookkeeping; the page stays spent on the caller's budget.
        crate::revoke::forget_root(root);
        ASIDS.lock().free(asid);
    }
    name
}

/// Map `phys` into the user-built space `name` at `va` (the `MAP_INTO` engine). Tables and the
/// §13 record come from the space's own backing region; an unrecordable mapping is unmapped and
/// refused, exactly as at the `page_frame::MAP` syscall, because a mapping revocation cannot see is
/// the §13 use-after-free.
///
/// `under` says which capability's authority this mapping was made with, which is what scopes a
/// later `PageFrame::REVOKE` to that capability's derivation family rather than to the physical
/// page (DECISIONS §132). The `MAP_INTO` syscall passes the invoked frame capability's object; the
/// kernel's own callers, which build a space directly out of a region, pass
/// `PageMapSource::NoCapability`.
pub fn user_address_space_map(
    name: u64,
    va: u64,
    phys: u64,
    flags: Flags,
    under: crate::revoke::PageMapSource,
) -> Result<(), MapError> {
    let mut spaces = USER_SPACES.lock();
    let space = spaces.get_mut(name).ok_or(MapError::NotMapped)?;

    // `map_physical` maps and records in one step since 2026-09-21, including the unmap-and-refuse
    // on an unrecordable mapping that used to live here: this function was the one caller that
    // remembered to record, which is exactly why it is now the one caller with nothing extra to
    // remember. See that function's own docs for the defect the other callers carried.
    space.map_physical(va, phys, flags, under)?;
    // A code page a loader just filled via data writes (milestone 19d): the instruction fetcher
    // has its own cache and has never heard of those bytes. On aarch64 the I-cache is not
    // coherent with the D-cache, so make it so now, via the frame's direct-map VA (any VA that
    // maps the physical page works; caches are PIPT to the point of unification). Without this,
    // the child fetches whatever was in the frame before the loader wrote its program.
    if flags.is_user_executable() {
        sync_icache(mmu::phys_to_virt(phys), FRAME_SIZE as usize);
    }
    Ok(())
}

/// The root table of a user-built space, named generationally like the registry it reads.
///
/// Built for tests (so a walker can ask what a space really maps) and now also
/// `abi::address_space::LIST`'s way in (milestone 126's `pmap`, DECISIONS §114): the syscall handler
/// resolves the capability's `name` to a root here before consulting `revoke::list_mapping` and
/// `arch::mmu::translate_at`. `None` once the name is gone from the registry, which is
/// `ThreadControlBlock::CONFIGURE`'s doing the moment a space is bound to a thread (`take_user_address_space` removes
/// the entry): a `LIST` against a capability that outlived its space's registry membership reads
/// as "nothing to report," the same as an empty space, because the capability itself was never
/// refused and the kernel has nothing left to say about where it used to point.
pub fn user_address_space_root(name: u64) -> Option<u64> {
    USER_SPACES.lock().get(name).map(|s| s.root())
}

/// **Take a user-built address space out of the registry** (milestone 19c.3): `ThreadControlBlock::CONFIGURE`
/// moves it into the TCB, so it stops being a standalone object and starts dying with the
/// thread. `None` if the name does not resolve. This is what retires 19b's "immortal until 19c"
/// note: a bound space is reaped, an unbound one still leaks until teardown wiring, which is the
/// half-built audit's job.
pub fn take_user_address_space(name: u64) -> Option<AddressSpace> {
    USER_SPACES.lock().remove(name)
}

/// **Tear down every user address space whose root page lies in `[base, end)`** (object revocation,
/// the address-space case): each removed `AddressSpace` drops here, and its `Drop` forgets its
/// revocation records and frees its ASID (its region's memory comes back at the enclosing
/// `reclaim_region`, which unpins after this). This retires the "an unbound one still leaks" note on
/// `take_user_address_space`: a space created but never bound into a TCB is reclaimed with its region.
///
/// Bound spaces are **not** here: `CONFIGURE` moved them out of this registry into a TCB, so they
/// die with the thread (`Thread`'s drop), not through this sweep. Takes only the address-space registry
/// lock, no `IPC_TABLES`, so `sched::reclaim_region` runs it as a step separate from the thread reap.
pub fn reap_address_spaces_in_region(base: u64, end: u64) {
    // Find-then-remove one at a time, never dropping an `AddressSpace` while holding the registry
    // lock: its `Drop` takes the revocation, region, and ASID locks, and must not do so under ours.
    loop {
        let victim = {
            let spaces = USER_SPACES.lock();
            spaces.iter().find_map(|(name, space)| {
                let root = space.root.addr();
                (base <= root && root < end).then_some(name)
            })
        };
        let Some(name) = victim else { break };
        // `remove` returns the space; the registry lock is released at the `;`, then the space drops.
        let space = USER_SPACES.lock().remove(name);
        drop(space);
    }
}

/// Put a space back into the registry (milestone 19c.3): the unwind path if `CONFIGURE` took a
/// space and then could not bind it. It gets a fresh name; the caller's stale address space cap will no
/// longer resolve, which is correct (the operation failed, but the space is not lost).
pub fn readopt_user_address_space(space: AddressSpace) -> Option<u64> {
    USER_SPACES.lock().insert_with(|_| space)
}

impl Drop for AddressSpace {
    fn drop(&mut self) {
        // Drop this address space's entries from the revocation database (§13) before its page
        // tables are freed and reused: a stale (root, va) would send a later revoke to walk tables
        // that now belong to someone else.
        crate::revoke::forget_root(self.root.addr());

        // If we are the live address space, stop being it BEFORE the frames go back on the free
        // list. Otherwise the TTBR0 the CPU is walking points at memory the allocator has
        // already handed to somebody else, and the next low-half access reads whatever they put
        // there. This is about the *walker*, not the TLB: since milestone 58 neither ISA flushes
        // anything on a root switch, and the cached translations are dealt with by the `flush_asid`
        // at the bottom of this function, which is the half that has to reach the other cores.
        if mmu::current_user_root() == self.root.addr() {
            mmu::deactivate_user();
        }

        // One call: revoke anything delegated out of this region (nothing can be: the region
        // has no capability, so userspace could never retype from it), then return the whole
        // run, root and tables and leaves alike, to the allocator. This is the
        // "reclaim-on-process-death" wiring §13 deferred; the frame list it replaced is gone.
        //
        // **Only for a region we own.** A lent one (`user_address_space_create`) belongs to whoever
        // holds its `MemoryRegion` capability, and freeing it here is a double free of the whole run
        // the moment `sched::reclaim_region` has already unpinned it. `Backing` carries the
        // whole argument.
        if let Backing::Owned(region) = self.backing {
            crate::memory_region::destroy(region);
        }

        // The current-CPU page is the one frame this space owns that did NOT come out of its
        // region, so `destroy` above does not cover it and ownership has to do the work by hand.
        // Safe to do here, after the root is no longer live: nothing can read the mapping any
        // more, and the only writer was the context switch of a thread that is gone.
        if let Some(frame) = self.current_cpu_page.take() {
            crate::memory::free(frame);
        }

        // The ASID contract (crates/address_space_identifier): invalidate every TLB entry wearing our tag, THEN
        // hand the number back. In the other order, the next owner of this ASID could hit our
        // stale translations, which is exactly the bug tagging exists to prevent.
        mmu::flush_asid(self.asid);
        ASIDS.lock().free(self.asid);
    }
}

/// Why a binary was refused.
///
/// **A bad user program must not be a kernel panic.** Every one of these is a thing a file can
/// simply *say*, and the answer is to decline and kill the thread, not to take the machine down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    /// The file is not an aarch64 static ELF we are willing to run. See `elf::Error`.
    NotLoadable(elf::Error),

    /// It asked to be loaded somewhere it may not go.
    ///
    /// **Including a KERNEL address.** An ELF gets to name its own load address, so this is
    /// exactly the thing a hostile binary tries: ask to be mapped over the kernel. It is
    /// refused by construction rather than by a check, because the `Mapper` is built with
    /// `Half::Low` and a high address is not a thing it can express (`MapError::WrongHalf`).
    Unmappable(MapError),
}

/// Parse an ELF, build an address space, and put it in memory. Do **not** run it.
///
/// Split out from [`run`] on purpose: this is the part that can fail, so it is the part a test can
/// call without dying (`run` diverges into the new process, or into `exit`).
///
/// **`windowed` is how many pages the caller is about to map into the new space itself**, and a
/// caller that maps nothing passes zero.
///
/// **A `Spawn::maps` page is not free**, and since 2026-09-21 it is less free than it was: it costs
/// a share of an intermediate table and, now that `map_physical` records, a share of a log page,
/// both out of this space's own region. `AS_OVERHEAD`'s sixteen pages absorb that for the handful
/// of pages most callers map and do not absorb it for the two that map thousands: the progenitor's
/// archive window, which pays for itself at its own call site, and the installer's copy of the boot
/// file, which did not and could not, because it goes through [`run`] and [`run`] had nowhere to
/// put the number.
///
/// So the accounting is done **here**, from `spawn.maps` itself, rather than asked of every caller.
/// That is AGENTS.md's ladder read downward: a budget a caller must remember to widen is a budget
/// that is wrong the first time somebody maps a bigger window, and the failure it produces is an
/// `OutOfPageFrames` panic in a spawn three frames away from anything that mentions memory.
///
/// What is charged is the cost **above** [`WINDOW_IN_OVERHEAD`], for the reason recorded there: a
/// handful of mapped pages has always been paid for out of `AS_OVERHEAD`'s margin, and charging it
/// twice costs a frame per process forever.
pub fn load(image: &[u8], windowed: u64) -> Result<(AddressSpace, u64), LoadError> {
    let elf = Elf::parse(image).map_err(LoadError::NotLoadable)?;

    // The budget, counted from the file before anything is carved: every segment's pages, plus
    // one for the stack, plus what the caller's own windows will cost. (AS_OVERHEAD covers the
    // tables for everything else.) A binary that lies about its size simply exhausts its own
    // region and fails to map, spending nobody else's memory.
    let content: u64 = elf
        .segments()
        .map(|seg| {
            let (start, end) = seg.page_range(FRAME_SIZE);
            (end - start) / FRAME_SIZE
        })
        .sum::<u64>()
        + 1
        + (windowed / 512 + crate::revoke::log_pages_for(windowed))
            .saturating_sub(WINDOW_IN_OVERHEAD);

    let mut space =
        AddressSpace::new(content).ok_or(LoadError::Unmappable(MapError::OutOfPageFrames))?;

    map_segments(&mut space, &elf)?;

    space
        .map_new(USER_STACK_VA, Flags::user_data())
        .map_err(LoadError::Unmappable)?;

    // The timebase page, from milestone 161 (the kernel port) and its `cntfrq` follow-up,
    // widened to riscv64 on 2026-09-21: the
    // one number `user_mode_runtime::now()` needs on an architecture with no `CNTFRQ_EL0` to read it
    // from. `x86_64` measures it; `riscv64` reads it out of the device tree, which is privileged
    // knowledge a process has no way to reach. `map_physical` does not
    // spend `content`'s budget (only the intermediate table pages it walks come from the space's
    // own region, the same as every `Spawn::maps` entry `run()` applies below), so this needs no
    // extra accounting here. Unconditional, the same "grant is unconditional, a zeroed page reads
    // as unknown" shape `boot_clock_page` already uses: see `timebase_page_phys`'s own docs.
    #[cfg(any(target_arch = "x86_64", target_arch = "riscv64"))]
    if let Some(phys) = timebase_page_phys() {
        space
            .map_physical(
                counter_frequency_protocol::PAGE_VA,
                phys,
                Flags::user_rodata(),
                crate::revoke::PageMapSource::NoCapability,
            )
            .map_err(LoadError::Unmappable)?;
    }

    Ok((space, elf.entry()))
}

/// **The timebase page's one physical frame**, computed and written once, then reused for
/// every process `load` maps it into. The frequency the kernel measured (`x86_64`) or read from the
/// device tree (`riscv64`)
/// does not change while the machine runs, so one frame mapped read-only into every address space
/// is correct rather than merely convenient: there is only ever one true answer to publish.
///
/// **aarch64 has no such frame and needs none**: `CNTFRQ_EL0` is architected, the kernel opens it to
/// EL0, and a register the machine itself states cannot go stale between the kernel reading it and
/// a process reading it.
///
/// `None` only if the frame allocator is out of memory (propagated by `load` as the same
/// `OutOfFrames` a segment that would not fit reports; this is not a bad-program condition, so it
/// is not a panic). If [`crate::arch::timer::frequency_checked`] has not resolved yet (never
/// observed: `init_frequency` runs early in both boot tours, well before the first call to
/// `load`), the frame is allocated anyway and left zeroed, which [`counter_frequency_protocol::TimebasePage::hz`]
/// reads as "unknown" rather than a fabricated rate; that keeps every process's layout
/// identical regardless of boot order, the same reason `boot_clock_page` hands out a zeroed page
/// when there is no `clock` program to ask.
#[cfg(any(target_arch = "x86_64", target_arch = "riscv64"))]
fn timebase_page_phys() -> Option<u64> {
    use core::sync::atomic::{AtomicU64, Ordering};
    static PAGE_PHYS: AtomicU64 = AtomicU64::new(0);

    let cached = PAGE_PHYS.load(Ordering::Acquire);
    if cached != 0 {
        return Some(cached);
    }

    // `alloc_zeroed`, not `alloc`: only the first 16 bytes of this frame are written below, and
    // the WHOLE frame is then mapped read-only into every process on this architecture. An
    // unzeroed frame would carry whatever its last owner left in it across that boundary. Found
    // 2026-09-21 by the lane that built the current-CPU page on this function's shape.
    let phys = crate::memory::alloc_zeroed()?.addr();
    // `phys_to_virt` is a plain address computation (a `const fn`, no memory access), so nothing
    // below this line needs a safety comment for naming `dst`; the comments that follow cover the
    // two places `dst` is actually written through.
    let dst = mmu::phys_to_virt(phys) as *mut u8;
    match crate::arch::timer::frequency_checked() {
        Some(hz) => {
            let bytes = counter_frequency_protocol::build_page(hz);
            // SAFETY: `dst` names a freshly allocated frame, reachable through the direct map and
            // owned by nobody else yet; `bytes` is `PAGE_BYTES` (16) bytes, far under the frame's
            // `FRAME_SIZE`, so the copy does not run past it.
            unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len()) };
        }
        // Not yet measured: leave the frame zeroed (retype_page-equivalent frames from
        // `memory::alloc` are not guaranteed pre-zeroed the way `Untyped::retype_page` promises,
        // so this writes zero explicitly rather than assuming it).
        //
        // SAFETY: as the `Some` arm above: `dst` names a freshly allocated, exclusively owned
        // frame, and `PAGE_BYTES` is far under `FRAME_SIZE`.
        None => unsafe { core::ptr::write_bytes(dst, 0, counter_frequency_protocol::PAGE_BYTES) },
    }

    PAGE_PHYS.store(phys, Ordering::Release);
    Some(phys)
}

/// Map the timebase page into `space`, if this process needs one built directly rather
/// than through [`load`]. Several kernel-side functions build a top-level process's own
/// `AddressSpace` by hand instead of calling `load` (`spawn_hello`, and every
/// `spawn_<program>`-shaped test harness that hands a narrowed archive to a named program:
/// `timetable_tests::spawn_timetable`, `authority_tests`' `root_supervisor` spawn,
/// `c_seam_tests::spawn_confiner`, `login_service`, `live_swap_tests`' `swapper` spawn), because
/// each wants a narrower or differently-shaped world than a generic `load` call builds. `load`'s
/// own unconditional mapping never reaches any of them, and each one found this the same way:
/// **empirically**, as an unmapped-read page fault the first time something in that process
/// called `user_mode_runtime::cntfrq` (`timetable`'s own scheduling logic was the one that actually found
/// this; `load`'s coverage alone left every one of these kernel-built processes unmapped and it
/// took a real `script/test --arch x86_64` run, not a reading of the call graph, to find them
/// all). Factored out once here rather than copied into each, per CLAUDE.md rule 7's reasoning
/// one level down: this is kernel-internal, not shared with a second *binary*, but the six call
/// sites are exactly the "same lines three times" shape that rule exists to prevent.
///
/// **`riscv64` joined this path on 2026-09-21** and inherited every one of those call sites at
/// once, rather than rediscovering them one page fault at a time, which is what the factoring above
/// bought: there was one function to widen and one `cfg` per site to change.
#[cfg(any(target_arch = "x86_64", target_arch = "riscv64"))]
fn map_timebase_page(space: &mut AddressSpace) -> Result<(), MapError> {
    if let Some(phys) = timebase_page_phys() {
        space.map_physical(
            counter_frequency_protocol::PAGE_VA,
            phys,
            Flags::user_rodata(),
            crate::revoke::PageMapSource::NoCapability,
        )?;
    }
    Ok(())
}

/// Lay an ELF's loadable segments into `space`, honouring their permissions exactly (milestone
/// 19d factored this out of `load` so `spawn_hello` shares it; the progenitor's userspace
/// loader mirrors it).
/// A read-only segment gets `user_rodata`, not `user_data`: a loader that widens permissions is
/// a loader you cannot reason about. `.bss` is free because `map_new` zeroes every page.
fn map_segments(space: &mut AddressSpace, elf: &Elf) -> Result<(), LoadError> {
    for seg in elf.segments() {
        let flags = if seg.is_executable() {
            Flags::user_code()
        } else if seg.is_writable() {
            Flags::user_data()
        } else {
            Flags::user_rodata()
        };

        let (start, end) = seg.page_range(FRAME_SIZE);
        let mut va = start;
        while va < end {
            let page = space.map_new(va, flags).map_err(LoadError::Unmappable)?;

            // Which of the file's bytes land in this page? An intersection, because `p_vaddr`
            // need not be page-aligned.
            let file_lo = seg.vaddr;
            let file_hi = seg.vaddr + seg.data.len() as u64;
            let lo = va.max(file_lo);
            let hi = (va + FRAME_SIZE).min(file_hi);
            if lo < hi {
                let dst = (lo - va) as usize;
                let src = (lo - file_lo) as usize;
                let n = (hi - lo) as usize;
                page[dst..dst + n].copy_from_slice(&seg.data[src..src + n]);
            }

            if seg.is_executable() {
                sync_icache(page.as_ptr() as u64, FRAME_SIZE as usize);
            }
            va += FRAME_SIZE;
        }
    }
    Ok(())
}

/// The program QEMU loaded into RAM for us, found via the device tree.
///
/// **The same road Linux's initramfs travels.** Nothing about this binary is known to the kernel
/// at build time: QEMU put a file somewhere in RAM and wrote the address into
/// `/chosen/linux,initrd-start`, and `memory::init` read it there and told the frame allocator
/// to keep its hands off. That reservation was written at milestone 3, for this.
#[cfg_attr(feature = "bench", allow(dead_code))] // the bench boot runs no user programs
pub fn initrd() -> Option<&'static [u8]> {
    let (start, size) = memory::initrd_region()?;

    // SAFETY: the region came from the device tree, it is inside RAM, the frame allocator has
    // been told it is forbidden, and the direct map names it. Nothing else will ever write here.
    Some(unsafe {
        core::slice::from_raw_parts(mmu::phys_to_virt(start) as *const u8, size as usize)
    })
}

/// The bytes of the program named `name` inside the initrd archive (milestone 19f). The initrd is a
/// nifefs image carrying the progenitor plus the programs it loads. The milestone tour and the
/// kernel-side service demos ask for whichever program they wire, by name; since milestone 291
/// that is one program per demo rather than one role of [`HELLO_ENTRY`].
/// `spawn_hello` and `boot_progenitor` instead take the whole archive, because the
/// progenitor parses the rest itself. Returns `None` if there is no initrd, it will not parse, or
/// it holds no such program.
// Used by the milestone tour, the kernel-wired virtio/console/shell demos, and the tests that load
// a user program; dead only in the bench boot, which runs no user programs.
#[cfg_attr(feature = "bench", allow(dead_code))]
pub fn program(name: &str) -> Option<&'static [u8]> {
    nifefs::Fs::parse(initrd()?).ok()?.read(name)
}

/// A physical page to map into a new process's address space, at a chosen VA.
///
/// The frame is **not** owned by the process (it is shared, or it is device MMIO), so it is not
/// freed when the process dies. See [`AddressSpace::map_physical`].
#[derive(Clone, Copy)]
pub struct Mapping {
    pub va: u64,
    pub phys: u64,
    pub flags: Flags,
}

/// **Everything a new process is handed at birth.** Its world, made explicit.
///
/// A capability system has no ambient environment: no inherited file descriptors, no `PATH`, no
/// uid. So a process gets *exactly* what is in this struct and nothing else. The whole of what it
/// can do is a function of `arg0`, `grants`, and `maps`, and reading a `Spawn` literal tells you
/// the complete authority of the thing you are about to start.
pub struct Spawn<'a> {
    /// Lands in `x0` at `_start`. A tiny channel for "which role are you" that needs no
    /// capability, the way a real kernel hands a new process its argc.
    pub arg0: u64,
    /// Lands in `x1`. A second scalar the process needs before it can name anything: the virtio
    /// driver's DMA region physical address, which it must write into device descriptors and
    /// cannot discover, because a process only knows virtual addresses.
    pub arg1: u64,
    /// Lands in `x2`. The virtio driver's device registers sit at a sub-page offset (slots are
    /// 0x200 apart, pages are 0x1000), so we map the containing page and tell the driver where in
    /// it the slot begins.
    pub arg2: u64,
    /// Capabilities, granted into slots 0, 1, 2, ... in order.
    pub grants: &'a [crate::cap::Cap],
    /// Extra pages: a shared buffer, a device's registers. Mapped after the ELF's own segments.
    pub maps: &'a [Mapping],
}

/// Where the kernel maps the initrd read-only into the progenitor's address space (milestone 19d): the progenitor
/// reads the ELF to parse it here. High enough not to collide with the progenitor's own segments (`0x40_0000`)
/// or its stack (`0x50_0000`).
#[cfg_attr(not(test), allow(dead_code))] // becomes the boot path at 19d.2; test-driven until then
pub const INITRD_VA: u64 = 0x2000_0000;

/// **Spawn the progenitor task** (milestone 19d): load `image` as an ordinary user process, but also
/// map the whole initrd read-only at [`INITRD_VA`] so the progenitor can parse it, and hand the progenitor a building
/// budget (an untyped, slot 0) plus `report` (slot 1, `WRITE|GRANT` so the progenitor can endow a child).
/// The progenitor enters with `x0` = `role` and `x1` = the initrd length. This is the one program the kernel
/// still loads; the progenitor loads the rest (design/init-and-granular-spawn.md).
/// The interrupt the kernel routes to the progenitor for the IRQ-delegation test (19d.2b).
///
/// aarch64: SGI 3, distinct from the scheduler's RESCHED (0) and the older endpoint SGIs (1, 2).
/// RISC-V has no software-generated interrupt a test can raise on itself at all (the SBI IPI
/// arrives down the *software*-interrupt arm, never touching `irq_route`), so it names the console
/// UART's own line, which is the one interrupt this ISA can assert by hand. That makes it the same
/// number as [`UART_RX_INTID`] there, deliberately; [`spawn_hello`] binds the route once and grants
/// two capabilities naming it. See `sched::tests`' `DELIVERY_IRQ`, which reached the same conclusion.
#[cfg_attr(not(test), allow(dead_code))]
#[cfg(target_arch = "aarch64")]
pub const INIT_TEST_SGI: u32 = 3;
#[cfg_attr(not(test), allow(dead_code))]
#[cfg(target_arch = "riscv64")]
pub const INIT_TEST_SGI: u32 = 10;
/// `x86_64` (milestone 161, updated by roadmap item 4): **the local APIC's self-IPI test vector**,
/// which puts this ISA on aarch64's side of the split rather than RISC-V's. The local APIC will
/// deliver a vector to its own CPU on demand through the ICR, so x86 needs no device to raise an
/// interrupt by hand, and `arch::x86_64::irq::raise_self_interrupt` is the mechanism. The intid for
/// such a source **is its vector**, which is why this is 0x22 and not a small number like the other
/// two: see `arch::x86_64::exceptions::x86_trap_body`'s self-IPI arm for the naming rule.
///
/// It was 4 (COM1's legacy IRQ) while the APIC was unbuilt and that arm's own comment said to
/// revisit this when it landed.
#[cfg_attr(not(test), allow(dead_code))]
#[cfg(target_arch = "x86_64")]
pub const INIT_TEST_SGI: u32 = crate::arch::irq::SELF_TEST_VECTOR as u32;

/// The console UART's receive interrupt on QEMU `virt`. The progenitor routes and delegates it so the input
/// driver it builds (19d.2c) can wait on keystrokes. aarch64's PL011 is SPI 1 = INTID 33; RISC-V's
/// NS16550 is PLIC source 10.
///
/// **The documented fallback, not the answer.** The boot paths ask the machine first
/// ([`uart_irq_and_source`]): on the JH7110, UART0 interrupts on PLIC line 32, and a kernel that
/// armed this constant there enabled an unrelated source, proven on silicon when a key press at
/// boot 13's completed tour reached nothing (notes/visionfive2.md, BUGS). This number is what a
/// tree that does not say falls back to, which on QEMU is also the right answer.
///
/// Name: provisional, flagged 2026-09-25 by the lane that re-derived the x86 port falsifications
/// (design/naming/boolean-predicates-worklist.md, "`rx` and `tx`"). calef asked what `rx` stands
/// for in his #1255 review; recommended `UART_RECEIVE_INTID`; `INTID` is the GIC's own term and
/// stays.
#[cfg(target_arch = "aarch64")]
pub const UART_RX_INTID: u32 = 33;
/// Name: provisional, flagged 2026-09-25 by the lane that re-derived the x86 port falsifications
/// (design/naming/boolean-predicates-worklist.md, "`rx` and `tx`"). calef asked what `rx` stands
/// for in his #1255 review; recommended `UART_RECEIVE_INTID`; `INTID` is the GIC's own term and
/// stays.
#[cfg(target_arch = "riscv64")]
pub const UART_RX_INTID: u32 = 10;
/// `x86_64`: COM1 is ISA IRQ 4, which has been true since the PC/AT and is what QEMU's `q35`
/// presents. **What that number means depends on the interrupt controller**, and on x86 that is
/// two questions rather than one: which IO APIC input the legacy IRQ was remapped to (the ACPI
/// MADT's interrupt source overrides say, and this port does not read them), and which IDT vector
/// that input is programmed to raise. 4 is the legacy line, not either of those.
///
/// Name: provisional, flagged 2026-09-25 by the lane that re-derived the x86 port falsifications
/// (design/naming/boolean-predicates-worklist.md, "`rx` and `tx`"). calef asked what `rx` stands
/// for in his #1255 review; recommended `UART_RECEIVE_INTID`; `INTID` is the GIC's own term and
/// stays.
#[cfg(target_arch = "x86_64")]
pub const UART_RX_INTID: u32 = 4;

/// The console UART's interrupt line and which source decided it: the machine's own answer when
/// it gave one (`memory::uart_irq`, a device tree on aarch64/riscv64 or ACPI on `x86_64`), else
/// [`UART_RX_INTID`], QEMU `virt`'s constant. The source string exists to be printed: a bench
/// transcript that names the number's origin is diagnosable, and the one that did not already
/// cost a boot (notes/visionfive2.md).
pub fn uart_irq_and_source() -> (u32, &'static str) {
    match crate::memory::uart_irq() {
        Some(n) => (n, "machine description"),
        None => (UART_RX_INTID, "QEMU-virt fallback; the machine did not say"),
    }
}

/// The console UART's registers, physically. aarch64 `virt` puts a PL011 at `0x0900_0000`; RISC-V
/// `virt` puts an NS16550 at `0x1000_0000`. The progenitor holds a device capability for it and delegates it
/// to the console and input drivers it builds. Matches `console::UART_PHYS`.
#[cfg_attr(not(test), allow(dead_code))]
#[cfg(target_arch = "aarch64")]
pub const UART_PHYS: u64 = 0x0900_0000;
#[cfg_attr(not(test), allow(dead_code))]
#[cfg(target_arch = "riscv64")]
pub const UART_PHYS: u64 = 0x1000_0000;
/// `x86_64` has **no physical address for its console at all**: COM1 lives in the I/O port space,
/// which has no page tables in front of it, so there is nothing here for a device capability to be
/// a mapping *of*. The console is reached through [`X86_COM1_PORT_BASE`] as a `PortRange` capability
/// instead (milestone 299, DECISIONS §121 reversed 2026-09-15); this constant stays zero because a
/// port has no physical page, and the predicate below still reads it to keep a fixture from ever
/// granting a device page where there is none. See `arch/x86_64/port.rs` and `segments.rs`.
#[cfg_attr(not(test), allow(dead_code))]
#[cfg(target_arch = "x86_64")]
pub const UART_PHYS: u64 = 0;

/// **COM1's I/O ports** (milestone 299): the eight consecutive ports `0x3F8..=0x3FF` a 16550 UART
/// occupies, which QEMU's `q35` and every PC since the PC/AT put COM1 at. The progenitor is minted a
/// `PortRange(0x3F8, 8)` capability over exactly this range and delegates it to the console and input
/// drivers, so their `in`/`out` reach these ports and no others (enforced by the TSS I/O bitmap). The
/// port analogue of [`UART_PHYS`] on the other two architectures.
#[cfg(target_arch = "x86_64")]
pub const X86_COM1_PORT_BASE: u16 = 0x3F8;
/// The eight ports a 16550 occupies (data/interrupt-enable/FIFO-control/line-control/modem-control/
/// line-status/modem-status/scratch). A range because a device capability names what the hardware
/// names, the port-space twin of a `DeviceFrame` naming a page.
#[cfg(target_arch = "x86_64")]
pub const X86_COM1_PORT_COUNT: u16 = 8;

/// **Is there a UART a device capability can be a mapping of on this machine?** (milestone 161.)
///
/// [`UART_PHYS`] is zero on `x86_64` and that zero is the marker for
/// [DECISIONS §121](../../design/decisions/121-port-io-capability.md) (PROPOSED). Several fixtures
/// need to ask, so they ask here rather than each testing a constant against zero and each writing
/// its own sentence about why.
///
/// **The trap this exists to prevent is a green test rather than a red one.** A fixture that went
/// ahead anyway would grant a device capability over *physical page zero*, map it, and read
/// real-mode interrupt-vector bytes; the read would succeed, a revoke would still fault, and a
/// suite would report that x86 has a userspace device story. It does not.
#[cfg_attr(not(test), allow(dead_code))]
pub fn machine_has_no_device_page_for_the_console() -> bool {
    UART_PHYS == 0
}

/// The reason a fixture gives when [`machine_has_no_device_page_for_the_console`] is true.
#[cfg_attr(not(test), allow(dead_code))]
pub const NO_UART_PAGE: &str = "this machine's console UART is in the I/O port space, so there is \
                                no page for a device capability to be a mapping of and no \
                                capability shape for a port yet (DECISIONS \u{a7}121)";

/// **The archive entry the kernel enters as the first process**, on every architecture.
///
/// One name, one binary, one program (milestone 266). This used to be `init`, and it meant
/// `fixtures/src/hello.rs`'s `init_boot` role on aarch64 and `system_initializer` on riscv64: an alias
/// standing over two implementations of one job, which is DECISIONS §19's own failure mode and had
/// already been paid for once as a boot that reached userspace and printed nothing at all.
#[cfg_attr(not(test), allow(dead_code))]
pub const PROGENITOR_ENTRY: &str = "progenitor";

/// The archive entry holding milestone 19d's and 19e's **init roles**: the one binary the kernel
/// still re-enters at a chosen role, to play a userspace parent that builds a child out of an ELF
/// it parsed.
///
/// One name on all three architectures since milestone 266. aarch64 used to pack it as `init`,
/// because there it carried the boot role as well; that role is [`PROGENITOR_ENTRY`]'s own program
/// now, and `hello` is packed as `hello` everywhere. The kernel still enters it directly for those
/// init roles, which is why it is in `boot_programs` and measured.
///
/// **It was the whole milestone 7-19 role catalogue until milestone 291**, thirty-one roles in one
/// binary. Twenty-two of them are their own programs or `block_driver`'s roles now; nine are left,
/// and milestone 405, `design/roadmap/405-nine-init-roles-and-the-entry-the-kernel-picks.md`, is what would
/// take them, since splitting them is a change to [`spawn_hello`]'s choice of entry rather
/// than to `fixtures/`.
///
/// Name: provisional (milestone 266 (one progenitor, on all three architectures)): a constant
/// rather than a program, but it is the name a reader meets at eight call sites, and
/// `kernel::user::tests` already spelled it this way. The program's own name is overdue and is
/// calef's; see that file's `BUGS`.
#[cfg_attr(not(test), allow(dead_code))]
pub const HELLO_ENTRY: &str = "hello";

/// The progenitor's stack, in pages (19d.2c): it loads whole ELFs with deep call chains, so its stack is
/// larger than an ordinary process's one page. 8 pages (32 KiB) is generous.
#[cfg_attr(not(test), allow(dead_code))]
const INIT_STACK_PAGES: u64 = 8;

/// **The role that means "boot the system"**, as opposed to milestone 19d's test roles.
///
/// [`boot_progenitor`] passes it as the progenitor's `x0` when it loads [`PROGENITOR_ENTRY`] and
/// grants the boot capability set. The progenitor has one role and ignores it; the value is passed
/// for the symmetry the old `spawn_progenitor` established, when a single function both booted the
/// system and re-entered [`HELLO_ENTRY`] at milestone 19d's test roles (milestone 166 split those
/// two jobs, so this role no longer selects an entry).
///
/// The number is 27 because that is the role `hello`'s retired `init_boot` had, and every 19d role
/// number around it is load-bearing to a test. **Name provisional** (milestone 266).
#[cfg_attr(not(test), allow(dead_code))]
pub const PROGENITOR_ROLE: u64 = 27;

/// **Spawn a milestone-19d/19e test role of [`HELLO_ENTRY`]** (milestone 19d.2c; the boot half of
/// this function became [`boot_progenitor`] at milestone 166).
///
/// The kernel re-enters the one `hello` binary at `role` to play a userspace parent, handing it the
/// bare 19d world at fixed slots: the building budget (slot 0), the kernel's `report` endpoint (slot
/// 1, `WRITE|GRANT` so a role can report and endow a child on it), the console UART's registers
/// (slot 2), the 19d.2b test interrupt (slot 3), and the UART receive interrupt (slot 4). It returns
/// a [`holding::Holding`] over the thread and its building budget, so a test finished with it can
/// hand **2048 frames** back: six of these tests reserve 8 MiB each and the measured aarch64 boot
/// spent 12289 frames on them, **42% of everything the suite never returned** (notes/frames.md).
///
/// **This is no longer the boot path.** Until milestone 166 it was both jobs at once: at
/// [`PROGENITOR_ROLE`] it loaded [`PROGENITOR_ENTRY`] and booted the whole system, and at any other
/// role it loaded `hello`. That sharing is exactly why aarch64's boot once carried slots 1 and 3 the
/// interactive system never used, and DECISIONS §19's silent-divergence risk lived in the split. The
/// boot now goes through [`boot_progenitor`] on all three architectures, which hands the progenitor
/// the same slot layout everywhere; this function only ever plays a test role and grants only the
/// five capabilities those roles use.
///
/// **`hello` still has nine roles**, and splitting them is a follow-on to milestone 291
/// (`fixtures/src/hello.rs` was thirty-one programs wearing one name), tracked as milestone 405
/// (`design/roadmap/405-nine-init-roles-and-the-entry-the-kernel-picks.md`): six are separate
/// programs waiting to happen, and each would need its own archive entry named here.
///
/// Name: ratified 2026-09-15 (calef, this header). Refused keeping `spawn_progenitor`, the name
/// this function carried until milestone 166 split it in two: the boot half it was named for is
/// [`boot_progenitor`] now, and what stayed here never spawns the progenitor at all. It only ever
/// re-enters [`HELLO_ENTRY`] at one of milestone 19d/19e's test roles, at every call site it has
/// (all of them in `kernel/src/user/tests.rs`), so the old name pointed at the half that left.
/// `spawn` is the verb this body performs and `hello` the program it performs it on, which makes
/// the name a claim about what the function does rather than about what it used to do, and greps
/// with [`HELLO_ENTRY`] as one family.
#[cfg_attr(not(test), allow(dead_code))]
pub fn spawn_hello(
    image: &'static [u8],
    role: u64,
    report: crate::sched::RendezvousId,
) -> holding::Holding {
    let (initrd_start, initrd_len) =
        memory::initrd_region().expect("no initrd to hand the progenitor");
    let initrd_pages = initrd_len.div_ceil(FRAME_SIZE);

    // Route the test interrupt (19d.2b) BEFORE spawning the progenitor: the test raises the SGI as soon as
    // this returns, and an interrupt that fires before it is routed is dropped ("unexpected
    // interrupt"), not queued. Setting up the route here means the fire is counted on the routed
    // endpoint even though the progenitor-built child is not yet waiting; the child's WAIT drains it.
    crate::sched::bind_irq(INIT_TEST_SGI, crate::sched::create_rendezvous());
    crate::arch::irq::enable(INIT_TEST_SGI);
    // And the UART receive interrupt (19d.2c): the input driver the progenitor builds waits on it. Route and
    // enable it here, so the progenitor can delegate the Irq cap to that driver. The number is the machine's
    // (uart_irq_and_source; on the JH7110 the QEMU constant armed the wrong PLIC source, see its
    // doc), and the line names the source so a transcript is diagnosable. On QEMU RISC-V the
    // discovered line and INIT_TEST_SGI are the SAME source (see INIT_TEST_SGI), and binding it
    // twice would leave the first endpoint routed to nothing while the test waits on it, so bind
    // once and grant twice.
    let (uart_rx_intid, uart_irq_source) = uart_irq_and_source();
    crate::println!("  uart irq  : {uart_rx_intid} ({uart_irq_source})");
    if uart_rx_intid != INIT_TEST_SGI {
        crate::sched::bind_irq(uart_rx_intid, crate::sched::create_rendezvous());
        crate::arch::irq::enable(uart_rx_intid);
    }

    // Read and MEASURE `hello` here, before the thread is created (milestone 22 phase B.1): the
    // check has to be the thing that decides whether a thread is created at all, not something the
    // new thread does to itself. `trust::require` halts on a mismatch, so past this line the bytes
    // are the ones this kernel image was built against. The program-measurement table (milestone
    // 104) is required for the same reason: the whole archive is mapped into `hello`.
    let boot_fs = match nifefs::Fs::parse(image) {
        Ok(fs) => fs,
        Err(e) => {
            crate::println!("  archive is not a nifefs image: {e:?}");
            crate::arch::halt();
        }
    };
    let Some(init_bytes) = boot_fs.read(HELLO_ENTRY) else {
        crate::println!("  archive has no '{HELLO_ENTRY}' program");
        crate::arch::halt();
    };
    crate::trust::require(HELLO_ENTRY, init_bytes);
    crate::trust::require_program_measurements(&boot_fs);

    // **The building budget is carved here, not inside the thread**, so the caller has a name for it
    // and can reclaim it: a large untyped a role retypes its child's address space, frames and TCB
    // from. Carving it out here changes nothing about what the role gets; it changes who can name it
    // afterwards, the whole difference between 8 MiB spent and 8 MiB lent. See notes/frames.md.
    let build_region = crate::memory_region::create(12288).expect("no building budget for hello");

    let tid = crate::sched::spawn(move || {
        let elf = match Elf::parse(init_bytes) {
            Ok(e) => e,
            Err(e) => {
                crate::println!("  the hello image is not loadable: {e:?}");
                crate::sched::exit();
            }
        };
        // A region big enough for hello's own segments, the initrd's page tables, and slack: a role
        // that builds a child loads whole ELFs with deep call chains.
        let content: u64 = elf
            .segments()
            .map(|seg| {
                let (start, end) = seg.page_range(FRAME_SIZE);
                (end - start) / FRAME_SIZE
            })
            .sum::<u64>()
            + 1
            + initrd_pages / 512
            + crate::revoke::log_pages_for(initrd_pages)
            + INIT_STACK_PAGES
            + 8;
        let mut space = AddressSpace::new(content).expect("no memory for hello");
        map_segments(&mut space, &elf).expect("could not lay out hello");
        // A multi-page stack: a role that parses and builds a child ELF has deep call chains (the
        // loader loop, copy_from_slice, the elf parser), so one page overflows. Map INIT_STACK_PAGES
        // down from USER_STACK_TOP; the entry sp is unchanged (USER_STACK_TOP).
        for k in 0..INIT_STACK_PAGES {
            space
                .map_new(USER_STACK_VA - k * FRAME_SIZE, Flags::user_data())
                .expect("could not map hello's stack");
        }
        #[cfg(any(target_arch = "x86_64", target_arch = "riscv64"))]
        map_timebase_page(&mut space).expect("could not map hello's timebase page");

        // Map the initrd, one page at a time, read-only. These are reserved RAM pages the frame
        // allocator does not own, so this maps rather than allocates. A role that builds a child
        // parses the archive here.
        for i in 0..initrd_pages {
            space
                .map_physical(
                    INITRD_VA + i * FRAME_SIZE,
                    initrd_start + i * FRAME_SIZE,
                    Flags::user_rodata(),
                    crate::revoke::PageMapSource::NoCapability,
                )
                .expect("could not map the initrd into hello");
        }

        crate::sched::adopt_address_space(space);
        // slot 0: the delegable root budget. A role narrows and hands budgets to the children it
        // builds, so the root carries GRANT (milestone 31). Rights only narrow downward from here.
        crate::sched::grant(crate::cap::memory_region_root_cap(build_region))
            .expect("grant untyped");
        // slot 1: the kernel's report endpoint, WRITE|GRANT so a role can report a result and endow
        // a child to send on it. This slot is the boot layout's slot 1 too, which is exactly why
        // [`boot_progenitor`] once carried a report endpoint it never used: the shared function.
        crate::sched::grant(crate::cap::rendezvous_cap(
            report,
            crate::cap::Rights::WRITE.union(crate::cap::Rights::GRANT),
        ))
        .expect("grant report");
        // slot 2: the UART registers, so a console role can build a driver and hand it the registers
        // (19d.2). WRITE (device access) | GRANT (delegate to the driver).
        //
        // **On x86_64 `UART_PHYS` is zero and this grants a device capability over physical page
        // zero**, which is a foot gun and is marked as one rather than designed away (AGENTS.md's
        // ladder: an exception must say it is an exception). The slot is positional, so declining to
        // grant here would renumber the interrupts below and every role that names them. Nothing
        // reaches it today: every fixture that would map it asks
        // `machine_has_no_device_page_for_the_console()` first and skips.
        crate::sched::grant(crate::cap::device_frame_cap(
            UART_PHYS,
            crate::cap::Rights::WRITE.union(crate::cap::Rights::GRANT),
        ))
        .expect("grant uart device");
        // slot 3: the 19d.2b test interrupt, so the IRQ-delegation role can build an interrupt-driven
        // driver and hand it the Irq cap. The route was set up above, before the spawn; this only
        // grants the cap (a per-thread act). READ (WAIT/ACK) | GRANT (delegate).
        crate::sched::grant(crate::cap::irq_cap_rights(
            INIT_TEST_SGI,
            crate::cap::Rights::READ.union(crate::cap::Rights::GRANT),
        ))
        .expect("grant test irq");
        // slot 4: the UART receive interrupt, for the input driver a console role builds (19d.2c).
        // The same discovered number the route above was bound with, or the cap would name a source
        // no endpoint serves.
        crate::sched::grant(crate::cap::irq_cap_rights(
            uart_rx_intid,
            crate::cap::Rights::READ.union(crate::cap::Rights::GRANT),
        ))
        .expect("grant uart rx irq");

        // A test role has no filesystem, so `x2` (the file-service rights the boot path passes) is 0.
        enter_frame(elf.entry(), USER_STACK_TOP, role, initrd_len, 0)
    })
    .expect("could not spawn hello");

    let mut held = holding::Holding::new();
    held.add_thread(tid);
    held.add_region(build_region);
    held
}

/// **A run of contiguous device pages mapped into a new process before it starts**, for a window
/// too large to spell as [`Mapping`]s (the shell on the firmware screen: a screen's aperture is a
/// thousand pages or more, and the kernel has no heap to build a slice that long in).
///
/// Always device-typed and writable, because the one thing that needs it is a driver's view of a
/// device's memory. Like every [`Spawn::maps`] entry the process holds no *name* for it: it cannot
/// map it again, delegate it, or revoke it, which is the property `non_volatile_memory_express_service`
/// and milestone 159's TRNG driver chose spawn-time mappings for. **Name provisional.**
#[derive(Clone, Copy)]
pub struct DeviceRun {
    /// Where the first page lands in the new process.
    pub va: u64,
    /// The first page's physical address. Page-aligned.
    pub phys: u64,
    /// How many pages. The intermediate page tables come out of the address space's own
    /// `AS_OVERHEAD`, so a caller bounds this (`display_service`'s `MAX_APERTURE_PAGES`).
    pub pages: u64,
}

/// Load the initrd program and become it, handed the world described by `spawn`. Never returns.
pub fn run(image: &[u8], spawn: Spawn) -> ! {
    run_with(image, spawn, None)
}

/// [`run`], with one [`DeviceRun`] mapped as well. Never returns. **Name provisional.**
pub fn run_with_device_run(image: &[u8], spawn: Spawn, device: DeviceRun) -> ! {
    run_with(image, spawn, Some(device))
}

fn run_with(image: &[u8], spawn: Spawn, device: Option<DeviceRun>) -> ! {
    // What this process is about to have mapped into it beyond its own image: the `Spawn` windows
    // and a device run. See [`load`] for why the number is taken here rather than asked of
    // each caller.
    let windowed = spawn.maps.len() as u64 + device.as_ref().map_or(0, |d| d.pages);
    let (mut space, entry) = match load(image, windowed) {
        Ok(v) => v,
        Err(e) => {
            crate::println!();
            crate::println!("  refused to load a user program: {e:?}");
            crate::println!("  the kernel is fine.");
            crate::sched::exit();
        }
    };

    // The extra pages go in BEFORE we hand the address space off: a shared message buffer, or a
    // device's MMIO for a driver. This is the line that puts a UART into a userspace process.
    for m in spawn.maps {
        space
            .map_physical(
                m.va,
                m.phys,
                m.flags,
                crate::revoke::PageMapSource::NoCapability,
            )
            .expect("could not map a Spawn page into the new address space");
    }
    if let Some(d) = device {
        for k in 0..d.pages {
            space
                .map_physical(
                    d.va + k * FRAME_SIZE,
                    d.phys + k * FRAME_SIZE,
                    Flags::user_device(),
                    crate::revoke::PageMapSource::NoCapability,
                )
                .expect("could not map a device run into the new address space");
        }
    }

    crate::sched::adopt_address_space(space);

    // HAND IT ITS WORLD. Granted in order, so slot 0 is `grants[0]`, and reading the caller's
    // `Spawn` literal tells you the entire authority of the process. There is no path it can
    // say, no uid it can be. A capability system's "environment" is not a variable, it is this.
    for &cap in spawn.grants {
        crate::sched::grant(cap).expect("no free capability slot");
    }

    enter_at(entry, spawn.arg0, spawn.arg1, spawn.arg2)
}

/// Drop to EL0 at `entry`, on a fresh stack, with `arg0` in `x0`. Never returns.
///
/// `arg0` reaches `_start` as its first argument (AAPCS64 puts it in `x0`). It is how the kernel
/// tells one binary which of several roles to play, the way a real kernel hands a new process
/// its argc/argv. See the console server, which is the same ELF as its client with a different
/// `arg0`.
/// Drop the **current** thread to EL0 at `entry` on `user_sp`, no arguments (milestone 19c.3).
/// The entry path for a thread started through the TCB object surface, which runs on the freshly
/// scheduled thread rather than the one that called `START`. The address space is already
/// installed (the context switch that scheduled us in used our `space` field). This is `enter_at`
/// with a caller-chosen stack and zero args; `enter_at` is now the exec wrapper over it.
pub fn enter_at_on_current(entry: u64, user_sp: u64, arg0: u64, arg1: u64, arg2: u64) -> ! {
    enter_frame(entry, user_sp, arg0, arg1, arg2)
}

fn enter_at(entry: u64, arg0: u64, arg1: u64, arg2: u64) -> ! {
    enter_frame(entry, USER_STACK_TOP, arg0, arg1, arg2)
}

fn enter_frame(entry: u64, user_sp: u64, arg0: u64, arg1: u64, arg2: u64) -> ! {
    // THE TRAPFRAME IS NOT AN ORDINARY LOCAL, and this cost us an afternoon.
    //
    // It must sit at the TOP OF THIS THREAD'S KERNEL STACK, because that is where the hardware
    // will look for it. `enter_userspace` does `mov sp, x0`, and `exception_restore` leaves
    // SP_EL1 = x0 + 272 across the `eret`. So when the user traps back in, `SAVE_CONTEXT`
    // subtracts 272 and rebuilds the frame **at exactly this address**. It had better be
    // writable, and it had better be a stack.
    //
    // The first version wrote `enter_userspace(&TrapFrame { .. })`, and every field of that
    // struct is a compile-time constant, so Rust CONST-PROMOTED IT INTO .rodata. The kernel
    // set SP_EL1 to read-only memory, and the user's first `svc` faulted trying to write its
    // own trap frame there. See notes/userspace.md: the kernel then walked `sp` DOWNWARD
    // through .rodata and the whole of .text, 272 bytes and one fault at a time, until it fell
    // out of the bottom of the image into writable RAM and could finally tell us.
    let top = crate::sched::current_kernel_stack_top()
        .expect("a user thread needs a kernel stack of its own to be trapped onto");

    // **The frame goes at the very top of the kernel stack, on both ISAs, and it must be ABOVE the
    // live `sp`.** Everything below `sp` belongs to somebody else: a callee's frame, and on a trap
    // the 288/272 bytes the vector subtracts from `sp` to build its own frame. An object parked
    // there is not stored, it is lent.
    //
    // RISC-V used to compute this from the live `sp` instead (`(current_sp().min(top) - size) & !15`),
    // because its TCB entry path is shallow and a frame at the top would have overlapped this
    // function's own stack. That traded a deterministic overlap for an intermittent one, and
    // milestone 71 caught it: `current_sp()` is a real call at opt-level 0, so it returned
    // `sp - 16`, which put the frame at `sp - 304` while `trap.s` builds an S-mode trap frame at
    // `sp - 288`. The two differ by exactly 16 bytes, so the user frame's `x[2]` (the user `sp`)
    // sat precisely on the trap frame's `x[0]` slot, which `trap.s` writes as a literal zero. Any
    // timer interrupt taken between building the frame and consuming it therefore rewrote the whole
    // frame: user `sp` read 0 every time, `sepc` read whatever `t5` held, and `sstatus` read the
    // trap's `scause` (whose UXL bits are 0, an illegal U-mode XLEN). When `t5` happened to be 0 the
    // `sepc == 0` guard in `enter_user` fired; when it did not, the thread `sret`ed to a garbage PC,
    // died on its first instruction, and never answered whoever was waiting on it, which is a
    // lost-wakeup hang with no guard message. See notes/riscv-port.md.
    //
    // The shallow-path problem the old code was avoiding is real, and the fix for it is a
    // reservation rather than a moving target: `user_entry_trampoline` (both ISAs) drops `sp` by a
    // frame's worth before the first Rust frame exists, so this region is off-limits to the entry
    // path by construction. See arch/*/context.s.
    //
    // `thread_trampoline` deliberately does NOT reserve, and the asymmetry is the point. Only the
    // TCB path can be shallow; the exec path reaches here through `run` and the ELF loader, so its
    // frames are always far below the stack top, and reserving would spend a frame's worth on every
    // kernel thread to insure against a depth that cannot happen. If that ever stops being true, the
    // assertion below is what says so.
    let slot = top - size_of::<TrapFrame>() as u64;
    let frame = slot as *mut TrapFrame;

    // And prove it, rather than trusting the reasoning above. This is one check, once per
    // exec, against a bug whose symptom is a nested fault storm that eats the kernel image.
    assert!(
        mmu::translate(frame as u64).is_some_and(|(_, f)| f.is_writable()),
        "the user's TrapFrame at {frame:p} is not in writable memory",
    );

    // **The invariant the milestone 71 fault violated**, checked rather than reasoned about. A slot
    // at or below the live `sp` is one a callee or a trap will build over, and the old RISC-V
    // placement failed this on the very first user entry. Necessary rather than sufficient: this
    // function's own frame sits *above* `sp` and is not covered, which is what the trampoline
    // reservation handles. Cheap enough to keep: one comparison per exec.
    assert!(
        slot >= crate::arch::current_sp(),
        "the user's TrapFrame at {frame:p} is below the live sp: a callee frame or a trap frame \
         will be built over it",
    );

    // SAFETY: `frame` is 16-byte-aligned writable kernel stack (a KernelStack top is page
    // aligned and TrapFrame is a multiple of 16), the user code and stack are mapped, and the
    // user address space is installed. `arch` owns the register layout: we ask for a user-entry
    // frame and hand it back to `arch` to make the jump (notes/riscv-port.md, leak #3).
    unsafe {
        frame.write(TrapFrame::for_user_entry(
            entry,
            user_sp,
            [arg0, arg1, arg2],
        ));
        enter_user(frame)
    }
}

// --- the test programs the kernel loads by name ---
//
// **There used to be five hand-assembled programs here**, three aarch64 and two RISC-V, written as
// `global_asm!` machine code in `.rodata` and copied into a user page by a one-page loader. They
// were honest milestone-7a scaffolding: there was no ELF loader and no filesystem to load from, so
// the "binary" rode inside the kernel image.
//
// They are gone (milestone 19's user-test port). The behaviours are ordinary (yield twice, read a
// forbidden address, spin forever), the toolchain builds them for both targets, and the initrd
// already delivers thirty other programs, so the scaffolding had outlived its reason twice over:
// once when 7c shipped the ELF loader, and again when the second ISA turned "one hand-written
// program" into "two hand-written programs, forever". Keeping them would have meant hand-assembling
// every one of them a second time to run the same tests on RISC-V.
//
// What replaced each:
//
//   - aarch64 `hello`, riscv `USER_HELLO` (yield, yield)  -> `outlaw`, role `OUTLAW_ROUND_TRIP`
//   - aarch64 `outlaw`  (read a kernel address)           -> `outlaw`, role `OUTLAW_READ_KERNEL`
//   - aarch64 `spin`    (loop, no syscall, no stack)      -> the `interrupt_ignorer` binary (DECISIONS §24)
//   - riscv `USER_REPORTER` (invoke a cap, SEND a word)   -> `riscv_least_authority_demo`, which builds a
//     process from the same parts and runs a real ELF through them
//
// This also removed `exec`, the one-page raw-machine-code loader they needed. Every program the
// kernel runs now arrives as an ELF.

/// The `outlaw` program's roles (fixtures/src/outlaw.rs), passed in the first argument register.
///
/// `ROUND_TRIP` yields twice and exits: two syscalls from user mode, where the second can only
/// happen if the return from the first genuinely put the thread back at EL0/U-mode.
// The tests use it on both ISAs; of the two boot tours only RISC-V's has a syscall-count step.
#[cfg_attr(not(test), allow(dead_code))]
pub const OUTLAW_ROUND_TRIP: u64 = 0;

/// `READ_KERNEL` reads the address handed to it in the second argument register, which is what makes
/// the program portable: the kernel's own memory lives at a different virtual address on each ISA,
/// and the caller knows which. See `tests::a_user_program_cannot_read_a_kernel_address`.
// The tour uses it, and the shell/bench boots skip the tour.
#[cfg_attr(not(test), allow(dead_code))]
pub const OUTLAW_READ_KERNEL: u64 = 1;

/// **Load and run a real compiled ELF at U-mode on RISC-V** (milestone 20, the user-ELF step).
///
/// This takes the bytes of the `least_authority_demo` program (a Rust binary compiled to a riscv64 ELF, delivered
/// as the initrd)
/// and runs them through the kernel's *real* ELF loader. [`load`] parses the file, builds an address
/// space with each `PT_LOAD` segment mapped W^X at the VA it names, and maps a stack; nothing here is
/// riscv-specific except that the loader was just taught to accept `EM_RISCV`. The `least_authority_demo` is granted
/// WRITE on one endpoint as its slot 0, started with the input `n` in its second argument register
/// (`a1`), squares it, and SENDs the answer home.
///
/// Receiving `n*n` proves the whole ELF path works on RISC-V: parse, segment mapping with correct
/// permissions, the entry point, argument passing across the `START` boundary, and the endpoint
/// SEND, all from a program the kernel did not hand-write. `load` is arch-neutral; this is the same
/// code aarch64 runs, now on the RISC-V address space and trap path.
/// The hand-assembled `x86_64` programs. Compiled ones now exist (item 4's hand-off), but these
/// stay: the boot tour runs before any archive is parsed, and a fixture that needs no initrd is
/// what lets the userspace demo run on a `cargo run` with no `-initrd` at all.
#[cfg(target_arch = "x86_64")]
pub mod x86_programs;

/// Where the x86 demo's children put their code and stack. Any two low-half pages would do; these
/// are the ones the supervision fixtures use on every architecture, so a reader who has seen one
/// recognises them.
#[cfg(target_arch = "x86_64")]
const X86_DEMO_CODE_VA: u64 = 0x40_0000;
#[cfg(target_arch = "x86_64")]
const X86_DEMO_STACK_VA: u64 = 0x50_0000;
/// The word the reporting child SENDs, and the address the faulting one loads from. Both are
/// distinctive so that a zero anywhere in the report is visibly a failure rather than a plausible
/// value.
#[cfg(target_arch = "x86_64")]
const X86_DEMO_WORD: u32 = 0x0161_0004;
#[cfg(target_arch = "x86_64")]
const X86_DEMO_BAD_ADDR: u32 = 0x00A5_0000;

/// What the x86 userspace demo found. Every field is something a **program in ring 3** or the
/// **kernel's own supervision path** produced, rather than something the tour assumed.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy)]
pub struct X86UserspaceReport {
    /// The word the reporting child sent, as it arrived on the endpoint. Proves the child reached
    /// ring 3, made a `syscall` that reached the portable dispatcher, and was answered.
    pub reported: u64,
    /// The thread id the kernel stamped on the faulting child's death message.
    pub faulted_tid: u64,
    /// The pc the faulting child died at, from the message. `X86_DEMO_CODE_VA + 5` if the fault
    /// landed on the instruction it was supposed to.
    pub fault_pc: u64,
    /// The address it faulted on, from the message.
    pub fault_addr: u64,
    /// **What the first round of two children cost the frame allocator**, net of their regions
    /// being destroyed.
    pub first_round_frames: isize,
    /// **What an identical second round cost.** This is the number that means something, and the
    /// reason the demo runs twice: a first round pays first-use carves that are not leaks (the
    /// kernel's object budget, the endpoint region, a thread stack the recycler has not seen yet),
    /// and a system that has reached a steady state charges the second round **zero**. It is the
    /// same distinction `thread.rs`'s stack-VA reuse test draws, and the same evidence.
    pub second_round_frames: isize,
}

/// **Build one hand-assembled child out of `region` and start it.** The x86 boot tour's own
/// `build_child_in`, kept beside the demo rather than shared with `supervision_tests` because that
/// module is `#[cfg(test)]` and this runs on an ordinary boot.
///
/// `slot0` is the capability the program's own slot 0 will hold, if any; `fault_ep` goes in the
/// reserved fault slot, so `START` records it as this child's supervision endpoint.
#[cfg(target_arch = "x86_64")]
fn x86_build_child(
    region: u64,
    program: &[u32],
    slot0: Option<crate::cap::Cap>,
    fault_ep: Option<crate::sched::RendezvousId>,
) -> Result<u64, &'static str> {
    let aspace = user_address_space_create(region).ok_or("no aspace for the child")?;

    let code_phys = crate::memory_region::retype_page(region).ok_or("no code frame")?;
    // SAFETY: a fresh frame this region owns, reachable through the direct map; the program is
    // written there and then mapped executable. The kernel cannot address `X86_DEMO_CODE_VA`
    // itself, which is why the frame is written through its physical name instead.
    unsafe {
        let dst = mmu::phys_to_virt(code_phys) as *mut u32;
        for (i, &word) in program.iter().enumerate() {
            dst.add(i).write(word);
        }
    }
    // A no-op on this architecture (the instruction cache is architecturally coherent), and called
    // anyway because the seam is what the other two need and skipping it here would make this code
    // wrong to copy.
    sync_icache(
        mmu::phys_to_virt(code_phys),
        core::mem::size_of_val(program),
    );
    user_address_space_map(
        aspace,
        X86_DEMO_CODE_VA,
        code_phys,
        Flags::user_code(),
        crate::revoke::PageMapSource::NoCapability,
    )
    .map_err(|_| "could not map the child's code")?;

    let stack_phys = crate::memory_region::retype_page(region).ok_or("no stack frame")?;
    user_address_space_map(
        aspace,
        X86_DEMO_STACK_VA,
        stack_phys,
        Flags::user_data(),
        crate::revoke::PageMapSource::NoCapability,
    )
    .map_err(|_| "could not map the child's stack")?;

    let tid = crate::sched::create_thread_control_block(region).ok_or("no tcb")?;
    if let Some(cap) = slot0 {
        let slot = crate::sched::thread_control_block_insert_cap(tid, cap, None)
            .map_err(|_| "no room for the child's slot 0")?;
        if slot != 0 {
            return Err("the child's capability did not land in slot 0, which its code assumes");
        }
    }
    if let Some(ep) = fault_ep {
        // The spawn-slot convention: a supervision endpoint goes in the reserved fault slot, and
        // the kernel consumes it at START so the child cannot forge fault messages on it.
        let cap = crate::cap::rendezvous_cap(ep, crate::cap::Rights::READ);
        crate::sched::thread_control_block_insert_cap(tid, cap, Some(abi::fault::FAULT_EP_SLOT))
            .map_err(|_| "no room for the fault endpoint")?;
    }
    crate::sched::configure_thread_control_block(
        tid,
        X86_DEMO_CODE_VA,
        X86_DEMO_STACK_VA + FRAME_SIZE,
        aspace,
    )
    .map_err(|_| "could not configure the child")?;
    crate::sched::start_thread_control_block(tid, [0; 3])
        .map_err(|_| "could not start the child")?;
    Ok(tid)
}

/// **Prove there is a userspace on `x86_64`**, which is the claim roadmap item 4 exists to make and
/// is a strictly larger one than item 3's ring-3 probe.
///
/// Two children, because the two halves of "a process" fail differently and a single program that
/// did both could hide one behind the other:
///
///   - **One reports and exits.** Its whole world (address space, code page, stack page, TCB) is
///     carved from one untyped region, it is dispatched to ring 3 by the *scheduler* rather than by
///     a hand-written entry path, it invokes a capability it was granted, and the word it SENDs
///     arrives here. That is the loader-shaped path minus the ELF: every kernel object a process
///     needs, built from a budget, in the order a real spawn builds them.
///   - **One faults.** It loads from an address nothing maps, the page tables refuse it, and the
///     trap path turns that into a supervision message naming the thread, the pc and the address.
///     Until this item the same arm recorded the fault and then panicked, because there was no
///     thread to kill.
///
/// And then both regions are destroyed and the frame count is compared, because a userspace that
/// leaks its processes is not one.
///
/// Name: provisional (milestone 161, roadmap item 4).
#[cfg(target_arch = "x86_64")]
pub fn x86_userspace_demo() -> Result<X86UserspaceReport, &'static str> {
    let before = crate::memory::free_page_frames();
    let round = x86_userspace_round()?;
    let after_first = crate::memory::free_page_frames();
    // The same two children again, from scratch. See `X86UserspaceReport::second_round_frames`.
    x86_userspace_round()?;
    let after_second = crate::memory::free_page_frames();

    Ok(X86UserspaceReport {
        first_round_frames: before as isize - after_first as isize,
        second_round_frames: after_first as isize - after_second as isize,
        ..round
    })
}

/// One round of the demo: build both children, collect what each produced, and give their regions
/// back. Called twice by [`x86_userspace_demo`], which is what turns its frame numbers into
/// evidence.
///
/// **Every kernel object a child needs comes out of that child's own region**, its two endpoints
/// included (`create_rendezvous_from`), so one `DESTROY` reclaims the whole of it and the frame
/// count is an exact statement rather than an approximate one. The first version drew the endpoints
/// from the kernel's shared pool and never collected the reporting child's corpse, and the tour
/// reported sixteen frames a round going missing: correct, and exactly the kind of thing a
/// steady-state number is for.
#[cfg(target_arch = "x86_64")]
fn x86_userspace_round() -> Result<X86UserspaceReport, &'static str> {
    use abi::fault::{EVENT_EXIT, EVENT_FAULT};

    // Sixteen pages is what the supervision fixtures give a child on the other two architectures:
    // an address space's root and tables, a code page, a stack page, a TCB, and here two endpoints.
    let report_region =
        crate::memory_region::create(16).ok_or("no region for the reporting child")?;
    let report_ep =
        crate::sched::create_rendezvous_from(report_region).ok_or("no reporting endpoint")?;
    let reporter_supervisor = crate::sched::create_rendezvous_from(report_region)
        .ok_or("no supervision endpoint for the reporting child")?;
    let reporter = x86_build_child(
        report_region,
        &x86_programs::report(X86_DEMO_WORD),
        Some(crate::cap::rendezvous_cap(
            report_ep,
            crate::cap::Rights::WRITE,
        )),
        Some(reporter_supervisor),
    )?;
    let reported = crate::sched::ipc_recv(report_ep)[0];

    // **Collect the corpse before reclaiming the region**, which is what a supervisor is for and
    // what the first draft of this left out: a region still holding a live TCB is refused, and the
    // refusal is silent because `destroy` has nowhere to report it.
    let exit = crate::sched::ipc_recv(reporter_supervisor);
    if exit[0] != EVENT_EXIT {
        return Err("the reporting child's clean exit did not arrive as an EXIT event");
    }
    crate::sched::reap_supervised(reporter_supervisor, reporter)
        .map_err(|_| "the reporting child's corpse refused to be reaped")?;

    // The faulting child, in a region of its own.
    let fault_region =
        crate::memory_region::create(16).ok_or("no region for the faulting child")?;
    let fault_ep = crate::sched::create_rendezvous_from(fault_region)
        .ok_or("no supervision endpoint for the faulting child")?;
    let child = x86_build_child(
        fault_region,
        &x86_programs::fault(X86_DEMO_BAD_ADDR),
        None,
        Some(fault_ep),
    )?;
    let msg = crate::sched::ipc_recv(fault_ep);
    if msg[0] != EVENT_FAULT {
        return Err("the child's death did not arrive as a FAULT event");
    }
    if msg[1] != child {
        return Err("the fault message named the wrong thread");
    }
    if msg[2] != X86_DEMO_CODE_VA + x86_programs::FAULT_PC_OFFSET {
        return Err("the faulting pc was not the load instruction");
    }
    if msg[3] != X86_DEMO_BAD_ADDR as u64 {
        return Err("the faulting address was not carried in the message");
    }
    crate::sched::reap_supervised(fault_ep, child)
        .map_err(|_| "the faulting child's corpse refused to be reaped")?;

    crate::memory_region::destroy(report_region);
    crate::memory_region::destroy(fault_region);

    Ok(X86UserspaceReport {
        reported,
        faulted_tid: msg[1],
        fault_pc: msg[2],
        fault_addr: msg[3],
        first_round_frames: 0,
        second_round_frames: 0,
    })
}

#[cfg(target_arch = "riscv64")]
pub fn riscv_least_authority_demo(least_authority_demo: &[u8], n: u64) -> Result<u64, LoadError> {
    // The kernel's real loader: parse, build the address space, map the W^X segments and a stack.
    let (space, entry) = load(least_authority_demo, 0)?;
    // `load` returns an owned AddressSpace; the TCB path binds one by registry name, so register it.
    let aspace_name = readopt_user_address_space(space).expect("register the loaded address space");

    // The least_authority_demo's one authority: WRITE on a report endpoint, which it will hold as slot 0.
    let result = crate::sched::create_rendezvous();
    let result_cap = crate::cap::rendezvous_cap(result, crate::cap::Rights::WRITE);

    // Build the thread from parts: a TCB, the cap in slot 0, configure at the ELF's entry, start.
    let thread_control_block_region = crate::memory_region::create(2).expect("no tcb region");
    let tid =
        crate::sched::create_thread_control_block(thread_control_block_region).expect("no tcb");
    let slot =
        crate::sched::thread_control_block_insert_cap(tid, result_cap, None).expect("cap insert");
    assert_eq!(
        slot, 0,
        "the least_authority_demo's report cap must land in slot 0"
    );
    crate::sched::configure_thread_control_block(tid, entry, USER_STACK_TOP, aspace_name)
        .expect("configure");
    // The least_authority_demo reads its input from a1 (the second argument); a0 and a2 are unused.
    crate::sched::start_thread_control_block(tid, [0, n, 0]).expect("start");

    Ok(crate::sched::ipc_recv(result)[0])
}

/// **Start the interrupt-driven UART driver as an unprivileged userspace process** (milestone 20).
///
/// The device-interrupt story's real form: a driver that owns the UART's interrupt by *capability*,
/// not by privilege. The kernel loads `driver` from the archive, builds its address space, maps the
/// NS16550's registers into it device-typed (so the driver reads the byte itself; the kernel is not
/// in the data path), and grants it exactly two capabilities: an `Irq` capability for the UART
/// interrupt (slot 0) and a report endpoint (slot 1). It routes the interrupt to the endpoint the
/// `Irq` cap waits on, starts the driver, and arms the source (PLIC), the receive interrupt (UART),
/// and supervisor external interrupts (`sie.SEIE`).
///
/// Returns the report endpoint. This does **not** block: the caller spawns a receiver so the boot
/// tour continues, and the driver's `WAIT`/read/report/`ACK` loop runs whenever a byte arrives. The
/// `ACK` is the point of the whole exercise: it crosses the `arch::irq` seam (the PLIC on RISC-V, the
/// GIC on aarch64) to re-arm the source, from an unprivileged process holding only a capability.
#[cfg(target_arch = "riscv64")]
pub fn riscv_uart_driver_demo(
    archive: &'static [u8],
    uart_irq: u32,
) -> Result<crate::sched::RendezvousId, LoadError> {
    const DRIVER_UART_VA: u64 = 0x0070_0000; // must match components/src/serial_driver.rs UART_VA
    const UART_PHYS: u64 = 0x1000_0000; // the NS16550 on QEMU virt

    let fs = nifefs::Fs::parse(archive).expect("initrd is not a nifefs archive");
    let driver_bytes = fs
        .read("serial_driver")
        .expect("archive has no 'serial_driver' program");
    let elf = Elf::parse(driver_bytes).map_err(LoadError::NotLoadable)?;

    // The driver's address space: its segments, a stack, and the UART's registers device-typed.
    let content: u64 = elf
        .segments()
        .map(|seg| {
            let (s, e) = seg.page_range(FRAME_SIZE);
            (e - s) / FRAME_SIZE
        })
        .sum::<u64>()
        + 1
        + INIT_STACK_PAGES
        + 8;
    let mut space =
        AddressSpace::new(content).ok_or(LoadError::Unmappable(MapError::OutOfPageFrames))?;
    map_segments(&mut space, &elf)?;
    for k in 0..INIT_STACK_PAGES {
        space
            .map_new(USER_STACK_VA - k * FRAME_SIZE, Flags::user_data())
            .map_err(LoadError::Unmappable)?;
    }
    // The UART registers, device-typed and user-accessible: the driver reads RBR/LSR directly.
    space
        .map_physical(
            DRIVER_UART_VA,
            UART_PHYS,
            Flags::user_device(),
            crate::revoke::PageMapSource::NoCapability,
        )
        .map_err(LoadError::Unmappable)?;

    let aspace_name = readopt_user_address_space(space).expect("register driver address space");

    // Route the UART interrupt to an endpoint; the Irq cap's WAIT blocks on it. The report endpoint
    // is where the driver SENDs each byte, and where the caller's receiver waits.
    let irq_ep = crate::sched::create_rendezvous();
    crate::sched::bind_irq(uart_irq, irq_ep);
    let report = crate::sched::create_rendezvous();

    let thread_control_block_region = crate::memory_region::create(2).expect("no tcb region");
    let tid =
        crate::sched::create_thread_control_block(thread_control_block_region).expect("no tcb");
    // slot 0: the Irq capability (READ permits WAIT/ACK). slot 1: the report endpoint (WRITE).
    let s0 = crate::sched::thread_control_block_insert_cap(
        tid,
        crate::cap::irq_cap_rights(uart_irq, crate::cap::Rights::READ),
        None,
    )
    .expect("insert irq cap");
    assert_eq!(s0, 0, "the Irq cap must land in slot 0");
    let s1 = crate::sched::thread_control_block_insert_cap(
        tid,
        crate::cap::rendezvous_cap(report, crate::cap::Rights::WRITE),
        None,
    )
    .expect("insert report");
    assert_eq!(s1, 1, "the report endpoint must land in slot 1");
    crate::sched::configure_thread_control_block(tid, elf.entry(), USER_STACK_TOP, aspace_name)
        .expect("configure");
    crate::sched::start_thread_control_block(tid, [0, 0, 0]).expect("start");

    // Arm the whole chain, now that the driver is running and routed: the source at the PLIC, the
    // receive interrupt at the UART, and supervisor external interrupts in `sie`.
    crate::drivers::plic::enable(uart_irq, crate::arch::irq::boot_s_context());
    crate::console::rx_enable();
    crate::arch::exceptions::enable_external();

    Ok(report)
}

/// **Boot the system: load the progenitor and start it as the first process, on every architecture**
/// (milestone 166, which merged aarch64's `spawn_progenitor` boot half and `riscv64`/`x86_64`'s
/// `riscv_shell_boot` into this one body).
///
/// It loads [`PROGENITOR_ENTRY`], measures it under the trust root (milestone 22 phase B.1) and the
/// program-measurement table it will check its own loads against (milestone 104), builds its address
/// space (its segments, a deep stack, and the whole initrd mapped read-only so it can parse and load
/// the rest by name), endows it with the boot capability set, and starts it. From those capabilities
/// and nothing else, `crates/system_initializer` builds the console server, the input driver, the
/// line discipline and the shell out of the progenitor's own budget and wires them together; the
/// kernel touches none of it. It does not block: the progenitor and its children run on the
/// scheduler while the boot thread parks.
///
/// **The capability slot layout is the same on all three architectures** (milestone 166's point):
/// the delegable root budget at slot 0, the console device at slot 1, the UART receive interrupt at
/// slot 2, the wall clock page read-only at slot 3 (milestone 51's wiring), the inert-configuration
/// page read-only at slot 4 (milestone 47's environment-variable fork, DECISIONS §111), the file
/// service and the page its clients share at slots 5 and 6 when a RedoxFS disk is attached (milestone
/// 50), the virtio-rng trio at 7-9, the graphical terminal stack at 10-12 and the virtio-net trio
/// at 13-15 (milestone 590 (the booted system starts its network stack)) when each is present.
/// That fills sixteen of the table's
/// twenty-four slots at spawn, which is why the progenitor spends the net trio before anything else.
/// `components/src/progenitor.rs`'s single `GRANTS` table reads exactly this. Until milestone 166
/// aarch64's boot carried two extra capabilities at slots 1 and 3 (a report endpoint and a test
/// interrupt) that the interactive system never used, only because its loader was shared with
/// milestone 19d's test roles; [`spawn_hello`] is that shared role path, now boot-free.
///
/// **Only two differences are the hardware's, and only those are `#[cfg]`-gated**:
/// - **The console device (slot 1).** aarch64 and riscv64 grant the UART's registers as a
///   `DeviceFrame` (a page), `WRITE|GRANT` so the progenitor maps them into the console and input
///   drivers it builds. `x86_64` has no page for its console (COM1 is port I/O), so it grants a
///   `PortRange` over `0x3F8..=0x3FF` instead (milestone 299, DECISIONS §121 reversed 2026-09-15):
///   the progenitor delegates it with `CAP_INSERT` rather than `MAP_INTO`, and the kernel's TSS I/O
///   bitmap is what lets the drivers' `in`/`out` reach exactly these ports.
/// - **Arming the interrupt controller.** aarch64's GIC has no boot-hart lottery, so its UART line
///   is enabled inline before the thread is built and its virtio-rng source as its caps are inserted;
///   riscv64 enables the PLIC source and supervisor external interrupts only after the driver is
///   running, because the lottery forbids arming earlier (see [`VirtioBootGrant::intid`]); `x86_64`
///   arms nothing here, because it has no userspace input driver to feed until DECISIONS §149.
///
/// Returns the progenitor's thread, so the caller can say how it left.
///
/// Name: ratified 2026-09-15 (calef, this header). Refused `boot_via_progenitor` (the provisional
/// name from milestone 268, whose `via` did two jobs and has spent both: it disambiguated this entry
/// from aarch64's separate boot path, which milestone 166 unified away, and it gestured at the
/// microkernel indirection, which the sentence "on this path the progenitor **is** the system" says
/// better than a preposition in a name can). `boot` is the term-of-art verb and `progenitor` the
/// noun it acts on, so the name claims this function's own action, load the `progenitor` program,
/// measure it, and start it, rather than the system bring-up `progenitor` itself does next. Greps
/// with [`PROGENITOR_ENTRY`] and [`PROGENITOR_ROLE`] as one family.
// One caller per architecture, all in `kernel::main`'s hand-off (aarch64's default boot, and
// `riscv_hand_over`/`x86_hand_over`). The `allow` is kept for the configurations that reach none of
// them: a `soak` or `job_mix` build replaces the hand-off with its own workload, and `test`/`bench`
// park before it.
#[cfg_attr(
    any(
        test,
        feature = "bench",
        feature = "soak_test",
        feature = "job_mix",
        feature = "disk_throughput"
    ),
    allow(dead_code)
)]
pub fn boot_progenitor(archive: &'static [u8]) -> Result<crate::thread::ThreadId, LoadError> {
    use crate::cap::Rights;

    // The UART receive interrupt line, from the machine's own description when it gave one
    // (`memory::uart_irq`), else QEMU virt's constant. The source is printed so a bench transcript
    // names where the number came from: a wrong PLIC source once cost a boot on the JH7110
    // (notes/visionfive2.md, BUGS).
    let (uart_irq, uart_irq_source) = uart_irq_and_source();
    crate::println!("  uart irq  : {uart_irq} ({uart_irq_source})");

    let (initrd_start, initrd_len) = memory::initrd_region().expect("no initrd region");
    let initrd_pages = initrd_len.div_ceil(FRAME_SIZE);

    let fs = nifefs::Fs::parse(archive).expect("initrd is not a nifefs archive");
    let init_bytes = fs
        .read(PROGENITOR_ENTRY)
        .unwrap_or_else(|| panic!("archive has no '{PROGENITOR_ENTRY}' program"));
    // Measured boot (milestone 22 phase B.1): the progenitor is this board's boot program too, so
    // it is in the trust root under its own name and checked here, before its address space is
    // built.
    crate::trust::require(PROGENITOR_ENTRY, init_bytes);
    // And the table it measures the six boot components and every spawnable program against
    // (milestone 104). This is the boot path that genuinely uses it: `crates/system_initializer` is
    // the same code aarch64's the progenitor runs, so the two boards extend the chain by the same lines.
    crate::trust::require_program_measurements(&fs);
    let elf = Elf::parse(init_bytes).map_err(LoadError::NotLoadable)?;

    // system_initializer's address space: its segments, a deep stack (it runs an ELF loader that builds three
    // children), and the whole archive mapped read-only so it can load them by name.
    //
    // `log_pages_for(initrd_pages)` is the term that was not here before 2026-09-21: the archive's
    // pages are mapped with `map_physical`, which records now, and a record is paid for out of this
    // space's own region like the page tables beside it. It is the largest single such window in
    // the tree (the aarch64 archive is a few thousand pages), so it is the one place the cost is
    // visible rather than lost in `AS_OVERHEAD`'s slack. See `crate::revoke::log_pages_for`.
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
    let mut space =
        AddressSpace::new(content).ok_or(LoadError::Unmappable(MapError::OutOfPageFrames))?;
    map_segments(&mut space, &elf)?;
    for k in 0..INIT_STACK_PAGES {
        space
            .map_new(USER_STACK_VA - k * FRAME_SIZE, Flags::user_data())
            .map_err(LoadError::Unmappable)?;
    }
    // The timebase page, which [`load`] maps for every process it builds and a hand-built
    // address space has to map for itself (see [`map_timebase_page`] for the six call sites
    // that each found this as a page fault). The progenitor reads the clock like any program does.
    #[cfg(any(target_arch = "x86_64", target_arch = "riscv64"))]
    map_timebase_page(&mut space).map_err(LoadError::Unmappable)?;
    for i in 0..initrd_pages {
        space
            .map_physical(
                INITRD_VA + i * FRAME_SIZE,
                initrd_start + i * FRAME_SIZE,
                Flags::user_rodata(),
                crate::revoke::PageMapSource::NoCapability,
            )
            .map_err(LoadError::Unmappable)?;
    }
    let aspace_name =
        readopt_user_address_space(space).expect("register the progenitor's address space");

    // Route the UART receive interrupt to an endpoint; the input driver's Irq cap will WAIT on it.
    let irq_ep = crate::sched::create_rendezvous();
    crate::sched::bind_irq(uart_irq, irq_ep);
    // aarch64's GIC has no boot-hart-lottery hazard, so its UART line is enabled here, the same
    // place and way the old `spawn_progenitor` armed it. riscv64 must wait until the driver is
    // running (the arming block after the start below); `x86_64` arms nothing (DECISIONS §149).
    #[cfg(target_arch = "aarch64")]
    crate::arch::irq::enable(uart_irq);
    let build_region =
        crate::memory_region::create(12288).expect("no building budget for the progenitor");

    let thread_control_block_region = crate::memory_region::create(2).expect("no tcb region");
    let tid =
        crate::sched::create_thread_control_block(thread_control_block_region).expect("no tcb");
    // slot 0: the delegable root budget (milestone 31), GRANT included so the progenitor can split
    // off a budget for the shell and hand it on; rights only narrow downward. slot 1: the console
    // device (see below), WRITE|GRANT so the progenitor delegates it into the console and input
    // drivers. slot 2: the UART Irq, READ|GRANT so it can delegate it to input.
    let s0 = crate::sched::thread_control_block_insert_cap(
        tid,
        crate::cap::memory_region_root_cap(build_region),
        None,
    )
    .expect("insert budget");
    assert_eq!(s0, 0);
    // Slot 1: the console device. On aarch64 and riscv64 it is the UART's registers as a
    // `DeviceFrame` (a page); on `x86_64` the console is port I/O with no page, so it is a
    // `PortRange` over COM1's eight ports instead. See this function's doc for the full split.
    #[cfg(not(target_arch = "x86_64"))]
    let uart_slot = crate::cap::device_frame_cap(UART_PHYS, Rights::WRITE.union(Rights::GRANT));
    // **On `x86_64` the console is a port range, not a page** (milestone 299, DECISIONS §121
    // reversed 2026-09-15). COM1's 16550 lives at I/O ports `0x3F8..=0x3FF`, so `uart_dev` names a
    // `PortRange` capability rather than a device page: the progenitor holds it with `GRANT` and
    // delegates it (`CAP_INSERT`, not `MAP_INTO`) into the console and input drivers it builds, and
    // the kernel's TSS I/O bitmap is what lets their `in`/`out` reach exactly these ports.
    #[cfg(target_arch = "x86_64")]
    let uart_slot = crate::cap::port_range_cap(
        X86_COM1_PORT_BASE,
        X86_COM1_PORT_COUNT,
        Rights::WRITE.union(Rights::GRANT),
    );
    let s1 = crate::sched::thread_control_block_insert_cap(tid, uart_slot, Some(1))
        .expect("insert uart device");
    assert_eq!(s1, 1);
    let s2 = crate::sched::thread_control_block_insert_cap(
        tid,
        crate::cap::irq_cap_rights(uart_irq, Rights::READ.union(Rights::GRANT)),
        Some(2),
    )
    .expect("insert uart irq");
    assert_eq!(s2, 2);
    // The clock page (slot 3), read-only, ahead of the filesystem pair so its number is the same on
    // every boot. `READ` is DECISIONS §43's split at this boundary: the progenitor can endow a reader and holds
    // nothing that could set the time. See [`boot_clock_page`].
    let s3 = crate::sched::thread_control_block_insert_cap(
        tid,
        crate::cap::page_frame_cap(boot_clock_page(), Rights::READ.union(Rights::GRANT)),
        Some(3),
    )
    .expect("insert the clock page");
    assert_eq!(s3, 3);
    // The inert-configuration page (slot 4), `clock`'s twin (milestone 47, DECISIONS §111): ahead
    // of the filesystem pair for the identical reason, its slot must not depend on whether a disk
    // was attached. See [`boot_config_page`].
    let s4 = crate::sched::thread_control_block_insert_cap(
        tid,
        crate::cap::page_frame_cap(boot_config_page(), Rights::READ.union(Rights::GRANT)),
        Some(4),
    )
    .expect("insert the config page");
    assert_eq!(s4, 4);
    // The file service (slot 5) and the page its clients share with it (slot 6), when this boot has
    // a filesystem (milestone 50). GRANT on both, because the progenitor's job with them is to delegate: it
    // narrows the endpoint into the shell and maps the frame into its address space. `a2` carries
    // the rights the endpoint holds, which is also how the progenitor is told there is one at all. `None` is
    // the ordinary case for a run with no RedoxFS disk attached.
    let fs_rights = match program("redoxfs_server").and_then(|redoxfs_server| {
        fs_service::root_directory(fs_service::blk_server_image(), redoxfs_server)
    }) {
        Some((file_ep, file_shared)) => {
            let s5 = crate::sched::thread_control_block_insert_cap(
                tid,
                crate::cap::rendezvous_cap(file_ep, Rights::WRITE.union(Rights::GRANT)),
                Some(5),
            )
            .expect("insert the file service");
            assert_eq!(s5, 5);
            let s6 = crate::sched::thread_control_block_insert_cap(
                tid,
                crate::cap::page_frame_cap(file_shared, Rights::WRITE.union(Rights::GRANT)),
                Some(6),
            )
            .expect("insert the shared file page");
            assert_eq!(s6, 6);
            filesystem_protocol::dir::ALL
        }
        None => 0,
    };
    // The virtio-rng device (slots 7-9, always, even without a disk), when this boot has one
    // (DECISIONS §120's 2026-08-26 amendment). GRANT on all three so system_initializer can
    // delegate them onward to an entropy service it builds, the same shape every other device
    // authority here already takes. `None` exactly as the filesystem pair can be:
    // system_initializer's own probe (`invoke` on an ungranted slot answers `NoSuchSlot`) is what
    // tells it apart from a real one. See [`boot_virtio_rng_device`] for what wiring it costs and
    // why it hands back an un-enabled interrupt.
    //
    // **Explicit slots, not `None`'s first-free** (`thread_control_block_insert_cap`'s own second
    // sense, `notes/abi.md` §4's "the emptiness is load-bearing"), and this is not a style choice:
    // the filesystem pair above is itself conditional, and first-free numbering would silently
    // shift these three down by two on the (ordinary) boot that has virtio-rng but no attached
    // disk. Explicit targets keep slots 7-9 the virtio-rng trio's own regardless of what the
    // filesystem pair did or did not consume. Fixed past the pair's own max reach (slot 6), not
    // slot 5, because the inert-configuration page (slot 4) shifted that pair down by one.
    let virtio_rng = boot_virtio_rng_device();
    if let Some(g) = &virtio_rng {
        let s7 = crate::sched::thread_control_block_insert_cap(
            tid,
            crate::cap::virtio_cap_rights(g.vid, Rights::WRITE.union(Rights::GRANT)),
            Some(7),
        )
        .expect("insert the virtio-rng transport");
        assert_eq!(s7, 7);
        let s8 = crate::sched::thread_control_block_insert_cap(
            tid,
            crate::cap::irq_cap_rights(g.intid, Rights::READ.union(Rights::GRANT)),
            Some(8),
        )
        .expect("insert the virtio-rng interrupt");
        assert_eq!(s8, 8);
        let s9 = crate::sched::thread_control_block_insert_cap(
            tid,
            crate::cap::page_frame_cap(
                g.dma,
                Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
            ),
            Some(9),
        )
        .expect("insert the virtio-rng DMA page");
        assert_eq!(s9, 9);
        // aarch64's GIC has no hart lottery, so its virtio-rng source is enabled here, inline,
        // exactly as the old `spawn_progenitor` did before the thread started. riscv64's is armed at
        // the PLIC in the block after the start below; `x86_64` arms nothing.
        #[cfg(target_arch = "aarch64")]
        crate::arch::irq::enable(g.intid);
    }
    // **Or the CPU's own seed instruction** (slot 16, milestone 595 (provisional)), when there is
    // no virtio-rng: an entropy service the *kernel* built and proved, granted as its request
    // endpoint, the way the file service in slot 5 is. See [`boot_instruction_entropy`] for why the
    // kernel builds this one rather than the progenitor, and why the grant is never both.
    if virtio_rng.is_none()
        && let Some(request) = boot_instruction_entropy()
    {
        let s16 = crate::sched::thread_control_block_insert_cap(
            tid,
            crate::cap::rendezvous_cap(request, Rights::WRITE.union(Rights::GRANT)),
            Some(16),
        )
        .expect("insert the instruction entropy service");
        assert_eq!(s16, 16);
    }
    // **The graphical terminal stack's raw materials** (milestone 600 (provisional); milestone 177 (wire the graphical terminal stack into the real interactive boot)
    // built the stack itself here), when a virtio-gpu is attached: the gpu's confined transport,
    // interrupt and DMA run, and the surface run inside it, in slots 17, 18, 19 and 12. The
    // progenitor builds `gpu_driver` and `display_terminal` from these, exactly as it builds
    // entropy from the rng's trio, and so it holds their endowments and is the supervisor they
    // were born under, which is what swapping `display_terminal` live (milestone 23 (a capability-routed component OS with live replacement)) needs.
    //
    // **Why this is no longer built here.** Milestone 177 spawned the stack kernel-side because a
    // virtio-gpu "needed eleven capability-table slots, one `PageFrame` per DMA page". DECISIONS
    // §102 (a Frame names a run of pages) had already given `PageFrame` a page count, and milestone 142 (a text display good enough that people use it instead of a GUI) made the gpu's region one
    // run and one capability (`display_service::wire_device`), so the premise had expired: the gpu
    // is four capabilities here, and the keyboard three.
    //
    // **Two frame capabilities over one region, deliberately.** The driver gets the whole run and
    // the terminal only the surface after its first page, which is the split
    // `display_service::start_terminal` already makes; nothing lets a holder narrow a run, so the
    // kernel mints both. `None` with no GPU on the bus, or no `gpu_driver` or `display_terminal` in
    // the archive: no device is wired for programs this boot could never run.
    let gpu = if program("gpu_driver").is_some() && program("display_terminal").is_some() {
        display_service::wire_device()
    } else {
        None
    };
    // **And a virtio keyboard, when the graphical boot has one** (slots 20-22), the rng trio's
    // shape exactly. `None` is milestone 192 (a keyboard on real silicon)'s option A, not an absence: the progenitor then builds
    // the UART's `input` driver for the graphical terminal from slots 1 and 2, as it does on a
    // plain boot. Wired only beside a GPU, because a keyboard with no screen has no terminal to
    // type into on this boot.
    let keyboard = if gpu.is_some() && program("keyboard_driver").is_some() {
        keyboard_service::wire_device()
    } else {
        None
    };
    if gpu.is_some() {
        crate::println!(
            "  graphics  : a virtio-gpu and {}; the progenitor builds the terminal stack",
            if keyboard.is_some() {
                "a virtio keyboard"
            } else {
                "no keyboard (the UART is the keystroke source)"
            }
        );
    }
    // **Or a terminal on the screen the firmware left running** (the shell on the firmware screen,
    // milestone 198's rung 1b), when there is no GPU: slots 10 and 11, the terminal's endpoint and
    // its output page. (They were the graphical stack's slots too, until milestone 600
    // (provisional) moved that stack's construction into the progenitor.) `None` on every machine whose
    // console has no screen, which is every boot but a UEFI one today. See
    // [`boot_screen_terminal`].
    let screen = if gpu.is_none() {
        boot_screen_terminal()
    } else {
        None
    };
    if let Some(t) = &screen {
        let s10 = crate::sched::thread_control_block_insert_cap(
            tid,
            crate::cap::rendezvous_cap(t.term, Rights::WRITE.union(Rights::GRANT)),
            Some(10),
        )
        .expect("insert the screen terminal's endpoint");
        assert_eq!(s10, 10);
        let s11 = crate::sched::thread_control_block_insert_cap(
            tid,
            crate::cap::page_frame_cap(
                t.out,
                Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
            ),
            Some(11),
        )
        .expect("insert the screen terminal's output page");
        assert_eq!(s11, 11);
    }
    let insert = |cap, slot: u64, what: &str| {
        let s = crate::sched::thread_control_block_insert_cap(tid, cap, Some(slot))
            .unwrap_or_else(|_| panic!("insert {what}"));
        assert_eq!(s, slot, "{what} landed in the wrong slot");
    };
    if let Some(g) = &gpu {
        insert(
            crate::cap::page_frame_run_cap(
                g.surface,
                display_service::GPU_SURFACE_RUN,
                Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
            ),
            12,
            "the gpu's surface run",
        );
        insert(
            crate::cap::virtio_cap_rights(g.vid, Rights::WRITE.union(Rights::GRANT)),
            17,
            "the gpu transport",
        );
        insert(
            crate::cap::irq_cap_rights(g.intid, Rights::READ.union(Rights::GRANT)),
            18,
            "the gpu interrupt",
        );
        insert(
            crate::cap::page_frame_run_cap(
                g.dma,
                display_service::GPU_DMA_RUN,
                Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
            ),
            19,
            "the gpu's DMA run",
        );
    }
    if let Some(k) = &keyboard {
        insert(
            crate::cap::virtio_cap_rights(k.vid, Rights::WRITE.union(Rights::GRANT)),
            20,
            "the keyboard transport",
        );
        insert(
            crate::cap::irq_cap_rights(k.intid, Rights::READ.union(Rights::GRANT)),
            21,
            "the keyboard interrupt",
        );
        insert(
            crate::cap::page_frame_cap(
                k.dma,
                Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
            ),
            22,
            "the keyboard's DMA page",
        );
    }
    // **The network card** (slots 13-15, milestone 590 (provisional)), when this boot has a
    // virtio-net device on the MMIO bus: the virtio-rng trio's shape exactly, three slots past the
    // graphical stack's own, explicit for the same reason those are (every conditional grant
    // before these would otherwise shift them). GRANT on all three, because the progenitor's only
    // use for them is to delegate them into the `net_stack` it builds and then delete its own
    // copies; see `crates/system_initializer`'s network block. `None` on every real board today and
    // on any run with `NIFE_NET` unset, and the progenitor's probe tells that apart the way it
    // tells a missing virtio-rng apart. See [`boot_virtio_net_device`].
    let virtio_net = boot_virtio_net_device();
    if let Some(g) = &virtio_net {
        let s13 = crate::sched::thread_control_block_insert_cap(
            tid,
            crate::cap::virtio_cap_rights(g.vid, Rights::WRITE.union(Rights::GRANT)),
            Some(13),
        )
        .expect("insert the virtio-net transport");
        assert_eq!(s13, 13);
        let s14 = crate::sched::thread_control_block_insert_cap(
            tid,
            crate::cap::irq_cap_rights(g.intid, Rights::READ.union(Rights::GRANT)),
            Some(14),
        )
        .expect("insert the virtio-net interrupt");
        assert_eq!(s14, 14);
        let s15 = crate::sched::thread_control_block_insert_cap(
            tid,
            crate::cap::page_frame_cap(
                g.dma,
                Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
            ),
            Some(15),
        )
        .expect("insert the virtio-net DMA page");
        assert_eq!(s15, 15);
        // aarch64 arms inline, riscv64 at the PLIC after the start below: the virtio-rng
        // source's split, for its reason.
        #[cfg(target_arch = "aarch64")]
        crate::arch::irq::enable(g.intid);
    }
    // **Say that this boot worked**, if a chooser started it (rung 2b of milestone 198's other
    // half). Here and not a line earlier or later, and the position is the mechanism: the
    // filesystem server above has mounted the installed disk and reported ready, which is as late
    // a criterion as this system can evaluate without a person, and the progenitor has not started,
    // so the disk's one transfer region still has a single user. `install_service::confirm` argues
    // both halves and names the foot gun in the second.
    #[cfg(target_arch = "x86_64")]
    if fs_rights != 0 {
        install_service::confirm();
    }
    crate::sched::configure_thread_control_block(tid, elf.entry(), USER_STACK_TOP, aspace_name)
        .expect("configure");
    // x0 = the boot role (the progenitor has one role and ignores it, but it is passed for the
    // symmetry the old `spawn_progenitor` established); x1 = the archive length; x2 = the file-service rights.
    crate::sched::start_thread_control_block(tid, [PROGENITOR_ROLE, initrd_len, fs_rights])
        .expect("start");

    // Arm the interrupt chain so the input driver's keystrokes flow. aarch64 already did its arming
    // inline above (the GIC's UART line before the thread was built, its virtio-rng source as the
    // caps were inserted), because the GIC has no boot-hart lottery. What is left here is the
    // hardware that must wait until the driver is running:
    //
    // riscv64: the source at the PLIC and supervisor external interrupts in `sie`. The input driver
    // arms the NS16550's own RX interrupt (its IER) when it starts, and re-arms the PLIC source
    // through its Irq cap's ACK.
    //
    // `x86_64`: nothing. There is no userspace input driver to feed until DECISIONS §149 chooses a
    // console, so arming COM1's line would deliver keystrokes to nobody.
    #[cfg(target_arch = "riscv64")]
    {
        crate::drivers::plic::enable(uart_irq, crate::arch::irq::boot_s_context());
        // The virtio-rng device's own source, pinned to the same boot-hart context for the same
        // reason (`notes/harts-and-pes.md`'s hart lottery; see [`VirtioBootGrant::intid`]'s own doc).
        if let Some(g) = &virtio_rng {
            crate::drivers::plic::enable(g.intid, crate::arch::irq::boot_s_context());
        }
        // The NIC's, for the same reason (milestone 590 (provisional)).
        if let Some(g) = &virtio_net {
            crate::drivers::plic::enable(g.intid, crate::arch::irq::boot_s_context());
        }
        crate::arch::exceptions::enable_external();
    }
    #[cfg(target_arch = "x86_64")]
    let _ = (&virtio_rng, &virtio_net);
    Ok(tid)
}

/// Bringing the console driver up in userspace, and wiring a client to it.
///
/// **This is the milestone-8 payload.** It creates the shared machinery (two endpoints and a
/// shared page), spawns the console *server* as a user process that owns the UART, and returns
/// what a client needs to reach it. The server binary and the client binary are the *same ELF*,
/// told apart by the argument in `x0`.
// The milestone tour is the only consumer, so this is dead in exactly the configurations that
// have no tour: a test build, and the two alternate boot modes. The allow sits on the module
// because the module is one wiring, not a bag of independent items.
#[cfg_attr(any(test, feature = "shell", feature = "bench"), allow(dead_code))]
pub mod console_service;

/// Bringing the virtio block driver up in userspace.
///
/// **Milestone 9's headline.** The kernel enumerates the bus (kernel/src/virtio.rs) to find the
/// block device, then hands a userspace driver everything it needs and nothing it does not: the
/// device's registers, a DMA page, an interrupt, and an endpoint to report what it read. The
/// kernel does not touch the device.
#[cfg_attr(not(test), allow(dead_code))] // the tour spawns it; the tests drive it
pub mod virtio_service;

/// **A service's memory, remembered so a finished test can hand it back.** The bookkeeping half of
/// DECISIONS §16 object revocation, applied to the services the test boot builds: without it a boot
/// that runs many service-shaped tests runs out of frames, and does so in whichever innocent test
/// happens to allocate next. See the module note and notes/frames.md.
#[cfg_attr(not(test), allow(dead_code))] // the tests are the callers; the tour never tears down
pub mod holding;

/// **The RedoxFS filesystem service** (milestone 32 phase 2): three confined processes and the
/// endpoints and shared pages that wire them, spawned by the test that proves the stack end to end.
///
/// ```text
///   disk ──virtio──► block server ──blk IPC──► FS server ──file IPC──► client ──► report to kernel
/// ```
///
/// The kernel builds the wiring and hands each process exactly its world (a `Spawn` literal each);
/// it never sees a filesystem operation, an opcode, or a byte of file data. The FS server owns
/// RedoxFS and its own heap; the block server owns the DMA confinement; the client holds only a
/// directory capability. This is the same shape as `virtio_service` and the console, one level up.
///
/// The service drives the **second** mmio block disk (the RedoxFS image); the first is the nifefs
/// disk the phase-1 driver tests use. `None` if there is no such disk attached to this run.
#[cfg_attr(not(test), allow(dead_code))] // spawned only by the phase-2 test
pub mod fs_service;

/// **The block-device roster and the disk surveyor** (milestone 57, notes/block-devices.md).
///
/// Two authorities that every other operating system hands out as one: a **read-only mapping**
/// listing what block devices exist, and a **block-service endpoint** for exactly one of them. A
/// program with the first can see the machine's disks and open none of them; a program with the
/// second was handed one disk and has no way to name a second.
///
/// The kernel's part is small and stops early: scan the buses, write the page, confine one device
/// under a block server, spawn. It never reads a partition table. Every byte of GPT judgement is in
/// `crates/globally_unique_identifier_partition_table`, whose tests run on the host against tables
/// `sgdisk` and macOS `diskutil` wrote.
///
/// Arch-neutral, like the clock and entropy wirings: one portable binary over one host-tested
/// contract, so **both ISAs run literally the same test** (DECISIONS §19).
#[cfg_attr(not(test), allow(dead_code))] // the tests are its only caller today
pub mod disk_service;

/// **Reading a real disk's partition table, and the difference between listing and holding**
/// (milestone 57).
///
/// The first test is the half of the milestone that is not optional: which blocks of a device are a
/// filesystem is written in the partition table and nowhere else, so this reads one off a virtio-blk
/// device. The table was written by `sgdisk`, in C++, by people who never heard of this project;
/// that provenance is what makes the parse worth asserting.
///
/// The second is the negative control the first would be weaker without. The roster is a read-only
/// mapping, so a program that knows exactly where it is still cannot add a device to it or turn an
/// entry into a handle. `lsblk` plus `parted` cannot make that claim.
#[cfg(all(test, initrd))]
mod disk_tests;

/// **Play an application printing to a display terminal**: put `text` in its output page and
/// `OP_WRITE` it.
///
/// Shared by both of the terminal's wirings (the whole scanout, and a compositor window) because the
/// terminal contract does not know which one it is in: an `OP_WRITE` is an `OP_WRITE`. Returns when
/// the reply arrives, which the contract says means the bytes are on the console's side, so a test
/// needs no polling and no sleep between writes.
#[cfg_attr(not(test), allow(dead_code))] // the milestone-29 tests are the callers
fn term_print(out: u64, ep: crate::sched::RendezvousId, text: &[u8]) {
    assert!(
        text.len() <= FRAME_SIZE as usize,
        "an OP_WRITE past its output page",
    );
    let base = mmu::phys_to_virt(out);
    for (i, &b) in text.iter().enumerate() {
        // SAFETY: inside the output frame this kernel allocated and shares with the terminal.
        unsafe { core::ptr::write_volatile((base + i as u64) as *mut u8, b) };
    }
    // The bytes must be visible to the terminal before the request that names them.
    //
    // PAIR: no acquire fence, and none is needed. The terminal is blocked in `recv_cap` and the
    // `ipc_call` below is what wakes it, so the kernel's release of the `IPC_TABLES` lock and the
    // terminal's acquire of it are the pair (`spin::Mutex` locks `Acquire` and unlocks `Release`).
    // Redundant, kept: it is one `dmb` on a path that prints a line, and the contract does not
    // forbid a terminal that polls its page instead of blocking. See notes/memory-ordering.md.
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    let w0 = line_editor::proto::req(line_editor::proto::OP_WRITE, text.len() as u64);
    let r = crate::sched::ipc_call(ep, [w0, 0]);
    assert_eq!(
        r[0],
        text.len() as u64,
        "the terminal consumed {} of {} bytes",
        r[0],
        text.len(),
    );
}

/// **The display service** (milestone 29, the display ladder's rung one): a confined virtio-gpu
/// driver and a client that draws, wired by the kernel and then left alone.
///
/// ```text
///   virtio-gpu ──virtio (PCIe, behind the IOMMU)──► gpu_driver ──display IPC──► painter
///        │                                              │                             │
///        └──── DMA: the whole region ───────────────────►│                             │
///                                    the surface (pages 1..) ─────── shared ──────────┘
/// ```
///
/// The kernel's part is the same as every other service here: build the wiring, hand each process a
/// `Spawn` literal, and know nothing about what they do. It never sees a virtio-gpu command, a
/// pixel, or a rectangle. What is new is the **size** of the DMA region, and that is the whole
/// memory story: a framebuffer does not fit in the single page the disk and NIC drivers get, so the
/// region is `1 + graphics_protocol::SURFACE_PAGE_FRAMES` **contiguous** frames, page 0 for the rings and the
/// control buffers and the rest for the surface. Registering the whole run as the driver's DMA region
/// is what keeps the framebuffer inside the grant: the shadow-ring validator bounds every descriptor
/// to it, and `iommu::confine` maps exactly it, so the device can reach the pixels and nothing else.
/// The block server already took two pages this way (milestone 32); this is the same move, wider.
///
/// The client maps only the surface frames. It never sees page 0, so it cannot touch a descriptor
/// ring, and it holds no `Virtio` capability, no interrupt, and no physical address. See
/// notes/framebuffer-contract.md.
#[cfg_attr(not(test), allow(dead_code))] // spawned only by the milestone-29 test
pub mod display_service;

/// **The compositor: one screen, several mutually distrusting clients** (milestone 33, the display
/// ladder's rung two).
///
/// ```text
///   display (or a kernel stand-in) ──gfx FLUSH(damage)──► compositor ◄──one doorbell──── window clients
///                                            the scanout, shared ──┘  │                 (a surface each)
///                                                                     └─► one input endpoint per focusable
/// ```
///
/// The kernel's part is what it always is: allocate the frames, mint the endpoints, hand each process
/// a `Spawn` literal, and know nothing about what they do. It never sees a pixel, a window, or a
/// damage rectangle. What is worth reading here is the **shape of the grants**, because the isolation
/// this rung exists to prove is a property of exactly that shape:
///
/// - every client's control page and surface are its own frames, mapped **at the same virtual
///   addresses** in every client. Two clients' surfaces are the same address in different address
///   spaces, so "my neighbour's surface" is not somewhere a client can reach by guessing;
/// - the clients' frames are allocated as **one contiguous run**, deliberately, so that the page just
///   past a client's grant really is its neighbour's memory. That makes the attack in
///   `a_client_holds_no_capability_for_its_neighbours_pixels_or_the_screen` a fair one: the attacker is
///   handed the exact address, the bytes it wants are physically adjacent, and the mapping is the only
///   thing in its way;
/// - the screen and the window list are mapped **read-only** and **only** into a client granted them.
///   That mapping is the screenshot capability and the enumeration capability; there is no verb for
///   either, and a client without the mapping has nothing to ask and nowhere to look.
#[cfg_attr(not(test), allow(dead_code))] // spawned only by the milestone-33 tests
pub mod compositor_service;

/// **The keyboard service** (milestone 29's input): a confined userspace virtio-input driver that
/// turns key events into the bytes a terminal understands, and publishes them where the compositor
/// reads them.
///
/// ```text
///   virtio-input ──virtio (PCIe, IOMMU)──► keyboard_driver ──the input ring──► whoever maps it
///                                           └──doorbell COMMIT──► "look at the surfaces"
/// ```
///
/// The grant shape is the whole security argument and it is worth reading beside
/// `compositor_service`: the driver gets the device, its interrupt, its DMA page, the doorbell, and
/// **the ring page**. It does not get any client's endpoint, so it cannot choose who receives what
/// it types; that is focus, and focus is the compositor's decision expressed as which of the input
/// capabilities *it* holds it uses (DECISIONS §33). And the ring is what makes typing possible at
/// all: the doorbell every client holds is content-free, so a client that rang it forever could not
/// produce a single character.
///
/// In the test below the **kernel** plays the compositor, which is the same substitution three of the
/// four rung-two tests make: it holds the doorbell and the ring, so the bytes a real keyboard
/// produced are a value it can read and compare rather than a picture it has to infer.
// The driver is spawned only by the milestone-29 test; the boot calls `wire_device` alone and the
// progenitor builds the driver (milestone 600 (provisional)).
#[cfg_attr(not(test), allow(dead_code))]
pub mod keyboard_service;

/// **The clock service** (milestone 51 lane A, DECISIONS §43): the RTC's registers, the wall
/// clock's offset, and the propose endpoint, in one confined userspace process.
///
/// The kernel's whole part in wall-clock time is here and it is small: find the RTC in the device
/// tree (by `compatible`, `memory::rtc_region`), allocate one frame for the clock page, and hand
/// the service the registers, the page read/write, and an endpoint. It does not read the clock, does
/// not know what time it is, and has no notion of an offset. Everything after the spawn is
/// userspace agreeing with userspace over `clock_protocol`.
///
/// Arch-neutral, like the display and compositor wiring: the component is one portable binary
/// carrying both RTC drivers, and the *machine* says which one it has, so **both ISAs run literally
/// the same test** (DECISIONS §19).
// The interactive boot calls `start` on both ISAs since milestone 51's wiring lane; the rest of the
// module (the propose helper, the kernel-side page reader) is still the tests' alone.
#[cfg_attr(not(test), allow(dead_code))]
pub mod clock_service;

/// **The clock page the interactive boot hands the progenitor**, and the one place both ISAs agree on what a
/// machine with no clock looks like (milestone 51's wiring; `boot_progenitor`).
///
/// The grant is **unconditional**, and that is the design rather than an oversight. A zeroed page
/// reads as `clock_protocol::state::UNKNOWN` (`a_zeroed_page_reads_as_unknown`), so a boot with no
/// `clock` program in its initrd hands the progenitor a page that honestly says "the machine has no clock it
/// believes" instead of no page at all. That keeps the slot numbering the same on every boot, which
/// matters more than it sounds: the progenitor's capability table is read positionally, and a capability whose *slot*
/// depends on what the machine turned out to have is a wiring nobody can check by reading.
///
/// It is also the DECISIONS §43 split, delivered: the progenitor gets `READ` on a frame. Nothing on this path
/// can hand a child the writable mapping that would let it set the time, because the progenitor never had one.
fn boot_clock_page() -> u64 {
    match program("clock") {
        Some(image) => {
            let wiring = clock_service::start(image);
            // The service publishes the RTC reading and *then* announces, with a blocking send. It
            // does not need to be drained for the page to be right, but an undrained announcement
            // parks the service inside it forever, so it would never serve a proposal. One thread
            // whose whole life is that receive costs nothing and leaves the propose endpoint live.
            let report = wiring.report;
            let _ = crate::sched::spawn(move || {
                crate::sched::ipc_recv(report);
                crate::sched::exit();
            });
            wiring.page_phys
        }
        // No `clock` program packed: allocate the page anyway and leave it zeroed. This is the
        // honest unknown clock, and it is the same state `date`'s test allocates deliberately.
        None => crate::memory::alloc_zeroed()
            .expect("no frame for the clock page")
            .addr(),
    }
}

/// The confined transport, the completion interrupt, and the DMA page's physical base, for a
/// virtio device (the rng, or since milestone 590 (provisional) the NIC) this kernel discovered and
/// wired at boot. Returned to the caller rather than
/// stored, because `crate::sched::grant`'s next-free-slot placement means the caller decides
/// exactly where these land relative to whatever else it has already granted.
struct VirtioBootGrant {
    /// The `Virtio` capability's id (`crate::virtio::register`'s return value).
    vid: usize,
    /// The device's completion interrupt. **Routed** (`crate::sched::bind_irq`) but not yet
    /// **enabled**: the two boards enable an interrupt source differently (aarch64's GIC inline,
    /// riscv64's PLIC pinned to the boot hart's own context via `boot_s_context`, not the
    /// hart-spreading `crate::arch::irq::enable` uses -- `notes/harts-and-pes.md`'s hart lottery
    /// is why, the same reasoning `uart_irq`'s identical two-step split in `boot_progenitor`
    /// already follows), so the caller does that part itself, the same place it already enables
    /// `uart_irq`.
    intid: u32,
    /// The DMA region's physical base. `entropy.rs` (and `net_stack`) needs this as a plain value
    /// (it builds virtio
    /// ring descriptors, which are physical-address-based by the spec, not a fact any capability
    /// exposes), and there is no fourth `START` argument word to carry it across the kernel/progenitor
    /// boundary (`start_thread_control_block`'s own `[u64; 3]`, already spent on
    /// `role`/`initrd_len`/`fs_rights`). So it travels the way the page's *contents* already do: written
    /// into the page itself at [`VIRTIO_DMA_PHYS_OFFSET`], which the progenitor reads back out once,
    /// after mapping the granted frame briefly, and relays to entropy's own `arg1` exactly the way
    /// it already relays `fs_rights`.
    dma: u64,
}

/// Where [`boot_virtio_mmio_device`] (and, since milestone 600 (provisional), the gpu and keyboard
/// wiring in [`display_service`] and [`keyboard_service`]) writes a DMA region's own physical base,
/// inside that same region. Safe for every device granted this way for one reason: no driver's
/// layout reaches the first page's tail. Entropy's ring
/// (`components/src/entropy.rs`'s `Q_DESC`/`Q_AVAIL`/`Q_USED`) and its one pool buffer (`POOL_OFF`
/// 0x400, `POOL_LEN` 256 bytes) end at byte 0x500; `net_stack`'s two rings and four frame buffers
/// (`components/src/net_transport.rs`'s `BUF_BASE` 0x400 plus four `BUF`s of 0x2C0) end at 0xF00.
/// This sits in the last eight bytes, past all of them, so a future widening of any has room to
/// move without colliding. The value is `abi::virtio::DMA_PHYS_OFFSET`, which every reader shares.
pub(crate) const VIRTIO_DMA_PHYS_OFFSET: u64 = abi::virtio::DMA_PHYS_OFFSET;
const _: () = assert!(VIRTIO_DMA_PHYS_OFFSET + 8 <= FRAME_SIZE);

/// **Write a DMA region's own physical base into its first page**, at [`VIRTIO_DMA_PHYS_OFFSET`].
/// The one place every boot virtio device's region learns where it is; see that constant.
pub(crate) fn write_dma_phys(dma: u64) {
    // SAFETY: `dma` is a fresh frame the caller just allocated, direct-mapped and owned by nobody
    // else yet, and `VIRTIO_DMA_PHYS_OFFSET + 8` is inside `FRAME_SIZE` (the assertion above), so
    // the write stays in the frame.
    unsafe {
        core::ptr::write_unaligned(
            (mmu::phys_to_virt(dma) as *mut u8)
                .add(VIRTIO_DMA_PHYS_OFFSET as usize)
                .cast::<u64>(),
            dma,
        );
    }
}

/// **Discover and wire a virtio-rng device on the MMIO bus, for the interactive boot's own use**
/// (DECISIONS §120's 2026-08-26 amendment: "grant the QEMU-only virtio-rng stopgap"). `None` on a
/// boot with no such device: real hardware (milestone 55's actual target has no virtio-rng at all,
/// §120's own text), or a run with `NIFE_RNG` unset. The whole chain past this point treats that
/// exactly as "this boot has no filesystem" is already treated by [`boot_progenitor`]: an absence
/// the caller can act on, not a failure.
///
/// **Only the MMIO transport**, unlike `entropy_service::start`'s own test-harness wiring, which
/// also offers PCIe: a first cut scoped to what an interactive boot actually needs, on the same
/// "a minimal device surface for the boot a person actually meets" posture already named for the
/// GPU/keyboard/NVMe flags (`components/src/login.rs`'s own BUGS, before this amendment). Widening to
/// PCIe (behind the IOMMU) is real follow-on, not invented here.
///
/// Mirrors `kernel::user::entropy_service::start`'s own kernel-side setup (device discovery, a
/// zeroed DMA frame, the interrupt route, `crate::virtio::register`) up to the point that function
/// spawns the service itself: this one hands the three capabilities back for the **caller** to
/// grant and delegate, because on the interactive boot the caller is the progenitor, not the kernel, and the progenitor
/// is the one that builds the entropy service: `crates/system_initializer`'s own ELF loader, the
/// tree's only one (milestone 96), and that crate's own header says why a second loader would be
/// the wrong shape.
fn boot_virtio_rng_device() -> Option<VirtioBootGrant> {
    boot_virtio_mmio_device(crate::virtio::find_entropy_device()?)
}

/// **An entropy service on the CPU's own seed instruction, for a boot with no virtio-rng**
/// (milestone 595 (provisional); promoted from the proposal
/// `the-x86-64-progenitor-serves-entropy-from-rdseed`). The request endpoint, which
/// [`boot_progenitor`] grants at slot 16, or `None`, and a `None` is said on the console.
///
/// **The kernel confirms the instruction exists, not the progenitor**, which is the question that
/// proposal left open. Three things decided it, and none of them is effort:
///
/// - It is what the tree already does. `entropy_service::is_instruction_backend_available` reads
///   the feature bit from `arch::isa`'s boot record, and `components/src/entropy.rs` says outright
///   that it trusts its spawner's choice of mode. The installer (`install_service`) takes the same
///   service the same way on the booted `x86_64` path.
/// - It is the only answer that works on aarch64 too. `ID_AA64ISAR0_EL1` is not readable at EL0,
///   so a progenitor that ran `CPUID` itself would be an `x86_64` special case with no aarch64 twin
///   (DECISIONS §19). Here the function is arch-neutral: aarch64 with `FEAT_RNG` and no
///   `NIFE_RNG` takes this path as well, and riscv64 answers `None` from the same predicate.
/// - Detection stays in `kernel/src/arch/` (AGENTS.md's rule 1), and the progenitor stays free of
///   `cfg(target_arch)`.
///
/// **The kernel builds the service rather than telling the progenitor to**, because a fact with
/// no capability has no slot to travel in: the progenitor's three `START` words are spent
/// (`system_initializer::BootEndowment::virtio_rng`'s doc), and a flag in a page
/// would be a second format for one bit. A built service is a capability, and a probe on its slot
/// is how the progenitor already tells every optional grant from an absent one. `fs_ep` is the
/// precedent: the kernel wires the file service before the progenitor exists and grants its
/// endpoint.
///
/// **A refusal, not weaker bytes.** No instruction, or a first draw of all zeros
/// (`entropy_protocol::readiness`), and this returns `None` with a sentence saying so. There is no
/// fallback to `RDRAND`/`RNDR` (DRBG output, which `entropy.rs`'s header refuses) and no software
/// generator. The bytes served are the instruction's own, unmixed and not health-tested after the
/// first draw: option A of DECISIONS §137 (a hardware TRNG with no published health-test claim),
/// which is what every other backend here does. Choosing B or C is calef's.
///
/// **`ready` may already be taken.** `entropy_service::ensure` hands the readiness report to
/// whoever wired the service first, and on `x86_64` that can be the installer earlier in this boot.
/// The installer does not read the verdict either, so a condemned service would answer
/// `NO_ENTROPY` to every request rather than serve anything; the progenitor's own first draw (the
/// login password) is then what fails, and it builds no login stack.
fn boot_instruction_entropy() -> Option<crate::sched::RendezvousId> {
    let image = program("entropy")?;
    let Some(w) = entropy_service::ensure(image, entropy_service::Bus::Instruction) else {
        crate::println!(
            "  entropy     : NONE. No virtio-rng device, and this CPU has no seed instruction \
             (RDSEED, RNDRRS), so nothing at the prompt can draw random bytes and there is no login."
        );
        return None;
    };
    if let Some(report) = w.wait_for_ready()
        && report[0] != entropy_protocol::READY
    {
        crate::println!(
            "  entropy     : REFUSED. The seed instruction's first draw was all zeros ({:#x}), so \
             the service is condemned for this boot; nothing at the prompt can draw random bytes.",
            report[0]
        );
        return None;
    }
    crate::println!("  entropy     : the CPU's seed instruction; the progenitor serves it");
    Some(w.request)
}

/// **The same for the network card** (milestone 590 (provisional), the booted system starts its
/// network stack; promoted from the proposal `the-booted-system-has-no-network`). `None` on a boot
/// with no virtio-net device on the MMIO bus: every real board today, and every QEMU run with
/// `NIFE_NET` unset. The progenitor builds `net_stack` from these three and nothing else, exactly
/// as it builds entropy from the rng's three.
///
/// **The MMIO NIC, not the PCIe one**, for [`boot_virtio_rng_device`]'s reason, and with a cost
/// that one does not carry: the MMIO NIC has no IOMMU in front of it, so the confinement of this
/// device's DMA is the transport's shadow-ring validator alone, not the SMMU of DECISIONS §20
/// (IOMMU-backed DMA isolation: one seam, two arch drivers). The runners attach both NICs under
/// `NIFE_NET`; the PCIe one sits unclaimed on this boot. See
/// milestone 590's block for why that is recorded rather than fixed here.
fn boot_virtio_net_device() -> Option<VirtioBootGrant> {
    boot_virtio_mmio_device(crate::virtio::find_net_device()?)
}

/// The shared body of [`boot_virtio_rng_device`] and [`boot_virtio_net_device`]: a zeroed DMA frame
/// with its own physical base written at [`VIRTIO_DMA_PHYS_OFFSET`], the interrupt routed but not
/// enabled, and the transport registered with the kernel, confined to that one frame.
fn boot_virtio_mmio_device(d: crate::virtio::VirtioMmioDevice) -> Option<VirtioBootGrant> {
    // Zeroed first, so no stale descriptor or buffer content is visible to the device or to the
    // driver's own first read.
    let dma = crate::memory::alloc_contiguous_zeroed(1)
        .expect("no DMA frame for a boot virtio device")
        .addr();
    // The physical base is written into the tail of the same page, at an offset neither driver's
    // ring-and-buffer layout reaches (see [`VIRTIO_DMA_PHYS_OFFSET`]'s own doc).
    write_dma_phys(dma);
    // Routed, not yet enabled; see [`VirtioBootGrant::intid`]'s own doc for why enabling is the
    // caller's job.
    crate::sched::bind_irq(d.intid, crate::sched::create_rendezvous());
    let vid = crate::virtio::register(
        crate::virtio::Transport::Mmio {
            mmio_phys: d.mmio_phys,
        },
        dma,
        FRAME_SIZE,
        None,
    );
    Some(VirtioBootGrant {
        vid,
        intid: d.intid,
        dma,
    })
}

/// **The inert-configuration page the interactive boot hands the progenitor** (milestone 47's
/// environment-variable fork, DECISIONS §111; `boot_progenitor`).
/// [`boot_clock_page`]'s twin, minus the service: nothing here runs, so there is nothing to spawn
/// and nothing to wait for a report from. The page is assembled once, into a frame nothing else
/// can see, and only then handed to the progenitor; see `environment_protocol`'s own docs for why that
/// ordering needs no seqlock.
///
/// The grant is **unconditional**, [`boot_clock_page`]'s own reason: a fixed slot on every boot,
/// whether or not anything downstream ever declares wanting the page, is what lets the progenitor's
/// capability table stay positional. The values are the conservative universal defaults this
/// tree's kernel test harness for `std` programs already uses
/// (`kernel/src/user/std_service.rs`): "nothing configured this program's locale or terminal, so
/// tell it the least assuming thing" is the honest baseline, the same posture `boot_clock_page`
/// takes for a machine with no RTC. There is no shell-held default config set yet to pass instead
/// (the "inheritance with visibility" shape design/roadmap/47-navigation-and-naming.md names);
/// this is the fixed default until one exists.
fn boot_config_page() -> u64 {
    let bytes = environment_protocol::PageBuilder::new()
        .tz("UTC")
        .expect("UTC is not a recognized environment_protocol::domain::KNOWN_TZ member")
        .lang("C")
        .expect("C is not a recognized environment_protocol::domain::KNOWN_LANG member")
        .term("dumb")
        .expect("dumb is not a recognized environment_protocol::domain::KNOWN_TERM member")
        .build();
    // Zeroed before the assembled bytes are written, so nothing left behind by a previous
    // occupant of this physical page is visible through the reserved tail past `PAGE_BYTES`
    // (`ConfigPage` only ever reads the first `PAGE_BYTES`, but a frame's contents are otherwise
    // unspecified until written; the same shape `std_service::start_on` uses).
    let phys = crate::memory::alloc_zeroed()
        .expect("no frame for the config page")
        .addr();
    // SAFETY: `phys` names that frame, direct-mapped and owned by nobody else yet, and `bytes` is
    // `PAGE_BYTES` long, far under `FRAME_SIZE`, so the copy does not run past the frame.
    unsafe {
        core::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            mmu::phys_to_virt(phys) as *mut u8,
            bytes.len(),
        );
    }
    phys
}

/// **The shell's terminal on the screen the firmware left running** (the shell on the firmware
/// screen, milestone 198's rung 1b; `design/roadmap/` has its block).
///
/// Milestone 243 (a machine with no serial port) put the *kernel's* boot tour on a UEFI machine's
/// framebuffer. Since milestone 299 (the serial console becomes a userspace driver) the console is
/// a userspace process that writes COM1, so on a PC with no serial port the tour scrolled past and
/// the prompt appeared nowhere. This puts `display_terminal` on that same screen, served by
/// `framebuffer_driver`, and returns what the progenitor needs to hand the console server so that
/// it writes every byte to the screen as well as to the UART.
///
/// **The order is the handover, and it is the point of the function.** The programs are found
/// first, so a build that lacks one leaves the kernel painting rather than a blank screen. Then
/// [`crate::console::yield_screen`] clears the screen and stops the kernel's `print!` from painting
/// it, under the console lock, and only then is the driver spawned. So there is no moment with two
/// painters, and after this the kernel's own lines (the progenitor's exit, a user fault report) go
/// to the UART alone, including the two this function prints: a line printed on the screen just
/// before the yield would be cleared by it before anybody could read it.
///
/// If the wiring refuses after the yield (a screen too large to map, [`display_service`]'s
/// `MAX_APERTURE_PAGES`), the screen is left blank rather than handed back: the boot goes on over
/// the UART exactly as a machine with no screen does. Recorded here rather than papered over with a
/// second handover path, because the refusal is a bound no screen in the fleet is near.
///
/// Readiness is drained here, the idiom the kernel-built virtio-gpu stack used before milestone 600
/// (provisional) moved it into the progenitor: when this returns, the driver and the terminal are
/// running and the terminal has painted its blank grid.
///
/// Arch-neutral, and `None` on aarch64 and riscv64 today only because nothing there tells the
/// console about a screen: milestone 157 (real display output on the board), the U-Boot
/// `simple-framebuffer` discovery, is what would, and then this function needs no change. **Name
/// provisional.**
fn boot_screen_terminal() -> Option<display_service::TerminalWiring> {
    let driver = program("framebuffer_driver")?;
    let terminal = program("display_terminal")?;
    // **A screen this kernel owns the memory of is not handed away** (milestone 243).
    //
    // The handover's whole shape assumes a UEFI aperture: a BAR on a display adapter, memory no
    // part of this kernel is otherwise in, whose physical range `display_service` maps into a
    // userspace driver. `ramfb` broke that assumption on the two `virt` boards, where the
    // framebuffer is `kernel/src/screen.rs`'s own `.bss` and mapping its range into a driver would
    // hand a userspace process a window onto kernel statics.
    //
    // Checked before the yield rather than after, so a refusal leaves the kernel still painting
    // rather than leaving a screen cleared and unclaimed. It is a range test rather than a flag, so
    // milestone 157's U-Boot aperture (outside the kernel image, like the UEFI one) passes without
    // anybody having to remember to set anything.
    let screen = crate::console::peek_screen()?;
    if crate::screen::is_kernel_memory(&screen) {
        crate::println!(
            "  screen    : kept by the kernel; its framebuffer is kernel memory, not an aperture"
        );
        return None;
    }
    let screen = crate::console::yield_screen()?;
    crate::println!(
        "  screen    : handed to a userspace terminal; the kernel writes the UART alone"
    );
    let w = display_service::start_screen_terminal(driver, terminal, screen)?;
    let [tag, geometry, ..] = crate::sched::ipc_recv(w.driver_report);
    assert_eq!(
        tag,
        graphics_protocol::status::UP,
        "the framebuffer driver did not come up ({tag:#x})",
    );
    let [tag, cells, ..] = crate::sched::ipc_recv(w.term_report);
    assert_eq!(
        tag,
        video_terminal::status::TERM_UP,
        "the display terminal did not come up ({tag:#x})",
    );
    crate::println!(
        "  screen    : {}x{} pixels of it served by framebuffer_driver, a {}x{} terminal on it",
        geometry & 0xffff_ffff,
        geometry >> 32,
        cells & 0xffff_ffff,
        cells >> 32,
    );
    Some(w)
}

/// **Wall-clock time** (milestone 51 lane A, DECISIONS §43).
///
/// Arch-neutral on purpose: one portable binary carrying both RTC drivers, one host-tested
/// contract, and the machine's own device tree choosing between them, so **both ISAs run literally
/// these tests** rather than two copies that can drift (DECISIONS §19, parity is a gate).
#[cfg(all(test, initrd))]
mod clock_tests;

/// **`date`** (milestone 51; DECISIONS §43, notes/date.md).
///
/// The command that makes the wall clock visible to a person, and the first thing in the tree that
/// exercises `crates/calendar` against a clock the machine actually read. Arch-neutral like the
/// service it reads from: one portable binary over one host-tested contract, so **both ISAs run
/// literally these tests** (DECISIONS §19).
///
/// What these prove that nothing else does: the printed text, parsed back, names the same instant
/// the kernel computes independently from the page; and **an unknown clock produces a sentence
/// rather than 1970 or a panic**, which DECISIONS §43 listed as proven by construction only. It is
/// proven in the guest now, on a board whose RTC works, because the page is the thing under test
/// and a frame nobody has published to is an honest unknown clock.
#[cfg(all(test, initrd))]
mod date_tests;

/// **`printenv`** (milestone 47's environment-variable fork, DECISIONS §111, notes/env-config.md).
///
/// `date`'s own proof, one manifest field over: the program that makes the inert-configuration
/// page visible to a person, and the first spawnable, shell-facing program to declare
/// [`grant_plan::Manifest::config`]. Arch-neutral like the page it reads: one portable binary over
/// one host-tested contract (`environment_protocol`), so **both ISAs run literally these tests**
/// (DECISIONS §19).
///
/// What these prove that nothing else does: a real spawned process, given the real capability
/// `crates/system_initializer`'s wiring would grant, reads the three validated keys back
/// unchanged; a page nobody assembled (the zeroed frame the allocator hands out, `date_tests`'s
/// own unpublished-clock shape) reads as no configuration rather than three empty strings; and a
/// process granted no capability at all answers without touching the page, the same
/// without-touching-memory-it-does-not-hold property `date`'s clock probe already proves.
#[cfg(all(test, initrd))]
mod printenv_tests;

/// **`uuid`** (milestone 111, `components/src/uuid.rs`, notes/entropy.md).
///
/// `printenv`'s proof one manifest field over, and the field is
/// [`grant_plan::Manifest::entropy`]: the first spawnable, shell-facing program that needs random
/// bytes, which before this milestone was authority the *system* could grant and a person could
/// not reach. Arch-neutral, so **both ISAs run literally this test** (DECISIONS §19).
///
/// What it proves that nothing else does: a real spawned process holding an **empty** entropy slot
/// prints no identifier at all, says why, and ends normally. That is the direction the milestone
/// rests on, because randomness is the one authority whose use leaves no trace in what a program
/// does; only removing the capability distinguishes a program that drew bytes from one that
/// invented them. The endowed direction is proven at the real prompt by `script/swish-check`,
/// through the real `crates/system_initializer`, for the reason this module's own
/// `spawn_uuid_holding_no_entropy` records: `Spawn::grants` fills a capability table from zero and
/// cannot place one at the slot a manifest names.
#[cfg(all(test, initrd))]
mod uuid_tests;

/// **The entropy service** (milestone 56, DECISIONS §44): a virtio-rng device, its DMA page, its
/// interrupt, and the request endpoint clients hold, in one confined userspace process.
///
/// The kernel's whole part in randomness is here and it is smaller than the clock's: find an RNG on
/// whichever bus the caller named, confine it to one DMA page, and hand the service the transport,
/// the interrupt, and two endpoints. **The kernel never reads the device and holds no entropy of
/// its own.** Everything after the spawn is userspace agreeing with userspace over
/// `entropy_protocol`.
///
/// The authority split is the point, and it is one sentence: the service holds the device; a client
/// holds an endpoint that means *"you may obtain randomness"*. Those are different powers, and only
/// the second one is safe to hand around. A client cannot program the queue, cannot map the page
/// the device writes into, and cannot ask for anything the service did not ask on its behalf.
///
/// Arch-neutral: one portable binary, both transports, both ISAs (DECISIONS §19).
#[cfg_attr(not(test), allow(dead_code))]
// the tests, std_service, the installer and boot_progenitor are its callers
pub mod entropy_service;

/// **The EL0 NVMe block server's wiring** (milestone 261; DECISIONS §86's option 2a).
///
/// The kernel keeps the admin plane, which is the authority to say where a queue lives, and hands
/// a process the doorbell page and the data plane's pages of one confined DMA region. What it is
/// granted and what it is refused is written out in that module's own header, in the shape
/// milestone 159's TRNG driver established, because the confinement is the claim and the driver is
/// only what exercises it.
#[cfg_attr(not(test), allow(dead_code))] // the tests are its callers
pub mod non_volatile_memory_express_service;

/// **The offer a booted stick makes** (milestone 198 (a package manager, and the trivial install
/// that makes a second customer possible), rung 2a): ask whether to put this system on the
/// machine's own disk, and wire the two confined programs that do it. Boot policy only; nothing in
/// it holds a disk.
///
/// **`x86_64` only, and the gap is the loader's rather than this module's.** It needs two things
/// the other two architectures do not have: a copy of the file this machine booted from
/// (`memory::boot_file_region`, which is `None` wherever `uefi_loader` cannot hand over a second
/// module, and a device-tree handoff has one initrd slot in `/chosen` and no second one), and a
/// console it can read a line back from (`console::read_line`, which the aarch64 console's PL011
/// driver has no receive path for). Both are recorded where they are, and both would have to move
/// before this module could. Written as a `cfg` rather than as a no-op body on purpose: a module
/// that compiled everywhere and could only ever decline on two of three would read as portable.
#[cfg(target_arch = "x86_64")]
pub mod install_service;

/// **A confined EL0 process drives a real, non-virtio DMA device** (milestone 261).
///
/// What these prove that nothing else would: that the NVMe queue mechanics work from ring 3 with
/// no authority over the controller's own registers, that the blk contract a client holds names
/// neither the device nor the doorbells, and that the bytes a client reads back are the bytes it
/// wrote, through a controller an IOMMU confined before it was ever enabled.
///
/// `cfg(initrd)`: see `kernel/build.rs::declare_initrd_cfg`; the server is a packed program, so a
/// build without an archive cannot spawn it.
#[cfg(all(test, initrd))]
mod non_volatile_memory_express_tests;

/// **Randomness that an adversary cannot predict** (milestone 56, DECISIONS §44).
///
/// Not arch-gated and not transport-gated: the same binary, the same contract, the same assertions,
/// over virtio-mmio and over PCIe on both ISAs, because a random source that works on one bus is
/// not a random source (§18, §19).
///
/// What these prove that nothing else would: that bytes from a *device* reach a userspace client
/// through a capability that names no device, that consecutive draws are not the same bytes (a
/// stuck source, a re-served buffer, or a driver reading a stale ring all present as repeats), and
/// that the count in a reply is honoured so a caller cannot be handed zeros it mistakes for entropy.
///
/// `cfg(initrd)`: see `kernel/build.rs::declare_initrd_cfg`. Not aarch64/riscv64-only for much
/// longer: `x86_64` picks it up the moment milestone 161 item 4's userspace-compilation hand-off
/// lands (no `x86_64` arm in `declare_initrd_cfg` as of this writing), and every test in this
/// module, including the instruction-backend one below, needs no further change to run there.
/// Confirmed 2026-08-25 by cherry-picking milestone 162's `x86_64` scheduling fix onto that
/// hand-off's branch:
/// `a_client_obtains_unpredictable_bytes_from_rndrrs_with_no_device_at_all` passes under the
/// suite's default `-cpu max` with no code change of its own.
#[cfg(all(test, initrd))]
mod entropy_tests;

/// **What a userspace program is told the counter runs at** (2026-09-21, calef's ruling that a
/// program returns accurate numbers rather than hardcoded ones).
///
/// Its own file rather than a case in `tests.rs`, and arch-neutral on purpose: all three
/// architectures run literally these tests (DECISIONS §19, parity is a gate), even though each
/// learns its rate a different way and two of the three carry it to userspace through a page the
/// third does not need. The question ("does a process time itself against the number the machine
/// stated") is the same on all three, so the assertion is too.
#[cfg(all(test, initrd))]
mod counter_frequency_tests;

/// **A thread's own CPU, read from a page with no syscall** (calef's 2026-09-21 ruling that
/// observing yourself is a page and observing another thread is a selector; that decision's
/// section is on another branch and is named here rather than cited).
///
/// Its own file rather than a case in `tests.rs`, and arch-neutral on purpose: all three
/// architectures run literally these tests (§19 (architectural parity is a tenet)), and here that
/// is more than a convention. The page exists *because* the register that would answer this is
/// x86_64-only, so a suite that proved it on one ISA would prove the wrong thing.
#[cfg(all(test, initrd))]
mod current_cpu_tests;

/// **The credential service, its provisioner, and its clients** (milestone 56, the credential half;
/// notes/credentials.md).
///
/// The kernel's part is the wiring, and here more than anywhere the wiring *is* the argument. Four
/// processes, and the difference between three of them is one field of a `Spawn` literal:
///
/// | process | slot 0 | what that means |
/// |---|---|---|
/// | `credentialer` | the provision endpoint (READ) **and** the verify endpoint (READ, slot 1) | holds the store |
/// | `credentialer_test_client` provisioner | the **provision** endpoint (WRITE) | may write the store, until the seal |
/// | `credentialer_test_client` client | the **verify** endpoint (WRITE) | may ask a question about the store |
/// | `credentialer_test_client` attacker | the **verify** endpoint (WRITE) | the identical endowment, used otherwise |
///
/// The kernel never sees a secret, holds no store, and computes no hash. It creates two endpoints,
/// two frames, and a budget, and hands each process a different subset. Everything after the spawn
/// is userspace agreeing with userspace over `credential_protocol`.
///
/// **Two frames and not one**, which is the detail worth stating: the provisioner writes plaintext
/// secrets into its page, so a client sharing that frame would read them. The two pages are
/// separate physical frames and neither process is ever given the other's.
///
/// Arch-neutral: one portable binary each, both ISAs (DECISIONS §19). Argon2id is arithmetic on
/// `u64`s and neither the service nor its clients contain a line of assembly.
#[cfg_attr(not(test), allow(dead_code))] // the milestone-56 credential tests are its callers
pub mod credential_service;

/// **A secret you can check and cannot read** (milestone 56, the credential half).
///
/// Not arch-gated: the same binaries, the same contract, the same assertions on aarch64 and
/// riscv64, because a credential store that authenticates on one instruction set is not a
/// credential store (§19).
///
/// What these prove that nothing else would:
///
/// - that a **userspace** client with one endpoint and no store gets a correct yes/no over a real
///   Argon2id verification, with the salt drawn from a real virtio-rng;
/// - that the **identical endowment**, used by a program that wants to write the store instead of
///   reading it, cannot;
/// - that the frame a client shares with the service holds **nothing** after the answer, which is
///   the strongest form of "the reply carried no data" that a test can check.
#[cfg(all(test, initrd))]
mod credential_tests;

/// **The login service: authentication produces capabilities, not a mutated identity** (milestone
/// 49, DECISIONS §109). The kernel spawns it exactly as it spawns `credentialer`: the archive
/// mapped read-only, a construction budget, and the endpoints it needs, so what is under test is
/// `components/src/login.rs`'s own choices rather than a privileged shortcut.
///
/// Not arch-gated, for `credential_service`'s own reason: `nifefs`, `elf`, and
/// `supervision_protocol::build_child` are portable, and a login service that mints capabilities on
/// one instruction set and not another is not the claim this milestone makes.
#[cfg_attr(not(test), allow(dead_code))] // the milestone-49 login tests are its callers
pub mod login_service;

/// **A login produces a directory and a budget, not a changed identity** (milestone 49).
///
/// What these prove that nothing else would: that a correct identity and secret yield capabilities
/// which actually work (a real `READDIR` through a freshly built `fs_subtree_caretaker`, a real
/// page retyped from a freshly split budget), that a wrong secret is refused and nothing follows
/// the refusal, and that two different identities' channels are independently working and
/// correctly attributed in the service's own audit trail (DECISIONS §109's property, made
/// checkable). See `components/src/login.rs`'s BUGS for what this slice does not attempt: a terminal,
/// per-principal subtree scoping, and wiring into the interactive boot are all named there as
/// follow-on rather than guessed at here.
#[cfg(all(test, initrd))]
mod login_tests;

/// **A provisioning tool: create an identity and its home subtree together** (milestone 155,
/// DECISIONS §117). Spawned once per identity, against a credential service's still-open provision
/// endpoint and a directory capability wide enough to hold the new subtree, exactly as
/// `credentialer_test_client`'s provisioner role is spawned, except this is the first real caller
/// rather than a test harness.
#[cfg_attr(not(test), allow(dead_code))] // the milestone-155 provisioning tests are its callers
pub mod identity_provisioner_service;

/// **A principal never exists with a credential and no home, or a home and no credential, for
/// longer than one tool invocation** (milestone 155).
///
/// What these prove that nothing else would: that a fresh identity gets both a working credential
/// (a real `VERIFY` against what was just `PUT`) and a real subtree (a real `MKDIR` that a
/// subsequent `OPENDIR` can descend into), that a duplicate identity's credential half is refused
/// without disturbing an existing subtree, and that re-running the tool against a subtree that
/// already exists (`EEXIST`) is recovery rather than a second failure. See
/// `components/src/identity_provisioner.rs`'s own module docs for the ordering argument these tests hold
/// it to.
#[cfg(all(test, initrd))]
mod identity_provisioning_tests;

/// **The boot-time re-deriver** (milestone 152's third piece, provisional name `session_reviver`;
/// DECISIONS §123). Spawned once, holding exactly a construction budget and the store-read
/// capability, checked against the boot's measurement table before it is granted either
/// (DECISIONS §123's second hardening refinement). See `components/src/session_reviver.rs`'s own module
/// docs for what it does with them and why it is a new process rather than a phase of an existing
/// boot component.
#[cfg_attr(not(test), allow(dead_code))] // the milestone-152 durable-schedule tests are its callers
pub mod session_reviver_service;

/// **The schedule store's write path and read-at-boot path prove each other** (milestone 152's
/// first and third pieces; DECISIONS §122, §123, §125).
///
/// What these prove that nothing else would: that a schedule entry `fs_test_client`'s
/// `ROLE_SCHEDULE_SEED` writes through ordinary `filesystem_protocol` verbs is the same document
/// `session_reviver` reads back and `timetable::parse` accepts, that the manifest (§125's own answer
/// to "which identities", read by name rather than by `READDIR`) carries that identity to the
/// re-deriver without either program enumerating anything, that a session re-derived at boot has the
/// identical §16 lifecycle a live login's own durable session does, and that
/// the re-deriver's own capabilities are gone and provably so once its one pass finishes.
#[cfg(all(test, initrd))]
mod session_reviver_tests;

/// **The NTP client, and the test server that answers it** (milestone 51; DECISIONS §43, §44).
///
/// The kernel's part is the wiring, and the wiring *is* the argument. An NTP client here gets five
/// slots: a report endpoint, the socket contract's endpoint, an untyped budget, the clock service's
/// **propose** endpoint, and the entropy service's endpoint. What it does not get is a mapping of
/// the clock page, in either direction, which is the whole difference between this and a Unix
/// `ntpd` running as root.
///
/// The test server is a separate program holding `READ` on the endpoint the client holds `WRITE`
/// on. Substituting the peer at a capability boundary is how a capability system tests a client:
/// the client's code does not change and cannot tell. See
/// components/src/network_time_client.rs for what that proves and what it leaves to milestone 30's
/// socket-contract tests. It was a role of the client's own binary until milestone 290, and nothing
/// about the substitution depended on that: the boundary is the capability.
///
/// Arch-neutral: three portable binaries, both ISAs (DECISIONS §19).
#[cfg_attr(not(test), allow(dead_code))] // the tests are its callers
pub mod ntp_service;

/// **An NTP client may propose a time and may not set one** (milestone 51).
///
/// The milestone's demonstrable claim, and the one Unix cannot make: `ntpd` runs as root and may set
/// the clock to anything. Here the network-facing component holds an endpoint the clock service is
/// free to refuse, and holds no mapping of the page the offset lives in. These tests take that
/// apart: the happy path lands as a **proposal**, a reply that fails validation moves nothing, a
/// proposal outside the policy's bounds is refused **by the service**, and a write aimed straight at
/// the clock page kills the process.
///
/// Not arch-gated: three portable binaries, the same assertions on aarch64 and riscv64
/// (DECISIONS §19).
#[cfg(all(test, initrd))]
mod ntp_tests;

/// Milestone 11: hand a process an untyped budget and let it spend it.
#[cfg_attr(not(test), allow(dead_code))] // the tour and the userspace tests
pub mod memory_region_service;

/// **The untyped-backed userspace heap** (milestone 27): spawn the `allocator_exerciser` workload, the
/// first program that links `extern crate alloc`, with an untyped budget (slot 0) and a report
/// endpoint (slot 1). The program wires `user_mode_runtime::heap` as its global allocator, churns
/// `Vec`/`String`/`BTreeMap` with frees in arbitrary order, asserts every intermediate result
/// itself (a wrong value faults), and reports a magic word plus how many bytes of heap it
/// committed. Portable: the same test runs the riscv64 ELF on riscv and the aarch64 ELF on
/// aarch64, out of each arch's own initrd.
#[cfg(test)]
pub mod alloc_service;

#[cfg(all(test, initrd))]
mod heap_tests;

/// **The compositor: one screen, several mutually distrusting clients** (milestone 33, rung two of
/// the display ladder).
///
/// Arch-neutral, like rung one and for the same reasons: two portable binaries in both archives, one
/// host-tested contract crate, and an isolation property that is the kernel's own (mappings and
/// capabilities), so **both ISAs run literally these tests**.
///
/// Three of the four tests do not need a GPU at all, and take a **kernel stand-in for the display**
/// instead. That is not a shortcut, it is two things at once: it keeps four device bring-ups down to
/// one, and it makes the flush rectangles *observable*, which is how "a one-window redraw does not
/// cost a whole screen" becomes an assertion instead of a claim. It is also the swappable-component
/// story falling out for free: the compositor cannot tell whether the endpoint it flushes to is a
/// virtio-gpu driver or the kernel.
///
/// **These tests run before `display_tests`** (`compositor_tests` sorts first), which matters for the
/// host-side scanout check: the composed screen goes up first and rung one's pattern last, and
/// `cargo xtask` looks for both in that order. See notes/compositor.md.
#[cfg(all(test, initrd))]
mod compositor_tests;

/// **The display: virtio-gpu, a confined driver, and a client that draws** (milestone 29, rung one
/// of the display ladder).
///
/// Arch-neutral on purpose, unlike most of the device tests here: the driver and the client are
/// portable binaries in both archives, the transport is the same PCIe seam on both boards, and the
/// contract is one host-tested crate, so **both ISAs run literally this test** rather than two
/// copies of it that can drift (DECISIONS §19: parity is a gate).
#[cfg(all(test, initrd))]
mod display_tests;

/// **Rust `std` on the native ABI** (milestone 27): spawn the `std_exerciser` demo, an ordinary Rust
/// program (no `no_std`, no attributes) built for the `*-unknown-nife` custom target with std's
/// PAL implemented directly over the capability ABI. It gets the same two grants as `allocator_exerciser`,
/// an untyped budget (slot 0, which the std `GlobalAlloc` draws the heap from) and an endpoint
/// (slot 1, which `println!` SENDs to). Its stdout is a fixed, deterministic transcript the test
/// reassembles from the endpoint and checks byte for byte. Portable: the aarch64 ELF runs on
/// aarch64 and the riscv64 ELF on riscv, out of each arch's own initrd.
///
/// **Since milestone 51 it is also granted a wall clock**: a clock service is started first, and
/// the program gets that service's page as a `PageFrame` capability with `READ` in slot 5 plus a
/// read-only mapping of it. That is the whole of a std program's wall-clock authority, and it is
/// what turns `SystemTime::now()` from "1970 plus uptime" into a real answer (DECISIONS §43).
#[cfg(test)]
pub mod std_service;

#[cfg(all(test, initrd))]
mod std_tests;

/// **Unmodified `ripgrep` from crates.io** (milestone 121), which skips unless somebody ran
/// `helpers/build-ripgrep.sh`. Every ISA the `std` port ships on, per DECISIONS §19, which is all
/// three since milestone 184 built `x86_64-unknown-nife`.
#[cfg(all(test, initrd))]
mod ripgrep_tests;

/// **A TLS crypto provider's primitives against published test vectors**, for milestone 442 (a crypto provider `rustls` can use on all three bare-metal targets), on
/// `ripgrep`'s exact terms: present only when `helpers/build-cryptography-exerciser.sh` has been
/// run, because the crates under it are a dependency decision DECISIONS §46 (thin primitives or whole subsystems; we write everything in between) makes calef's. Every
/// ISA, per DECISIONS §19 (architectural parity is a tenet; the targets are aarch64, riscv64 and x86_64),
/// and here the x86_64 leg is the one that matters most: it is the only
/// target whose build forces the portable implementations.
#[cfg(all(test, initrd))]
mod cryptography_tests;

/// **Capability delegation: authority moves between processes at runtime.**
///
/// Every other capability in nife is minted by the kernel and handed to a process at spawn.
/// That made the kernel a central authority-granting oracle, which is the ambient-authority shape
/// §10 argued against, just relocated. A capability system's defining move is that a process can
/// pass authority it holds to another process, narrowing it on the way, and only if it was trusted
/// to (`GRANT`). This wires the smallest scenario that exercises all three: a *granter* delegates a
/// resource capability to a *receiver* over a channel, narrowed to `WRITE` (no `GRANT`); the
/// receiver uses it and then cannot pass it on. See fixtures/src/hello.rs `granter()/receiver()`.
/// **`PageFrame` capabilities: shared memory a process holds, maps, and delegates.**
///
/// The payoff of delegation applied to memory. A *producer* retypes a page out of its own untyped
/// into a `PageFrame` capability, maps it, writes into it, and delegates a READ-only view to a
/// *consumer*, which maps the same physical page and reads what the producer wrote. The kernel
/// copies nothing and pre-arranges nothing: the two processes compose the sharing themselves, and
/// the read-only narrowing means the consumer can look but not write. See fixtures/src/hello.rs
/// `page_frame_producer()/page_frame_consumer()`.
// Test scaffolding: the `tests` module below is the only caller, and it runs on both ISAs now
// (milestone 19's user-test port). This wiring was already portable; it was compiled out on riscv64
// only because its consumer was.
#[cfg(test)]
pub mod page_frame_service;

// Test scaffolding: the `tests` module below is the only caller, and it runs on both ISAs now
// (milestone 19's user-test port). This wiring was already portable; it was compiled out on riscv64
// only because its consumer was.
#[cfg(test)]
pub mod delegation_service;

/// **Milestone 19a: a process mints an endpoint from its own memory, at EL0.** The maker holds
/// an untyped budget and a channel; the peer holds the channel and a report line. Everything
/// else, the endpoint itself included, is created at runtime by the maker out of its own pages
/// and delegated. See fixtures/src/hello.rs `ep_maker()/ep_user()`.
// Test scaffolding: the `tests` module below is the only caller, and it runs on both ISAs now
// (milestone 19's user-test port). This wiring was already portable; it was compiled out on riscv64
// only because its consumer was.
#[cfg(test)]
pub mod retype_ep_service;

/// **Milestone 19b: a process builds an address space, at EL0.** One role: an untyped budget
/// and a report line; everything else it constructs. See `fixtures/src/address_space_witness.rs`,
/// which milestone 291 split out of the `hello` multiplexer this line still pointed into.
// Test scaffolding: the `tests` module below is the only caller, and it runs on both ISAs now
// (milestone 19's user-test port). This wiring was already portable; it was compiled out on riscv64
// only because its consumer was.
#[cfg(test)]
pub mod address_space_service;

/// **Milestone 12: Call/Reply, at EL0.** One request endpoint, a server that answers a caller it was
/// never wired to, and the one-shot reply capability proven across the boundary. See
/// fixtures/src/hello.rs `call_server()/call_client()`.
// Test scaffolding: the `tests` module below is the only caller, and it runs on both ISAs now
// (milestone 19's user-test port). This wiring was already portable; it was compiled out on riscv64
// only because its consumer was.
#[cfg(test)]
pub mod call_service;

/// **Milestone 13: revoke a frame, at EL0.** One process with an untyped budget retypes a frame,
/// maps it, revokes it, and reports whether the revoke deleted its own capability. See
/// fixtures/src/hello.rs `revoke_demo()`.
// Test scaffolding: the `tests` module below is the only caller, and it runs on both ISAs now
// (milestone 19's user-test port). This wiring was already portable; it was compiled out on riscv64
// only because its consumer was.
#[cfg(test)]
pub mod revoke_service;

/// **The in-kernel userspace suite, on both instruction sets** (milestone 19's user-test port).
///
/// It was aarch64-only for most of this project's life, and the module comment used to say the
/// reason was the tests: "every test drives a hand-written aarch64 program through `exec` and reads
/// aarch64 fault registers". That was true, and it was the wrong thing to fix. The tests were fine;
/// their *scaffolding* was aarch64. Three things moved and the tests came along unchanged:
///
/// 1. The hand-assembled programs became real ELFs the toolchain builds for both targets (the
///    `outlaw` binary and the `interrupt_ignorer` that already existed). See the note above
///    `OUTLAW_ROUND_TRIP`.
/// 2. `ESR`/`FAR` became `arch::UserFault`, the same fact in words RISC-V can say, which is what
///    keeps "a PERMISSION fault at exactly this address" assertable rather than softened to "a fault
///    happened".
/// 3. `hello`, which carries the milestone 7-19 role catalogue, was found to build for RISC-V once
///    six syscalls it had hand-rolled in aarch64 `asm!` were routed through `user_mode_runtime`, which already
///    had portable versions of all six.
///
/// **What is still gated, and why, is written at each test rather than here**, because a blanket
/// module comment is how the old claim survived past the point of being true. Two kinds of gate
/// appear below: a property that has no RISC-V analogue at all (`el1_runs_on_sp_el1`), and a
/// property whose RISC-V twin lives in `riscv_virtio_tests` and would be duplicated rather than
/// gained. See notes/riscv-parity-scope.md.
#[cfg(all(test, initrd))]
mod tests;

#[cfg(test)]
/// Spin the scheduler until `done()`, or give up after a wall-clock deadline. Returns whether it
/// happened. **Time-based, not a fixed yield count** (DECISIONS §28): with work spread across
/// cores, the test thread's own core is often idle, so a yield returns at once and a fixed count
/// of them elapses in almost no real time, timing out before a parallel result on another core
/// lands. A ~2 s deadline gives the other cores real time to finish while staying far under the
/// 60 s hang watchdog, so a genuine hang still fails.
///
/// It lives **here** rather than in `tests` because six sibling modules use it and that one does
/// not compile on every architecture: `user::tests` needs a real ELF program out of the initrd
/// and is `#[cfg(all(test, initrd))]`, which would have taken this helper down with it on a
/// target that packs none (milestone 161, roadmap item 4). A helper every module uses does not
/// belong inside one of them. Milestone 81 needed it in two of them: running on the physical core makes the
/// yield-count version fail for the *mirror* reason it fails on a loaded host, since a yield on an
/// idle core costs nanoseconds there. See notes/hvf-leg.md.
pub(crate) fn wait_for(mut done: impl FnMut() -> bool) -> bool {
    let deadline = crate::arch::timer::now() + 2 * crate::arch::timer::frequency();
    while crate::arch::timer::now() < deadline {
        if done() {
            return true;
        }
        crate::sched::yield_now();
    }
    done()
}

/// **Forcible teardown: `DESTROY` tears a runaway down** (DECISIONS §16 amendment, §24's second-`^C`
/// tier). A child spinning at EL0, never yielding and never checking an endpoint, cannot be waited
/// out; its region's owner must be able to reclaim it anyway. This is the one cross-ISA test in this
/// file, because the mechanism it proves is pure portable scheduler logic: the only per-architecture
/// part is the single spin instruction (`b .` / `j .`), and the whole capability dance around it is
/// the same code both ISAs run. It is separate from the aarch64 module above precisely so it can run
/// on both, which the parity gate (DECISIONS §19) asks of every kernel capability.
#[cfg(test)]
mod force_kill_tests;

/// **A first process that gives its authority away, and a supervision tree that outlives it** (milestone 22
/// phase B.2).
///
/// Cross-ISA, because every piece is portable: the whole tree is four ordinary user programs
/// (`root_supervisor`, `spawner`, `sub_server_supervisor`, `flaky`) built out of the capability verbs, and the kernel's only
/// part is the fault endpoint phase A already built.
///
/// The kernel spawns `root_supervisor` the way it spawns the progenitor: the archive mapped read-only, one untyped
/// budget, one report endpoint. `root_supervisor` then builds a construction sub-server and a supervisor, hands
/// each exactly what it needs, and **deletes its own budget**. From then on the tree runs without it:
/// the sub-server crashes, its supervisor hears about it, reaps it through the spawner, and asks for a
/// replacement, which runs and exits cleanly. The progenitor could not have done any of that, and that is what
/// these two tests prove.
#[cfg(all(test, initrd))]
mod authority_tests;

/// **The interactive boot's half of the same idea: a job's memory comes home** (milestone 22, the
/// increment that migrated the hand-validated boot path).
///
/// The tree above proves a first process that can hand its construction authority away entirely. The
/// interactive progenitor cannot: it stays the shell's spawn service, so it must keep *some* budget. What
/// it can do instead is keep a **bounded** one and make it renewable, which is what these two tests
/// are about. Every job the prompt spawns is built in a region split off that pool and born
/// supervised, and `job_undertaker` (one endpoint capability, no memory at all) collects the corpse
/// through `Rendezvous::REAP`, which returns the region to **The progenitor's** pool under §13 region ownership.
///
/// The pair is a control and a claim, in that order: three jobs exhaust the pool when nothing
/// collects, and twelve go through the same pool when `job_undertaker` does. Neither is a timing
/// argument; the assertion in both is which budget the pages are in.
///
/// Cross-ISA, because every piece is portable: `job_undertaker` is an ordinary program in both archives
/// and the reap authorization reads two TCB fields.
#[cfg(all(test, initrd))]
mod job_undertaker_tests;

/// **A memory-unsafe C component, confined** (milestone 36, DECISIONS §31).
///
/// The thesis (§14) is a verified core that confines unverified workloads, and C is the most
/// unverified workload available: no bounds checks, no borrow checker, nothing between a bad index
/// and a store. So this is not a dilution of the claim, it is the sharpest available test of it. The
/// contrast is concrete rather than rhetorical: in a monolith, C filesystem or driver code with this
/// bug is a kernel memory corruption; here it is a page fault in an unprivileged process, and its
/// supervisor restarts it.
///
/// **What is under test is the seam, not the C.** `fixtures/c/c_seam.c` is deliberately throwaway: 150
/// lines, one honest function and two one-line bugs. What the milestone de-risks is everything around
/// it, before a real foreign component (libghostty-vt, milestone 29's later rung) depends on it: a
/// bare-metal clang in the build for both ISAs, a Rust `user_mode_runtime` shell that holds every capability so
/// the C can hold none, and five libc symbols shimmed rather than a libc ported.
///
/// **The four claims, and how each is proven rather than assumed.** All four are asserted from
/// outside the faulting address space, by `c_confiner`, after the component is dead:
///
/// 1. *It faults*, rather than silently corrupting and continuing. Proven by the death message
///    existing at all, with `EVENT_FAULT` and a non-zero kernel-stamped tid.
/// 2. *The fault is the bug we planted.* The kernel's reported fault address equals the address the C
///    code computed, so the crash is not something unrelated on the way there, which would make the
///    rest of the assertions vacuous.
/// 3. *Nothing outside the grant changed.* Two witness pages, both position-derived patterns
///    checked byte by byte through the confiner's own mappings. `WITNESS_RO` is the **same physical
///    frame** the component holds read-only, so an unchanged page is not "the store landed
///    elsewhere"; the page was reachable and the store did not happen. `WITNESS_FAR` is a
///    **different frame at the same virtual address**, which is the statement that a virtual
///    address means nothing outside the address space that owns it.
/// 4. *The supervisor restarts it and the restart works.* Not "an instance ran": the replacement's
///    output is read out of the shared grant and checked against an independent Rust computation of
///    the same checksum, so a restart that produced a process which merely reported for duty fails.
///
/// The in-grant marker byte is the control for all of it. Each misbehaving C function stores inside
/// its grant first, and that store must be visible; a process whose stores never worked would satisfy
/// every witness check while proving nothing.
///
/// Both ISAs, because a fault that only manifests on one would be a finding, not a pass. The two
/// bugs take *different* fault paths on each (a permission fault on the read-only page, a translation
/// fault on the unmapped one), which is more of each architecture's fault machinery than any previous
/// test has exercised from userspace.
#[cfg(all(test, initrd))]
mod c_seam_tests;

/// **A running component replaced under a talking client** (milestone 23, DECISIONS §41).
///
/// The flagship the roadmap points at, and the thing to notice about it is what the kernel does not
/// contain. There is no component object, no swap syscall, no naming service, and no
/// lifecycle-aware anything: `swapper` is an unprivileged process with a budget, one device
/// capability and four endpoints, and the swap is the composition of mechanisms that already
/// existed for their own reasons. What milestone 23 needed the kernel to grow is exactly one thing:
/// `PageFrame::REVOKE` now answers on a `DeviceFrame`, with take-back semantics (§41).
///
/// **The claim is not that a swap completes. It is that a client does not notice.** So the shape is
/// the one milestones 29, 33 and 36 used: two witnesses in two address spaces, an attacker with
/// real authority, and a control that must fail.
///
/// 1. **The client's witness**, computed inside `chatty` from the replies it received. It holds one
///    capability to one endpoint for its whole life, calls sixty-four times in a plain loop, and
///    checks every answer against its own independent computation of the digest. It has no code
///    path for "the server went away" because there is no such event to have one for.
/// 2. **The operator's witness**, a shared page in `swapper`'s address space that each instance
///    stamps with its own version per request. Read after every writer is dead, it says that no
///    request went unserved (nothing was lost in the down window) and that the version never goes
///    backwards (**there were never two owners of the device at once**, which is the whole reason
///    step 2 revokes).
/// 3. **The control that must fail**: the outgoing instance is told to read one UART register
///    *after* the operator revoked it. It faults, and the kernel's fault message carries the
///    device's own virtual address. Before the revoke the same read succeeded, which is what makes
///    this a receipt rather than a coincidence.
/// 4. **The attacker**, `chatty` in its usurper role, endowed with exactly the honest client's
///    capabilities including a real working capability to the stable endpoint. It tries to park
///    itself in `RECV_CAP` and become the server. `NotPermitted`: its capability carries `WRITE`
///    and not `READ`, so endpoint-only naming does not mean "whoever holds the endpoint is the
///    server".
///
/// **The replacement is written in C** (`fixtures/c/c_swappable.c`, over the seam DECISIONS §31 built),
/// and that is the strongest form of the claim: what held across the swap was the contract, not a
/// recompile of the same source.
///
/// The second test covers the latency ladder's opt-in rung, `broker`. Both ISAs, because a swap
/// that only worked on one would be a finding, not a pass.
#[cfg(all(test, initrd))]
mod live_swap_tests;

/// **Measured boot: the kernel refuses to enter a first process it was not built for** (milestone 22 phase
/// B.1, DECISIONS §22).
///
/// Cross-ISA, because the check is portable: one hash implementation (`crates/measured_boot`), one trust
/// root generated into the kernel image by `build.rs`, called from the boot path on every
/// architecture (`boot_progenitor`; the riscv milestone-20 demo `riscv_initrd_demo` measures too).
///
/// **What these two prove, and why the boot path itself cannot be tested directly.** A real refusal
/// halts the machine, so a test cannot take that branch and live. What *can* be proven, and is what
/// actually matters, is the decision: the same function the boot path consults says Ok for the bytes
/// in the initrd QEMU loaded (which proves the whole build composition end to end: userspace built,
/// archive packed, digest written, kernel compiled with it, and the digest in the running image
/// matches the archive in RAM), and says Err for bytes off by one bit. The boot path's only response
/// to Err is `arch::halt()`, which is three lines up from here in `trust::require` and is the sort of
/// thing a reader can check by looking.
#[cfg(all(test, initrd))]
mod measured_boot_tests;

/// **The fault endpoint: a supervisor watches a child die and reap it** (milestone 22, DECISIONS
/// §26). These are the cross-ISA tests, because the mechanism is portable: a supervised child that
/// faults (or exits) turns into a five-word message on its supervision endpoint, its corpse persists
/// until the supervisor reaps it with §16 revocation, and a fresh child runs in its place. The only
/// per-architecture parts are the two tiny code stubs (a null load that faults, and a `SEND` + exit),
/// and even those are the same shape both ISAs already use elsewhere in this file. The kernel is the
/// only sender on the fault endpoint, so the tid the supervisor reads is trustworthy without a badge.
#[cfg(test)]
mod supervision_tests;

/// The two load-bearing tests of the x86 port-range capability (milestone 299): a non-holder faults
/// on `out` (and a holder's grant does not leak across the switch to it), and a revoked holder faults
/// on its next `out`. `x86_64` only, because the mechanism is the TSS I/O permission bitmap, which
/// the other two architectures have no counterpart to.
#[cfg(all(test, target_arch = "x86_64"))]
mod x86_port_tests;

/// **A supervisor may collect a corpse without being able to build one** (DECISIONS §32,
/// `rendezvous::REAP`). Cross-ISA, because the authorization check is architecture-neutral: it reads
/// two fields of a TCB and compares two generational names, so a divergence here would mean
/// something is wrong under `arch/`, not in this feature.
///
/// **What shape these tests are, and why.** Every reap goes through the real syscall dispatcher
/// (`syscall::invoke`), from a thread whose capability table holds **endpoint capabilities and
/// nothing else**: that is what a supervisor's authority actually is, and calling `sched` directly
/// would prove the helper rather than the boundary. The *building* is done with kernel-internal
/// calls, which is deliberate: it keeps the builder's authority out of the supervisor's capability table, so
/// "structurally unable to build" is a fact about the table these tests audit rather than a promise.
///
/// The accounting proof is the one that makes §32 worth having. A test that only showed the corpse
/// gone would be satisfied by a reap that quietly handed the pages to the reaper. So the builder's
/// region is one the test still owns and can measure, and the assertion is that its watermark comes
/// back down and it can spend those pages again, while the supervisor's capability table does not grow.
#[cfg(test)]
mod reap_tests;

/// **A process listing is a capability, not a fact about the machine** (milestone 126,
/// `rendezvous::SURVEY`, notes/process-view.md). Cross-ISA for the same reason `reap_tests` is: the
/// scope decision reads one field of a TCB and compares two generational names, so a divergence
/// here would mean something is wrong under `arch/` rather than in this feature.
///
/// **The shape, and why it is this shape.** Every survey goes through the real syscall dispatcher
/// (`syscall::invoke`), and the walk is driven by `ps::collect`, which is the loop `components/src/ps.rs`
/// really runs: a bug in the cursor protocol therefore cannot hide in the gap between the kernel's
/// half and the program's. The tests build real supervised children out of a real region, so the
/// domain under test is one the kernel built rather than one a helper described.
///
/// The negative control is the one that matters, and it keeps milestone 108's shape: a viewer run
/// against a domain it was not granted is **refused loudly** rather than shown an empty list, and
/// an empty domain answers rather than refusing. Both are asserted in the same test, because
/// neither claim means anything without the other.
///
/// `pgrep`'s filter is driven here too, and this is the only place in the tree that can be: the
/// selector arrives in a register, and the prompt cannot spell one (`crates/pgrep`'s `BUGS`). The
/// negative control gains a fourth answer with it, which is a selector that **matched nothing** in a
/// domain that really has members: distinct from an empty domain and from a refusal, where upstream
/// `pgrep` collapses all three into printing nothing.
#[cfg(test)]
mod survey_tests;

/// **The other axis of a survey: which per-thread fact it asks for** (calef's 2026-09-21 selector
/// ruling, `abi::survey::record`).
///
/// `survey_tests` above proves the walk, meaning what a domain contains and who may look at it.
/// This proves the selector, meaning which record a walk returns. Separate files because the two
/// properties are independent and their failures read nothing alike: a broken walk reports the
/// wrong threads, where a broken selector reports the wrong fact about the right threads, with
/// every tid still looking correct.
///
/// Cross-ISA, and here that is a claim rather than a habit (DECISIONS §19). The one record this
/// ships with is placement, which is `sched`'s: `pick_spawn_target` samples two online cpus and
/// `place_on` enqueues onto the winner, with no line of either under `arch/`. All three
/// architectures therefore run literally these assertions, and a divergence would mean the
/// scheduler is wrong rather than an ISA.
#[cfg(test)]
mod survey_record_tests;

/// **What the CPU-time record's number means** (milestone 282 (a thread's CPU time, and the `top` it makes possible), DECISIONS §150 (how does a thread's CPU time reach userspace?)).
///
/// `survey_record_tests` proves that a record can be asked for and that an unknown one is refused,
/// which a record returning a constant zero would satisfy. This proves the figure: a runaway is
/// charged for the CPU it took, a thread blocked in a send is charged for nothing, and a corpse
/// keeps what it earned. The first of those is the assertion the wall-clock age §150 refused would
/// fail, since two threads of the same age read identically under it.
#[cfg(test)]
mod cpu_time_tests;

/// **`pmap`'s split, one object type over `survey_tests`** (milestone 126, `address_space::LIST`,
/// DECISIONS §114). Cross-ISA for `survey_tests`'s reason: the method reads `Flags` through
/// `arch::mmu::translate_at`, so a divergence here means something is wrong under `arch/`.
///
/// Every listing goes through the real syscall dispatcher, driven by `pmap::collect`, the loop
/// `components/src/pmap.rs` really runs, `survey_tests`'s discipline verbatim. The negative control is
/// the one that matters: a capability holding `ENUMERATE` alone can list every mapping and is
/// refused `MAP_INTO`, and a capability holding `WRITE` alone can map and is refused `LIST`, so
/// the split is proved in both directions rather than asserted in prose.
#[cfg(test)]
mod pmap_tests;

/// **Scheduled execution, where every entry is a grant** (milestone 129, notes/scheduled-execution.md).
///
/// One module for both ISAs, like `dir_capability_tests`: nothing in it is architecture-specific, so
/// the parity gate (DECISIONS §19) is met by literally the same test running twice.
///
/// The claim is Unix cron's inversion. A crontab line runs as a user and can do whatever that user
/// can do, and there is nothing to print and nothing to check; here an entry is a grant expression
/// checked at registration by the same `grant_plan::plan` the prompt uses, so what a scheduled child
/// will hold is printable before the first tick. The test reads that plan off the real program
/// running the real `components/timetable.conf`, then watches what fires.
///
/// The negative control is what makes it worth having: the shipped document contains entries a Unix
/// cron would simply have run (`date` wants a clock, `ps` wants a process view), and the timetable
/// holds neither, so both are refused **in writing, before anything fires** and neither ever runs.
#[cfg(all(test, initrd))]
mod timetable_tests;

/// **The directory capability, attacked** (milestone 47, notes/dir-capability.md).
///
/// One module for both ISAs rather than an aarch64 test with a riscv twin, which the FS tests above
/// have. Nothing here is architecture-specific: it wires three portable programs and asserts on a
/// bitmap, so the only difference between the legs is which binary carries the block-server role,
/// and that is one `cfg` in [`blk_server_image`] rather than a second copy of every assertion. The
/// parity gate (DECISIONS §19) is met by literally the same test running twice.
#[cfg(all(test, initrd))]
mod dir_capability_tests;

/// **One process, two directory capabilities** (milestone 154,
/// design/roadmap/154-multi-directory-namespace.md).
///
/// One module for both ISAs, for [`dir_capability_tests`]'s reason: nothing here is
/// architecture-specific, so the parity gate (DECISIONS §19) is met by literally the same test
/// running twice. It wires the same three portable programs [`dir_capability_tests`] does, twice
/// (a second `fs_subtree_caretaker`, a second capability table slot) for one confined program, and proves
/// the deliverable both milestone 47's `bind` and milestone 64's `File::open` fork were blocked
/// on: `/a/x` and `/b/y` both resolve, `/a/../b` is refused, and neither caretaker can see the
/// other's tree.
#[cfg(all(test, initrd))]
mod multi_dir_namespace_tests;

/// **The navigation builtins, and the property that two shells cannot name each other's files**
/// (milestone 47's commands; notes/shell-navigation.md).
///
/// One module for both ISAs, for [`dir_capability_tests`]'s reason: nothing here is
/// architecture-specific, so the parity gate (DECISIONS §19) is met by the same test running twice.
///
/// What is wired is the **real shell binary**, in a role that reads a script instead of a keyboard,
/// holding a `fs_subtree_caretaker`'s narrowed endpoint where the interactive one holds a terminal.
/// So the builtins under test are the builtins at the prompt rather than a reimplementation of
/// them, and the thing being confined is a shell.
#[cfg(all(test, initrd))]
mod shell_navigation_tests;

/// **`rm` as a program, and a recursive removal bounded by the capability it was handed**
/// (milestone 47's `rm -r`; notes/rm.md).
///
/// One module for both ISAs, for [`dir_capability_tests`]'s reason: nothing here is
/// architecture-specific, so the parity gate (DECISIONS §19) is met by the same test running twice.
///
/// What is wired is the **real `rm` binary** (`components/src/rm.rs`) behind a real
/// `fs_subtree_caretaker`, started the way the shell would start it: the name in a grant's two
/// argument words and the options in the spec word, in `grant_plan::rmopt`'s bit order, so the numbers
/// here come from the manifest the prompt checks against rather than from a second copy of an
/// ordering.
///
/// The thing being demonstrated is not that a loop can delete a tree. It is that **the walk stops
/// exactly where the capabilities stop**: the same command line against the same tree does the
/// whole job through one grant and cannot begin through a narrower one, and no branch in the
/// program decides which.
#[cfg(all(test, initrd))]
mod rm_program_tests;

/// **Globbing: the expansion you see is the grant** (milestone 47's globbing lane;
/// notes/glob-grant.md).
///
/// One module for both ISAs, for [`dir_capability_tests`]'s reason: nothing here is
/// architecture-specific, so the parity gate (DECISIONS §19) is met by the same test running twice.
///
/// What is wired is the **real shell binary** (expanding one pattern two ways over a real
/// `READDIR`) and then the **real `rm` binary** behind a real `fs_nameset_caretaker`. The argument
/// the two halves make together is the one Unix cannot make: the names a command displays are
/// literally the authority it would transfer, and nothing else in the directory moves.
#[cfg(all(test, initrd))]
mod glob_grant_tests;

/// Parity C: the virtio-blk driver, its two attackers, and the DMA confinement, on RISC-V.
///
/// These are the riscv twins of the three disk tests in the aarch64 module above, separate
/// because that module leans on aarch64-only scaffolding (the hand-written 7a user programs and
/// the PL011-wired `hello` roles), while these need only the ELF loader and the initrd archive.
/// The driver is the SAME `virtio` module the aarch64 roles compile, packed as the dedicated
/// `block_driver` binary (`components/src/block_driver.rs`); the kernel-side wiring (`virtio_service`) is
/// the same code,
/// unconditionally. What these prove that aarch64's runs do not: userspace device drivers with
/// DMA, and the kernel's DMA confinement, on the second ISA.
#[cfg(all(test, target_arch = "riscv64"))]
mod riscv_virtio_tests;

/// **The operators, end to end: `|` is two processes and an endpoint** (milestone 50,
/// notes/pipes.md).
///
/// One module for both ISAs, for [`shell_navigation_tests`]'s reason: nothing here is
/// architecture-specific, so the parity gate (DECISIONS §19) is met by the same test running twice.
///
/// What is wired is the **real shell binary**, in a role that reads a script instead of a keyboard,
/// with the interactive endowment: a terminal, a spawn channel, a result channel, and a budget. The
/// kernel plays the two parties on the other ends.
///
/// - **The terminal.** The test itself serves `line_editor::proto::OP_WRITE` and collects every byte
///   the shell prints. So the assertion is made against *what a person would see*, which is the
///   strongest form this can take: a pipeline that ran but printed the wrong thing fails here.
/// - **The progenitor.** A second thread serves `grant_plan::spawnproto`, receiving the delegated sink and source
///   capabilities and building each stage with them. It is deliberately the same protocol
///   `user/src/system_initializer.rs` serves, because the shell cannot tell the difference and neither should
///   this test; what it is not is the same *code*, and that gap is named in notes/pipes.md's BUGS.
#[cfg(test)]
pub mod pipeline_service;

/// **`>`, `<` and `|` at a real prompt** (milestone 50, notes/pipes.md).
///
/// The claim under test is one sentence: **a program holds an endpoint for its output and cannot
/// tell what is on the other end.** So the assertions are all of the form "the same binary, two
/// destinations, the same bytes", never "the pipeline printed something".
#[cfg(all(test, initrd))]
mod pipeline_tests;

/// **`>` and `<` at a prompt that holds a filesystem** (milestone 50, notes/pipes.md).
///
/// [`pipeline_tests`]'s shell with one more capability: a directory at slot 4, narrowed by a
/// `fs_subtree_caretaker` to one subtree of the real RedoxFS image. Everything else is identical,
/// which is the point of running both. The refusal in
/// `pipeline_tests::a_redirection_a_shell_cannot_back_is_refused_rather_than_dropped` and the file
/// written here are the same binary, and the only difference between them is one capability table slot.
///
/// The assertions are all of the "same producer, two destinations, the same bytes" shape, because
/// that is the only shape that can distinguish a redirection that worked from one that wrote
/// something plausible: a `>` that dropped every second byte would still produce a file, and a `wc`
/// that agreed with it would still print three numbers.
///
/// One module for both ISAs, for [`shell_navigation_tests`]'s reason: nothing here is
/// architecture-specific, so the parity gate (DECISIONS §19) is met by the same test running twice.
#[cfg(all(test, initrd))]
mod redirection_tests;

/// **`time <command>` at a real prompt** (milestone 86, notes/time-command.md).
///
/// [`pipeline_tests`]'s shell with at most one more capability: a read-only clock page. The claim
/// under test is that **the timed command needs no authority to be timed**, so the timing is the
/// shell's own reading and the child is spawned with exactly the endowment its command line names.
///
/// The three clock states are three capability tables rather than three branches, which is the shape
/// [`redirection_tests`] uses for the directory: a published page, a page nobody published to, and
/// no capability at all. Two of those refusals are `date`'s sentences one milestone later, and the
/// only reason they are reachable is that the wiring changed.
///
/// One module for both ISAs, for [`shell_navigation_tests`]'s reason: nothing here is
/// architecture-specific, so the parity gate (DECISIONS §19) is met by the same test running twice.
#[cfg(all(test, initrd))]
mod time_tests;

/// **Quoting, sequencing and `$?` at a real prompt** (milestone 67, notes/swish-language.md).
///
/// The **same run** of the same script [`redirection_tests`] asserts about, whose tail milestone 67
/// added: one shell, once. A seventh scripted shell would have been a seventh live process whose
/// frames nothing reclaims, and wiring one put [`time_tests`] over the frame pool intermittently
/// (`refused to load a user program: Unmappable(OutOfPageFrames)`). The wiring these lines need is
/// [`redirection_tests`]'s exactly, so a second copy bought nothing but the failure.
///
/// It is still its own module, because what it claims is its own: the redirection tests are about
/// where bytes go, and these are about what a word *is* and what a status means.
///
/// The assertions are pairs, which is [`redirection_tests`]'s shape and for the same reason. `echo
/// "*.txt"` against `echo *.txt` is one line quoted and one not; `least_authority_demo 3 && echo
/// yes` against `least_authority_demo && echo yes` is one connector against a refused left-hand
/// side. A single line proving "it printed something" would pass on a shell that ignored quoting
/// entirely.
///
/// One module for both ISAs, for [`shell_navigation_tests`]'s reason: nothing here is
/// architecture-specific, so the parity gate (DECISIONS §19) is met by the same test running twice.
#[cfg(all(test, initrd))]
mod language_tests;

/// **The sink contract, and the one behaviour it changed** (milestone 50, notes/sink-protocol.md).
///
/// Two claims, one per test, and they need each other. The first is that a program cannot tell what
/// its output slot holds; the second is that when what it held is destroyed, the program finds out.
/// Without the second, "indifferent" would mean "unable to notice anything", which is a much
/// cheaper property and the wrong one.
///
/// Both run on both ISAs (§19), because the claim is about a contract and not about an instruction
/// set.
#[cfg(all(test, initrd))]
mod sink_tests;

/// **The raw-keystroke input primitive** (milestone 169): a real `line_editor` process, wired
/// exactly as the boot path wires it except that the test plays both the input driver and the
/// application, so `OP_RAWMODE` and `OP_READRAW` can be driven directly with real keystrokes.
#[cfg_attr(not(test), allow(dead_code))] // the milestone-169 raw-mode tests are its only caller
pub mod raw_mode_service;

/// **`OP_RAWMODE` and `OP_READRAW`, proved against a real `line_editor`** (milestone 169): echo
/// suppression, literal (uninterpreted) delivery of what the line discipline would otherwise
/// consume as an editing command, the two input models refusing each other, and a read parked
/// before data arrives still being answered once it does. See the module's own doc for why the
/// echo-suppression check is proven both ways rather than only the direction that matters.
#[cfg(all(test, initrd))]
mod raw_mode_tests;

/// **`rmle`'s wiring** (milestone 169): a real terminal ([`raw_mode_service`]'s own shape) and a
/// real filesystem ([`fs_service::narrow_dir`]'s shape) composed for the one program in this tree
/// that needs both at once.
#[cfg_attr(not(test), allow(dead_code))] // the milestone-169 rmle tests are its only caller
pub mod rmle_service;

/// **`rmle` itself**: open a file, move a cursor, insert and delete characters, save. Driven with
/// real keystrokes over the raw-keystroke primitive, and the saved file verified independently of
/// `rmle`'s own report. See the module's own doc for why that independence matters.
#[cfg(all(test, initrd))]
mod rmle_tests;

/// **No test may leak a runnable thread** (the regression proxy for the test-thread starvation that
/// made the RedoxFS mount overrun the hang watchdog under the net boot). A one-shot driver that
/// spins forever instead of exiting stays `Ready`/`Running` for the rest of the boot; enough of them
/// crammed onto core 0 (the scheduler places every spawn and wake on the current core, DECISIONS
/// "Open design ideas": the SMP placement gap) starve a later heavy test past the 60 s watchdog.
///
/// It quiesces first (yielding lets a just-finished thread be reaped by the next context switch),
/// then asserts nothing but the idle threads and this probe is still runnable. A leak fails here with
/// the offending thread in the dump, on the test that leaked's own turf, rather than as a mysterious
/// watchdog trip three tests later.
///
/// **The name is what makes this run last, not its position in the file**, and getting that wrong is
/// how the module spent many milestones never policing the one place that needed it. Tests run in
/// link order, which is alphabetical by module path, so being the last thing in the file bought
/// nothing: as `no_leaked_threads` it sorted before `tests`, and `kernel::user::tests` is precisely
/// the module whose whole subject is user threads. Measured on 2026-08-02, the probe ran 158 test
/// lines before the last test it was supposed to police.
///
/// So it is named to sort after `tests`, and the tree's own word for it (`notes/riscv-parity-scope.md`
/// calls this the leak police) is the name.
///
/// # BUGS
///
/// The ordering is still only alphabetical. A future `kernel::user` module sorting after
/// `thread_leak_police` would run after the probe and could leak unpoliced, silently, exactly as
/// `tests` did. Nothing enforces this; there is no "run me last" attribute in
/// `custom_test_frameworks`. If that happens, the symptom will again be a starvation watchdog
/// somewhere unrelated rather than a failure here.
#[cfg(test)]
mod thread_leak_police;

/// **Revocation against a capability that is in flight** (risk 7's adversarial pass, 2026-09-21).
///
/// Every revocation sweep in this kernel walks capability tables. A capability handed to a
/// rendezvous nobody is receiving on yet sits in `Thread::outgoing_cap` instead, which no sweep but
/// `sched::delete_reply_caps_naming` reads. The module's own header has the reasoning and the
/// `BUGS`; it is here rather than in [`tests`] because that file is this tree's worst merge hotspot.
///
/// Cross-ISA: `outgoing_cap`, the sweeps and the rendezvous are portable scheduler code, so the
/// parity gate (DECISIONS §19, architectural parity is a tenet) is met by the same test running on
/// each architecture.
#[cfg(test)]
mod revocation_in_flight_tests;

/// **Revocation against a mapping the kernel wired** (the `map_physical` mapping record,
/// 2026-09-21).
///
/// Every unmap sweep in `crate::revoke` is driven by the mapping log, and `AddressSpace::map_physical`
/// filed nothing in it, so a `Spawn::maps` entry survived a revoke of its own frame. The module's
/// own header has the reasoning, the reachable boot path and the `BUGS`; it is here rather than in
/// [`tests`] because that file is this tree's worst merge hotspot, and it is named to sort before
/// [`thread_leak_police`] for that module's own reason.
///
/// Cross-ISA: the mapping log, the sweeps and `map_physical` are portable kernel code, so the
/// parity gate (DECISIONS §19, architectural parity is a tenet) is met by the same test running on
/// each architecture.
#[cfg(test)]
mod spawn_mapping_revocation_tests;
