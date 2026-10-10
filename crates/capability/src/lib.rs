//! Capability tables. **A file descriptor table that can point at anything.**
//!
//! See notes/capabilities.md and DECISIONS §10. The one sentence:
//!
//! > A capability is a file descriptor that can point at *anything*, not just files.
//!
//! Same mechanism, generalized. **The unforgeability is boring, and that is the point**: this
//! table lives in kernel memory and userspace never sees a byte of it. Userspace sees an
//! integer. You cannot fabricate slot 7 for exactly the same reason you cannot fabricate `fd 7`.
//! There is no cryptography here and there is no magic. There is an array and a bounds check.
//!
//! Pure logic, so it compiles for the host and its tests run in milliseconds (DECISIONS §7).
//! Nothing in here knows what a console is; the kernel supplies the object type.
//!
//! Allocation-free since milestone 14 phase B.1: the table is a const-generic array, so a
//! capability table's size is part of its type and creating one cannot touch a heap. The kernel picks the
//! size once (16, in kernel/src/cap.rs); the tests and proofs pick small ones.
//!
//! # Examples
//!
//! Rights are a lattice, and the only operation that ever runs on the transfer path is
//! **intersection**: a capability you pass on can never carry more than you held. That is the
//! monotonicity the whole model rests on, so it is what an example should show.
//!
//! ```
//! use capability::Rights;
//!
//! let held = Rights::READ.union(Rights::WRITE);
//! assert!(held.allows(Rights::READ));
//! assert!(!held.allows(Rights::GRANT));
//!
//! // Handing it on with a mask can only ever narrow it.
//! let passed_on = held.intersect(Rights::READ.union(Rights::GRANT));
//! assert_eq!(passed_on, Rights::READ);
//! assert!(passed_on.is_subset_of(held));
//! ```
//!
//! There is no operation that widens, which is worth stating as code because it is a claim about
//! what is *absent*:
//!
//! ```
//! use capability::Rights;
//!
//! let read_only = Rights::READ;
//! // The best you can do with ALL on the other side is get back what you already had.
//! assert_eq!(read_only.intersect(Rights::ALL), read_only);
//! assert!(!read_only.intersect(Rights::ALL).allows(Rights::WRITE));
//! ```
//!
//! Name: ratified 2026-08-01 (calef, milestone 63), replacing `caps`. Refused `caps` (a container's
//! name for what is the capability model: it exports `Cap`, `Rights`, `Object`, `Reap` and
//! `CapabilityTable`) and `cap_space` (names one of five exports, and stutters as `cap_space::CapabilityTable`).
//! The exported type was originally kept as `CSpace`, seL4's own spelling; DECISIONS §113's
//! amendment (calef, 2026-08-25) renamed it to `CapabilityTable`, the plain term this crate's own
//! prose already reached for ("A thread's capability table," above).

#![no_std]

/// What you may do with a capability.
///
/// **Rights only ever narrow.** [`CapabilityTable::derive`] will not widen them, and that is not a policy
/// we remembered to enforce, it is the only operation the type offers. If delegation could
/// widen authority, the whole model is theatre: anyone could hand themselves the rights they
/// were denied by passing a capability to themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rights(u32);

impl Rights {
    /// No rights at all.
    pub const NONE: Rights = Rights(0);
    /// The right to read the object's contents.
    pub const READ: Rights = Rights(1 << 0);
    /// The right to modify the object's contents.
    pub const WRITE: Rights = Rights(1 << 1);

    /// The right to **pass this capability on**. Without it, you may use a thing and not lend it.
    ///
    /// This is the right that makes delegation a decision rather than an accident. Unix has no
    /// equivalent, which is why "the child inherits every fd" is the default and why
    /// `FD_CLOEXEC` had to be invented as an afterthought.
    pub const GRANT: Rights = Rights(1 << 2);

    /// The right to **learn what exists**, as distinct from acting on it (milestone 126).
    ///
    /// This is the kernel-level twin of `filesystem_protocol`'s directory `ENUMERATE`, and it is the same
    /// argument one layer down: listing is a larger power than reading something you were handed,
    /// so it is a right of its own rather than a corner of `READ`. The name is deliberately the
    /// word the filesystem already uses, because a reader who has met one has met both.
    ///
    /// **Why it had to exist.** `rendezvous::SURVEY` first shipped needing `READ`, and `READ` on a
    /// supervision rendezvous is also what `RECEIVE` and `REAP` take. A viewer granted `READ` could
    /// therefore reap a child, which is acting on a member of a domain rather than naming one.
    /// calef ruled on 2026-08-17 that **a domain names its members and never acts on them**, and
    /// one bit for three operations cannot express that. With this right the wrong operation is
    /// not refused to a viewer, it is unnameable by one.
    ///
    /// **Deliberately not pre-wired to other objects.** Only `Rendezvous` consults it today. `AddressSpace`
    /// wants it when `pmap` is built (observe a mapping without being able to map) and `MemoryRegion`
    /// wants it when `free` is built (ask what is committed without being able to `SPLIT` or
    /// `DESTROY`), and both are named in milestone 126's strata rather than built ahead of a
    /// consumer that can say what it needs. A right defined for three hypothetical callers is the
    /// speculative abstraction this tree declines.
    pub const ENUMERATE: Rights = Rights(1 << 3);

    /// **The right to add a thread to a process** (milestone 812 (`std::thread::spawn` runs real
    /// threads in one address space); §269 (how threads share a process) fork 1, calef's ruling
    /// (b) on pull request #1892, 2026-10-10 UTC). Held on a process capability, it lets
    /// `ThreadControlBlock::CONFIGURE` join an embryo to that process without consuming the
    /// capability. Adding a thread is running code inside the process, so it is an authority that
    /// can be withheld and narrowed on delegation, which a method form could not be. Only
    /// `Process` consults it. Zircon's `ZX_RIGHT_MANAGE_THREAD` on a process handle is the shape.
    pub const BIND: Rights = Rights(1 << 4);

    /// **Every defined bit.** [`from_bits`](Self::from_bits) masks against this, so a right that is
    /// missing here is silently dropped at every delegation: adding a constant above without
    /// widening this mask produces a right that cannot survive being passed on.
    pub const ALL: Rights = Rights(0b1_1111);

    /// The raw bit pattern, for carrying rights across a boundary as an integer (a syscall
    /// register). [`from_bits`](Self::from_bits) is the inverse.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Rebuild rights from a raw bit pattern, keeping only defined bits. **The reverse of
    /// [`bits`](Self::bits), for rights that crossed a boundary as an integer.** Delegation names
    /// the rights to narrow a capability to by their bits (they travel in a syscall register), and
    /// this turns those bits back into a `Rights` the subset check can vet. Undefined bits are
    /// dropped, so a caller cannot conjure a right that does not exist.
    pub const fn from_bits(bits: u32) -> Rights {
        Rights(bits & Rights::ALL.0)
    }

    /// Every right either side holds.
    pub const fn union(self, other: Rights) -> Rights {
        Rights(self.0 | other.0)
    }

    /// Only the rights both sides hold. The one operation on the delegation path: it can only
    /// narrow, never widen, which is the monotonicity the whole model rests on.
    pub const fn intersect(self, other: Rights) -> Rights {
        Rights(self.0 & other.0)
    }

    /// Do I hold *at least* these rights?
    pub const fn allows(self, needed: Rights) -> bool {
        self.0 & needed.0 == needed.0
    }

    /// Is `self` no more than `other`? The subset test that [`CapabilityTable::derive`] turns on.
    pub const fn is_subset_of(self, other: Rights) -> bool {
        self.0 & !other.0 == 0
    }
}

/// A capability: **an object, and what you may do with it.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cap<O> {
    /// What this capability names.
    pub object: O,
    /// What the holder may do with it.
    pub rights: Rights,
}

impl<O> Cap<O> {
    /// **Mint a fresh capability to `object` carrying no more authority than this one.** The rule a
    /// fresh mint site *outside* [`CapabilityTable::derive`] must honor by hand so that authority never
    /// widens: the child inherits the parent's rights, never a superset.
    ///
    /// `MemoryRegion::SPLIT` is such a site (kernel/src/cap.rs, milestone 31): it carves a child budget
    /// off a parent untyped and mints a capability to it, and because that mint is not a `derive` the
    /// "derive never widens rights" proof does not reach it. Routing it through here puts it under the
    /// [`split_never_widens_rights`](../src/lib.rs) proof, so a spend-only (GRANT-less) untyped cannot
    /// split itself a GRANT-bearing child and manufacture the right its own capability was denied. See
    /// DECISIONS §16 and notes/verification.md.
    pub fn mint_child(&self, object: O) -> Cap<O> {
        Cap {
            object,
            rights: self.rights,
        }
    }
}

