//! **The kernel's own object budget** (milestone 19c.1; design/init-and-granular-spawn.md).
//!
//! One untyped region, carved once at first use, that every kernel-owned object draws from:
//! kernel endpoints (moved here from `IpcTables`'s private region), kernel thread stacks
//! (which used to draw open-endedly from the frame allocator), and, with 19c.2, the kernel's
//! own TCB pages. This is the milestone 14 thesis extended to its last uncovered corner: after
//! this carve, **the kernel cannot spend beyond it, structurally**. The frame allocator's count
//! does not move when kernel threads come and go; a steady-state test asserts exactly zero.
//!
//! # Recycling, and why it is not an allocator
//!
//! Region pages are watermark-carved and never individually returned (untyped.rs). Kernel
//! threads churn constantly (every test spawns some), so spend-only pages would exhaust any
//! carve. The fix is a fixed free-stack of dead object pages, reused page-for-page: a reaped
//! thread's stack pages and (19c.2) TCB page go on it, the next spawn takes them back. This is
//! page-granular, bounded by the carve, FIFO-free, and deliberately not a general allocator:
//! one size (a page), one owner (the kernel), no headers, no coalescing, nothing to verify
//! beyond a bounded stack. The distinction between this and the heap milestone 14 deleted is
//! the distinction between a budget and a market.
//!
//! Creator-paid, resolved (the 19c conversation): a kernel-created thread's stack comes from
//! this carve because **the kernel is its creator**; a progenitor-created TCB's memory comes from
//! the progenitor's untyped because the progenitor is. One principle, two payers, no third regime.

use page_frames::FRAME_SIZE;

use crate::sync::{IrqSafeMutex, rank};

/// The carve: 2048 pages (8 MiB). Endpoints (one page each, ~60 across a full test run), live
/// kernel stacks (6 pages x up to `MAX_THREADS`; see thread.rs `STACK_PAGES`), TCB pages (one
/// each, 19c.2), and slack. Exhaustion fails the spawn or create cleanly, the same contract as
/// every budget since milestone 11; raising this constant is the fix if the image ever
/// legitimately needs more, and there have now been two such days.
///
/// 2026-08-15: this was 768 when stacks were 4 pages (512 of 768 budgeted for stacks, the same
/// two-to-one ratio kept since), and the 24 KiB stacks made stacks alone outgrow the whole carve.
/// The symptom was `kmem::page()` returning `None` late in the aarch64 suite, surfacing as an
/// unrelated test's spawn failing with "no memory to wire one", exactly the failure shape this
/// module's budget contract promises (clean refusal, far from the cause).
///
/// 2026-08-27: `sched::MAX_THREADS` doubled to 256 on a measured peak (its own doc comment carries
/// the numbers), and this constant is the reason that raise is not free. A thread costs **7 pages
/// here**, six of stack and one of TCB, so 256 of them is 1792, endpoints are ~60, and 2048 leaves
/// 196 of slack. That is the whole arithmetic, and it is deliberately written as a sum rather than
/// as a ratio to the old number: a carve sized by doubling would have been a guess, and a carve
/// left at 1024 would have moved the ceiling from the thread table down to here, one level lower
/// and no easier to diagnose.
const KERNEL_OBJ_PAGES: u64 = 2048;

struct Pool {
    /// The region id, once carved. Lazy: the first object need triggers the carve, which is
    /// early (the idle thread's stack) and cannot fragment-fail at that point.
    region: Option<u64>,
    /// Dead object pages, ready for reuse. Bounded by the carve itself.
    free: [u64; KERNEL_OBJ_PAGES as usize],
    free_len: usize,
}

/// Rank note: taken alone or under `IPC_TABLES` (`create_rendezvous` holds `IPC_TABLES`, then this, then the
/// region lock: 60 -> 59 -> 58, a legal descent). Shares 59 with INBOX and MAPPINGS, with which
/// it is never nested: inboxes are scheduling traffic, the mapping registry is user-space
/// bookkeeping, and this is the kernel shopping for its own pages.
static POOL: IrqSafeMutex<Pool> = IrqSafeMutex::new(
    rank::KMEM,
    Pool {
        region: None,
        free: [0; KERNEL_OBJ_PAGES as usize],
        free_len: 0,
    },
);

/// One page of the kernel's budget, zeroed, recycled if a dead object's page is waiting, carved
/// from the region otherwise. `None` when the carve is exhausted (raise `KERNEL_OBJ_PAGES`).
pub fn page() -> Option<u64> {
    let phys = {
        let mut pool = POOL.lock();
        if pool.free_len > 0 {
            pool.free_len -= 1;
            let phys = pool.free[pool.free_len];
            drop(pool);
            // Recycled pages carry the dead object's bytes; zero for parity with retype, so a
            // recycled page is indistinguishable from a fresh one to whatever it becomes next.
            // SAFETY: the page is the pool's own, off every live structure, direct-mapped.
            unsafe {
                core::ptr::write_bytes(
                    crate::arch::mmu::phys_to_virt(phys) as *mut u8,
                    0,
                    FRAME_SIZE as usize,
                );
            }
            return Some(phys);
        }
        let region = match pool.region {
            Some(r) => r,
            None => {
                let Some(r) = crate::memory_region::create(KERNEL_OBJ_PAGES) else {
                    crate::println!("kmem: the {KERNEL_OBJ_PAGES}-page carve itself was refused");
                    return None;
                };
                pool.region = Some(r);
                r
            }
        };
        drop(pool);
        // Pin-and-carve, like every object page since 19a: this region hosts kernel objects for
        // the machine's lifetime, and nothing may ever destroy it.
        let Some(phys) = crate::memory_region::retype_object_page(
            region,
            crate::memory_region::ObjectKind::KernelPool,
        ) else {
            crate::println!(
                "kmem: carve exhausted ({KERNEL_OBJ_PAGES} pages spent, none recycled); raise \
                 KERNEL_OBJ_PAGES"
            );
            return None;
        };
        phys
    };
    Some(phys)
}

/// A dead object's page comes home. The caller guarantees nothing references it any more (the
/// object was dropped in place, its mappings unmapped); the next `page()` may hand it out.
pub fn recycle(phys: u64) {
    let mut pool = POOL.lock();
    debug_assert!(
        pool.free_len < pool.free.len(),
        "more recycled pages than the carve holds"
    );
    if pool.free_len < pool.free.len() {
        let n = pool.free_len;
        pool.free[n] = phys;
        pool.free_len += 1;
    } // else: leak rather than corrupt; unreachable per the bound above
}
