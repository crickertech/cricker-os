//! Capability revocation and untyped reclamation (milestone 13, DECISIONS §13; storage reworked
//! at milestone 14 phase C).
//!
//! Until now a granted capability could not be retracted and a spent page could not be reclaimed.
//! That was safe only by a structural accident: retyped frames are **spend-only, never reused**
//! (untyped.rs), so a peer that still mapped a shared frame after the granter left was mapping
//! valid, non-reused memory. `memory_region::destroy` carried a tripwire saying exactly this: wiring up
//! any reclamation before revocation exists turns those "harmless" dangling mappings into a
//! use-after-free.
//!
//! This module is that revocation. It keeps a **mapping database, lite**: every mapping of an
//! untyped-derived page, and since §132 the object each mapping was made under.
//!
//! # Two scopes, because two questions are being asked
//!
//! **Reclamation asks "is this page safe to hand out again", so it is object-blind.**
//! `memory_region::destroy` unmaps a page from *every* address space that held it and deletes every
//! capability whose run overlaps the range, after which no holder maps it and no capability names
//! it. Anything less is §13's use-after-free, because the allocator is about to give the memory to
//! somebody else.
//!
//! **`PageFrame::REVOKE` asks "whose authority is being taken back", so it is capability-scoped**
//! (DECISIONS §132, option C, decided 2026-08-27). It reclaims nothing (a region is spend-only), so
//! a mapping made under a *different* capability over the same physical memory is somebody else's
//! authority and survives. §102 is what made those two answers diverge: once one capability can name
//! a run, two capabilities can overlap, and the tree's display wiring has three that do.
//!
//! seL4 keeps a full capability-derivation tree and revokes a *subtree*. This keeps no tree: the
//! object address in each record stands in for one, because `derive` never changes the object, so
//! one word matches a capability and all of its derivatives.
//!
//! # Who pays for the records (phase C, the heap's last customer)
//!
//! The database used to be a global `Vec`: one entry per user mapping, growing without bound, on
//! the kernel's heap. **Now the mapper pays.** Each address space's records live in log pages
//! retyped from *its own* untyped region, reached through a fixed registry of live spaces (root,
//! region, log head). A process that maps a thousand shared pages spends its own budget recording
//! them; a process that cannot afford the record cannot make the mapping. And teardown got
//! simpler, not more complex: the log pages are region pages, so `memory_region::destroy` reclaims the
//! records with the process, and "forget this root" is one registry slot going empty.

use crate::arch::mmu;
use crate::sync::{IrqSafeMutex, rank};

/// One recorded mapping: `va` in the owning space maps `phys`, **under the capability whose run
/// starts at `object`**. `phys == 0` is a tombstone (RAM starts at `0x4000_0000` on this board, so
/// no real frame is 0).
///
/// # Why the third word exists (DECISIONS §132, option C)
///
/// Without it a revocation cannot tell *which* capability produced a mapping, so `PageFrame::REVOKE`
/// had to unmap the physical page from every space that held it, whoever mapped it and under
/// whatever authority. §102 made that visible by letting one capability name a run: the tree's own
/// display wiring has a driver holding `PageFrame(dma, 312)` and two clients holding
/// `PageFrame(dma + 4096, 311)` over the same memory, so a client's revoke reached into the
/// driver's space under a capability nobody revoked.
///
/// **The run's base address identifies the capability, derivatives included**, and that is the
/// whole reason one word is enough: `Cap::derive` narrows rights and never changes the object, so
/// matching on the object matches the entire derivation family without the §13 derivation tree this
/// kernel deferred.
///
/// **A mapping made with no `PageFrame` capability at all records itself as its own object**
/// (`object == phys`): `MemoryRegion::MAP` retypes a page and maps it in one step, and there is
/// never a capability to name. That is the honest encoding rather than a sentinel, because the page
/// really is the whole of what was mapped.
///
/// # BUGS
///
/// **Two capabilities sharing a base but not a length are one object here.** `PageFrame(p, 401)`
/// and `PageFrame(p, 74)` are distinct objects to `Cap`, and a revoke of the shorter one unmaps the
/// longer one's mappings of the pages they share. Carrying the length too would cost a fourth word
/// and take `LOG_ENTRIES` from 170 to 127 (a `LogEntry` would round up to 32 bytes), which is twice
/// the log pages a space pays now rather than 1.5x, to separate a pair nothing in the tree mints.
/// The failure is bounded in the safe direction: the over-broad unmap can only reach pages inside
/// the revoked run, and it is strictly narrower than the space-blind unmap this replaced. Named
/// here because a reader who trusts "capability-scoped" without qualification would be wrong.
#[repr(C)]
#[derive(Clone, Copy)]
struct LogEntry {
    phys: u64,
    va: u64,
    /// The base address of the run named by the capability this mapping was made under.
    object: u64,
}

/// How many entries fit a log page after its header.
///
/// **Fell from 255 to 170 when [`LogEntry`] grew its third word** (§132 option C, 2026-08-27):
/// `16 + 24 * 170 == 4096` exactly, and the `LogPage` size assertion below is what keeps that
/// arithmetic honest rather than a comment claiming it.
///
/// **This number is a benchmark input, which is not obvious from here.** [`record_mapping`] scans
/// the head page's used slots for a tombstone before it appends, so the average scan is half of
/// this constant and the whole cost of recording a mapping is linear in it. Lowering it to 170 took
/// `map_el0` down 16.9% on aarch64 (464,182 -> 385,919) and 16.4% on riscv64 (73,990 -> 61,868),
/// measured by changing this one number on `main` and nothing else: 383,325, which is the branch's
/// figure to within 0.7%. Nothing was skipped to earn it; a shorter search for a free slot is the
/// same search over a smaller page. It is paid for on the other side, in the space's own region
/// budget, which now funds 1.5x the log pages for the same number of mappings ([`LogEntry`]'s
/// `BUGS` prices the next word on that same scale). Whoever moves this constant again moves both
/// benchmarks with it and should expect to re-record them.
const LOG_ENTRIES: usize = 170;