/// **May this supervision rendezvous collect this thread's corpse?** (DECISIONS §32.)
///
/// The whole authorization decision of `rendezvous::REAP`, as pure logic, so it is proved for every
/// input rather than tested on the cases we thought of. The kernel calls exactly this
/// (`sched::reap_supervised`) and then does the reclaim; nothing here mints, widens, or returns a
/// capability, which is why a supervisor gains no memory authority by reaping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reap {
    /// Authorized: this rendezvous supervises that thread, and the thread is a corpse.
    Permitted,
    /// The thread is supervised here but still running. Collecting a corpse is not killing.
    StillAlive,
    /// No thread by that name is supervised by *this* rendezvous: it never existed, it is
    /// unsupervised, it belongs to another supervisor, or its name is stale.
    NotSupervised,
}

/// The decision, from the three facts the kernel has: the thread's recorded supervision rendezvous
/// (`None` if the thread does not resolve at all, or resolves and is unsupervised), the rendezvous
/// actually being invoked, and whether the thread is dead.
///
/// **Ordering matters and is deliberate.** The supervision check comes first, so a supervisor
/// learns nothing about a thread it does not supervise: a live stranger and a stale name give the
/// same answer. Only once the relationship is established does the liveness answer differ, because
/// only then is the distinction the holder's business.
pub const fn reap_decision(fault_ep: Option<u64>, invoked_ep: u64, dead: bool) -> Reap {
    match fault_ep {
        Some(ep) if ep == invoked_ep => {
            if dead {
                Reap::Permitted
            } else {
                Reap::StillAlive
            }
        }
        _ => Reap::NotSupervised,
    }
}

/// **Is this thread inside the domain that rendezvous supervises?** (milestone 126,
/// `rendezvous::SURVEY`.)
///
/// The whole scoping decision of a process view, as pure logic, so it is proved for every input
/// rather than tested on the cases we thought of. The kernel walks its thread table and reports an
/// entry exactly when this says so (`sched::survey_supervised`).
///
/// **It is deliberately the same predicate [`reap_decision`] authorizes with**, and that is the
/// point rather than a saving: the set of threads a supervisor may *see* is the set whose deaths
/// arrive on its rendezvous, so a domain cannot drift out of agreement with the relationship the
/// kernel already maintains. Two predicates would have been two things to keep in step.
///
/// Liveness is not an input. A corpse is still in the domain, which is what makes `ps` able to show
/// a `Dead` child the supervisor has not collected yet.
pub const fn survey_includes(fault_ep: Option<u64>, invoked_ep: u64) -> bool {
    matches!(fault_ep, Some(ep) if ep == invoked_ep)
}

/// Why a capability table operation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// **The slot is empty.** Note what this is *not*: it is not "permission denied." There is
    /// nothing there. The thing you tried to name does not exist, for you.
    NoSuchSlot,
    /// You asked to derive a capability with rights you do not hold.
    CannotWiden,
    /// The table is full.
    NoFreeSlot,
}

/// **The most slots a [`CapabilityTable`] may have: one per bit of its free-slot word.**
///
/// The free-slot bitmap is a `u64` (milestone 754 (the capability table grows to 64 slots), calef's
/// ruling A on #1608, 2026-10-04, UTC; it was a `u32` from milestone 126 (the `procps` package),
/// calef's ruling on #1360, 2026-09-27), so a table of more than 64 slots does not build:
/// [`CapabilityTable::new`] asserts it at compile time for every `N` anybody instantiates.
/// `kernel/src/cap.rs` asserts it again beside `CAPABILITY_TABLE_SLOTS`, where the next raise will
/// be typed. Growing past 64 is a wider word or a second one, and at that point
/// `design/roadmap/0778-capability-tables-sized-per-process.md` is the question to answer
/// first. The `u32` it replaced fitted in a table with no padding at 32 slots (1,032 bytes); the
/// `u64` rounds a 64-slot table to 2,064 bytes, eight more than the slots, word and counts need.
pub const MAX_SLOTS: usize = u64::BITS as usize;

pub use storage::CapabilityTable;

/// **The only code that can touch a slot**, and that is the mechanism rather than a convention.
///
/// A table carries three facts about its slots besides the slots themselves: which are free (the
/// bitmap `insert` takes its slot from), how many are occupied, and the most that ever were. Each
/// is a second record of something the array already says, and a second record is only worth
/// having if it cannot drift from the first. So the fields are private to this module, and the
/// module has exactly two ways to change a slot: [`fill`](CapabilityTable::fill) and
/// [`empty`](CapabilityTable::empty). Every mutator the rest of the crate offers (`insert`,
/// `insert_at`, `put`, `derive`, `delete`, `delete_matching`) is written outside this module in
/// terms of those two, so a new mutator *cannot* be written that forgets the bookkeeping: the
/// compiler refuses the field access. `AGENTS.md`'s ladder, rung one.
///
/// Milestone 231 (nothing counts how many capability slots a boot actually uses) built the
/// counting half as a pair of private `grew`/`shrank` calls that each mutator had to make; review
/// found that the freeing side had two callers and nothing forcing either. Folding them into the
/// two doors here is that finding taken to its end. `verification::the_free_mask_is_the_empty_slots` and `the_count_is_the_slots` check
/// the two doors against the array over every small table state.
mod storage {
    use super::{Cap, MAX_SLOTS, note_peak};

    /// A thread's capability table.
    ///
    /// **Flat, not a tree.** seL4 uses a tree of CNodes with guard bits, which buys enormous sparse
    /// capability spaces and costs a great deal of explanation. We do not need it, and a flat array
    /// is honest: it is an fd table with a type tag on each entry, which is exactly what a capability
    /// space *is*. If we ever need the sparse version, it is a change to this file and nothing else.
    pub struct CapabilityTable<O, const N: usize> {
        slots: [Option<Cap<O>>; N],
        /// **Bit `i` set means slot `i` is empty** (milestone 126 (the `procps` package), calef's
        /// ruling on #1360, 2026-09-27, UTC). Set-means-free rather than set-means-used so that
        /// `insert`'s first free slot is one `trailing_zeros` with no inversion, and bits at and
        /// above `N` are always clear, so a full table is a zero word. It replaced a linear scan
        /// that LLVM unrolled once per slot, which grew the IPC fastpath with every slot added
        /// (`script/fastpath-footprint`); the before and after are in milestone 126's block.
        ///
        /// A `u64` since milestone 754 (the capability table grows to 64 slots) (a `u32` before it); the table is `N` slots, this word and
        /// two `u16` counts, rounded to eight bytes.
        free: u64,
        /// How many of the slots are occupied right now. Maintained by the two doors rather than
        /// counted on demand, because the thing that wants it is a high-water mark and a mark
        /// sampled at a convenient moment is not one.
        used: u16,
        /// The largest `used` this table has ever held. **Never decreases**, which is the whole
        /// point: the number a boot reports is what it *reached*, not what it happens to be holding
        /// when somebody looks. See [`peak`](Self::peak) and [`highest_seen`](super::highest_seen).
        peak: u16,
    }

    impl<O: Copy, const N: usize> CapabilityTable<O, N> {
        /// Every slot free: the low `N` bits. The assertion is the compile-time wall
        /// [`MAX_SLOTS`] describes, evaluated for each `N` that is instantiated.
        const ALL_FREE: u64 = {
            assert!(
                N <= MAX_SLOTS,
                "a capability table has one free-slot bit per slot, and the word is a u64"
            );
            if N == MAX_SLOTS {
                u64::MAX
            } else {
                (1u64 << N) - 1
            }
        };

        /// **A brand-new process holds nothing.**
        ///
        /// This is the whole decision, expressed as a constructor. Under Unix a fresh process
        /// inherits every fd its parent had and can `open()` anything its uid permits. Here it can
        /// name nothing at all until somebody hands it something.
        pub const fn new() -> Self {
            CapabilityTable {
                slots: [const { None }; N],
                free: Self::ALL_FREE,
                used: 0,
                peak: 0,
            }
        }

        /// The capability in `slot`, or `None` for an empty or out-of-range one.
        pub(super) fn slot(&self, slot: usize) -> Option<Cap<O>> {
            self.slots.get(slot).copied().flatten()
        }

        /// The lowest-numbered empty slot, or `None` when the table is full. One count of trailing
        /// zeros, whatever `N` is.
        pub(super) fn first_free(&self) -> Option<usize> {
            if self.free == 0 {
                None
            } else {
                Some(self.free.trailing_zeros() as usize)
            }
        }

