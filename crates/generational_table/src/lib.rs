//! A fixed-capacity generational table: **names that die with what they named.**
//!
//! Milestone 14 phase A (design/kernel-objects-from-untyped.md, decision D2). The scheduler's
//! thread table used to be a `BTreeMap<ThreadId, Box<Thread>>` fed by a global counter: every spawn
//! allocated map nodes, and the map was unbounded. This replaces it with an array, and it
//! replaces the counter with something better than a counter.
//!
//! # The name is the safety mechanism
//!
//! A name here packs `(generation, slot)` into one `u64`. Lookup is an index and one compare.
//! When an entry is removed, its slot's generation is bumped, so every name minted for the old
//! occupant **stops resolving, forever**, even after the slot is reused. A stale name is not a
//! dangling reference and not somebody else's thread: it is `None`, safely.
//!
//! That property is why this table is the *first step toward* capability-only thread naming
//! rather than a detour from it (see the design doc). Long-lived capabilities (`Reply` today)
//! carry thread names; a name that can never resolve to the wrong thread is what makes that
//! sound without seL4's CDT. The properties are proved below, not argued.
//!
//! # The honest limits
//!
//! - **Capacity is fixed.** `insert_with` returns `None` when all `N` slots are live. The
//!   caller decides what a full table means (for the kernel: spawn fails, the same contract as
//!   out-of-memory today, and N becomes a documented limit of the image).
//! - **Generations are 32-bit.** A slot must be reused 2^32 times before an ancient name could
//!   resolve again. At one spawn per microsecond against a single slot, that is an hour and a
//!   quarter of doing nothing else; a real workload spreads reuse over every free slot. Recorded
//!   because "never" is not the same as 2^32, but this is not a bound anything will meet.
//! - **Insert scans for a free slot, O(N).** Spawn-rate work, not switch-rate work. The switch
//!   path only ever does `get`/`get_mut`, which are O(1).
//!
//! # Examples
//!
//! A name is a slot index plus a generation, so a name never refers to a later occupant of the
//! same slot. That is the property worth showing:
//!
//! ```
//! use generational_table::Table;
//!
//! let mut t: Table<&str, 4> = Table::new();
//! let a = t.insert_with(|_name| "first").unwrap();
//! assert_eq!(t.get(a), Some(&"first"));
//! assert_eq!(t.len(), 1);
//!
//! // Remove it, then fill the same slot again.
//! assert_eq!(t.remove(a), Some("first"));
//! let b = t.insert_with(|_name| "second").unwrap();
//!
//! // The old name does NOT resolve to the new occupant.
//! assert_ne!(a, b);
//! assert_eq!(t.get(a), None);
//! assert_eq!(t.get(b), Some(&"second"));
//! ```
//!
//! The table is fixed-size, so a full one refuses rather than allocating:
//!
//! ```
//! use generational_table::Table;
//!
//! let mut t: Table<u32, 2> = Table::new();
//! assert!(t.insert_with(|_| 1).is_some());
//! assert!(t.insert_with(|_| 2).is_some());
//! assert!(t.insert_with(|_| 3).is_none(), "a full table refuses");
//! ```
//!
//! Name: ratified 2026-08-23 (calef, a kernel-dependency crate naming review). Renamed from
//! `slots`: flagged in CLAUDE.md's own text as an example of a too-generic name; matches the
//! crate's own opening line ("a fixed-capacity generational table").

#![cfg_attr(not(test), no_std)]