/// **How many log pages `n` recorded mappings cost**, for a caller sizing an address space's
/// backing region.
///
/// Every mapping is recorded ([`record_mapping`]), and the record is paid for out of the mapped
/// space's own region, so a caller that carves a budget and then maps a large window has to pay for
/// the window's records as well as for the page tables reaching it. That was free until 2026-09-21,
/// because [`crate::user::AddressSpace::map_physical`] recorded nothing; it is not free now, and
/// the callers that map a whole initrd (thousands of pages) are the ones where the difference is
/// visible rather than lost in `AS_OVERHEAD`'s slack.
///
/// Exported rather than left as arithmetic at each call site because `LOG_ENTRIES` is this
/// module's own business: a caller that spelled `n / 170` would be a copy of a constant that moves
/// (its own doc prices a fourth `LogEntry` word at 127 entries per page).
pub fn log_pages_for(n: u64) -> u64 {
    n.div_ceil(LOG_ENTRIES as u64)
}

/// **What authority a mapping was made under**, which is the question [`record_mapping`] now
/// requires an answer to (DECISIONS §132).
///
/// A two-variant enum rather than a bare address, for the ladder's own reason (AGENTS.md, rung
/// one): twelve of this kernel's fourteen mapping sites map a page no capability names, and passing
/// the page's own address as "the object" a second time would read as a duplicated argument rather
/// than as the decision it is. The variant makes the case explicit at every call site and makes the
/// wrong one hard to write by accident. It is the same move `InputSpec::Required` made when it
/// stopped being a unit variant.
///
/// Name: ratified 2026-08-27 (calef). Landed as `MappedUnder`; renamed to `PageMapSource` on
/// request, since "mapping" alone is overloaded across this tree (page mappings, capability
/// derivation, filesystem grants) and this tree already has an established, narrower term for
/// exactly this concept: `crates/pmap`, "page map." `Source` matches this tree's existing pattern
/// for a small enum naming where something came from (`grant_plan::Source`).
#[derive(Clone, Copy)]
pub enum PageMapSource {
    /// The capability whose object begins at this physical address, **and every capability derived
    /// from it**: `Cap::derive` narrows rights and never changes the object, so one address names
    /// the whole family. For a `PageFrame` run this is the run's base, not the page being mapped.
    Capability(u64),
    /// No capability names this page. `MemoryRegion::MAP` retypes and maps in one step, and the
    /// kernel's own loaders map pages straight into a space they are building; in both cases there
    /// is no derivation family for a revoke to be scoped to, so the page stands as its own object.
    NoCapability,
}

impl PageMapSource {
    /// The object address to file with the record: the run's base, or the mapped page itself when
    /// nothing names it.
    fn object(self, phys: u64) -> u64 {
        match self {
            PageMapSource::Capability(base) => base,
            PageMapSource::NoCapability => phys,
        }
    }
}

/// One page of mapping records, retyped from the owning space's region. Exactly one frame.
#[repr(C)]
struct LogPage {
    /// Physical address of the next (older) log page in this space's chain; 0 ends it.
    next: u64,
    /// High-water mark of entries ever written here. Slots below it may be tombstones.
    used: u64,
    entries: [LogEntry; LOG_ENTRIES],
}

const _: () = assert!(size_of::<LogPage>() == page_frames::FRAME_SIZE as usize);

/// A live address space, as revocation sees it: where its tables root, which region pays for its
/// records, and its newest log page.
struct SpaceLog {
    root: u64,
    region: u64,
    /// Physical address of the newest log page; 0 until the first record needs one.
    head: u64,
}

/// The most concurrently-live address spaces the registry can track: every user thread has one
/// (bounded by [`crate::sched::MAX_THREADS`]), plus headroom for the tests' bare `AddressSpace`s.
///
/// **Written as arithmetic on that constant rather than as the literal 160 it was until
/// 2026-08-27**, when the thread ceiling doubled and this was one of four numbers that had to
/// move with it. The 32 is the same headroom the old literal carried (160 - 128); what changed is
/// that the relationship is now in the code instead of only in this sentence.
const MAX_SPACES: usize = crate::sched::MAX_THREADS + 32;

/// **The registry of live address spaces.** Fixed (milestone 14 phase C): the records themselves
/// live in the spaces' own regions, so this is just the index that finds them, bounded by how
/// many spaces can exist at once.
static SPACES: IrqSafeMutex<Registry> = IrqSafeMutex::new(rank::MAPPINGS, Registry::new());

/// The registry array plus **one past its highest live slot**, for the same reason
/// `generational_table::Table` carries one: without it every walk here costs [`MAX_SPACES`], and
/// that constant is derived from `sched::MAX_THREADS`, so raising the thread ceiling made address
/// space creation and teardown slower for slots nothing occupied.
///
/// Measured rather than assumed. Doubling `MAX_THREADS` from 128 to 256 cost the `spawn_el0`
/// icount benchmark **+138,584 ticks** through this file, almost exactly what the capability sweep
/// in `sched` cost through the other half of the same regression, and `forget_root` is the worst
/// of the sites because it scanned the whole array unconditionally on every `AddressSpace::drop`.
/// See notes/benchmarks.md and `sched::MAX_THREADS`'s own ledger.
///
/// `claim` is first-fit, so occupancy packs toward slot 0 and `top` stays near the live count.
/// Every slot at or above `top` is `None`, which is what makes a bounded walk identical to a full
/// one.
struct Registry {
    spaces: [Option<SpaceLog>; MAX_SPACES],
    top: usize,
}

impl Registry {
    const fn new() -> Self {
        Self {
            spaces: [const { None }; MAX_SPACES],
            top: 0,
        }
    }

    /// Every live space. The `flatten` still skips holes; the bound is what stops the walk from
    /// visiting slots that have never been occupied at all.
    fn live(&self) -> impl Iterator<Item = &SpaceLog> + '_ {
        self.spaces[..self.top].iter().flatten()
    }

    /// [`live`](Self::live), mutably.
    fn live_mut(&mut self) -> impl Iterator<Item = &mut SpaceLog> + '_ {
        self.spaces[..self.top].iter_mut().flatten()
    }

    /// Take the first free slot, growing the bound if it is past the end of the occupied prefix.
    /// `false` (and nothing stored) when the registry is full, which is `register_space`'s own
    /// "the caller should fail creation" answer.
    fn claim(&mut self, log: SpaceLog) -> bool {
        let Some(slot) = self.spaces.iter().position(|s| s.is_none()) else {
            return false;
        };
        self.spaces[slot] = Some(log);
        if slot >= self.top {
            self.top = slot + 1;
        }
        true
    }

    /// Drop every entry naming `root` and bring the bound back down over whatever emptiness that
    /// left at the top. The downward walk stops at the first live slot and can only step over a
    /// slot that was filled and freed, so it is amortised O(1).
    fn release(&mut self, root: u64) {
        for slot in self.spaces[..self.top].iter_mut() {
            if slot.as_ref().is_some_and(|s| s.root == root) {
                *slot = None;
            }
        }
        // Bounded by the array's own length, the same shape and for the same reason as
        // `generational_table::Table::remove`: the bound the type guarantees, written where a
        // reader (and a prover) can see it. The `break` keeps the runtime cost amortised O(1).
        let mut top = self.top;
        for _ in 0..MAX_SPACES {
            if top == 0 || self.spaces[top - 1].is_some() {
                break;
            }
            top -= 1;
        }
        self.top = top;
    }
}