        /// The lowest empty slot at or above `floor`, or `None` when there is none.
        pub(super) fn first_free_from(&self, floor: usize) -> Option<usize> {
            let above = if floor >= 64 {
                0
            } else {
                self.free & (u64::MAX << floor)
            };
            (above != 0).then(|| above.trailing_zeros() as usize)
        }

        /// The free-slot word itself, for the proofs and the tests that check it against the array.
        #[cfg_attr(not(any(test, kani)), allow(dead_code))]
        pub(super) fn free_mask(&self) -> u64 {
            self.free
        }

        /// The occupied slots, as a mask: `free`'s complement, restricted to the `N` bits that
        /// exist (bits at and above `N` are always clear in `free`, so inverting it would set
        /// them). [`delete_matching`](super::CapabilityTable::delete_matching) walks this instead
        /// of `0..N` so a sweep over a mostly-empty table costs its occupancy, not its capacity;
        /// milestone 126 (the `procps` package) raised `N` and found the sweep had never made that
        /// trade, on `insert`'s reasoning exactly, in the one caller (revocation, `sched.rs`'s
        /// `delete_page_frame_caps_where`) that runs it over every thread's table for every frame.
        pub(super) fn occupied_mask(&self) -> u64 {
            !self.free & Self::ALL_FREE
        }

        /// Whether every slot is empty: one compare, because the word already knows.
        pub(super) fn all_free(&self) -> bool {
            self.free == Self::ALL_FREE
        }

        /// **Door one: put `cap` in `slot`**, occupied or not. `None` if `slot` is out of range;
        /// otherwise whether it was empty before. Clears the slot's free bit, and if the slot was
        /// empty, counts the growth and raises the high-water mark.
        pub(super) fn fill(&mut self, slot: usize, cap: Cap<O>) -> Option<bool> {
            let s = self.slots.get_mut(slot)?;
            let was_empty = s.is_none();
            *s = Some(cap);
            self.free &= !(1u64 << slot);
            if was_empty {
                // Wrapping, because the wrap cannot happen and this is on every reply-capability
                // mint (release builds check overflow, notes/overflow-checks.md). `used` counts
                // occupied slots (`the_count_is_the_slots` proves it), this slot was empty, so the
                // result is at most `N`, which the `u64` free mask bounds at 64.
                self.used = self.used.wrapping_add(1);
                if self.used > self.peak {
                    self.peak = self.used;
                    note_peak(self.peak, N);
                }
            }
            Some(was_empty)
        }

        /// **Door two: take whatever is in `slot`**, leaving it empty. `None` if `slot` is out of
        /// range or already empty, and then nothing changes. Sets the slot's free bit and counts
        /// the loss.
        ///
        /// `peak` is deliberately untouched. A high-water mark that fell when the table drained
        /// would report what a boot is holding rather than what it reached, which is the number
        /// milestone 230 (`script/shell-check` is red on `main`) needed and could not get.
        pub(super) fn empty(&mut self, slot: usize) -> Option<Cap<O>> {
            let cap = self.slots.get_mut(slot)?.take()?;
            self.free |= 1u64 << slot;
            self.used -= 1;
            Some(cap)
        }

        /// How many slots are occupied right now.
        pub fn used(&self) -> usize {
            self.used as usize
        }

        /// **The most slots this table has ever held at once**, which is the number a capacity
        /// decision is actually made against. `used` at any given moment is whatever the last few
        /// calls left behind; this is the wall the table came closest to.
        pub fn peak(&self) -> usize {
            self.peak as usize
        }
    }
}

impl<O: Copy, const N: usize> Default for CapabilityTable<O, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<O: Copy, const N: usize> CapabilityTable<O, N> {
    /// The table's fixed capacity, `N`. Never changes; a capability table's size is part of its type.
    pub const fn len(&self) -> usize {
        N
    }

    /// Whether every slot is currently empty.
    pub fn is_empty(&self) -> bool {
        self.all_free()
    }

    /// Look up a slot. **The entire security mechanism, and it is a bounds check.**
    pub fn get(&self, slot: u64) -> Result<Cap<O>, Error> {
        self.slot(slot as usize).ok_or(Error::NoSuchSlot)
    }

    /// Look up a slot and require rights.
    ///
    /// Returns [`Error::NoSuchSlot`] for an empty slot even when the caller wanted rights, which
    /// is deliberate: **"you may not" and "there is nothing there" are different answers**, and
    /// the second one leaks less.
    pub fn get_with(&self, slot: u64, needed: Rights) -> Result<Cap<O>, Error> {
        let cap = self.get(slot)?;
        if !cap.rights.allows(needed) {
            return Err(Error::CannotWiden);
        }
        Ok(cap)
    }

    /// Put a capability in the first free slot. Used at spawn, to hand a process its world, and
    /// on every `CALL` for the one-shot Reply, which is why it is one `trailing_zeros` and not a
    /// scan (see the free-slot word's own doc).
    pub fn insert(&mut self, cap: Cap<O>) -> Result<u64, Error> {
        let slot = self.first_free().ok_or(Error::NoFreeSlot)?;
        self.fill(slot, cap);
        Ok(slot as u64)
    }

    /// Put a capability in the **first free slot at or above `floor`** (milestone 812
    /// (`std::thread::spawn` runs real threads in one address space)). [`insert`](Self::insert) is
    /// `floor` 0. A `std` program's low slots are fixed by number and an empty one means "not
    /// granted", so what the program makes for itself must land above them.
    pub fn insert_at_or_above(&mut self, floor: u64, capability: Cap<O>) -> Result<u64, Error> {
        let floor = usize::try_from(floor).unwrap_or(usize::MAX);
        let slot = self.first_free_from(floor).ok_or(Error::NoFreeSlot)?;
        self.fill(slot, capability);
        Ok(slot as u64)
    }

    /// Put a capability in a **specific free slot**, refusing an occupied or out-of-range one.
    ///
    /// The targeted counterpart of [`insert`](Self::insert). A supervisor uses it to place a
    /// child's supervision rendezvous in the reserved fault slot (abi's `FAULT_EP_SLOT`) rather than
    /// wherever first-free would fall, and refusing an occupied slot keeps that reservation honest:
    /// two things must never share the slot the kernel reads at `START`.
    pub fn insert_at(&mut self, slot: u64, cap: Cap<O>) -> Result<u64, Error> {
        if slot >= N as u64 {
            return Err(Error::NoSuchSlot);
        }
        if self.slot(slot as usize).is_some() {
            return Err(Error::NoFreeSlot);
        }
        self.fill(slot as usize, cap);
        Ok(slot)
    }

    /// Put a capability in a specific slot, replacing whatever was there.
    pub fn put(&mut self, slot: u64, cap: Cap<O>) -> Result<(), Error> {
        self.fill(slot as usize, cap).ok_or(Error::NoSuchSlot)?;
        Ok(())
    }

    /// **Copy a capability into another slot, with rights that are no greater.**
    ///
    /// The only way authority moves. `rights` is intersected with what the source slot actually
    /// holds, and if the caller asked for more than that, it is an error rather than a silent
    /// clamp: **asking to widen is a bug in the caller, and a loader that quietly grants less
    /// than was asked for is a loader nobody can reason about.**
    pub fn derive(&mut self, from: u64, to: u64, rights: Rights) -> Result<(), Error> {
        let src = self.get(from)?;

        if !rights.is_subset_of(src.rights) {
            return Err(Error::CannotWiden);
        }

        self.put(
            to,
            Cap {
                object: src.object,
                rights,
            },
        )
    }

    /// Drop a capability. The object may still exist; **we simply can no longer name it.**
    pub fn delete(&mut self, slot: u64) -> Result<(), Error> {
        self.empty(slot as usize).ok_or(Error::NoSuchSlot)?;
        Ok(())
    }

    /// Delete every occupied slot whose object satisfies `matches`, in place. The revocation
    /// sweeps' primitive (a whole-range reclamation visits every table in the system), so it reads
    /// each slot once instead of a `get`-then-`delete` pair, whose halves both re-run the bounds
    /// check and which a sweep over every slot of every thread pays thousands of times per call.
    /// Behaviorally identical to that pair.
    ///
    /// **Walks the occupied mask, not `0..N`** (milestone 126 (the `procps` package), 2026-09-27,
    /// UTC): raising `CAPABILITY_TABLE_SLOTS` to 32 put this sweep's cost, paid by every thread's
    /// table on every frame a revocation touches, on the table's *capacity* rather than what it
    /// actually holds, and `spawn_el0` (which destroys a small, mostly-empty table every
    /// iteration) is exactly that sweep's shape. `insert` stopped scanning for the same reason;
    /// this is that fix's other caller.
    pub fn delete_matching(&mut self, matches: impl Fn(&O) -> bool) {
        let mut occupied = self.occupied_mask();
        while occupied != 0 {
            let slot = occupied.trailing_zeros() as usize;
            occupied &= occupied - 1; // clear the lowest set bit; each slot visited once
            if matches!(self.slot(slot), Some(c) if matches(&c.object)) {
                self.empty(slot);
            }
        }
    }
}