/// A fixed-capacity table whose entries are named by `(generation, slot)` pairs packed in a
/// `u64`: generation in the high 32 bits, slot in the low 32.
///
/// Two consequences of that packing, both load-bearing for the kernel:
///
/// - The very first insert into a fresh table is named `0` (slot 0, generation 0), which is how
///   the boot thread keeps its traditional tid without a special case.
/// - With any sane `N`, no name ever equals `u64::MAX`, so a caller may keep using it as a
///   "no thread" sentinel (`cpu::NO_TID`).
pub struct Table<T, const N: usize> {
    slots: [Option<T>; N],
    /// The current generation of each slot: the one a live occupant's name carries, and the one
    /// the *next* occupant's name will carry after a bump. Survives the occupant on purpose.
    gens: [u32; N],
    /// Live-entry count, maintained so `len` is O(1).
    live: usize,
    /// **One past the highest live slot**, and the reason a whole-table sweep costs what the table
    /// holds rather than what it could hold.
    ///
    /// Every slot at or above this index is `None`, which is the invariant that makes bounding a
    /// walk by it identical to walking all `N`. `insert_with` is first-fit, so occupancy packs
    /// toward slot 0 and this stays close to [`len`](Self::len).
    ///
    /// It exists because the cost of a sweep was tracking `N`. `iter_mut` and friends yield only
    /// live entries but *visited* every slot to filter them, so `sched::delete_page_frame_caps_where`
    /// (which walks every thread's capability table on every `MemoryRegion::DESTROY`) got more
    /// expensive when `sched::MAX_THREADS` was raised, even though the number of live threads had
    /// not moved. Measured on the `spawn_el0` icount benchmark, doubling that ceiling from 128 to
    /// 256 cost **+141,359 ticks** through this path alone, for slots nothing occupied. Worse, it
    /// was a ratchet: every future raise would have paid again. See notes/benchmarks.md.
    ///
    /// It shrinks as well as grows, so the property is "tracks live occupancy" rather than the
    /// weaker "tracks peak occupancy". The shrink walks down over slots that are already empty and
    /// stops at the first live one, which is amortised O(1): a step down can only happen once per
    /// slot that was filled and freed.
    top: usize,
}

impl<T, const N: usize> Table<T, N> {
    /// An empty table: every slot free, every generation at 0.
    pub const fn new() -> Self {
        // A compile-time guard on the packing: slots must fit in the low 32 bits.
        const { assert!(N > 0 && N <= u32::MAX as usize) };
        Self {
            slots: [const { None }; N],
            gens: [0; N],
            live: 0,
            top: 0,
        }
    }

    const fn name(slot: usize, generation: u32) -> u64 {
        ((generation as u64) << 32) | slot as u64
    }

    const fn unpack(name: u64) -> (usize, u32) {
        ((name & 0xffff_ffff) as usize, (name >> 32) as u32)
    }

    /// Insert, minting the entry's name first so the entry can carry it: `f` receives the name
    /// and returns the value to store. `None` (and `f` never called) if every slot is live.
    pub fn insert_with(&mut self, f: impl FnOnce(u64) -> T) -> Option<u64> {
        let slot = self.slots.iter().position(|s| s.is_none())?;
        let name = Self::name(slot, self.gens[slot]);
        self.slots[slot] = Some(f(name));
        self.live += 1;
        // First-fit, so the free slot found is at most `top`, and this is the only way `top` grows.
        if slot >= self.top {
            self.top = slot + 1;
        }
        Some(name)
    }

    /// **The validated slot index behind a name**, or `None` exactly when the name is dead or
    /// garbage. This is the first half of every lookup, exposed (milestone 14 phase B.2) because
    /// the kernel stores TCBs in a parallel pool with pool slot i = table slot i, so the index
    /// *is* the storage address. `get`/`get_mut`/`remove` are built on this, so every harness
    /// that proves a stale name never resolves proves it for `slot_of` too.
    pub fn slot_of(&self, name: u64) -> Option<usize> {
        let (slot, generation) = Self::unpack(name);
        if *self.gens.get(slot)? != generation {
            return None;
        }
        self.slots[slot].as_ref()?;
        Some(slot)
    }

    /// Resolve a name. `None` for a name whose entry was removed (however long ago), a name from
    /// a previous occupant of the slot, or garbage. **Never** the wrong entry.
    pub fn get(&self, name: u64) -> Option<&T> {
        let slot = self.slot_of(name)?;
        self.slots[slot].as_ref()
    }

    /// [`get`](Self::get), mutably.
    pub fn get_mut(&mut self, name: u64) -> Option<&mut T> {
        let slot = self.slot_of(name)?;
        self.slots[slot].as_mut()
    }