/// A log page, by physical address, through the direct map.
///
/// # Safety
/// `phys` must be a page this module linked into some space's chain (retyped exclusively for the
/// log), and the caller must hold the `SPACES` lock, which serializes every touch of every chain.
unsafe fn log_page(phys: u64) -> &'static mut LogPage {
    // SAFETY: per the function's contract; the direct map names every RAM page.
    unsafe { &mut *(mmu::phys_to_virt(phys) as *mut LogPage) }
}

/// Enter a newly created address space into the registry. `false` (and the caller should fail
/// creation) if the registry is full.
pub fn register_space(root: u64, region: u64) -> bool {
    let mut spaces = SPACES.lock();
    spaces.claim(SpaceLog {
        root,
        region,
        head: 0,
    })
}

/// Forget an address space. Called from `AddressSpace::drop` **before** its region is destroyed:
/// its page tables and its log pages are about to be freed, and a stale registry entry would send
/// a later revoke walking memory that belongs to someone else. The records need no cleanup of
/// their own; they are region pages, and the region is about to come back whole.
pub fn forget_root(root: u64) {
    let mut spaces = SPACES.lock();
    spaces.release(root);
}

/// Record that the address space rooted at `root` mapped `phys` at `va`, **under the capability
/// whose run begins at `object`**, and **paid for by that space's own region**: the record goes in
/// an existing log slot, or a fresh log page is retyped from the region (rank MAPPINGS >
/// `MEMORY_REGION` makes that legal under this lock). Returns `false` if
/// the space is unknown or its budget is exhausted, and the caller must then unmap what it just
/// mapped: an unrecorded mapping is invisible to revocation, which is the §13 use-after-free.
///
/// `under` is a required argument with no default, which is the point (AGENTS.md's ladder, rung
/// one): a mapping that cannot say which capability made it is exactly the record §132 found
/// missing, and a caller must now answer the question to compile. See [`PageMapSource`].
#[must_use]
pub fn record_mapping(phys: u64, root: u64, va: u64, under: PageMapSource) -> bool {
    let object = under.object(phys);
    // The run's base is at or below the page it covers, and a whole number of pages below it. The
    // enum stops a caller confusing the two arguments; this catches a caller computing the base
    // wrongly, which would silently scope the record to a family it does not belong to.
    debug_assert!(
        object <= phys && (phys - object).is_multiple_of(page_frames::FRAME_SIZE),
        "a mapping of {phys:#x} recorded under an object at {object:#x}: not a page of that run",
    );
    let mut spaces = SPACES.lock();
    let Some(space) = spaces.live_mut().find(|s| s.root == root) else {
        return false;
    };

    // A free slot in the chain: the first tombstone, or headroom in any page.
    let mut page_phys = space.head;
    while page_phys != 0 {
        // SAFETY: pages in the chain are the log's own; SPACES is held.
        let page = unsafe { log_page(page_phys) };
        for e in page.entries.iter_mut().take(page.used as usize) {
            if e.phys == 0 {
                *e = LogEntry { phys, va, object };
                return true;
            }
        }
        if (page.used as usize) < LOG_ENTRIES {
            page.entries[page.used as usize] = LogEntry { phys, va, object };
            page.used += 1;
            return true;
        }
        page_phys = page.next;
    }

    // No room anywhere: a fresh page from the space's own budget becomes the new head. Retyped
    // zeroed, so `used = 0` and `next = 0` need no separate scrub.
    let Some(fresh) = crate::memory_region::retype_page(space.region) else {
        return false; // out of budget: the caller unmaps, the process pays for its own limit
    };
    // SAFETY: just retyped exclusively for the log; SPACES is held.
    let page = unsafe { log_page(fresh) };
    page.next = space.head;
    page.entries[0] = LogEntry { phys, va, object };
    page.used = 1;
    space.head = fresh;
    true
}

/// **Undo one [`record_mapping`]**: tombstone the record that `root` maps `phys` at `va`, without
/// touching any other space's view of `phys`.
///
/// The counterpart the rollback paths needed and did not have. [`unmap_everywhere`] is the wrong
/// tool for undoing a half-finished `PageFrame::MAP`, because it is space-blind by design: it pulls
/// the physical page out of *every* address space that maps it, which for a shared frame would
/// punish the peers for the mapper's failure. This removes exactly the one record the caller just
/// wrote, and the caller unmaps exactly the one page it just mapped.
///
/// Silent when the space or the record is unknown, because both mean the same thing to a rollback:
/// there is nothing left to undo.
pub fn forget_mapping(phys: u64, root: u64, va: u64) {
    let mut spaces = SPACES.lock();
    let Some(space) = spaces.live_mut().find(|s| s.root == root) else {
        return;
    };
    let mut page_phys = space.head;
    while page_phys != 0 {
        // SAFETY: pages in the chain are the log's own; SPACES is held.
        let page = unsafe { log_page(page_phys) };
        for e in page.entries.iter_mut().take(page.used as usize) {
            if e.phys == phys && e.va == va {
                e.phys = 0; // tombstone: reusable by the next record, exactly as a revoke leaves it
                return;
            }
        }
        page_phys = page.next;
    }
}