/// **The high-water mark across every capability table in this binary, and the ceiling it was
/// measured against** (milestone 231).
///
/// Packed into one `AtomicU32` (`peak << 16 | ceiling`) so a reader gets a pair that were true at
/// the same instant. Both halves fit: a capability table is a const-generic array and `N` is a
/// small number, and [`note_peak`] saturates rather than wrapping if anyone ever makes one that
/// does not fit.
///
/// **Why a global at all.** The per-table [`CapabilityTable::peak`] is the honest record and this
/// is a convenience over it: the kernel wants to know *that the mark moved* without walking 256
/// thread tables under the scheduler lock on every idle pass, and a single atomic answers that in
/// one load. Nothing here decides anything; `kernel/src/sched.rs`'s idle loop is the only reader,
/// and all it does is print.
///
/// # BUGS
///
/// **It does not say which table.** A `static` cannot be keyed by a const generic, so this is one
/// number for every instantiation of [`CapabilityTable`] linked into the binary. The kernel has
/// exactly one (`kernel::cap::CapabilityTable`, `N = kernel::cap::CAPABILITY_TABLE_SLOTS`), so there the number is unambiguous; a
/// host test binary that exercises several sizes will see whichever one climbed highest, and the
/// `ceiling` half tells a reader which. Finding the owning thread means the scan this exists to
/// avoid, and `kernel/src/sched.rs` does it once, only when it is about to print.
static PEAK: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

/// Raise [`PEAK`] if `peak` beats what is recorded. Called from [`CapabilityTable`]'s one private
/// growth path, so it cannot be forgotten.
fn note_peak(peak: u16, ceiling: usize) {
    use core::sync::atomic::Ordering;
    let ceiling = core::cmp::min(ceiling, u16::MAX as usize) as u32;
    let packed = ((peak as u32) << 16) | ceiling;
    // `fetch_max` rather than a compare-exchange loop: the packed word puts the peak in the high
    // half, so numeric max IS max-by-peak, and the ceiling that rides along is the one belonging to
    // the table that set the record.
    PEAK.fetch_max(packed, Ordering::Relaxed);
}

/// **The highest occupancy any capability table has reached, and that table's capacity.**
///
/// `(0, 0)` before anything has ever been inserted.
///
/// **It does not say which table.** A `static` cannot be keyed by a const generic, so this is one
/// number for every instantiation of [`CapabilityTable`] linked into the binary; the ceiling half
/// tells a reader which one set it. The kernel has exactly one, so there it is unambiguous.
pub fn highest_seen() -> (usize, usize) {
    let packed = PEAK.load(core::sync::atomic::Ordering::Relaxed);
    ((packed >> 16) as usize, (packed & 0xffff) as usize)
}

/// Machine-checked proofs of the capability model (DECISIONS §14, the verification thesis).
///
/// These are not tests. A test in the module below checks the handful of cases we thought to
/// write down; each harness here asks Kani (a bounded model checker) to prove a property for
/// **every** input, symbolically. `kani::any()` is an unconstrained value, so
/// `derive_never_widens_rights` covers all 2^32 source-rights and all 2^32 requested-rights
/// patterns at once, which is the whole difference between "we tested READ cannot become WRITE"
/// and "no reachable state widens rights."
///
/// The module is behind `#[cfg(kani)]`, so an ordinary `cargo build`/`cargo test` never sees it;
/// only `cargo kani` sets that cfg and links the `kani` intrinsics. Run with `script/verify` (or
/// `cargo kani -p capability`).
#[cfg(kani)]
mod verification {
    use super::*;

    /// Every capability is a subset of itself: the reflexive base case of the derivation order.
    /// Falsification: replayable `crates/capability/falsifications/verification.subset_is_reflexive.patch`
    #[kani::proof]
    fn subset_is_reflexive() {
        let a = Rights(kani::any());
        assert!(a.is_subset_of(a));
    }

    /// **Rights cannot be laundered through a chain.** If B is derived from A and C from B, then C
    /// is no more than A. This is why a *flat* subset check suffices and we never need to walk a
    /// derivation tree to bound a capability: subset is transitive, so the chain can only narrow.
    /// Falsification: replayable `crates/capability/falsifications/verification.subset_is_transitive.patch`
    #[kani::proof]
    fn subset_is_transitive() {
        let (a, b, c) = (
            Rights(kani::any()),
            Rights(kani::any()),
            Rights(kani::any()),
        );
        kani::assume(a.is_subset_of(b));
        kani::assume(b.is_subset_of(c));
        assert!(a.is_subset_of(c));
    }

    /// **Userspace cannot forge a right.** `from_bits` takes an attacker-controlled syscall register
    /// (any u32) and the result holds only defined rights, for every possible input.
    /// Falsification: replayable `crates/capability/falsifications/verification.from_bits_cannot_forge_a_right.patch`
    #[kani::proof]
    fn from_bits_cannot_forge_a_right() {
        let raw: u32 = kani::any();
        assert!(Rights::from_bits(raw).is_subset_of(Rights::ALL));
    }

    /// The two ways of asking the question agree: "a is no more than b" is exactly "b holds at
    /// least a". Proving them equivalent means a bug in one would show up against the other.
    /// Falsification: replayable `crates/capability/falsifications/verification.subset_matches_allows.patch`
    #[kani::proof]
    fn subset_matches_allows() {
        let (a, b) = (Rights(kani::any()), Rights(kani::any()));
        assert_eq!(a.is_subset_of(b), b.allows(a));
    }

    /// A small table in an arbitrary state: every slot independently empty or holding a capability
    /// with symbolic object and rights. Three slots is enough to exhibit "the slot in question, a
    /// different slot, and an empty one" in every combination.
    fn any_small_capability_table() -> CapabilityTable<u8, 3> {
        let mut cs: CapabilityTable<u8, 3> = CapabilityTable::new();
        for slot in 0..3u64 {
            if kani::any() {
                cs.put(
                    slot,
                    Cap {
                        object: kani::any(),
                        rights: Rights(kani::any()),
                    },
                )
                .unwrap();
            }
        }
        cs
    }

    /// **Consume-on-use is final.** For every table state and every slot (in bounds or not), once
    /// `delete` succeeds the slot answers `NoSuchSlot` to both `get` and a second `delete`. This is
    /// the mechanism that makes the one-shot Reply one-shot (DECISIONS §12): the syscall layer
    /// deletes the Reply capability the instant it is invoked, and this proof says no state exists
    /// in which the deleted slot can be invoked again.
    /// **The occupancy count is the slots, for every reachable table state** (milestone 231).
    ///
    /// The count exists so a boot can report the capability-slot high-water mark it reached, and a
    /// count that has drifted reports a mark that was never true. Nothing else in this file checks
    /// it: the accessors return it, the tests check the sequences somebody thought to write, and
    /// the kernel prints it.
    ///
    /// **Written because review of milestone 231 asked the question the change put in front of the
    /// two `delete` harnesses below**: occupying a slot has one path and freeing had two, so a
    /// mutator that emptied a slot without saying so would leave `used` too high forever and the
    /// mark with it. This proves it symbolically over every combination of occupied slots and every
    /// slot index, rather than answering with the reasoning that there are only two such sites
    /// today.
    ///
    /// It covers the freeing side deliberately: `any_small_capability_table` builds through `put`,
    /// so growth is exercised on the way in, and both `delete` and `delete_matching` run here.
    ///
    /// **Why this harness carries `#[kani::unwind(4)]`, and why 4 is sound** (milestone 126 (the
    /// `procps` package), 2026-09-29, UTC). The free-slot word's sweep (`delete_matching`) walks the
    /// occupied mask, and a bit-scan's trip count is data-dependent (one iteration per set bit,
    /// lowest first), so symbolic execution cannot read it off an induction variable the way it
    /// reads `0..N`: unbounded, Kani was still unwinding iteration 1669 on this three-slot table
    /// when the CI shard hit its cap. The bound is N + 1 for this harness's N = 3: the builder and
    /// the occupancy fold loop over `0..3` exactly, and the bit-scan clears one occupied bit per
    /// iteration over a mask `occupied_mask()` restricts to the N bits that exist, so no loop in
    /// here can run a fourth time. Kani's unwinding assertion fails if one could, which makes a
    /// raise of `N` that outgrew the bound a verification failure rather than a proof that quietly
    /// covers less.
    ///
    /// Falsification: replayable `crates/capability/falsifications/verification.the_count_is_the_slots.patch`
    #[kani::proof]
    #[kani::unwind(4)]
    fn the_count_is_the_slots() {
        fn occupied(cs: &CapabilityTable<u8, 3>) -> usize {
            (0..3).filter(|&i| cs.slot(i).is_some()).count()
        }
        let mut cs = any_small_capability_table();
        assert_eq!(cs.used(), occupied(&cs), "the count is wrong at the start");

        let slot: u64 = kani::any();
        let _ = cs.delete(slot);
        assert_eq!(cs.used(), occupied(&cs), "delete left the count behind");

        // The sweep, which frees any number of slots in one pass and is the other freeing path.
        let wanted: u8 = kani::any();
        cs.delete_matching(|o| *o == wanted);
        assert_eq!(
            cs.used(),
            occupied(&cs),
            "delete_matching left the count behind"
        );

        // And the mark never fell while all that was happening, which is what makes it a mark.
        assert!(cs.peak() >= cs.used());
    }