    /// Remove and return the entry, bumping the slot's generation: from this moment `name` (and
    /// every copy of it, wherever it is held) resolves to `None`, forever.
    pub fn remove(&mut self, name: u64) -> Option<T> {
        let slot = self.slot_of(name)?;
        let value = self.slots[slot].take()?;
        self.gens[slot] = self.gens[slot].wrapping_add(1);
        self.live -= 1;
        // Removing anything below the top leaves a hole, which costs a walk nothing to skip.
        // Removing the top one means the bound can come down, past however many holes are under it.
        if slot + 1 == self.top {
            // **Bounded by `N` rather than by `while`**, which is not style: CBMC cannot see that
            // `top <= N` from the field alone, so a `while` here unwinds without limit and the
            // Kani harness below never finishes. A `for` over the array's own length gives the
            // prover the bound the type already guarantees, and the `break` means the runtime cost
            // is unchanged (it stops at the first live slot, and can only step over a slot that
            // was filled and freed, so it is amortised O(1)).
            let mut top = self.top;
            for _ in 0..N {
                if top == 0 || self.slots[top - 1].is_some() {
                    break;
                }
                top -= 1;
            }
            self.top = top;
        }
        Some(value)
    }

    /// The number of live entries. O(1): maintained on every insert and remove rather than
    /// counted by walking `slots`.
    pub fn len(&self) -> usize {
        self.live
    }

    /// Whether the table currently holds no live entries.
    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// Every live entry, in slot order. For whole-table sweeps (revocation walks every capability table);
    /// nothing on a hot path iterates.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        let top = self.top;
        self.slots[..top].iter_mut().filter_map(|s| s.as_mut())
    }

    /// The slot index of every live entry, in slot order; each index appears at most once. The
    /// pool-backed kernel table iterates its parallel storage with this.
    pub fn live_slots(&self) -> impl Iterator<Item = usize> + '_ {
        self.slots[..self.top]
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|_| i))
    }

    /// Every live entry by shared reference, in slot order. For a table whose values *are* the
    /// storage (the kernel's page-resident TCB table stores a `*mut Thread` per name), this is
    /// how the caller reaches each without a second lookup.
    pub fn values(&self) -> impl Iterator<Item = &T> + '_ {
        self.slots[..self.top].iter().filter_map(|s| s.as_ref())
    }

    /// Every live entry with its current name, in slot order. For sweeps that must *remove* what
    /// they find and so need the name, not just the value (object revocation: find every object
    /// backed by a region, then remove it by name). `values`/`iter_mut` suffice when the caller
    /// only needs the entry, as a TCB carries its own name and does not need this.
    pub fn iter(&self) -> impl Iterator<Item = (u64, &T)> + '_ {
        self.iter_from(0).map(|(_, name, t)| (name, t))
    }

    /// Every live entry from slot `from` onward, as `(slot, name, &T)`, in slot order.
    ///
    /// **The slot index is what makes a sweep resumable across calls**, which [`iter`](Self::iter)
    /// cannot be: a caller that must give the lock back between entries needs somewhere to start
    /// again, and a *position* in a filtered sequence is not that, because entries removed in the
    /// gap move it. A slot index is stable in the only sense a resume needs: the entry that was at
    /// slot `k` is still the only thing that can be at slot `k`, and if it died, `k` is simply
    /// empty rather than somebody else's.
    ///
    /// The one caller today is `rendezvous::SURVEY` (milestone 126), which reports one supervised
    /// thread per syscall and hands the next slot back to userspace as a cursor. What it buys is
    /// stated plainly and so is what it does not: the *set* can change between calls, so a resumed
    /// sweep is a sequence of snapshots rather than one. It never reports an entry twice and never
    /// resolves a cursor to the wrong thread, which is all a `readdir` shape ever promised.
    ///
    /// `from >= N` yields nothing, so a caller need not bound its own cursor.
    ///
    /// ```
    /// use generational_table::Table;
    ///
    /// let mut t: Table<&str, 4> = Table::new();
    /// let a = t.insert_with(|_| "a").unwrap();
    /// let b = t.insert_with(|_| "b").unwrap();
    ///
    /// // Resume after the first entry's slot and the second is what comes back.
    /// let (slot, _, _) = t.iter_from(0).next().unwrap();
    /// let rest: Vec<_> = t.iter_from(slot + 1).map(|(_, n, v)| (n, *v)).collect();
    /// assert_eq!(rest, vec![(b, "b")]);
    ///
    /// // The resumed entry is not affected by what happened to the one before it.
    /// t.remove(a);
    /// let rest: Vec<_> = t.iter_from(slot + 1).map(|(_, n, v)| (n, *v)).collect();
    /// assert_eq!(rest, vec![(b, "b")], "a removal below the cursor moved the resume point");
    ///
    /// // And a cursor past the end is empty rather than a panic.
    /// assert_eq!(t.iter_from(99).count(), 0);
    /// ```
    pub fn iter_from(&self, from: usize) -> impl Iterator<Item = (usize, u64, &T)> + '_ {
        self.slots[..self.top]
            .iter()
            .enumerate()
            .skip(from)
            .filter_map(|(slot, s)| {
                s.as_ref()
                    .map(|t| (slot, Self::name(slot, self.gens[slot]), t))
            })
    }
}