/// **One entry of what `root` has mapped, resuming from `cursor`** (`abi::address_space::LIST`,
/// milestone 126's `pmap`, DECISIONS §114). `(0, 0)` means done, the same `abi::survey::DONE`
/// convention `SURVEY` uses on the endpoint side: start with `cursor = 0`, feed each returned
/// cursor back, stop when it comes back 0.
///
/// **Reads the space's own revocation log rather than walking page tables**, which is the same
/// move `ps` makes over `/proc`: the kernel already keeps this record for reclamation, so
/// answering "what is mapped" costs nothing the space did not already pay for, and the answer
/// cannot drift out of agreement with what `revoke_page_frame`/`revoke_region` would find. The caller
/// (`kernel::syscall`) turns each `va` this hands back into a `(phys, Flags)` with
/// `arch::mmu::translate_at`, which is where the permission bits `pmap` prints come from; this
/// function knows nothing about flags, on purpose, because the log does not record them.
///
/// **A tombstoned entry (`phys == 0`, an unmapped or revoked slot) is skipped silently**, and a
/// slot a later `record_mapping` reused for an unrelated mapping is not detected: unlike
/// `SURVEY`'s slot table, a log entry carries no generation, so a resumed cursor that outlives a
/// tombstone-then-reuse in the same slot can read the wrong mapping there. Recorded in
/// `crates/pmap`'s `BUGS`, because nothing in this module can tell the difference.
///
/// **Cursor encoding**: a log page's own physical address (always page-aligned, so its low 12
/// bits are free and never legitimately 0 -- RAM starts at `0x4000_0000` on this board, the same
/// fact [`LogEntry`]'s tombstone convention leans on) OR'd with the index into that page. Pages
/// are only ever *prepended* to a space's chain, never freed or reordered until the whole space
/// dies (`forget_root`), so a cursor this function handed back stays valid regardless of what
/// `record_mapping` does to the chain in between: a page prepended after a walk starts is simply
/// never reached by it (`SURVEY`'s "can miss a member born into an already-passed slot," one
/// object type over), and a page already visited is never revisited because pages are singly
/// linked toward *older* entries and a cursor only ever advances that way.
pub fn list_mapping(root: u64, cursor: u64) -> (u64, u64) {
    let spaces = SPACES.lock();
    let Some(space) = spaces.live().find(|s| s.root == root) else {
        // The space is gone (a race with teardown, or a stale cursor from a caller that kept one
        // past the space's life): nothing to report. Not a refusal; the syscall layer already
        // checked the capability before calling here.
        return (0, 0);
    };

    const PAGE_MASK: u64 = !(page_frames::FRAME_SIZE - 1);
    let (mut page_phys, mut index) = if cursor == 0 {
        (space.head, 0usize)
    } else {
        (cursor & PAGE_MASK, (cursor & !PAGE_MASK) as usize)
    };

    while page_phys != 0 {
        // SAFETY: `page_phys` is either this space's own `head` or a cursor this function minted
        // from a page in this space's chain; SPACES is held.
        let page = unsafe { log_page(page_phys) };
        while index < page.used as usize {
            let entry = page.entries[index];
            index += 1;
            if entry.phys != 0 {
                // `page_phys` is always nonzero (RAM starts at 0x4000_0000), so `page_phys |
                // index` can never collide with the `(0, 0)` DONE sentinel below, even for the
                // very last real entry in a space, where `index` has just walked off the end of
                // this page. The bug this replaced returned a bare `0` for exactly that case
                // (nothing left to point at), which a caller cannot tell apart from "this call
                // found nothing": the last real mapping in every space was silently dropped.
                // Pointing at the (now out-of-range) position instead costs one extra call --
                // the next one finds `index == page.used`, falls through to `page.next`, and
                // returns genuine `(0, 0)` if there is none -- and it is what keeps a hit's
                // `next` and the DONE sentinel from ever being the same value.
                return (page_phys | index as u64, entry.va);
            }
        }
        page_phys = page.next;
        index = 0;
    }
    (0, 0)
}

/// Unmap `phys` from every address space whose log records it, tombstoning the records.
///
/// The unmapping (TLB broadcast included) happens under the registry lock. The old database
/// lifted victims out first to keep the §9 critical section short; without a heap there is
/// nowhere to lift them to, and the honest accounting is: revocation is rare, the lock is
/// contended only by `record_mapping` (a syscall path that can afford to wait), and a `tlbi`
/// completes in hardware regardless of who spins on what.
///
/// `spare` is an address-space root to leave alone (0 spares none). Only the device take-back
/// below passes one: reclamation must unmap everywhere or the page is not safe to reuse, but
/// transferring a device wants the invoker to keep what it is about to hand on. See
/// [`revoke_device_from_others`].
///
/// **Deliberately object-blind, and that is not an oversight left over from §132.** Its two
/// remaining callers are reclamation ([`revoke_region`]) and the device take-back, and neither is
/// asking a capability's question. Reclamation is about to hand these pages back to an allocator,
/// so *any* surviving mapping is §13's use-after-free regardless of which capability made it; the
/// device take-back scopes by **holder** (§41), which is a different axis entirely. Capability
/// scope belongs to `PageFrame::REVOKE`, and that is [`unmap_under_object`].
fn unmap_everywhere(phys: u64, spare: u64) {
    unmap_matching(phys, spare, None);
}

/// Unmap `phys` **only from the mappings made under the capability whose run begins at `object`**,
/// its narrowed derivatives included, tombstoning those records and leaving every other holder's
/// mapping of the same physical page alone (DECISIONS §132, option C).
///
/// This is `PageFrame::REVOKE`'s unmap half. The question it answers is "what authority is being
/// taken back", not "is this page safe to reuse": `REVOKE` reclaims nothing (a region is
/// spend-only), so a mapping made under a *different* capability is somebody else's authority and
/// survives. Under the old space-blind sweep, revoking a client's 311-page surface pulled those
/// pages out of the gpu driver's address space too, under a `PageFrame(dma, 312)` nobody had
/// revoked.
fn unmap_under_object(phys: u64, object: u64) {
    unmap_matching(phys, 0, Some(object));
}

/// The body both sweeps share: unmap `phys` from every recorded mapping except those in `spare`'s
/// space and, when `object` is `Some`, except those made under a different capability. Split out
/// for the reason `sched::delete_page_frame_caps_where` is: the two policies differ by one
/// predicate, and one body is what keeps the locking, the tombstoning and the TLB broadcast the
/// same for both.
fn unmap_matching(phys: u64, spare: u64, object: Option<u64>) {
    let spaces = SPACES.lock();
    for space in spaces.live() {
        if space.root == spare {
            continue;
        }
        let mut page_phys = space.head;
        while page_phys != 0 {
            // SAFETY: chain pages under the held SPACES lock.
            let page = unsafe { log_page(page_phys) };
            for e in page.entries.iter_mut().take(page.used as usize) {
                if e.phys == phys && object.is_none_or(|o| e.object == o) {
                    mmu::unmap_user_at(space.root, e.va);
                    e.phys = 0; // tombstone: reusable by the next record
                }
            }
            page_phys = page.next;
        }
    }
}