    /// **The free-slot word is the set of empty slots, after every mutator** (milestone 126 (the
    /// `procps` package), calef's ruling on #1360, 2026-09-27, UTC).
    ///
    /// `insert` no longer looks at the array to find room; it believes the word. A word that says
    /// "free" over an occupied slot makes `insert` overwrite a live capability, which is a
    /// capability silently vanishing from a server's table (a Reply orphaned, a grant lost), and a
    /// word that says "occupied" over an empty slot leaks that slot until the thread dies. The
    /// module privacy around `storage` is what keeps the word honest in practice; this is the check
    /// that the two doors inside it are right, symbolically, over every state of a small table and
    /// every slot index, in bounds or not, through every mutator in turn.
    ///
    /// Bits at and above `N` must stay clear too, or a full table would not be a zero word and
    /// `insert` would hand out a slot that does not exist.
    ///
    /// **Same bound and same argument as `the_count_is_the_slots`: `#[kani::unwind(4)]` is N + 1
    /// for this harness's N = 3.** Every loop here is the builder's exact `0..3`, one of the two
    /// `empties` folds over `0..3`, or the sweep's bit-scan, which `occupied_mask()` holds to the N
    /// bits that exist and which clears one bit per iteration; nothing can iterate a fourth time,
    /// and the unwinding assertion says so by failing if one ever could.
    ///
    /// Falsification: replayable `crates/capability/falsifications/verification.the_free_mask_is_the_empty_slots.patch`
    #[kani::proof]
    #[kani::unwind(4)]
    fn the_free_mask_is_the_empty_slots() {
        fn empties(cs: &CapabilityTable<u8, 3>) -> u64 {
            (0..3)
                .filter(|&i| cs.slot(i).is_none())
                .fold(0, |m, i| m | (1 << i))
        }
        let cap = || Cap {
            object: kani::any(),
            rights: Rights(kani::any()),
        };
        let mut cs = any_small_capability_table();
        assert_eq!(cs.free_mask(), empties(&cs), "wrong after put");

        let _ = cs.insert(cap());
        assert_eq!(cs.free_mask(), empties(&cs), "wrong after insert");

        let _ = cs.delete(kani::any());
        assert_eq!(cs.free_mask(), empties(&cs), "wrong after delete");

        let _ = cs.insert_at(kani::any(), cap());
        assert_eq!(cs.free_mask(), empties(&cs), "wrong after insert_at");

        let _ = cs.derive(kani::any(), kani::any(), Rights(kani::any()));
        assert_eq!(cs.free_mask(), empties(&cs), "wrong after derive");

        let wanted: u8 = kani::any();
        cs.delete_matching(|o| *o == wanted);
        assert_eq!(cs.free_mask(), empties(&cs), "wrong after delete_matching");
        assert_eq!(cs.is_empty(), empties(&cs) == 0b111);
    }

    /// Falsification: replayable `crates/capability/falsifications/verification.a_deleted_capability_stays_deleted.patch`
    #[kani::proof]
    fn a_deleted_capability_stays_deleted() {
        let mut cs = any_small_capability_table();
        let slot: u64 = kani::any();
        if cs.delete(slot).is_ok() {
            // The storage itself, before the accessors (milestone 211's sweep). A
            // strengthening rather than a finding: `delete` does not call `get`, and the sweep
            // found no defect the two-accessor phrasing misses, because a `delete` that
            // returned `Ok` without clearing the slot is caught by the `get` line and a `get`
            // stuck at `NoSuchSlot` is caught by `derive_never_widens_rights` next door. It is
            // here so the claim rests on the array rather than on two readers of it.
            assert!(
                cs.slot(slot as usize).is_none(),
                "the slot still holds a capability"
            );
            assert_eq!(cs.get(slot).err(), Some(Error::NoSuchSlot));
            assert_eq!(cs.delete(slot).err(), Some(Error::NoSuchSlot));
        }
    }

    /// **Delete is slot-local.** Deleting any slot, in bounds or not, leaves every other slot
    /// exactly as it was. A server holding one-shot Reply capabilities for two callers consumes
    /// one and must still hold the other, or answering caller A would silently orphan caller B.
    /// Falsification: replayable `crates/capability/falsifications/verification.delete_touches_only_its_slot.patch`
    #[kani::proof]
    fn delete_touches_only_its_slot() {
        let mut cs = any_small_capability_table();
        let victim: u64 = kani::any();
        let other: u64 = kani::any();
        kani::assume(victim != other);
        kani::assume(other < 3);

        let before = cs.get(other);
        let _ = cs.delete(victim);
        assert_eq!(cs.get(other), before);
    }

    /// **The central theorem, on the real `CapabilityTable::derive`.** For any source rights and any request,
    /// if the derive succeeds then the capability it stored holds no more than the source did, and
    /// holds exactly what was asked (no silent grant of more). There is no reachable input that
    /// widens authority.
    /// Falsification: replayable `crates/capability/falsifications/verification.derive_never_widens_rights.patch`
    #[kani::proof]
    fn derive_never_widens_rights() {
        let src_rights = Rights(kani::any());
        let requested = Rights(kani::any());

        let mut cs: CapabilityTable<u8, 2> = CapabilityTable::new();
        cs.put(
            0,
            Cap {
                object: 0u8,
                rights: src_rights,
            },
        )
        .unwrap();

        if cs.derive(0, 1, requested).is_ok() {
            let derived = cs
                .slot(1)
                .expect("a successful derive filled the destination slot");
            // **Stated in raw bits, not through `is_subset_of`** (milestone 211). `derive` guards
            // on `rights.is_subset_of(src.rights)`, so a phrasing that asserted
            // `derived.rights.is_subset_of(src_rights)` would be satisfied by any consistently
            // wrong `is_subset_of` and could not see the predicate underneath the guard break.
            // Milestone 194 measured exactly that: swapping the operands inside `is_subset_of`
            // left this harness green. `x & !y == 0` is the definition the predicate is supposed
            // to implement, written where the implementation cannot choose it.
            assert_eq!(
                derived.rights.0 & !src_rights.0,
                0,
                "a derived capability holds a right the source did not",
            );
            assert_eq!(
                requested.0 & !src_rights.0,
                0,
                "a request wider than the source got past the guard",
            );
            // And it holds exactly what was asked: no silent grant of more, no silent clamp to
            // less. Field equality, so no predicate stands between the claim and the bits.
            assert_eq!(derived.rights.0, requested.0);
            assert_eq!(derived.object, 0u8);
        }
    }

    /// **Authority never widens at the other mint site either: `split` inherits, it does not grant.**
    /// `MemoryRegion::SPLIT` mints a child budget outside [`CapabilityTable::derive`] (kernel/src/cap.rs), so
    /// `derive_never_widens_rights` does not reach it; the kernel routes that mint through
    /// [`Cap::mint_child`], and this proves the theorem there. Milestone 35.
    ///
    /// **What the property is, precisely, because DECISIONS §16's amendment makes the obvious phrasing
    /// wrong.** `SPLIT` grants the child `GRANT` so that a budget can be delegated onward, so "SPLIT
    /// never *changes* rights" is false as a description of the feature's intent, and "never widens"
    /// invites the question of what the model permits. What is proved is the strongest of the three and
    /// the one that settles it: **the child's rights are exactly the parent's.** `mint_child` takes no
    /// rights argument, so there is no input by which a caller could ask for more, and equality implies
    /// the subset relation the rest of the model is stated in (asserted too, so the corollary the
    /// syscall layer relies on is checked here and not inferred by a reader).
    ///
    /// The delegability is therefore a property of the *root's* mint, not a widening at `SPLIT`:
    /// `memory_region_root_cap` mints once with `READ|WRITE|GRANT`, `SPLIT` inherits whatever the parent
    /// holds, `CAP_INSERT` narrows on the way into a child. Rights down a budget tree are monotonically
    /// non-increasing from the root, a child holds `GRANT` only because the root did, and a spend-only
    /// untyped provably cannot split itself a `GRANT`-bearing child.
    /// Falsification: replayable `crates/capability/falsifications/verification.split_never_widens_rights.patch`
    #[kani::proof]
    fn split_never_widens_rights() {
        let parent = Cap {
            object: 7u8,
            rights: Rights(kani::any()),
        };
        let child_object: u8 = kani::any();

        let child = parent.mint_child(child_object);

        // Exactly the parent's, for every one of the 2^32 rights patterns: inheritance, not a grant.
        assert_eq!(
            child.rights, parent.rights,
            "an inheriting mint changed the child's rights",
        );
        // The weaker relation the capability model is phrased in, which the above implies.
        assert!(child.rights.is_subset_of(parent.rights));
        assert_eq!(child.object, child_object);
    }