impl<T, const N: usize> Default for Table<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Machine-checked proofs (DECISIONS §14). These are the properties the design doc says
/// capability payloads will one day lean on, proved before anything leans on them. A two-slot
/// table exhibits every case that matters: the slot in question, a different slot, and reuse.
#[cfg(kani)]
mod verification {
    use super::*;

    /// **A removed name never resolves again**, even after its slot is reused. This is the
    /// stale-`ThreadId` safety the kernel currently gets from map-lookup-fails, kept under reuse: the
    /// old name and the new occupant's name differ (the generation moved), so `get`, `get_mut`,
    /// and `remove` all refuse the old one. The one stated precondition: the slot's generation
    /// has not wrapped all the way around (see the crate doc on 2^32).
    /// Falsification: replayable `crates/generational_table/falsifications/verification.a_removed_name_never_resolves_again.patch`
    #[kani::proof]
    fn a_removed_name_never_resolves_again() {
        let mut t: Table<u8, 2> = Table::new();
        let Some(name) = t.insert_with(|_| kani::any()) else {
            return;
        };
        t.remove(name);

        // Reuse the slot: the new occupant must not be reachable through the dead name.
        let reused = t.insert_with(|_| kani::any());
        if let Some(new_name) = reused {
            assert_ne!(new_name, name);
        }
        assert!(t.get(name).is_none());
        assert!(t.get_mut(name).is_none());
        assert!(t.remove(name).is_none());
    }

    /// **Two live entries never share a name**, and each name resolves to its own entry: the
    /// packing cannot alias two occupants.
    /// Falsification: replayable `crates/generational_table/falsifications/verification.live_names_are_distinct_and_resolve_to_their_own_entry.patch`
    #[kani::proof]
    fn live_names_are_distinct_and_resolve_to_their_own_entry() {
        let mut t: Table<u8, 2> = Table::new();
        let (a_val, b_val): (u8, u8) = (kani::any(), kani::any());
        let a = t.insert_with(|_| a_val).unwrap();
        let b = t.insert_with(|_| b_val).unwrap();

        assert_ne!(a, b);
        assert_eq!(t.get(a).copied(), Some(a_val));
        assert_eq!(t.get(b).copied(), Some(b_val));
    }

    /// **A garbage name resolves to nothing.** For any u64 whatsoever, resolution either fails
    /// or lands on an entry whose minted name is exactly that u64: there is no input that
    /// reaches an entry through a name the table never issued. (The u64 arrives in syscall
    /// registers by way of capability bookkeeping; this is the "cannot forge" claim for names.)
    /// Falsification: replayable `crates/generational_table/falsifications/verification.a_name_the_table_never_minted_resolves_to_nothing.patch`
    #[kani::proof]
    fn a_name_the_table_never_minted_resolves_to_nothing() {
        let mut t: Table<u8, 2> = Table::new();
        let minted = t.insert_with(|_| kani::any());
        let probe: u64 = kani::any();
        if t.get(probe).is_some() {
            assert_eq!(Some(probe), minted);
        }
    }