/// **Revoke a single-page frame from everyone.** Delete every `PageFrame(phys, 1)` capability from
/// every capability table, then unmap `phys` from every address space. Caps go **first**, so a
/// `PageFrame::MAP` that starts after this cannot re-establish a mapping we would then miss. (The
/// remaining window, an in-flight map on another core between the cap delete and the unmap, is the
/// SMP race §13 names; a full mapping-database lock is seL4's answer and this milestone's
/// deferral.)
///
/// **This is not "no capability names the page afterwards", and the earlier wording that said so
/// was wrong from the day §102 landed.** The sweep is by exact object, so a `PageFrame(p, n)` run
/// that merely *contains* `phys` survives it. That is harmless where this function is used (a
/// driver taking one of its own kernel-minted pages back, the tests below) and would not be
/// harmless on a reclamation path, which is why reclamation does not use it:
/// [`revoke_region`] sweeps capabilities by overlap over the whole range instead.
///
/// **Nor is it "nothing maps the page afterwards"**, since §132: the unmap half is scoped to the
/// capability being revoked, so a mapping some *other* capability made of this same page survives.
/// For a one-page object those two cannot differ unless the pages were also named by a longer run
/// sharing this base; see [`LogEntry`]'s `BUGS`.
///
/// The single-page case of [`revoke_page_frame_run`], and now literally so. `PageFrame::REVOKE`'s
/// own syscall path uses that one, because there the object being revoked really is the run named
/// by the invoked capability.
///
/// **Test-only in a non-`initrd` build**, and it stopped being anything else when `revoke_region`
/// took its own range sweep: its remaining callers are the two `#[cfg(all(test, initrd))]` suites
/// that revoke a single kernel-minted page (`user::disk_tests`, `user::tests`) and the suite in
/// this file. Kept rather than deleted because those are the tests that prove the property, and
/// because it is what a caller wanting exactly one page should still reach for.
#[cfg_attr(not(all(test, initrd)), allow(dead_code))]
pub fn revoke_page_frame(phys: u64) {
    revoke_page_frame_run(phys, 1);
}

/// **Revoke a run of `count` frames from everyone** (DECISIONS §102). Deletes every capability
/// naming exactly the run `(phys, count)`, once, then unmaps each of the `count` physical pages from
/// every address space that mapped it. This is `PageFrame::REVOKE`'s body: the capability being
/// invoked names the whole run, so the whole run is what gets deleted and unmapped, in one syscall
/// regardless of how many pages the run holds.
///
/// The two passes are not the same granularity on purpose. Capability deletion is one exact-object
/// match against `(phys, count)`, because that is the one capability (and its narrowed derivatives)
/// this invocation could possibly be revoking. Unmapping stays per-page, because the mapping
/// database records one entry per mapped virtual page regardless of how long the run is
/// (`PageFrame::MAP` loops over the run and records each page individually); that granularity does
/// not change here.
///
/// **Both passes are scoped to the invoked capability's derivation family** (DECISIONS §132, option
/// C, decided 2026-08-27). The capability pass always was: `derive` narrows rights and never
/// changes the object, so exact-object equality *is* the family. The unmap pass now is too, and
/// that is what changed here: it takes only the mappings recorded under this run's base, so
/// revoking a client's `PageFrame(dma + 4096, 311)` no longer reaches into the gpu driver's space,
/// which holds `PageFrame(dma, 312)` over the same physical memory and was never revoked. What
/// makes one word enough to say "and its derivatives" is [`LogEntry`], which carries the reasoning
/// and the one case it cannot separate.
///
/// # BUGS
///
/// **A capability whose run overlaps this one is left holding authority over pages this call has
/// unmapped out of *its own* holders' spaces**, and that is now deliberate rather than a gap: §102
/// contemplates two capabilities coexisting over sub-ranges of one region, and letting a one-page
/// holder delete a 312-page capability by naming a page inside it is an authority a one-page
/// capability should not have (§132 option B, refused). The overlapping holder keeps both its
/// capability and its mappings; only what was mapped under *this* capability goes.
///
/// **Revocation still owes a device nothing** (§132's question 3, deliberately not answered by this
/// work). `PageFrame::REVOKE` is a CPU-side operation: the gpu driver registers its DMA window with
/// `virtio::register`, that window is not derived from any capability, and revoking one does not
/// narrow it. So a capability-perfect revocation of a surface leaves the device able to write those
/// pages until the driver's virtio registration is itself torn down. Coupling `PageFrame` and
/// `Virtio`, which are independent today, is a separate decision and remains an architect's call:
/// see design/decisions/132-overlapping-page-frame-runs.md.
pub fn revoke_page_frame_run(phys: u64, count: u64) {
    crate::sched::delete_page_frame_caps(phys, count);
    for k in 0..count {
        unmap_under_object(phys + k * page_frames::FRAME_SIZE, phys);
    }
}

/// **Take a device's registers back from everyone else** (milestone 23, DECISIONS §41). Delete
/// every `DeviceFrame` capability naming `phys` except the invoking thread's own, then unmap `phys`
/// from every address space except the invoker's. Afterwards exactly one process can reach the
/// device: the one that asked.
///
/// The asymmetry with [`revoke_page_frame`] is the point, not an oversight. Revoking a *frame* exists to
/// make reclamation safe, so the revoker's own capability and mapping must go too: the page is about
/// to be returned to the allocator and reused. A device page is never reclaimed; revoking it exists
/// to make ownership **exclusive**, which is what live replacement needs between tearing one driver
/// down and endowing the next. A take-back that also deleted the invoker's capability would leave
/// the registers unreachable forever, because only the kernel mints a `DeviceFrame` and it does so
/// once, at boot.
///
/// This is one level of the capability-derivation tree §13 deferred, and only one: the invoker is
/// the root by construction (it holds `GRANT` and it is the one asking), and every other holder is
/// treated as a derivative. Revoking one *named* holder while sparing another still wants the real
/// tree, and still is not built.
///
/// **§132 left this alone on purpose**, and the reason is that it scopes by a different thing.
/// Capability scope answers "whose authority is being taken back"; this answers "who is allowed to
/// keep reaching the registers", and the answer is one *holder*, not one capability. Scoping the
/// unmap to an object here would spare a second capability to the same MMIO page, which is exactly
/// the outcome a live replacement must not have: after this call the device has one owner. So it
/// keeps [`unmap_everywhere`], and the swap suite (`user::live_swap_tests`, `LOG_REVOKE_ENFORCED`)
/// is the end-to-end evidence that the behaviour did not move.
pub fn revoke_device_from_others(phys: u64) {
    crate::sched::delete_device_frame_caps_from_others(phys);
    // The invoker is the current thread, so its address space is the one installed in TTBR0 (satp
    // on RISC-V) right now: no lookup, and no way for the spare to name someone else's space.
    unmap_everywhere(phys, mmu::current_user_root());
}