    /// **A reap is authorized by the supervision relationship, and by nothing else** (DECISIONS
    /// §32). For every pair of rendezvous names and every liveness, `Permitted` implies the thread's
    /// recorded fault rendezvous *is* the rendezvous being invoked and the thread is dead. There is no
    /// input where an unsupervised thread, another supervisor's child, or a stale name (all
    /// `fault_ep` values that do not equal `invoked_ep`, including `None`) is reapable.
    ///
    /// This is the property that makes "a capability that cannot build cannot be made to build via
    /// reap" hold: authorization consumes no capability and produces none. `reap_decision` takes no
    /// `Cap` and returns no `Cap`, so there is no channel through which authority could flow to the
    /// reaper. The proof pins the *gate*; the type pins the absence of a grant.
    /// Falsification: replayable `crates/capability/falsifications/verification.reap_is_permitted_only_to_the_supervising_rendezvous.patch`
    #[kani::proof]
    fn reap_is_permitted_only_to_the_supervising_rendezvous() {
        let invoked: u64 = kani::any();
        let fault_ep: Option<u64> = if kani::any() { Some(kani::any()) } else { None };
        let dead: bool = kani::any();

        match reap_decision(fault_ep, invoked, dead) {
            Reap::Permitted => {
                assert_eq!(
                    fault_ep,
                    Some(invoked),
                    "reaped through the wrong rendezvous"
                );
                assert!(dead, "reaped a live thread");
            }
            // A live child is refused as such only when it really is ours; otherwise the answer
            // must disclose nothing, which is the next harness.
            Reap::StillAlive => {
                assert_eq!(fault_ep, Some(invoked));
                assert!(!dead);
            }
            Reap::NotSupervised => assert_ne!(fault_ep, Some(invoked)),
        }
    }

    /// **The refusal for a thread you do not supervise does not depend on the thread.** For any two
    /// liveness values, a tid whose fault rendezvous is not the invoked one answers identically, so a
    /// supervisor cannot use `REAP` to learn whether some other supervisor's child (or a recycled
    /// tid) is alive. The two facts §32 wants distinguishable are distinguishable only *inside* the
    /// relationship.
    /// Falsification: replayable `crates/capability/falsifications/verification.a_stranger_reveals_nothing_about_its_liveness.patch`
    #[kani::proof]
    fn a_stranger_reveals_nothing_about_its_liveness() {
        let invoked: u64 = kani::any();
        let fault_ep: Option<u64> = if kani::any() { Some(kani::any()) } else { None };
        kani::assume(fault_ep != Some(invoked));

        assert_eq!(
            reap_decision(fault_ep, invoked, true),
            reap_decision(fault_ep, invoked, false),
        );
        assert_eq!(reap_decision(fault_ep, invoked, true), Reap::NotSupervised);
    }

    /// **A survey shows a thread only to the rendezvous that supervises it** (milestone 126). The
    /// scoping half of the same claim the two harnesses above make about the reap: for every pair
    /// of rendezvous names, inclusion implies the recorded fault rendezvous *is* the invoked one, so
    /// there is no input where an unsupervised thread, another supervisor's child, or a stale name
    /// appears in somebody else's `ps`.
    ///
    /// The `if and only if` matters in both directions here, which it does not for the reap. One
    /// direction is confinement (a stranger is never shown). The other is truthfulness: **every
    /// thread that really is in the domain is reported**, so a monitor that sees an entry missing
    /// can conclude the thread is gone rather than hidden. A view that could silently omit its own
    /// children would be worse than no view.
    /// Falsification: replayable `crates/capability/falsifications/verification.a_survey_shows_exactly_the_endpoints_own_children.patch`
    #[kani::proof]
    fn a_survey_shows_exactly_the_endpoints_own_children() {
        let invoked: u64 = kani::any();
        let fault_ep: Option<u64> = if kani::any() { Some(kani::any()) } else { None };

        assert_eq!(
            survey_includes(fault_ep, invoked),
            fault_ep == Some(invoked)
        );
    }