    /// **The occupancy bound never hides a live entry.** The walks are bounded by `top` rather
    /// than by `N`, which is sound only while every slot at or above `top` is empty. Fill a table,
    /// then remove an arbitrary subset (all eight combinations, including the ones that leave a
    /// hole under a live entry and the one that empties the table), and every walk must still
    /// report exactly as many entries as `len` claims. A bound that had come down too far would
    /// yield fewer, and the old unbounded walk could not go wrong this way, so this proof is new
    /// with the bound.
    ///
    /// Three slots and a subset rather than a nondeterministic sequence of operations: the first
    /// draft interleaved arbitrary inserts and removes and did not finish, which is a cost CI pays
    /// on every run (`script/verify` is already the merge queue's long pole). This covers the same
    /// cases the bound can get wrong.
    /// Falsification: replayable `crates/generational_table/falsifications/verification.a_bounded_walk_yields_every_live_entry.patch`
    #[kani::proof]
    fn a_bounded_walk_yields_every_live_entry() {
        let mut t: Table<u8, 3> = Table::new();
        let names = [
            t.insert_with(|_| 1).unwrap(),
            t.insert_with(|_| 2).unwrap(),
            t.insert_with(|_| 3).unwrap(),
        ];

        let drop_it: [bool; 3] = [kani::any(), kani::any(), kani::any()];
        for (name, drop_it) in names.iter().zip(drop_it) {
            if drop_it {
                t.remove(*name);
            }
        }

        assert_eq!(t.values().count(), t.len());
        assert_eq!(t.live_slots().count(), t.len());
        assert_eq!(t.iter_from(0).count(), t.len());

        // And each survivor is still reachable by name, so the walk and the lookup agree.
        for (name, dropped) in names.iter().zip(drop_it) {
            assert_eq!(t.get(*name).is_some(), !dropped);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The boot-thread contract: the first insert into a fresh table is named 0.
    #[test]
    fn the_first_name_is_zero() {
        let mut t: Table<&str, 8> = Table::new();
        assert_eq!(t.insert_with(|_| "boot"), Some(0));
        assert_eq!(t.get(0), Some(&"boot"));
    }

    /// The entry can carry its own name, which is how a Thread learns its `ThreadId`.
    #[test]
    fn the_entry_is_handed_its_name() {
        let mut t: Table<u64, 8> = Table::new();
        let name = t.insert_with(|n| n).unwrap();
        assert_eq!(t.get(name), Some(&name));
    }

    /// `iter` yields each live entry paired with the very name that resolves to it, and skips the
    /// dead. This is what lets object revocation find every object backed by a region and then
    /// remove it by name (an `AddressSpace`, unlike a Thread, does not carry its own name).
    #[test]
    fn iter_yields_names_that_resolve_to_their_entry() {
        let mut t: Table<&str, 4> = Table::new();
        let a = t.insert_with(|_| "a").unwrap();
        let b = t.insert_with(|_| "b").unwrap();
        let c = t.insert_with(|_| "c").unwrap();
        t.remove(b); // a hole in the middle: iter must skip it

        let (mut count, mut saw_a, mut saw_c) = (0, false, false);
        for (name, &val) in t.iter() {
            count += 1;
            // Every name iter hands back resolves to exactly that entry.
            assert_eq!(t.get(name), Some(&val));
            match val {
                "a" => {
                    assert_eq!(name, a);
                    saw_a = true;
                }
                "c" => {
                    assert_eq!(name, c);
                    saw_c = true;
                }
                other => panic!("iter yielded a removed or unknown entry: {other}"),
            }
        }
        assert_eq!(count, 2, "iter must skip the removed slot");
        assert!(saw_a && saw_c, "iter must yield every live entry");
    }

    #[test]
    fn a_full_table_refuses_and_does_not_call_the_closure() {
        let mut t: Table<u32, 2> = Table::new();
        t.insert_with(|_| 1).unwrap();
        t.insert_with(|_| 2).unwrap();
        assert_eq!(
            t.insert_with(|_| unreachable!("closure ran on a full table")),
            None
        );
        assert_eq!(t.len(), 2);
    }

    /// Remove, then reuse: the slot comes back, the name does not.
    #[test]
    fn reuse_changes_the_name_and_kills_the_old_one() {
        let mut t: Table<&str, 1> = Table::new();
        let first = t.insert_with(|_| "first").unwrap();
        assert_eq!(t.remove(first), Some("first"));

        let second = t.insert_with(|_| "second").unwrap();
        assert_ne!(first, second, "a reused slot must mint a fresh name");
        assert_eq!(t.get(first), None, "the dead name resolved");
        assert_eq!(t.get(second), Some(&"second"));
    }

    #[test]
    fn iter_mut_sees_only_live_entries() {
        let mut t: Table<u32, 4> = Table::new();
        let a = t.insert_with(|_| 1).unwrap();
        t.insert_with(|_| 2).unwrap();
        t.remove(a);

        let live: Vec<u32> = t.iter_mut().map(|v| *v).collect();
        assert_eq!(live, vec![2]);
        assert_eq!(t.len(), 1);
    }

    /// `get_mut` hands back the entry itself, and a write through it lands in that slot and no
    /// other. The kernel reaches every thread this way and `unwrap()`s the result on the switch
    /// path (`sched.rs`), so a `get_mut` that refused a live name would be a panic in the
    /// scheduler rather than a lookup that missed.
    #[test]
    fn get_mut_reaches_the_live_entry_and_writes_through_it() {
        let mut t: Table<u32, 4> = Table::new();
        let a = t.insert_with(|_| 1).unwrap();
        let b = t.insert_with(|_| 2).unwrap();

        *t.get_mut(a).expect("a live name must resolve") = 41;
        assert_eq!(t.get(a), Some(&41));
        assert_eq!(t.get(b), Some(&2), "the write landed in the wrong slot");

        t.remove(a);
        assert_eq!(t.get_mut(a), None, "a dead name resolved through get_mut");
    }

    /// Empty in both directions. Nothing in the tree calls `is_empty` yet (it exists because a
    /// type with `len` owes one), which makes it exactly the kind of accessor that can rot into a
    /// constant with no caller to notice.
    #[test]
    fn a_table_is_empty_only_while_it_holds_nothing() {
        let mut t: Table<u32, 2> = Table::new();
        assert!(t.is_empty(), "a fresh table holds nothing");
        let a = t.insert_with(|_| 9).unwrap();
        assert!(!t.is_empty());
        t.remove(a);
        assert!(t.is_empty(), "removing the last entry must empty the table");
    }

    /// **A hole below the bound is skipped, and the bound still reaches what is above it.** The
    /// case the occupancy bound could get wrong: remove from the middle and the walk must still
    /// find the entry in the higher slot, because `top` may only come down over slots that are
    /// empty.
    #[test]
    fn a_hole_below_the_bound_does_not_truncate_the_walk() {
        let mut t: Table<u32, 8> = Table::new();
        let a = t.insert_with(|_| 10).unwrap();
        let b = t.insert_with(|_| 20).unwrap();
        let c = t.insert_with(|_| 30).unwrap();

        t.remove(b);
        assert_eq!(t.values().copied().collect::<Vec<_>>(), vec![10, 30]);
        assert_eq!(t.live_slots().collect::<Vec<_>>(), vec![0, 2]);
        assert_eq!(t.iter().map(|(n, _)| n).collect::<Vec<_>>(), vec![a, c]);

        // Removing the top one lets the bound come down past the hole; what is left is still found.
        t.remove(c);
        assert_eq!(t.values().copied().collect::<Vec<_>>(), vec![10]);
        // The values alone cannot see the bound: a `top` stuck at 3 walks two empty slots and yields
        // the same list. What the bound buys is the walk's length, so read it directly; slot 0 is
        // the highest live slot left, and the shrink stepped over the hole at slot 1 to get there.
        assert_eq!(
            t.top, 1,
            "the bound comes down past the hole to the highest live slot"
        );

        // And it grows again on the next insert, which first-fit puts back in the hole.
        let d = t.insert_with(|_| 40).unwrap();
        assert_eq!(t.values().copied().collect::<Vec<_>>(), vec![10, 40]);
        assert_eq!(t.get(d), Some(&40));
        assert_eq!(
            t.iter_from(1).count(),
            1,
            "a resumed sweep still reaches the refilled slot"
        );
    }

    /// **An emptied table walks nothing at all**, which is the bound at its floor: `top` came all
    /// the way back to zero rather than staying at the high-water mark.
    #[test]
    fn an_emptied_table_costs_a_walk_nothing() {
        let mut t: Table<u32, 8> = Table::new();
        let names: Vec<_> = (0..5).map(|i| t.insert_with(|_| i).unwrap()).collect();
        for n in &names {
            t.remove(*n);
        }
        assert_eq!(t.len(), 0);
        assert_eq!(t.values().count(), 0);
        assert_eq!(t.iter_from(0).count(), 0);
        assert_eq!(t.top, 0, "an emptied table's walks visit no slot at all");
    }

    /// Out-of-range slots and wrong generations are both just `None`.
    #[test]
    fn garbage_names_are_none() {
        let mut t: Table<u32, 2> = Table::new();
        t.insert_with(|_| 7).unwrap();
        assert_eq!(t.get(1), None); // valid slot, nothing there
        assert_eq!(t.get(99), None); // no such slot
        assert_eq!(t.get(1 << 32), None); // slot 0, wrong generation
        assert_eq!(t.get(u64::MAX), None); // the NO_TID sentinel never resolves
    }
}