/// **Take an x86 port range back from every other thread** (milestone 299, DECISIONS §121 reversed
/// 2026-09-15). The port analogue of [`revoke_device_from_others`]: a port range has no page to
/// unmap, so revocation is entirely capability deletion plus forgetting the cached grant, which
/// [`crate::sched::delete_port_range_caps_from_others`] does, and the TSS reset it triggers is what
/// makes the revoked holder fault on its next `in`/`out`. This is `PortRange::REVOKE`'s body.
/// `x86_64` only, like the `PortRange` object it revokes ([`crate::cap::Object`]).
#[cfg(target_arch = "x86_64")]
pub fn revoke_port_range_from_others(base: u16, count: u16) {
    crate::sched::delete_port_range_caps_from_others(base, count);
}

/// **Take an x86 port range back from everyone.** The whole-machine sweep [`revoke_page_frame`] is
/// to a frame: it deletes the capability from the invoker too, which no live-replacement path wants
/// but the kernel's own port-capability tests do (they grant a range to a child, prove it can `out`,
/// revoke, and prove it faults). Test-only, like its frame twin. `x86_64` only.
#[cfg(target_arch = "x86_64")]
#[cfg_attr(not(all(test, initrd)), allow(dead_code))]
pub fn revoke_port_range(base: u16, count: u16) {
    crate::sched::delete_port_range_caps(base, count);
}