    /// **Seeing and collecting are scoped by the same relationship** (milestone 126). For every
    /// input, a thread is in the survey exactly when a reap of it would get past the supervision
    /// check, whatever its liveness. So the domain `ps` reports and the domain a supervisor may
    /// reap from cannot diverge, which is the property that lets one predicate answer both.
    /// Falsification: replayable `crates/capability/falsifications/verification.the_view_and_the_reap_have_the_same_scope.patch`
    #[kani::proof]
    fn the_view_and_the_reap_have_the_same_scope() {
        let invoked: u64 = kani::any();
        let fault_ep: Option<u64> = if kani::any() { Some(kani::any()) } else { None };
        let dead: bool = kani::any();

        assert_eq!(
            survey_includes(fault_ep, invoked),
            reap_decision(fault_ep, invoked, dead) != Reap::NotSupervised,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Obj {
        Console,
        PageFrame(u64),
    }

    /// **The occupied mask is the occupied slots and nothing above them.** A sweep walks this word
    /// in place of `0..N`, so a bit set for a free slot, or for a slot past `N`, is a visit to
    /// something that is not there; the sweep's own guard would hide it, so the word is checked
    /// itself.
    #[test]
    fn the_occupied_mask_is_exactly_the_occupied_slots() {
        let cap = |n| Cap {
            object: Obj::PageFrame(n),
            rights: Rights::ALL,
        };
        let mut cs: CapabilityTable<Obj, 8> = CapabilityTable::new();
        assert_eq!(cs.occupied_mask(), 0);
        cs.insert_at(2, cap(1)).unwrap();
        cs.insert_at(5, cap(2)).unwrap();
        assert_eq!(cs.occupied_mask(), 0b0010_0100);
        cs.delete(2).unwrap();
        assert_eq!(cs.occupied_mask(), 0b0010_0000);
        let mut full: CapabilityTable<Obj, 64> = CapabilityTable::new();
        assert_eq!(full.occupied_mask(), 0);
        full.insert_at(63, cap(3)).unwrap();
        assert_eq!(full.occupied_mask(), 1 << 63);
    }

    /// **A new process holds nothing.** The decision, as an assertion.
    #[test]
    fn a_new_capability_table_can_name_nothing() {
        let cs: CapabilityTable<Obj, 16> = CapabilityTable::new();

        assert!(cs.is_empty());
        for slot in 0..cs.len() as u64 {
            assert_eq!(cs.get(slot).err(), Some(Error::NoSuchSlot));
        }
    }

    /// `delete_matching` is `get`-then-`delete` over every slot, done in place: everything the
    /// predicate matches goes, and **a non-matching neighbor survives untouched**, rights and all.
    /// The second half is the property worth the test: a revocation sweep that took a bystander's
    /// capability would be a denial of service wearing a security fix's clothes.
    #[test]
    fn delete_matching_takes_the_matched_and_spares_the_neighbor() {
        let mut cs: CapabilityTable<Obj, 16> = CapabilityTable::new();
        cs.put(
            0,
            Cap {
                object: Obj::PageFrame(0x1000),
                rights: Rights::READ,
            },
        )
        .unwrap();
        cs.put(
            1,
            Cap {
                object: Obj::Console,
                rights: Rights::WRITE,
            },
        )
        .unwrap();
        cs.put(
            15,
            Cap {
                object: Obj::PageFrame(0x2000),
                rights: Rights::ALL,
            },
        )
        .unwrap();

        cs.delete_matching(|o| matches!(o, Obj::PageFrame(_)));

        // Both frames are gone, wherever they sat, including the last slot.
        assert_eq!(cs.get(0).err(), Some(Error::NoSuchSlot));
        assert_eq!(cs.get(15).err(), Some(Error::NoSuchSlot));
        // The bystander keeps its object and its rights.
        let survivor = cs.get(1).unwrap();
        assert_eq!(survivor.object, Obj::Console);
        assert_eq!(survivor.rights, Rights::WRITE);
    }

    /// You cannot fabricate a capability. There is nothing to guess.
    #[test]
    fn an_unheld_slot_is_not_permission_denied_it_is_nothing() {
        let mut cs: CapabilityTable<Obj, 16> = CapabilityTable::new();
        cs.put(
            0,
            Cap {
                object: Obj::Console,
                rights: Rights::WRITE,
            },
        )
        .unwrap();

        assert_eq!(cs.get(0).unwrap().object, Obj::Console);

        // Every other slot, and every slot past the end.
        assert_eq!(cs.get(1).err(), Some(Error::NoSuchSlot));
        assert_eq!(cs.get(15).err(), Some(Error::NoSuchSlot));
        assert_eq!(cs.get(16).err(), Some(Error::NoSuchSlot));
        assert_eq!(cs.get(u64::MAX).err(), Some(Error::NoSuchSlot));
    }

    /// **Rights only ever narrow.** If delegation could widen, the model is theatre: you would
    /// simply derive yourself a better capability from the one you have.
    #[test]
    fn derive_cannot_widen_rights() {
        let mut cs: CapabilityTable<Obj, 16> = CapabilityTable::new();
        cs.put(
            0,
            Cap {
                object: Obj::PageFrame(0x1000),
                rights: Rights::READ,
            },
        )
        .unwrap();

        // Narrowing to nothing: fine.
        assert!(cs.derive(0, 1, Rights::NONE).is_ok());
        // Same rights: fine.
        assert!(cs.derive(0, 2, Rights::READ).is_ok());

        // **Asking for WRITE when we only hold READ.**
        assert_eq!(
            cs.derive(0, 3, Rights::WRITE).err(),
            Some(Error::CannotWiden)
        );
        assert_eq!(cs.derive(0, 3, Rights::ALL).err(), Some(Error::CannotWiden));

        // And the failed derive left nothing behind.
        assert_eq!(cs.get(3).err(), Some(Error::NoSuchSlot));
    }

    /// A narrowed capability names the **same object**. Delegation moves authority, not identity.
    #[test]
    fn a_derived_capability_names_the_same_object_with_less_authority() {
        let mut cs: CapabilityTable<Obj, 16> = CapabilityTable::new();
        cs.put(
            0,
            Cap {
                object: Obj::PageFrame(0x4000),
                rights: Rights::READ.union(Rights::WRITE).union(Rights::GRANT),
            },
        )
        .unwrap();

        cs.derive(0, 5, Rights::READ).unwrap();

        let orig = cs.get(0).unwrap();
        let copy = cs.get(5).unwrap();

        assert_eq!(orig.object, copy.object, "not the same object");
        assert!(orig.rights.allows(Rights::WRITE));
        assert!(!copy.rights.allows(Rights::WRITE), "the copy kept WRITE");
    }

    /// Deriving from an empty slot fails. You cannot lend what you do not hold.
    #[test]
    fn you_cannot_delegate_what_you_do_not_hold() {
        let mut cs: CapabilityTable<Obj, 16> = CapabilityTable::new();
        assert_eq!(cs.derive(3, 4, Rights::READ).err(), Some(Error::NoSuchSlot));
    }

    #[test]
    fn get_with_refuses_rights_you_do_not_have() {
        let mut cs: CapabilityTable<Obj, 16> = CapabilityTable::new();
        cs.put(
            0,
            Cap {
                object: Obj::Console,
                rights: Rights::READ,
            },
        )
        .unwrap();

        assert!(cs.get_with(0, Rights::READ).is_ok());
        assert_eq!(
            cs.get_with(0, Rights::WRITE).err(),
            Some(Error::CannotWiden)
        );
        assert_eq!(cs.get_with(9, Rights::READ).err(), Some(Error::NoSuchSlot));
    }

    #[test]
    fn deleting_a_capability_makes_the_object_unnameable() {
        let mut cs: CapabilityTable<Obj, 16> = CapabilityTable::new();
        cs.put(
            0,
            Cap {
                object: Obj::Console,
                rights: Rights::ALL,
            },
        )
        .unwrap();

        cs.delete(0).unwrap();
        assert_eq!(cs.get(0).err(), Some(Error::NoSuchSlot));
        assert_eq!(cs.delete(0).err(), Some(Error::NoSuchSlot));
    }

    #[test]
    fn rights_arithmetic() {
        assert!(Rights::ALL.allows(Rights::WRITE));
        assert!(!Rights::READ.allows(Rights::WRITE));
        assert!(Rights::NONE.is_subset_of(Rights::READ));
        assert!(Rights::READ.is_subset_of(Rights::ALL));
        assert!(!Rights::ALL.is_subset_of(Rights::READ));
        assert_eq!(Rights::ALL.intersect(Rights::READ), Rights::READ);
    }

    /// The four cases of §32's reap gate, spelled out as the readable companion to the symbolic
    /// proofs: my corpse, my live child, another supervisor's child, an unsupervised thread (which
    /// is also what a stale tid looks like, because a name that does not resolve carries no
    /// rendezvous).
    #[test]
    fn reap_authorizes_only_a_corpse_this_rendezvous_supervises() {
        assert_eq!(reap_decision(Some(7), 7, true), Reap::Permitted);
        assert_eq!(reap_decision(Some(7), 7, false), Reap::StillAlive);
        assert_eq!(reap_decision(Some(9), 7, true), Reap::NotSupervised);
        assert_eq!(reap_decision(None, 7, true), Reap::NotSupervised);
    }

    /// The scoping half of §32's gate, and the reason it has a `cargo test` companion at all: a
    /// mutation run of 2026-09-19 (milestone 326) replaced `survey_includes` with a constant `true`
    /// and again with a constant `false`, and **both survived the whole suite**. Kani proves this
    /// predicate for every input, but `script/verify` is a different gate on a different cadence,
    /// so a build that answered "everyone is in your domain" would reach a reviewer with
    /// `script/test` green. Liveness is deliberately absent: a corpse is still in the domain, which
    /// is what lets `ps` show a `Dead` child its supervisor has not collected.
    #[test]
    fn a_survey_shows_this_rendezvous_children_and_nobody_else() {
        assert!(survey_includes(Some(7), 7));
        assert!(survey_includes(Some(0), 0));
        assert!(!survey_includes(Some(9), 7));
        assert!(!survey_includes(None, 7));
    }

    /// **Both halves of the packed high-water word, read exactly rather than as a floor.**
    ///
    /// The floor test below cannot pin either half, because `PEAK` is one static shared by every
    /// `CapabilityTable` in this binary, and a mutation run (milestone 326) showed what that costs:
    /// `>>` for `<<` in the peak half and `&` for `|` or `^` in the ceiling half all survived,
    /// since a garbage number is still above a floor of five.
    ///
    /// Exactness is available here because `fetch_max` only ever raises: an occupancy and a
    /// capacity larger than any other test in this module reaches is a record nothing else can
    /// overwrite, whatever order the harness runs things in. That is also this test's one
    /// maintenance cost, and it fails loudly rather than silently if a future test outgrows it.
    #[test]
    fn the_high_water_word_reports_the_occupancy_and_the_capacity_that_set_it() {
        let cap = Cap {
            object: Obj::PageFrame(3),
            rights: Rights::READ,
        };
        // Full at sixty-four, the most a table may have (`MAX_SLOTS`). This was forty of sixty-four
        // until the free-slot word capped a table at one `u32`, then 32 of 32; milestone 754 (the capability table grows to 64 slots) made
        // the word a `u64`, so the ceiling is sixty-four again and the free-slot word's edge test
        // fills a 64-slot table too, which writes the same word.
        let mut cs: CapabilityTable<Obj, 64> = CapabilityTable::new();
        for _ in 0..64 {
            cs.insert(cap).expect("sixty-four slots hold sixty-four");
        }

        assert_eq!(
            highest_seen(),
            (64, 64),
            "no table in this test binary can exceed 64 of 64, so this record is the one standing"
        );
    }

    /// **The free-slot word at its edge, and first-free after a hole.** Sixty-four slots is the
    /// case where every bit of the `u64` is a slot, so "all free" is `u64::MAX` and "full" is zero
    /// with no bits above `N` to mistake for room. A hole punched in the middle must be the next
    /// slot `insert` takes, lowest first, because the kernel's Reply mint relies on first-free.
    #[test]
    fn the_free_slot_word_fills_to_zero_and_hands_back_the_lowest_hole() {
        let cap = Cap {
            object: Obj::PageFrame(1),
            rights: Rights::READ,
        };
        let mut cs: CapabilityTable<Obj, 64> = CapabilityTable::new();
        assert_eq!(cs.free_mask(), u64::MAX);
        for want in 0..64u64 {
            assert_eq!(cs.insert(cap), Ok(want));
        }
        assert_eq!(cs.free_mask(), 0, "a full table is a zero word");
        assert_eq!(cs.insert(cap), Err(Error::NoFreeSlot));

        cs.delete(20).unwrap();
        cs.delete(7).unwrap();
        assert_eq!(cs.free_mask(), (1 << 20) | (1 << 7));
        assert_eq!(cs.insert(cap), Ok(7), "lowest hole first");
        assert_eq!(cs.insert(cap), Ok(20));
        assert_eq!(cs.used(), 64);

        // A small table's bits above N stay clear, so full is still zero there too.
        let mut small: CapabilityTable<Obj, 3> = CapabilityTable::new();
        assert_eq!(small.free_mask(), 0b111);
        for _ in 0..3 {
            small.insert(cap).unwrap();
        }
        assert_eq!(small.free_mask(), 0);
        assert_eq!(small.insert(cap), Err(Error::NoFreeSlot));
    }

    #[test]
    fn a_full_table_says_so() {
        let mut cs: CapabilityTable<Obj, 2> = CapabilityTable::new();
        let cap = Cap {
            object: Obj::Console,
            rights: Rights::ALL,
        };
        assert_eq!(cs.insert(cap), Ok(0));
        assert_eq!(cs.insert(cap), Ok(1));
        assert_eq!(cs.insert(cap).err(), Some(Error::NoFreeSlot));
    }

    /// The rights bits are the syscall wire format: delegation names them in a register, and
    /// `from_bits` is what turns the register back into authority. Milestone 85's mutation run
    /// showed the exact values were pinned nowhere (WRITE's `1 << 1` could become `1 >> 1`, zero,
    /// and delegating WRITE would delegate nothing), and `from_bits` could OR instead of mask, so
    /// undefined bits became defined rights.
    ///
    /// `ENUMERATE` and the `ALL` identity below were added by the 2026-08-17 documentation sweep,
    /// which found this test pinning three bits the day after a fourth landed. The `ALL` assertion
    /// is the one worth having: [`Rights::ALL`]'s own comment warns that a constant defined without
    /// widening that mask is **silently dropped at every delegation**, and until now nothing
    /// checked it. Spelling the union out by hand is deliberate, because a derivation that read
    /// `ALL` to check `ALL` would pass whatever it was given.
    /// **`insert_at_or_above` never lands below its floor**, finds the first free slot at or above
    /// it, and is `insert` at floor 0 (milestone 812).
    #[test]
    fn insert_at_or_above_respects_its_floor() {
        let mut t: CapabilityTable<u8, 64> = CapabilityTable::new();
        let capability = |o| Cap {
            object: o,
            rights: Rights::ALL,
        };
        assert_eq!(t.insert_at_or_above(12, capability(1)), Ok(12));
        assert_eq!(t.insert_at_or_above(12, capability(2)), Ok(13));
        assert_eq!(
            t.insert_at_or_above(0, capability(3)),
            Ok(0),
            "floor 0 is first free"
        );
        assert_eq!(
            t.insert_at_or_above(64, capability(4)),
            Err(Error::NoFreeSlot)
        );
        for s in 14..64 {
            t.insert_at(s, capability(5)).unwrap();
        }
        assert_eq!(
            t.insert_at_or_above(12, capability(6)),
            Err(Error::NoFreeSlot),
            "slots below the floor do not count"
        );
    }

    #[test]
    fn rights_bits_are_the_wire_format() {
        assert_eq!(
            [
                Rights::READ.bits(),
                Rights::WRITE.bits(),
                Rights::GRANT.bits(),
                Rights::ENUMERATE.bits(),
                Rights::BIND.bits()
            ],
            [1, 2, 4, 8, 16]
        );
        // Every defined bit is in ALL, and ALL defines nothing that is not a named right.
        assert_eq!(
            Rights::ALL.bits(),
            Rights::READ.bits()
                | Rights::WRITE.bits()
                | Rights::GRANT.bits()
                | Rights::ENUMERATE.bits()
                | Rights::BIND.bits()
        );
        // Undefined bits are dropped, never reinterpreted.
        assert_eq!(Rights::from_bits(!Rights::ALL.bits()), Rights::NONE);
        // Defined bits come back as exactly the rights they name.
        assert_eq!(Rights::from_bits(0b101), Rights::READ.union(Rights::GRANT));
        // Union is a union, not a toggle: adding a right twice cannot remove it.
        assert_eq!(Rights::READ.union(Rights::READ), Rights::READ);
    }

    /// `insert_at` is how a supervisor reserves the fault slot, so what matters is that the
    /// capability lands in THAT slot, the slot comes back as the receipt, and an occupied slot
    /// refuses rather than replaces (two things must never share the slot the kernel reads at
    /// START).
    #[test]
    fn insert_at_fills_exactly_the_named_slot() {
        let mut cs: CapabilityTable<Obj, 4> = CapabilityTable::new();
        let cap = Cap {
            object: Obj::PageFrame(7),
            rights: Rights::READ,
        };
        assert_eq!(cs.insert_at(2, cap), Ok(2));
        assert_eq!(cs.get(2).unwrap().object, Obj::PageFrame(7));
        assert!(!cs.is_empty());
        // The other slots are still empty, so it did not spray.
        assert_eq!(cs.get(0).err(), Some(Error::NoSuchSlot));
        // Occupied refuses; out of range is a different answer, as everywhere.
        assert_eq!(cs.insert_at(2, cap).err(), Some(Error::NoFreeSlot));
        assert_eq!(cs.insert_at(9, cap).err(), Some(Error::NoSuchSlot));
        // len is the table's size in slots, not its population.
        assert_eq!(cs.len(), 4);
    }

    /// **The high-water mark is a mark, not a level**, which is the whole reason it exists: a boot
    /// that filled the table and then emptied it reached the wall, and a count sampled afterwards
    /// says it did not.
    ///
    /// Every mutator that can occupy an empty slot is exercised here (`insert`, `insert_at`, and
    /// `put` over an empty slot), because each has its own call to the private growth path and a
    /// missed one would be invisible: the number would simply be a little too low, forever.
    #[test]
    fn the_peak_records_what_was_reached_and_never_what_is_left() {
        let cap = Cap {
            object: Obj::PageFrame(1),
            rights: Rights::READ,
        };
        let mut cs: CapabilityTable<Obj, 4> = CapabilityTable::new();
        assert_eq!((cs.used(), cs.peak()), (0, 0));

        assert_eq!(cs.insert(cap), Ok(0));
        assert_eq!(cs.insert_at(2, cap), Ok(2));
        cs.put(3, cap).expect("slot 3 is in range");
        assert_eq!((cs.used(), cs.peak()), (3, 3));

        // Replacing an occupant is not growth.
        cs.put(3, cap).expect("slot 3 is in range");
        assert_eq!((cs.used(), cs.peak()), (3, 3));

        // And emptying it does not lower the mark.
        cs.delete(0).expect("slot 0 is occupied");
        cs.delete_matching(|o| matches!(o, Obj::PageFrame(_)));
        assert_eq!((cs.used(), cs.peak()), (0, 3));

        // Refusals leave the count alone. Both of `insert_at`'s, and `delete`'s.
        assert!(cs.insert_at(9, cap).is_err());
        assert!(cs.delete(1).is_err());
        assert_eq!((cs.used(), cs.peak()), (0, 3));

        // A full table refuses, and the mark is the capacity rather than one past it.
        let mut full: CapabilityTable<Obj, 2> = CapabilityTable::new();
        assert!(full.insert(cap).is_ok());
        assert!(full.insert(cap).is_ok());
        assert_eq!(full.insert(cap).err(), Some(Error::NoFreeSlot));
        assert_eq!((full.used(), full.peak()), (2, 2));
    }

    /// The global is a convenience over the per-table mark and must never read *below* it, which is
    /// the only property the kernel relies on: it prints the global and would understate the boot
    /// if this were ever false. It is a floor rather than an equality because the static is shared
    /// by every instantiation in the binary, including the other tests in this module.
    #[test]
    fn the_global_high_water_mark_is_never_below_a_table_that_reached_it() {
        let cap = Cap {
            object: Obj::PageFrame(2),
            rights: Rights::WRITE,
        };
        let mut cs: CapabilityTable<Obj, 8> = CapabilityTable::new();
        for _ in 0..5 {
            cs.insert(cap).expect("eight slots hold five");
        }
        let (peak, ceiling) = highest_seen();
        assert!(
            peak >= cs.peak(),
            "the global said {peak} while a table had reached {}",
            cs.peak()
        );
        assert!(
            ceiling > 0,
            "a recorded peak carries the table it came from"
        );
    }
}