/// Revoke every page in `[base, base + size)`. `memory_region::destroy` calls this before
/// returning a region to the allocator, which is what turns the old "spend-only, never reused"
/// invariant into the stronger "no live mapping survives" one that makes reuse actually safe.
///
/// **Two passes, at two granularities, and the split is the point.** The capability sweep is over
/// the whole range **once**, up front, and it matches by overlap rather than by object equality
/// ([`crate::sched::delete_page_frame_caps_overlapping`], which carries the reasoning). The unmap
/// sweep stays per-page and mapping-log-driven, because the log is per-page and says nothing about
/// which capability produced an entry.
///
/// Doing capabilities first, for the whole range, is the same ordering [`revoke_page_frame`]
/// documents one function up, applied at range scale: a `PageFrame::MAP` that starts after the
/// sweep has no capability left to map with, so it cannot re-establish a mapping the unmap pass
/// would then miss.
///
/// It also fixes what the per-page version could not see. That version deleted capabilities one
/// *mapped* page at a time, so a capability naming a page nobody had mapped was never a candidate:
/// the log had no record to find it by. A range sweep does not consult the log, so a
/// retyped-but-never-mapped frame in a destroyed region loses its capability too.
///
/// The unmap pass is one page per iteration: find a recorded page in range under the registry lock,
/// release it, then unmap (unmapping retakes the registry lock, so it cannot be called while it is
/// held). Each pass tombstones every record of its page, so the scan strictly shrinks and
/// terminates.
pub fn revoke_region(base: u64, size: u64) {
    crate::sched::delete_page_frame_caps_overlapping(base, size);
    loop {
        let victim = {
            let spaces = SPACES.lock();
            let mut found = None;
            'scan: for space in spaces.live() {
                let mut page_phys = space.head;
                while page_phys != 0 {
                    // SAFETY: chain pages under the held SPACES lock.
                    let page = unsafe { log_page(page_phys) };
                    for e in page.entries.iter().take(page.used as usize) {
                        if e.phys >= base && e.phys < base + size {
                            found = Some(e.phys);
                            break 'scan;
                        }
                    }
                    page_phys = page.next;
                }
            }
            found
        };
        match victim {
            // Unmap only: the capability sweep above already covered the whole range, and calling
            // `revoke_page_frame` here would re-run an exact-match sweep per page that by
            // construction can no longer find anything.
            Some(phys) => unmap_everywhere(phys, 0),
            None => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use paging::Flags;

    use super::*;
    use crate::cap::{Rights, page_frame_run_cap, page_frame_run_len};
    use crate::user::AddressSpace;

    /// `N` consecutive pages out of one region. A region is a contiguous span and `retype_page`
    /// bumps through it, so this holds; it is asserted rather than assumed because the run tests
    /// below are meaningless if it ever stops holding.
    fn consecutive<const N: usize>(region: u64) -> [u64; N] {
        let mut pages = [0u64; N];
        for (k, page) in pages.iter_mut().enumerate() {
            *page = crate::memory_region::retype_page(region)
                .unwrap_or_else(|| panic!("region ran out at page {k} of {N}"));
        }
        for k in 1..N {
            assert_eq!(
                pages[k],
                pages[k - 1] + page_frames::FRAME_SIZE,
                "a region stopped retyping contiguously at page {k}; a PageFrame run has no \
                 meaning without that",
            );
        }
        pages
    }

    /// **A multi-page `REVOKE` unmaps every page of the run, in every address space, and deletes
    /// the capability that named it** (DECISIONS §102, milestone 142).
    ///
    /// The property §102 asserted and nothing tested: before this, every revocation test drove the
    /// `count: 1` path, so "the run" and "the page" were the same thing and a loop bound could have
    /// been wrong in either direction without a failure. Two spaces hold different pages of one
    /// three-page run at different virtual addresses, which is also what makes the space-blind
    /// unmap visible: the run is revoked once and both spaces lose their page.
    #[test_case]
    fn revoking_a_run_unmaps_every_page_of_it_everywhere() {
        let mut a = AddressSpace::new(2).expect("no space A");
        let mut b = AddressSpace::new(2).expect("no space B");
        let region = crate::memory_region::create(4).expect("no region");
        let run = consecutive::<3>(region);

        // A maps the first two pages; B maps the third, at an unrelated VA. Nothing about the
        // revocation may depend on where a holder put the run.
        let (va_a, va_b) = (0x40_0000u64, 0x80_0000u64);
        for (k, phys) in run[..2].iter().enumerate() {
            let va = va_a + k as u64 * page_frames::FRAME_SIZE;
            a.map_physical(
                va,
                *phys,
                Flags::user_data(),
                PageMapSource::Capability(run[0]),
            )
            .expect("map A");
        }
        b.map_physical(
            va_b,
            run[2],
            Flags::user_rodata(),
            PageMapSource::Capability(run[0]),
        )
        .expect("map B");

        let slot = crate::sched::grant(page_frame_run_cap(
            run[0],
            page_frame_run_len(3),
            Rights::ALL,
        ))
        .expect("grant the run");

        revoke_page_frame_run(run[0], 3);

        for k in 0..2u64 {
            assert!(
                mmu::translate_at(a.root(), va_a + k * page_frames::FRAME_SIZE).is_none(),
                "page {k} of the run survived the revoke in A",
            );
        }
        assert!(
            mmu::translate_at(b.root(), va_b).is_none(),
            "the run's last page survived the revoke in B",
        );
        assert!(
            crate::sched::current_cap(slot).is_err(),
            "REVOKE left the capability that named the run in the revoker's own table",
        );

        crate::memory_region::destroy(region);
    }

    /// **`REVOKE` takes back what was mapped under the capability invoked, and nothing else**
    /// (DECISIONS §132, option C, decided 2026-08-27).
    ///
    /// The shape is the tree's own display wiring, shrunk to four pages: one holder's capability
    /// spans the whole window (`PageFrame(run[0], 4)`, what a gpu driver registers with the IOMMU)
    /// and another's starts one page in (`PageFrame(run[1], 3)`, the surface a client paints). They
    /// are different objects over overlapping memory, which is legal and, on that path, required.
    ///
    /// Both spaces map the **same physical page** at their own addresses under their own
    /// capabilities, and that page is the whole test. Before this, `REVOKE`'s unmap half was
    /// space-blind: the client handing its surface back pulled the page out of the driver's address
    /// space too, under a capability nobody had revoked, while leaving the driver's capability alive
    /// to re-map it. Both halves of that are asserted here, in the direction they should now go.
    #[test_case]
    fn revoking_one_run_leaves_an_overlapping_capability_and_its_mappings_alone() {
        let mut driver = AddressSpace::new(2).expect("no driver space");
        let mut client = AddressSpace::new(2).expect("no client space");
        let region = crate::memory_region::create(8).expect("no region");
        let run = consecutive::<4>(region);

        // `Rights::ALL` carries `GRANT`, which is what `PageFrame::REVOKE` requires and what no run
        // capability in the shipping tree has yet: minting it here is how this path is reachable at
        // all. See design/decisions/132-*.md on why that made the question worth answering early
        // rather than at the first real `GRANT`.
        let window = crate::sched::grant(page_frame_run_cap(
            run[0],
            page_frame_run_len(4),
            Rights::ALL,
        ))
        .expect("grant the whole window");
        let surface = crate::sched::grant(page_frame_run_cap(
            run[1],
            page_frame_run_len(3),
            Rights::ALL,
        ))
        .expect("grant the inner surface");

        let (va_driver, va_client) = (0x40_0000u64, 0x80_0000u64);
        driver
            .map_physical(
                va_driver,
                run[1],
                Flags::user_data(),
                PageMapSource::Capability(run[0]),
            )
            .expect("map driver");
        client
            .map_physical(
                va_client,
                run[1],
                Flags::user_data(),
                PageMapSource::Capability(run[1]),
            )
            .expect("map client");

        // The client hands its surface back.
        revoke_page_frame_run(run[1], 3);

        assert!(
            mmu::translate_at(client.root(), va_client).is_none(),
            "the revoked capability's own mapping survived the revoke",
        );
        assert!(
            mmu::translate_at(driver.root(), va_driver).is_some(),
            "revoking one capability unmapped a page another capability had mapped: the \
             space-blind unmap §132 replaced",
        );
        assert!(
            crate::sched::current_cap(surface).is_err(),
            "REVOKE left the capability that named the revoked run in the revoker's own table",
        );
        assert!(
            crate::sched::current_cap(window).is_ok(),
            "revoking a run deleted an overlapping capability nobody revoked: §132 option B, \
             refused because a three-page holder must not be able to delete a four-page one",
        );

        crate::memory_region::destroy(region);
    }

    /// **A device take-back is scoped by holder, not by capability, and §132 left that alone.**
    ///
    /// The regression guard for DECISIONS §41, which is the one revocation in the tree that was
    /// already selective and is selective along a *different* axis. If capability scope ever leaked
    /// into this path, a second capability to the same MMIO page would survive the take-back and the
    /// device would have two owners, which is exactly what live replacement (`user::live_swap_tests`)
    /// exists to prevent.
    ///
    /// Two holders map one physical page **under deliberately different objects**, so an
    /// object-scoped sweep would spare one of them; the take-back must still take it. What is
    /// exercised is `unmap_everywhere` rather than `revoke_device_from_others` itself, because that
    /// function reads its spare from `mmu::current_user_root()` and a kernel test is not running in
    /// either of these spaces. The end-to-end evidence for the whole function is the swap suite's
    /// `LOG_REVOKE_ENFORCED`, where a real userspace invoker keeps the registers and the outgoing
    /// driver faults on them.
    #[test_case]
    fn a_device_take_back_ignores_the_object_and_spares_one_holder() {
        let mut keeper = AddressSpace::new(2).expect("no keeper space");
        let mut loser = AddressSpace::new(2).expect("no loser space");
        let region = crate::memory_region::create(4).expect("no region");
        let run = consecutive::<2>(region);
        let shared = run[1];

        let (va_keeper, va_loser) = (0x40_0000u64, 0x80_0000u64);
        keeper
            .map_physical(
                va_keeper,
                shared,
                Flags::user_data(),
                PageMapSource::Capability(run[0]),
            )
            .expect("map keeper");
        loser
            .map_physical(
                va_loser,
                shared,
                Flags::user_data(),
                PageMapSource::Capability(shared),
            )
            .expect("map loser");

        unmap_everywhere(shared, keeper.root());

        assert!(
            mmu::translate_at(keeper.root(), va_keeper).is_some(),
            "the take-back unmapped the invoker's own mapping: §41's asymmetry is the point",
        );
        assert!(
            mmu::translate_at(loser.root(), va_loser).is_none(),
            "a holder that recorded its mapping under a different object survived the take-back: \
             the device now has two owners",
        );

        crate::memory_region::destroy(region);
    }

    /// **Destroying a region deletes a run capability naming its pages, so the reclaimed memory is
    /// not nameable** (milestone 142's review, CRITICAL 1).
    ///
    /// The regression this exists for: reclamation used to delete capabilities by *exact object*,
    /// one mapped page at a time, and `PageFrame(base, 3)` is equal to no `PageFrame(p, 1)` for any
    /// `p`. So a holder kept a live capability over pages that had gone back to the allocator and
    /// could re-map them read/write once they had been handed out again as a page table or another
    /// process's stack: §13's use-after-free, straight through the door `MemoryRegion::DESTROY`
    /// exists to shut.
    ///
    /// Two capabilities, because the two halves failed for different reasons. The run is the
    /// widening's own hole. The single unmapped page is the older one the same fix closes: it was
    /// never mapped, so the mapping log had no record to find it by and the per-page sweep never
    /// looked at it at all.
    #[test_case]
    fn destroying_a_region_deletes_every_capability_naming_it() {
        let mut space = AddressSpace::new(2).expect("no space");
        let region = crate::memory_region::create(4).expect("no region");
        let run = consecutive::<3>(region);
        let unmapped = crate::memory_region::retype_page(region).expect("retype 3");

        let va = 0x40_0000u64;
        space
            .map_physical(
                va,
                run[0],
                Flags::user_data(),
                PageMapSource::Capability(run[0]),
            )
            .expect("map");

        let run_slot = crate::sched::grant(page_frame_run_cap(
            run[0],
            page_frame_run_len(3),
            Rights::ALL,
        ))
        .expect("grant the run");
        let lone_slot = crate::sched::grant(crate::cap::page_frame_cap(unmapped, Rights::ALL))
            .expect("grant the unmapped page");

        crate::memory_region::destroy(region);

        assert!(
            crate::sched::current_cap(run_slot).is_err(),
            "a run capability outlived the reclamation of the pages it names: §13's use-after-free",
        );
        assert!(
            crate::sched::current_cap(lone_slot).is_err(),
            "a capability to a never-mapped page outlived its region: the mapping log cannot see it",
        );
        assert!(
            mmu::translate_at(space.root(), va).is_none(),
            "destroy reclaimed a page a live address space still maps",
        );
    }

    /// **Revocation unmaps a shared page from every address space that held it.** Two address
    /// spaces map one physical page; after `revoke_page_frame` neither maps it. This is the property
    /// the whole reclamation story rests on: a page may be reused only once no holder still maps
    /// it. (The records now live in the spaces' own regions; nothing else changed here.)
    #[test_case]
    fn revoke_unmaps_a_shared_page_from_every_address_space() {
        let mut a = AddressSpace::new(2).expect("no space A");
        let mut b = AddressSpace::new(2).expect("no space B");
        let shared = crate::memory::alloc().expect("no frame").addr();
        let (va_a, va_b) = (0x40_0000u64, 0x80_0000u64);

        a.map_physical(
            va_a,
            shared,
            Flags::user_data(),
            PageMapSource::NoCapability,
        )
        .expect("map A");
        b.map_physical(
            va_b,
            shared,
            Flags::user_rodata(),
            PageMapSource::NoCapability,
        )
        .expect("map B");

        assert!(
            mmu::translate_at(a.root(), va_a).is_some(),
            "A does not map the page"
        );
        assert!(
            mmu::translate_at(b.root(), va_b).is_some(),
            "B does not map the page"
        );

        revoke_page_frame(shared);

        assert!(
            mmu::translate_at(a.root(), va_a).is_none(),
            "A still maps the revoked page"
        );
        assert!(
            mmu::translate_at(b.root(), va_b).is_none(),
            "B still maps the revoked page"
        );

        crate::memory::free(page_frames::PageFrame::from_addr(shared));
    }

    /// **Destroying an untyped region unmaps its pages, THEN reclaims them.** A page from the
    /// region is mapped into an address space; `memory_region::destroy` must remove that mapping before
    /// the page returns to the allocator, or a later allocation hands out memory a live process
    /// still maps (the use-after-free the tripwire in untyped.rs warns of). Both halves are
    /// asserted: the mapping is gone, and the region's frames come back.
    #[test_case]
    fn destroy_unmaps_a_region_before_reclaiming_it() {
        let mut space = AddressSpace::new(2).expect("no space");
        let region = crate::memory_region::create(4).expect("no region");
        let phys = crate::memory_region::retype_page(region).expect("retype");
        let va = 0x40_0000u64;
        space
            .map_physical(va, phys, Flags::user_data(), PageMapSource::NoCapability)
            .expect("map");
        assert!(
            mmu::translate_at(space.root(), va).is_some(),
            "the page was not mapped"
        );

        let free_before = crate::memory::stats().unwrap().free();
        crate::memory_region::destroy(region);
        let free_after = crate::memory::stats().unwrap().free();

        assert!(
            mmu::translate_at(space.root(), va).is_none(),
            "destroy reclaimed a page a live address space still maps: the tripwire's use-after-free",
        );
        assert_eq!(
            free_after,
            free_before + 4,
            "destroy did not return the region's 4 frames to the allocator",
        );
    }

    /// **A mapping that cannot be recorded cannot exist, and the failure is the mapper's own.**
    /// A space with a tiny region records mappings until its budget is gone; the failing record
    /// returns false rather than silently leaving a mapping revocation would miss.
    #[test_case]
    fn an_exhausted_budget_refuses_the_record_not_the_safety() {
        let mut space = AddressSpace::new(0).expect("no space");
        let shared = crate::memory::alloc().expect("no frame").addr();

        // Burn the region down to nothing by recording mappings. The arithmetic that makes
        // refusal certain: a 0-content space has ~15 spendable pages, and 4096 mappings need
        // ~16 log pages plus ~8 table pages, so the budget must run out mid-loop. (2048 was
        // tried first and fit EXACTLY: 6 tables + 9 log pages = 15. Off by nothing.)
        //
        // The map and the record are one call since 2026-09-21 (`AddressSpace::map_physical`'s own
        // docs), so the two ways the budget can end are now one `Err`: an exhausted region refuses
        // a table page or a log page and the caller cannot tell which, which is the same answer.
        let mut refused = false;
        for i in 0..4096u64 {
            let va = 0x40_0000 + i * page_frames::FRAME_SIZE;
            if space
                .map_physical(
                    va,
                    shared,
                    Flags::user_rodata(),
                    PageMapSource::NoCapability,
                )
                .is_err()
            {
                refused = true;
                break;
            }
        }
        assert!(
            refused,
            "2048 mappings recorded out of a {}-page region: records are not being paid for",
            crate::memory_region::usage(0).map(|(_, p)| p).unwrap_or(0),
        );

        crate::memory::free(page_frames::PageFrame::from_addr(shared));
    }
}
